//! Structured error model. Every failure maps to a category with a stable code
//! and a user-facing message. Production paths must not panic on recoverable errors.

use serde::Serialize;
use thiserror::Error;

/// Top-level application error. Variants map 1:1 to the documented categories.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("provider error: {message}")]
    Provider { message: String },
    #[error("validation error: {message}")]
    Validation { message: String },
    #[error("market error: {message}")]
    Market { message: String },
    #[error("route error: {message}")]
    Route { message: String },
    #[error("bridge error: {message}")]
    Bridge { message: String },
    #[error("session error: {message}")]
    Session { message: String },
    #[error("configuration error: {message}")]
    Configuration { message: String },
    #[error("network error: {message}")]
    Network { message: String },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    /// Stable machine-readable code for logs/telemetry.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Storage(_) => "storage",
            AppError::Provider { .. } => "provider",
            AppError::Validation { .. } => "validation",
            AppError::Market { .. } => "market",
            AppError::Route { .. } => "route",
            AppError::Bridge { .. } => "bridge",
            AppError::Session { .. } => "session",
            AppError::Configuration { .. } => "configuration",
            AppError::Network { .. } => "network",
            AppError::Serialization(_) => "serialization",
            AppError::Io(_) => "io",
        }
    }

    /// User-facing message: no panics, no internal paths, actionable where possible.
    pub fn user_message(&self) -> String {
        match self {
            AppError::Storage(e) => format!("Database error: {e}"),
            AppError::Provider { message } => format!("Data provider error: {message}"),
            AppError::Validation { message } => format!("Invalid data: {message}"),
            AppError::Market { message } => format!("Market data error: {message}"),
            AppError::Route { message } => format!("Route error: {message}"),
            AppError::Bridge { message } => format!("Addon sync error: {message}"),
            AppError::Session { message } => format!("Session error: {message}"),
            AppError::Configuration { message } => format!("Configuration error: {message}"),
            AppError::Network { message } => {
                format!("Network error: {message}. Check your connection and retry.")
            }
            AppError::Serialization(e) => format!("Data format error: {e}"),
            AppError::Io(e) => format!("File error: {e}"),
        }
    }

    pub fn provider(msg: impl Into<String>) -> Self {
        AppError::Provider { message: msg.into() }
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation { message: msg.into() }
    }
    pub fn market(msg: impl Into<String>) -> Self {
        AppError::Market { message: msg.into() }
    }
    pub fn route(msg: impl Into<String>) -> Self {
        AppError::Route { message: msg.into() }
    }
    pub fn bridge(msg: impl Into<String>) -> Self {
        AppError::Bridge { message: msg.into() }
    }
    pub fn session(msg: impl Into<String>) -> Self {
        AppError::Session { message: msg.into() }
    }
    pub fn config(msg: impl Into<String>) -> Self {
        AppError::Configuration { message: msg.into() }
    }
    pub fn network(msg: impl Into<String>) -> Self {
        AppError::Network { message: msg.into() }
    }
}

/// JSON-serializable error payload returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

impl From<&AppError> for ErrorPayload {
    fn from(err: &AppError) -> Self {
        ErrorPayload { code: err.code().to_string(), message: err.user_message() }
    }
}

/// Convenience alias used across the crate.
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(AppError::route("x").code(), "route");
        assert_eq!(AppError::market("x").code(), "market");
        assert_eq!(AppError::bridge("x").code(), "bridge");
    }

    #[test]
    fn user_messages_are_actionable() {
        let msg = AppError::network("timeout").user_message();
        assert!(msg.contains("Network error"));
        assert!(msg.contains("retry"));
    }
}
