use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use tracing::error;

use crate::infrastructure::auth::jwt::TokenKind;
use crate::interfaces::AppState;
use crate::interfaces::dto::auth::{
    LogoutSessionResponse, ProtectedSessionResponse, RefreshTokenResponse,
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

pub async fn refresh_session(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    Extension(authenticated): Extension<AuthenticatedRefreshToken>,
) -> Result<ApiSuccess<RefreshTokenResponse>, AppError> {
    state
        .infrastructure
        .auth
        .blacklist_service
        .revoke(
            TokenKind::Refresh,
            &authenticated.jti,
            authenticated.expires_at,
        )
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
        .infrastructure
        .auth
        .jwt_service
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
        .infrastructure
        .auth
        .blacklist_service
        .revoke_many(&[
            crate::infrastructure::auth::blacklist::TokenRevocation {
                token_kind: TokenKind::Access,
                jti: &access_token.jti,
                expires_at: access_token.expires_at,
            },
            crate::infrastructure::auth::blacklist::TokenRevocation {
                token_kind: TokenKind::Refresh,
                jti: &refresh_token.jti,
                expires_at: refresh_token.expires_at,
            },
        ])
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
