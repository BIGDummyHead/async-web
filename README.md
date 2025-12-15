# async-web

Minimal asynchronous web server framework in Rust built on Tokio. Designed as a learning project to demonstrate a simple router, worker pool, and a Resolution trait for generating HTTP responses.

## Key features
- Async TCP listener using Tokio
- RouteTree with per-method handlers and a missing-route (404) handler
- Worker pool (WorkManager + Worker) with a shared async Queue
- Resolution trait for producing response headers and content (e.g. `FileResolution`)
- Simple request parsing (method, path, headers)

## Quick start

1. Build and run:
```bash
cargo run
```

2. Example server in `src/main.rs` binds to `127.0.0.1:8080` by default.

## Example: register routes (from src/main.rs)
```rust
use std::sync::Arc;
use tokio::time::sleep;
use crate::web::{App, Method, Resolution, resolution::FileResolution};

async fn add_routes(app: &mut App) {
    app.add_or_panic(
        "/tasks/users",
        Method::GET,
        Arc::new(|req| {
            Box::pin(async move {
                println!("Request: {}", req.route);
                Box::new(FileResolution { file: "tasks.html".to_string() }) as Box<dyn Resolution + Send>
            })
        }),
    ).await;

    // root route with an async delay:
    app.add_or_change_route(
        "/",
        Method::GET,
        Arc::new(|req| {
            Box::pin(async move {
                println!("Request: {}", req.route);
                sleep(std::time::Duration::from_secs(2)).await;
                Box::new(FileResolution { file: "home.html".to_string() }) as Box<dyn Resolution + Send>
            })
        }),
    ).await.unwrap();
}
```

## RouteTree and handler shape

Handlers are stored as `ResolutionFunc`:
```rust
// simplified type alias from route_tree.rs
type ResolutionFunc = Arc<
    dyn Fn(crate::web::Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Box<dyn crate::web::Resolution + Send>> + Send>>
        + Send
        + Sync
        + 'static,
>;
```
A handler takes a `Request` and returns a boxed `Resolution` in an async task.

## Work manager / workers

Create App with a worker pool:
```rust
let app = App::bind(worker_count, addr).await?;
```
The `WorkManager` internally spawns `worker_count` workers. You can also get the shared queue:
```rust
let queue = app.work_manager.get_queue();
queue.queue(Box::pin(async { /* produce result */ })).await;
```

## Resolution trait and FileResolution

Resolution provides headers and content:
```rust
pub trait Resolution {
    fn get_headers(&self) -> Vec<String>;
    fn get_content(&self) -> String;
}
```
`FileResolution` reads a file and returns `HTTP/1.1 200 OK` plus file contents (sync read).


## Project layout
- src/main.rs — example app and route registration
- src/web/app.rs — App + listener + router glue
- src/web/route_tree.rs — routing tree and handler storage
- src/web/request.rs — request parsing
- src/web/worker.rs, work_manager.rs, queue.rs — worker pool & async queue
- src/web/resolution.rs — Resolution trait + FileResolution
- src/web/errors — routing & worker error types

## Contributing
Pull requests and issues welcome. If you plan production use, consider improving HTTP parsing, non-blocking file IO, and comprehensive error handling.

