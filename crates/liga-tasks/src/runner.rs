use std::borrow::Cow;
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use liga_core::Shutdown;
use tokio::sync::{Notify, Semaphore, watch};
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::error::{SpawnError, TaskError, TaskFailure};
use crate::retry::RetryPolicy;

/// Identifier unique within one runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    /// Numeric value.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task-{}", self.0)
    }
}

/// Observable state of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskStatus {
    /// Waiting for a concurrency slot.
    Queued,
    /// An attempt is running.
    Running {
        /// 1-based attempt number.
        attempt: u32,
    },
    /// Waiting before the next attempt.
    Retrying {
        /// Attempt that will run next.
        next_attempt: u32,
        /// Backoff being waited.
        delay: Duration,
    },
    /// Finished successfully.
    Succeeded,
    /// Finished with an error.
    Failed,
    /// Last attempt timed out.
    TimedOut,
    /// Cancelled.
    Cancelled,
    /// Panicked.
    Panicked,
}

impl TaskStatus {
    /// `true` for states that will not change any more.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::TimedOut | Self::Cancelled | Self::Panicked
        )
    }
}

/// Per-task options. Unset values use the runner defaults.
#[derive(Debug, Clone)]
pub struct TaskSpec {
    name: Cow<'static, str>,
    timeout: Option<Duration>,
    retry: Option<RetryPolicy>,
}

impl TaskSpec {
    /// A task with a name used in logs and traces.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            timeout: None,
            retry: None,
        }
    }

    /// Timeout for each attempt.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Retry policy.
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = Some(retry);
        self
    }
}

/// Passed to every attempt.
#[derive(Debug, Clone)]
pub struct TaskContext {
    id: TaskId,
    name: Arc<str>,
    attempt: u32,
    cancel: Shutdown,
    runner_cancel: Shutdown,
}

impl TaskContext {
    /// Task ID.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Task name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 1-based attempt number.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// `true` once the task (or the whole runner) was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_triggered() || self.runner_cancel.is_triggered()
    }

    /// Resolves on cancellation; lets long work clean up cooperatively.
    pub async fn cancelled(&self) {
        cancelled(&self.cancel, &self.runner_cancel).await;
    }
}

async fn cancelled(task: &Shutdown, runner: &Shutdown) {
    tokio::select! {
        () = task.wait() => {}
        () = runner.wait() => {}
    }
}

/// Handle to a spawned task. Dropping it does **not** cancel the task.
#[derive(Debug)]
pub struct TaskHandle<T> {
    id: TaskId,
    name: Arc<str>,
    status: watch::Receiver<TaskStatus>,
    cancel: Shutdown,
    join: JoinHandle<Result<T, TaskFailure>>,
}

impl<T> TaskHandle<T> {
    /// Task ID.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Task name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current status.
    pub fn status(&self) -> TaskStatus {
        self.status.borrow().clone()
    }

    /// Requests cancellation. Idempotent.
    pub fn cancel(&self) {
        self.cancel.trigger();
    }

    /// Waits for the final outcome.
    pub async fn join(self) -> Result<T, TaskFailure> {
        match self.join.await {
            Ok(outcome) => outcome,
            Err(err) if err.is_panic() => Err(TaskFailure::Panicked),
            Err(_) => Err(TaskFailure::Cancelled),
        }
    }
}

/// Result of [`TaskRunner::shutdown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    /// `true` if every task finished within the grace period.
    pub drained: bool,
    /// Tasks still active when the grace period ended (and were cancelled).
    pub cancelled: usize,
}

struct Inner {
    permits: Arc<Semaphore>,
    default_timeout: Duration,
    default_retry: RetryPolicy,
    closed: AtomicBool,
    cancel_all: Shutdown,
    active: AtomicUsize,
    idle: Notify,
    next_id: AtomicU64,
}

/// Configures a [`TaskRunner`].
#[derive(Debug, Clone)]
pub struct TaskRunnerBuilder {
    max_concurrency: usize,
    default_timeout: Duration,
    default_retry: RetryPolicy,
}

