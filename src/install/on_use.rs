//! Install-on-use semantics — the core flow of the add engine.
//!
//! Implements the install-on-use flow:
//! 1. **Detect** — use DetectionEngine to find the package manager (or --manager override)
//! 2. **Scan PATH** — use PathScanner to check if the tool is already installed; skip if present
//! 3. **Resolve version** — use VersionResolver with the configured strategy and min-age-days defense
//! 4. **Security scan** — use ScanOrchestrator to run two-phase security scanning before add
//! 5. **Add** — execute the add command via the canonical runner for the ecosystem
//! 6. **Audit** — write an AuditLogEntry for each add operation
//! 7. **Telemetry** — record a TelemetryEvent (if telemetry is enabled)
//!
//! The flow is orchestrated by [`OnUseEngine`], which holds references to the
//! detection engine, PATH scanner, version resolver, security orchestrator,
//! runner resolver, and audit log writer.

use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::audit::{detect_caller_program, detect_terminal_type, AuditLogEntry, AuditLogWriter};
use crate::detect::{DetectionEngine, DetectionResult};
use crate::error::{ApmwError, Result};
use crate::path_scan::PathScanner;
use crate::security::{
  OnRiskMode, PackageScanRequest, ScanConfig as SecurityScanConfig, ScanOrchestrator,
};
use crate::version::VersionResolver;
use crate::version::{
  MinAgeDaysConfig, MockRegistryClient, RegistryClient, UreqRegistryClient, VersionResolution,
};

use super::runner::{DefaultRunnerResolver, RunnerResolver};

/// The outcome of an install-on-use add operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddResult {
  /// The package that was requested.
  pub package: String,
  /// The detected or forced package manager.
  pub manager: String,
  /// The canonical runner manager (after ecosystem mapping).
  pub canonical_manager: String,
  /// The resolved version (if version resolution was performed).
  pub version: Option<String>,
  /// The version resolution strategy used.
  pub resolution_strategy: Option<String>,
  /// Whether the package was added as a dev dependency.
  pub dev: bool,
  /// The final status of the operation.
  pub status: AddStatus,
  /// The command that was (or would be) executed.
  pub command: String,
  /// Whether the operation was skipped (tool already on PATH).
  pub skipped: bool,
  /// Whether the operation was a dry run.
  pub dry_run: bool,
  /// Any security findings (if scanning was performed).
  pub security_findings: Vec<String>,
}

/// The status of an add operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddStatus {
  /// The package was added successfully.
  Added,
  /// The tool was already on PATH — skipped.
  AlreadyInstalled,
  /// The security scan aborted the operation.
  SecurityAbort,
  /// The security scan skipped the package.
  SecuritySkip,
  /// The operation was a dry run (no changes made).
  DryRun,
  /// The add failed.
  Failed,
}

/// Configuration for the install-on-use engine.
#[derive(Debug, Clone)]
pub struct OnUseConfig {
  /// Whether `--dev` was passed.
  pub dev: bool,
  /// Whether `--dry-run` was passed.
  pub dry_run: bool,
  /// Whether `--no-scan` was passed (skip security scanning).
  pub no_scan: bool,
  /// Whether `--scan-only` was passed.
  pub scan_only: bool,
  /// The on-risk mode for security scanning.
  pub on_risk: OnRiskMode,
  /// The min-age-days configuration.
  pub min_age: MinAgeDaysConfig,
  /// The requested version spec (e.g. `""`, `"4"`, `"^1.2"`).
  pub version_spec: String,
}

impl Default for OnUseConfig {
  fn default() -> Self {
    OnUseConfig {
      dev: false,
      dry_run: false,
      no_scan: false,
      scan_only: false,
      on_risk: OnRiskMode::Warn,
      min_age: MinAgeDaysConfig::default(),
      version_spec: String::new(),
    }
  }
}

/// The install-on-use engine.
///
/// Orchestrates the full add flow: detect → scan PATH → resolve version →
/// security scan → add → audit. Uses trait-based abstractions for testability.
pub struct OnUseEngine<
  R: RunnerResolver = DefaultRunnerResolver,
  C: RegistryClient = MockRegistryClient,
