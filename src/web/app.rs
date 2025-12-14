use std::{net::SocketAddr, sync::Arc};

use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task::{self, JoinHandle},
};

use crate::web::{Request, WorkManager};

pub struct App {
    pub work_manager: Arc<WorkManager<()>>,
    pub listener: Arc<TcpListener>,
}

impl App {
    pub async fn bind<A>(worker_count: usize, addr: A) -> Result<Self, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        //bind our tcp listener to handle request.
        let bind_result = TcpListener::bind(addr).await;

        if let Err(e) = bind_result {
            return Err(e);
        }

        let work_manager = Arc::new(WorkManager::new(worker_count, None).await);

        let listener = Arc::new(bind_result.unwrap());

        Ok(Self {
            work_manager,
            listener,
        })
    }

    async fn process_acception(mut stream: TcpStream) -> Result<Request, std::io::Error> {
        let request_result = Request::parse_request(&mut stream).await;

        if let Err(e) = request_result {
            return Err(e);
        }

        let request = request_result.unwrap();

        Ok(request)
    }

    pub async fn start_listening(&self) -> JoinHandle<()> {
        let listener = self.listener.clone();
        let work_manager = self.work_manager.clone();

        task::spawn(async move {
            loop {
                let client_result = listener.accept().await;

                if let Err(c_err) = client_result {
                    eprintln!("Failed to connect client: {c_err}");
                    continue;
                }

                let (stream, _) = client_result.unwrap();

                work_manager
                    .add_work(Box::pin(async move {
                        let req_result = Self::process_acception(stream).await;

                        if let Err(e) = req_result {
                            eprintln!("Error in processing request: {}", e);
                            return;
                        }
                    }))
                    .await;
            }
        })
    }
}
