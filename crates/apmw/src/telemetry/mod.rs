//! Anonymized telemetry collector for apmw (PRD FR-6).
//!
//! Collects ONLY categorical usage data: terminal type, command name, manager
//! name, outcome, error category, duration, and apmw version. No personal
//! identifiers, no package names, no file paths, and no URLs are collected.
//!
//! # Opt-out Mechanisms
//!
//! Telemetry can be disabled via any of:
//! - `--no-telemetry` CLI flag (per-invocation)
//! - `telemetry = false` in config TOML (persistent)
//! - `APMW_NO_TELEMETRY=1` environment variable
//! - `DO_NOT_TRACK=1` environment variable (universal)
//! - `DISABLE_TELEMETRY=1` environment variable (universal)
//!
//! # Non-blocking Send
//!
//! The sender is fire-and-forget: it spawns a tokio task to POST the event
//! and never blocks the main operation. If the endpoint is unreachable, the
//! event is silently dropped.
//!
//! # Preview Mode
//!
//! `--telemetry-preview` prints the payload that would be sent without
//! actually sending it, for transparency.

pub mod event;
pub mod sender;

pub use event::{
  ErrorCategory, Outcome, TelemetryCommand, TelemetryEvent, TelemetryEventBuilder,
  TelemetryTerminalType,
};
pub use sender::{HttpSender, MockSender, NoopSender, TelemetrySender};

use std::sync::Arc;
use std::time::Instant;

use tracing::debug;

use crate::config::ApmwConfig;

/// Environment variable for apmw-specific telemetry opt-out.
pub const ENV_APMW_NO_TELEMETRY: &str = "APMW_NO_TELEMETRY";

/// Universal opt-out environment variable (recognized by many tools).
pub const ENV_DO_NOT_TRACK: &str = "DO_NOT_TRACK";

/// Universal opt-out environment variable.
pub const ENV_DISABLE_TELEMETRY: &str = "DISABLE_TELEMETRY";

/// Check if telemetry is opted out via environment variables.
///
/// Returns `true` if any of the following env vars are set to `1`:
/// - `APMW_NO_TELEMETRY=1`
/// - `DO_NOT_TRACK=1`
/// - `DISABLE_TELEMETRY=1`
pub fn is_env_opt_out() -> bool {
  is_env_set_to_one(ENV_APMW_NO_TELEMETRY)
    || is_env_set_to_one(ENV_DO_NOT_TRACK)
    || is_env_set_to_one(ENV_DISABLE_TELEMETRY)
}

/// Check if an environment variable is set to `1`.
fn is_env_set_to_one(name: &str) -> bool {
  std::env::var_os(name)
    .map(|v| v.to_string_lossy() == "1")
    .unwrap_or(false)
}

/// The telemetry collector.
///
/// Integrates with the config module for the `telemetry` and
/// `telemetry_endpoint` settings, and with CLI flags for per-invocation
/// opt-out and preview mode.
#[derive(Clone)]
pub struct TelemetryCollector {
  /// Whether telemetry is enabled (after resolving all opt-out sources).
  enabled: bool,
  /// The endpoint URL to send events to.
  endpoint: String,
  /// The sender implementation.
  sender: Arc<dyn TelemetrySender>,
}

impl std::fmt::Debug for TelemetryCollector {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TelemetryCollector")
      .field("enabled", &self.enabled)
      .field("endpoint", &self.endpoint)
      .field("sender", &"<dyn TelemetrySender>")
      .finish()
  }
}

impl TelemetryCollector {
  /// Create a new telemetry collector from the resolved config and CLI flags.
  ///
  /// # Arguments
  /// - `config`: The resolved apmw config.
  /// - `no_telemetry`: Whether `--no-telemetry` was passed on the CLI.
  ///
  /// Telemetry is disabled if any of:
  /// - `no_telemetry` is `true` (CLI flag)
  /// - `config.telemetry` is `false` (config file)
  /// - Any opt-out environment variable is set
  pub fn new(config: &ApmwConfig, no_telemetry: bool) -> Self {
    let env_opt_out = is_env_opt_out();
    let enabled = config.telemetry && !no_telemetry && !env_opt_out;

    if !enabled {
      debug!(
        cli_no_telemetry = no_telemetry,
        config_telemetry = config.telemetry,
        env_opt_out,
        "Telemetry disabled"
      );
    }

    let sender: Arc<dyn TelemetrySender> = if enabled {
      Arc::new(HttpSender::new(config.telemetry_endpoint.clone()))
    } else {
      Arc::new(NoopSender)
    };

    TelemetryCollector {
      enabled,
      endpoint: config.telemetry_endpoint.clone(),
      sender,
    }
  }