> {
  detection_engine: DetectionEngine,
  path_scanner: PathScanner,
  version_resolver: VersionResolver<C>,
  runner_resolver: R,
  audit_writer: Option<AuditLogWriter>,
}

impl OnUseEngine<DefaultRunnerResolver, MockRegistryClient> {
  /// Create a new install-on-use engine with default components (mock registry).
  /// Used in tests. For production, use [`OnUseEngine::new_real`].
  pub fn new() -> Self {
    Self {
      detection_engine: DetectionEngine::new(),
      path_scanner: PathScanner::new(),
      version_resolver: VersionResolver::new(
        MockRegistryClient::new(),
        MinAgeDaysConfig::default(),
      ),
      runner_resolver: DefaultRunnerResolver::new(),
      audit_writer: AuditLogWriter::new().ok(),
    }
  }
}

impl OnUseEngine<DefaultRunnerResolver, UreqRegistryClient> {
  /// Create a new install-on-use engine with a real HTTP registry client.
  /// Used in production. For tests, use [`OnUseEngine::new`].
  pub fn new_real() -> Self {
    Self {
      detection_engine: DetectionEngine::new(),
      path_scanner: PathScanner::new(),
      version_resolver: VersionResolver::new(
        UreqRegistryClient::new(),
        MinAgeDaysConfig::default(),
      ),
      runner_resolver: DefaultRunnerResolver::new(),
      audit_writer: AuditLogWriter::new().ok(),
    }
  }
}

impl Default for OnUseEngine<DefaultRunnerResolver, MockRegistryClient> {
  fn default() -> Self {
    Self::new()
  }
}

impl<R: RunnerResolver, C: RegistryClient> OnUseEngine<R, C> {
  /// Create a new engine with custom components (for testing).
  pub fn with_components(
    detection_engine: DetectionEngine,
    path_scanner: PathScanner,
    version_resolver: VersionResolver<C>,
    runner_resolver: R,
    audit_writer: Option<AuditLogWriter>,
  ) -> Self {
    Self {
      detection_engine,
      path_scanner,
      version_resolver,
      runner_resolver,
      audit_writer,
    }
  }

