pub mod app;
pub mod errors;
pub mod resolution;
pub mod routing;
pub mod streams;

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex;

use crate::web::{
    resolution::{
        empty_resolution::EmptyResolution,
        error_resolution::{Configured, ErrorResolution},
        file_resolution::FileResolution,
        json_resolution::JsonResolution,
        redirect::{Redirect, RedirectType},
    },
    routing::middleware::MiddlewareClosure,
};

pub use self::{
    app::App, resolution::Resolution, routing::method::Method, routing::middleware::Middleware,
    routing::request::Request, routing::route::Route, routing::router::endpoint::EndPoint,
};

/// ## resolve!
///
/// Shorthand for writing a route resolver.
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
///
/// Allows for shorthand collection of middleware collection for example.
///
/// ```
///
/// let m_w1 = middleware(|req| async move {
///     Middleware::Next
/// });
///
/// let m_w2 = middleware(|req| async move {
///     Middleware::Next
/// });
///
/// //allows for the collection of vec![m_w1, m_w2]
/// app.add_or_panic("/api", Method::GET, middleware!(m_w1, m_w2), |req| async move {...});    
///
/// ```
///
#[macro_export]
macro_rules! middleware {

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

/// # Middleware
///
/// Allows for the creation of middleware closures.
///
/// Example:
///
/// ```
///     let mw_1 = middleware(|req| async move {
///         Middleware::InvalidEmpty(403)
///     });
///
///     //or moving some value
///     let some_ref = Arc::new(10);
///     
///     let mw_2 = middleware(move |req| {
///         let some_ref = some_ref.clone();
///
///         async move {
///             Middleware::Next
///         }
///     });
/// ```
pub fn middleware<F, Fut>(f: F) -> MiddlewareClosure
where
    F: Fn(Arc<Mutex<Request>>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Middleware> + Send + Sync + 'static, //middleware future
{
    Arc::new(move |req: Arc<Mutex<Request>>| Box::pin(f(req)))
}

pub type Resolved = Box<dyn Resolution + Send + 'static>;

/// # Status
///
/// Short for `EmptyResolution::status(code)`
pub fn status(code: i32) -> impl Resolution {
    EmptyResolution::status(code)
}

/// # Serialized
///
/// Short for:
///
/// ```
/// JsonResolution::serialize(value)
/// ```
pub fn serialized<V>(value: V) -> Result<JsonResolution, ErrorResolution>
where
    V: Serialize,
{
    JsonResolution::serialize(value)
}

/// # Error
///
/// Short for `ErrorResolution::from_error(error, configured)`
///
/// Note: Code is 500 by default, see `error_status`
pub fn error<E, C>(error: E, configured: C) -> ErrorResolution
where
    E: std::error::Error + 'static,
    C: Into<Option<Configured>>,
{
    ErrorResolution::from_error(error, configured)
}

/// # Error Status
///
/// Short for:
///
/// ```
/// let mut err = ErrorResolution::from_error(error, configured);
///
/// err.code = code;
///
/// err
/// ```
pub fn error_status<E, C>(err: E, configured: C, code: i32) -> impl Resolution
where
    E: std::error::Error + 'static,
    C: Into<Option<Configured>>,
{
    let mut res = error(err, configured);
    res.code = code;

    res
}

/// # Resolve
///
/// Short for `resolution.resolve()`
pub fn resolve(to_resolve: impl Resolution) -> Resolved {
    to_resolve.resolve()
}

/// # File
///
/// Short for `FileResolution::new(file)`
pub fn file(file: &str) -> impl Resolution {
    FileResolution::new(file)
}

/// # Redirect
///
/// Short for `Redirect::new(RedirectType::SomeRedir)`
pub fn redirect(redir_type: RedirectType) -> impl Resolution {
    Redirect::new(redir_type)
}
