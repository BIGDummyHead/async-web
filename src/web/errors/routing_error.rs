/// # routing error
/// 
/// An error that represents when trying to add, remove, change, or get a route.
#[derive(Debug)]
pub enum RoutingError {
    Exist,
    Missing,
    MethodMissing,
    InvalidRoute(String),
    NoRouteExist
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match &self {
            RoutingError::Exist => "the route already exist",
            RoutingError::Missing => "the route does not exist",
            RoutingError::MethodMissing => "the route exist, however the requested method for the route does not.",
            RoutingError::InvalidRoute(reason) => &format!("the route provided was invalid because {reason}"),
            RoutingError::NoRouteExist => "no route exist"
        };
        write!(f, "{err}")
    }
}

impl std::error::Error for RoutingError {}