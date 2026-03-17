use axum::Extension;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use tracing::error;

use crate::application::login::GoogleLoginCallbackCommand;
use crate::interfaces::AppState;
use crate::interfaces::dto::auth::{
    CurrentUserResponse, GoogleAuthorizationUrlResponse, GoogleCallbackQuery,
    GoogleCallbackResponse, LogoutSessionResponse, ProtectedSessionResponse, RefreshTokenResponse,
};
use crate::interfaces::error::auth::AuthHttpError;
use crate::interfaces::middleware::authentication::{
    AuthenticatedAccessToken, AuthenticatedRefreshToken,
};
use crate::interfaces::middleware::trace_id::RequestTrace;
use crate::interfaces::response::ApiSuccess;

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
) -> Result<ApiSuccess<CurrentUserResponse>, AuthHttpError> {
    let account_service = state
        .services
        .account_service()
        .ok_or_else(AuthHttpError::current_user_unavailable)?;

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
            AuthHttpError::current_user_lookup_failed()
        })?
        .ok_or_else(AuthHttpError::authenticated_user_not_found)?;

    Ok(ApiSuccess::ok(user.into()))
}

pub async fn refresh_session(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    Extension(authenticated): Extension<AuthenticatedRefreshToken>,
) -> Result<ApiSuccess<RefreshTokenResponse>, AuthHttpError> {
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
            AuthHttpError::blacklist_unavailable()
        })?;

    let token_pair = state
        .services
        .auth
        .issue_token_pair(&authenticated.subject, &authenticated.user_agent)
        .map_err(|_| AuthHttpError::token_refresh_failed())?;

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
) -> Result<ApiSuccess<GoogleAuthorizationUrlResponse>, AuthHttpError> {
    let login_service = state
        .services
        .login_service()
        .ok_or_else(AuthHttpError::google_login_unavailable)?;

    let authorization_request =
        login_service
            .build_google_authorization_request()
            .map_err(|error| {
                error!(
                    trace_id = %request_trace.trace_id(),
                    error = %error,
                    "failed to build google authorization url"
                );
                AuthHttpError::from_google_authorization_request_error(error)
            })?;

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
) -> Result<ApiSuccess<GoogleCallbackResponse>, AuthHttpError> {
    if let Some(error_code) = query.error.as_deref() {
        return Err(AuthHttpError::google_authorization_denied(
            error_code,
            query.error_description.as_deref(),
        ));
    }

    let login_service = state
        .services
        .login_service()
        .ok_or_else(AuthHttpError::google_login_unavailable)?;

    let authorization_code = query
        .code
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(AuthHttpError::missing_authorization_code)?;
    let oauth_state = query
        .state
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(AuthHttpError::missing_google_state)?;

    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(AuthHttpError::missing_user_agent)?;

    let login_result = login_service
        .complete_google_login(GoogleLoginCallbackCommand {
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
            AuthHttpError::from_google_callback_error(error)
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
) -> Result<ApiSuccess<LogoutSessionResponse>, AuthHttpError> {
    if access_token.subject != refresh_token.subject {
        return Err(AuthHttpError::token_subject_mismatch());
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
            AuthHttpError::blacklist_unavailable()
        })?;

    Ok(ApiSuccess::ok(LogoutSessionResponse {
        subject: access_token.subject,
        access_token_revoked: true,
        refresh_token_revoked: true,
    }))
}