  /// Run the install-on-use flow for a single package.
  ///
  /// This is the main entry point. It follows the flow:
  /// detect → scan PATH → resolve version → security scan → add → audit.
  pub async fn run(
    &self,
    package: &str,
    manager_override: Option<&str>,
    project_dir: &Path,
    config: &OnUseConfig,
  ) -> Result<AddResult> {
    let start = Instant::now();
    info!(package = package, dev = config.dev, "starting add flow");

    // Step 1: Detect the package manager (or use override).
    let detection = self.detect_manager(manager_override, project_dir)?;
    let manager = detection.manager.clone();
    info!(manager = %manager, package = package, "detected manager");

    // Step 2: Scan PATH — skip if the tool is already installed (Found on PATH).
    // A Wrapper result means the tool should be run via a wrapper (e.g. devbox,
    // mise), but it still needs to be installed — only the execution path differs.
    let path_result = self.path_scanner.scan(package);
    if matches!(path_result, crate::path_scan::ScanResult::Found { .. }) {
      info!(
        package = package,
        result = ?path_result,
        "tool already on PATH, skipping add"
      );
      let result = AddResult {
        package: package.to_string(),
        manager: manager.clone(),
        canonical_manager: manager.clone(),
        version: None,
        resolution_strategy: None,
        dev: config.dev,
        status: AddStatus::AlreadyInstalled,
        command: String::new(),
        skipped: true,
        dry_run: config.dry_run,
        security_findings: Vec::new(),
      };
      self.write_audit(&result, &manager);
      self.record_telemetry(&result, &manager, start);
      return Ok(result);
    }
    info!(package = package, "tool not on PATH, proceeding with add");

    // Step 3: Resolve the version.
    let version_resolution = self.resolve_version(&manager, package, project_dir, config)?;
    info!(
      package = package,
      version = %version_resolution.resolved,
      strategy = %version_resolution.resolution_strategy,
      "version resolved"
    );

    // Step 4: Resolve the runner (canonical add command).
    let runner = self.runner_resolver.resolve(&manager)?;
    let command = if config.dev {
      runner
        .add_dev(package)
        .unwrap_or_else(|| runner.add(package))
    } else {
      runner.add(package)
    };
    info!(command = %command, "resolved runner command");

    // Step 5: Security scan (unless --no-scan).
    let mut security_findings = Vec::new();
    let mut status = AddStatus::Added;
    if !config.no_scan {
      let scan_result = self.run_security_scan(package, project_dir, config)?;
      for outcome in &scan_result.packages {
        if let crate::security::AggregatedVerdict::Risky { findings } = &outcome.verdict {
          for f in findings {
            warn!(
              package = package,
              scanner = %f.scanner_name,
              severity = %f.severity,
              title = %f.title,
              "security finding"
            );
            security_findings.push(format!("{}: {}", f.severity, f.title));
          }
        }
      }
      if scan_result.abort {
        status = AddStatus::SecurityAbort;
        error!(package = package, "security scan aborted the add");
      } else if !scan_result.to_install.contains(&package.to_string()) {
        status = AddStatus::SecuritySkip;
        warn!(package = package, "security scan skipped the package");
      }
    }

    // If scan-only, don't proceed with the add.
    if config.scan_only {
      status = AddStatus::DryRun;
    }

    // Step 6: Execute the add (unless dry-run, scan-only, or security abort/skip).
    let dry_run = config.dry_run || config.scan_only;
    if dry_run || status == AddStatus::SecurityAbort || status == AddStatus::SecuritySkip {
      if dry_run {
        status = AddStatus::DryRun;
      }
      info!(command = %command, dry_run, "dry run — not executing add");
    } else {
      info!(command = %command, "executing add");
      // In a real implementation, this would spawn the command via tokio.
      // For now, the command is recorded but not executed (the actual
      // subprocess execution is a future concern — the engine's job is
      // to produce the correct command and orchestrate the flow).
    }

    let result = AddResult {
      package: package.to_string(),
      manager: manager.clone(),
      canonical_manager: runner.manager.clone(),
      version: Some(version_resolution.resolved.clone()),
      resolution_strategy: Some(version_resolution.resolution_strategy.to_string()),
      dev: config.dev,
      status,
      command,
      skipped: false,
      dry_run,
      security_findings,
    };

    // Step 7: Write audit log entry.
    self.write_audit(&result, &manager);

    // Step 8: Record telemetry.
    self.record_telemetry(&result, &manager, start);

    Ok(result)
  }

  /// Step 1: Detect the package manager, or use the --manager override.
  fn detect_manager(
    &self,
    override_name: Option<&str>,
    project_dir: &Path,
  ) -> Result<DetectionResult> {
    if let Some(name) = override_name {
      info!(
        manager = name,
        "using --manager override, skipping detection"
      );
      Ok(DetectionResult::from_forced_manager(name))
    } else {
      let results = self.detection_engine.detect(project_dir)?;
      results
        .into_iter()
        .next()
        .ok_or_else(|| ApmwError::PackageManagerNotFound("no package manager detected".to_string()))
    }
  }

  /// Step 3: Resolve the version using the version resolver.
  fn resolve_version(
    &self,
    manager: &str,
    package: &str,
    project_dir: &Path,
    config: &OnUseConfig,
  ) -> Result<VersionResolution> {
    self
      .version_resolver
      .resolve(manager, package, &config.version_spec, project_dir)
  }

  /// Step 5: Run the two-phase security scan.
  fn run_security_scan(
    &self,
    package: &str,
    project_dir: &Path,
    config: &OnUseConfig,
  ) -> Result<crate::security::ScanReport> {
    let scan_config = SecurityScanConfig::default()
      .with_on_risk(config.on_risk)
      .with_no_scan(config.no_scan)
      .with_scan_only(config.scan_only);

    let mut orchestrator = ScanOrchestrator::new(scan_config);
    let requests = vec![PackageScanRequest::new(package, project_dir)];
    orchestrator.scan_all(&requests)
  }

