mod admin_user;
mod auth;
mod health;

use axum::{Json, Router, extract::State, middleware::from_fn_with_state, routing::get};
use serde::Serialize;

use super::{SharedState, middleware};

pub fn build_routes(shared_state: SharedState) -> Router {
    let protected_routes = Router::new()
        .nest("/admin-users", admin_user::routes())
        .layer(from_fn_with_state(
            shared_state.clone(),
            middleware::validate_admin_token,
        ));

    Router::new()
        .route("/", get(index))
        .nest("/auth", auth::routes())
        .nest("/health", health::routes())
        .merge(protected_routes)
        .layer(from_fn_with_state(
            shared_state.clone(),
            middleware::log_request_response,
        ))
        .with_state(shared_state)
}

async fn index(State(_state): State<SharedState>) -> Json<IndexResponse> {
    Json(IndexResponse {
        service: "kiro-admin",
        routes: ["/auth", "/health", "/admin-users"],
    })
}

#[derive(Serialize)]
struct IndexResponse {
    service: &'static str,
    routes: [&'static str; 3],
}
