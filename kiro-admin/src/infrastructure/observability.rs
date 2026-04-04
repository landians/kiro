use std::time::Duration;

use axum::http::{Method, StatusCode};
use opentelemetry::{
    KeyValue,
    metrics::{Counter, Histogram, Meter},
};

#[derive(Clone)]
pub struct HttpObservability {
    requests_total: Counter<u64>,
    responses_total: Counter<u64>,
    request_duration_ms: Histogram<f64>,
    request_body_size_bytes: Histogram<u64>,
    response_body_size_bytes: Histogram<u64>,
}

impl HttpObservability {
    pub fn new(meter: Meter) -> Self {
        Self {
            requests_total: meter
                .u64_counter("kiro_admin_http_server_requests_total")
                .with_description("Total number of HTTP requests received by the admin server.")
                .build(),
            responses_total: meter
                .u64_counter("kiro_admin_http_server_responses_total")
                .with_description("Total number of HTTP responses sent by the admin server.")
                .build(),
            request_duration_ms: meter
                .f64_histogram("kiro_admin_http_server_request_duration_ms")
                .with_description("Admin HTTP request duration in milliseconds.")
                .build(),
            request_body_size_bytes: meter
                .u64_histogram("kiro_admin_http_server_request_body_size_bytes")
                .with_description("Admin HTTP request body size in bytes.")
                .build(),
            response_body_size_bytes: meter
                .u64_histogram("kiro_admin_http_server_response_body_size_bytes")
                .with_description("Admin HTTP response body size in bytes.")
                .build(),
        }
    }

    pub fn record_request(&self, method: &Method, route: &str, request_body_size: usize) {
        let attributes = base_http_attributes(method, route);

        self.requests_total.add(1, &attributes);
        self.request_body_size_bytes
            .record(request_body_size as u64, &attributes);
    }

    pub fn record_response(
        &self,
        method: &Method,
        route: &str,
        status: StatusCode,
        elapsed: Duration,
        response_body_size: usize,
    ) {
        let attributes = response_http_attributes(method, route, status);

        self.responses_total.add(1, &attributes);
        self.request_duration_ms
            .record(elapsed.as_secs_f64() * 1000.0, &attributes);
        self.response_body_size_bytes
            .record(response_body_size as u64, &attributes);
    }
}

fn base_http_attributes(method: &Method, route: &str) -> [KeyValue; 2] {
    [
        KeyValue::new("http.request.method", method.to_string()),
        KeyValue::new("http.route", route.to_owned()),
    ]
}

fn response_http_attributes(method: &Method, route: &str, status: StatusCode) -> [KeyValue; 3] {
    [
        KeyValue::new("http.request.method", method.to_string()),
        KeyValue::new("http.route", route.to_owned()),
        KeyValue::new("http.response.status_code", i64::from(status.as_u16())),
    ]
}
