use std::io::Cursor;
use std::sync::Arc;

use async_web::web::errors::AppState;
use async_web::web::resolution::empty_resolution::EmptyResolution;
use async_web::web::resolution::file_resolution::FileResolution;
use async_web::web::{App, Method, Resolution};
use local_ip_address::local_ip;
use tokio::sync::Mutex;

pub mod alt_text;
pub mod loaded_model;
pub mod model;
pub mod token_output_resolution;
pub mod token_output_stream;

use crate::alt_text::AltText;
use crate::loaded_model::LoadedModel;
use crate::token_output_resolution::TokenOutputResolution;

#[tokio::main]
async fn main() -> Result<(), AppState> {
    let mut app = create_local_app().await;

    let _ = app.start()?;

    loop {
        let mut buffer = String::new();
        let _ = std::io::stdin().read_line(&mut buffer);

        break;
    }

    app.close().await?;

    Ok(())
}

/// # Create Local App
///
/// Binds the app to the local machine to PORT 8080. It then creates a route for `/alt` that takes a file as a body.
async fn create_local_app() -> App {
    //get local address and worker_count
    let address = local_ip()
        .map(|ip| format!("{ip}:80"))
        .expect("Could not get computer IP address.");

    println!("Hosting on: http://{address}");

    let worker_count = 9000;

    let app = App::bind(worker_count, &address)
        .await
        .expect("App failed to bind to address.");

    let loaded_model = Arc::new(Mutex::new(LoadedModel::new().await));

    app.add_or_panic("/alt", Method::POST, None, move |req| {
        let loaded_model = loaded_model.clone();
        async move {
            // get the request body
            let body = {
                let mut guard = req.lock().await;

                std::mem::take(&mut guard.body)
            };

            if body.is_empty() {
                return AltText::with_error("No request body found!".to_string()).resolve();
            }

            let file_data = Cursor::new(body);

            let loaded_model = loaded_model.clone();
            let result = tokio::task::spawn_blocking(move || {
                // This runs on a dedicated thread, not blocking the async web server
                TokenOutputResolution::stream(file_data, loaded_model).resolve()
            })
            .await
            .unwrap();

            result
        }
    })
    .await;

    let _ = app
        .add_or_change_route("/", Method::GET, None, |_req| async move {
            FileResolution::new("public/index.html").resolve()
        })
        .await;

    app.add_or_panic("/{file}", Method::GET, None, |req| async move {
        let file_name = match req.lock().await.variables.get("file") {
            None => return EmptyResolution::status(404).resolve(),
            Some(v) => v.to_string()
        };

        FileResolution::new(&format!("public/{file_name}")).resolve()
    }).await;

    app
}
