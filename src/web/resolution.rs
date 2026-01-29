use futures::Stream;
use linked_hash_map::LinkedHashMap;
use std::pin::Pin;


pub mod empty_resolution;
pub mod error_resolution;
pub mod file_resolution;
pub mod json_resolution;
pub mod merged_resolution;
pub mod redirect;

/// # Resolution
///
/// A trait that allows you to return a struct to an endpoint from a web app.
///
pub trait Resolution: Send + 'static {
    /// # Get headers
    ///
    /// Return a collection of Header keys and values.
    ///
    fn get_headers(&self) -> LinkedHashMap<String, Option<String>>;

    /// # Get Content
    ///
    /// This function should return a stream back to the endpoint that can be iterated to retrieve and serve content to a TCP Stream.
    ///
    fn get_content(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;

    /// # resolve
    ///
    /// Converts the T type into a Box<dyn Resolution ...
    ///
    /// Please use the following example for basic conversions
    ///
    /// Example:
    /// ```
    /// fn resolve(self) -> Box<dyn Resolution> + Send + 'static {
    ///     Box::new(self)
    /// }
    /// ```
    fn resolve(self) -> Box<dyn Resolution + Send + 'static>;
}

/// # Get Status
///
/// This function can be used to create headers based on code.
///
/// The get_status function takes in a status code and turns it into the header. For example
///
/// ```
///
/// //okay code
/// let code = 200;
///
/// // status is equal to "OK"
/// let status = get_status(&code);
///
/// ```
pub fn get_status(status_code: &i32) -> &str {
    match status_code {
        // 1xx Informational
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",

        // 2xx Success
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",

        // 3xx Redirection
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",

        // 4xx Client Error
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a Teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",

        // 5xx Server Error
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",

        _ => "Unknown Status Code",
    }
}

/// Gives you back the appropriate header based on a status code.
///
/// ### Example
///
/// ```
/// let (header_key, header_val) = get_status_header(200);
///
/// //prints out "HTTP/1.1 200 OK"
/// println!("{header_key} {header_val}");
///
/// ```
pub fn get_status_header(status_code: i32) -> (String, String) {
    let status = get_status(&status_code);

    ("HTTP/1.1".to_string(), format!("{status_code} {status}"))
}

/// # Empty Content
///
/// Signals that there is no content to serve.
///
/// Equal to
///
/// ```
/// let result:Vec<u8> = Vec::new();
/// ```
pub fn empty_content() -> Vec<u8> {
    Vec::with_capacity(0)
}
