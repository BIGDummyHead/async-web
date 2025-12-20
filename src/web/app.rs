use std::sync::Arc;

use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard},
    task::{self, JoinHandle},
};

use crate::web::{
    EndPoint, Method, Middleware, Request, Resolution, WorkManager,
    errors::{RoutingError, routing_error::RoutingErrorType},
    middleware::MiddlewareCollection,
    resolution::empty_resolution::EmptyResolution,
    router::{ResolutionFunc, RouteNodeRef, RouteTree},
};

pub struct App {
    pub work_manager: Arc<WorkManager<()>>,
    pub listener: Arc<TcpListener>,
    pub router: Arc<Mutex<RouteTree>>,
}

/// Represents a web application where you can bind, route, and do other web server related activities.
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

        let work_manager = Arc::new(WorkManager::new(worker_count, Some(100)).await);

        let listener = Arc::new(bind_result.unwrap());
        let router = Arc::new(Mutex::new(RouteTree::new(None)));

        let bind = Self {
            work_manager,
            listener,
            router,
        };

        bind.consume().await;

        Ok(bind)
    }

    /// Spawns a task to consume received information from the work manager.
    async fn consume(&self) -> JoinHandle<()> {
        let receiver = self.work_manager.receiver.clone();

        task::spawn(async move {
            let mut rx = receiver.lock().await;

            while let Some(_) = rx.recv().await {}
        })
    }

    /// Proccesses each acception from the stream
    async fn process_acception(mut stream: &mut TcpStream) -> Result<Request, std::io::Error> {
        let request_result = Request::parse_request(&mut stream).await;

        if let Err(e) = request_result {
            return Err(e);
        }

        let request = request_result.unwrap();

        Ok(request)
    }

    async fn set_request_variables(req_ref: Arc<Mutex<Request>>, route_ref: RouteNodeRef) -> () {
        let route_parts: Vec<String> = req_ref
            .lock()
            .await
            .route
            .init_route
            .split('/')
            .rev()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let mut current_node = Some(route_ref);

        // the value given here is from the route, so it is the value the user provided
        for value in route_parts {
            // we are done searching here
            let node_ref = match current_node {
                Some(n) => n,
                None => break,
            };

            let node = node_ref.lock().await;

            if node.is_var {
                let mut id = node.id.clone();
                id.remove(0);
                id.remove(id.len() - 1);

                req_ref.lock().await.variables.insert(id, value);
            }

            let next_node = node.parent.clone();

            current_node = next_node;
        }
    }

    /// Starts the app, returns a handle referencing the app's task.
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

                let (stream, _) = client_result.unwrap();

                let router_ref = router.clone();

                work_manager
                    .add_work(Box::pin(async move {
                        Self::request_work(stream, router_ref).await;
                    }))
                    .await;
            }
        })
    }

    /// Work to be completed by a worker, takes the stream to write a resolution and the route tree to use.
    async fn request_work(mut stream: TcpStream, router_ref: Arc<Mutex<RouteTree>>) -> () {
        //process the acception and get the result from the stream
        let req_result = Self::process_acception(&mut stream).await;

        if let Err(e) = req_result {
            eprintln!("Error in processing request: {}", e);
            return;
        }

        //the web request
        let web_request = req_result.unwrap();

        let request = Arc::new(Mutex::new(web_request));

        //get the function to handle the resolution, backs up to a 404 if existant
        let (init_route, method) = {
            let request_lock = request.lock().await;
            (
                request_lock.route.init_route.clone(),
                request_lock.method.clone(),
            )
        };

        let endpoint_opt = {
            let binding = router_ref.lock().await;

            let route = binding.get_route(&init_route).await;

            match route {
                Some(r) => {
                    // This no longer deadlocks because the lock was dropped above
                    Self::set_request_variables(request.clone(), r.clone()).await;

                    let route_lock = r.lock().await;
                    route_lock.get_resolution(&method).clone()
                }
                None => binding
                    .missing_route
                    .as_ref()
                    .and_then(|mr| mr.get_resolution(&Method::GET))
                    .clone(),
            }
        };

        if endpoint_opt.as_ref().is_none() {
            return;
        }

        let endpoint = endpoint_opt.unwrap();

        let middleware_failed_resolution = match &endpoint.middleware {
            None => None,
            Some(middleware) => {
                let mut final_middleware = None;

                for m in middleware {
                    let m_result = match m(request.clone()).await {
                        Middleware::Invalid(res) => Some(res),
                        Middleware::InvalidEmpty(status_code) => {
                            Some(EmptyResolution::new(status_code))
                        }
                        Middleware::Next => None,
                    };

                    if m_result.is_none() {
                        continue;
                    }

                    final_middleware = Some(m_result.unwrap());
                    break;
                }

                final_middleware
            }
        };

        let write_resolution = if let Some(failed_middleware) = middleware_failed_resolution {
            Some(failed_middleware)
        } else {
            Some((endpoint.resolution)(request.clone()).await)
        };


        if write_resolution.as_ref().is_none() {
            return;
        }

        Self::resolve(write_resolution.unwrap(), &mut stream).await;
    }

    /// Calls and consumes the resolution, passing the request, then writes to the stream
    async fn resolve(resolved: Box<dyn Resolution + Send>, stream: &mut TcpStream) {
        // get the resolution if any

        let mut full_response = resolved.get_headers().await.join("\r\n");
        let content = resolved.get_content().await;
        let c_length = content.len();

        full_response.push_str(&format!("\r\nContent-Length: {c_length}\r\n"));
        full_response.push_str("\r\n");
        full_response.push_str(&content);

        let write_result = stream.write_all(full_response.as_bytes()).await;

        if let Err(e) = write_result {
            eprintln!("Error when writing to the endpoint TCP Stream: {e}");
        }
    }

    /// Adds or changes the given route.
    ///
    /// Returns an error if there was any error adding the route.
    pub async fn add_or_change_route(
        &self,
        route: &str,
        method: Method,
        middleware: Option<MiddlewareCollection>,
        resolution: ResolutionFunc,
    ) -> Result<(), RoutingError> {
        let endpoint = EndPoint::new(resolution, middleware);

        let mut router = self.router.lock().await;
        router.add_route(route, Some((method, endpoint))).await
    }

    /// Add route to the router.
    ///
    /// Returns a Routing Error if the route exist or if there was any error adding the route.
    pub async fn add_route(
        &self,
        route: &str,
        method: Method,
        middleware: Option<MiddlewareCollection>,
        resolution: ResolutionFunc,
    ) -> Result<(), RoutingError> {
        let mut router = self.router.lock().await;

        let pos_route = router.get_route(route).await;

        if let Some(r) = pos_route {
            if r.lock().await.get_resolution(&method).is_some() {
                return Err(RoutingError::new(RoutingErrorType::Exist));
            }
        }

        let endpoint = EndPoint::new(resolution, middleware);
        let route_res = Some((method, endpoint));

        router.add_route(route, route_res).await
    }

    /// Adds the route to the router or panics if the exact route and method exist!
    pub async fn add_or_panic(
        &self,
        route: &str,
        method: Method,
        middleware: Option<MiddlewareCollection>,
        resolution: ResolutionFunc,
    ) -> () {
        let result = self.add_route(route, method, middleware, resolution).await;

        if let Err(e) = result {
            panic!("When adding route '{route}' an error occurred because '{e}'");
        }
    }

    pub async fn get_router(&self) -> MutexGuard<'_, RouteTree> {
        self.router.lock().await
    }
}
