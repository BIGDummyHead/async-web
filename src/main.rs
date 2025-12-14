use std::net::{Ipv4Addr, SocketAddrV4};

use crate::web::{App, Method, Resolution, resolution::FileResolution};

pub mod web;

async fn add_routes(app: &mut App) -> () {
    let _ = app.get_router().await.add_route(
        "/tasks",
        Some((
            Method::GET,
            Box::new(|req| {
                Box::pin(async move {
                    println!("{}", req.route);
                    Box::new(FileResolution {
                        file: "tasks.html".to_string(),
                    }) as Box<dyn Resolution + Send>
                })
            }),
        )),
    );

    let _ = app
        .get_router()
        .await
        .get_route("/")
        .unwrap()
        .insert_resolution(
            Method::GET,
            Box::new(|req| {
                Box::pin(async move {
                    println!("{}", req.route);
                    Box::new(FileResolution {
                        file: "home.html".to_string(),
                    }) as Box<dyn Resolution + Send>
                })
            }),
        );
}

async fn create_local_app() -> App {
    let addr = Ipv4Addr::new(127, 0, 0, 1);
    let port = 8080;

    let app_bind = App::bind(100, SocketAddrV4::new(addr, port)).await;

    if let Err(e) = app_bind {
        panic!("Could not bind app! {e}");
    }

    app_bind.unwrap()
}

#[tokio::main]
async fn main() {
    let mut app = create_local_app().await;

    add_routes(&mut app).await;

    let _ = app.start().await.await;
}
