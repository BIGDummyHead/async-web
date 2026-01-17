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

pub type ResolutionFuture = dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send;

pub type RequestFunction =
    dyn Fn(Arc<Mutex<Request>>) -> Pin<Box<ResolutionFuture>> + Send + Sync + 'static;

/// Describes an async function that takes in a request and gives back the Resolution trait.
pub type ResolutionFunc = Arc<RequestFunction>;

pub type RouteNodeRef = Arc<Mutex<RouteNode>>;