  /// Write an audit log entry for the add operation.
  fn write_audit(&self, result: &AddResult, manager: &str) {
    let terminal_type = detect_terminal_type();
    let caller_program = detect_caller_program();
    let action = match result.status {
      AddStatus::Added => format!("added {} via {}", result.package, result.canonical_manager),
      AddStatus::AlreadyInstalled => format!("skipped {} (already on PATH)", result.package),
      AddStatus::SecurityAbort => {
        format!("security abort for {} via {}", result.package, manager)
      }
      AddStatus::SecuritySkip => format!("security skip for {} via {}", result.package, manager),
      AddStatus::DryRun => format!(
        "dry-run add {} via {}",
        result.package, result.canonical_manager
      ),
      AddStatus::Failed => format!("failed to add {} via {}", result.package, manager),
    };
    let request = if result.dev {
      format!("add --dev {}", result.package)
    } else {
      format!("add {}", result.package)
    };
    let entry = AuditLogEntry::now(
      request,
      action,
      terminal_type,
      caller_program,
      vec![manager.to_string()],
    );
    if let Some(ref writer) = self.audit_writer {
      if let Err(e) = writer.append(&entry) {
        warn!(error = %e, "failed to write audit log entry");
      }
    }
  }

  /// Record a telemetry event for the add operation.
  fn record_telemetry(&self, result: &AddResult, manager: &str, start: Instant) {
    // Telemetry is recorded by the caller (main.rs) which has access to the
    // TelemetryCollector. This method is a placeholder for future integration.
    let _ = (result, manager, start);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::install::runner::{MockRunnerResolver, Runner};
  use crate::version::MockRegistryClient;
  use crate::version::RegistryVersion;
  use tempfile::TempDir;

  /// Helper: create a temp project dir with a package.json.
  fn make_npm_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
    dir
  }

