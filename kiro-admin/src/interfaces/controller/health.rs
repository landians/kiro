use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::interfaces::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}
