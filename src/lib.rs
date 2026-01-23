pub mod factory;
pub mod web;

#[cfg(test)]
mod tests {

    use std::{
        io::Error,
        net::{Ipv4Addr, SocketAddrV4},
        sync::Arc, time::Duration,
    };

    use futures::future::join_all;
    use tokio::{sync::Mutex, time::sleep};

    use crate::{
        middleware, resolve,
        web::{
            App, EndPoint, Method, Middleware, Request,
            resolution::empty_resolution::EmptyResolution, routing::router::route_tree::RouteTree,
        },
    };

    //ensures that routing works.
    #[tokio::test]
    async fn routing_ensure() {
        let mut tree = RouteTree::new(None);

        let add_result = tree
            .add_route(
                "/api",
                Some((
                    Method::GET,
                    EndPoint::new(resolve!(_req, moves[], {EmptyResolution::new(200)}), None),
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

    #[tokio::test]
    async fn create_app() {

        //bind to local machine
        let app = App::bind(1000, "127.0.0.1:80").await;

        assert!(app.is_ok(), "app could not bind!");

        let app = app.unwrap();

        let closure_func = app.start();

        closure_func.await;

    }

}