impl TaskRunnerBuilder {
    /// Maximum attempts running at the same time. Default 16 (minimum 1).
    pub fn max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max.clamp(1, Semaphore::MAX_PERMITS);
        self
    }

    /// Default timeout per attempt. Default 60 s.
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Default retry policy. Default: no retries.
    pub fn default_retry(mut self, retry: RetryPolicy) -> Self {
        self.default_retry = retry;
        self
    }

    /// Builds the runner.
    pub fn build(self) -> TaskRunner {
        TaskRunner {
            inner: Arc::new(Inner {
                permits: Arc::new(Semaphore::new(self.max_concurrency)),
                default_timeout: self.default_timeout,
                default_retry: self.default_retry,
                closed: AtomicBool::new(false),
                cancel_all: Shutdown::new(),
                active: AtomicUsize::new(0),
                idle: Notify::new(),
                next_id: AtomicU64::new(1),
            }),
        }
    }
}

/// Cloneable handle to a local task runner. Clones share limits and tasks.
#[derive(Clone)]
pub struct TaskRunner {
    inner: Arc<Inner>,
}

impl fmt::Debug for TaskRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskRunner")
            .field("active", &self.active())
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl TaskRunner {
    /// Starts configuring a runner.
    pub fn builder() -> TaskRunnerBuilder {
        TaskRunnerBuilder {
            max_concurrency: 16,
            default_timeout: Duration::from_secs(60),
            default_retry: RetryPolicy::none(),
        }
    }

    /// A runner with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a task. `task` is called once per attempt.
    ///
    /// Must be called within a Tokio runtime.
    pub fn spawn<T, F, Fut>(&self, spec: TaskSpec, task: F) -> Result<TaskHandle<T>, SpawnError>
    where
        T: Send + 'static,
        F: Fn(TaskContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, TaskError>> + Send + 'static,
    {
        // Count first, then check `closed`, so `shutdown` never misses a task.
        self.inner.active.fetch_add(1, Ordering::SeqCst);
        if self.inner.closed.load(Ordering::SeqCst) {
            ActiveGuard(self.inner.clone()).release();
            return Err(SpawnError::Closed);
        }
        let id = TaskId(self.inner.next_id.fetch_add(1, Ordering::Relaxed));
        let name: Arc<str> = Arc::from(spec.name.as_ref());
        let (status_tx, status_rx) = watch::channel(TaskStatus::Queued);
        let cancel = Shutdown::new();
        let attempt = Attempt {
            inner: self.inner.clone(),
            id,
            name: name.clone(),
            timeout: spec.timeout.unwrap_or(self.inner.default_timeout),
            retry: spec.retry.unwrap_or(self.inner.default_retry),
            cancel: cancel.clone(),
        };
        let span = tracing::info_span!("task", task.id = %id, task.name = %name);
        let join = tokio::spawn(attempt.run(task, status_tx).instrument(span));
        Ok(TaskHandle {
            id,
            name,
            status: status_rx,
            cancel,
            join,
        })
    }

    /// Tasks spawned and not yet finished.
    pub fn active(&self) -> usize {
        self.inner.active.load(Ordering::SeqCst)
    }

    /// `true` once [`TaskRunner::close`] or [`TaskRunner::shutdown`] was called.
    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::SeqCst)
    }

    /// Stops accepting new tasks; running tasks continue.
    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::SeqCst);
    }

    /// Closes the runner, waits up to `grace` for active tasks to finish, then
    /// cancels whatever is left and waits up to `grace` again for it to stop.
    pub async fn shutdown(&self, grace: Duration) -> ShutdownReport {
        self.close();
        if tokio::time::timeout(grace, self.wait_idle()).await.is_ok() {
            return ShutdownReport {
                drained: true,
                cancelled: 0,
            };
        }
        let cancelled = self.active();
        tracing::warn!(
            cancelled,
            "task runner grace period elapsed; cancelling remaining tasks"
        );
        self.inner.cancel_all.trigger();
        let _ = tokio::time::timeout(grace, self.wait_idle()).await;
        ShutdownReport {
            drained: false,
            cancelled,
        }
    }

    async fn wait_idle(&self) {
        loop {
            let notified = std::pin::pin!(self.inner.idle.notified());
            let mut notified = notified;
            notified.as_mut().enable();
            if self.active() == 0 {
                return;
            }
            notified.await;
        }
    }
}

struct ActiveGuard(Arc<Inner>);

impl ActiveGuard {
    fn release(self) {
        drop(self);
    }
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if self.0.active.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.0.idle.notify_waiters();
        }
    }
}

/// Publishes status; reports `Panicked`/`Cancelled` if the task future ends abnormally.
struct StatusGuard {
    tx: watch::Sender<TaskStatus>,
    finished: bool,
}

impl StatusGuard {
    fn set(&self, status: TaskStatus) {
        self.tx.send_replace(status);
    }

