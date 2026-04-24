use axum::{
    Json, Router,
    extract::{Extension, State},
    http::StatusCode,
    routing::{get, post},
};
use validator::Validate;

use crate::{
    application::user::{UpdateUser, UserLogicError},
    infrastructure::auth::AuthError,
    interfaces::{
        SharedState,
        dto::user::{UpdateUserRequest, UserDto},
        error::AppError,
        middleware::AuthenticatedUser,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/me", get(get_user).patch(update_user))
}

async fn get_user(
    State(state): State<SharedState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<UserDto>, AppError> {
    let user = state
        .user_logic()
        .get(authenticated_user.user_id)
        .await
        .map_err(UserAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(UserDto::from(user)))
}

async fn logout(
    State(state): State<SharedState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<StatusCode, AppError> {
    state
        .auth_service()
        .revoke_claims(&authenticated_user.token_claims)
        .await
        .map_err(LogoutAppError::from)
        .map_err(AppError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn update_user(
    State(state): State<SharedState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserDto>, AppError> {
    request.validate()?;
    
    let user = state
        .user_logic()
        .update(authenticated_user.user_id, build_update_user(request))
        .await
        .map_err(UserAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(UserDto::from(user)))
}

fn build_update_user(request: UpdateUserRequest) -> UpdateUser {
    UpdateUser {
        display_name: request.display_name,
        avatar_url: request.avatar_url,
    }
}

struct UserAppError(anyhow::Error);
struct LogoutAppError(AuthError);

impl From<anyhow::Error> for UserAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<AuthError> for LogoutAppError {
    fn from(value: AuthError) -> Self {
        Self(value)
    }
}

impl From<UserAppError> for AppError {
    fn from(value: UserAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<UserLogicError>() {
            return match error {
                UserLogicError::UserNotFound { user_id } => {
                    AppError::not_found("user_not_found", format!("user {user_id} not found"))
                }
                UserLogicError::EmptyUserUpdate => AppError::bad_request(
                    "empty_user_update",
                    "at least one updatable field is required",
                ),
            };
        }

        AppError::internal_server_error("get_user_error", value.0.to_string())
    }
}

impl From<LogoutAppError> for AppError {
    fn from(value: LogoutAppError) -> Self {
        AppError::internal_server_error("logout_error", value.0.to_string())
    }
}
