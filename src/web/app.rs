use std::{net::SocketAddr, pin::Pin, sync::Arc};

use futures::StreamExt;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard, broadcast},
    task::{self, JoinHandle},
};

use crate::{factory::WorkManager, web::errors::AppState};

use crate::web::{
    EndPoint, Method, Middleware, Request, Resolution,
    errors::RoutingError,
    resolution::empty_resolution::EmptyResolution,
    routing::{
        ResolutionFnRef, RouteNodeRef,
        middleware::{MiddlewareClosure, MiddlewareCollection},
        router::route_tree::RouteTree,
    },
};

/// # App
///
/// Represents an async Web Based Application with workers, routers, and a TCP Listener.
///
/// To create an app you may use:
///
/// ```
///  // --snip--
/// let app_creation_result = App::bind(worker_count, addr);
///
/// // Check if app was created successfully
/// ```
pub struct App {
    pub work_manager: Arc<WorkManager<()>>,
    pub listener: Option<TcpListener>,
    pub router: Arc<Mutex<RouteTree>>,
    //middleware that is applied to all routes called
    global_middleware: Arc<Mutex<Vec<MiddlewareClosure>>>,
    //handled to the spawned task
    app_task: Option<JoinHandle<()>>,
    //callback to handle errors that in inbound from
    error_callback: Option<Arc<Pin<Box<dyn Fn(String) -> () + Send + Sync + 'static>>>>,
    shutdown: Option<broadcast::Sender<()>>,
}

/// Represents a web application where you can bind, route, and do other web server related activities.
impl App {
    /// ## Use Middleware
    ///
    /// Adds middleware that is used for each request that is created by the client.
    ///
    /// This is useful for a function that needs to be called for each request like authentication.
    pub async fn use_middleware(&mut self, closure: MiddlewareClosure) {
        self.global_middleware.lock().await.push(closure);
    }

    /// ## Bind
    ///
    /// Binds the program to a Socket via TCP.
    ///
    /// ### Example
    ///
    /// ```
    /// //local app settings.
    ///let addr = Ipv4Addr::new(127, 0, 0, 1);
    ///let port = 8080;
    ///
    /// //the count of wokrers to make
    ///let workers = 100;
    ///
    /////try bind socket.
    ///let app_bind = App::bind(workers, SocketAddrV4::new(addr, port)).await;
    /// ```
    pub async fn bind<A>(worker_count: usize, addr: A) -> Result<Self, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        //bind our tcp listener to handle request.
        let bind_result = TcpListener::bind(addr).await;

        if let Err(e) = bind_result {
            return Err(e);
        }

        let work_manager = Arc::new(WorkManager::new(worker_count).await);

        let listener = Some(bind_result.unwrap());
        let router = Arc::new(Mutex::new(RouteTree::new(None)));

        let bind = Self {
            work_manager,
            listener,
            router,
            global_middleware: Arc::new(Mutex::new(Vec::new())),
            app_task: None,
            error_callback: None,
            shutdown: None,
        };

        bind.consume().await;

