use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{get, post},
};

use crate::{
    application::user::UserLogicError,
    infrastructure::auth::AuthError,
    interfaces::{SharedState, dto::user::UserDto, error::AppError, middleware::AuthenticatedUser},
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/logout", post(logout))
        .route("/{user_id}", get(get_user))
}

async fn get_user(
    State(state): State<SharedState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserDto>, AppError> {
    let user_id = parse_user_id(&user_id)?;
    let user = state
        .user_logic()
        .get(user_id)
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

fn parse_user_id(user_id: &str) -> Result<i64, AppError> {
    user_id
        .parse::<i64>()
        .map_err(|_| AppError::bad_request("invalid_user_id", "user_id must be a valid integer"))
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
