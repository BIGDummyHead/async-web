use crate::web::Method;

pub struct RouteInfo{
    pub route: String,
    pub method: Method,
    //TODO Add middleware
    //pub middle_ware: Vec<MiddleWare>
}