    fn finish<T>(
        &mut self,
        status: TaskStatus,
        outcome: Result<T, TaskFailure>,
    ) -> Result<T, TaskFailure> {
        self.tx.send_replace(status);
        self.finished = true;
        outcome
    }
}

impl Drop for StatusGuard {
    fn drop(&mut self) {
        if !self.finished {
            let status = if std::thread::panicking() {
                TaskStatus::Panicked
            } else {
                TaskStatus::Cancelled
            };
            self.tx.send_replace(status);
        }
    }
}

struct Attempt {
    inner: Arc<Inner>,
    id: TaskId,
    name: Arc<str>,
    timeout: Duration,
    retry: RetryPolicy,
    cancel: Shutdown,
}

impl Attempt {
    async fn run<T, F, Fut>(
        self,
        task: F,
        status_tx: watch::Sender<TaskStatus>,
    ) -> Result<T, TaskFailure>
    where
        F: Fn(TaskContext) -> Fut,
        Fut: Future<Output = Result<T, TaskError>>,
    {
        let _active = ActiveGuard(self.inner.clone());
        let mut status = StatusGuard {
            tx: status_tx,
            finished: false,
        };
        let max_attempts = self.retry.max_attempts();
        let runner_cancel = &self.inner.cancel_all;
        let mut attempt = 0;

        loop {
            attempt += 1;
            let permit = tokio::select! {
                biased;
                () = cancelled(&self.cancel, runner_cancel) => {
                    return status.finish(TaskStatus::Cancelled, Err(TaskFailure::Cancelled));
                }
                permit = self.inner.permits.clone().acquire_owned() => permit.ok(),
            };
            status.set(TaskStatus::Running { attempt });
            let ctx = TaskContext {
                id: self.id,
                name: self.name.clone(),
                attempt,
                cancel: self.cancel.clone(),
                runner_cancel: runner_cancel.clone(),
            };
            let outcome = tokio::select! {
                biased;
                () = cancelled(&self.cancel, runner_cancel) => {
                    tracing::info!(attempt, "task cancelled");
                    return status.finish(TaskStatus::Cancelled, Err(TaskFailure::Cancelled));
                }
                outcome = tokio::time::timeout(self.timeout, task(ctx)) => outcome,
            };
            drop(permit);

            match outcome {
                Ok(Ok(value)) => {
                    tracing::debug!(attempt, "task succeeded");
                    return status.finish(TaskStatus::Succeeded, Ok(value));
                }
                Ok(Err(error)) if !error.is_transient() || attempt >= max_attempts => {
                    tracing::warn!(attempt, error = %error, "task failed");
                    return status.finish(
                        TaskStatus::Failed,
                        Err(TaskFailure::Failed {
                            attempts: attempt,
                            error,
                        }),
                    );
                }
                Ok(Err(error)) => {
                    tracing::warn!(attempt, error = %error, "task attempt failed; will retry");
                }
                Err(_) if attempt >= max_attempts => {
                    tracing::warn!(attempt, "task timed out");
                    return status.finish(
                        TaskStatus::TimedOut,
                        Err(TaskFailure::TimedOut { attempts: attempt }),
                    );
                }
                Err(_) => {
                    tracing::warn!(attempt, "task attempt timed out; will retry");
                }
            }

            let delay = self.retry.delay_after(attempt, self.id.0);
            status.set(TaskStatus::Retrying {
                next_attempt: attempt + 1,
                delay,
            });
            tokio::select! {
                biased;
                () = cancelled(&self.cancel, runner_cancel) => {
                    return status.finish(TaskStatus::Cancelled, Err(TaskFailure::Cancelled));
                }
                () = tokio::time::sleep(delay) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    fn fast_retry(attempts: u32) -> RetryPolicy {
        RetryPolicy::exponential(attempts, Duration::from_millis(1)).jitter(false)
    }

    #[tokio::test]
    async fn succeeds_and_reports_status() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(TaskSpec::new("answer"), |_| async {
                Ok::<_, TaskError>(42)
            })
            .unwrap();
        let mut status = handle.status.clone();
        assert_eq!(handle.join().await.unwrap(), 42);
        status.wait_for(TaskStatus::is_terminal).await.unwrap();
        assert_eq!(*status.borrow(), TaskStatus::Succeeded);
        assert_eq!(runner.active(), 0);
    }

    #[tokio::test]
    async fn retries_transient_errors_then_succeeds() {
        let runner = TaskRunner::new();
        let calls = Arc::new(AtomicU32::new(0));
        let counter = calls.clone();
        let handle = runner
            .spawn(TaskSpec::new("flaky").retry(fast_retry(5)), move |ctx| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    if ctx.attempt() < 3 {
                        Err(TaskError::transient("connection reset"))
                    } else {
                        Ok(ctx.attempt())
                    }
                }
            })
            .unwrap();
        assert_eq!(handle.join().await.unwrap(), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn permanent_errors_are_not_retried() {
        let runner = TaskRunner::new();
        let calls = Arc::new(AtomicU32::new(0));
        let counter = calls.clone();
        let handle = runner
            .spawn(TaskSpec::new("bad-input").retry(fast_retry(5)), move |_| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(TaskError::permanent("invalid payload"))
                }
            })
            .unwrap();
        let failure = handle.join().await.unwrap_err();
        assert!(matches!(failure, TaskFailure::Failed { attempts: 1, .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_stop_after_max_attempts() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(TaskSpec::new("down").retry(fast_retry(3)), |_| async {
                Err::<(), _>(TaskError::transient("503"))
            })
            .unwrap();
        assert!(matches!(
            handle.join().await.unwrap_err(),
            TaskFailure::Failed { attempts: 3, .. }
        ));
    }

    #[tokio::test]
    async fn attempts_time_out() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(
                TaskSpec::new("slow")
                    .timeout(Duration::from_millis(10))
                    .retry(fast_retry(2)),
                |_| async {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    Ok::<_, TaskError>(())
                },
            )
            .unwrap();
        assert!(matches!(
            handle.join().await.unwrap_err(),
            TaskFailure::TimedOut { attempts: 2 }
        ));
    }

    #[tokio::test]
    async fn cancel_stops_a_running_task() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(TaskSpec::new("forever"), |_| async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<_, TaskError>(())
            })
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(handle.status(), TaskStatus::Running { attempt: 1 });
        handle.cancel();
        let mut status = handle.status.clone();
        assert!(matches!(handle.join().await, Err(TaskFailure::Cancelled)));
        status.wait_for(TaskStatus::is_terminal).await.unwrap();
        assert_eq!(*status.borrow(), TaskStatus::Cancelled);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrency_is_limited() {
        let runner = TaskRunner::builder().max_concurrency(2).build();
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let running = running.clone();
                let peak = peak.clone();
                runner
                    .spawn(TaskSpec::new(format!("job-{i}")), move |_| {
                        let running = running.clone();
                        let peak = peak.clone();
                        async move {
                            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                            peak.fetch_max(now, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(15)).await;
                            running.fetch_sub(1, Ordering::SeqCst);
                            Ok::<_, TaskError>(())
                        }
                    })
                    .unwrap()
            })
            .collect();
        for handle in handles {
            handle.join().await.unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn panics_are_reported() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(TaskSpec::new("bug"), |_| async {
                panic!("oops");
                #[allow(unreachable_code)]
                Ok::<(), TaskError>(())
            })
            .unwrap();
        let mut status = handle.status.clone();
        assert!(matches!(handle.join().await, Err(TaskFailure::Panicked)));
        status.wait_for(TaskStatus::is_terminal).await.unwrap();
        assert_eq!(*status.borrow(), TaskStatus::Panicked);
        assert_eq!(runner.active(), 0);
    }

    #[tokio::test]
    async fn shutdown_drains_quick_tasks_and_rejects_new_ones() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(TaskSpec::new("quick"), |_| async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<_, TaskError>("done")
            })
            .unwrap();
        let report = runner.shutdown(Duration::from_secs(1)).await;
        assert_eq!(
            report,
            ShutdownReport {
                drained: true,
                cancelled: 0
            }
        );
        assert_eq!(handle.join().await.unwrap(), "done");
        assert!(matches!(
            runner.spawn(TaskSpec::new("late"), |_| async { Ok::<_, TaskError>(()) }),
            Err(SpawnError::Closed)
        ));
    }

    #[tokio::test]
    async fn shutdown_cancels_tasks_that_exceed_the_grace_period() {
        let runner = TaskRunner::new();
        let handle = runner
            .spawn(TaskSpec::new("stuck"), |ctx| async move {
                ctx.cancelled().await;
                Ok::<_, TaskError>(())
            })
            .unwrap();
        let report = runner.shutdown(Duration::from_millis(20)).await;
        assert_eq!(
            report,
            ShutdownReport {
                drained: false,
                cancelled: 1
            }
        );
        assert!(matches!(handle.join().await, Err(TaskFailure::Cancelled)));
    }
}
