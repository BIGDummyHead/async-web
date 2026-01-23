use std::{collections::HashMap, net::SocketAddr};

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

    /// The connected socket of the client
    pub client_socket: SocketAddr,
}

impl Request {
    /// Parse a tcp stream request and gives back the Request
    pub async fn parse_request(
        stream: &mut TcpStream,
        client_socket: SocketAddr,
    ) -> Result<Self, std::io::Error> {
        //create a buffer that will read each line
        let mut reader = BufReader::new(stream);

        let mut first_line = String::new();

        //the first line should be parsed independently
        reader.read_line(&mut first_line).await?;

        if first_line.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "parse request failed due to no data being provided",
            ));
        }

        let mut request_header = first_line.split(" ");

        let method = request_header
            .next()
            .map(|header_value| {
                Ok(match header_value {
                    "GET" => Method::GET,
                    "PUT" => Method::PUT,
                    "POST" => Method::POST,
                    "DELETE" => Method::DELETE,
                    "PATCH" => Method::PATCH,
                    header_value => Method::Other(header_value.to_string()),
                })
            })
            .unwrap_or(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing header for method",
            )))?;

        let route = request_header
            .next()
            .map(|header_value| Ok(Route::parse_route(header_value.to_string())))
            .unwrap_or(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing header for request",
            )))?;

        //all other headers beside the first
        let mut headers = HashMap::new();

        //insert all headers
        loop {
            let mut read_header = String::new();

            reader.read_line(&mut read_header).await?;

            let read_header = read_header.trim_end();

            //no more headers.
            if read_header.is_empty() {
                break;
            }

            let split_header = read_header.split_once(":");

            if split_header.is_none() {
                continue;
            }

            //unwrap the known some value and insert into the headers.
            let (header_key, header_val) = split_header.unwrap();
            headers.insert(String::from(header_key), String::from(header_val.trim()));
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
            client_socket,
        })
    }
}
