use axum::{Extension, Json, Router, extract::State, routing::get};

use crate::{
    application::user::AdminUserLogicError,
    interfaces::{
        SharedState, dto::admin_user::AdminUserDto, error::AppError,
        middleware::AuthenticatedAdminUser,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/me", get(get_admin_user))
}

async fn get_admin_user(
    State(state): State<SharedState>,
    Extension(authenticated_admin_user): Extension<AuthenticatedAdminUser>,
) -> Result<Json<AdminUserDto>, AppError> {
    let admin_user = state
        .admin_user_logic()
        .get(authenticated_admin_user.admin_user_id)
        .await
        .map_err(AdminUserAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(AdminUserDto::from(admin_user)))
}

struct AdminUserAppError(anyhow::Error);

impl From<anyhow::Error> for AdminUserAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<AdminUserAppError> for AppError {
    fn from(value: AdminUserAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<AdminUserLogicError>() {
            return match error {
                AdminUserLogicError::AdminUserNotFound { admin_user_id } => AppError::not_found(
                    "admin_user_not_found",
                    format!("admin user {admin_user_id} not found"),
                ),
            };
        }

        AppError::internal_server_error("get_admin_user_error", value.0.to_string())
    }
}
