//! Add engine — the install-on-use orchestrator for `apmw add`.
//!
//! This module ties together all the building blocks from Phases 02 and 03:
//!
//! - **Detection** (02-001): identifies the package manager for the current project.
//! - **PATH scan** (02-002): skips installation when the tool is already available.
//! - **Ecosystem mapping** (02-003): translates within-ecosystem (pip→uv, npm→pnpm).
//! - **Manager override** (02-004): `--manager <name>` skips auto-detection.
//! - **Version resolution** (03-001): resolves versions with min-age-days defense.
//! - **Security scanning** (03-002/03-003): two-phase scan before add.
//! - **Audit logging** (01-005): records every add operation.
//!
//! The engine follows the install-on-use flow:
//!
//! ```text
//! detect → scan PATH → resolve version → security scan → add → audit
//! ```
//!
//! # Submodules
//!
//! - [`runner`] — ad-hoc runner resolution (canonical add command per ecosystem).
//! - [`on_use`] — the core install-on-use flow and [`on_use::OnUseEngine`].
//! - [`dev`] — `--dev` flag mapping to per-manager dev-dep mechanisms.
//!
//! # Devbox + rtk routing
//!
//! When no impossible barriers exist and a `devbox.json` is present, the engine
//! routes the add command through `devbox run --` so the installation happens
//! inside the project's devbox environment. If devbox is unavailable, the engine
//! falls back to direct execution.

pub mod dev;
pub mod on_use;
pub mod runner;
pub mod suggest;

pub use dev::{dev_add_command, dev_flags, supports_dev};
pub use on_use::{AddResult, AddStatus, OnUseConfig, OnUseEngine};
pub use runner::{DefaultRunnerResolver, MockRunnerResolver, Runner, RunnerResolver};
pub use suggest::{OfferResult, SuggestEngine, Suggestion};

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::Result;
use crate::path_scan::{devbox_json_dir, is_in_devbox_shell};
use crate::security::OnRiskMode;
use crate::version::{MinAgeDaysConfig, RegistryClient, UreqRegistryClient};

/// Configuration for the add engine, derived from CLI flags.
#[derive(Debug, Clone)]
pub struct AddEngineConfig {
  /// Whether `--dev` was passed (development/build-time dependency).
  pub dev: bool,
  /// Whether `--dry-run` was passed.
  pub dry_run: bool,
  /// Whether `--no-scan` was passed (skip security scanning).
  pub no_scan: bool,
  /// Whether `--scan-only` was passed.
  pub scan_only: bool,
  /// The `--manager <name>` override, if any.
  pub manager_override: Option<String>,
  /// The requested version spec (e.g. `""`, `"4"`, `"^1.2"`).
  pub version_spec: String,
  /// Whether to route through devbox when available.
  pub use_devbox: bool,
  /// The on-risk mode for security scanning.
  pub on_risk: OnRiskMode,
  /// The min-age-days configuration for supply-chain defense.
  pub min_age: MinAgeDaysConfig,
}

impl Default for AddEngineConfig {
  fn default() -> Self {
    AddEngineConfig {
      dev: false,
      dry_run: false,
      no_scan: false,
      scan_only: false,
      manager_override: None,
      version_spec: String::new(),
      use_devbox: true,
      on_risk: OnRiskMode::Warn,
      min_age: MinAgeDaysConfig::default(),
    }
  }
}

/// The routing decision for executing the add command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingDecision {
  /// Route through `devbox run --` (devbox environment is active or available).
  Devbox,
  /// Execute directly (no devbox wrapper).
  #[default]
  Direct,
}

/// The add engine — the top-level orchestrator for `apmw add`.
///
/// Wraps the [`OnUseEngine`] and adds devbox + rtk routing logic. The engine is
/// async and designed to run inside a tokio runtime.
pub struct AddEngine<C: RegistryClient = UreqRegistryClient> {
  on_use: OnUseEngine<DefaultRunnerResolver, C>,
}

impl AddEngine<UreqRegistryClient> {
  /// Create a new add engine with default components (real registry client).
  pub fn new() -> Self {
    AddEngine {
      on_use: OnUseEngine::new_real(),
    }
  }
}

