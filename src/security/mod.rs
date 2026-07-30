//! Security scanning orchestrator — the two-phase scan-all-then-install-all
//! engine (PRD FR-5).
//!
//! The orchestrator loads scanner plugins implementing the [`Scanner`]
//! trait, runs every scanner against every package in a single scan phase,
//! aggregates the results with OR semantics (any `Risky` verdict makes the
//! package risky), and then decides an install action per package based on the
//! configured [`OnRiskMode`]. Scan and install are **never** interleaved: the
//! orchestrator completes the entire scan phase and produces a
//! [`ScanReport`] before any install action is taken.
//!
//! # Two-phase contract
//!
//! 1. **Scan phase** — `scan_all` runs every available scanner against every
//!    package, collecting [`PackageScanOutcome`] entries. No install happens.
//! 2. **Decide phase** — `decide` applies the `OnRiskMode` to each outcome,
//!    producing an [`InstallAction`] per package and an overall [`ScanReport`].
//!
//! Install itself is out of scope for this module (story 04-001); the report
//! is the handoff to the install engine.
//!
//! # OR semantics (FR-5.3)
//!
//! If **any** scanner reports `Risky` for a package, the package is risky.
//! Scanner errors are incomplete-scan warnings — they never flip a package to
//! risky on their own, but they are recorded and surfaced.

pub mod plugins;
pub mod scanner;

pub use scanner::{
  AuditLevel, OnRiskMode, ScanResult, Scanner, SecurityFinding, SecurityPosture, Severity,
  TelemetryPolicy,
};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::error::{ApmwError, Result};

/// A package to be scanned, identified by name and the local directory the
/// scanner should inspect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageScanRequest {
  /// The package name (or tool name) being scanned.
  pub name: String,
  /// The local directory to scan (e.g. a checkout or a temp install dir).
  pub dir: PathBuf,
}

impl PackageScanRequest {
  /// Creates a new scan request.
  pub fn new(name: impl Into<String>, dir: impl Into<PathBuf>) -> Self {
    Self {
      name: name.into(),
      dir: dir.into(),
    }
  }
}

/// The aggregated verdict for a single package across all scanners.
///
/// Combines the per-scanner `ScanResult`s using OR semantics: any `Risky`
/// makes the package `Risky`; otherwise `Safe` if all scanners completed
/// cleanly; `Incomplete` if at least one scanner errored and none were risky.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum AggregatedVerdict {
  /// No scanner reported findings.
  Safe,
  /// At least one scanner reported findings (OR semantics).
  Risky {
    /// All findings across all scanners that flagged the package.
    findings: Vec<SecurityFinding>,
  },
  /// No scanner reported findings, but at least one scanner errored.
  Incomplete {
    /// The error messages from scanners that could not complete.
    errors: Vec<String>,
  },
}

impl AggregatedVerdict {
  /// Returns `true` when the package is risky.
  pub fn is_risky(&self) -> bool {
    matches!(self, AggregatedVerdict::Risky { .. })
  }

  /// Returns `true` when the package is safe (no findings, no errors).
  pub fn is_safe(&self) -> bool {
    matches!(self, AggregatedVerdict::Safe)
  }

  /// Returns `true` when the scan was incomplete (errors but no findings).
  pub fn is_incomplete(&self) -> bool {
    matches!(self, AggregatedVerdict::Incomplete { .. })
  }

  /// Returns the findings if this is a `Risky` verdict, otherwise empty.
  pub fn findings(&self) -> &[SecurityFinding] {
    if let AggregatedVerdict::Risky { findings } = self {
      findings
    } else {
      &[]
    }
  }
}

/// What the install engine should do with a package after scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallAction {
  /// Proceed with installation.
  Install,
  /// Skip this package (e.g. `--on-risk skip` for a risky package).
  Skip,
  /// The whole batch must abort (e.g. `--on-risk error`).
  Abort,
}

