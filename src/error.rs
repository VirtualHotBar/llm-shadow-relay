//! Error types for LLM Shadow Relay

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Main error type for the application
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// Invalid client request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Upstream API error (contains status code and message)
    #[error("Upstream API error: {0} - {1}")]
    UpstreamApi(u16, String),

    /// Audit failed
    #[error("Audit failed: {0}")]
    AuditFailed(String),

    /// Audit blocked (content rejected)
    #[error("Audit blocked: {0}")]
    AuditBlocked(String),

    /// Invalid response from upstream
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::Config(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::HttpRequest(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            Error::InvalidRequest(e) => (StatusCode::BAD_REQUEST, e),
            Error::UpstreamApi(status, msg) => (
                StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
                msg,
            ),
            Error::AuditFailed(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::AuditBlocked(e) => (StatusCode::FORBIDDEN, e.to_string()),
            Error::InvalidResponse(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            Error::Json(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": "llm_shadow_error",
            }
        }));

        (status, body).into_response()
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Audit decision types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditDecision {
    pub allowed: bool,
    pub risk_level: String,
    pub risk_score: f32,
    pub findings: Vec<AuditFinding>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditFinding {
    pub category: String,
    pub severity: String,
    pub description: String,
    pub evidence: Option<String>,
}

impl AuditDecision {
    pub fn pass() -> Self {
        Self {
            allowed: true,
            risk_level: "none".to_string(),
            risk_score: 0.0,
            findings: vec![],
            blocked_reason: None,
        }
    }

    pub fn block(reason: &str, risk_level: &str, risk_score: f32) -> Self {
        Self {
            allowed: false,
            risk_level: risk_level.to_string(),
            risk_score,
            findings: vec![],
            blocked_reason: Some(reason.to_string()),
        }
    }
}
