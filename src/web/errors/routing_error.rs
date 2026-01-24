use std::error::Error;

/// # routing error
/// 
/// An error that represents when trying to add, remove, change, or get a route.
#[derive(Debug)]
pub enum RoutingError {
    Exist,
    Missing,
    MethodMissing,
    InvalidRoute(String)
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match &self {
            RoutingError::Exist => "the route already exist".to_string(),
            RoutingError::Missing => "the route does not exist".to_string(),
            RoutingError::MethodMissing => "the route exist, however the requested method for the route does not.".to_string(),
            RoutingError::InvalidRoute(reason) => format!("the route provided was invalid because {reason}")
        };
        write!(f, "{err}")
    }
}

impl Error for RoutingError {}