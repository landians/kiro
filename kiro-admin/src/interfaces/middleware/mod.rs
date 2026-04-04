use std::time::Instant;

use axum::{
    body::{Body, Bytes, to_bytes},
    extract::{MatchedPath, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument, Span, field::Empty, info, info_span};
use ulid::Ulid;

use crate::{
    infrastructure::auth::{AuthError, TokenClaims},
    interfaces::{SharedState, error::AppError},
};

const LOG_BODY_LIMIT_BYTES: usize = 64 * 1024;
const TRACE_ID_HEADER_NAME: HeaderName = HeaderName::from_static("x-trace-id");
const BEARER_PREFIX: &str = "Bearer ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceId(String);

impl TraceId {
    fn generate() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthenticatedAdminUser {
    pub admin_user_id: i64,
    pub token_claims: TokenClaims,
}

pub async fn log_request_response(
    State(state): State<SharedState>,
    mut request: Request,
    next: Next,
) -> Response {
    let started_at = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let trace_id = TraceId::generate();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| uri.path())
        .to_owned();
    let span = info_span!(
        "http.request",
        otel.name = %format!("{method} {route}"),
        trace_id = %trace_id.as_str(),
        http.method = %method,
        http.route = %route,
        http.target = %uri,
        enduser.id = Empty,
        http.status_code = Empty,
        http.request.body.size = Empty,
        http.response.body.size = Empty,
        http.duration_ms = Empty,
    );
    request.extensions_mut().insert(trace_id.clone());
    let (request, request_body) = buffer_request(request).await;
    let request_body_len = request_body.len();

    span.record("http.request.body.size", request_body_len as u64);
    info!(
        parent: &span,
        request_body = %render_body(&request_body),
        "incoming request",
    );

    state
        .http_observability()
        .record_request(&method, &route, request_body_len);

    let response = next.run(request).instrument(span.clone()).await;
    let status = response.status();
    let (response, response_body) = buffer_response(response).await;
    let elapsed = started_at.elapsed();
    let response_body_len = response_body.len();

    span.record("http.status_code", i64::from(status.as_u16()));
    span.record("http.response.body.size", response_body_len as u64);
    span.record("http.duration_ms", elapsed.as_millis() as u64);
    info!(
        parent: &span,
        response_body = %render_body(&response_body),
        "outgoing response",
    );

    state
        .http_observability()
        .record_response(&method, &route, status, elapsed, response_body_len);

    attach_trace_id_header(response, &trace_id)
}

pub async fn validate_admin_token(
    State(state): State<SharedState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(request.headers())?;
    let token_claims = state
        .auth_service()
        .validate_access_token(token)
        .map_err(map_access_token_error)?;
    let admin_user_id = token_claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::unauthorized("invalid_token_subject", "invalid token subject"))?;

    Span::current().record("enduser.id", admin_user_id);
    request.extensions_mut().insert(AuthenticatedAdminUser {
        admin_user_id,
        token_claims,
    });

    Ok(next.run(request).await)
}

async fn buffer_request(request: Request) -> (Request, Bytes) {
    let (parts, body) = request.into_parts();
    let body_bytes = read_body(body).await;
    let request = Request::from_parts(parts, Body::from(body_bytes.clone()));

    (request, body_bytes)
}

async fn buffer_response(response: Response) -> (Response, Bytes) {
    let (parts, body) = response.into_parts();
    let body_bytes = read_body(body).await;
    let response = Response::from_parts(parts, Body::from(body_bytes.clone()));

    (response, body_bytes)
}

async fn read_body(body: Body) -> Bytes {
    match to_bytes(body, LOG_BODY_LIMIT_BYTES).await {
        Ok(bytes) => bytes,
        Err(error) => Bytes::from(format!(
            "__failed_to_read_body__: {error}; limit={LOG_BODY_LIMIT_BYTES}"
        )),
    }
}

fn render_body(body: &Bytes) -> String {
    if body.is_empty() {
        return "<empty>".to_owned();
    }

    String::from_utf8_lossy(body).into_owned()
}

fn attach_trace_id_header(mut response: Response, trace_id: &TraceId) -> Response {
    let Ok(value) = HeaderValue::from_str(trace_id.as_str()) else {
        return response;
    };

    response.headers_mut().insert(TRACE_ID_HEADER_NAME, value);
    response
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

    let token = authorization.strip_prefix(BEARER_PREFIX).ok_or_else(|| {
        AppError::unauthorized(
            "invalid_authorization_scheme",
            "authorization header must use Bearer scheme",
        )
    })?;

    if token.trim().is_empty() {
        return Err(AppError::unauthorized(
            "invalid_access_token",
            "access token cannot be empty",
        ));
    }

    Ok(token)
}

fn map_access_token_error(error: AuthError) -> AppError {
    match error {
        AuthError::Jwt(_) | AuthError::InvalidTokenType { .. } => {
            AppError::unauthorized("invalid_access_token", "invalid access token")
        }
        other => AppError::internal_server_error("auth_service_error", other.to_string()),
    }
}
