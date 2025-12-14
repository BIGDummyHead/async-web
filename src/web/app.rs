use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

use tokio::{
    io::AsyncWriteExt, net::{TcpListener, TcpStream, ToSocketAddrs}, sync::{Mutex, MutexGuard}, task::{self, JoinHandle}
};

use crate::web::{Method, Request, RouteTree, WorkManager};

pub struct App {
    pub work_manager: Arc<WorkManager<()>>,
    pub listener: Arc<TcpListener>,
    pub router: Arc<Mutex<RouteTree>>,
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
        let router = Arc::new(Mutex::new(RouteTree::new(None)));

        Ok(Self {
            work_manager,
            listener,
            router,
        })
    }

    async fn process_acception(mut stream: &mut TcpStream) -> Result<Request, std::io::Error> {
        let request_result = Request::parse_request(&mut stream).await;

        if let Err(e) = request_result {
            return Err(e);
        }

        let request = request_result.unwrap();

        Ok(request)
    }

    pub async fn start(&self) -> JoinHandle<()> {
        let listener = self.listener.clone();
        let work_manager = self.work_manager.clone();
        let router = self.router.clone();

        task::spawn(async move {
            loop {
                let client_result = listener.accept().await;

                if let Err(c_err) = client_result {
                    eprintln!("Failed to connect client: {c_err}");
                    continue;
                }

                let (mut stream, _) = client_result.unwrap();

                let router_ref = router.clone();


                work_manager
                    .add_work(Box::pin(async move {
                        let req_result = Self::process_acception(&mut stream).await;

                        if let Err(e) = req_result {
                            eprintln!("Error in processing request: {}", e);
                            return;
                        }

                        let request = req_result.unwrap();

                        let mut binding = router_ref.lock().await;
                        let pos_route = binding.get_route(&request.route.init_route);

                        if let Some(route) = pos_route {
                            let pos_resolution = route.get_resolution(&request.method);

                            if let None = pos_resolution {
                                //TODO: Implement 404 error.
                            }
                        
                            if let Some(resolution) = pos_resolution {
                                let resolved = resolution(request).await;

                                let mut full_response = resolved.get_headers().join("\r\n");
                                let content = resolved.get_content();
                                let c_length = content.len();

                                full_response.push_str(&format!("Content-Length: {c_length}\r\n"));
                                full_response.push_str("\r\n");
                                full_response.push_str(&content);

                                let write_result = stream.write_all(full_response.as_bytes()).await;

                                if let Err(e) = write_result {
                                    eprintln!("Error when writing to the endpoint TCP Stream: {e}");
                                }
                            }
                        }

                    }))
                    .await;
            }
        })
    }

    pub async fn get_router(&self) -> MutexGuard<'_, RouteTree> {
        self.router.lock().await
    }
}
