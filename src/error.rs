use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    DatabaseError(String),
    ExternalApiError(String),
    ValidationError(String),
    PriceOracleError(String),
    NotFound(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ApiError::ExternalApiError(msg) => write!(f, "External API error: {}", msg),
            ApiError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ApiError::PriceOracleError(msg) => write!(f, "Price oracle error: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::DatabaseError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Internal server error",
                    "message": self.to_string()
                }))
            }
            ApiError::ExternalApiError(_) => {
                HttpResponse::ServiceUnavailable().json(serde_json::json!({
                    "error": "Service unavailable",
                    "message": self.to_string()
                }))
            }
            ApiError::ValidationError(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Bad request",
                    "message": self.to_string()
                }))
            }
            ApiError::PriceOracleError(_) => {
                HttpResponse::ServiceUnavailable().json(serde_json::json!({
                    "error": "Price service unavailable",
                    "message": self.to_string()
                }))
            }
            ApiError::NotFound(_) => {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Not found",
                    "message": self.to_string()
                }))
            }
        }
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        ApiError::DatabaseError(err.to_string())
    }
}

impl From<r2d2::Error> for ApiError {
    fn from(err: r2d2::Error) -> Self {
        ApiError::DatabaseError(err.to_string())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::ExternalApiError(err.to_string())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;