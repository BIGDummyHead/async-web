use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::web::{
    App, EndPoint, Method, Middleware, Request,
    middleware::MiddlewareClosure,
    resolution::{file_resolution::FileResolution, json_resolution::JsonResolution},
};

pub mod web;

#[derive(Serialize, Deserialize)]
pub struct Person {
    name: String,
    age: i32,
}

async fn add_routes(app: &mut App) -> () {
    let admin: MiddlewareClosure = Arc::new(|req: Arc<Mutex<Request>>| {
        Box::pin(async move {
            req.lock()
                .await
                .variables
                .insert("is_admin".to_string(), "yes".to_string());
            Middleware::Next
        })
    });

    let is_admin: MiddlewareClosure = Arc::new(|req: Arc<Mutex<Request>>| {
        Box::pin(async move {
            let req_lock = req.lock().await;

            println!("Request cleaned: {}", req_lock.route.cleaned_route);
            println!("Request dirty: {}", req_lock.route.init_route);
            println!("Name: {:#?}", req_lock.route.get_param("name"));

            if req_lock.variables.get("is_admin").is_none() {
                return Middleware::InvalidEmpty(403);
            }
            Middleware::Next
        })
    });

    app.add_or_panic(
        "/tasks",
        Method::GET,
        Some(vec![admin, is_admin]),
        Arc::new(|_| Box::pin(async move { FileResolution::new(Some("tasks.html")) })),
    )
    .await;

    app.add_or_panic(
        "/json/{name}",
        Method::POST,
        None,
        Arc::new(|req| {
            Box::pin(async move {
                let mut people = vec![];

                let req_lock = req.lock().await;

                let name = req_lock.variables.get("name").unwrap();

                for age in 0..100 {
                    people.push(Person {
                        age,
                        name: name.to_string(),
                    });
                }

                let serialize = JsonResolution::new(people);

                if let Err(e) = serialize {
                    panic!("Could not serialize: {e}");
                }

                let mut resolution = serialize.unwrap();
                resolution.set_status(200);
                resolution.into_resolution()
            })
        }),
    )
    .await;

    let _ = app
        .add_or_change_route(
            "/",
            Method::GET,
            None,
            Arc::new(|_| Box::pin(async move { FileResolution::new(Some("home.html")) })),
        )
        .await;

    app.get_router().await.add_missing_route(EndPoint::new(
        Arc::new(|_| Box::pin(async move { FileResolution::new(Some("404.html")) })),
        None,
    ));
}

async fn create_local_app() -> App {
    //local app settings.
    let addr = Ipv4Addr::new(127, 0, 0, 1);
    let port = 8080;
    let workers = 100;

    //try bind socket.
    let app_bind = App::bind(workers, SocketAddrV4::new(addr, port)).await;

    if let Err(e) = app_bind {
        panic!("Could not bind app! {e}");
    }

    app_bind.unwrap()
}

#[tokio::main]
async fn main() {
    let mut app = create_local_app().await;

    add_routes(&mut app).await;

    //start the app, get the join handle, then await to keep in a loop.
    let _ = app.start().await.await;
}
