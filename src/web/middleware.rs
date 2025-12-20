use tokio::sync::Mutex;

use crate::web::{Request, Resolution};

use std::{pin::Pin, sync::Arc};

pub type MiddlewareFuture = dyn Future<Output = Middleware> + Send;

pub type MiddlewareRequest =
    dyn Fn(Arc<Mutex<Request>>) -> Pin<Box<MiddlewareFuture>> + Send + Sync + 'static;

/// Describes an async function that takes in a request and gives back the Resolution trait.
pub type MiddlewareClosure = Arc<MiddlewareRequest>;

pub type MiddlewareCollection = Vec<MiddlewareClosure>;


/// ## Middleware
/// 
/// Middleware is used to regulate and control the flow of an app's routing. 
/// Allowing for things like authentication, adding values to the request, and many other things.
/// 
/// This removes the overall reptition of creating request.
/// 
/// ### Example
/// 
/// ```
/// let is_admin: MiddlewareClosure = Arc::new(|req: Arc<Mutex< Request>>| Box::pin(async move { 
///
///        //snip
///
///        if is_admin {
///            //or pass any type of resolution
///            //return Middleware::Invalid(EmptyResolution::new(200))
///            return Middleware::InvalidEmpty(403);
///        }
///        Middleware::Next
///    
///    }));
/// ```
/// 
/// The middleware can then be added to an app's routing. 
/// Each middleware is called until all of them return Middleware::Next OR an invalid resolution is provided
/// (in which the invalid resolution is returned).
/// 
/// If all are successful (Next) then the final app endpoint is called. 
pub enum Middleware {
    /// Represents that the middleware failed and cannot move forward towards the resolution.
    ///
    /// Gives a resolution back to the .
    Invalid(Box<dyn Resolution + Send>),

    ///Represents that the middleware failed and cannot move forward towards the resolution.
    ///
    /// Filled with a status code
    InvalidEmpty(i32),

    /// The middleware was a success, move forward towards the request, however you can optionally send a resolution.
    Next,
}
