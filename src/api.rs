mod routes;

use std::sync::Arc;

use ::time::OffsetDateTime;
use axum::{
    Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::routes::{fraud_score, health},
    index::VectorsData,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct LastTransactionData {
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub km_from_current: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Terminal {
    pub is_online: bool,
    pub card_present: bool,
    pub km_from_home: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Merchant {
    pub id: String,
    pub mcc: String,
    pub avg_amount: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Customer {
    pub avg_amount: f32,
    pub tx_count_24h: i32,
    pub known_merchants: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    pub amount: f32,
    pub installments: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub requested_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload {
    pub id: String,
    pub transaction: Transaction,
    pub customer: Customer,
    pub merchant: Merchant,
    pub terminal: Terminal,
    pub last_transaction: Option<LastTransactionData>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    approved: bool,
    fraud_score: f32,
}

#[derive(Clone)]
pub struct AppState {
    pub index: Arc<VectorsData>,
}

pub fn get_app(index: AppState) -> Router {
    Router::new()
        .route("/ready", get(health))
        .route("/fraud_score", post(fraud_score))
        .with_state(index)
}
