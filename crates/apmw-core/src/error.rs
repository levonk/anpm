//! Error types for apmw-core.
//!
//! Uses thiserror for structured error types. Library code never panics.

use thiserror::Error;

/// Errors produced by the core detection, ecosystem, and version modules.
#[derive(Error, Debug)]
pub enum CoreError {
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Package manager not found: {0}")]
  PackageManagerNotFound(String),

  #[error("Version resolution error: {0}")]
  VersionResolution(String),

  #[error("JSON error: {0}")]
  Json(#[from] serde_json::Error),

  #[error("TOML error: {0}")]
  Toml(#[from] toml::de::Error),
}

/// Result type alias for apmw-core operations.
pub type Result<T> = std::result::Result<T, CoreError>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_io_error_display() {
    let err = CoreError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
    assert_eq!(err.to_string(), "IO error: missing");
  }

  #[test]
  fn test_package_manager_not_found_display() {
    let err = CoreError::PackageManagerNotFound("npm".to_string());
    assert_eq!(err.to_string(), "Package manager not found: npm");
  }

  #[test]
  fn test_version_resolution_display() {
    let err = CoreError::VersionResolution("no match".to_string());
    assert_eq!(err.to_string(), "Version resolution error: no match");
  }

  #[test]
  fn test_json_error_from() {
    let json_err = serde_json::from_str::<serde_json::Value>("{bad}").unwrap_err();
    let err: CoreError = json_err.into();
    assert!(err.to_string().starts_with("JSON error: "));
  }

  #[test]
  fn test_toml_error_from() {
    let toml_err = toml::from_str::<toml::Value>("bad = ").unwrap_err();
    let err: CoreError = toml_err.into();
    assert!(err.to_string().starts_with("TOML error: "));
  }

  #[test]
  fn test_error_debug() {
    let err = CoreError::VersionResolution("x".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("VersionResolution"));
  }
}
