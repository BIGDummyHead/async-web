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
* What is the req variable in `resolve!` and `middleware`
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
