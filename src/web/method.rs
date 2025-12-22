/// ## Method
/// 
/// The routing method.
/// 
/// Used when adding a routing type to your program or when a request comes in.
#[derive(Debug)]
#[derive(Eq, Hash, PartialEq)]
#[derive(Clone)]
pub enum Method {
    GET,
    POST,
    PUT, 
    DELETE,
    PATCH,
    Other(String)
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        let m = match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::DELETE => "DELETE",
            Self::PATCH => "PATCH",
            Self::Other(x) => &format!("Other({x})"),
        };

        write!(f, "{m}")
    }
}
