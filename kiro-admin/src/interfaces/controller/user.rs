use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use validator::Validate;

use crate::interfaces::{
    SharedState,
    dto::user::{ListUsersRequest, UserListResponse},
    error::AppError,
};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", get(list_users))
}

async fn list_users(
    State(state): State<SharedState>,
    Query(request): Query<ListUsersRequest>,
) -> Result<Json<UserListResponse>, AppError> {
    request.validate()?;

    let users = state
        .user_logic()
        .list(request.into_query())
        .await
        .map_err(ListUsersAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(UserListResponse::from(users)))
}

struct ListUsersAppError(anyhow::Error);

impl From<anyhow::Error> for ListUsersAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<ListUsersAppError> for AppError {
    fn from(value: ListUsersAppError) -> Self {
        AppError::internal_server_error("list_users_error", value.0.to_string())
    }
}
