use core::panic;
use std::{collections::HashMap, pin::Pin, sync::Arc};

use crate::web::{
    Method, Request, Resolution,
    errors::{RoutingError, routing_error::RoutingErrorType},
};

/// Describes an async function that takes in a request and gives back the Resolution trait.
pub type ResolutionFunc = Arc<
    dyn Fn(Request) -> Pin<Box<dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send>>
        + Send
        + Sync
        + 'static,
>;

pub struct RouteNode {
    // The ID of the node, usually part of a larger string. Ex. api/admin/users -> ID's may be (api, admin, users)
    id: String,
    /// A map of resolutions, used to find the function to call for a request. Only one func may exist per Method for THIS node.
    resolutions: HashMap<Method, ResolutionFunc>,
    /// Is Variable
    // TODO: Implement is_var
    is_var: bool,
    /// The children of this node.
    ///
    /// Assume that the node is part of a tree for ["api/admin/users", "api/partner/users", "api/agency/users"] and this node is "api"
    ///
    /// The children of this node would be ["admin", "partner", "agency"]
    children: HashMap<String, RouteNode>,
}

/// A node from a Route Tree
impl RouteNode {
    /// Create a new node, simply takes an ID (part of a url) and an optional resolution.
    pub fn new(id: String, resolution: Option<(Method, ResolutionFunc)>) -> Self {
        let mut resolutions = HashMap::new();

        if let Some((m, r)) = resolution {
            resolutions.insert(m, r);
        }

        let is_var = id.starts_with("{") && id.starts_with("}");
        Self {
            id,
            resolutions,
            is_var,
            children: HashMap::new(),
        }
    }

    /// Borrow the current resolution for a method.
    pub fn get_resolution(&self, method: &Method) -> Option<&ResolutionFunc> {
        self.resolutions.get(method)
    }

    /// Borrow a child of the node. None if not present.
    pub fn get_child(&self, id: String) -> Option<&RouteNode> {
        self.children.get(&id)
    }

    /// Borrow a mutable child of the node. None if not present.
    pub fn get_child_as_mut(&mut self, id: String) -> Option<&mut RouteNode> {
        self.children.get_mut(&id)
    }

    /// Insert a resolution for the node. Replaces the current resolution for the method if it already exist.
    pub fn insert_resolution(&mut self, method: Method, resolution: ResolutionFunc) -> () {
        self.resolutions.insert(method, resolution);
    }

    /// Add a child to this node. Same as using the new function but directly adds to this node.
    pub fn add_child(
        &mut self,
        id: String,
        resolution: Option<(Method, ResolutionFunc)>,
    ) -> &mut RouteNode {
        let node = Self::new(id.clone(), resolution);

        self.children.insert(id.clone(), node);

        if let Some(n) = self.children.get_mut(&id) {
            return n;
        }

        panic!("The value did not exist after insertion.");
    }
}

///Binary type tree that takes in parts of a route and ends up at a final function.
pub struct RouteTree {
    /// Route node for /
    pub root: RouteNode,

    ///404 node
    pub missing_route: Option<RouteNode>,
}

/// Routing Tree that holds information about resolutions for all your routes.
impl RouteTree {

    /// Create a new route tree with a resolution. Usually a GET
    pub fn new(base_resolution: Option<(Method, ResolutionFunc)>) -> Self {
        let root = RouteNode::new("/".to_string(), base_resolution);

        Self {
            root,
            missing_route: None,
        }
    }

    /// Add a 404 resolution
    pub fn add_missing_route(&mut self, resolution: ResolutionFunc) -> () {

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
    /// ```
    pub fn add_route(
        &mut self,
        route: &str,
        resolution: Option<(Method, ResolutionFunc)>,
    ) -> Result<(), RoutingError> {
        if route.is_empty() {
            return Err(RoutingError::new(RoutingErrorType::InvalidRoute(
                "empty".to_string(),
            )));
        }

        if route == "/" {
            if let Some((m, r)) = resolution {
                self.root.insert_resolution(m, r);
                return Ok(());
            }

            return Err(RoutingError::new(RoutingErrorType::MethodMissing));
        }

        let full_route = route.to_string();

        //break the route into digestable parts (nodes)
        let mut route_parts = full_route.split("/").peekable();

        let mut node = &mut self.root;

        while let Some(rte) = route_parts.next() {
            if rte.is_empty() {
                continue;
            }

            let route_part = rte.to_string();

            let is_last = route_parts.peek().is_none();

            //there is a child on this node and it is the last add it
            if node.children.contains_key(&route_part) {
                //insert the resolution since it exists
                if is_last {
                    if let Some((m, r)) = resolution {
                        node.get_child_as_mut(route_part)
                            .unwrap()
                            .insert_resolution(m, r);
                    }
                    return Ok(());
                }

                // ! must come second to avoid overuse of borrow
                let child = node.get_child_as_mut(route_part).unwrap();
                node = child;
            } else {
                //there is no child, we must now add it to the current node
                if is_last {
                    node.add_child(route_part, resolution);
                    return Ok(());
                }

                node = node.add_child(route_part, None);
            }
        }

        Ok(())
    }

    /// Borrow an existing mutable route.
    pub fn get_mut_route(&mut self, full_route: &str) -> Option<&mut RouteNode> {
        //start with the root and work our way down
        let mut current_node = Some(&mut self.root);

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

            let child = node.get_child_as_mut(route_part.to_string());

            current_node = child;
        }

        return current_node;
    }

    /// Borrow an existing route.
    pub fn get_route(&self, full_route: &str) -> Option<&RouteNode> {
        //start with the root and work our way down
        let mut current_node = Some(&self.root);

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

            let child = node.get_child(route_part.to_string());

            current_node = child;
        }

        return current_node;
    }
}
