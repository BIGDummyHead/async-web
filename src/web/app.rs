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
    /// The listener used for binding.
    listener: Option<TcpListener>,

    /// The router that controls all routes in the App
    router: Arc<Mutex<RouteTree>>,
    //middleware that is applied to all routes called
    global_middleware: Arc<Mutex<Vec<MiddlewareClosure>>>,

    //handle to the spawned task
    app_task: Option<JoinHandle<()>>,

    // callback to handle errors
    error_callback: Option<Arc<Pin<Box<dyn Fn(String) -> () + Send + Sync + 'static>>>>,

    /// Broadcast channel sender to kill the app task
    shutdown: Option<broadcast::Sender<()>>,

    /// reference to the work manager to control workers.
    work_manager: Arc<Mutex<WorkManager<()>>>,

    /// Worker Scale Factor
    ///
    /// The factor at which the workers will scale when the workload becomes too intense.
    ///
    /// By default (10)
    pub worker_scale_factor: Arc<Mutex<usize>>,
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
    /////try bind socket.
    ///let app_bind = App::bind(SocketAddrV4::new(addr, port)).await;
    /// ```
    pub async fn bind<A>(addr: A) -> Result<Self, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        //bind our tcp listener to handle request.
        let bind_result = TcpListener::bind(addr).await?;

        let initial_workers_size: usize = 1;
        let work_manager = Arc::new(Mutex::new(WorkManager::new(initial_workers_size).await));

        let listener = Some(bind_result);
        let router = Arc::new(Mutex::new(RouteTree::new(None)));

        let bind = Self {
            work_manager,
            listener,
            router,
            global_middleware: Arc::new(Mutex::new(Vec::new())),
            app_task: None,
            error_callback: None,
            shutdown: None,
            worker_scale_factor: Arc::new(Mutex::new(10)),
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
        let receiver = {
            let guard = self.work_manager.lock().await;

            guard.receiver.clone()
        };

        task::spawn(async move {
            let mut rx = receiver.lock().await;

            while let Some(_) = rx.recv().await {}
        })
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

        //error call back clone
        let error_callback = self.error_callback.as_ref().map(|cb| cb.clone());

        //listener
        let listener = self.listener.take().unwrap();

        //shutdown sender/receiver.
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        self.shutdown = Some(shutdown_tx);

        //scaling
        let scale_factor_clone = self.worker_scale_factor.clone();

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

                        //failed to accept the client send the error to the callback
                        if let Err(e) = accepted_client {
                            error_callback(e.to_string());
                            continue;
                        }

                        //get refs for the worker.
                        let router_ref = router.clone();
                        let middleware_ref = global_middleware.clone();
                        let error_callback = error_callback.clone();

                        //get work that needs to be completed.
                        let mut current_work = Box::pin(
                            async move {

                                //handle the client request
                                let completed_work =
                                    handle_client_request(accepted_client.unwrap(), middleware_ref, router_ref).await;

                                //handle any errors
                                if let Err(e) = completed_work {
                                    error_callback(e.to_string());
                                }
                            }
                        ) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

                        //loop, needed to ensure that work is queued properly. please see below
                        loop {

                            //lock the work managet
                            let mut work_manager = work_manager.lock().await;

                            //queue some work
                            match work_manager.queue_work(current_work).await {
                                crate::factory::queue::QueueState::Free => break, //work was successfully added to the queue (enough workers)
                                crate::factory::queue::QueueState::Blocked(returned_work) => { //the queue was blocked (no workers) this gives us back the work that was not queued.
                                    current_work = returned_work;

                                    //scale our worker count.
                                    let scale_factor = *scale_factor_clone.lock().await;
                                    work_manager.scale_workers(scale_factor).await;

                                    drop(work_manager);

                                    //hand control back to the async controller.
                                    tokio::task::yield_now().await;
                                }
                            };


                        }
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

    /// # close
    ///
    /// Closes the web app but does not join with the app handle to ensure that the app was closed.
    ///
    /// Only use this function if you are not in a async environment.
    ///
    /// Otherwise, use the `close` function.
    ///
    /// ## Returns
    ///
    /// This function returns:
    ///
    /// Ok(()) if the app successfully sent a notification to the app thread to stop.
    ///
    /// Err(AppState) if the app was already closed OR if the app failed to send a notification to stop the app thread.
    pub fn close_unchecked(&mut self) -> Result<(), AppState> {
        if self.app_task.is_none() {
            return Err(AppState::Closed);
        }

        let _ = self.app_task.take();
        let _ = self
            .shutdown
            .take()
            .unwrap()
            .send(())
            .map_err(|_| AppState::Running)?;

        Ok(())
    }

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
        let resolution: ResolutionFnRef =
            Arc::new(move |req: Arc<Mutex<Request>>| Box::pin(resolution(req)));

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

        let resolution: ResolutionFnRef =
            Arc::new(move |req: Arc<Mutex<Request>>| Box::pin(resolution(req)));

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

impl Drop for App {
    /// Drops when the application goes out of scope. This is equivalent to calling (&mut self).close
    fn drop(&mut self) {
        let _ = self.close();
    }
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

/// # Handle Client Request
///
/// This function is called whenever a client is accepted from the tcp listener.
///
/// Each time a client is accepted, the request is parsed, a route is found, middleware is called, and a endpoint is resolved.

async fn handle_client_request(
    client: (TcpStream, SocketAddr),
    global_middleware: Arc<Mutex<Vec<MiddlewareClosure>>>,
    router_ref: Arc<Mutex<RouteTree>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut stream, client_socket) = client;

    //process the acception and get the result from the stream
    let request = Arc::new(Mutex::new(
        Request::from_stream(&mut stream, client_socket).await?,
    ));

    //get the function to handle the resolution, backs up to a 404 if existant
    let (cleaned_route, method) = {
        let request_lock = request.lock().await;
        (
            request_lock.route.cleaned_route.clone(),
            request_lock.method.clone(),
        )
    };

    let endpoint = {
        let binding = router_ref.lock().await;

        let route = binding.get_route(&cleaned_route).await;

        match route {
            Some(r) => {
                // This no longer deadlocks because the lock was dropped above
                set_request_variables(request.clone(), r.clone()).await;
                let route_lock = r.lock().await;
                route_lock.brw_resolution(&method)
            }
            None => binding
                .missing_route
                .as_ref()
                .and_then(|mr| mr.brw_resolution(&Method::GET)),
        }
        .and_then(|end_point_ref| Some(end_point_ref.clone()))
    }
    .ok_or(RoutingError::NoRouteExist)?;

    //find any middleware function that when called, returns an Invalid or InvalidEmpty
    let middleware_failed_resolution = {
        //the given back final middleware.
        let mut invalid_middleware = None;

        let global_mw_guard = global_middleware.lock().await;

        //size of all middleware included
        let mware_col_size =
            global_mw_guard.len() + endpoint.middleware.as_ref().map(|mw| mw.len()).unwrap_or(0);

        let mut test_middleware = Vec::with_capacity(mware_col_size);

        test_middleware.extend_from_slice(&global_mw_guard);

        // ! Drop reference once we have all the function refs.
        drop(global_mw_guard);

        if let Some(route_middleware) = &endpoint.middleware {
            test_middleware.extend_from_slice(route_middleware);
        }

        for middleware_closure in test_middleware {
            //call each middleware and map it out
            match middleware_closure(request.clone()).await {
                Middleware::Invalid(res) => {
                    invalid_middleware = Some(res);
                    break;
                }
                Middleware::InvalidEmpty(status_code) => {
                    invalid_middleware = Some(EmptyResolution::status(status_code).resolve());
                    break;
                }
                Middleware::Next => continue,
            };
        }

        invalid_middleware
    };

    //get either the failed middleware, or the endpoint resolution
    let resolved =
        middleware_failed_resolution.unwrap_or((endpoint.resolution)(request.clone()).await);

    //finally resolve this and send the request
    resolve(&mut stream, request, resolved).await?;

    Ok(())
}

/// # Resolve
///
/// Takes a boxed resolution and TcpStream(client)
///
/// The function does the following:
///
/// i. push the transfer encoding header
///
/// ii. write all headers required to the stream
///
/// iii. retrieves the content stream
///
/// iv. loops over the content stream chunk by chunk, writing to the client
///
/// v. writes the termination of the stream when stream ends
async fn resolve(
    stream: &mut TcpStream,
    request: Arc<Mutex<Request>>,
    resolved: Box<dyn Resolution + Send>,
) -> Result<(), std::io::Error> {
    //maps the header from a k,v to a String

    // collect all of our headers from the resolution and the middleware
    let headers = resolved.get_headers();

    let mut req_guard = request.lock().await;

    let mut response_headers = req_guard.take_headers().ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "the headers were already taken",
    ))?;

    // ! no need for the request guard.
    drop(req_guard);

    //insert our headers from the resolution onto our
    for (key, val) in headers {
        response_headers.insert(key, val);
    }

    let first_rep_key = "HTTP/1.1";
    let status = response_headers
        .remove(first_rep_key)
        .map(|s| s.expect("you must include a status"))
        .unwrap_or_else(|| "200 OK".to_string());

    //the header string to convert to bytes
    let mut header_str = String::new();

    let status_header = format!("{first_rep_key} {status}\r\n");
    header_str.push_str(&status_header);

    //Fn to format the headers into a single string
    let format_headers = |(key, val): (String, Option<String>)| {
        let value = match val {
            None => "".to_string(),
            Some(v) => format!(":{v}"),
        };

        format!("{key}{value}")
    };

    //pushes the formatted header into the header_str
    let push_to_str = |s: String| {
        header_str.push_str(&s);
        header_str.push_str("\r\n");
    };

    //converts all the headers into a single string.
    response_headers
        .into_iter()
        .map(format_headers) // map these items to an appropriate format.
        .for_each(push_to_str); //foreach string push onto the string.

    // ? tell the client this is streamed
    header_str.push_str("Transfer-Encoding: chunked\r\n\r\n");

    // ! write the headers to the stream.
    stream.write_all(header_str.as_bytes()).await?;

    let mut content_stream = resolved.get_content();

    //retrieve the next chunk of the body
    while let Some(chunk) = content_stream.next().await {
        let size = chunk.len();

        if size <= 0 {
            continue; //nothing to write 
        }

        //create the size header for the stream chunk
        let size_header = format!("{size:X}\r\n");
        let size_header = size_header.as_bytes();

        //create a buffer that will hold this chunk data
        let mut buffer = Vec::with_capacity(size_header.len() + chunk.len() + 2);

        //the buffer is comprised of the size header, the data chunk, the terminator for the chunk.
        buffer.extend_from_slice(size_header);
        buffer.extend_from_slice(&chunk);
        buffer.extend_from_slice(b"\r\n");

        //write ONCE
        stream.write_all(&buffer).await?;
    }

    //indicate end of stream
    stream.write_all(b"0\r\n\r\n").await?;

    Ok(())
}
