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

        let mut end_point = end_point;

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

        while let Some(rte_part) = route_parts.next() {
            if rte_part.is_empty() {
                continue;
            }

            //checks if this the last element in the iteration
            let is_last = route_parts.peek().is_none();

            //checks if the node has a child for the rte_part
            let has_child = {
                let node_lock = node.lock().await;
                node_lock.children.contains_key(rte_part)
            };

            //check if the child on this route exist.
            if has_child {
                //clone the nnode values
                let node_clone = node.clone();
                let brw_node = node_clone.lock().await;

                //omsert the endpoint to the route, then return ok(), since this is the last item
                if is_last {
                    //check if there is an endpoint to add
                    if let Some((m, r)) = end_point {
                        brw_node
                            .brw_child(rte_part)
                            .unwrap()
                            .lock()
                            .await
                            .insert_resolution(m, r);
                    }
                    return Ok(());
                }

                //if not the last, brw the child and clone for next iteration
                let child = brw_node.brw_child(rte_part).unwrap();
                node = child.clone();

                continue;
            }

            //get element for adding.
            let rte_str = rte_part.to_string();
            let node_clone = node.clone();

            // gets the endpoint if is last and the endpoint is some
            let end_point = if is_last { end_point.take() } else { None };

            //add the route
            let added = RouteNode::add_child(node_clone, rte_str, end_point).await;

            //last route to add, ok to return
            if is_last {
                return Ok(());
            }

            //node was note last next iteration
            node = added;

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

            let mut child = brw_node.brw_child(route_part);

            //do a check to ensure that there is no var child we are missing.
            if child.is_none() {
                //nothing further to do
                if brw_node.var_child.is_none() {
                    return None;
                }

                let var_child_node = brw_node
                    .var_child
                    .as_ref()
                    .map(|r_node| r_node.clone())
                    .unwrap();

                let is_wild_card = {
                    let node_in = var_child_node.lock().await;
                    node_in.id.eq("{*}")
                };

                child = Some(var_child_node);

                //wild carded
                if is_wild_card {
                    return child;
                }
            }

            current_node = child;
        }

        return current_node;
    }
}
