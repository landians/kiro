use axum::Extension;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use tracing::error;

use crate::interfaces::AppState;
use crate::interfaces::dto::auth::{
    CurrentUserResponse, GoogleAuthorizationUrlResponse, GoogleCallbackQuery,
    GoogleCallbackResponse, LogoutSessionResponse, ProtectedSessionResponse, RefreshTokenResponse,
};
use crate::interfaces::middleware::authentication::{
    AuthenticatedAccessToken, AuthenticatedRefreshToken,
};
use crate::interfaces::middleware::trace_id::RequestTrace;
use crate::interfaces::response::{ApiSuccess, AppError};

pub async fn protected_session(
    Extension(authenticated): Extension<AuthenticatedAccessToken>,
) -> ApiSuccess<ProtectedSessionResponse> {
    ApiSuccess::ok(ProtectedSessionResponse {
        subject: authenticated.subject,
        jti: authenticated.jti,
        ua_hash: authenticated.ua_hash,
        issued_at: authenticated.issued_at,
        expires_at: authenticated.expires_at,
    })
}

pub async fn current_user(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    Extension(authenticated): Extension<AuthenticatedAccessToken>,
) -> Result<ApiSuccess<CurrentUserResponse>, AppError> {
    let account_service = state
        .services
        .account_service()
        .ok_or_else(|| current_user_unavailable_error(request_trace.trace_id().to_owned()))?;

    let user = account_service
        .find_user_by_user_code(&authenticated.subject)
        .await
        .map_err(|error| {
            error!(
                trace_id = %request_trace.trace_id(),
                subject = %authenticated.subject,
                error = %error,
                "failed to load current user"
            );
            AppError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "current_user_lookup_failed",
                "Failed to load current user.",
                request_trace.trace_id().to_owned(),
            )
        })?
        .ok_or_else(|| {
            AppError::unauthorized(
                "authenticated_user_not_found",
                "Authenticated user does not exist.",
                request_trace.trace_id().to_owned(),
            )
        })?;

    Ok(ApiSuccess::ok(user.into()))
}

pub async fn refresh_session(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    Extension(authenticated): Extension<AuthenticatedRefreshToken>,
) -> Result<ApiSuccess<RefreshTokenResponse>, AppError> {
    state
        .services
        .auth
        .revoke_refresh_token(&authenticated.jti, authenticated.expires_at)
        .await
        .map_err(|error| {
            error!(
                trace_id = %request_trace.trace_id(),
                error = %error,
                "failed to revoke refresh token during token rotation"
            );
            blacklist_unavailable_error(request_trace.trace_id().to_owned())
        })?;

    let token_pair = state
        .services
        .auth
        .issue_token_pair(&authenticated.subject, &authenticated.user_agent)
        .map_err(|_| {
            AppError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "token_refresh_failed",
                "Failed to refresh session tokens.",
                request_trace.trace_id().to_owned(),
            )
        })?;

    Ok(ApiSuccess::ok(RefreshTokenResponse {
        access_token: token_pair.access_token.token,
        refresh_token: token_pair.refresh_token.token,
        access_token_expires_at: token_pair.access_token.expires_at,
        refresh_token_expires_at: token_pair.refresh_token.expires_at,
    }))
}

pub async fn google_authorization_url(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
) -> Result<ApiSuccess<GoogleAuthorizationUrlResponse>, AppError> {
    let login_service = state
        .services
        .login_service()
        .ok_or_else(|| google_login_unavailable_error(request_trace.trace_id().to_owned()))?;

    let authorization_request = login_service.build_google_authorization_request().map_err(
        |error| {
            error!(
                trace_id = %request_trace.trace_id(),
                error = %error,
                "failed to build google authorization url"
            );

            match error {
                crate::application::login::LoginServiceError::GoogleLoginDisabled
                | crate::application::login::LoginServiceError::GoogleStateServiceUnavailable => {
                    google_login_unavailable_error(request_trace.trace_id().to_owned())
                }
                _ => AppError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "google_authorization_url_build_failed",
                    "Failed to build Google authorization url.",
                    request_trace.trace_id().to_owned(),
                ),
            }
        },
    )?;

    Ok(ApiSuccess::ok(GoogleAuthorizationUrlResponse {
        authorization_url: authorization_request.authorization_url,
        state: authorization_request.state,
        nonce: authorization_request.nonce,
    }))
}

pub async fn google_callback(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    headers: HeaderMap,
    Query(query): Query<GoogleCallbackQuery>,
) -> Result<ApiSuccess<GoogleCallbackResponse>, AppError> {
    if let Some(error_code) = query.error.as_deref() {
        let message = query
            .error_description
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Google authorization failed.");
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "google_authorization_denied",
            format!("{error_code}: {message}"),
            request_trace.trace_id().to_owned(),
        ));
    }

    let login_service = state
        .services
        .login_service()
        .ok_or_else(|| google_login_unavailable_error(request_trace.trace_id().to_owned()))?;

    let authorization_code = query
        .code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                StatusCode::BAD_REQUEST,
                "missing_authorization_code",
                "Google authorization code is required.",
                request_trace.trace_id().to_owned(),
            )
        })?;
    let oauth_state = query
        .state
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                StatusCode::BAD_REQUEST,
                "missing_google_state",
                "Google oauth state is required.",
                request_trace.trace_id().to_owned(),
            )
        })?;

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                StatusCode::BAD_REQUEST,
                "missing_user_agent",
                "User-Agent header is required.",
                request_trace.trace_id().to_owned(),
            )
        })?;

    let login_result = login_service
        .complete_google_login(crate::application::login::GoogleLoginCallbackCommand {
            authorization_code: authorization_code.to_owned(),
            oauth_state: oauth_state.to_owned(),
            user_agent: user_agent.to_owned(),
        })
        .await
        .map_err(|error| {
            error!(
                trace_id = %request_trace.trace_id(),
                google_state = ?query.state,
                error = %error,
                "failed to complete google login callback"
            );
            map_google_callback_error(error, request_trace.trace_id().to_owned())
        })?;

    Ok(ApiSuccess::ok(GoogleCallbackResponse {
        user_code: login_result.user_code,
        identity_code: login_result.identity_code,
        provider: login_result.provider.as_str().to_owned(),
        is_new_user: login_result.is_new_user,
        access_token: login_result.access_token,
        refresh_token: login_result.refresh_token,
        access_token_expires_at: login_result.access_token_expires_at,
        refresh_token_expires_at: login_result.refresh_token_expires_at,
    }))
}

