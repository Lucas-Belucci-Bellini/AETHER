use std::borrow::Cow;
use std::error::Error as StdError;

/// Error returned by a task attempt. Decides whether the runner may retry.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct TaskError {
    transient: bool,
    message: Cow<'static, str>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl TaskError {
    /// A failure that may succeed if retried (network blip, rate limit, lock timeout).
    pub fn transient(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            transient: true,
            message: message.into(),
            source: None,
        }
    }

    /// A failure that retrying will not fix (bad input, missing resource).
    pub fn permanent(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            transient: false,
            message: message.into(),
            source: None,
        }
    }

    /// Attaches the underlying cause.
    pub fn with_source(mut self, source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// `true` when the runner is allowed to retry.
    pub fn is_transient(&self) -> bool {
        self.transient
    }
}

/// Final outcome of a task that did not succeed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TaskFailure {
    /// The last attempt returned an error (permanent, or retries exhausted).
    #[error("task failed after {attempts} attempt(s): {error}")]
    Failed {
        /// Attempts made.
        attempts: u32,
        /// Error of the last attempt.
        #[source]
        error: TaskError,
    },
    /// The last attempt exceeded its timeout.
    #[error("task timed out after {attempts} attempt(s)")]
    TimedOut {
        /// Attempts made.
        attempts: u32,
    },
    /// The task was cancelled (individually or by runner shutdown).
    #[error("task was cancelled")]
    Cancelled,
    /// The task panicked.
    #[error("task panicked")]
    Panicked,
}

/// The runner refused a new task.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SpawnError {
    /// The runner is closed or shutting down.
    #[error("the task runner is shutting down and accepts no new tasks")]
    Closed,
}