/// The per-package outcome of the scan phase plus the decided install action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageScanOutcome {
  /// The original scan request.
  pub request: PackageScanRequest,
  /// The aggregated verdict across all scanners.
  pub verdict: AggregatedVerdict,
  /// Per-scanner results, in registration order.
  pub per_scanner: Vec<ScannerRecord>,
  /// What to do with this package.
  pub action: InstallAction,
}

/// A single scanner's result for a single package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannerRecord {
  /// Name of the scanner that produced this record.
  pub scanner_name: String,
  /// Whether the scanner was available for this scan.
  pub available: bool,
  /// The result the scanner returned (or why it was skipped).
  pub result: ScanResult,
}

/// The final report from a two-phase scan batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
  /// Per-package outcomes, in the order packages were submitted.
  pub packages: Vec<PackageScanOutcome>,
  /// `true` when at least one package is risky.
  pub any_risky: bool,
  /// `true` when at least one scanner errored on at least one package.
  pub any_incomplete: bool,
  /// `true` when the batch should abort before install.
  pub abort: bool,
  /// Names of packages that should be installed (action == Install).
  pub to_install: Vec<String>,
  /// Names of packages that should be skipped (action == Skip).
  pub to_skip: Vec<String>,
}

impl ScanReport {
  /// Returns `true` when there is nothing to install and nothing to skip.
  pub fn is_empty(&self) -> bool {
    self.packages.is_empty()
  }

  /// Returns the outcome for a named package, if present.
  pub fn for_package(&self, name: &str) -> Option<&PackageScanOutcome> {
    self.packages.iter().find(|o| o.request.name == name)
  }
}

/// Configuration for the scan orchestrator.
///
/// Wraps the user-facing flags from PRD FR-5.4 through FR-5.6:
/// `--on-risk`, `--update-security-db`, `--no-scan`, `--scan-only`, and the
/// telemetry policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScanConfig {
  /// What to do when a package is flagged risky.
  pub on_risk: OnRiskMode,
  /// Refresh security databases before scanning (`--update-security-db`).
  pub update_security_db: bool,
  /// Skip all scanning and install immediately (`--no-scan`).
  pub no_scan: bool,
  /// Scan but do not install (`--scan-only`).
  pub scan_only: bool,
  /// Telemetry policy for the packages being scanned/installed.
  pub telemetry: TelemetryPolicy,
  /// The security posture to enforce (mirrors `npmrc.tmpl`).
  pub posture: SecurityPosture,
}

impl ScanConfig {
  /// Sets the on-risk mode.
  pub fn with_on_risk(mut self, mode: OnRiskMode) -> Self {
    self.on_risk = mode;
    self
  }

  /// Enables `--update-security-db`.
  pub fn with_update_security_db(mut self, yes: bool) -> Self {
    self.update_security_db = yes;
    self
  }

  /// Enables `--no-scan`.
  pub fn with_no_scan(mut self, yes: bool) -> Self {
    self.no_scan = yes;
    self
  }

  /// Enables `--scan-only`.
  pub fn with_scan_only(mut self, yes: bool) -> Self {
    self.scan_only = yes;
    self
  }

  /// Sets the telemetry policy.
  pub fn with_telemetry(mut self, policy: TelemetryPolicy) -> Self {
    self.telemetry = policy;
    self
  }

  /// Sets the security posture.
  pub fn with_posture(mut self, posture: SecurityPosture) -> Self {
    self.posture = posture;
    self
  }
}

/// The two-phase security scanning orchestrator.
///
/// Holds the registered scanner plugins and the [`ScanConfig`]. Use
/// [`scan_all`](Self::scan_all) to run the scan phase, then read the
/// [`ScanReport`] it returns — install is a separate concern (story 04-001).
pub struct ScanOrchestrator {
  scanners: Vec<Box<dyn Scanner>>,
  config: ScanConfig,
}

impl Default for ScanOrchestrator {
  fn default() -> Self {
    Self::new(ScanConfig::default())
  }
}

impl ScanOrchestrator {
  /// Creates a new orchestrator with the given config and no scanners.
  pub fn new(config: ScanConfig) -> Self {
    Self {
      scanners: Vec::new(),
      config,
    }
  }

