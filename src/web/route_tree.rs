use std::{cell::RefCell, collections::HashMap, pin::Pin, rc::Rc, task::Context};

use crate::web::{Method, Request, Resolution};

pub type ResolutionFunc = Box<
    dyn Fn(Request) -> Pin<Box<dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send>>
        + Send
        + Sync
        + 'static
>;


pub struct RouteNode {
    // part of the route, /admin/api (api or admin) would be the id
    id: String,
    resolutions: HashMap<Method, ResolutionFunc>,
    is_var: bool,
    children: HashMap<String, RouteNode>,
}

impl RouteNode {
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

    pub fn get_resolution(&self, method: &Method) -> Option<&ResolutionFunc> {
        self.resolutions.get(method)
    }

    pub fn get_child(&self, id: String) -> Option<&RouteNode> {
        self.children.get(&id)
    }

    pub fn get_child_as_mut(&mut self, id: String) -> Option<&mut RouteNode> {
        self.children.get_mut(&id)
    }

    pub fn insert_resolution(&mut self, method: Method, resolution: ResolutionFunc) -> () {
        self.resolutions.insert(method, resolution);
    }

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
}

impl RouteTree {
    pub fn new(base_resolution: Option<(Method, ResolutionFunc)>) -> Self {
        let root = RouteNode::new("/".to_string(), base_resolution);

        Self { root }
    }

    pub fn add_route(
        &mut self,
        route: &str,
        resolution: Option<(Method, ResolutionFunc)>,
    ) -> Result<(), ()> {

        let full_route = route.to_string();

        let mut route_parts = full_route.split("/").peekable();

        let mut node = &mut self.root;

        while let Some(route_part) = route_parts.next() {
            if route_part.is_empty() {
                continue;
            }

            let is_last = route_parts.peek().is_none();

            //there is a child on this node and it is the last add it
            if node.children.contains_key(&route_part.to_string()) {
                let child = node.get_child_as_mut(route_part.to_string()).unwrap();
                if is_last {
                    child.add_child(route_part.to_string(), resolution);
                    return Ok(());
                }
                node = child;
            } else {
                //there is no child, we must now add it to the current node
                if is_last {
                    node.add_child(route_part.to_string(), resolution);
                    return Ok(());
                }

                node = node.add_child(route_part.to_string(), None);
            }
        }

        todo!();
    }

    pub fn get_route(&mut self, full_route: &str) -> Option<&mut RouteNode> {
        //they just want the base...
        if full_route == "/" {
            return Some(&mut self.root);
        }

        let mut current_node = Some(&mut self.root);

        let route_parts = full_route.split("/");

        for route_part in route_parts {
            if route_part.is_empty() {
                continue;
            }

            if let None = current_node {
                return None;
            }

            let node = current_node.unwrap();

            let child = node.get_child_as_mut(route_part.to_string());

            current_node = child;
        }

        return current_node;
    }
}
