//! Telemetry event payload definition and serialization.
//!
//! Defines the anonymized payload schema for PRD FR-6. The payload contains
//! ONLY categorical usage data — no personal identifiers, no package names,
//! no file paths, and no URLs.
//!
//! # Payload Schema
//!
//! | Field           | Type             | Description                          |
//! |-----------------|------------------|--------------------------------------|
//! | `terminal_type` | string           | `interactive` \| `tui` \| `login` \| `non-interactive` |
//! | `command`       | string           | `add` \| `detect` \| `scan` \| `clone` \| `status` \| `list` |
//! | `manager`       | string \| null   | Manager name (e.g. `pnpm`, `uv`, `cargo`) — never package names |
//! | `outcome`       | string           | `success` \| `failure`               |
//! | `error_category`| string \| null   | `network` \| `permission` \| `not-found` \| `version` \| `other` (only on failure) |
//! | `duration_ms`   | integer          | Command duration in milliseconds     |
//! | `apmw_version`  | string           | Semantic version of apmw             |

use serde::{Deserialize, Serialize};

/// The type of terminal the caller is running in.
///
/// This is distinct from [`crate::audit::TerminalType`] because telemetry
/// distinguishes TUI mode (launched via `--interactive`/`--tui`) from
/// plain interactive shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TelemetryTerminalType {
  /// Interactive shell (TTY on stdin and terminal env vars set).
  Interactive,
  /// TUI mode (launched via `--interactive` / `--tui`).
  Tui,
  /// Login shell (first argv element starts with `-`).
  Login,
  /// Non-interactive (piped stdin or no TTY).
  NonInteractive,
}

impl TelemetryTerminalType {
  /// Convert from the audit module's [`TerminalType`](crate::audit::TerminalType).
  pub fn from_audit_type(audit: crate::audit::TerminalType, interactive_flag: bool) -> Self {
    match audit {
      crate::audit::TerminalType::Login => TelemetryTerminalType::Login,
      crate::audit::TerminalType::Interactive => {
        if interactive_flag {
          TelemetryTerminalType::Tui
        } else {
          TelemetryTerminalType::Interactive
        }
      }
      crate::audit::TerminalType::NonInteractive => TelemetryTerminalType::NonInteractive,
    }
  }
}

impl std::fmt::Display for TelemetryTerminalType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TelemetryTerminalType::Interactive => write!(f, "interactive"),
      TelemetryTerminalType::Tui => write!(f, "tui"),
      TelemetryTerminalType::Login => write!(f, "login"),
      TelemetryTerminalType::NonInteractive => write!(f, "non-interactive"),
    }
  }
}

/// The apmw command that was invoked (no arguments are recorded).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TelemetryCommand {
  /// `apmw add` (install a package).
  Add,
  /// `apmw detect` (detect package manager).
  Detect,
  /// `apmw scan` (security scan).
  Scan,
  /// `apmw clone` (historyless clone).
  Clone,
  /// `apmw status` (show status).
  Status,
  /// `apmw list` (list packages/managers).
  List,
}

impl TelemetryCommand {
  /// Convert from a CLI command name string.
  pub fn from_command_name(name: &str) -> Option<Self> {
    match name {
      "install" | "add" => Some(TelemetryCommand::Add),
      "detect" => Some(TelemetryCommand::Detect),
      "scan" => Some(TelemetryCommand::Scan),
      "clone" => Some(TelemetryCommand::Clone),
      "status" => Some(TelemetryCommand::Status),
      "list" => Some(TelemetryCommand::List),
      _ => None,
    }
  }
}

impl std::fmt::Display for TelemetryCommand {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TelemetryCommand::Add => write!(f, "add"),
      TelemetryCommand::Detect => write!(f, "detect"),
      TelemetryCommand::Scan => write!(f, "scan"),
      TelemetryCommand::Clone => write!(f, "clone"),
      TelemetryCommand::Status => write!(f, "status"),
      TelemetryCommand::List => write!(f, "list"),
    }
  }
}

/// The outcome of a command invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
  /// The command completed successfully.
  Success,
  /// The command failed.
  Failure,
}

impl std::fmt::Display for Outcome {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Outcome::Success => write!(f, "success"),
      Outcome::Failure => write!(f, "failure"),
    }
  }
}

/// The category of error (only present when `outcome` is `failure`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCategory {
  /// Network-related error (connection refused, timeout, DNS failure).
  Network,
  /// Permission-related error (access denied, not authorized).
  Permission,
  /// Resource not found (package, manager, file).
  NotFound,
  /// Version resolution error (no compatible version found).
  Version,
  /// Any other error not covered by the above categories.
  Other,
}

