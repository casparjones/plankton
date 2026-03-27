// Einheitliche Fehlerbehandlung für alle API-Handler.

use axum::{http::StatusCode, response::IntoResponse, Json};

/// Einheitlicher Fehlertyp für alle API-Handler.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Konvertiert `ApiError` in eine HTTP-Antwort mit JSON-Body `{"error": "..."}`.
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m),
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            ApiError::Request(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            ApiError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::Json(e) => (StatusCode::BAD_REQUEST, e.to_string()),
        };
        // Structured errors: "CODE:details" → { "error": "details", "code": "CODE" }
        let (code, message) = if let Some(idx) = msg.find(':') {
            let c = &msg[..idx];
            if c.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_') {
                (Some(c.to_string()), msg[idx + 1..].trim().to_string())
            } else {
                (None, msg)
            }
        } else {
            (None, msg)
        };
        let body = if let Some(c) = code {
            serde_json::json!({"error": message, "code": c})
        } else {
            serde_json::json!({"error": message})
        };
        (status, Json(body)).into_response()
    }
}
