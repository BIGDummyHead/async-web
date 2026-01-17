pub mod app;
pub mod endpoint;
pub mod errors;
pub mod method;
pub mod middleware;
pub mod queue;
pub mod request;
pub mod resolution;
pub mod route;
pub mod router;
pub mod work_manager;
pub mod worker;
pub mod streams;

pub use self::{
    app::App, endpoint::EndPoint, method::Method, middleware::Middleware, queue::Queue,
    request::Request, resolution::Resolution, route::Route, work_manager::WorkManager,
    worker::Worker
};

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
    ($req:ident, moves[$($cap:ident),*], $body:block) => {
        ::std::sync::Arc::new(move |$req| {
            $(let $cap = $cap.clone();)*

            ::std::boxed::Box::pin(async move $body)
        })
    };

    ($req:ident, $body:block) => {
        $crate::resolve!($req, moves[], $body)
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

    // single middleware
    ($req:ident, moves[$($cap:ident),*], $body:block) => {
        ::std::sync::Arc::new(
            move |$req: ::std::sync::Arc<::tokio::sync::Mutex<$crate::web::Request>>| {
                $(let $cap = $cap.clone();)*

                ::std::boxed::Box::pin(async move $body)
                    as ::std::pin::Pin<
                        ::std::boxed::Box<
                            dyn ::std::future::Future<Output = $crate::web::Middleware> + Send
                        >
                    >
            }
        )
            as ::std::sync::Arc<
                dyn Fn(
                    ::std::sync::Arc<::tokio::sync::Mutex<$crate::web::Request>>
                ) -> ::std::pin::Pin<
                    ::std::boxed::Box<
                        dyn ::std::future::Future<Output = $crate::web::Middleware> + Send
                    >
                > + Send + Sync
            >
    };

    // shorthand
    ($req:ident, $body:block) => {
        $crate::middleware!($req, moves[], $body)
    };

    // collection
    ( $( $items:ident ),* ) => {{
        let mut collection: ::std::vec::Vec<
            ::std::sync::Arc<
                dyn Fn(
                    ::std::sync::Arc<::tokio::sync::Mutex<$crate::web::Request>>
                ) -> ::std::pin::Pin<
                    ::std::boxed::Box<
                        dyn ::std::future::Future<Output = $crate::web::Middleware> + Send
                    >
                > + Send + Sync
            >
        > = ::std::vec::Vec::new();

        $( collection.push($items.clone()); )*

        ::std::option::Option::Some(collection)
    }};
}
