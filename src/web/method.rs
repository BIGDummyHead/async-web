#[derive(Debug)]
#[derive(Eq, Hash, PartialEq)]
#[derive(Clone)]
pub enum Method {
    GET,
    POST,
    PUT, 
    DELETE,
    Other(String)
}