  /// Create a new telemetry collector with a custom sender (for testing).
  pub fn with_sender(
    config: &ApmwConfig,
    no_telemetry: bool,
    sender: Arc<dyn TelemetrySender>,
  ) -> Self {
    let env_opt_out = is_env_opt_out();
    let enabled = config.telemetry && !no_telemetry && !env_opt_out;

    if !enabled {
      debug!(
        cli_no_telemetry = no_telemetry,
        config_telemetry = config.telemetry,
        env_opt_out,
        "Telemetry disabled"
      );
    }

    TelemetryCollector {
      enabled,
      endpoint: config.telemetry_endpoint.clone(),
      sender,
    }
  }

  /// Returns whether telemetry collection is enabled.
  pub fn is_enabled(&self) -> bool {
    self.enabled
  }

  /// Returns the configured endpoint URL.
  pub fn endpoint(&self) -> &str {
    &self.endpoint
  }

  /// Record a command invocation.
  ///
  /// This is the main entry point for collecting telemetry. It builds an
  /// event from the provided parameters and sends it (if enabled).
  ///
  /// # Arguments
  /// - `command`: The telemetry command name.
  /// - `manager`: The package manager name (if known), never a package name.
  /// - `terminal_type`: The detected terminal type.
  /// - `outcome`: Whether the command succeeded or failed.
  /// - `error_category`: The error category (only on failure).
  /// - `duration`: The duration of the command.
  pub fn record(
    &self,
    command: TelemetryCommand,
    manager: Option<&str>,
    terminal_type: TelemetryTerminalType,
    outcome: Outcome,
    error_category: Option<ErrorCategory>,
    duration: std::time::Duration,
  ) {
    if !self.enabled {
      debug!("Telemetry disabled, skipping event collection");
      return;
    }

    let mut builder = TelemetryEvent::builder()
      .terminal_type(terminal_type)
      .command(command)
      .duration_ms(duration.as_millis() as u64);

    if let Some(m) = manager {
      builder = builder.manager(m);
    }

    builder = match outcome {
      Outcome::Success => builder.success(),
      Outcome::Failure => {
        let category = error_category.unwrap_or(ErrorCategory::Other);
        builder.failure(category)
      }
    };

    if let Some(event) = builder.build() {
      debug!(
        command = %event.command,
        outcome = %event.outcome,
        "Recording telemetry event"
      );
      self.sender.send(&event);
    }
  }

  /// Record a successful command invocation.
  pub fn record_success(
    &self,
    command: TelemetryCommand,
    manager: Option<&str>,
    terminal_type: TelemetryTerminalType,
    duration: std::time::Duration,
  ) {
    self.record(
      command,
      manager,
      terminal_type,
      Outcome::Success,
      None,
      duration,
    );
  }

  /// Record a failed command invocation.
  pub fn record_failure(
    &self,
    command: TelemetryCommand,
    manager: Option<&str>,
    terminal_type: TelemetryTerminalType,
    error_category: ErrorCategory,
    duration: std::time::Duration,
  ) {
    self.record(
      command,
      manager,
      terminal_type,
      Outcome::Failure,
      Some(error_category),
      duration,
    );
  }

  /// Preview the telemetry payload that would be sent for a command.
  ///
  /// Returns the JSON payload as a pretty-printed string. This does NOT send
  /// the event — it is for the `--telemetry-preview` flag.
  pub fn preview(
    &self,
    command: TelemetryCommand,
    manager: Option<&str>,
    terminal_type: TelemetryTerminalType,
    outcome: Outcome,
    error_category: Option<ErrorCategory>,
    duration: std::time::Duration,
  ) -> String {
    let mut builder = TelemetryEvent::builder()
      .terminal_type(terminal_type)
      .command(command)
      .duration_ms(duration.as_millis() as u64);

    if let Some(m) = manager {
      builder = builder.manager(m);
    }

    builder = match outcome {
      Outcome::Success => builder.success(),
      Outcome::Failure => {
        let category = error_category.unwrap_or(ErrorCategory::Other);
        builder.failure(category)
      }
    };

    match builder.build() {
      Some(event) => event.to_json_pretty().unwrap_or_else(|e| {
        format!("{{\"error\": \"Failed to serialize telemetry payload: {e}\"}}")
      }),
      None => "{}".to_string(),
    }
  }
}

/// A timing guard that measures the duration of a command.
///
/// Created with [`Timer::start`], it records the start time and computes
/// the elapsed duration when [`Timer::elapsed`](Timer::elapsed) is called.
#[derive(Debug)]
pub struct Timer {
  start: Instant,
}

