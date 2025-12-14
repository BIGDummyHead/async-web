#[derive(Debug)]
pub enum Method {
    GET,
    POST,
    PUT, 
    DELETE,
    Other(String)
}