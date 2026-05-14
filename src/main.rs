use rinha::{
    api::{AppState, get_app},
    configuration::get_configuration,
    index::Index,
};

async fn run_server() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let state = AppState {
        index: Index::load(),
    };
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    let app = get_app(state);
    axum::serve(listener, app).await.unwrap();
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let _configuration = get_configuration().expect("Error getting configuration");

    println!("starting server");
    run_server().await;
}
