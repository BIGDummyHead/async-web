pub mod factory;
pub mod web;

#[cfg(test)]
mod tests {

    use std::{
        io::Error,
        net::{Ipv4Addr, SocketAddrV4},
        sync::Arc,
    };

    use futures::future::join_all;
    use tokio::sync::Mutex;

    use crate::{
        middleware, resolve,
        web::{
            App, EndPoint, Method, Middleware,
            errors::{WorkerError, worker_error::WorkerErrorType},
            resolution::{empty_resolution::EmptyResolution, error_resolution::{Configured, ErrorResolution}},
            routing::router::route_tree::RouteTree,
        },
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
            resolve!(_eq, { EmptyResolution::new(200) }),
        )
        .await;

        app.add_or_panic(
            "/public/{*}",
            Method::GET,
            None,
            resolve!(_req, { EmptyResolution::new(200) }),
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

        app.add_or_panic(
            "/error_test",
            Method::GET,
            None,
            resolve!(_req, {
                let test = {
                    let a = 10;
                    let b = 20;

                    if a + b == 30 {
                        Err(WorkerError::new(WorkerErrorType::AlreadyRunning))
                    } else {
                        Ok(200)
                    }
                }
                .map_err(|e| ErrorResolution::from_error_with_config(e, Configured::Json));

                if let Err(e) = test {
                    e
                } else {
                    EmptyResolution::new(200)
                }
            }),
        )
        .await;

        app.add_or_panic(
            "/test/wild/{*}",
            Method::GET,
            None,
            resolve!(req, {
                let req = req.lock().await;

                println!("Request for: {}", req.variables.get("*").unwrap());

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
            route_lock.brw_resolution(&Method::GET).is_some(),
            "Missing GET method"
        );
        assert!(
            route_lock.brw_resolution(&Method::PATCH).is_some(),
            "Missing PATCH method"
        );

        drop(route_lock);
    }

    #[tokio::test]
    async fn basic_routing() {
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

    #[tokio::test]
    async fn wild_card_routing() {
        let mut router = RouteTree::new(None);

        let wild_card = router
            .add_route(
                "/wild/{*}",
                Some((
                    Method::GET,
                    EndPoint::new(resolve!(_req, { EmptyResolution::new(200) }), None),
                )),
            )
            .await;

        let _ = router
            .add_route(
                "/wild/asd",
                Some((
                    Method::POST,
                    EndPoint::new(resolve!(_req, { EmptyResolution::new(200) }), None),
                )),
            )
            .await;

        assert!(wild_card.is_ok(), "Wild card was invalid route.");

        let test_routes = vec![
            "/wild/test/test/test/test",
            "/wild/test/test/test/test/wild/test/test/test/test/wild/test/test/test/test/wild/test/test/test/test",
            "/wild/test",
        ];

        let mut futs = vec![];
        for test in test_routes {
            futs.push(router.get_route(test));
        }
        let routes = join_all(futs).await;

        for route in routes {
            assert!(route.is_some());

            let route = route.unwrap();

            let route_lock = route.lock().await;

            let resolve = route_lock.brw_resolution(&Method::GET);

            assert!(
                resolve.is_some(),
                "Resolve was missing for: {}",
                route_lock.id
            );
        }

        let route = router.get_route("/wild/asd").await;

        assert!(route.is_some());

        let route = route.unwrap();

        let route = route.lock().await;

        let route_post = route.brw_resolution(&Method::POST);

        assert!(route_post.is_some());
    }
}
