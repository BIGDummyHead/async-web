/// # Method
/// 
/// Describes a method that a HTTP request may send. 
/// 
/// Commont variants may include GET, POST, DELETE, etc...
/// 
/// This is used majorly when creating route. 
/// 
/// Routes may have the same path if and only if the method does not match an existing method for that route.
#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum Method {
    /// GET, retrieve and serve content
    GET,
    /// POST, add content
    POST,
    /// PUT, update/replace content
    PUT, 
    /// DElETE, remove content
    DELETE,
    /// PATCH, partial update to content
    PATCH,
    /// Unknown variant, parsed as other, containing the request.
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
