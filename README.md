# async-web

Minimal asynchronous web server framework in Rust built on Tokio. It exposes a small router, middleware pipeline, and a worker-driven executor that schedules per-connection tasks.

# Getting Started

To get started all we need is a valid Socket and the tokio package, which can be installed via:

```
cargo add tokio --features full

```

To give you the main function attribute for async as will be used in the following examples.

`You should assume all examples given are in an async function that return Result<(), Box<dyn std::error::Error>>`

## Binding an App

This is the most crucial of any step. When binding your server you are creating workers, router trees, middleware, and various other items to enhance the flow of the application.

```rust

    //thread pool count, this is the amount of workers you want processing work.
    let worker_count = 100;

    // you may also use IPAddr4 or anything that implements ToSocketAddr
    let addr = "127.0.0.1:8080";

    //we will bind the app with a worker count and socket    
    let app = App::bind(worker_count, addr).await?;

```

## Routing your App

Once an application has been bound, we are allowed to create routes, this may seem familiar if you have used something like express before where we add routes, methods, middleware, and some kind of resolution.

`You may also use routing variables but we will explore this in later examples.`

There are three different functions you may use to add a route, one of these functions can be used to change the route however.

For example, let's add three different routes, using the three different functions and explore their outcomes.

` Note: Two routes intersect when their route path "/path" and method match Method::GET. It is important to note that multiple of the same path may exist with different Methods     ` 

```rust

    // --snip--
    // assume we have bound an app.

    //assume that no_middleware = Option::None

    //here we make a call to add_or_change_route, this is pretty explicit with what it will do, note that it returns an error for YOU to handle incase anything is wrong with the provided route
    //you however, do not receive an error if the route exist, as it does not care and will override it.
    let changed: Result<(), async_web::web::errors::RoutingError> = app
        .add_or_change_route(
            "/home", //change the home page
            Method::GET, //method, GET, POST, PUT, DELETE, etc... with exception to enum item Other(String)
            no_middleware, // None
            resolve!(req, {
                //tell the server to serve a 200 message.
                EmptyResolution::new(200)
            }),
        )
        .await;

    //here we try to add this route, the RoutingError is again handed back to YOU. This time however, a new error may be present in the chance that the route already exist!
    //in this case we would have a route conflict because '/home' with the method GET already exist.
    let changed: Result<(), async_web::web::errors::RoutingError> = app
        .add_route(
            "/home",
            Method::GET,
            no_middleware,
            resolve!(req, { EmptyResolution::new(200) }),
        )
        .await;

    //finally, my preferred choice, add_or_panic.
    //in this scenario we panic if the route has an error or it already exist. In my opinion this is the best way to add routes IF you are adding routes on startup only. 
    app
        .add_or_panic(
            "/home",
            Method::POST, //note that we are using a different method
            no_middleware,
            resolve!(req, { EmptyResolution::new(200) }),
        )
        .await;
```

## Middleware

You may have noticed that in the previous examples we passed `no_middleware` to each route or `Option::None` (as stated)

But what if we want to have middleware, and what exactly is it?

Middleware is a way to reuse functions in between routing to your app route.

This library uses it in this fasion:

`User Request Route -> Route Exist -> Invoke Global Middlewares -> Invoke Route Specific Middleware -> Invoke Resolution`

It is to be noted that if the middleware (as will see soon) stops at any point the `Resolution` is never reached.

### Global 

```rust

    //here we can create a singular middleware function that will be used to check if the requester is logged in. Of course it is just a snippet.
    let check_logged_in = middleware!(req, {
        //--snip do code to make sure logged in--
        // if so move next
        Middleware::Next 
    });

    //this indicates the middleware will be used for every route call
    app.use_middleware(check_logged_in).await;

```

### Singular

```rust

    //check's if the user is an admin
    let check_is_admin = middleware!(req, {
        //--snip checks if admin, if admin move next--
        Middleware::Next
    });

    //checks if the admin has access to the resource
    let has_access = middleware!(req, {
        //--snip-- user was denied access for xyz
        //note how we can return a resolution here, we will go into resolutions later on...
        //you may also use Middleware::InvalidEmpty(403) to indicate the same effect.
        Middleware::Invalid(EmptyResolution::new(403))
    });

    app.add_or_panic(
        "/admin/home",
        Method::POST, //note that we are using a different method
        middleware!(check_is_admin, has_access), //note how we can reuse the middleware! macro to conjoin our middleware.
        resolve!(req, { EmptyResolution::new(200) }),
    )
    .await;

```


## Starting our App

Now that you understand how to bind the app, add middleware, and route the app. We can look into how to start this thing!

You probably also have questions about other things like:

* Moving values between `resolve!` and `middleware!`
* What else can routes return?
* How does one use variables in their route?
* How can I create a custom Resulition?

