use std::time::Instant;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::api::{AppState, Payload, Response};

pub async fn fraud_score(
    State(state): State<AppState>,
    Json(input): Json<Payload>,
) -> impl IntoResponse {
    let start = Instant::now();
    let vector = input.vectorize();
    let response = state.index.knn_fraud_ratio(&vector, 5);
    println!("[{}] {}ms", input.id, start.elapsed().as_millis());

    (
        StatusCode::OK,
        Json(Response {
            approved: response < 0.6,
            fraud_score: response,
        }),
    )
}

pub async fn health() -> impl IntoResponse {
    StatusCode::OK
}
