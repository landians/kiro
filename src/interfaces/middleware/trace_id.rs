use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;
use uuid::Uuid;

pub const TRACE_ID_HEADER_NAME: HeaderName = HeaderName::from_static("x-trace-id");

#[derive(Clone, Debug)]
pub struct RequestTrace {
    trace_id: String,
}

impl RequestTrace {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
        }
    }

    pub fn trace_id(&self) -> &str {
        self.trace_id.as_str()
    }
}

pub async fn trace_id_middleware(mut request: Request, next: Next) -> Response {
    let trace_id = request
        .headers()
        .get(&TRACE_ID_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .and_then(validate_trace_id)
        .map(ToOwned::to_owned)
        .unwrap_or_else(generate_trace_id);

    let request_trace = RequestTrace::new(trace_id.clone());
    request.extensions_mut().insert(request_trace.clone());

    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let mut response = next.run(request).await;

    response.extensions_mut().insert(request_trace.clone());
    response.headers_mut().insert(
        TRACE_ID_HEADER_NAME,
        HeaderValue::from_str(request_trace.trace_id())
            .expect("generated trace id should be a valid header value"),
    );

    info!(
        trace_id = request_trace.trace_id(),
        http_method = %method,
        http_path = %path,
        http_status = response.status().as_u16(),
        "request completed"
    );

    response
}

fn validate_trace_id(value: &str) -> Option<&str> {
    if value.is_empty() || value.len() > 64 {
        return None;
    }

    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Some(value)
    } else {
        None
    }
}

fn generate_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}
