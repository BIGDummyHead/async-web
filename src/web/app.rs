use std::net::SocketAddr;

use tokio::net::{TcpListener, ToSocketAddrs};

use crate::web::WorkManager;

pub struct App {
    pub work_manager: WorkManager<()>,
    pub listener: TcpListener,
}

impl App {
    pub async fn new<A>(worker_count: usize, addr: A) -> Result<Self, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        //bind our tcp listener to handle request.
        let bind_result = TcpListener::bind(addr).await;

        if let Err(e) = bind_result {
            return Err(e);
        }

        let work_manager: WorkManager<()> = WorkManager::new(worker_count, None).await;

        let listener = bind_result.unwrap();

        Ok(Self {
            work_manager,
            listener
        })
    }
}
