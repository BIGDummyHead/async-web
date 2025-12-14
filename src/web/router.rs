use std::{collections::HashMap, sync::Arc};


use tokio::sync::Mutex;

use crate::web::{Method, RouteInfo, method};

pub struct Router {
    routes: Arc<Mutex<HashMap<String, HashMap<Method, RouteInfo>>>>,
     //TODO Add middleware.
}

impl Router {

    fn new() -> Self {
        let routes = Arc::new(Mutex::new(HashMap::new()));

        Self {
            routes
        }
    }

    pub async fn add_route(&self, route: RouteInfo) -> bool {
        let rs = self.routes.lock().await;

        let existing_map = rs.get(&route.route);

        if let Some(map) = existing_map {
            //map.contains_key(&route.method)
        }

        todo!()
    }
}
