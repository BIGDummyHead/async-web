use std::collections::HashMap;

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    net::TcpStream,
};

use crate::web::{Method, Route};

/// Represents a web request.
pub struct Request {
    /// The method used for this request.
    pub method: Method,
    /// The route of the request
    pub route: Route,
    /// Any other header.
    pub headers: HashMap<String, String>,

    /// Variable path items.
    ///
    /// ### Example
    ///
    /// You add the route "/tasks/{userId}/delete"
    ///
    /// > The user fetches "/tasks/1/delete"
    ///
    /// You may now retrieve from the table "userId" and get the value "1"
    pub variables: HashMap<String, String>,

    /// Body of the request.
    pub body: Vec<u8>,
}

impl Request {
    /// Parse a tcp stream request and gives back the Request
    pub async fn parse_request(stream: &mut TcpStream) -> Result<Self, std::io::Error> {
        //create a buffer that will read each line
        let mut reader = BufReader::new(stream);

        let mut first_line = String::new();

        //the first line should be parsed independently
        reader.read_line(&mut first_line).await?;

        if first_line.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "The first line was not found.",
            ));
        }

        let mut request_header = first_line.split(" ");

        let method = match request_header.next() {
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "The header for the request was missing the method.",
                ));
            }
            Some(v) => {
                let method = match v {
                    "GET" => Method::GET,
                    "PUT" => Method::PUT,
                    "POST" => Method::POST,
                    "DELETE" => Method::DELETE,
                    "PATCH" => Method::PATCH,
                    v => Method::Other(String::from(v)),
                };

                method
            }
        };

        let route = match request_header.next() {
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "The header for the request was missing the route.",
                ));
            }
            Some(route) => Route::parse_route(String::from(route)),
        };

        //all other headers beside the first
        let mut headers = HashMap::new();

        //insert all headers
        loop {
            let mut read_line = String::new();

            reader.read_line(&mut read_line).await?;

            let header = read_line.trim_end();

            if header.is_empty() {
                break;
            }

            if let Some((header_key, header_val)) = header.split_once(":") {
                headers.insert(String::from(header_key), String::from(header_val.trim()));
            }
        }

        let content_length = headers
            .get("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];

        if content_length > 0 {
            reader.read_exact(&mut body).await?;
        }

        Ok(Self {
            method,
            route,
            headers,
            body,
            variables: HashMap::new(),
        })
    }
}