        Ok(bind)
    }

    ///  consume
    ///
    /// Spawns a background task that continuously consumes messages from the work manager receiver.
    ///
    /// Prevents the internal work channel from filling and blocking producers.
    ///
    /// Runs until the receiver channel is closed.

    async fn consume(&self) -> JoinHandle<()> {
        let receiver = self.work_manager.receiver.clone();

        task::spawn(async move {
            let mut rx = receiver.lock().await;

            while let Some(_) = rx.recv().await {}
        })
    }

    /// Reads from an incoming TCP stream and parses it into a `Request`.
    ///
    /// Handles request decoding and validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream cannot be read or the request is malformed.

    async fn process_acception(
        mut stream: &mut TcpStream,
        connected_socket: SocketAddr,
    ) -> Result<Request, std::io::Error> {
        let request_result = Request::parse_request(&mut stream, connected_socket).await;

        if let Err(e) = request_result {
            return Err(e);
        }

        let request = request_result.unwrap();

        Ok(request)
    }

    /// Extracts dynamic route parameters from the matched route tree.
    ///
    /// Traverses parent route nodes and assigns variable values into the request.
    /// This is executed after routing but before middleware and resolution execution.

    async fn set_request_variables(req_ref: Arc<Mutex<Request>>, route_ref: RouteNodeRef) -> () {
        //the given route by the user, cleaned.
        let given_route: String = {
            let req_lock = req_ref.lock().await;

            req_lock.route.cleaned_route.clone()
        };

        let mut given_route_parts: Vec<&str> = given_route.split('/').collect();

        let mut current_ref = Some(route_ref.clone());

        let wild_card_skip = {
            let mut current = Some(route_ref.clone());
            let mut wild_skip = 0;

            while let Some(node) = current {
                let guard = node.lock().await;
                current = guard.parent.clone();
                wild_skip += 1;
            }

            //skip for the WILDCARD {*} and SKIP for the beginning "/" route.
            wild_skip - 1
        };

        while let Some(c_ref) = current_ref {
            //pop a route part
            let route_part = given_route_parts.pop();

            //if none, something is wrong, break out
            if route_part.is_none() {
                break;
            }

            //unwrap the route part
            let route_part = route_part.unwrap();

            //check if the route part is empty, we are allowed to continue from this
            if route_part.is_empty() {
                //since we own c_ref and have not locked, we can just reuse.
                //we need to pass into some for ownership
                current_ref = Some(c_ref);
                continue;
            }

            //lock for checks
            let c_ref_lock = c_ref.lock().await;

            if c_ref_lock.is_var {
                //clean the ID from {name} -> name
                let mut id = c_ref_lock.id.clone();
                id.remove(0);
                id.remove(id.len() - 1);

                let is_wild = id.eq("*");

                let value = if is_wild {
                    given_route_parts.push(route_part);

                    given_route_parts
                        .iter()
                        .skip(wild_card_skip)
                        .copied()
                        .collect::<Vec<&str>>()
                        .join("/")
                } else {
                    route_part.to_string()
                };

                req_ref.lock().await.variables.insert(id, value);

                if is_wild {
                    break;
                }
            }

            current_ref = c_ref_lock.parent.clone();
        }
    }

    /// # Start
    ///
    /// Starts the application.
    ///
    /// ## Returns
    ///
    /// This function returns:
    ///
    /// Err(AppState::Running) if the application was already running
    /// Err(AppState::Closed) if the application was closed
    /// or
    ///
    /// Ok(AppState::Running) if the application was started successfully.
    pub fn start(&mut self) -> Result<AppState, AppState> {
        if self.app_task.is_some() {
            return Err(AppState::Running);
        }

        //err cannot start.
        if self.listener.is_none() {
            return Err(AppState::Closed);
        }

        // create reference clones to each thing passed to the opened task
        let work_manager = self.work_manager.clone();
        let router = self.router.clone();
        let global_middleware = self.global_middleware.clone();

        let error_callback = self.error_callback.as_ref().map(|cb| cb.clone());

        let listener = self.listener.take().unwrap();

        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        self.shutdown = Some(shutdown_tx);

        //add the app_task
        self.app_task = Some(task::spawn(async move {
            //create a default callback if none.
            let error_callback = error_callback.unwrap_or(Arc::new(Box::pin(|_| {})));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    },
                    accepted_client = listener.accept() => {

                        if let Err(e) = accepted_client {
                            error_callback(e.to_string());
                            continue;
                        }

                        let router_ref = router.clone();
                        let middleware_ref = global_middleware.clone();

                        work_manager
                            .add_work(Box::pin(async move {
                                Self::request_work(accepted_client.unwrap(), middleware_ref, router_ref)
                                    .await;
                            }))
                            .await;
                    }
                }
            }
        }));

        Ok(AppState::Running)
    }

    /// # close
    ///
    /// Closes the web app.
    ///
    /// You must await this function to join the app handle with this thread.
    ///
    /// ## Returns
    ///
    /// This function returns:
    ///
    /// `Err(AppState::Closed)` if the application was already closed
    ///
    /// or
    ///
    /// `Ok(AppState::Closed)` if the application was closed.
    pub async fn close(&mut self) -> Result<AppState, AppState> {
        if self.app_task.is_none() {
            return Err(AppState::Closed);
        }

        let task = self.app_task.take().unwrap();

        let closure = self.shutdown.take().unwrap();
        let _ = closure.send(());

        let _ = task.await;

        Ok(AppState::Closed)
    }

    /// Executes all logic required to handle a single client request.
    ///
    /// This includes:
    /// - Parsing the request
    /// - Resolving the route and method
    /// - Applying middleware
    /// - Executing the endpoint resolution
    /// - Writing the response to the TCP stream
    ///
    /// Errors during processing terminate handling for the request.

    async fn request_work(
        client: (TcpStream, SocketAddr),
        global_middleware: Arc<Mutex<Vec<MiddlewareClosure>>>,
        router_ref: Arc<Mutex<RouteTree>>,
    ) -> () {
        let mut stream = client.0;
        let client_socket = client.1;

        //process the acception and get the result from the stream
        let req_result = Self::process_acception(&mut stream, client_socket).await;

        if let Err(e) = req_result {
            eprintln!("Error in processing request: {}", e);
            return;
        }

        //the web request
        let web_request = req_result.unwrap();

        let request = Arc::new(Mutex::new(web_request));

        //get the function to handle the resolution, backs up to a 404 if existant
        let (cleaned_route, method) = {
            let request_lock = request.lock().await;
            (
                request_lock.route.cleaned_route.clone(),
                request_lock.method.clone(),
            )
        };

        let endpoint_opt = {
            let binding = router_ref.lock().await;

            let route = binding.get_route(&cleaned_route).await;

            match route {
                Some(r) => {
                    // This no longer deadlocks because the lock was dropped above
                    Self::set_request_variables(request.clone(), r.clone()).await;
                    let route_lock = r.lock().await;
                    route_lock.brw_resolution(&method).clone()
                }
                None => binding
                    .missing_route
                    .as_ref()
                    .and_then(|mr| mr.brw_resolution(&Method::GET))
                    .clone(),
            }
        };

        if endpoint_opt.as_ref().is_none() {
            return;
        }

        let endpoint = endpoint_opt.unwrap();

        // middleware_failed_resolution, gives back an Option<Middleware> with some if it failed
        let middleware_failed_resolution = {
            let mut final_middleware = None;

            let global_middleware_lock = global_middleware.lock().await;

            let mut all_middleware = Vec::new();
            all_middleware.extend_from_slice(&global_middleware_lock);

            // ! Drop reference once we have all the function refs.
            drop(global_middleware_lock);

            if let Some(route_middleware) = &endpoint.middleware {
                all_middleware.extend_from_slice(&route_middleware);
            }

            for middle_ware_closure in all_middleware {
                match middle_ware_closure(request.clone()).await {
                    Middleware::Invalid(res) => {
                        final_middleware = Some(res);
                        break;
                    }
                    Middleware::InvalidEmpty(status_code) => {
                        final_middleware = Some(EmptyResolution::status(status_code).resolve());
                        break;
                    }
                    Middleware::Next => continue,
                };
            }

            final_middleware
        };

        let write_resolution = if let Some(failed_middleware) = middleware_failed_resolution {
            Some(failed_middleware)
        } else {
            
            Some((endpoint.resolution)(request.clone()).await)
        };

        if write_resolution.as_ref().is_none() {
            return;
        }

        let resolved = Self::resolve(write_resolution.unwrap(), &mut stream).await;

        if let Err(e_r) = resolved {
            println!("Failed to resolve request: {e_r}");
        }
    }

    /// Finalizes a `Resolution` into a complete HTTP response.
    ///
    /// Writes headers, content length, and body to the provided TCP stream.
    ///
    /// # Errors
    ///
    /// I/O errors encountered during writing are logged but not returned.

    async fn resolve(
        resolved: Box<dyn Resolution + Send>,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error>> {
        //write the headers to the stream
        let mut headers = resolved.get_headers().await.join("\r\n");
        headers.push_str("\r\nTransfer-Encoding: chunked\r\n\r\n");
        stream.write_all(headers.as_bytes()).await?;

        let mut content = resolved.get_content();

        //retrieve the next chunk of the body
        while let Some(chunk) = content.next().await {
            let size = chunk.len();

            if size <= 0 {
                continue; //nothing to write 
            }

            //size header.
            let size_header = format!("{size:X}\r\n");
            stream.write_all(size_header.as_bytes()).await?;

            //content
            stream.write_all(&chunk).await?;

            //terminator
            stream.write_all(b"\r\n").await?;
        }

        //indicate end of stream
        stream.write_all(b"0\r\n\r\n").await?;

        Ok(())
    }
}

