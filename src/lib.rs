pub mod web;
pub mod factory;

#[cfg(test)]
mod tests {

    use std::{
        io::Error,
        net::{Ipv4Addr, SocketAddrV4},
        sync::Arc,
    };


    use tokio::sync::Mutex;

    use crate::{
        middleware, resolve, web::{
            App, EndPoint, Method, Middleware, resolution::empty_resolution::EmptyResolution, routing::router::route_tree::RouteTree
        }
    };

    /// Can be used for other test to create a bind on the local machine.
    async fn create_local_app() -> Result<App, Error> {
        //local app settings.
        let addr = Ipv4Addr::new(127, 0, 0, 1);
        let port = 8080;
        let workers = 5;

        //try bind socket.
        App::bind(workers, SocketAddrV4::new(addr, port)).await
    }

    #[tokio::test]
    async fn test_route_macro() {
        let app = create_local_app().await;

        assert!(
            app.is_ok(),
            "App was not created successfully {:?}",
            app.err()
        );

        let app = Arc::new(app.unwrap());

        app.add_or_panic(
            "/test/this",
            Method::GET,
            None,
            resolve!(req, {
                let req = req.lock().await;

                println!("Request for: {}", req.method);

                EmptyResolution::new(200)
            }),
        )
        .await;

        let counter = 10;

        let counter_ref = Arc::new(Mutex::new(counter));

        app.add_or_panic(
            "/test/this",
            Method::PATCH,
            None,
            resolve!(req, moves[counter_ref], {
                let req = req.lock().await;

                println!("Request for: {}", req.method);

                {
                    let mut times_called = counter_ref.lock().await;
                    *times_called += 1;
                    println!("Request called {} times", *times_called);
                }

                EmptyResolution::new(200)
            }),
        )
        .await;

        let m_ware = middleware!(_req, {
            println!("Middleware 1");
            Middleware::Next
        });

        let m_ware_other = middleware!(_req, {
            println!("Middleware 2");
            Middleware::Next
        });

        let m_collect = middleware!(m_ware, m_ware_other);

        app.add_or_panic(
            "/test/this/middleware",
            Method::GET,
            m_collect,
            resolve!(_req, { EmptyResolution::new(200) }),
        )
        .await;

        let r = app.get_router().await.get_route("/test/this").await;

        assert!(r.is_some());

        let r = r.unwrap();
        let route_lock = r.lock().await;

        assert!(
            route_lock.get_resolution(&Method::GET).is_some(),
            "Missing GET method"
        );
        assert!(
            route_lock.get_resolution(&Method::PATCH).is_some(),
            "Missing PATCH method"
        );

    }

    #[tokio::test]
    async fn create_router() {
        let mut router = RouteTree::new(None);

        let bad_route = router
            .add_route(
                "",
                Some((
                    Method::GET,
                    EndPoint::new(resolve!(_req, { EmptyResolution::new(200) }), None),
                )),
            )
            .await;

        let good_route = router
            .add_route(
                "/test",
                Some((
                    Method::GET,
                    EndPoint::new(resolve!(_req, { EmptyResolution::new(200) }), None),
                )),
            )
            .await;

        let good_var_route: Result<(), crate::web::errors::RoutingError> = router
            .add_route(
                "/test/{user_id}/{name}",
                Some((
                    Method::GET,
                    EndPoint::new(resolve!(_req, { EmptyResolution::new(200) }), None),
                )),
            )
            .await;

        assert!(bad_route.is_err(), "(Invalid) Empty route was added.");
        assert!(good_route.is_ok(), "(Valid) Valid route was not added.");
        assert!(
            good_var_route.is_ok(),
            "(Valid) Valid var route was not added."
        );

        let found_route = router.get_route("/test").await;

        let left_id = 1;
        let right_id = 2;
        let found_var_route = router
            .get_route(&format!("/test/{left_id}/{right_id}"))
            .await;

        assert!(
            found_route.is_some(),
            "/test (a valid route) was missing from the router."
        );
        assert!(
            found_var_route.is_some(),
            "/test/{left_id}/{right_id} (a valid route) was missing from the router."
        );
    }
}
