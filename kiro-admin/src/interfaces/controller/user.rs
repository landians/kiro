use axum::{
    Json, Router,
    extract::{Json as JsonBody, Path, Query, State},
    routing::{get, patch},
};
use chrono::Utc;
use validator::Validate;

use crate::{
    application::user::UserLogicError,
    interfaces::{
        SharedState,
        dto::user::{
            ListUsersRequest, ManageableUserStatus, UpdateUserStatusRequest, UserDto,
            UserListResponse,
        },
        error::AppError,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{user_id}/status", patch(update_user_status))
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

async fn update_user_status(
    State(state): State<SharedState>,
    Path(user_id): Path<i64>,
    JsonBody(request): JsonBody<UpdateUserStatusRequest>,
) -> Result<Json<UserDto>, AppError> {
    let user = match request.account_status {
        ManageableUserStatus::Active => state.user_logic().activate(user_id).await,
        ManageableUserStatus::Frozen => state.user_logic().freeze(user_id, Utc::now()).await,
        ManageableUserStatus::Banned => state.user_logic().ban(user_id, Utc::now()).await,
    }
    .map_err(UserAppError::from)
    .map_err(AppError::from)?;

    Ok(Json(UserDto::from(user)))
}

struct ListUsersAppError(anyhow::Error);
struct UserAppError(anyhow::Error);

impl From<anyhow::Error> for ListUsersAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<anyhow::Error> for UserAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<ListUsersAppError> for AppError {
    fn from(value: ListUsersAppError) -> Self {
        AppError::internal_server_error("list_users_error", value.0.to_string())
    }
}

impl From<UserAppError> for AppError {
    fn from(value: UserAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<UserLogicError>() {
            return match error {
                UserLogicError::UserNotFound { user_id } => {
                    AppError::not_found("user_not_found", format!("user {user_id} not found"))
                }
                UserLogicError::InvalidUserStatusTransition {
                    user_id,
                    current_status,
                    target_status,
                } => AppError::bad_request(
                    "invalid_user_status_transition",
                    format!(
                        "user {user_id} cannot transition from {current_status} to {target_status}"
                    ),
                ),
            };
        }

        AppError::internal_server_error("update_user_status_error", value.0.to_string())
    }
}
