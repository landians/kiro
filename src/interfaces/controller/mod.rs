mod auth;
mod health;
mod product;
mod user;

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use super::SharedState;

pub fn build_routes(shared_state: SharedState) -> Router {
    Router::new()
        .route("/", get(index))
        .nest("/auth", auth::routes())
        .nest("/health", health::routes())
        .nest("/products", product::routes())
        .nest("/users", user::routes())
        .with_state(shared_state)
}

async fn index(State(_state): State<SharedState>) -> Json<IndexResponse> {
    Json(IndexResponse {
        service: "kiro",
        routes: ["/auth", "/health", "/products", "/users"],
    })
}

#[derive(Serialize)]
struct IndexResponse {
    service: &'static str,
    routes: [&'static str; 4],
}
