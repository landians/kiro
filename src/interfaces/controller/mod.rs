mod auth;
mod health;
mod product;
mod user;

use axum::{Json, Router, extract::State, middleware::from_fn_with_state, routing::get};
use serde::Serialize;

use super::{SharedState, middleware};

pub fn build_routes(shared_state: SharedState) -> Router {
    Router::new()
        .route("/", get(index))
        .nest("/auth", auth::routes())
        .nest("/health", health::routes())
        .nest("/products", product::routes())
        .nest("/users", user::routes())
        .layer(from_fn_with_state(
            shared_state.clone(),
            middleware::log_request_response,
        ))
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
