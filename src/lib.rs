pub mod web;

#[cfg(test)]
mod tests {

    use std::{
        io::Error,
        net::{Ipv4Addr, SocketAddrV4},
        sync::Arc,
    };

    use crate::{route, web::{
        App, EndPoint, Method, resolution::empty_resolution::EmptyResolution, router::RouteTree,
    }};

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

        assert!(app.is_ok(), "App was not created successfully {:?}", app.err());

        let app = app.unwrap();

        app.add_or_panic("/test/this", Method::GET, None, route!(req, {
            
            let req = req.lock().await;

            println!("Request for: {}", req.method);

            EmptyResolution::new(200)
        })).await;

        let r = app.get_router().await.get_route("/test/this").await;

        assert!(r.is_some());
    }

    #[tokio::test]
    async fn create_router() {
        let mut router = RouteTree::new(None);

        let bad_route = router
            .add_route(
                "",
                Some((
                    Method::GET,
                    EndPoint::new(
                        Arc::new(|_| Box::pin(async move { EmptyResolution::new(200) })),
                        None,
                    ),
                )),
            )
            .await;

        let good_route = router
            .add_route(
                "/test",
                Some((
                    Method::GET,
                    EndPoint::new(
                        Arc::new(|_| Box::pin(async move { EmptyResolution::new(200) })),
                        None,
                    ),
                )),
            )
            .await;

        let good_var_route: Result<(), crate::web::errors::RoutingError> = router
            .add_route(
                "/test/{user_id}/{name}",
                Some((
                    Method::GET,
                    EndPoint::new(
                        Arc::new(|_| Box::pin(async move { EmptyResolution::new(200) })),
                        None,
                    ),
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
    async fn test_bind() {
        let local_app_bind = create_local_app().await;

        assert!(
            local_app_bind.is_ok(),
            "failed to bind application {:?}",
            local_app_bind.err()
        );

        let app = Arc::new(local_app_bind.unwrap());

        drop(app);
    }
}
