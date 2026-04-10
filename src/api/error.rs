//! Unified API error type for JSON error responses
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
/// API error variants returned by handler functions
pub enum ApiError {
    /// 400 Bad Request
    BadRequest(String),
    /// 401 Unauthorized
    Unauthorized,
    /// 404 Not Found
    NotFound(String),
    /// 403 Forbidden
    Forbidden(String),
    /// 500 Internal Server Error
    Internal(String),
}

/// [`ApiError`] implementation of [`IntoResponse`]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // --------------------------------------------------
        // map each error variant to an HTTP status code and
        // a human-readable message string
        // --------------------------------------------------
        let (status, message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        // --------------------------------------------------
        // serialize into a JSON body with a single "error"
        // key and return with the appropriate status code
        // --------------------------------------------------
        (status, axum::Json(json!({ "error": message }))).into_response()
    }
}
