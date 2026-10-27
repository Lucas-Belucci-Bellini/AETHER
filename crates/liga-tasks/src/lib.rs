//! # liga-tasks
//!
//! A local (in-process) runner for asynchronous work with explicit limits:
//!
//! - **Concurrency:** at most `max_concurrency` attempts run at once; the rest
//!   wait in [`TaskStatus::Queued`]. Waiting for a retry does not hold a slot.
//! - **Timeout per attempt:** a timed-out attempt counts as a transient failure.
//! - **Bounded retries:** only [`TaskError::transient`] errors and timeouts are
//!   retried, with exponential backoff and jitter ([`RetryPolicy`]).
//! - **Cancellation:** [`TaskHandle::cancel`], or [`TaskRunner::shutdown`]
//!   for everything. Cancelled work is dropped at its next `.await`.
//! - **Observability:** every task runs in a `task` tracing span and exposes
//!   its [`TaskStatus`].
//!
//! Tasks are **not durable**: if the process stops, queued and running tasks
//! are lost. Use a persistent queue for work that must survive restarts.

mod error;
mod retry;
mod runner;

pub use error::{SpawnError, TaskError, TaskFailure};
pub use retry::RetryPolicy;
pub use runner::{
    ShutdownReport, TaskContext, TaskHandle, TaskId, TaskRunner, TaskRunnerBuilder, TaskSpec,
    TaskStatus,
};