impl Timer {
  /// Start a new timer.
  pub fn start() -> Self {
    Timer {
      start: Instant::now(),
    }
  }

  /// Returns the elapsed duration since the timer was started.
  pub fn elapsed(&self) -> std::time::Duration {
    self.start.elapsed()
  }
}

impl Default for Timer {
  fn default() -> Self {
    Self::start()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::telemetry::event::TelemetryTerminalType;

  fn default_config() -> ApmwConfig {
    ApmwConfig::default()
  }

  fn config_with_telemetry(enabled: bool) -> ApmwConfig {
    ApmwConfig {
      telemetry: enabled,
      ..Default::default()
    }
  }

  #[test]
  fn test_collector_enabled_by_default() {
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);
    assert!(collector.is_enabled());
  }

  #[test]
  fn test_collector_disabled_by_cli_flag() {
    let config = default_config();
    let collector = TelemetryCollector::new(&config, true);
    assert!(!collector.is_enabled());
  }

  #[test]
  fn test_collector_disabled_by_config() {
    let config = config_with_telemetry(false);
    let collector = TelemetryCollector::new(&config, false);
    assert!(!collector.is_enabled());
  }

  #[test]
  fn test_collector_cli_flag_overrides_config() {
    let config = config_with_telemetry(true);
    let collector = TelemetryCollector::new(&config, true);
    assert!(!collector.is_enabled());
  }