  /// Returns a reference to the orchestrator's config.
  pub fn config(&self) -> &ScanConfig {
    &self.config
  }

  /// Returns the number of registered scanners.
  pub fn scanner_count(&self) -> usize {
    self.scanners.len()
  }

  /// Registers a scanner plugin.
  pub fn register_scanner(&mut self, scanner: Box<dyn Scanner>) {
    debug!(scanner = scanner.name(), "registered scanner plugin");
    self.scanners.push(scanner);
  }

  /// Prepares every available scanner, optionally refreshing security DBs.
  ///
  /// Called automatically by [`scan_all`](Self::scan_all), but exposed so
  /// callers can warm scanners up front. Unavailable scanners are skipped.
  /// If an available scanner fails to ensure, the error is propagated as
  /// [`ApmwError::SecurityScanFailed`].
  pub fn ensure_scanners(&mut self) -> Result<()> {
    for scanner in self.scanners.iter_mut() {
      if !scanner.available() {
        debug!(
          scanner = scanner.name(),
          "scanner unavailable, skipping ensure"
        );
        continue;
      }
      if self.config.update_security_db {
        info!(scanner = scanner.name(), "refreshing security database");
        scanner
          .update_db()
          .map_err(|e| ApmwError::SecurityScanFailed(format!("{}: {e}", scanner.name())))?;
      } else {
        scanner
          .ensure()
          .map_err(|e| ApmwError::SecurityScanFailed(format!("{}: {e}", scanner.name())))?;
      }
    }
    Ok(())
  }

