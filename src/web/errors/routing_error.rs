use std::error::Error;

/// Types of routing errors
#[derive(Debug)]
pub enum RoutingErrorType {
    Exist,
    Missing,
    MethodMissing,
    InvalidRoute(String)
}

/// Represents a routing error in the Routing Tree.
#[derive(Debug)]
pub struct RoutingError {
    pub error_type: RoutingErrorType
}

impl RoutingError {

    pub fn new(e_type: RoutingErrorType) -> Self {
        Self { error_type: e_type }
    }

    pub fn get_error_str(&self) -> String {
        match &self.error_type {
            RoutingErrorType::Exist => "the route already exist".to_string(),
            RoutingErrorType::Missing => "the route does not exist".to_string(),
            RoutingErrorType::MethodMissing => "the route exist, however the requested method for the route does not.".to_string(),
            RoutingErrorType::InvalidRoute(reason) => format!("the route provided was invalid because {reason}")
        }
    }
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = self.get_error_str();
        write!(f, "{err}")
    }
}

impl Error for RoutingError {}