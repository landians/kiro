use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    routing::post,
};
use chrono::Utc;

use crate::{
    application::auth::password_login::{PasswordLogin, PasswordLoginError},
    infrastructure::auth::ACCESS_TOKEN_EXPIRES_IN_SECS,
    interfaces::{
        SharedState,
        dto::{
            admin_user::AdminUserDto,
            auth::{PasswordLoginRequest, PasswordLoginResponse},
        },
        error::AppError,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/login", post(password_login))
}

async fn password_login(
    State(state): State<SharedState>,
    request: Result<Json<PasswordLoginRequest>, JsonRejection>,
) -> Result<Json<PasswordLoginResponse>, AppError> {
    let Json(request) = request
        .map_err(|rejection| AppError::bad_request("invalid_request", rejection.body_text()))?;

    if request.email.trim().is_empty() || request.password.trim().is_empty() {
        return Err(AppError::bad_request(
            "invalid_request",
            "email and password are required",
        ));
    }

    let admin_user = state
        .auth_logic()
        .password_login(build_password_login(request))
        .await
        .map_err(PasswordLoginAppError::from)
        .map_err(AppError::from)?;
    let access_token = state
        .auth_service()
        .generate_access_token(&admin_user.id.to_string())
        .map_err(AdminTokenAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(PasswordLoginResponse {
        admin_user: AdminUserDto::from(admin_user),
        access_token,
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
    }))
}

fn build_password_login(request: PasswordLoginRequest) -> PasswordLogin {
    PasswordLogin {
        email: request.email.trim().to_lowercase(),
        password: request.password,
        login_at: Utc::now(),
    }
}

struct PasswordLoginAppError(anyhow::Error);
struct AdminTokenAppError(crate::infrastructure::auth::AuthError);

impl From<anyhow::Error> for PasswordLoginAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<crate::infrastructure::auth::AuthError> for AdminTokenAppError {
    fn from(value: crate::infrastructure::auth::AuthError) -> Self {
        Self(value)
    }
}

impl From<PasswordLoginAppError> for AppError {
    fn from(value: PasswordLoginAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<PasswordLoginError>() {
            return match error {
                PasswordLoginError::InvalidCredentials => {
                    AppError::unauthorized("invalid_admin_credentials", "invalid admin credentials")
                }
                PasswordLoginError::AdminUserFrozen { admin_user_id } => AppError::forbidden(
                    "admin_user_frozen",
                    format!("admin user {admin_user_id} is frozen"),
                ),
            };
        }

        AppError::internal_server_error("admin_password_login_error", value.0.to_string())
    }
}

impl From<AdminTokenAppError> for AppError {
    fn from(value: AdminTokenAppError) -> Self {
        AppError::internal_server_error("auth_service_error", value.0.to_string())
    }
}
