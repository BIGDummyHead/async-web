use crate::web::{middleware::MiddlewareCollection, router::ResolutionFunc};

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
    pub resolution: ResolutionFunc
}

impl EndPoint {
    pub fn new(resolution: ResolutionFunc, middleware: Option<MiddlewareCollection>) -> Self {
        Self {
            middleware,
            resolution
        }
    }
}