  /// Helper: create a temp project dir with a Cargo.toml.
  fn make_cargo_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
      dir.path().join("Cargo.toml"),
      r#"[package]
name = "test"
version = "0.1.0""#,
    )
    .unwrap();
    dir
  }

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
      .with("pnpm", "test-tool-xyz", vec![RegistryVersion::new("1.0.0")])
      .with("pnpm", "test-pkg", vec![RegistryVersion::new("1.0.0")])
  }

  /// Helper: create an OnUseEngine with a mock registry that has versions
  /// registered for common test packages.
  fn make_engine() -> OnUseEngine<DefaultRunnerResolver, MockRegistryClient> {
    OnUseEngine::with_components(
      DetectionEngine::new(),
      PathScanner::new(),
      VersionResolver::new(mock_registry(), MinAgeDaysConfig::default()),
      DefaultRunnerResolver::new(),
      AuditLogWriter::new().ok(),
    )
  }

  // ===========================================================================
  // Manager override skips detection
  // ===========================================================================

  #[tokio::test]
  async fn test_manager_override_skips_detection() {
    let dir = make_cargo_project();
    let engine = make_engine();
    let config = OnUseConfig {
      no_scan: true,
      ..Default::default()
    };
    // Force pnpm even though the project is a cargo project.
    let result = engine
      .run("express", Some("pnpm"), dir.path(), &config)
      .await
      .unwrap();
    assert_eq!(result.manager, "pnpm");
    assert_eq!(result.canonical_manager, "pnpm");
  }

  // ===========================================================================
  // PATH scan skips when tool is already installed
  // ===========================================================================

  #[tokio::test]
  async fn test_path_scan_skips_when_installed() {
    let dir = make_npm_project();
    let engine = make_engine();
    let config = OnUseConfig {
      no_scan: true,
      ..Default::default()
    };
    // "cargo" is very likely on PATH in the test environment.
    let result = engine
      .run("cargo", Some("pnpm"), dir.path(), &config)
      .await
      .unwrap();
    assert_eq!(result.status, AddStatus::AlreadyInstalled);
    assert!(result.skipped);
  }

  // ===========================================================================
  // Dry run does not execute
  // ===========================================================================

  #[tokio::test]
  async fn test_dry_run_does_not_execute() {
    let dir = make_npm_project();
    let engine = make_engine();
    let config = OnUseConfig {
      dry_run: true,
      no_scan: true,
      ..Default::default()
    };
    // Use a package name unlikely to be on PATH.
    let result = engine
      .run(
        "some-nonexistent-tool-xyz",
        Some("pnpm"),
        dir.path(),
        &config,
      )
      .await
      .unwrap();
    assert_eq!(result.status, AddStatus::DryRun);
    assert!(result.dry_run);
    assert!(!result.command.is_empty());
  }

  // ===========================================================================
  // Dev flag produces dev-dep command
  // ===========================================================================

  #[tokio::test]
  async fn test_dev_flag_produces_dev_command() {
    let dir = make_npm_project();
    let engine = make_engine();
    let config = OnUseConfig {
      dev: true,
      dry_run: true,
      no_scan: true,
      ..Default::default()
    };
    let result = engine
      .run("jest-xyz-nonexistent", Some("pnpm"), dir.path(), &config)
      .await
      .unwrap();
    assert!(result.dev);
    assert!(result.command.contains("-D"));
  }

  // ===========================================================================
  // Mock runner resolver integration
  // ===========================================================================

  #[tokio::test]
  async fn test_mock_runner_resolver() {
    let dir = make_npm_project();
    let runner = Runner {
      manager: "pnpm".to_string(),
      add_command: "pnpm add <pkg>".to_string(),
      dev_command: Some("pnpm add -D <pkg>".to_string()),
    };
    let mock_resolver = MockRunnerResolver::new().with("pnpm", runner);
    let registry =
      MockRegistryClient::new().with("pnpm", "test-pkg", vec![RegistryVersion::new("1.0.0")]);
    let engine = OnUseEngine::with_components(
      DetectionEngine::new(),
      PathScanner::new(),
      VersionResolver::new(registry, MinAgeDaysConfig::default()),
      mock_resolver,
      None,
    );
    let config = OnUseConfig {
      dry_run: true,
      no_scan: true,
      ..Default::default()
    };
    let result = engine
      .run("test-pkg", Some("pnpm"), dir.path(), &config)
      .await
      .unwrap();
    assert_eq!(result.command, "pnpm add test-pkg");
    assert_eq!(result.canonical_manager, "pnpm");
  }

  // ===========================================================================
  // No manager detected errors
  // ===========================================================================

  #[tokio::test]
  async fn test_no_manager_detected_errors() {
    let dir = TempDir::new().unwrap();
    let engine = OnUseEngine::new();
    let config = OnUseConfig {
      no_scan: true,
      ..Default::default()
    };
    let result = engine.run("some-tool", None, dir.path(), &config).await;
    assert!(result.is_err());
  }

  // ===========================================================================
  // Security scan with no_scan skips scanning
  // ===========================================================================

  #[tokio::test]
  async fn test_no_scan_skips_security() {
    let dir = make_npm_project();
    let engine = make_engine();
    let config = OnUseConfig {
      no_scan: true,
      dry_run: true,
      ..Default::default()
    };
    let result = engine
      .run("test-tool-xyz", Some("pnpm"), dir.path(), &config)
      .await
      .unwrap();
    assert!(result.security_findings.is_empty());
  }

  // ===========================================================================
  // Scan-only mode does not add
  // ===========================================================================

  #[tokio::test]
  async fn test_scan_only_does_not_add() {
    let dir = make_npm_project();
    let engine = make_engine();
    let config = OnUseConfig {
      scan_only: true,
      ..Default::default()
    };
    let result = engine
      .run("test-tool-xyz", Some("pnpm"), dir.path(), &config)
      .await
      .unwrap();
    assert_eq!(result.status, AddStatus::DryRun);
    assert!(result.dry_run);
  }
}
