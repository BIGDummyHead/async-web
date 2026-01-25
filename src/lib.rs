pub mod factory;
pub mod web;

#[cfg(test)]
mod tests {

    use std::sync::{Arc, LazyLock};

    use tokio::sync::Mutex;

    use crate::{
        middleware, resolve,
        web::{
            App, EndPoint, Method, Middleware, Resolution,
            resolution::empty_resolution::EmptyResolution, routing::router::route_tree::RouteTree,
        },
    };

    //ensures that routing works.
    #[tokio::test]
    async fn test_route_tree() {
        let mut tree = RouteTree::new(None);

        let add_result = tree
            .add_route(
                "/api",
                Some((
                    Method::GET,
                    EndPoint::new(
                        resolve!(_req, moves[], {EmptyResolution::status(200).resolve()}),
                        None,
                    ),
                )),
            )
            .await;

        let get_result = tree.get_route("/api").await;

        assert!(add_result.is_ok(), "did not add valid route");
        assert!(get_result.is_some(), "could not get added route");

        let route_node = get_result.unwrap();

        {
            let route_guard = route_node.lock().await;

            let res_ref = route_guard.brw_resolution(&Method::GET);

            assert!(
                res_ref.is_some(),
                "no resolution for GET when resolution was needed."
            );
        } //drop here just incase of further test.
    }

    static APP_CLOSURE_SAFETY: LazyLock<Arc<Mutex<()>>> =
        LazyLock::new(|| Arc::new(Mutex::new(())));

    #[tokio::test]
    async fn test_multi_app_bind() {
        let closure_guard = APP_CLOSURE_SAFETY.lock().await;

        //bind to local machine, then close, then try again to ensure binds work
        for _ in 0..2 {
            let app = App::bind(1000, "127.0.0.1:80").await;

            assert!(app.is_ok(), "app could not bind!");

            let mut app = app.unwrap();

            let start_result = app.start();

            assert!(
                start_result.is_ok(),
                "application could not be started because {}",
                start_result.unwrap_err()
            );

            let closure_result = app.close().await;
            assert!(
                closure_result.is_ok(),
                "app failed to closure because {}",
                closure_result.unwrap_err()
            );
        }

        drop(closure_guard);
    }

    #[tokio::test]
    async fn test_routing_app() {
        let closure_guard = APP_CLOSURE_SAFETY.lock().await;

        let mut app = App::bind(1000, "127.0.0.1:80")
            .await
            .expect("app did not bind");

        let m_ware = middleware!(_req, moves[], {

            Middleware::Next
        });

        app.add_or_panic(
            "/app",
            Method::GET,
            middleware!(m_ware),
            |_req| async move { EmptyResolution::status(200).resolve() },
        )
        .await;

        let app = app.close().await;

        //app never started... so yes.
        assert!(app.is_err());

        drop(closure_guard);
    }
}
