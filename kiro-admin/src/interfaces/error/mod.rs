use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use validator::ValidationErrors;

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

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code,
            message: message.into(),
        }
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code,
            message: message.into(),
        }
    }

    pub fn internal_server_error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code,
            message: message.into(),
        }
    }
}

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        Self::bad_request("invalid_request", errors.to_string())
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