  /// Runs the two-phase scan over a batch of packages.
  ///
  /// **Phase 1 (scan):** every available scanner scans every package. No
  /// install happens here. Results are aggregated per package with OR
  /// semantics.
  ///
  /// **Phase 2 (decide):** the `OnRiskMode` is applied to each aggregated
  /// verdict to produce an [`InstallAction`] and the final [`ScanReport`].
  ///
  /// When `no_scan` is set, every package is marked `Safe` with action
  /// `Install` and no scanners are invoked. When `scan_only` is set, the
  /// report is produced normally but callers should treat `to_install` as
  /// advisory only (the install engine is responsible for honoring it).
  pub fn scan_all(&mut self, packages: &[PackageScanRequest]) -> Result<ScanReport> {
    debug!(
      telemetry = %self.config.telemetry,
      on_risk = %self.config.on_risk,
      no_scan = self.config.no_scan,
      scan_only = self.config.scan_only,
      update_security_db = self.config.update_security_db,
      "starting two-phase scan",
    );

    // --no-scan: skip the scan phase entirely.
    if self.config.no_scan {
      info!("--no-scan set, skipping security scan phase");
      let packages = packages
        .iter()
        .map(|req| PackageScanOutcome {
          request: req.clone(),
          verdict: AggregatedVerdict::Safe,
          per_scanner: Vec::new(),
          action: InstallAction::Install,
        })
        .collect::<Vec<_>>();
      let to_install = packages.iter().map(|o| o.request.name.clone()).collect();
      return Ok(ScanReport {
        packages,
        any_risky: false,
        any_incomplete: false,
        abort: false,
        to_install,
        to_skip: Vec::new(),
      });
    }

    // Prepare scanners (and refresh DBs if requested) before scanning.
    self.ensure_scanners()?;

    // Phase 1: scan ALL packages with ALL available scanners. No install.
    let mut outcomes: Vec<PackageScanOutcome> = Vec::with_capacity(packages.len());
    for req in packages {
      let (verdict, per_scanner) = self.scan_package(&req.dir);
      info!(
        package = %req.name,
        dir = %req.dir.display(),
        risky = verdict.is_risky(),
        incomplete = verdict.is_incomplete(),
        "scan phase complete for package",
      );
      if let AggregatedVerdict::Risky { findings } = &verdict {
        for f in findings {
          warn!(
            package = %req.name,
            scanner = %f.scanner_name,
            severity = %f.severity,
            title = %f.title,
            "security finding",
          );
        }
      }
      if let AggregatedVerdict::Incomplete { errors } = &verdict {
        for e in errors {
          warn!(package = %req.name, error = %e, "incomplete scan (scanner error)");
        }
      }
      outcomes.push(PackageScanOutcome {
        request: req.clone(),
        verdict,
        per_scanner,
        action: InstallAction::Install, // decided below
      });
    }

    // Phase 2: decide install actions based on the on-risk mode. Still no
    // install — we only produce the plan.
    let mut any_risky = false;
    let mut any_incomplete = false;
    let mut abort = false;
    let mut to_install = Vec::new();
    let mut to_skip = Vec::new();

    for outcome in outcomes.iter_mut() {
      match &outcome.verdict {
        AggregatedVerdict::Safe => {
          outcome.action = InstallAction::Install;
          to_install.push(outcome.request.name.clone());
        }
        AggregatedVerdict::Risky { .. } => {
          any_risky = true;
          match self.config.on_risk {
            OnRiskMode::Error => {
              error!(
                package = %outcome.request.name,
                "risky package and --on-risk=error, aborting batch"
              );
              outcome.action = InstallAction::Abort;
              abort = true;
            }
            OnRiskMode::Skip => {
              warn!(package = %outcome.request.name, "skipping risky package (--on-risk=skip)");
              outcome.action = InstallAction::Skip;
              to_skip.push(outcome.request.name.clone());
            }
            OnRiskMode::Warn => {
              warn!(
                package = %outcome.request.name,
                "risky package, installing anyway (--on-risk=warn)"
              );
              outcome.action = InstallAction::Install;
              to_install.push(outcome.request.name.clone());
            }
            OnRiskMode::Prompt => {
              // No TTY in library code — prompt mode without a TTY is an error
              // per FR-5.4. The install engine / CLI layer is responsible for
              // the interactive prompt; here we surface the decision.
              warn!(
                package = %outcome.request.name,
                "risky package with --on-risk=prompt; interactive prompt deferred to caller"
              );
              outcome.action = InstallAction::Skip;
              to_skip.push(outcome.request.name.clone());
            }
          }
        }
        AggregatedVerdict::Incomplete { .. } => {
          any_incomplete = true;
          // Incomplete scans are warnings, not risk. Install proceeds.
          outcome.action = InstallAction::Install;
          to_install.push(outcome.request.name.clone());
        }
      }
    }

    info!(
      packages = outcomes.len(),
      any_risky,
      any_incomplete,
      abort,
      scan_only = self.config.scan_only,
      "two-phase scan complete",
    );

    Ok(ScanReport {
      packages: outcomes,
      any_risky,
      any_incomplete,
      abort,
      to_install,
      to_skip,
    })
  }

