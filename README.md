# async-web

A minimal, Express.js like RUST web server library. Allowing for you to create a server, route it, server custom request, and do much more.

Allows for ultimate control over your application with minimal overhead.

Offers:

* A Routing Tree
* Routing Methods
* Custom Resolutions
* Custom Middleware
* Async workers to handle multiple heavy request at once
* Auto worker scaling to ensure work is never blocked

## Creating a Server

You must first bind the app to the socket you want to use.

```rust

async fn() -> Result<(), AppState> {
    //bind the application to the socket.
    //i would recommend the IpAddr crate https://doc.rust-lang.org/std/net/enum.IpAddr.html
    let mut app = App::bind("127.0.0.1:80").await?;
    --snip--
```

Now that you have an app, you can add routes to it, with methods, middleware, and resolutions!

```rust
    --snip--
    
    let middle_ware = None;
    app.add_or_change_route("/", Method::GET, middle_ware, |req| async move {
        //serve a resolution:
        FileResolution::new("public/index.html").resolve() //-> returns a boxed dyn Resolution
    }).await.expect("could not change home page"); //this error is thrown if the home page was already routed @ get.

    app.add_or_panic("/{folder}/{*}", Method::GET, None, |req| async move {

        //get the contents from the route.
        let (folder, path) = {
            //drops after folder and path found
            let guard = req.lock().await;

            (
                //in this scenario, if we assume that the route was reached, these values must exist.
                guard.variables.get("folder").map(|f| f.to_string()).unwrap(),
                guard.variables.get("*").map(|f| f.to_string()).unwrap(),
            )
        };

        //create a file resolution and serve it. If this file does not exist 404 is returned.
        let path = format!("{folder}/{path});
        FileResolution::new(&path).resolve()
    });

    --snip---
```

In these two examples we use the `FileResolution` struct, which implements the `Resolution` (where `.resolve()` is implemented) trait.

However, the library has the following Resolutions pre-built for common resolutions:

* EmptyResolution
* ErrorResolution
* FileResolution
* JsonResolution

These resolutions are pretty light weight and dynamic.

### Empty Resolution

Simply returns no content to the user, but provides a status. 

`EmptyResolution::status(stats_code:i32).resolve()`

### Error Resolution

Returns a resolution with the status of 500, this is subject to change however.

These can be created in two ways:

* `ErrorResolution::from_error<T>(error: T, configured: impl Into<Option<Configured>>).resolve()`
* `ErrorResolution::from_boxed(error: Boxed<std::error::Error>, configured: impl Into<Option<Configured>>).resolve()`

This struct is useful because you can map errors into error resolutions and then serve them.

`Configured` is an enum that has 3 values

* `Configued::PlainText` the error to string is served back
* `Configured::Json` the error has an error message and status code (500)
* `Configured::Custom(Box<dyn Fn(&Box<dyn std::error::Error + Send>) -> String + Send>)` which is a function that changes our error into a String and is then served

### File Resolution

Serves a file back to the user with a status code (200 or 404).

This resolution is very dynamic as it will resolve the file type headers, code, and serve the file for you!

`FileResolution::new("path/to_file.html").resolve()`

### Json Resolution

Converts a value into a JSON string.

`JsonResolution::serialize<T>(value: T)` where T : serde_json::Serialize

It is important to note, that this value does not return `Self` instead, it returns `Result<Self, ErrorResolution>` if the serialization failed.

This is good to know because we can return the error resolution (configured to Json) back to the user if it did not serialize properly. 


## Creating our own Resolution

Now that we know how a resolution works, we can create our own.

For example if we want to create a resolution that slowly serves a string back to the user.

```rust

use async_stream::stream;
use async_web::web::{Resolution, resolution::get_status_header};

pub struct SlowString {
    serve: String
}

impl SlowString {
    pub fn new(serve: &str) -> Self {
        Self { serve: serve.to_string() }
    }

    pub fn serve(serve: &str) -> Box<dyn Resolution + Send + 'static> {
        Self::new(serve).resolve()
    }
}

impl Resolution for SlowString {
    // return any headers that are required to give back the content. For this we just serve a 200 since it will always be okay
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move {
            vec![get_status_header(200)]
        })        
    }

    //slowly stream the content back to the user.
    fn get_content(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = Vec<u8>> + Send>> {
        
        //clone this, so it is not moved.
        let serve = self.serve.clone();

        //stream the content chars.
        let content_stream = stream! {

            let mut serve_chars = serve.chars();

            while let Some(sc) = serve_chars.next() 
            {
                //yield and wait
                yield format!("{sc}").into_bytes();
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        };

        Box::pin(content_stream)
    }

    //turn this into a box and do whatever else you need to do to resolve.
    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
}

```

We can then use this just like every other resolution.

`SlowString::new("Epic resolution test").resolve()` or if we wanted `SlowString::serve("Epic resolution test")`

But you can use this to stream any type of content back to the client.

## Creating Middleware

Creating middleware is a bit different than creating a resolution. Mainly due to the fact we can intercept and interact with the request before meeting the resolution.

Let's create two steps in our middleware, the first will print the user IP, then the second will block the request from resolving, just cause.

```rust

--snip``

let step = middleware(|req| async move {

    // the requesting IP from their socket.
    let ip = {
        let guard = req.lock().await;
        guard.client_socket.ip().to_string()
    };

    println!("user requested from: {ip}");

    //indicate it can move formward to step2 or whatever middleware is next.
    Middleware::Next
});

//serve invalid middleware with a 400 request.
//the request is stopped here and the middleware is served to the client.
let step2 = middleware(|req| async move {
    Middleware::InvalidEmpty(400)
});

let global = middleware(|req| async move {
    println!("This is global middleware");
    Middleware::Invalid(EmptyResolution::status(500).resolve()) //we can also serve middleware with a resolution object.
});

//now no request passes through and all see the 500 status.
app.use_middleware(global); //DISABLE THIS LINE IF YOU WANT TO SEE THE OTHER MIDDLEWARE WORK

//assume that we have created an app and are routing it
app.add_or_panic("/api/create", Method::POST, middleware!(step, step2),
|req| async move {
    EmptyResolution::status(200).resolve()
});

```

## Starting and Closing your App

Once our app has been binded and routed with all of our routes and middleware, we can start the app!

```rust

--snip--

//assume this function binds and routes.
let mut app = route_app().await;

let start_result: AppState = app.start()?; //has an error if the app could not start OR the app was already running

loop {
    let mut buffer = String::new();
    let _ = std::io::stdin().read_line(&mut buffer);

    break; //kill on enter
}

//stops the app gracefully, then awaits the thread to close.
app.close().await?;

Ok(())

--snip--

```

## Examples

If you are interested in use the library.

I have created two grate examples that you can start with. 

* [image analyzer](https://github.com/BIGDummyHead/async-web/tree/master/examples/image-analyzer), allows for the upload of images and automatically captions them.
  * Covers serving files, serving content files for the index.html, creating/serving a custom resolution,  creating a limiting api call middleware.
* [screen share](https://github.com/BIGDummyHead/share-screen), serves a screen sharing interface that allows us to view a Windows WebCam or Screen!
  * Covers custom resolutions, app routing, and a CLI for screen sharing 
