use std::collections::{HashMap};
use std::io::Cursor;
use std::sync::Arc;

use async_web::middleware;
use async_web::web::errors::AppState;
use async_web::web::resolution::empty_resolution::EmptyResolution;
use async_web::web::resolution::error_resolution::{Configured, ErrorResolution};
use async_web::web::resolution::file_resolution::FileResolution;
use async_web::web::{App, Method, Middleware, Request, Resolution};
use local_ip_address::local_ip;
use tokio::sync::{Mutex, MutexGuard};

pub mod api_call;
pub mod loaded_model;
pub mod model;
pub mod token_output_resolution;
pub mod token_output_stream;

use crate::api_call::{ApiHandler};

use crate::loaded_model::LoadedModel;
use crate::token_output_resolution::TokenOutputResolution;

#[tokio::main]
async fn main() -> Result<(), AppState> {
    let mut app = route_app().await;

    let _ = app.start()?;

    loop {
        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);

        break;
    }

    app.close().await?;

    Ok(())
}

/// Creates a local app on the current IP address on port 80. 
/// 
/// Then routes the application with the following routes:
/// 
/// POST: /alt -> with body -> caption an image.
/// GET: / -> public/index.html
/// GET: /{file} -> public/{file}
async fn route_app() -> App {
    //get local address and worker_count
    let address = local_ip()
        .map(|ip| format!("{ip}:80"))
        .expect("Could not get computer IP address.");

    println!("Hosting on: http://{address}");

    let worker_count = 9000;

    let app = App::bind(worker_count, &address)
        .await
        .expect("App failed to bind to address.");

    let loaded_model = Arc::new(Mutex::new(LoadedModel::create().await));

    //api calls that have happened.
    let api_calls: Arc<Mutex<HashMap<String, ApiHandler>>> = Arc::new(Mutex::new(HashMap::new()));
    let api_calls_clone = api_calls.clone();

    // middleware that ensures the user cannot make a ridiculous amount of calls per hour.
    let limit_api_calls = middleware!(req, moves[api_calls_clone], {
        let ip_addr: String = {
            let guard: MutexGuard<'_, Request> = req.lock().await;

            match guard.client_socket {
                std::net::SocketAddr::V4(addr) => addr.ip().to_string(),
                std::net::SocketAddr::V6(addr) => addr.ip().to_string(),
            }
        };

        // ! remember to drop the lock.
        let mut api_guard = api_calls_clone.lock().await;

        //insert a new handler to the map
        if !api_guard.contains_key(&ip_addr) {
            //2 calls per minute. 120 calls per hour.
            let max_calls = 2;
            let time_frame = std::time::Duration::from_mins(1);
            api_guard.insert(ip_addr.clone(), ApiHandler::new(max_calls, time_frame));
        }

        //get the api call, this should be expected to always have the IP address.
        let api_handle: Result<Middleware, Middleware> =  
        api_guard
        .get_mut(&ip_addr)
        .unwrap()
        .make_call() 
        .map_err(|e| {
            Middleware::Invalid(ErrorResolution::from_error(e, Configured::Json).resolve())
        })
        .map(|_| Middleware::Next);

        //drop the api calls lock
        drop(api_guard);

        api_handle.unwrap_or_else(|m| m)
    });

    //post resolution that takes a body (image data) and gives back a stream of strings (tokens) to caption said image bytes.
    app.add_or_panic(
        "/alt",
        Method::POST,
        middleware!(limit_api_calls),
        move |req| {
            //load in the model for usage.
            let loaded_model = loaded_model.clone();
            async move {
                // take the request body, don't want to really copy it
                let body = {
                    let mut guard = req.lock().await;
                    std::mem::take(&mut guard.body)
                };

                //tell the frontend that the request body was empty.
                if body.is_empty() {
                    return ErrorResolution::from_error(
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Request body is empty",
                        ),
                        Configured::Json,
                    )
                    .resolve();
                }

                let file_data = Cursor::new(body);

                //send the file data and loaded model and create a streamed output from the image captioner.
                let result = tokio::task::spawn_blocking(move || {
                    TokenOutputResolution::stream(file_data, loaded_model).resolve()
                })
                .await
                .map_err(|e| ErrorResolution::from_error(e, Configured::PlainText).resolve());

                result.unwrap_or_else(|r| r)
            }
        },
    )
    .await;

    //homepage
    app.add_or_change_route("/", Method::GET, None, |_req| async move {
        FileResolution::new("public/index.html").resolve()
    })
    .await
    .expect("could not change home page.");

    //get content files.
    app.add_or_panic("/{file}", Method::GET, None, |req| async move {
        let file_name = match req.lock().await.variables.get("file") {
            None => return EmptyResolution::status(404).resolve(),
            Some(v) => v.to_string(),
        };

        FileResolution::new(&format!("public/{file_name}")).resolve()
    })
    .await;

    app
}
