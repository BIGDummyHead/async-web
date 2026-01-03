use futures::Stream;
use std::pin::Pin;

pub mod empty_resolution;
pub mod file_resolution;
pub mod file_text_resolution;
pub mod json_resolution;

/// Represents a resolution for a request
pub trait Resolution {
    ///
    /// Get all headers for the HTTP response.
    ///
    fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>>;

    ///
    /// Get the content for the resolution. Gets pushed into the headers. Then a length is used.
    fn get_content(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;
}

/// Returns a status string based on a code.
///
/// ### Example
/// ```
/// let status = get_status(200);
///
/// //output is OK
/// println!("{status}");
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
/// let header = get_status_header(200);
///
/// //outputs HTTP/1.1 200 OK
/// println!("{}", header);
///
///
/// ```
pub fn get_status_header(status_code: i32) -> String {
    let status = get_status(&status_code);

    return format!("HTTP/1.1 {} {}", status_code, status).to_string();
}

/// Signals that there is no content to serve.
///
/// Equal to
///
/// ```
/// let result:Vec<u8> = Vec::new();
/// ```
pub fn empty_content() -> Vec<u8> {
    Vec::new()
}
