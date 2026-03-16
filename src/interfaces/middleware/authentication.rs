use axum::Extension;
use axum::extract::{Request, State};
use axum::http::HeaderName;
use axum::http::header::{AUTHORIZATION, USER_AGENT};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::{error, warn};

use crate::infrastructure::auth::jwt::{JwtError, TokenKind};
use crate::interfaces::AppState;
use crate::interfaces::middleware::trace_id::RequestTrace;
use crate::interfaces::response::AppError;

#[derive(Clone, Debug)]
pub struct AuthenticatedAccessToken {
    pub subject: String,
    pub jti: String,
    pub ua_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AuthenticatedRefreshToken {
    pub subject: String,
    pub jti: String,
    pub ua_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub user_agent: String,
}

pub const REFRESH_TOKEN_HEADER_NAME: HeaderName = HeaderName::from_static("x-refresh-token");

pub async fn require_access_token(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    mut request: Request,
    next: Next,
) -> Response {
    let trace_id = request_trace.trace_id().to_owned();

    let Some(token) = extract_bearer_token(&request) else {
        return AppError::unauthorized(
            "missing_bearer_token",
            "Authorization header must use Bearer token.",
            trace_id,
        )
        .into_response();
    };

    let Some(user_agent) = request
        .headers()
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
    else {
        return AppError::unauthorized(
            "missing_user_agent",
            "User-Agent header is required for authenticated requests.",
            trace_id,
        )
        .into_response();
    };

    let validated = match state.services.auth.validate_access_token(token) {
        Ok(validated) => validated,
        Err(error) => return map_jwt_error_to_response(error, trace_id, TokenKind::Access),
    };

    let request_ua_hash = match state.services.auth.hash_user_agent(user_agent) {
        Ok(ua_hash) => ua_hash,
        Err(error) => return map_jwt_error_to_response(error, trace_id, TokenKind::Access),
    };

    if request_ua_hash != validated.ua_hash {
        return map_jwt_error_to_response(JwtError::UserAgentMismatch, trace_id, TokenKind::Access);
    }

    let is_revoked = match state
        .services
        .auth
        .is_access_token_revoked(&validated.jti)
        .await
    {
        Ok(is_revoked) => is_revoked,
        Err(blacklist_error) => {
            error!(
                trace_id = %trace_id,
                error = %blacklist_error,
                "failed to query token blacklist"
            );
            return blacklist_unavailable_response(trace_id);
        }
    };

    if is_revoked {
        return map_jwt_error_to_response(JwtError::TokenRevoked, trace_id, TokenKind::Access);
    }

    request.extensions_mut().insert(AuthenticatedAccessToken {
        subject: validated.subject,
        jti: validated.jti,
        ua_hash: validated.ua_hash,
        issued_at: validated.issued_at,
        expires_at: validated.expires_at,
    });

    next.run(request).await
}

pub async fn require_refresh_token(
    State(state): State<AppState>,
    Extension(request_trace): Extension<RequestTrace>,
    mut request: Request,
    next: Next,
) -> Response {
    let trace_id = request_trace.trace_id().to_owned();

    let Some(token) = request
        .headers()
        .get(&REFRESH_TOKEN_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
    else {
        return AppError::unauthorized(
            "missing_refresh_token",
            "Refresh token header is required.",
            trace_id,
        )
        .into_response();
    };

    let Some(user_agent) = request
        .headers()
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
    else {
        return AppError::unauthorized(
            "missing_user_agent",
            "User-Agent header is required for authenticated requests.",
            trace_id,
        )
        .into_response();
    };
    let user_agent = user_agent.to_owned();

    let validated = match state.services.auth.validate_refresh_token(token) {
        Ok(validated) => validated,
        Err(error) => return map_jwt_error_to_response(error, trace_id, TokenKind::Refresh),
    };

    let request_ua_hash = match state.services.auth.hash_user_agent(&user_agent) {
        Ok(ua_hash) => ua_hash,
        Err(error) => return map_jwt_error_to_response(error, trace_id, TokenKind::Refresh),
    };

    if request_ua_hash != validated.ua_hash {
        return map_jwt_error_to_response(
            JwtError::UserAgentMismatch,
            trace_id,
            TokenKind::Refresh,
        );
    }

    let is_revoked = match state
        .services
        .auth
        .is_refresh_token_revoked(&validated.jti)
        .await
    {
        Ok(is_revoked) => is_revoked,
        Err(blacklist_error) => {
            error!(
                trace_id = %trace_id,
                error = %blacklist_error,
                "failed to query token blacklist"
            );
            return blacklist_unavailable_response(trace_id);
        }
    };

    if is_revoked {
        return map_jwt_error_to_response(JwtError::TokenRevoked, trace_id, TokenKind::Refresh);
    }

    request.extensions_mut().insert(AuthenticatedRefreshToken {
        subject: validated.subject,
        jti: validated.jti,
        ua_hash: validated.ua_hash,
        issued_at: validated.issued_at,
        expires_at: validated.expires_at,
        user_agent,
    });

    next.run(request).await
}

fn extract_bearer_token(request: &Request) -> Option<&str> {
    let header_value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = header_value.strip_prefix("Bearer ")?;
    if token.trim().is_empty() {
        None
    } else {
        Some(token)
    }
}

fn map_jwt_error_to_response(error: JwtError, trace_id: String, token_kind: TokenKind) -> Response {
    warn!(trace_id = %trace_id, error = %error, "authentication failed");

    match error {
        JwtError::MissingUserAgent => AppError::unauthorized(
            "missing_user_agent",
            "User-Agent header is required for authenticated requests.",
            trace_id,
        )
        .into_response(),
        JwtError::TokenExpired => {
            AppError::unauthorized("token_expired", token_expired_message(token_kind), trace_id)
                .into_response()
        }
        JwtError::UnexpectedTokenKind { .. } => AppError::unauthorized(
            "invalid_token_kind",
            invalid_kind_message(token_kind),
            trace_id,
        )
        .into_response(),
        JwtError::UserAgentMismatch => AppError::unauthorized(
            "user_agent_mismatch",
            user_agent_mismatch_message(token_kind),
            trace_id,
        )
        .into_response(),
        JwtError::TokenRevoked => {
            AppError::unauthorized("token_revoked", token_revoked_message(token_kind), trace_id)
                .into_response()
        }
        _ => AppError::unauthorized(
            invalid_token_code(token_kind),
            invalid_token_message(token_kind),
            trace_id,
        )
        .into_response(),
    }
}

fn invalid_token_code(token_kind: TokenKind) -> &'static str {
    match token_kind {
        TokenKind::Access => "invalid_access_token",
        TokenKind::Refresh => "invalid_refresh_token",
    }
}

fn invalid_token_message(token_kind: TokenKind) -> &'static str {
    match token_kind {
        TokenKind::Access => "Access token is invalid.",
        TokenKind::Refresh => "Refresh token is invalid.",
    }
}

fn blacklist_unavailable_response(trace_id: String) -> Response {
    AppError::new(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "blacklist_unavailable",
        "Token blacklist backend is unavailable.",
        trace_id,
    )
    .into_response()
}

fn invalid_kind_message(token_kind: TokenKind) -> &'static str {
    match token_kind {
        TokenKind::Access => "Access token is invalid for this endpoint.",
        TokenKind::Refresh => "Refresh token is invalid for this endpoint.",
    }
}

fn token_expired_message(token_kind: TokenKind) -> &'static str {
    match token_kind {
        TokenKind::Access => "Access token has expired.",
        TokenKind::Refresh => "Refresh token has expired.",
    }
}

fn user_agent_mismatch_message(token_kind: TokenKind) -> &'static str {
    match token_kind {
        TokenKind::Access => "Access token does not match the current user agent.",
        TokenKind::Refresh => "Refresh token does not match the current user agent.",
    }
}

fn token_revoked_message(token_kind: TokenKind) -> &'static str {
    match token_kind {
        TokenKind::Access => "Access token has been revoked.",
        TokenKind::Refresh => "Refresh token has been revoked.",
    }
}
