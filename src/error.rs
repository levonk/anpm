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

  #[error("Version resolution error: {0}")]
  VersionResolution(String),

  #[error("Ecosystem mapping error: {0}")]
  EcosystemMapping(String),

  #[error("Clone failed: {0}")]
  CloneFailed(String),

  #[error("Path scan failed: {0}")]
  PathScanFailed(String),

  #[error("MCP error: {0}")]
  McpError(String),

  #[error("Hook error: {0}")]
  HookError(String),

  #[error("Audit log error: {0}")]
  AuditLogError(String),

  #[error("Index error: {0}")]
  IndexError(String),

  #[error("JSON error: {0}")]
  Json(#[from] serde_json::Error),

  #[error("TOML error: {0}")]
  Toml(#[from] toml::de::Error),
}

/// Result type alias for apmw operations.
pub type Result<T> = std::result::Result<T, ApmwError>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_io_error_display() {
    let err = ApmwError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
    assert_eq!(err.to_string(), "IO error: missing");
  }

  #[test]
  fn test_config_error_display() {
    let err = ApmwError::Config("bad value".to_string());
    assert_eq!(err.to_string(), "Configuration error: bad value");
  }

  #[test]
  fn test_package_manager_not_found_display() {
    let err = ApmwError::PackageManagerNotFound("npm".to_string());
    assert_eq!(err.to_string(), "Package manager not found: npm");
  }

  #[test]
  fn test_security_scan_failed_display() {
    let err = ApmwError::SecurityScanFailed("vuln".to_string());
    assert_eq!(err.to_string(), "Security scan failed: vuln");
  }

  #[test]
  fn test_daemon_error_display() {
    let err = ApmwError::Daemon("timeout".to_string());
    assert_eq!(err.to_string(), "Daemon error: timeout");
  }

  #[test]
  fn test_version_resolution_display() {
    let err = ApmwError::VersionResolution("no match".to_string());
    assert_eq!(err.to_string(), "Version resolution error: no match");
  }

  #[test]
  fn test_ecosystem_mapping_display() {
    let err = ApmwError::EcosystemMapping("unknown".to_string());
    assert_eq!(err.to_string(), "Ecosystem mapping error: unknown");
  }

  #[test]
  fn test_clone_failed_display() {
    let err = ApmwError::CloneFailed("repo".to_string());
    assert_eq!(err.to_string(), "Clone failed: repo");
  }

  #[test]
  fn test_path_scan_failed_display() {
    let err = ApmwError::PathScanFailed("denied".to_string());
    assert_eq!(err.to_string(), "Path scan failed: denied");
  }

  #[test]
  fn test_mcp_error_display() {
    let err = ApmwError::McpError("conn".to_string());
    assert_eq!(err.to_string(), "MCP error: conn");
  }

  #[test]
  fn test_hook_error_display() {
    let err = ApmwError::HookError("pre-add".to_string());
    assert_eq!(err.to_string(), "Hook error: pre-add");
  }

  #[test]
  fn test_audit_log_error_display() {
    let err = ApmwError::AuditLogError("write".to_string());
    assert_eq!(err.to_string(), "Audit log error: write");
  }

  #[test]
  fn test_index_error_display() {
    let err = ApmwError::IndexError("ast".to_string());
    assert_eq!(err.to_string(), "Index error: ast");
  }

  #[test]
  fn test_json_error_from() {
    let json_err = serde_json::from_str::<serde_json::Value>("{bad}").unwrap_err();
    let err: ApmwError = json_err.into();
    assert!(err.to_string().starts_with("JSON error: "));
  }

  #[test]
  fn test_toml_error_from() {
    let toml_err = toml::from_str::<toml::Value>("bad = ").unwrap_err();
    let err: ApmwError = toml_err.into();
    assert!(err.to_string().starts_with("TOML error: "));
  }

  #[test]
  fn test_error_debug() {
    let err = ApmwError::Config("x".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("Config"));
  }
}