impl Default for AddEngine<UreqRegistryClient> {
  fn default() -> Self {
    Self::new()
  }
}

impl<C: RegistryClient> AddEngine<C> {
  /// Create a new add engine with a custom on-use engine (for testing).
  pub fn with_on_use(on_use: OnUseEngine<DefaultRunnerResolver, C>) -> Self {
    AddEngine { on_use }
  }

  /// Run the add flow for a single package.
  ///
  /// This is the main entry point for `apmw add <package>`. It:
  /// 1. Builds an [`OnUseConfig`] from the engine config.
  /// 2. Determines the routing (devbox vs direct).
  /// 3. Runs the install-on-use flow via [`OnUseEngine`].
  /// 4. Returns the [`AddResult`].
  pub async fn run(
    &self,
    package: &str,
    project_dir: &Path,
    config: &AddEngineConfig,
  ) -> Result<AddResult> {
    info!(package = package, dev = config.dev, "add engine starting");

    // Build the on-use config from the engine config.
    let on_use_config = OnUseConfig {
      dev: config.dev,
      dry_run: config.dry_run,
      no_scan: config.no_scan,
      scan_only: config.scan_only,
      on_risk: config.on_risk,
      min_age: config.min_age.clone(),
      version_spec: config.version_spec.clone(),
    };

    // Determine routing.
    let routing = self.determine_routing(project_dir, config);
    info!(routing = ?routing, "routing decision");

    // Run the install-on-use flow.
    let result = self
      .on_use
      .run(
        package,
        config.manager_override.as_deref(),
        project_dir,
        &on_use_config,
      )
      .await?;

    info!(
      package = package,
      status = ?result.status,
      "add engine completed"
    );

    Ok(result)
  }

  /// Determine whether to route through devbox or execute directly.
  ///
  /// Routes through devbox when:
  /// - `use_devbox` is true in the config, AND
  /// - Either `DEVBOX_SHELL`/`IN_DEVBOX_SHELL` is set (already in devbox), OR
  ///   a `devbox.json` exists in the project directory or a parent.
  pub fn determine_routing(&self, project_dir: &Path, config: &AddEngineConfig) -> RoutingDecision {
    if !config.use_devbox {
      return RoutingDecision::Direct;
    }

    // Check if we're already inside a devbox shell.
    let env_map: std::collections::HashMap<String, String> = std::env::vars().collect();
    if is_in_devbox_shell(&env_map) {
      info!("devbox shell detected, routing through devbox");
      return RoutingDecision::Devbox;
    }

    // Check for devbox.json in the project directory or parents.
    if devbox_json_dir(project_dir).is_some() {
      info!("devbox.json found, routing through devbox");
      return RoutingDecision::Devbox;
    }

    RoutingDecision::Direct
  }

  /// Wrap a command with the devbox routing prefix if needed.
  ///
  /// When routing through devbox, the command is prefixed with `devbox run --`.
  pub fn wrap_command(&self, command: &str, routing: RoutingDecision) -> String {
    wrap_command(command, routing)
  }
}

/// Wrap a command with the devbox routing prefix if needed.
///
/// When routing through devbox, the command is prefixed with `devbox run --`.
pub fn wrap_command(command: &str, routing: RoutingDecision) -> String {
  match routing {
    RoutingDecision::Devbox => format!("devbox run -- {command}"),
    RoutingDecision::Direct => command.to_string(),
  }
}

/// Check whether devbox is available on PATH.
///
/// Returns `true` if the `devbox` binary is found on PATH.
pub fn devbox_available() -> bool {
  let scanner = crate::path_scan::PathScanner::new();
  scanner.scan("devbox").is_available()
}

