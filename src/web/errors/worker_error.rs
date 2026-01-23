use std::error::Error;


/// # Worker Error Type
/// 
/// Resolves into a worker error type
/// 
/// Notably implements:
/// Debug, Into (Result<T, WorkerErrorType>), Display and Error
#[derive(Debug)]
pub enum WorkerError {
    /// While trying to start the worker, it was already running.
    AlreadyRunning, 
    /// While trying to stop the worker, it was already closed.
    AlreadyClosed,

    /// There worker had no task to close
    NoTaskRunning,

    /// When joining incoming task, the join result failed
    TaskJoinFailure,
}

impl<T> Into<Result<T, WorkerError>> for WorkerError {
    /// Transform the worker error type into a worker error.
    fn into(self) -> Result<T, WorkerError> {
        Err(self)
    }
}


impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match &self {
            WorkerError::AlreadyRunning => "the worker was running",
            WorkerError::AlreadyClosed => "the worker was closed",
            WorkerError::NoTaskRunning => "no task running",
            WorkerError::TaskJoinFailure => "when joining task, join result failed"
        };

        write!(f, "{err}")
    }
}


impl Error for WorkerError {}

