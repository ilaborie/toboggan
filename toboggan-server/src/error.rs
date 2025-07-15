use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize, derive_more::Display, derive_more::Error)]
#[non_exhaustive]
pub enum ApiError {
    #[display("Maximum number of clients exceeded")]
    TooManyClients,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            ApiError::TooManyClients => StatusCode::TOO_MANY_REQUESTS,
        };
        let message = self.to_string();
        let payload = json!({
            "message": message,
            "error": self,
        });

        (status, Json(payload)).into_response()
    }
}
