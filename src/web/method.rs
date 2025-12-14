#[derive(Debug)]
#[derive(Eq, Hash, PartialEq)]
pub enum Method {
    GET,
    POST,
    PUT, 
    DELETE,
    Other(String)
}