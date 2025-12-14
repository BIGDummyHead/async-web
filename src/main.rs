use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
    vec,
};

use futures::join;
use std::future::Future;
use std::pin::Pin;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, net::{TcpListener, TcpStream}, time::sleep
};
use std::fs;

use crate::web::{Queue, Request, WorkManager};

pub mod web;

#[tokio::main]
async fn main() {
    let mut man: WorkManager<()> = WorkManager::new(3, Some(100)).await;

    let addr = Ipv4Addr::new(127, 0, 0, 1);
    let port = 8080;

    let listener_result = TcpListener::bind(SocketAddrV4::new(addr, port)).await;

    if let Err(e) = &listener_result {
        eprintln!("Could not bind to address: {e}");
        return;
    }

    let listener = listener_result.unwrap();

    loop {
        let client_result = listener.accept().await;

        if let Err(c_err) = client_result {
            eprintln!("Failed to connect client: {c_err}");
            continue;
        }

        let (stream, _) = client_result.unwrap();

        man.add_work(Box::pin(async move {
            let req_result = process_acception(stream).await;

            if let Err(e) = req_result {
                eprintln!("Error in processing request: {}", e);
                return;
            }

            

        }))
        .await;
    }
}


async fn process_acception(mut stream: TcpStream) -> Result<Request, std::io::Error> {
    
    let request_result = Request::parse_request(&mut stream).await;

    if let Err(e) = request_result {
        return Err(e);
    }

    let request= request_result.unwrap();

    Ok(request)
}
