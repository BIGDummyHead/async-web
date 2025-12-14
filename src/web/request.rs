use std::collections::HashMap;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpStream, tcp::WriteHalf},
};

use crate::web::{Method, Route};

pub struct Request {
    pub method: Method,
    pub route: Route,
    pub headers: HashMap<String, String>,
}

impl Request {
    pub async fn parse_request(stream: &mut TcpStream) -> Result<Self, std::io::Error> {
        //create a buffer that will read each line
        let (reader, write_half) = stream.split();
        let buf_reader = BufReader::new(reader);
        let mut lines = buf_reader.lines();

        //the first line should be parsed independently
        let first_line_result = lines.next_line().await;

        if let Err(e) = first_line_result {
            return Err(e);
        }

        let opt_first_line = first_line_result.unwrap();

        if opt_first_line.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "The first line was not found.",
            ));
        }

        let first_line = opt_first_line.unwrap();

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
                    "PUSH" => Method::POST,
                    "DELETE" => Method::DELETE,
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
            Some(v) => Route::new(String::from(v)),
        };

        //all other headers beside the first
        let mut headers = HashMap::new();

        //insert all headers
        while let Ok(Some(v)) = lines.next_line().await {

            if v.is_empty() {
                break;
            }

            let header = v.split_once(":");

            if let None = header {
                continue;
            }

            let (key, val) = header.unwrap();

            headers.insert(String::from(key), String::from(val));
        }

        Ok(Self {
            method,
            route,
            headers,
        })
    }
}
