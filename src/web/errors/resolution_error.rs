use std::error::Error;

/// Types of routing errors
#[derive(Debug)]
pub enum ResolutionError {
    CouldNotResolve(String),
    Other(String)
}

impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        
        let err = match self {
            ResolutionError::CouldNotResolve(reason) => reason,
            ResolutionError::Other(r) => r
        };

        write!(f, "{err}")
    }
}

impl Error for ResolutionError {}