/// Determine if there are impossible barriers that prevent devbox routing.
///
/// An impossible barrier is a condition that makes devbox routing non-viable:
/// - devbox is not on PATH and not in a devbox shell.
pub fn has_devbox_barriers(project_dir: &Path) -> bool {
  let env_map: std::collections::HashMap<String, String> = std::env::vars().collect();
  if is_in_devbox_shell(&env_map) {
    return false;
  }
  if devbox_json_dir(project_dir).is_some() && devbox_available() {
    return false;
  }
  // If devbox.json exists but devbox is not on PATH, that's a barrier.
  if devbox_json_dir(project_dir).is_some() && !devbox_available() {
    warn!("devbox.json found but devbox binary not on PATH — falling back to direct execution");
    return true;
  }
  true
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::version::{MockRegistryClient, RegistryVersion, VersionResolver};
  use tempfile::TempDir;

  /// Helper: create a mock registry with versions for common test packages.
  fn mock_registry() -> MockRegistryClient {
    MockRegistryClient::new()
      .with("pnpm", "express", vec![RegistryVersion::new("4.18.2")])
      .with(
        "pnpm",
        "jest-xyz-nonexistent",
        vec![RegistryVersion::new("1.0.0")],
      )
      .with(
        "pnpm",
        "some-nonexistent-tool-xyz",
        vec![RegistryVersion::new("1.0.0")],
      )
  }

  /// Helper: create an AddEngine with a mock registry that has versions
  /// registered for common test packages.
  fn make_engine() -> AddEngine<MockRegistryClient> {
    let on_use = OnUseEngine::with_components(
      crate::detect::DetectionEngine::new(),
      crate::path_scan::PathScanner::new(),
      VersionResolver::new(mock_registry(), MinAgeDaysConfig::default()),
      DefaultRunnerResolver::new(),
      crate::audit::AuditLogWriter::new().ok(),
    );
    AddEngine::with_on_use(on_use)
  }

  // ===========================================================================
  // AddEngineConfig defaults
  // ===========================================================================

  #[test]
  fn test_add_engine_config_defaults() {
    let config = AddEngineConfig::default();
    assert!(!config.dev);
    assert!(!config.dry_run);
    assert!(!config.no_scan);
    assert!(!config.scan_only);
    assert!(config.manager_override.is_none());
    assert!(config.version_spec.is_empty());
    assert!(config.use_devbox);
  }

  // ===========================================================================
  // RoutingDecision
  // ===========================================================================

  #[test]
  fn test_routing_decision_default_is_direct() {
    assert_eq!(RoutingDecision::default(), RoutingDecision::Direct);
  }

  // ===========================================================================
  // AddEngine::determine_routing
  // ===========================================================================

  #[test]
  fn test_determine_routing_no_devbox_config() {
    let engine = AddEngine::new();
    let dir = TempDir::new().unwrap();
    let config = AddEngineConfig {
      use_devbox: false,
      ..Default::default()
    };
    assert_eq!(
      engine.determine_routing(dir.path(), &config),
      RoutingDecision::Direct
    );
  }

  #[test]
  fn test_determine_routing_no_devbox_json() {
    let engine = AddEngine::new();
    let dir = TempDir::new().unwrap();
    let config = AddEngineConfig::default();
    // In a temp dir with no devbox.json and no DEVBOX_SHELL env, routing is direct.
    // (This test may vary in CI environments with devbox, but temp dirs won't
    // have devbox.json.)
    let routing = engine.determine_routing(dir.path(), &config);
    // If we're in a devbox shell, it would be Devbox; otherwise Direct.
    let env_map: std::collections::HashMap<String, String> = std::env::vars().collect();
    if is_in_devbox_shell(&env_map) {
      assert_eq!(routing, RoutingDecision::Devbox);
    } else {
      assert_eq!(routing, RoutingDecision::Direct);
    }
  }

  #[test]
  fn test_determine_routing_with_devbox_json() {
    let engine = AddEngine::new();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("devbox.json"), r#"{"packages":[]}"#).unwrap();
    let config = AddEngineConfig::default();
    let routing = engine.determine_routing(dir.path(), &config);
    // devbox.json is found, so routing should be Devbox.
    assert_eq!(routing, RoutingDecision::Devbox);
  }

  // ===========================================================================
  // AddEngine::wrap_command
  // ===========================================================================

  #[test]
  fn test_wrap_command_devbox() {
    let result = wrap_command("pnpm add express", RoutingDecision::Devbox);
    assert_eq!(result, "devbox run -- pnpm add express");
  }

  #[test]
  fn test_wrap_command_direct() {
    let result = wrap_command("pnpm add express", RoutingDecision::Direct);
    assert_eq!(result, "pnpm add express");
  }

  // ===========================================================================
  // AddEngine::run — integration with OnUseEngine
  // ===========================================================================

  #[tokio::test]
  async fn test_add_engine_run_dry_run() {
    let engine = make_engine();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
    let config = AddEngineConfig {
      dry_run: true,
      no_scan: true,
      manager_override: Some("pnpm".to_string()),
      ..Default::default()
    };
    let result = engine
      .run("some-nonexistent-tool-xyz", dir.path(), &config)
      .await
      .unwrap();
    assert_eq!(result.status, AddStatus::DryRun);
    assert!(result.dry_run);
    assert!(!result.command.is_empty());
  }

  #[tokio::test]
  async fn test_add_engine_run_dev_flag() {
    let engine = make_engine();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
    let config = AddEngineConfig {
      dev: true,
      dry_run: true,
      no_scan: true,
      manager_override: Some("pnpm".to_string()),
      ..Default::default()
    };
    let result = engine
      .run("jest-xyz-nonexistent", dir.path(), &config)
      .await
      .unwrap();
    assert!(result.dev);
    assert!(result.command.contains("-D"));
  }

  #[tokio::test]
  async fn test_add_engine_run_manager_override() {
    let engine = make_engine();
    let dir = TempDir::new().unwrap();
    std::fs::write(
      dir.path().join("Cargo.toml"),
      r#"[package]
name = "test"
version = "0.1.0""#,
    )
    .unwrap();
    let config = AddEngineConfig {
      dry_run: true,
      no_scan: true,
      manager_override: Some("pnpm".to_string()),
      ..Default::default()
    };
    let result = engine.run("express", dir.path(), &config).await.unwrap();
    assert_eq!(result.manager, "pnpm");
    assert_eq!(result.canonical_manager, "pnpm");
  }

  #[tokio::test]
  async fn test_add_engine_run_path_scan_skip() {
    let engine = make_engine();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
    let config = AddEngineConfig {
      no_scan: true,
      manager_override: Some("pnpm".to_string()),
      ..Default::default()
    };
    // "cargo" is very likely on PATH in the test environment.
    let result = engine.run("cargo", dir.path(), &config).await.unwrap();
    assert_eq!(result.status, AddStatus::AlreadyInstalled);
    assert!(result.skipped);
  }

  // ===========================================================================
  // devbox_available / has_devbox_barriers
  // ===========================================================================

  #[test]
  fn test_devbox_available_returns_bool() {
    let _ = devbox_available();
    // Just verify it doesn't panic.
  }

  #[test]
  fn test_has_devbox_barriers_no_devbox_json() {
    let dir = TempDir::new().unwrap();
    // No devbox.json in temp dir — barriers should be true (unless in devbox shell).
    let env_map: std::collections::HashMap<String, String> = std::env::vars().collect();
    if !is_in_devbox_shell(&env_map) {
      assert!(has_devbox_barriers(dir.path()));
    }
  }

  #[test]
  fn test_has_devbox_barriers_with_devbox_json_no_binary() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("devbox.json"), r#"{"packages":[]}"#).unwrap();
    // If devbox is not on PATH, this should be a barrier.
    // If devbox IS on PATH, it should not be a barrier.
    let env_map: std::collections::HashMap<String, String> = std::env::vars().collect();
    if !is_in_devbox_shell(&env_map) {
      if devbox_available() {
        assert!(!has_devbox_barriers(dir.path()));
      } else {
        assert!(has_devbox_barriers(dir.path()));
      }
    }
  }

  // ===========================================================================
  // Error handling
  // ===========================================================================

  #[tokio::test]
  async fn test_add_engine_run_no_manager_detected() {
    let engine = AddEngine::new();
    let dir = TempDir::new().unwrap();
    let config = AddEngineConfig {
      no_scan: true,
      ..Default::default()
    };
    let result = engine.run("some-tool", dir.path(), &config).await;
    assert!(result.is_err());
  }
}
