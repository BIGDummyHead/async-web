use std::collections::HashMap;

/// A route from a request
#[derive(Debug)]
pub struct Route {
    /// The full route given
    pub init_route: String,
    /// Any params within the route/
    params: HashMap<String, String>,
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.init_route)
    }
}

impl Route {
    fn parse_route(r: &String) -> HashMap<String, String> {
        let mut parsed = HashMap::new();

        /*
           /admin/api/test?v=
        */
        let route_items = r.split("/");

        for item in route_items {
            // admin or api or test?x=y&z=x

            let has_params = item.split_once("?");

            if has_params.is_none() {
                continue;
            }

            let (_, params) = has_params.unwrap();

            let param_items = params.split("&");

            for param_item in param_items {
                let opt_p = param_item.split_once("=");

                if opt_p.is_none() {
                    continue;
                }

                let (key, val) = opt_p.unwrap();

                parsed.insert(String::from(key), String::from(val));
            }
        }

        return parsed;
    }

    pub fn new(init_route: String) -> Self {
        let params = Self::parse_route(&init_route);

        Self { params, init_route }
    }

    pub fn get_param(&self, param_name: &String) -> Option<&String> {
        self.params.get(param_name)
    }

    pub fn get_params(&self) -> &HashMap<String, String> {
        &self.params
    }
}
