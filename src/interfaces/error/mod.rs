use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AppError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code,
            message: message.into(),
        }
    }

    pub fn bad_gateway(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse {
            code: self.code,
            message: self.message,
        });

        (self.status, body).into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}
