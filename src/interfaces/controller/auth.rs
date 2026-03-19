use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    response::Html,
    routing::{get, post},
};

use crate::{
    infrastructure::auth::{
        ACCESS_TOKEN_EXPIRES_IN_SECS, AuthError, GoogleUserProfile, REFRESH_TOKEN_EXPIRES_IN_SECS,
    },
    interfaces::{
        SharedState,
        dto::auth::{GoogleLoginRequest, GoogleLoginResponse},
        error::AppError,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/google/login", post(google_login))
        .route("/google/test", get(google_test_page))
        .route("/google/callback", get(google_callback_page))
}

async fn google_login(
    State(state): State<SharedState>,
    request: Result<Json<GoogleLoginRequest>, JsonRejection>,
) -> Result<Json<GoogleLoginResponse>, AppError> {
    let Json(request) = request
        .map_err(|rejection| AppError::bad_request("invalid_request", rejection.body_text()))?;

    if request.code.trim().is_empty() {
        return Err(AppError::bad_request(
            "invalid_request",
            "code cannot be empty",
        ));
    }

    let user = state
        .google_auth_service()
        .login_with_code(&request.code)
        .await
        .map_err(GoogleAuthAppError::from)
        .map_err(AppError::from)?;

    let subject = format!("google:{}", user.sub);
    let access_token = state
        .auth_service()
        .generate_access_token(&subject)
        .map_err(InternalAuthAppError::from)
        .map_err(AppError::from)?;
    let refresh_token = state
        .auth_service()
        .generate_refresh_token(&subject)
        .map_err(InternalAuthAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(build_google_login_response(
        user,
        access_token,
        refresh_token,
    )))
}

fn build_google_login_response(
    user: GoogleUserProfile,
    access_token: String,
    refresh_token: String,
) -> GoogleLoginResponse {
    GoogleLoginResponse {
        user,
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
        refresh_expires_in: REFRESH_TOKEN_EXPIRES_IN_SECS,
    }
}

struct GoogleAuthAppError(AuthError);

struct InternalAuthAppError(AuthError);

impl From<AuthError> for GoogleAuthAppError {
    fn from(value: AuthError) -> Self {
        Self(value)
    }
}

impl From<AuthError> for InternalAuthAppError {
    fn from(value: AuthError) -> Self {
        Self(value)
    }
}

impl From<GoogleAuthAppError> for AppError {
    fn from(value: GoogleAuthAppError) -> Self {
        match value.0 {
            AuthError::InvalidGoogleAuthorizationCode => AppError::unauthorized(
                "invalid_google_authorization_code",
                "invalid google authorization code",
            ),
            AuthError::InvalidGoogleAccessToken => {
                AppError::unauthorized("invalid_google_access_token", "invalid google access token")
            }
            AuthError::InvalidGoogleUserInfo { reason } => {
                AppError::unauthorized("invalid_google_user_info", reason)
            }
            AuthError::GoogleUpstream(error) => {
                AppError::bad_gateway("google_upstream_error", error.to_string())
            }
            AuthError::GoogleUpstreamStatus { status } => AppError::bad_gateway(
                "google_upstream_error",
                format!("google oauth endpoint returned unexpected status {status}"),
            ),
            other => AppError::from(InternalAuthAppError::from(other)),
        }
    }
}

impl From<InternalAuthAppError> for AppError {
    fn from(value: InternalAuthAppError) -> Self {
        AppError::bad_gateway("auth_service_error", value.0.to_string())
    }
}

async fn google_test_page(State(state): State<SharedState>) -> Html<String> {
    let authorize_url = state
        .google_auth_service()
        .build_authorization_url("kiro-google-login-test");
    let redirect_uri = state.google_auth_service().redirect_uri();

    let template = include_str!("../pages/google_test.html");
    let html = template
        .replace("__GOOGLE_AUTHORIZE_URL__", &authorize_url)
        .replace("__GOOGLE_REDIRECT_URI__", redirect_uri);

    Html(html)
}

async fn google_callback_page() -> Html<&'static str> {
    Html(include_str!("../pages/google_callback.html"))
}
