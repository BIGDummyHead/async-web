use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::web::{EndPoint, Method, router::RouteNodeRef};

pub struct RouteNode {
    // The ID of the node, usually part of a larger string. Ex. api/admin/users -> ID's may be (api, admin, users)
    pub id: String,

    /// A map of resolutions, used to find the function to call for a request. Only one func may exist per Method for THIS node.
    pub resolutions: HashMap<Method, Arc<EndPoint>>,

    /// Is Variable
    pub is_var: bool,

    /// The children of this node.
    ///
    /// Assume that the node is part of a tree for ["api/admin/users", "api/partner/users", "api/agency/users"] and this node is "api"
    ///
    /// The children of this node would be ["admin", "partner", "agency"]
    pub children: HashMap<String, RouteNodeRef>,

    /// The variable based child for this route node.
    pub var_child: Option<RouteNodeRef>,

    pub parent: Option<RouteNodeRef>,
}

/// A node from a Route Tree
impl RouteNode {
    /// Create a new node, simply takes an ID (part of a url) and an optional resolution.
    pub fn new(id: String, resolution: Option<(Method, EndPoint)>) -> Self {
        let mut resolutions = HashMap::new();

        if let Some((method, end_point)) = resolution {
            resolutions.insert(method, Arc::new(end_point));
        }

        let is_var = id.starts_with("{") && id.ends_with("}");
        Self {
            id,
            resolutions,
            is_var,
            children: HashMap::new(),
            var_child: None,
            parent: None,
        }
    }

    /// Borrow the current resolution for a method.
    pub fn get_resolution(&self, method: &Method) -> Option<Arc<EndPoint>> {
        match self.resolutions.get(method) {
            None => None,
            Some(v) => Some(v.clone())
        }
    }

    /// Borrow a child of the node. None if not present.
    pub fn get_child(&self, id: &str) -> Option<RouteNodeRef> {
        self.children.get(id).cloned()
    }

    /// Insert a resolution for the node. Replaces the current resolution for the method if it already exist.
    pub fn insert_resolution(&mut self, method: Method, endpoint: EndPoint) -> () {
        self.resolutions.insert(method, Arc::new(endpoint));
    }

    /// Add a child to this node. Same as using the new function but directly adds to this node.
    pub async fn add_child(
        parent_ref: RouteNodeRef,
        id: String,
        endpoint: Option<(Method, EndPoint)>,
    ) -> RouteNodeRef {
        let mut parent = parent_ref.lock().await;

        let mut node = Self::new(id.clone(), endpoint);
        node.parent = Some(parent_ref.clone());

        let node_ref = Arc::new(Mutex::new(node));

        if id.starts_with("{") && id.ends_with("}") {
            parent.var_child = Some(node_ref.clone());
        } else {
            parent.children.insert(id.clone(), node_ref.clone());
        }

        return node_ref;
    }
}
