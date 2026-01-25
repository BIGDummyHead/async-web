pub mod method;
pub mod middleware;
pub mod request;
pub mod route;
pub mod router;

pub use super::resolution::Resolution;
pub use method::Method;
pub use middleware::Middleware;
pub use request::Request;
pub use route::Route;

use std::{pin::Pin, sync::Arc};

use tokio::sync::Mutex;

use crate::web::{routing::router::route_node::RouteNode};

//highest
/// # Resolution Future
/// 
/// A future whose output is the resolution that the browser should be served.
/// 
/// ```
/// //although this is not valid code...
/// let res_fut: ResolutionFuture = async move {
///     EmptyResolution::status(200).resolve()
/// };
/// ```
pub type ResolutionFuture = dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send;

/// # Resolution Function (FN)
/// 
/// ```
/// //although this is not valid code...
/// let res_fut: ResolutionFn = |req: Arc<Mutex<Request>>| {
///     Box::pin(async move {
///         EmptyResolution::status(200).resolve()
///     })
/// };
/// ```
pub type ResolutionFn =
    dyn Fn(Arc<Mutex<Request>>) -> Pin<Box<ResolutionFuture>> + Send + Sync + 'static;

/// # Resolution Function (FN) Ref
/// 
/// ```
/// let res_fut: ResolutionFnRef = Arc::new(|req: Arc<Mutex<Request>>| {
///     Box::pin(async move {
///         EmptyResolution::status(200).resolve()
///     })
/// });
/// 
/// ```
pub type ResolutionFnRef = Arc<ResolutionFn>;

pub type RouteNodeRef = Arc<Mutex<RouteNode>>;
