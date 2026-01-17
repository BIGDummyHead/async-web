use std::sync::Arc;

use tokio::sync::Mutex;

use crate::web::{
    EndPoint, Method,
    errors::{RoutingError, routing_error::RoutingErrorType},
};

use crate::web::routing::RouteNodeRef;
use crate::web::routing::router::route_node::RouteNode;

/// # Route tree
///
/// Trie based tree that separates a given route into nodes and contains information about their nodes such as:
///
/// * id (the part of the route)
/// * EndPoint (middleware and requested function)
///
/// And Other relevant information about the node.
///
/// To edit the base route (/) you may edit the root variable.
///
/// You may also add a missing route via the "add_missing_route" function or set the missing_route variable.
///
/// #### Adding a Route
///
/// Suppose you would like to add a route to the tree, you may call the add_route function. Please refer to the documentation on the method as it is rather in depth.
///
/// #### Removing a Route
///
/// You cannot remove a Route, this is built on purpose, as Routing for a web application would usually be a STATIC based activity wherein you would not add/remove routing during the runtime.
///
///
/// #### Getting a Route
///
/// Getting a route is straight forward. You may refer to the get_route(&str) function to do so.
///
pub struct RouteTree {
    /// Route node for /
    pub root: RouteNodeRef,

    ///404 node
    pub missing_route: Option<RouteNode>,
}

impl RouteTree {
    /// Create a new route tree with a resolution. Usually a GET
    pub fn new(base_resolution: Option<(Method, EndPoint)>) -> Self {
        let root = RouteNode::new("/".to_string(), base_resolution);

        Self {
            root: Arc::new(Mutex::new(root)),
            missing_route: None,
        }
    }

    /// Add a 404 resolution
    pub fn add_missing_route(&mut self, resolution: EndPoint) -> () {
        let m_node = RouteNode::new("\\_missing_/".to_string(), Some((Method::GET, resolution)));

        self.missing_route = Some(m_node);
    }

    /// Add a route to the tree. Takes in two arguments and an optional resolution.
    /// > The route: "/tasks"
    ///
    /// > A resolution: A method (GET, POST, PUT, etc...) and a function to resolve it.
    ///
    /// ## Example
    ///
    /// ```
    /// let _ = app.get_router().await.add_route(
    ///    "/tasks",
    ///    Some((
    ///        Method::GET,
    ///        Box::new(|req| {
    ///            Box::pin(async move {
    ///                println!("{}", req.route);
    ///                Box::new(FileResolution {
    ///                    file: "tasks.html".to_string(),
    ///                }) as Box<dyn Resolution + Send>
    ///            })
    ///        }),
    ///    )),
    ///);
    ///);
    ///
    /// ```
    pub async fn add_route(
        &mut self,
        route: &str,
        end_point: Option<(Method, EndPoint)>,
    ) -> Result<(), RoutingError> {
        if route.is_empty() {
            return Err(RoutingError::new(RoutingErrorType::InvalidRoute(
                "empty".to_string(),
            )));
        }

        let root = self.root.clone();

        if route == "/" {
            if let Some((m, r)) = end_point {
                root.lock().await.insert_resolution(m, r);
                return Ok(());
            }

            return Err(RoutingError::new(RoutingErrorType::MethodMissing));
        }

        let full_route = route.to_string();

        //break the route into digestable parts (nodes)
        let mut route_parts = full_route.split("/").peekable();

        let mut node = root;

        while let Some(rte) = route_parts.next() {
            if rte.is_empty() {
                continue;
            }

            let route_part = rte.to_string();

            let is_last = route_parts.peek().is_none();

            //there is a child on this node and it is the last add it
            if node.lock().await.children.contains_key(&route_part) {
                let node_clone = node.clone();
                let brw_node = node_clone.lock().await;

                //insert the resolution since it exists
                if is_last {
                    if let Some((m, r)) = end_point {
                        brw_node
                            .get_child(&route_part)
                            .unwrap()
                            .lock()
                            .await
                            .insert_resolution(m, r);
                    }
                    return Ok(());
                }

                let child = brw_node.get_child(&route_part).unwrap();
                node = child.clone();
            } else {
                //there is no child, we must now add it to the current node
                if is_last {
                    RouteNode::add_child(node.clone(), route_part, end_point).await;
                    return Ok(());
                }

                let added = RouteNode::add_child(node.clone(), route_part, None).await;

                node = added;
            }
        }

        Ok(())
    }

    /// # Get Route
    ///
    /// Get an existing route node ref.
    ///
    /// Although you may not wind up using this function as it is used by the router. In where the route is completely sanitized before handed to this.
    ///
    /// Assume we have added a route to get a user by their ID such as "/api/admin/user/{id}"
    ///
    /// To get the route:
    ///
    /// ```
    /// {
    ///     //-- snip --
    ///     // see how we can pass in any variable to the route?
    ///     let opt_route: Option<RouteNodeRef> = tree.get_route("/api/admin/user/12");
    ///
    /// }
    ///
    /// ```
    ///
    /// Since it returns a reference (Arc<Mutex<RouteNode>>) you may lock it and change it via the mutability pattern.
    ///
    pub async fn get_route(&self, full_route: &str) -> Option<RouteNodeRef> {
        //start with the root and work our way down
        let mut current_node = Some(self.root.clone());

        //they just want the base, save time
        if full_route == "/" {
            return current_node;
        }

        //split into node ids
        let route_parts = full_route.split("/");

        for route_part in route_parts {
            if current_node.is_none() {
                return None;
            }

            if route_part.is_empty() {
                continue;
            }

            //safe to move and unwrap from previous is_none() check.
            let node = current_node.unwrap();

            let brw_node = node.lock().await;

            let mut child = brw_node.get_child(route_part);

            if let None = child {
                match &brw_node.var_child {
                    Some(x) => child = Some(x.clone()),
                    None => {
                        return None;
                    }
                }
            }

            current_node = child;
        }

        return current_node;
    }
}
