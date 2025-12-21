use std::collections::HashMap;

/// ## Route
/// 
/// A client provided browser url. Created by parsing the route and then can be used to get the parameters sent by the user and the true URL the user was meaning to fetch.
/// 
/// ### Example
/// 
/// ```
/// let route = Route::parse_route("/test/get-user?name=test".to_string());
/// 
/// ```
/// 
/// The route would then have the following meta data set.
/// 
/// Init Route: "/test/get-user?name=test"
/// Cleaned Route: "/test/get-user"
/// Params: [("name", "test")]
#[derive(Debug)]
pub struct Route {
    /// The full route given
    pub init_route: String,

    /// The full route given without any params. 
    pub cleaned_route: String,
    /// Any params within the route/
    params: HashMap<String, String>,
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.init_route)
    }
}

impl Route {

    pub fn parse_route(init_route: String) -> Self {
        let mut parsed = HashMap::new();

        let mut cleaned_route = "".to_string();

        /*
           /admin/api/test?v=
        */
        let route_parts = init_route.split("/").filter(|s| { !s.is_empty() });

        for route_part in route_parts {
            // admin or api or test?x=y&z=x

            let has_params = route_part.split_once("?");

            if has_params.is_none() {
                cleaned_route.push_str(&format!("/{route_part}"));
                continue;
            }

            let (non_param, params) = has_params.unwrap();

            // incase check
            if !non_param.is_empty() {
                cleaned_route.push_str(&format!("/{non_param}"));
            }

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

        Self {
            params: parsed,
            init_route,
            cleaned_route,
        }
    }

    pub fn get_param(&self, param_name: &str) -> Option<&String> {
        self.params.get(param_name)
    }

    pub fn get_params(&self) -> &HashMap<String, String> {
        &self.params
    }
}
