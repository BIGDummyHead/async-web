pub mod route_node;
pub mod route_tree;

use std::{pin::Pin, sync::Arc};

use tokio::sync::Mutex;

use crate::web::{Request, Resolution};

pub type ResolutionFuture = dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send;

pub type RequestFunction = dyn Fn(Arc<Mutex<Request>>) -> Pin<Box<ResolutionFuture>> + Send + Sync + 'static;

/// Describes an async function that takes in a request and gives back the Resolution trait.
pub type ResolutionFunc = Arc<RequestFunction>;

pub type RouteNodeRef = Arc<Mutex<RouteNode>>;


pub use self::{route_node::RouteNode, route_tree::RouteTree};