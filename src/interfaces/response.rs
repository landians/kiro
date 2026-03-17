use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiSuccessEnvelope<T> {
    pub success: bool,
    pub data: T,
}

pub struct ApiSuccess<T> {
    status: StatusCode,
    data: T,
}

impl<T> ApiSuccess<T> {
    pub fn ok(data: T) -> Self {
        Self {
            status: StatusCode::OK,
            data,
        }
    }
}

impl<T> IntoResponse for ApiSuccess<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiSuccessEnvelope {
                success: true,
                data: self.data,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct ApiErrorEnvelope {
    pub success: bool,
    pub error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AppError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "route_not_found",
            "The requested route does not exist.",
        )
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorEnvelope {
                success: false,
                error: ApiErrorBody {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}
