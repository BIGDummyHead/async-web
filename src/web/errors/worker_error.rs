use std::error::Error;

#[derive(Debug)]
pub enum WorkerErrorType {
    AlreadyRunning
}

#[derive(Debug)]
pub struct WorkerError {
    err_type: WorkerErrorType
}

impl WorkerError {
    pub fn new(err_type: WorkerErrorType) -> Self {
        Self { err_type }
    }
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match &self.err_type {
            WorkerErrorType::AlreadyRunning => "the worker was already running"
        };

        write!(f, "{err}")
    }
}


impl Error for WorkerError {}