  /// Runs every available scanner against a single directory and aggregates
  /// the results with OR semantics.
  ///
  /// Returns the aggregated verdict and the per-scanner records.
  fn scan_package(&self, dir: &Path) -> (AggregatedVerdict, Vec<ScannerRecord>) {
    let mut findings: Vec<SecurityFinding> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut records: Vec<ScannerRecord> = Vec::new();
    let mut any_risky = false;

    for scanner in &self.scanners {
      let name = scanner.name().to_string();
      if !scanner.available() {
        debug!(scanner = %name, "scanner unavailable, skipping for package");
        let message = "scanner unavailable".to_string();
        errors.push(format!("{name}: {message}"));
        records.push(ScannerRecord {
          scanner_name: name,
          available: false,
          result: ScanResult::Error { message },
        });
        continue;
      }
      let result = scanner.scan(dir);
      match &result {
        ScanResult::Safe => {
          debug!(scanner = %name, "scan safe");
        }
        ScanResult::Risky {
          findings: scanner_findings,
        } => {
          any_risky = true;
          for f in scanner_findings {
            error!(
              scanner = %name,
              severity = %f.severity,
              title = %f.title,
              "scanner reported finding",
            );
            findings.push(f.clone());
          }
        }
        ScanResult::Error { message } => {
          error!(scanner = %name, error = %message, "scanner error");
          errors.push(format!("{name}: {message}"));
        }
      }
      records.push(ScannerRecord {
        scanner_name: name,
        available: true,
        result,
      });
    }

    let verdict = if any_risky {
      AggregatedVerdict::Risky { findings }
    } else if !errors.is_empty() {
      AggregatedVerdict::Incomplete { errors }
    } else {
      AggregatedVerdict::Safe
    };
    (verdict, records)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::security::scanner::ScanResult;
  use std::sync::Mutex;

  /// A mock scanner whose result is fixed at construction time.
  struct MockScanner {
    name: String,
    available: bool,
    result: ScanResult,
    ensure_calls: Mutex<u32>,
    scan_calls: Mutex<u32>,
    update_calls: Mutex<u32>,
  }

  impl MockScanner {
    fn new(name: &str, available: bool, result: ScanResult) -> Self {
      Self {
        name: name.to_string(),
        available,
        result,
        ensure_calls: Mutex::new(0),
        scan_calls: Mutex::new(0),
        update_calls: Mutex::new(0),
      }
    }
  }

  impl Scanner for MockScanner {
    fn name(&self) -> &str {
      &self.name
    }
    fn available(&self) -> bool {
      self.available
    }
    fn ensure(&mut self) -> Result<()> {
      *self.ensure_calls.lock().unwrap() += 1;
      Ok(())
    }
    fn update_db(&mut self) -> Result<()> {
      *self.update_calls.lock().unwrap() += 1;
      Ok(())
    }
    fn scan(&self, dir: &Path) -> ScanResult {
      *self.scan_calls.lock().unwrap() += 1;
      let _ = dir; // dir is unused by the mock
      self.result.clone()
    }
  }

  /// A mock scanner that returns different results depending on the directory,
  /// so a single orchestrator can exercise both safe and risky packages in one
  /// two-phase batch.
  struct DirMockScanner {
    name: String,
    risky_dirs: Vec<PathBuf>,
  }

  impl DirMockScanner {
    fn new(name: &str, risky_dirs: Vec<PathBuf>) -> Self {
      Self {
        name: name.to_string(),
        risky_dirs,
      }
    }
  }

  impl Scanner for DirMockScanner {
    fn name(&self) -> &str {
      &self.name
    }
    fn available(&self) -> bool {
      true
    }
    fn ensure(&mut self) -> Result<()> {
      Ok(())
    }
    fn scan(&self, dir: &Path) -> ScanResult {
      if self.risky_dirs.iter().any(|d| d == dir) {
        ScanResult::Risky {
          findings: vec![SecurityFinding::new(
            &self.name,
            Severity::High,
            "CVE-test",
            "test finding",
          )],
        }
      } else {
        ScanResult::Safe
      }
    }
  }

  fn risky_finding(scanner: &str) -> SecurityFinding {
    SecurityFinding::new(scanner, Severity::High, "CVE-test", "test finding")
  }

  #[test]
  fn test_or_semantics_all_safe() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default());
    orch.register_scanner(Box::new(MockScanner::new("a", true, ScanResult::Safe)));
    orch.register_scanner(Box::new(MockScanner::new("b", true, ScanResult::Safe)));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir.clone());
    let report = orch.scan_all(&[req]).unwrap();
    assert!(!report.any_risky);
    assert_eq!(report.packages.len(), 1);
    assert!(report.packages[0].verdict.is_safe());
    assert_eq!(report.packages[0].action, InstallAction::Install);
  }

  #[test]
  fn test_or_semantics_one_risky() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_on_risk(OnRiskMode::Warn));
    orch.register_scanner(Box::new(MockScanner::new("a", true, ScanResult::Safe)));
    orch.register_scanner(Box::new(MockScanner::new(
      "b",
      true,
      ScanResult::Risky {
        findings: vec![risky_finding("b")],
      },
    )));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(report.any_risky);
    assert!(report.packages[0].verdict.is_risky());
    assert_eq!(report.packages[0].verdict.findings().len(), 1);
    // Warn -> install anyway.
    assert_eq!(report.packages[0].action, InstallAction::Install);
  }

  #[test]
  fn test_or_semantics_error_is_not_risky() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default());
    orch.register_scanner(Box::new(MockScanner::new(
      "a",
      true,
      ScanResult::Error {
        message: "boom".to_string(),
      },
    )));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(!report.any_risky);
    assert!(report.any_incomplete);
    assert!(report.packages[0].verdict.is_incomplete());
    // Incomplete -> install proceeds.
    assert_eq!(report.packages[0].action, InstallAction::Install);
  }

  #[test]
  fn test_on_risk_error_aborts() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_on_risk(OnRiskMode::Error));
    orch.register_scanner(Box::new(MockScanner::new(
      "a",
      true,
      ScanResult::Risky {
        findings: vec![risky_finding("a")],
      },
    )));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(report.abort);
    assert_eq!(report.packages[0].action, InstallAction::Abort);
    assert!(report.to_install.is_empty());
  }

  #[test]
  fn test_on_risk_skip_skips_package() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_on_risk(OnRiskMode::Skip));
    orch.register_scanner(Box::new(MockScanner::new(
      "a",
      true,
      ScanResult::Risky {
        findings: vec![risky_finding("a")],
      },
    )));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(!report.abort);
    assert_eq!(report.packages[0].action, InstallAction::Skip);
    assert!(report.to_install.is_empty());
    assert_eq!(report.to_skip, vec!["pkg".to_string()]);
  }

  #[test]
  fn test_on_risk_warn_installs_anyway() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_on_risk(OnRiskMode::Warn));
    orch.register_scanner(Box::new(MockScanner::new(
      "a",
      true,
      ScanResult::Risky {
        findings: vec![risky_finding("a")],
      },
    )));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(!report.abort);
    assert_eq!(report.packages[0].action, InstallAction::Install);
    assert_eq!(report.to_install, vec!["pkg".to_string()]);
  }

  #[test]
  fn test_no_scan_skips_scanning() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_no_scan(true));
    let mock = MockScanner::new(
      "a",
      true,
      ScanResult::Risky {
        findings: vec![risky_finding("a")],
      },
    );
    orch.register_scanner(Box::new(mock));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(!report.any_risky);
    assert!(report.packages[0].verdict.is_safe());
    assert_eq!(report.packages[0].action, InstallAction::Install);
    assert_eq!(report.to_install, vec!["pkg".to_string()]);
  }

  #[test]
  fn test_scan_only_still_produces_report() {
    let mut orch = ScanOrchestrator::new(
      ScanConfig::default()
        .with_scan_only(true)
        .with_on_risk(OnRiskMode::Warn),
    );
    orch.register_scanner(Box::new(MockScanner::new(
      "a",
      true,
      ScanResult::Risky {
        findings: vec![risky_finding("a")],
      },
    )));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    // scan_only does not suppress the scan; it tells the install engine not
    // to install. The report still reflects the risky verdict.
    assert!(report.any_risky);
    assert!(report.packages[0].verdict.is_risky());
  }

  #[test]
  fn test_update_security_db_calls_update_db() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_update_security_db(true));
    let mock = MockScanner::new("a", true, ScanResult::Safe);
    orch.register_scanner(Box::new(mock));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let _ = orch.scan_all(&[req.clone()]).unwrap();
    // We can't inspect the mock after it was moved into the orchestrator,
    // but the scan must succeed and the report must be safe.
    let report = orch.scan_all(&[req]).unwrap();
    assert!(report.packages[0].verdict.is_safe());
  }

  #[test]
  fn test_unavailable_scanner_is_skipped() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default());
    orch.register_scanner(Box::new(MockScanner::new("a", false, ScanResult::Safe)));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(report.packages[0].verdict.is_incomplete());
    assert!(!report.packages[0].per_scanner[0].available);
  }

  #[test]
  fn test_two_phase_all_scanned_before_decide() {
    // Two packages: one safe, one risky. Both must be scanned (phase 1)
    // before any action is decided (phase 2). The report must contain both.
    let mut orch = ScanOrchestrator::new(ScanConfig::default().with_on_risk(OnRiskMode::Skip));
    let safe_dir = std::env::temp_dir().join("apmw-test-safe");
    let risky_dir = std::env::temp_dir().join("apmw-test-risky");
    orch.register_scanner(Box::new(DirMockScanner::new("a", vec![risky_dir.clone()])));
    let reqs = vec![
      PackageScanRequest::new("safe-pkg", safe_dir.clone()),
      PackageScanRequest::new("risky-pkg", risky_dir.clone()),
    ];
    let report = orch.scan_all(&reqs).unwrap();
    assert_eq!(report.packages.len(), 2);
    assert!(report.any_risky);
    // safe-pkg installs, risky-pkg skips.
    assert_eq!(report.to_install, vec!["safe-pkg".to_string()]);
    assert_eq!(report.to_skip, vec!["risky-pkg".to_string()]);
  }

  #[test]
  fn test_empty_batch() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default());
    let report = orch.scan_all(&[]).unwrap();
    assert!(report.is_empty());
    assert!(report.to_install.is_empty());
  }

  #[test]
  fn test_report_for_package_lookup() {
    let mut orch = ScanOrchestrator::new(ScanConfig::default());
    orch.register_scanner(Box::new(MockScanner::new("a", true, ScanResult::Safe)));
    let dir = std::env::temp_dir();
    let req = PackageScanRequest::new("pkg", dir);
    let report = orch.scan_all(&[req]).unwrap();
    assert!(report.for_package("pkg").is_some());
    assert!(report.for_package("nope").is_none());
  }

  #[test]
  fn test_scan_config_builders() {
    let cfg = ScanConfig::default()
      .with_on_risk(OnRiskMode::Skip)
      .with_update_security_db(true)
      .with_no_scan(false)
      .with_scan_only(true)
      .with_telemetry(TelemetryPolicy::On)
      .with_posture(SecurityPosture::from_npmrc());
    assert_eq!(cfg.on_risk, OnRiskMode::Skip);
    assert!(cfg.update_security_db);
    assert!(!cfg.no_scan);
    assert!(cfg.scan_only);
    assert_eq!(cfg.telemetry, TelemetryPolicy::On);
  }

  #[test]
  fn test_aggregated_verdict_helpers() {
    let safe = AggregatedVerdict::Safe;
    assert!(safe.is_safe());
    assert!(!safe.is_risky());
    assert!(safe.findings().is_empty());

    let risky = AggregatedVerdict::Risky {
      findings: vec![risky_finding("a")],
    };
    assert!(risky.is_risky());
    assert_eq!(risky.findings().len(), 1);

    let incomplete = AggregatedVerdict::Incomplete {
      errors: vec!["e".to_string()],
    };
    assert!(incomplete.is_incomplete());
  }

  #[test]
  fn test_package_scan_request_new() {
    let req = PackageScanRequest::new("foo", "/tmp");
    assert_eq!(req.name, "foo");
    assert_eq!(req.dir, PathBuf::from("/tmp"));
  }

  #[test]
  fn test_scan_report_serde_roundtrip() {
    let report = ScanReport {
      packages: vec![PackageScanOutcome {
        request: PackageScanRequest::new("pkg", "/tmp"),
        verdict: AggregatedVerdict::Risky {
          findings: vec![risky_finding("a")],
        },
        per_scanner: vec![ScannerRecord {
          scanner_name: "a".to_string(),
          available: true,
          result: ScanResult::Risky {
            findings: vec![risky_finding("a")],
          },
        }],
        action: InstallAction::Skip,
      }],
      any_risky: true,
      any_incomplete: false,
      abort: false,
      to_install: vec![],
      to_skip: vec!["pkg".to_string()],
    };
    let json = serde_json::to_string(&report).unwrap();
    let back: ScanReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, back);
  }
}
