//! Error types for apmw.
//!
//! Uses thiserror for structured error types. Library code never panics.

use thiserror::Error;

/// All errors produced by apmw.
#[derive(Error, Debug)]
pub enum ApmwError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Package manager not found: {0}")]
    PackageManagerNotFound(String),

    #[error("Security scan failed: {0}")]
    SecurityScanFailed(String),

    #[error("Daemon error: {0}")]
    Daemon(String),
}

/// Result type alias for apmw operations.
pub type Result<T> = std::result::Result<T, ApmwError>;
