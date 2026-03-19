use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::Serialize;

use super::super::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_users))
        .route("/:user_id", get(get_user))
}

async fn list_users(State(_state): State<SharedState>) -> Json<UserListResponse> {
    Json(UserListResponse {
        message: "users collection route skeleton",
        items: Vec::new(),
    })
}

async fn get_user(
    State(_state): State<SharedState>,
    Path(user_id): Path<String>,
) -> Json<UserDetailResponse> {
    Json(UserDetailResponse {
        message: "user resource route skeleton",
        user_id,
    })
}

#[derive(Serialize)]
struct UserListResponse {
    message: &'static str,
    items: Vec<UserSummary>,
}

#[derive(Serialize)]
struct UserSummary {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct UserDetailResponse {
    message: &'static str,
    user_id: String,
}