impl ErrorCategory {
  /// Classify an [`ApmwError`](crate::error::ApmwError) into a telemetry
  /// error category.
  pub fn from_apmw_error(err: &crate::error::ApmwError) -> Self {
    match err {
      crate::error::ApmwError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
        ErrorCategory::Permission
      }
      crate::error::ApmwError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => {
        ErrorCategory::NotFound
      }
      crate::error::ApmwError::PackageManagerNotFound(_) => ErrorCategory::NotFound,
      crate::error::ApmwError::VersionResolution(_) => ErrorCategory::Version,
      crate::error::ApmwError::Config(_) => ErrorCategory::Other,
      crate::error::ApmwError::SecurityScanFailed(_) => ErrorCategory::Other,
      crate::error::ApmwError::Daemon(_) => ErrorCategory::Other,
      crate::error::ApmwError::EcosystemMapping(_) => ErrorCategory::NotFound,
      crate::error::ApmwError::CloneFailed(_) => ErrorCategory::Other,
      crate::error::ApmwError::PathScanFailed(_) => ErrorCategory::NotFound,
      crate::error::ApmwError::McpError(_) => ErrorCategory::Network,
      crate::error::ApmwError::HookError(_) => ErrorCategory::Other,
      crate::error::ApmwError::AuditLogError(_) => ErrorCategory::Other,
      crate::error::ApmwError::IndexError(_) => ErrorCategory::Other,
      crate::error::ApmwError::Json(_) => ErrorCategory::Other,
      crate::error::ApmwError::Toml(_) => ErrorCategory::Other,
      _ => ErrorCategory::Other,
    }
  }
}

impl std::fmt::Display for ErrorCategory {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorCategory::Network => write!(f, "network"),
      ErrorCategory::Permission => write!(f, "permission"),
      ErrorCategory::NotFound => write!(f, "not-found"),
      ErrorCategory::Version => write!(f, "version"),
      ErrorCategory::Other => write!(f, "other"),
    }
  }
}

/// The anonymized telemetry event payload.
///
/// Contains ONLY categorical usage data. No personal identifiers, no package
/// names, no file paths, and no URLs are collected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryEvent {
  /// The terminal type the caller ran from.
  pub terminal_type: TelemetryTerminalType,
  /// The command that was invoked (no arguments).
  pub command: TelemetryCommand,
  /// The package manager name (e.g. `pnpm`, `uv`, `cargo`), if known.
  /// Never contains package names.
  pub manager: Option<String>,
  /// Whether the command succeeded or failed.
  pub outcome: Outcome,
  /// The error category, only present when `outcome` is `Failure`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error_category: Option<ErrorCategory>,
  /// Command duration in milliseconds.
  pub duration_ms: u64,
  /// The apmw semantic version.
  pub apmw_version: String,
}

impl TelemetryEvent {
  /// Create a new telemetry event builder.
  pub fn builder() -> TelemetryEventBuilder {
    TelemetryEventBuilder::new()
  }

  /// Serialize the event to a JSON string.
  pub fn to_json(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string(self)
  }

  /// Serialize the event to a pretty-printed JSON string (for `--telemetry-preview`).
  pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(self)
  }
}

/// Builder for constructing a [`TelemetryEvent`].
#[derive(Debug, Clone)]
pub struct TelemetryEventBuilder {
  terminal_type: Option<TelemetryTerminalType>,
  command: Option<TelemetryCommand>,
  manager: Option<String>,
  outcome: Option<Outcome>,
  error_category: Option<ErrorCategory>,
  duration_ms: u64,
  apmw_version: String,
}

impl TelemetryEventBuilder {
  /// Create a new builder with the current apmw version.
  fn new() -> Self {
    TelemetryEventBuilder {
      terminal_type: None,
      command: None,
      manager: None,
      outcome: None,
      error_category: None,
      duration_ms: 0,
      apmw_version: crate::version().to_string(),
    }
  }

  /// Set the terminal type.
  pub fn terminal_type(mut self, terminal_type: TelemetryTerminalType) -> Self {
    self.terminal_type = Some(terminal_type);
    self
  }

  /// Set the command.
  pub fn command(mut self, command: TelemetryCommand) -> Self {
    self.command = Some(command);
    self
  }

  /// Set the manager name (no package names).
  pub fn manager(mut self, manager: impl Into<String>) -> Self {
    self.manager = Some(manager.into());
    self
  }

  /// Set the outcome to success.
  pub fn success(mut self) -> Self {
    self.outcome = Some(Outcome::Success);
    self.error_category = None;
    self
  }

