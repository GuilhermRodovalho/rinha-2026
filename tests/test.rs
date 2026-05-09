// use std::net::SocketAddr;
//
// use rinha::api::get_app;
//
// async fn spawn_app() -> SocketAddr {
//     let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
//
//     let port = listener
//         .local_addr()
//         .expect("Couldn't find a port to run the application");
//
//     let app = get_app();
//
//     tokio::spawn(async move {
//         axum::serve(listener, app).await.unwrap();
//     });
//
//     port
// }
//
// #[tokio::test]
// async fn test_health_check() {
//     let port = spawn_app().await;
//
//     let res = reqwest::get(format!("http://0.0.0.0:{}/ready", port.port()))
//         .await
//         .expect("couldn't complete request");
//
//     assert_eq!(res.status(), 200)
// }