These will all be covered, but for now let's focus on starting this app.

```rust

//start the app is simple, however can be confusing with using await

//firstly here, we start the app and use the await to invoke the future, this provides us with the spawned task or the 'app_thread"
let app_thread = app.start().await;

// we may use the app_thread on a separate task, or we can await that thread on our main function to keep the program alive!
//the power and choice is in your hands on how you want to run the app
let _ = app_thread.await;

```

### More examples to come...

## Moving Values 

Sometimes we need to move and clones values from one scope to another. However you cannot do this without using the `moves` variable in the `middleware!` and `resolve!` macro.

```rust

//this example of moves would work for both middleware!/resolve!

let moved_value = Arc::new(String::from("Test"));
let other_moved_value = Arc::new(String::from("Test"));

app.add_or_panic(
    "/admin/home",
    Method::POST, //note that we are using a different method
    None,
    resolve!(req, moves[moved_value, other_moved_value] {
        //it is important to note that moved_value/other_moved_value ARE moved however, they are also cloned. 
        EmptyResolution::new(200)
    }),
    )
    .await;

```

## Resolving a Route

You may have noticed that `route!` in all the examples use the `EmptyResolution::new(200)` value. Meaning to tell the request 200 (ok) with no content.

However, we use other pre-made resolutions in the library such as:

* FileTextResolution::new(&str) -> resolved into raw `utf8` text.
* FileResolution::new(String) -> resolved into `Vec<u8>` and sets appropriate headers.
* EmptyResolution::new(i32) -> resolves into an empty resolution with the appropriate status header.
* JsonResolution::new<T>(T) -> Result<Self, serde_json::Error>. This does not directly resolve into a resolution. You must call `into_resolution` 

Each of these pre-included resolutions implement the `Resolution` trait. This trait includes 2 functions to implement:

* `fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>>`
* `fn get_content(&self) ->  Pin<Box<dyn Stream<Item = Vec<u8>> + Send + 'static>>`

### Creating a Resolution

Creating a resolution is rather simple. Below is a "StreamedResolution" example. 

Below you will find an example in which we take a receiver, receive data from it, and yield it to the resolution.


```rust

// Using tokio (with features 'full') and tokio_stream
use std::sync::Arc;

use async_stream::stream;
use async_web::web::{Resolution, resolution::get_status_header};
use tokio::sync::{Mutex, broadcast::Receiver};

// Struct that includes an Async Safe Mutatable Receiver (tokio::sync::broadcast) of compressed framed data
pub struct StreamedResolution {
    rx:  Arc<Mutex<Receiver<Vec<u8>>>>,
}

impl StreamedResolution {
    /// Pass the receiver subscription into the StreamResolution structure.
    /// Returns the instance of the resolution trait boxed.
    pub fn new(rx: Receiver<Vec<u8>>) -> Box<dyn Resolution + Send> {
        let res = Self { rx: Arc::new(Mutex::new(rx)) };
        Box::new(res)
    }
}

/// Resolution implementation for StreamedResolution
impl Resolution for StreamedResolution {

    // Get Headers
    // Headers to send to the requester. This fortunately will always be 200.
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        //Box and pin the function that returns the single status header string.
        Box::pin(async move { vec![get_status_header(200)] })
    }

    /// Get Content
    /// Content function that will be invoked to get the streamed data.
    fn get_content(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = Vec<u8>> + Send>> {

        //create a clone of the receiver 
        let rx = self.rx.clone();

        // stream! macro from tokio_stream
        let content_stream = stream! {
    
            //loop to yield compressed data from the receiver.
            loop {

                //lock the receiver guard, recv the data and drop 
                let data = {
                    let mut guard = rx.lock().await;
                    guard.recv().await
                };

                //possible error from compression, okay to continue
                if data.is_err() {
                    continue;
                }

                let data = data.unwrap();

                //yield our unpacked data.
                yield data;

            }
        };

        //pin our stream.
        Box::pin(content_stream)
    }
}

```

#### Using our new Resolution

```rust

    //clone our broadcaster
    let compressed_frame_rx_clone = compressed_frame_rx.clone();

    //streamed POST for the content of the device
    app.add_or_panic(
        "/stream", //path
        Method::POST,
        None,
        resolve!(_req, moves[compressed_frame_rx_clone], {

            //create a receiver we can pass to the streamed resolution
            let rx = compressed_frame_rx_clone.subscribe();

            //return our new resolution
            StreamedResolution::new(rx)
        }),
    )
    .await;
```

Our frontend code (JavaScript) may look like this:

```js

async function readStream() {

    //fetch our stream
    const response = await fetch("/stream", { method: "POST" });

    //get the body reader
    const reader = response.body.getReader();

    //keep reading until done
    while (true) {
        
        const { value, done } = await reader.read();

        if (done)
            break;

        --snip--
    }
}
```

