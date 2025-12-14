use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
    vec,
};

use futures::join;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::sleep,
};

use crate::web::{App, Queue, Request, WorkManager, app};

pub mod web;

#[tokio::main]
async fn main() {

    let addr = Ipv4Addr::new(127, 0, 0, 1);
    let port = 8080;

    let app_bind = App::bind(100, SocketAddrV4::new(addr, port)).await;

    if let Err(e) = app_bind {
        eprintln!("Could not bind app! {e}");
        return;
    }

    let app = app_bind.unwrap();

    let handle = app.start_listening().await;

    let join_result = handle.await;

    if let Err(e) = join_result {
        eprintln!("Error joining the thread: {e}");
    }
}

