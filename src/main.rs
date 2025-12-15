use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use tokio::time::sleep;

use crate::web::{App, Method, Resolution, resolution::FileResolution, route_tree::RouteNode};

pub mod web;

async fn add_routes(app: &mut App) -> () {
    app.add_or_panic(
        "/tasks/users",
        Method::GET,
        Arc::new(|req| {
            Box::pin(async move {
                println!("X: {}", req.route);
                Box::new(FileResolution {
                    file: "tasks.html".to_string(),
                }) as Box<dyn Resolution + Send>
            })
        }),
    )
    .await;

    app.add_or_panic(
        "/tasks",
        Method::GET,
        Arc::new(|req| {
            Box::pin(async move {
                println!("X: {}", req.route);
                Box::new(FileResolution {
                    file: "tasks.html".to_string(),
                }) as Box<dyn Resolution + Send>
            })
        }),
    )
    .await;

    let _ = app
        .add_or_change_route(
            "/",
            Method::GET,
            Arc::new(|req| {
                Box::pin(async move {
                    println!("{}", req.route);
                    sleep(Duration::from_secs(2)).await;
                    Box::new(FileResolution {
                        file: "home.html".to_string(),
                    }) as Box<dyn Resolution + Send>
                })
            }),
        )
        .await;

    app.get_router().await.add_missing_route(Arc::new(|req| {
        Box::pin(async move {
            println!("{}", req.route);
            Box::new(FileResolution {
                file: "404.html".to_string(),
            }) as Box<dyn Resolution + Send>
        })
    }));
}

async fn create_local_app() -> App {
    let addr = Ipv4Addr::new(127, 0, 0, 1);
    let port = 8080;

    let app_bind = App::bind(3, SocketAddrV4::new(addr, port)).await;

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
