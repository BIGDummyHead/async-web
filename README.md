# async-web

Minimal asynchronous web server framework in Rust built on Tokio. It exposes a small router, middleware pipeline, and a worker-driven executor that schedules per-connection tasks.

## Overview

- **App & listener:** [`web::App`](src/web/app.rs) binds a `TcpListener`, dispatches accepted streams into the worker pool, and drives routing + middleware + resolution.
- **Routing:** Trie-based [`web::router::RouteTree`](src/web/router/route_tree.rs) and [`web::router::RouteNode`](src/web/router/route_node.rs) store per-method endpoints with optional middleware.
- **Middleware:** [`web::middleware::Middleware`](src/web/middleware.rs) closures can short-circuit with a custom resolution or continue to the endpoint.
- **Resolutions:** Implement [`web::Resolution`](src/web/resolution.rs) to produce headers and body. Built-ins: [`FileResolution`](src/web/resolution/file_resolution.rs), [`JsonResolution`](src/web/resolution/json_resolution.rs), [`EmptyResolution`](src/web/resolution/empty_resolution.rs).
- **Workers & queue:** [`web::WorkManager`](src/web/work_manager.rs) spawns [`web::Worker`](src/web/worker.rs) instances that consume a shared [`web::Queue`](src/web/queue.rs) of async tasks.
- **Requests:** [`web::Request`](src/web/request.rs) parses method, path, headers, body; routes are normalized with [`web::Route`](src/web/route.rs).
- **Errors:** Routing/worker/resolution errors live under [src/web/errors](src/web/errors.rs).

## Quick start

```rust
use std::sync::Arc;
use multi_web::web::{
    App, Method,
    resolution::{file_resolution::FileResolution, json_resolution::JsonResolution, empty_resolution::EmptyResolution},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = App::bind(4, "127.0.0.1:8080").await?;

    // Simple GET serving a file (200 or 404 decided by existence)
    app.add_or_change_route(
        "/",
        Method::GET,
        None,
        Arc::new(|_req| Box::pin(async move { FileResolution::new(Some("home.html")) })),
    ).await.unwrap();

    // JSON endpoint with custom status
    app.add_or_change_route(
        "/api/ping",
        Method::GET,
        None,
        Arc::new(|_req| {
            Box::pin(async move {
                let mut res = JsonResolution::new(serde_json::json!({"ok": true})).unwrap();
                res.set_status(200);
                res.into_resolution()
            })
        }),
    ).await.unwrap();

    // Example returning empty body + status
    app.add_or_change_route(
        "/healthz",
        Method::GET,
        None,
        Arc::new(|_req| Box::pin(async move { EmptyResolution::new(204) })),
    ).await.unwrap();

    app.start().await.await;
    Ok(())
}
```

## Routing & parameters

- Register routes with `add_or_change_route` / `add_route` on [`web::App`](src/web/app.rs).
- Dynamic segments use `{name}`. Resolved values populate `Request.variables`.
- 404 handling: set `RouteTree::missing_route` via [`RouteTree::add_missing_route`](src/web/router/route_tree.rs).

## Middleware

- Type: [`web::middleware::Middleware`](src/web/middleware.rs).
- Attach global middleware with `App::use_middleware`.
- Attach per-endpoint middleware by passing a `MiddlewareCollection` when adding a route.
- Middleware can return:
  - `Middleware::Next` to continue
  - `Middleware::Invalid(resolution)` to short-circuit
  - `Middleware::InvalidEmpty(status_code)` to short-circuit with an empty body

## Worker model

- [`WorkManager`](src/web/work_manager.rs) spawns `size` [`Worker`](src/web/worker.rs) threads, each `deque`ing tasks from a shared [`Queue`](src/web/queue.rs).
- The manager internally drains worker results via `consume` to avoid channel backpressure.
- Close workers with `WorkManager::close_and_finish_work`.

## Resolutions

- Implement [`web::Resolution`](src/web/resolution.rs) to define headers and body.
- Helpers:
  - [`FileResolution`](src/web/resolution/file_resolution.rs): serves a file (sync existence check, async read).
  - [`JsonResolution`](src/web/resolution/json_resolution.rs): serializes any `Serialize`.
  - [`EmptyResolution`](src/web/resolution/empty_resolution.rs): status-only response.

## Request parsing

- [`Request::parse_request`](src/web/request.rs) reads method, path, headers, and body from `TcpStream`.
- [`Route::parse_route`](src/web/route.rs) splits query params and exposes `cleaned_route` + `params`.

## Running & testing

```sh
cargo run          # start the sample server (edit main to wire routes)
cargo test         # run unit tests
```

## Project layout

- [src/lib.rs](src/lib.rs): crate entry & tests
- [src/web/app.rs](src/web/app.rs): application orchestration
- [src/web/router](src/web/router.rs): route tree and node definitions
- [src/web/resolution](src/web/resolution.rs): resolution trait and implementations
- [src/web/worker.rs](src/web/worker.rs), [src/web/work_manager.rs](src/web/work_manager.rs), [src/web/queue.rs](src/web/queue.rs): worker pool
- [src/web/request.rs](src/web/request.rs), [src/web/route.rs](src/web/route.rs): request parsing and routing helpers****
