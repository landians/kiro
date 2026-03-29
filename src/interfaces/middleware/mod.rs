use std::time::Instant;

use axum::{
    body::{Body, Bytes, to_bytes},
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument, field::Empty, info, info_span};
use ulid::Ulid;

use crate::interfaces::SharedState;

const LOG_BODY_LIMIT_BYTES: usize = 64 * 1024;
const TRACE_ID_HEADER_NAME: HeaderName = HeaderName::from_static("x-trace-id");

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
