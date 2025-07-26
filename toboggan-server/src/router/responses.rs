use std::time::Duration;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use toboggan_core::Timestamp;

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub timestamp: Timestamp,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            timestamp: Timestamp::now(),
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub timestamp: Timestamp,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            timestamp: Timestamp::now(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(message, "BAD_REQUEST")
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(message, "INTERNAL_ERROR")
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message, "NOT_FOUND")
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(message, "VALIDATION_ERROR")
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "BAD_REQUEST" | "VALIDATION_ERROR" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: HealthResponseStatus,
    pub started_at: Timestamp,
    pub elapsed: Duration,
    pub talk: String,
    pub active_clients: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HealthResponseStatus {
    Ok,
    Oops,
}

#[derive(Debug, Clone, Serialize)]
pub struct TalkResponse {
    pub talk: toboggan_core::Talk,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlidesResponse {
    pub slides: std::collections::HashMap<toboggan_core::SlideId, toboggan_core::Slide>,
}
