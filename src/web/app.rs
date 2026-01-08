use std::sync::Arc;

use futures::StreamExt;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard},
    task::{self, JoinHandle},
};

use crate::web::{
    EndPoint, Method, Middleware, Request, Resolution, WorkManager,
    errors::{RoutingError, routing_error::RoutingErrorType},
    middleware::{MiddlewareClosure, MiddlewareCollection},
    resolution::empty_resolution::EmptyResolution,
    router::{ResolutionFunc, RouteNodeRef, RouteTree},
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
    pub listener: Arc<TcpListener>,
    pub router: Arc<Mutex<RouteTree>>,
    global_middleware: Arc<Mutex<Vec<MiddlewareClosure>>>,
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

        let work_manager = Arc::new(WorkManager::new(worker_count, Some(100)).await);

        let listener = Arc::new(bind_result.unwrap());
        let router = Arc::new(Mutex::new(RouteTree::new(None)));

        let bind = Self {
            work_manager,
            listener,
            router,
            global_middleware: Arc::new(Mutex::new(Vec::new())),
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

    async fn process_acception(mut stream: &mut TcpStream) -> Result<Request, std::io::Error> {
        let request_result = Request::parse_request(&mut stream).await;

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
        let route_parts: Vec<String> = req_ref
            .lock()
            .await
            .route
            .cleaned_route
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

    /// Starts the main TCP accept loop for the application.
    ///
    /// Each accepted connection is submitted to the work manager for processing.
    ///
    /// # Returns
    ///
    /// A `JoinHandle` referencing the spawned server task.

    pub async fn start(&self) -> JoinHandle<()> {
        let listener = self.listener.clone();
        let work_manager = self.work_manager.clone();
        let router = self.router.clone();
        let global_middleware = self.global_middleware.clone();

        task::spawn(async move {
            loop {
                let client_result = listener.accept().await;

                if let Err(c_err) = client_result {
                    eprintln!("Failed to connect client: {c_err}");
                    continue;
                }

                let (stream, _) = client_result.unwrap();

                let router_ref = router.clone();
                let middleware_ref = global_middleware.clone();

                work_manager
                    .add_work(Box::pin(async move {
                        Self::request_work(stream, middleware_ref, router_ref).await;
                    }))
                    .await;
            }
        })
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
        mut stream: TcpStream,
        global_middleware: Arc<Mutex<Vec<MiddlewareClosure>>>,
        router_ref: Arc<Mutex<RouteTree>>,
    ) -> () {
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
                        final_middleware = Some(EmptyResolution::new(status_code));
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

/// ## resolve!
///
/// Shorthand for writing a route resolver!
///
/// ### Common "long"-hand:
///
/// ```
///
///     //create a route that throws an internal error!
///     let r = Arc::new(move |req| {
///         Box::pin(async move {
///             EmptyResolution::new(500)
///         })
///     })
///
///     //assume we have an app already made
///     app.add_or_panic("/test/this", Method::GET, None, r);
///
///
/// ```
///
/// ### Short Hand (with macro)
///
/// `Note: this does not capture any variables!`
///
/// ```
///
///     //create a route that throws an internal error
///     let r = resolve!(req, {
///         EmptyResolution::new(500)
///     });
///
///     //assume we have an app already made
///     app.add_or_panic("/test/this", Method::GET, None, r);
///
///
/// ```
///
/// ### Short Hand Capture (with macro)
///
/// Suppose you want to move a value from the program into the route, but you cannot with the basic `resolve!(req, { res })` macro.
///
/// `Note: this clones each moved value`
///
/// ```
///     
///     //create a variable
///     let counter = 0;
///     let outter_mut_var = Arc::new(Mutex::new(counter))
///
///     //create a value to move
///     let omv_clone = outter_mut_var.clone();
///
///     //this route throws an internal error and moves the omv_clone variable
///     let r = resolve!(req, moves[omv_clone], {
///         
///         let count = omv_clone.lock().await;
///         *count += 1;
///         println!("this function has been called {} times", *count);
///
///         EmptyResolution::new(500);
///     });
///
///     //assume we have an app already made
///     app.add_or_panic("/test/this", Method::GET, None, r);
///
///
/// ```
///
///
#[macro_export]
macro_rules! resolve {

    ($req: ident, moves[$($cap:ident),*], $body:block) => {

        Arc::new(move |$req| {

            $(let $cap = $cap.clone();)*

            Box::pin(async move $body)
        })

    };

    ($req:ident, $body:block) => {
        resolve!($req, moves[], $body)
    };
}

/// ## middleware!
/// 
/// This macro is responsible for giving you the ability to write middleware via shorthand. 
/// 
/// It works in the same fashion as resolve!, giving you the ability (and option) to capture elements around the function.
/// 
/// ### Example "long"-hand
/// For example, if we want some middleware applied to a route it may look like this:
/// 
/// ```
///     let check_auth = Arc::new(move |req| {
///         //capture elements here
///         
///         Box::pin(async move {
///              //check for authentication, move forward
///             Middleware::Next 
///         }) 
///     });
/// 
///     //then add a route
///     app.add_or_panic("/test", Method::GET, Some(vec![check_auth.clone()]), my_route).await;
/// 
///     
/// ```
/// 
/// In this example we have just added a route with a singular middleware item. This is the best way to write to have full control of the evenironment.
/// 
/// However it is by far the most tedious way of doing so.
/// 
/// ### Short Hand Examples 
/// 
/// In essense, we instead want to write code that is meaningful, short, and less ambigious.
/// 
/// So we can use the middleware! macro.
/// 
/// The macro can be used in two ways:
/// 
/// ```
///     
///     //middleware to check if the user is authenticated
///     let check_auth = middleware!(req, {
///         Middleware::Next;
///     });
///     
///     //create some variables to capture
///     let counter_ref = Arc::new(Mutex::new(0));
/// 
///     let counter_ref_clone = counter_ref.clone();
/// 
///     //it is important to note that these are cloned
///     let is_admin = middleware!(req, moves[counter_ref_clone], {
///         //deny 
///         Middleware::InvalidEmpty(403);
///     });     
/// 
///     //we can also use middleware as a collective
///     // This type of middleware! will give us Some(vec![...])
///     app.add_or_panic("/test", Method::GET, middleware!(check_auth, is_admin), ...).await;
/// 
///     
/// 
/// ```
#[macro_export]
macro_rules! middleware {

    //creates the body of middleware
    ($req:ident, moves[$($cap:ident),*], $body:block) => {
        Arc::new(move |$req: Arc<tokio::sync::Mutex<crate::web::Request>> | {
          Box::pin(async move $body) as std::pin::Pin<Box<dyn std::future::Future<Output = crate::web::Middleware> + Send>>
        }) as Arc<dyn Fn(Arc<tokio::sync::Mutex<crate::web::Request>>) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::web::Middleware> + Send>> + Send + Sync>
    };

    //short hand for no collection
    ($req:ident, $body:block) => {
        middleware!($req, moves[], $body)
    };

    //collection of middleware
    ( $( $items:ident ),* ) => {

        {
            let mut collection: Vec<Arc<dyn Fn(Arc<tokio::sync::Mutex<crate::web::Request>>) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::web::Middleware> + Send>> + Send + Sync>> = vec![];

            $( collection.push($items.clone()); )*

            Some(collection)
        }
    };
}

impl App {
    /// Adds a new route or replaces an existing route’s resolution for the given method.
    ///
    /// If the route already exists, its resolution for the specified method is overwritten.
    ///
    /// # Errors
    ///
    /// Returns a `RoutingError` if the route cannot be added.

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

    /// Adds a new route or replaces an existing route’s resolution for the given method.
    ///
    /// If the route already exists, its resolution for the specified method is overwritten.
    ///
    /// # Errors
    ///
    /// Returns a `RoutingError` if the route cannot be added.
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

    /// Adds a route and method combination to the router.
    ///
    /// # Panics
    ///
    /// Panics if the route already exists or cannot be added.
    /// Intended for use during application initialization.

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

    /// Provides exclusive access to the internal route tree.
    ///
    /// Returns a locked guard allowing inspection or modification of routing state.
    /// This call blocks until the router mutex becomes available.

    pub async fn get_router(&self) -> MutexGuard<'_, RouteTree> {
        self.router.lock().await
    }
}
