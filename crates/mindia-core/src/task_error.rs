//! Task execution error types
//!
//! This module provides error types specifically for task execution, allowing
//! tasks to indicate whether an error is recoverable (should be retried) or
//! unrecoverable (should fail immediately without retrying).

use std::fmt;

/// Task execution error that can be either recoverable or unrecoverable
#[derive(Debug)]
pub struct TaskError {
    inner: anyhow::Error,
    recoverable: bool,
}

impl TaskError {
    /// Create a new unrecoverable task error
    ///
    /// Unrecoverable errors will cause the task to fail immediately without retrying.
    /// Use this for errors like:
    /// - Missing or invalid configuration (API keys, credentials)
    /// - Invalid input data that won't change on retry
    /// - Authorization/permission errors
    pub fn unrecoverable(err: impl Into<anyhow::Error>) -> Self {
        Self {
            inner: err.into(),
            recoverable: false,
        }
    }

    /// Create a new recoverable task error
    ///
    /// Recoverable errors will be retried according to the task's retry policy.
    /// Use this for errors like:
    /// - Transient network failures
    /// - Temporary resource unavailability
    /// - Rate limiting
    pub fn recoverable(err: impl Into<anyhow::Error>) -> Self {
        Self {
            inner: err.into(),
            recoverable: true,
        }
    }

    /// Check if this error is recoverable (should be retried)
    pub fn is_recoverable(&self) -> bool {
        self.recoverable
    }

    /// Get the inner error
    pub fn inner(&self) -> &anyhow::Error {
        &self.inner
    }

    /// Consume self and return the inner error
    pub fn into_inner(self) -> anyhow::Error {
        self.inner
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::error::Error for TaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

impl From<anyhow::Error> for TaskError {
    /// Default conversion from anyhow::Error creates a recoverable error
    fn from(err: anyhow::Error) -> Self {
        Self::recoverable(err)
    }
}

// Note: From<TaskError> for anyhow::Error is automatically implemented by anyhow
// via its blanket implementation for any type that implements std::error::Error

/// Extension trait for Result to easily create unrecoverable task errors
pub trait TaskResultExt<T> {
    /// Mark this result as unrecoverable on error
    fn unrecoverable(self) -> Result<T, TaskError>;
}

impl<T, E: Into<anyhow::Error>> TaskResultExt<T> for Result<T, E> {
    fn unrecoverable(self) -> Result<T, TaskError> {
        self.map_err(|e| TaskError::unrecoverable(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unrecoverable_error() {
        let err = TaskError::unrecoverable(anyhow::anyhow!("Missing API key"));
        assert!(!err.is_recoverable());
        assert!(err.to_string().contains("Missing API key"));
    }

    #[test]
    fn test_recoverable_error() {
        let err = TaskError::recoverable(anyhow::anyhow!("Network timeout"));
        assert!(err.is_recoverable());
        assert!(err.to_string().contains("Network timeout"));
    }

    #[test]
    fn test_from_anyhow() {
        let err: TaskError = anyhow::anyhow!("Some error").into();
        assert!(err.is_recoverable(), "Default should be recoverable");
    }

    #[test]
    fn test_result_ext() {
        let result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("Config error"));
        let task_result = result.unrecoverable();
        assert!(task_result.is_err());
        assert!(!task_result.unwrap_err().is_recoverable());
    }
}