pub async fn logout_session(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    Extension(access_token): Extension<AuthenticatedAccessToken>,
    Extension(refresh_token): Extension<AuthenticatedRefreshToken>,
) -> Result<ApiSuccess<LogoutSessionResponse>, AppError> {
    if access_token.subject != refresh_token.subject {
        return Err(AppError::unauthorized(
            "token_subject_mismatch",
            "Access token and refresh token must belong to the same subject.",
            request_trace.trace_id().to_owned(),
        ));
    }

    state
        .services
        .auth
        .revoke_session_tokens(
            &access_token.jti,
            access_token.expires_at,
            &refresh_token.jti,
            refresh_token.expires_at,
        )
        .await
        .map_err(|error| {
            error!(
                trace_id = %request_trace.trace_id(),
                error = %error,
                "failed to revoke session tokens during logout"
            );
            blacklist_unavailable_error(request_trace.trace_id().to_owned())
        })?;

    Ok(ApiSuccess::ok(LogoutSessionResponse {
        subject: access_token.subject,
        access_token_revoked: true,
        refresh_token_revoked: true,
    }))
}

fn blacklist_unavailable_error(trace_id: String) -> AppError {
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "blacklist_unavailable",
        "Token blacklist backend is unavailable.",
        trace_id,
    )
}

fn google_login_unavailable_error(trace_id: String) -> AppError {
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "google_login_unavailable",
        "Google login is not enabled.",
        trace_id,
    )
}

fn current_user_unavailable_error(trace_id: String) -> AppError {
    AppError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "current_user_unavailable",
        "Current user service is not available.",
        trace_id,
    )
}

fn map_google_callback_error(
    error: crate::application::login::LoginServiceError,
    trace_id: String,
) -> AppError {
    match error {
        crate::application::login::LoginServiceError::GoogleLoginDisabled
        | crate::application::login::LoginServiceError::AccountServiceUnavailable
        | crate::application::login::LoginServiceError::UserIdentityServiceUnavailable => {
            google_login_unavailable_error(trace_id)
        }
        crate::application::login::LoginServiceError::MissingAuthorizationCode => AppError::new(
            StatusCode::BAD_REQUEST,
            "missing_authorization_code",
            "Google authorization code is required.",
            trace_id,
        ),
        crate::application::login::LoginServiceError::MissingOAuthState => AppError::new(
            StatusCode::BAD_REQUEST,
            "missing_google_state",
            "Google oauth state is required.",
            trace_id,
        ),
        crate::application::login::LoginServiceError::MissingUserAgent => AppError::new(
            StatusCode::BAD_REQUEST,
            "missing_user_agent",
            "User-Agent header is required.",
            trace_id,
        ),
        crate::application::login::LoginServiceError::GoogleOAuthState(
            crate::infrastructure::auth::google_state::GoogleOAuthStateError::InvalidState
            | crate::infrastructure::auth::google_state::GoogleOAuthStateError::StateExpired,
        ) => AppError::new(
            StatusCode::BAD_REQUEST,
            "invalid_google_state",
            "Google oauth state is invalid or expired.",
            trace_id,
        ),
        crate::application::login::LoginServiceError::GoogleOAuthState(
            crate::infrastructure::auth::google_state::GoogleOAuthStateError::MissingState,
        ) => AppError::new(
            StatusCode::BAD_REQUEST,
            "missing_google_state",
            "Google oauth state is required.",
            trace_id,
        ),
        crate::application::login::LoginServiceError::IdentityBindingConflict { .. } => {
            AppError::new(
                StatusCode::CONFLICT,
                "identity_binding_conflict",
                "Google identity conflicts with an existing binding.",
                trace_id,
            )
        }
        crate::application::login::LoginServiceError::GoogleOAuth(_) => AppError::new(
            StatusCode::BAD_GATEWAY,
            "google_oauth_exchange_failed",
            "Failed to exchange Google authorization code or fetch profile.",
            trace_id,
        ),
        crate::application::login::LoginServiceError::GoogleStateServiceUnavailable => {
            google_login_unavailable_error(trace_id)
        }
        crate::application::login::LoginServiceError::UserRepository(_)
        | crate::application::login::LoginServiceError::UserIdentityRepository(_)
        | crate::application::login::LoginServiceError::ProfileSerializationFailed(_)
        | crate::application::login::LoginServiceError::TokenIssuanceFailed(_)
        | crate::application::login::LoginServiceError::GoogleOAuthState(_)
        | crate::application::login::LoginServiceError::MissingGoogleSubject
        | crate::application::login::LoginServiceError::InconsistentIdentityUser { .. } => {
            AppError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "google_login_failed",
                "Failed to complete Google login.",
                trace_id,
            )
        }
    }
}
