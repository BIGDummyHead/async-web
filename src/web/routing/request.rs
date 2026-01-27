use std::{collections::HashMap, net::SocketAddr};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    net::TcpStream,
};

use crate::web::{Method, Route};

/// # Request
/// 
/// Represents a singular request that has been made by a TcpStream.
/// 
/// Data includes the method, the route, headers, variables, and the body of the request.
pub struct Request {
    /// The method used for this request.
    pub method: Method,

    /// The route of the request
    pub route: Route,

    /// # headers
    /// 
    /// The headers that are included in the request, such as the content length, and other misc header items
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

    /// The body of the request.
    /// 
    /// None if there was no body included in the request.
    pub body: Option<Vec<u8>>,

    /// The connected socket of the client
    pub client_socket: SocketAddr,
}

impl Request {
    /// # from_stream
    /// 
    /// Takes a mutable reference to the TcpStream (client), reading each line of the stream.
    /// 
    /// Each line is individually parsed to create a Request.
    /// 
    /// The client's socket is stored in the Request.
    pub async fn from_stream(
        stream: &mut TcpStream,
        client_socket: SocketAddr,
    ) -> Result<Self, std::io::Error> {
        //create a buffer that will read each line
        let mut reader = BufReader::new(stream);

        let mut request_line = String::new();

        //the first line should be parsed independently
        reader.read_line(&mut request_line).await?;

        if request_line.is_empty() {
            //no data
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "parse request failed due to no data being provided",
            ));
        }

        let mut request_header = request_line.split(" ");

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

        let body = if content_length > 0 {
            
            //read the body from the content length.
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body).await?;
            Some(body)

        } else {
            //no body was provided.
            None
        };

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
