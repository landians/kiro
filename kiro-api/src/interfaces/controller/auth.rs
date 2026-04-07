use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header},
    response::Html,
    routing::{get, post},
};
use chrono::Utc;
use validator::Validate;

use crate::{
    application::auth::google_login::{GoogleLogin, GoogleLoginError},
    infrastructure::auth::{
        ACCESS_TOKEN_EXPIRES_IN_SECS, AuthError, GoogleUserProfile, REFRESH_TOKEN_EXPIRES_IN_SECS,
        TokenPair,
    },
    interfaces::{
        SharedState,
        dto::{
            auth::{GoogleLoginRequest, GoogleLoginResponse, RefreshAccessTokenResponse},
            user::UserDto,
        },
        error::AppError,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/google/login", post(google_login))
        .route("/refresh-token", post(refresh_access_token))
        .route("/google/test", get(google_test_page))
        .route("/google/callback", get(google_callback_page))
}

async fn google_login(
    State(state): State<SharedState>,
    Json(request): Json<GoogleLoginRequest>,
) -> Result<Json<GoogleLoginResponse>, AppError> {
    request.validate()?;

    let google_user = state
        .google_auth_service()
        .login_with_code(&request.code)
        .await
        .map_err(GoogleAuthAppError::from)
        .map_err(AppError::from)?;

    let user = state
        .auth_logic()
        .google_login(build_google_login(google_user))
        .await
        .map_err(GoogleLoginAppError::from)
        .map_err(AppError::from)?;

    let subject = user.id.to_string();
    let token_pair = state
        .auth_service()
        .generate_token_pair(&subject)
        .map_err(InternalAuthAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(build_google_login_response(user, token_pair)))
}

async fn refresh_access_token(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<RefreshAccessTokenResponse>, AppError> {
    let refresh_token = extract_bearer_token(&headers)?;
    let access_token = state
        .auth_service()
        .refresh_access_token(refresh_token)
        .await
        .map_err(RefreshAccessTokenAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(RefreshAccessTokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
    }))
}

fn build_google_login_response(
    user: crate::domain::entity::user::User,
    token_pair: TokenPair,
) -> GoogleLoginResponse {
    GoogleLoginResponse {
        user: UserDto::from(user),
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_EXPIRES_IN_SECS,
        refresh_expires_in: REFRESH_TOKEN_EXPIRES_IN_SECS,
    }
}

fn build_google_login(user: GoogleUserProfile) -> GoogleLogin {
    GoogleLogin {
        provider_user_id: user.sub,
        email: Some(user.email),
        email_verified: user.email_verified,
        display_name: user.name,
        avatar_url: user.picture,
        login_at: Utc::now(),
    }
}

struct GoogleAuthAppError(AuthError);

struct InternalAuthAppError(AuthError);
struct GoogleLoginAppError(anyhow::Error);
struct RefreshAccessTokenAppError(AuthError);

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

impl From<anyhow::Error> for GoogleLoginAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<AuthError> for RefreshAccessTokenAppError {
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
        AppError::internal_server_error("auth_service_error", value.0.to_string())
    }
}

impl From<GoogleLoginAppError> for AppError {
    fn from(value: GoogleLoginAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<GoogleLoginError>() {
            return match error {
                GoogleLoginError::MissingLinkedUser {
                    user_id,
                    identity_id,
                } => AppError::internal_server_error(
                    "missing_linked_user",
                    format!("linked user {user_id} is missing for auth identity {identity_id}"),
                ),
                GoogleLoginError::UserFrozen { user_id } => {
                    AppError::forbidden("user_frozen", format!("user {user_id} is frozen"))
                }
                GoogleLoginError::UserBanned { user_id } => {
                    AppError::forbidden("user_banned", format!("user {user_id} is banned"))
                }
            };
        }

        AppError::internal_server_error("google_login_error", value.0.to_string())
    }
}

impl From<RefreshAccessTokenAppError> for AppError {
    fn from(value: RefreshAccessTokenAppError) -> Self {
        match value.0 {
            AuthError::Jwt(_) | AuthError::InvalidTokenType { .. } | AuthError::RevokedToken => {
                AppError::unauthorized("invalid_refresh_token", "invalid refresh token")
            }
            other => AppError::internal_server_error("auth_service_error", other.to_string()),
        }
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

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, AppError> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| {
            AppError::unauthorized(
                "missing_authorization_header",
                "authorization header is required",
            )
        })?
        .to_str()
        .map_err(|_| {
            AppError::unauthorized(
                "invalid_authorization_header",
                "authorization header must be valid ASCII",
            )
        })?;

    let token = authorization.strip_prefix("Bearer ").ok_or_else(|| {
        AppError::unauthorized(
            "invalid_authorization_scheme",
            "authorization header must use Bearer scheme",
        )
    })?;

    if token.trim().is_empty() {
        return Err(AppError::unauthorized(
            "invalid_refresh_token",
            "refresh token cannot be empty",
        ));
    }

    Ok(token)
}
