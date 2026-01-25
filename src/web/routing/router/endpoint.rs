use crate::web::routing::{ResolutionFnRef, middleware::MiddlewareCollection};


/// ## End Point
/// Represents an Endpoint of a Route Tree node. 
/// 
/// The endpoint contains two major items. 
/// 
/// #### MiddlewareCollection (optional)
/// 
/// A collection of middleware that is checked.
/// 
/// #### A resolution
/// 
/// The resolution that is called once the middleware has completed.
pub struct EndPoint {
    pub middleware: Option<MiddlewareCollection>,
    pub resolution: ResolutionFnRef
}

impl EndPoint {
    pub fn new(resolution: ResolutionFnRef, middleware: Option<MiddlewareCollection>) -> Self {
        Self {
            middleware,
            resolution
        }
    }
}