use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::web::{EndPoint, Method};
use crate::web::routing::RouteNodeRef;

/// # Is Variable Id
/// 
/// Takes a reference to a string and checks for a pattern on the string that:
/// 
/// true -> when the ID is of a variable type
/// false -> when the ID is not of a variable type
fn is_variable_id(id: &String) -> bool {
    id.starts_with("{") && id.ends_with("}")
}

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
    
    /// # New
    /// 
    /// Creates a new route node struct, takes an ID (part of a URL), takes a Optional Method and Endpoint tuple.
    pub fn new(id: String, resolution: Option<(Method, EndPoint)>) -> Self {
        let mut resolutions = HashMap::new();

        if let Some((method, end_point)) = resolution {
            resolutions.insert(method, Arc::new(end_point));
        }

        let is_var = is_variable_id(&id);

        Self {
            id,
            resolutions,
            is_var,
            children: HashMap::new(),
            var_child: None,
            parent: None,
        }
    }

    /// # Borrow Resolution
    /// 
    /// Borrows a resolution from the resolutions map.
    /// 
    /// None -> if the resolution for the given method does not exist.
    /// 
    /// Some -> If the resolution exist for the given method. Clones the Arc
    pub fn brw_resolution(&self, method: &Method) -> Option<Arc<EndPoint>> {
        self.resolutions.get(method).map(|r| r.clone())
    }

    /// # Borrow Child
    /// 
    /// Borrows a child from the route node ref 
    /// 
    /// None -> If the child does not exist
    /// 
    /// Some -> If the child with the ID exist.
    pub fn brw_child(&self, id: &str) -> Option<RouteNodeRef> {
        self.children.get(id).cloned()
    }

    /// # Insert Resolution
    /// 
    /// Inserts a resolution to an existing route node.
    pub fn insert_resolution(&mut self, method: Method, endpoint: EndPoint) -> () {
        self.resolutions.insert(method, Arc::new(endpoint));
    }

    /// # Add Child
    /// 
    /// Takes the parent reference node, has an ID for the route name, and an optional endpoint.
    /// 
    /// This directly adds the node to the parent reference. 
    pub async fn add_child(
        parent_ref: RouteNodeRef,
        id: String,
        endpoint: Option<(Method, EndPoint)>,
    ) -> RouteNodeRef {

        //create a new node
        let mut node = Self::new(id.clone(), endpoint);
        node.parent = Some(parent_ref.clone());

        //create a new ARC for the node with mutex wrapper. 
        //immediately clone it for the children
        let node_ref = Arc::new(Mutex::new(node));
        let node_ref_clone = node_ref.clone();

        let mut parent = parent_ref.lock().await;

        if is_variable_id(&id) {
            parent.var_child = Some(node_ref_clone);
        } else {
            parent.children.insert(id, node_ref_clone);
        }

        return node_ref;
    }
}
