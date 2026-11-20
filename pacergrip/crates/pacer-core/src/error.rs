use thiserror::Error;

/// Unified error type across all PacerGrip core, parser, client, and backend crates.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("authentication required: {0}")]
    AuthRequired(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("Pacer API error [{code}]: {message}")]
    ApiError { code: String, message: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("unsupported operation: {0}")]
    Unsupported(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Transport(err.to_string())
    }
}