  #[test]
  fn test_collector_disabled_by_env_apmw_no_telemetry() {
    std::env::set_var("APMW_NO_TELEMETRY", "1");
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);
    assert!(!collector.is_enabled());
    std::env::remove_var("APMW_NO_TELEMETRY");
  }

  #[test]
  fn test_collector_disabled_by_env_do_not_track() {
    std::env::set_var("DO_NOT_TRACK", "1");
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);
    assert!(!collector.is_enabled());
    std::env::remove_var("DO_NOT_TRACK");
  }

  #[test]
  fn test_collector_disabled_by_env_disable_telemetry() {
    std::env::set_var("DISABLE_TELEMETRY", "1");
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);
    assert!(!collector.is_enabled());
    std::env::remove_var("DISABLE_TELEMETRY");
  }

  #[test]
  fn test_collector_env_opt_out_does_not_affect_other_values() {
    // Setting to "0" or "false" should NOT opt out.
    std::env::set_var("APMW_NO_TELEMETRY", "0");
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);
    assert!(collector.is_enabled());
    std::env::remove_var("APMW_NO_TELEMETRY");
  }

  #[test]
  fn test_is_env_opt_out() {
    std::env::remove_var("APMW_NO_TELEMETRY");
    std::env::remove_var("DO_NOT_TRACK");
    std::env::remove_var("DISABLE_TELEMETRY");
    assert!(!is_env_opt_out());

    std::env::set_var("APMW_NO_TELEMETRY", "1");
    assert!(is_env_opt_out());
    std::env::remove_var("APMW_NO_TELEMETRY");

    std::env::set_var("DO_NOT_TRACK", "1");
    assert!(is_env_opt_out());
    std::env::remove_var("DO_NOT_TRACK");

    std::env::set_var("DISABLE_TELEMETRY", "1");
    assert!(is_env_opt_out());
    std::env::remove_var("DISABLE_TELEMETRY");

    assert!(!is_env_opt_out());
  }

  #[test]
  fn test_collector_record_success_uses_mock_sender() {
    let sender = Arc::new(MockSender::new());
    let config = default_config();
    let collector = TelemetryCollector::with_sender(&config, false, sender.clone());

    collector.record_success(
      TelemetryCommand::Add,
      Some("pnpm"),
      TelemetryTerminalType::Interactive,
      std::time::Duration::from_millis(150),
    );

    assert_eq!(sender.event_count(), 1);
    let event = &sender.events()[0];
    assert_eq!(event.command, TelemetryCommand::Add);
    assert_eq!(event.manager.as_deref(), Some("pnpm"));
    assert_eq!(event.outcome, Outcome::Success);
    assert_eq!(event.duration_ms, 150);
  }

  #[test]
  fn test_collector_record_failure_uses_mock_sender() {
    let sender = Arc::new(MockSender::new());
    let config = default_config();
    let collector = TelemetryCollector::with_sender(&config, false, sender.clone());

    collector.record_failure(
      TelemetryCommand::Detect,
      None,
      TelemetryTerminalType::NonInteractive,
      ErrorCategory::NotFound,
      std::time::Duration::from_millis(50),
    );

    assert_eq!(sender.event_count(), 1);
    let event = &sender.events()[0];
    assert_eq!(event.command, TelemetryCommand::Detect);
    assert!(event.manager.is_none());
    assert_eq!(event.outcome, Outcome::Failure);
    assert_eq!(event.error_category, Some(ErrorCategory::NotFound));
  }

  #[test]
  fn test_collector_disabled_does_not_send() {
    let sender = Arc::new(MockSender::new());
    let config = config_with_telemetry(false);
    let collector = TelemetryCollector::with_sender(&config, false, sender.clone());

    collector.record_success(
      TelemetryCommand::Add,
      Some("pnpm"),
      TelemetryTerminalType::Interactive,
      std::time::Duration::from_millis(100),
    );

    assert_eq!(sender.event_count(), 0);
  }

  #[test]
  fn test_collector_disabled_by_cli_does_not_send() {
    let sender = Arc::new(MockSender::new());
    let config = default_config();
    let collector = TelemetryCollector::with_sender(&config, true, sender.clone());

    collector.record_success(
      TelemetryCommand::Add,
      Some("pnpm"),
      TelemetryTerminalType::Interactive,
      std::time::Duration::from_millis(100),
    );

    assert_eq!(sender.event_count(), 0);
  }

  #[test]
  fn test_collector_preview_returns_json() {
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);

    let preview = collector.preview(
      TelemetryCommand::Add,
      Some("pnpm"),
      TelemetryTerminalType::Interactive,
      Outcome::Success,
      None,
      std::time::Duration::from_millis(100),
    );

    assert!(preview.contains("\"terminal_type\""));
    assert!(preview.contains("\"command\""));
    assert!(preview.contains("\"pnpm\""));
    assert!(preview.contains("\"success\""));
    assert!(preview.contains("100"));
  }

  #[test]
  fn test_collector_preview_failure_includes_error_category() {
    let config = default_config();
    let collector = TelemetryCollector::new(&config, false);

    let preview = collector.preview(
      TelemetryCommand::Scan,
      Some("cargo"),
      TelemetryTerminalType::Tui,
      Outcome::Failure,
      Some(ErrorCategory::Network),
      std::time::Duration::from_millis(500),
    );

    assert!(preview.contains("\"failure\""));
    assert!(preview.contains("\"network\""));
  }

  #[test]
  fn test_collector_preview_does_not_send() {
    let sender = Arc::new(MockSender::new());
    let config = default_config();
    let collector = TelemetryCollector::with_sender(&config, false, sender.clone());

    let _preview = collector.preview(
      TelemetryCommand::Add,
      Some("pnpm"),
      TelemetryTerminalType::Interactive,
      Outcome::Success,
      None,
      std::time::Duration::from_millis(100),
    );

    assert_eq!(sender.event_count(), 0);
  }

  #[test]
  fn test_collector_endpoint_configurable() {
    let config = ApmwConfig {
      telemetry_endpoint: "https://custom.endpoint.com/v1/track".to_string(),
      ..Default::default()
    };
    let collector = TelemetryCollector::new(&config, false);
    assert_eq!(collector.endpoint(), "https://custom.endpoint.com/v1/track");
  }

  #[test]
  fn test_timer_measures_duration() {
    let timer = Timer::start();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let elapsed = timer.elapsed();
    assert!(elapsed.as_millis() >= 10);
  }

  #[test]
  fn test_timer_default() {
    let timer = Timer::default();
    let _elapsed = timer.elapsed();
    // Should not panic.
  }

  #[test]
  fn test_collector_record_with_no_manager() {
    let sender = Arc::new(MockSender::new());
    let config = default_config();
    let collector = TelemetryCollector::with_sender(&config, false, sender.clone());

    collector.record_success(
      TelemetryCommand::Status,
      None,
      TelemetryTerminalType::Login,
      std::time::Duration::from_millis(5),
    );

    assert_eq!(sender.event_count(), 1);
    assert!(sender.events()[0].manager.is_none());
  }

  #[test]
  fn test_collector_record_all_commands() {
    let sender = Arc::new(MockSender::new());
    let config = default_config();
    let collector = TelemetryCollector::with_sender(&config, false, sender.clone());

    let commands = [
      TelemetryCommand::Add,
      TelemetryCommand::Detect,
      TelemetryCommand::Scan,
      TelemetryCommand::Clone,
      TelemetryCommand::Status,
      TelemetryCommand::List,
    ];

    for cmd in &commands {
      collector.record_success(
        *cmd,
        Some("pnpm"),
        TelemetryTerminalType::Interactive,
        std::time::Duration::from_millis(10),
      );
    }

    assert_eq!(sender.event_count(), commands.len());
    for (i, cmd) in commands.iter().enumerate() {
      assert_eq!(sender.events()[i].command, *cmd);
    }
  }
}
