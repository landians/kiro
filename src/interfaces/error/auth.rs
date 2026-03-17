use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::application::login::LoginServiceError;
use crate::infrastructure::auth::google_state::GoogleOAuthStateError;
use crate::interfaces::response::AppError;

#[derive(Debug)]
pub struct AuthHttpError(AppError);

impl AuthHttpError {
    pub fn current_user_unavailable() -> Self {
        Self(service_unavailable(
            "current_user_unavailable",
            "Current user service is not available.",
        ))
    }

    pub fn current_user_lookup_failed() -> Self {
        Self(internal_server_error(
            "current_user_lookup_failed",
            "Failed to load current user.",
        ))
    }

    pub fn authenticated_user_not_found() -> Self {
        let app_error = AppError::unauthorized(
            "authenticated_user_not_found",
            "Authenticated user does not exist.",
        );
        Self(app_error)
    }

    pub fn blacklist_unavailable() -> Self {
        Self(service_unavailable(
            "blacklist_unavailable",
            "Token blacklist backend is unavailable.",
        ))
    }

    pub fn token_refresh_failed() -> Self {
        Self(internal_server_error(
            "token_refresh_failed",
            "Failed to refresh session tokens.",
        ))
    }

    pub fn google_login_unavailable() -> Self {
        Self(service_unavailable(
            "google_login_unavailable",
            "Google login is not enabled.",
        ))
    }

    pub fn google_authorization_url_build_failed() -> Self {
        Self(internal_server_error(
            "google_authorization_url_build_failed",
            "Failed to build Google authorization url.",
        ))
    }

    pub fn google_authorization_denied(error_code: &str, error_description: Option<&str>) -> Self {
        let message = error_description
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Google authorization failed.");

        Self(bad_request(
            "google_authorization_denied",
            format!("{error_code}: {message}"),
        ))
    }

    pub fn missing_authorization_code() -> Self {
        Self(bad_request(
            "missing_authorization_code",
            "Google authorization code is required.",
        ))
    }

    pub fn missing_google_state() -> Self {
        Self(bad_request(
            "missing_google_state",
            "Google oauth state is required.",
        ))
    }

    pub fn missing_user_agent() -> Self {
        Self(bad_request(
            "missing_user_agent",
            "User-Agent header is required.",
        ))
    }

    pub fn token_subject_mismatch() -> Self {
        let app_error = AppError::unauthorized(
            "token_subject_mismatch",
            "Access token and refresh token must belong to the same subject.",
        );
        Self(app_error)
    }

    pub fn from_google_authorization_request_error(error: LoginServiceError) -> Self {
        match error {
            LoginServiceError::GoogleLoginDisabled
            | LoginServiceError::GoogleStateServiceUnavailable => Self::google_login_unavailable(),
            _ => Self::google_authorization_url_build_failed(),
        }
    }

    pub fn from_google_callback_error(error: LoginServiceError) -> Self {
        match error {
            LoginServiceError::GoogleLoginDisabled
            | LoginServiceError::AccountServiceUnavailable
            | LoginServiceError::UserIdentityServiceUnavailable
            | LoginServiceError::GoogleStateServiceUnavailable => Self::google_login_unavailable(),
            LoginServiceError::MissingAuthorizationCode => Self::missing_authorization_code(),
            LoginServiceError::MissingOAuthState => Self::missing_google_state(),
            LoginServiceError::MissingUserAgent => Self::missing_user_agent(),
            LoginServiceError::GoogleOAuthState(
                GoogleOAuthStateError::InvalidState | GoogleOAuthStateError::StateExpired,
            ) => Self(bad_request(
                "invalid_google_state",
                "Google oauth state is invalid or expired.",
            )),
            LoginServiceError::GoogleOAuthState(GoogleOAuthStateError::MissingState) => {
                Self::missing_google_state()
            }
            LoginServiceError::IdentityBindingConflict { .. } => {
                let app_error = AppError::new(
                    StatusCode::CONFLICT,
                    "identity_binding_conflict",
                    "Google identity conflicts with an existing binding.",
                );
                Self(app_error)
            }
            LoginServiceError::GoogleOAuth(_) => {
                let app_error = AppError::new(
                    StatusCode::BAD_GATEWAY,
                    "google_oauth_exchange_failed",
                    "Failed to exchange Google authorization code or fetch profile.",
                );
                Self(app_error)
            }
            LoginServiceError::UserRepository(_)
            | LoginServiceError::UserIdentityRepository(_)
            | LoginServiceError::ProfileSerializationFailed(_)
            | LoginServiceError::TokenIssuanceFailed(_)
            | LoginServiceError::GoogleOAuthState(_)
            | LoginServiceError::MissingGoogleSubject
            | LoginServiceError::InconsistentIdentityUser { .. } => Self(internal_server_error(
                "google_login_failed",
                "Failed to complete Google login.",
            )),
        }
    }
}

impl From<AppError> for AuthHttpError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for AuthHttpError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

fn bad_request(code: &'static str, message: impl Into<String>) -> AppError {
    AppError::new(StatusCode::BAD_REQUEST, code, message)
}

fn internal_server_error(code: &'static str, message: impl Into<String>) -> AppError {
    AppError::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
}

fn service_unavailable(code: &'static str, message: impl Into<String>) -> AppError {
    AppError::new(StatusCode::SERVICE_UNAVAILABLE, code, message)
}
