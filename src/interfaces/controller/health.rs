use axum::{Json, Router, routing::get};
use chrono::Utc;
use serde::Serialize;

use super::super::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", get(health))
}

async fn health() -> Json<HealthResponse> {
    let timestamp = Utc::now().timestamp();
    Json(HealthResponse { timestamp })
}

#[derive(Serialize)]
struct HealthResponse {
    timestamp: i64,
}
