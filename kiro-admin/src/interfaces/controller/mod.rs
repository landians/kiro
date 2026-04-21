mod admin_user;
mod auth;
mod health;
mod product;
mod user;

use axum::{Json, Router, extract::State, middleware::from_fn_with_state, routing::get};
use serde::Serialize;

use super::{SharedState, middleware};

pub fn build_routes(shared_state: SharedState) -> Router {
    let protected_routes = Router::new()
        .nest("/admin-users", admin_user::routes())
        .nest("/products", product::routes())
        .nest("/users", user::routes())
        .merge(product::plan_routes())
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
        routes: [
            "/auth",
            "/health",
            "/admin-users",
            "/products",
            "/product-plans",
            "/users",
        ],
    })
}

#[derive(Serialize)]
struct IndexResponse {
    service: &'static str,
    routes: [&'static str; 6],
}
