use std::error::Error;

/// Resolution errors when a resolution is called.
#[derive(Debug)]
pub enum ResolutionError {
    /// Could not resolve the resolution
    CouldNotResolve(String),
    /// There was another reason that the resolution could not resolve.
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