impl App {
    /// Adds a new route or replaces an existing route’s resolution for the given method.
    ///
    /// If the route already exists, its resolution for the specified method is overwritten.
    ///
    /// # Errors
    ///
    /// Returns a `RoutingError` if the route cannot be added.

    pub async fn add_or_change_route<F, Fut>(
        &self,
        route: &str,
        method: Method,
        middleware: Option<MiddlewareCollection>,
        resolution: F,
    ) -> Result<(), RoutingError>
    where
        F: Fn(Arc<Mutex<Request>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Box<dyn Resolution + Send + 'static>> + Send + 'static,
    {
        let resolution: ResolutionFnRef = Arc::new(move |req: Arc<Mutex<Request>>| {
            Box::pin(resolution(req))
                as Pin<Box<dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send>>
        });

        let endpoint = EndPoint::new(resolution, middleware);

        let mut router = self.router.lock().await;
        router.add_route(route, Some((method, endpoint))).await
    }

    /// Adds a new route or replaces an existing route’s resolution for the given method.
    ///
    /// If the route already exists, its resolution for the specified method is overwritten.
    ///
    /// # Errors
    ///
    /// Returns a `RoutingError` if the route cannot be added.
    pub async fn add_route<F, Fut>(
        &self,
        route: &str,
        method: Method,
        middleware: Option<MiddlewareCollection>,
        resolution: F,
    ) -> Result<(), RoutingError>
    where
        F: Fn(Arc<Mutex<Request>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Box<dyn Resolution + Send + 'static>> + Send + 'static,
    {
        let mut router = self.router.lock().await;

        if let Some(rte) = router.get_route(route).await {
            if rte.lock().await.brw_resolution(&method).is_some() {
                return Err(RoutingError::Exist);
            }
        }

        let resolution: ResolutionFnRef = Arc::new(move |req: Arc<Mutex<Request>>| {
            Box::pin(resolution(req))
                as Pin<Box<dyn Future<Output = Box<dyn Resolution + Send + 'static>> + Send>>
        });

        let endpoint = EndPoint::new(resolution, middleware);
        let route_res = Some((method, endpoint));

        router.add_route(route, route_res).await
    }

    /// Adds a route and method combination to the router.
    ///
    /// # Panics
    ///
    /// Panics if the route already exists or cannot be added.
    /// Intended for use during application initialization.

    pub async fn add_or_panic<F, Fut>(
        &self,
        route: &str,
        method: Method,
        middleware: Option<MiddlewareCollection>,
        resolution: F,
    ) -> ()
    where
        F: Fn(Arc<Mutex<Request>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Box<dyn Resolution + Send + 'static>> + Send + 'static,
    {
        let result = self.add_route(route, method, middleware, resolution).await;

        if let Err(e) = result {
            panic!("When adding route '{route}' an error occurred because '{e}'");
        }
    }

    /// Provides exclusive access to the internal route tree.
    ///
    /// Returns a locked guard allowing inspection or modification of routing state.
    /// This call blocks until the router mutex becomes available.

    pub async fn get_router(&self) -> MutexGuard<'_, RouteTree> {
        self.router.lock().await
    }

    /// # Set Error callback
    ///
    /// Sets the error callback using a FN closure.
    ///
    /// This error callback is used for the App task handle. Allowing you to control how errors are displayed to the user.
    ///
    /// This MUST be set before you start the app.
    pub fn set_error_callback(&mut self, callback: impl Fn(String) -> () + Send + Sync + 'static) {
        //pin the callback for the error.
        let callback: Arc<Pin<Box<dyn Fn(String) -> () + Send + Sync>>> =
            Arc::new(Box::pin(callback));
        self.error_callback = Some(callback);
    }

    /// # state
    ///
    /// Get the state of the application.
    pub fn state(&self) -> AppState {
        match &self.app_task {
            None => AppState::Closed,
            _ => AppState::Running,
        }
    }
}