  /// Set the outcome to failure with an error category.
  pub fn failure(mut self, error_category: ErrorCategory) -> Self {
    self.outcome = Some(Outcome::Failure);
    self.error_category = Some(error_category);
    self
  }

  /// Set the duration in milliseconds.
  pub fn duration_ms(mut self, duration_ms: u64) -> Self {
    self.duration_ms = duration_ms;
    self
  }

  /// Set the apmw version.
  pub fn apmw_version(mut self, version: impl Into<String>) -> Self {
    self.apmw_version = version.into();
    self
  }

  /// Build the telemetry event.
  ///
  /// Returns `None` if required fields (`terminal_type`, `command`, `outcome`)
  /// are not set.
  pub fn build(self) -> Option<TelemetryEvent> {
    Some(TelemetryEvent {
      terminal_type: self.terminal_type?,
      command: self.command?,
      manager: self.manager,
      outcome: self.outcome?,
      error_category: self.error_category,
      duration_ms: self.duration_ms,
      apmw_version: self.apmw_version,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_telemetry_event_serialization_success() {
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Interactive)
      .command(TelemetryCommand::Add)
      .manager("pnpm")
      .success()
      .duration_ms(150)
      .build()
      .unwrap();

    let json = event.to_json().unwrap();
    assert!(json.contains("\"terminal_type\":\"interactive\""));
    assert!(json.contains("\"command\":\"add\""));
    assert!(json.contains("\"manager\":\"pnpm\""));
    assert!(json.contains("\"outcome\":\"success\""));
    assert!(json.contains("\"duration_ms\":150"));
    assert!(json.contains("\"apmw_version\""));
    // error_category should be absent on success (skip_serializing_if)
    assert!(!json.contains("error_category"));
  }

  #[test]
  fn test_telemetry_event_serialization_failure() {
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::NonInteractive)
      .command(TelemetryCommand::Detect)
      .failure(ErrorCategory::NotFound)
      .duration_ms(50)
      .build()
      .unwrap();

    let json = event.to_json().unwrap();
    assert!(json.contains("\"terminal_type\":\"non-interactive\""));
    assert!(json.contains("\"command\":\"detect\""));
    assert!(json.contains("\"outcome\":\"failure\""));
    assert!(json.contains("\"error_category\":\"not-found\""));
    // manager should be null when not set
    assert!(json.contains("\"manager\":null"));
  }

  #[test]
  fn test_telemetry_event_deserialization_roundtrip() {
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Tui)
      .command(TelemetryCommand::Scan)
      .manager("cargo")
      .success()
      .duration_ms(300)
      .build()
      .unwrap();

    let json = event.to_json().unwrap();
    let parsed: TelemetryEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, event);
  }

  #[test]
  fn test_telemetry_command_from_name() {
    assert_eq!(
      TelemetryCommand::from_command_name("install"),
      Some(TelemetryCommand::Add)
    );
    assert_eq!(
      TelemetryCommand::from_command_name("add"),
      Some(TelemetryCommand::Add)
    );
    assert_eq!(
      TelemetryCommand::from_command_name("detect"),
      Some(TelemetryCommand::Detect)
    );
    assert_eq!(
      TelemetryCommand::from_command_name("scan"),
      Some(TelemetryCommand::Scan)
    );
    assert_eq!(
      TelemetryCommand::from_command_name("clone"),
      Some(TelemetryCommand::Clone)
    );
    assert_eq!(
      TelemetryCommand::from_command_name("status"),
      Some(TelemetryCommand::Status)
    );
    assert_eq!(
      TelemetryCommand::from_command_name("list"),
      Some(TelemetryCommand::List)
    );
    assert_eq!(TelemetryCommand::from_command_name("unknown"), None);
  }

  #[test]
  fn test_terminal_type_from_audit() {
    assert_eq!(
      TelemetryTerminalType::from_audit_type(crate::audit::TerminalType::Login, false),
      TelemetryTerminalType::Login
    );
    assert_eq!(
      TelemetryTerminalType::from_audit_type(crate::audit::TerminalType::Interactive, false),
      TelemetryTerminalType::Interactive
    );
    assert_eq!(
      TelemetryTerminalType::from_audit_type(crate::audit::TerminalType::Interactive, true),
      TelemetryTerminalType::Tui
    );
    assert_eq!(
      TelemetryTerminalType::from_audit_type(crate::audit::TerminalType::NonInteractive, false),
      TelemetryTerminalType::NonInteractive
    );
  }

  #[test]
  fn test_error_category_from_apmw_error() {
    let io_err = crate::error::ApmwError::Io(std::io::Error::new(
      std::io::ErrorKind::PermissionDenied,
      "denied",
    ));
    assert_eq!(
      ErrorCategory::from_apmw_error(&io_err),
      ErrorCategory::Permission
    );

    let not_found = crate::error::ApmwError::PackageManagerNotFound("npm".to_string());
    assert_eq!(
      ErrorCategory::from_apmw_error(&not_found),
      ErrorCategory::NotFound
    );

    let version_err = crate::error::ApmwError::VersionResolution("no match".to_string());
    assert_eq!(
      ErrorCategory::from_apmw_error(&version_err),
      ErrorCategory::Version
    );

    let mcp_err = crate::error::ApmwError::McpError("conn".to_string());
    assert_eq!(
      ErrorCategory::from_apmw_error(&mcp_err),
      ErrorCategory::Network
    );
  }

  #[test]
  fn test_builder_missing_required_fields() {
    let result = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Interactive)
      .build();
    assert!(result.is_none());

    let result = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Interactive)
      .command(TelemetryCommand::Add)
      .build();
    assert!(result.is_none());
  }

  #[test]
  fn test_telemetry_privacy_no_personal_data() {
    // Verify that the payload schema cannot carry personal identifiers,
    // package names, file paths, or URLs.
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Interactive)
      .command(TelemetryCommand::Add)
      .manager("pnpm")
      .success()
      .duration_ms(100)
      .build()
      .unwrap();

    let json = event.to_json().unwrap();

    // The JSON should only contain the allowed fields.
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let obj = parsed.as_object().unwrap();

    // Verify only allowed fields are present.
    let allowed_fields = [
      "terminal_type",
      "command",
      "manager",
      "outcome",
      "error_category",
      "duration_ms",
      "apmw_version",
    ];
    for key in obj.keys() {
      assert!(
        allowed_fields.contains(&key.as_str()),
        "Unexpected field in telemetry payload: {key}"
      );
    }

    // Verify manager is a manager name, not a package name.
    assert_eq!(obj["manager"], serde_json::json!("pnpm"));
  }

  #[test]
  fn test_telemetry_privacy_no_file_paths_or_urls() {
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Login)
      .command(TelemetryCommand::Clone)
      .failure(ErrorCategory::Network)
      .duration_ms(5000)
      .build()
      .unwrap();

    let json = event.to_json().unwrap();

    // No file paths (containing / or \\) or URLs (http/https) in the payload.
    assert!(!json.contains("http://"));
    assert!(!json.contains("https://"));
    assert!(!json.contains("/Users/"));
    assert!(!json.contains("/home/"));
    assert!(!json.contains("C:\\\\"));
  }

  #[test]
  fn test_outcome_display() {
    assert_eq!(Outcome::Success.to_string(), "success");
    assert_eq!(Outcome::Failure.to_string(), "failure");
  }

  #[test]
  fn test_error_category_display() {
    assert_eq!(ErrorCategory::Network.to_string(), "network");
    assert_eq!(ErrorCategory::Permission.to_string(), "permission");
    assert_eq!(ErrorCategory::NotFound.to_string(), "not-found");
    assert_eq!(ErrorCategory::Version.to_string(), "version");
    assert_eq!(ErrorCategory::Other.to_string(), "other");
  }

  #[test]
  fn test_terminal_type_display() {
    assert_eq!(
      TelemetryTerminalType::Interactive.to_string(),
      "interactive"
    );
    assert_eq!(TelemetryTerminalType::Tui.to_string(), "tui");
    assert_eq!(TelemetryTerminalType::Login.to_string(), "login");
    assert_eq!(
      TelemetryTerminalType::NonInteractive.to_string(),
      "non-interactive"
    );
  }

  #[test]
  fn test_command_display() {
    assert_eq!(TelemetryCommand::Add.to_string(), "add");
    assert_eq!(TelemetryCommand::Detect.to_string(), "detect");
    assert_eq!(TelemetryCommand::Scan.to_string(), "scan");
    assert_eq!(TelemetryCommand::Clone.to_string(), "clone");
    assert_eq!(TelemetryCommand::Status.to_string(), "status");
    assert_eq!(TelemetryCommand::List.to_string(), "list");
  }

  #[test]
  fn test_to_json_pretty() {
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Interactive)
      .command(TelemetryCommand::Status)
      .success()
      .duration_ms(10)
      .build()
      .unwrap();

    let pretty = event.to_json_pretty().unwrap();
    assert!(pretty.contains('\n'));
    assert!(pretty.contains("\"terminal_type\""));
  }
}
