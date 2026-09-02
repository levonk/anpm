//! Scanner plugin contract and core security data types.
//!
//! Defines the `Scanner` trait that every scanner plugin must implement,
//! following the `<name>_available` / `<name>_ensure` / `<name>_scan` contract
//! from `executable_skill-install.sh` (PRD FR-5.2). A scanner reports one of
//! three outcomes for a directory: `Safe`, `Risky` (with findings), or `Error`
//! (the scan could not complete — an incomplete-scan warning, not a risk
//! finding per FR-5.3).
//!
//! This module also defines the supporting data structures used across the
//! security subsystem: `Severity`, `SecurityFinding`, `ScanResult`,
//! `OnRiskMode`, `TelemetryPolicy`, and `SecurityPosture`.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Severity level for a single security finding.
///
/// Ordered from least to most severe. `Critical` is the highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
  /// Informational / low-impact issue.
  Low,
  /// Moderate issue worth addressing.
  Medium,
  /// Significant issue that should be fixed before release.
  High,
  /// Exploitable or otherwise urgent issue.
  Critical,
}

impl Severity {
  /// Returns `true` when this severity meets or exceeds the given `threshold`.
  pub fn at_least(self, threshold: Severity) -> bool {
    self >= threshold
  }
}

impl fmt::Display for Severity {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Severity::Low => write!(f, "low"),
      Severity::Medium => write!(f, "medium"),
      Severity::High => write!(f, "high"),
      Severity::Critical => write!(f, "critical"),
    }
  }
}

impl FromStr for Severity {
  type Err = String;

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    match s.to_ascii_lowercase().as_str() {
      "low" => Ok(Severity::Low),
      "medium" => Ok(Severity::Medium),
      "high" => Ok(Severity::High),
      "critical" => Ok(Severity::Critical),
      other => Err(format!(
        "unknown severity '{other}', expected low|medium|high|critical"
      )),
    }
  }
}

/// A single security finding reported by a scanner.
///
/// Mirrors the structured output a scanner plugin produces for one advisory
/// or policy violation. All fields are serializable so findings can flow
/// through the audit log and TOON output unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFinding {
  /// How serious the finding is.
  pub severity: Severity,
  /// Short human-readable summary (e.g. "CVE-2026-1234 in left-pad").
  pub title: String,
  /// Longer description with remediation guidance.
  pub description: String,
  /// Name of the scanner that produced this finding.
  pub scanner_name: String,
}

impl SecurityFinding {
  /// Creates a new finding with the given fields.
  pub fn new(
    scanner_name: impl Into<String>,
    severity: Severity,
    title: impl Into<String>,
    description: impl Into<String>,
  ) -> Self {
    Self {
      severity,
      title: title.into(),
      description: description.into(),
      scanner_name: scanner_name.into(),
    }
  }
}

/// The outcome of scanning a single directory with a single scanner.
///
/// Maps directly to the exit-code contract from `executable_skill-install.sh`:
/// `0` = safe, `1` = risky, `2` = error. Per FR-5.3, scanner errors are
/// incomplete-scan warnings — they do **not** count as risk findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ScanResult {
  /// No findings — the directory is considered safe by this scanner.
  #[default]
  Safe,
  /// One or more findings — the directory is considered risky.
  Risky {
    /// The findings that triggered the risky verdict.
    findings: Vec<SecurityFinding>,
  },
  /// The scanner could not complete — an incomplete-scan warning, not a risk.
  Error {
    /// Why the scan failed.
    message: String,
  },
}

impl ScanResult {
  /// Returns `true` when the scanner considers the directory safe.
  pub fn is_safe(&self) -> bool {
    matches!(self, ScanResult::Safe)
  }

  /// Returns `true` when the scanner considers the directory risky.
  pub fn is_risky(&self) -> bool {
    matches!(self, ScanResult::Risky { .. })
  }

  /// Returns `true` when the scanner could not complete the scan.
  pub fn is_error(&self) -> bool {
    matches!(self, ScanResult::Error { .. })
  }

  /// Returns the findings if this is a `Risky` result, otherwise an empty slice.
  pub fn findings(&self) -> &[SecurityFinding] {
    if let ScanResult::Risky { findings } = self {
      findings
    } else {
      &[]
    }
  }

  /// Returns the error message if this is an `Error` result.
  pub fn error_message(&self) -> Option<&str> {
    if let ScanResult::Error { message } = self {
      Some(message)
    } else {
      None
    }
  }
}

/// The scanner plugin contract (PRD FR-5.2).
///
/// Every scanner plugin implements three operations mirroring the
/// `<name>_available` / `<name>_ensure` / `<name>_scan` shell contract:
///
/// - `available` — can this scanner run in the current environment?
/// - `ensure`    — prepare/install the scanner so `scan` can run.
/// - `scan`      — scan a local directory and return a `ScanResult`.
///
/// Plugins are registered with the [`ScanOrchestrator`](crate::security::ScanOrchestrator)
/// and invoked during the two-phase scan. The orchestrator uses OR semantics:
/// if **any** registered scanner reports `Risky`, the package is risky.
pub trait Scanner: Send + Sync {
  /// The stable name of this scanner (e.g. `"npm-audit"`, `"trivy"`).
  fn name(&self) -> &str;

  /// Returns `true` when the scanner can run in the current environment.
  ///
  /// Mirrors `<name>_available`. This must be cheap (no network, no installs).
  fn available(&self) -> bool;

  /// Prepares the scanner so that `scan` can run — installs the scanner
  /// binary if missing, fetches signature databases, etc.
  ///
  /// Mirrors `<name>_ensure`. Called once before a scan batch when the
  /// scanner reports `available() == true`.
  fn ensure(&mut self) -> Result<()>;

  /// Refreshes the scanner's security database.
  ///
  /// Called when the user passes `--update-security-db` (PRD FR-5.5). The
  /// default implementation simply calls [`ensure`](Self::ensure); plugins
  /// that maintain a separate update flow override this.
  fn update_db(&mut self) -> Result<()> {
    self.ensure()
  }

  /// Scans a local directory and returns a `ScanResult`.
  ///
  /// Mirrors `<name>_scan DIR`. Must not panic — return `ScanResult::Error`
  /// for any failure so the orchestrator can record an incomplete-scan
  /// warning without aborting the whole batch.
  fn scan(&self, dir: &Path) -> ScanResult;
}

/// How to react when a package is flagged risky (PRD FR-5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnRiskMode {
  /// Ask the user on an interactive TTY; error if there is no TTY.
  #[default]
  Prompt,
  /// Abort the entire operation immediately.
  Error,
  /// Print a warning and install anyway.
  Warn,
  /// Skip the risky package and continue with the rest.
  Skip,
}

impl OnRiskMode {
  /// Returns `true` when this mode requires a TTY for confirmation.
  pub fn requires_tty(self) -> bool {
    matches!(self, OnRiskMode::Prompt)
  }
}

impl fmt::Display for OnRiskMode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OnRiskMode::Prompt => write!(f, "prompt"),
      OnRiskMode::Error => write!(f, "error"),
      OnRiskMode::Warn => write!(f, "warn"),
      OnRiskMode::Skip => write!(f, "skip"),
    }
  }
}

impl FromStr for OnRiskMode {
  type Err = String;

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    match s.to_ascii_lowercase().as_str() {
      "prompt" => Ok(OnRiskMode::Prompt),
      "error" => Ok(OnRiskMode::Error),
      "warn" => Ok(OnRiskMode::Warn),
      "skip" => Ok(OnRiskMode::Skip),
      other => Err(format!(
        "unknown on-risk mode '{other}', expected prompt|error|warn|skip"
      )),
    }
  }
}

/// Telemetry policy for packages being scanned/installed (PRD FR-5.6).
///
/// This is distinct from the apmw tool's own anonymized usage telemetry
/// (FR-5.8). The package telemetry policy controls whether telemetry is
/// collected **for the package under installation**: off for third-party
/// packages, on or neutral for levonk-owned packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryPolicy {
  /// Telemetry is off — used for third-party packages.
  #[default]
  Off,
  /// Telemetry is on — used for levonk-owned packages.
  On,
  /// Telemetry is left at the package's own default (neither forced on nor off).
  Neutral,
}

impl fmt::Display for TelemetryPolicy {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      TelemetryPolicy::Off => write!(f, "off"),
      TelemetryPolicy::On => write!(f, "on"),
      TelemetryPolicy::Neutral => write!(f, "neutral"),
    }
  }
}

impl FromStr for TelemetryPolicy {
  type Err = String;

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    match s.to_ascii_lowercase().as_str() {
      "off" => Ok(TelemetryPolicy::Off),
      "on" => Ok(TelemetryPolicy::On),
      "neutral" => Ok(TelemetryPolicy::Neutral),
      other => Err(format!(
        "unknown telemetry policy '{other}', expected off|on|neutral"
      )),
    }
  }
}

/// The audit level that triggers a risky verdict.
///
/// Mirrors the `audit-level=high` setting from `npmrc.tmpl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
  /// Report low-severity or higher.
  Low,
  /// Report medium-severity or higher.
  Moderate,
  /// Report high-severity or higher (the npmrc.tmpl default).
  #[default]
  High,
  /// Report critical-severity only.
  Critical,
}

impl fmt::Display for AuditLevel {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AuditLevel::Low => write!(f, "low"),
      AuditLevel::Moderate => write!(f, "moderate"),
      AuditLevel::High => write!(f, "high"),
      AuditLevel::Critical => write!(f, "critical"),
    }
  }
}

/// Security posture mirrored from `npmrc.tmpl` (PRD FR-5.7).
///
/// These are the npm configuration defaults apmw enforces when scanning and
/// installing Node packages. They encode a conservative supply-chain stance:
/// audit on at `high` severity, strict engine checks, frozen lockfiles,
/// exact saves, and provenance verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPosture {
  /// `audit=true` — run `npm audit` before install.
  pub audit: bool,
  /// `audit-level=high` — the severity that triggers a risky verdict.
  pub audit_level: AuditLevel,
  /// `engine-strict=true` — fail when the Node engine range is not satisfied.
  pub engine_strict: bool,
  /// `frozen-lockfile=true` — refuse to mutate the lockfile during install.
  pub frozen_lockfile: bool,
  /// `save-exact=true` — pin installed versions exactly (no `^`/`~`).
  pub save_exact: bool,
  /// `provenance=true` — require build provenance for published packages.
  pub provenance: bool,
}

impl SecurityPosture {
  /// Returns the posture exactly as specified by `npmrc.tmpl`.
  ///
  /// `audit=true`, `audit-level=high`, `engine-strict=true`,
  /// `frozen-lockfile=true`, `save-exact=true`, `provenance=true`.
  pub fn from_npmrc() -> Self {
    Self {
      audit: true,
      audit_level: AuditLevel::High,
      engine_strict: true,
      frozen_lockfile: true,
      save_exact: true,
      provenance: true,
    }
  }
}

impl Default for SecurityPosture {
  fn default() -> Self {
    Self::from_npmrc()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_severity_ordering() {
    assert!(Severity::Critical > Severity::High);
    assert!(Severity::High > Severity::Medium);
    assert!(Severity::Medium > Severity::Low);
    assert!(Severity::Critical.at_least(Severity::High));
    assert!(!Severity::Low.at_least(Severity::High));
  }

  #[test]
  fn test_severity_serde_roundtrip() {
    let json = serde_json::to_string(&Severity::High).unwrap();
    assert_eq!(json, "\"high\"");
    let back: Severity = serde_json::from_str(&json).unwrap();
    assert_eq!(back, Severity::High);
  }

  #[test]
  fn test_severity_from_str() {
    assert_eq!("HIGH".parse::<Severity>().unwrap(), Severity::High);
    assert_eq!("critical".parse::<Severity>().unwrap(), Severity::Critical);
    assert!("unknown".parse::<Severity>().is_err());
  }

  #[test]
  fn test_severity_display() {
    assert_eq!(Severity::Low.to_string(), "low");
    assert_eq!(Severity::Medium.to_string(), "medium");
    assert_eq!(Severity::High.to_string(), "high");
    assert_eq!(Severity::Critical.to_string(), "critical");
  }

  #[test]
  fn test_security_finding_new() {
    let f = SecurityFinding::new("trivy", Severity::High, "CVE-1", "bad dep");
    assert_eq!(f.scanner_name, "trivy");
    assert_eq!(f.severity, Severity::High);
    assert_eq!(f.title, "CVE-1");
    assert_eq!(f.description, "bad dep");
  }

  #[test]
  fn test_scan_result_safe() {
    let r = ScanResult::Safe;
    assert!(r.is_safe());
    assert!(!r.is_risky());
    assert!(!r.is_error());
    assert!(r.findings().is_empty());
    assert!(r.error_message().is_none());
  }

  #[test]
  fn test_scan_result_risky() {
    let f = SecurityFinding::new("npm-audit", Severity::High, "CVE-1", "x");
    let r = ScanResult::Risky {
      findings: vec![f.clone()],
    };
    assert!(!r.is_safe());
    assert!(r.is_risky());
    assert_eq!(r.findings(), &[f]);
  }

  #[test]
  fn test_scan_result_error() {
    let r = ScanResult::Error {
      message: "boom".to_string(),
    };
    assert!(r.is_error());
    assert_eq!(r.error_message(), Some("boom"));
    assert!(!r.is_risky());
  }

  #[test]
  fn test_scan_result_serde_roundtrip() {
    let r = ScanResult::Risky {
      findings: vec![SecurityFinding::new("s", Severity::Low, "t", "d")],
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: ScanResult = serde_json::from_str(&json).unwrap();
    assert_eq!(r, back);
  }

  #[test]
  fn test_scan_result_default_is_safe() {
    assert_eq!(ScanResult::default(), ScanResult::Safe);
  }

  #[test]
  fn test_on_risk_mode_from_str() {
    assert_eq!("prompt".parse::<OnRiskMode>().unwrap(), OnRiskMode::Prompt);
    assert_eq!("ERROR".parse::<OnRiskMode>().unwrap(), OnRiskMode::Error);
    assert_eq!("warn".parse::<OnRiskMode>().unwrap(), OnRiskMode::Warn);
    assert_eq!("skip".parse::<OnRiskMode>().unwrap(), OnRiskMode::Skip);
    assert!("nope".parse::<OnRiskMode>().is_err());
  }

  #[test]
  fn test_on_risk_mode_default_is_prompt() {
    assert_eq!(OnRiskMode::default(), OnRiskMode::Prompt);
  }

  #[test]
  fn test_on_risk_mode_requires_tty() {
    assert!(OnRiskMode::Prompt.requires_tty());
    assert!(!OnRiskMode::Error.requires_tty());
    assert!(!OnRiskMode::Warn.requires_tty());
    assert!(!OnRiskMode::Skip.requires_tty());
  }

  #[test]
  fn test_on_risk_mode_serde_roundtrip() {
    let json = serde_json::to_string(&OnRiskMode::Skip).unwrap();
    assert_eq!(json, "\"skip\"");
    let back: OnRiskMode = serde_json::from_str(&json).unwrap();
    assert_eq!(back, OnRiskMode::Skip);
  }

  #[test]
  fn test_telemetry_policy_from_str() {
    assert_eq!(
      "off".parse::<TelemetryPolicy>().unwrap(),
      TelemetryPolicy::Off
    );
    assert_eq!(
      "ON".parse::<TelemetryPolicy>().unwrap(),
      TelemetryPolicy::On
    );
    assert_eq!(
      "neutral".parse::<TelemetryPolicy>().unwrap(),
      TelemetryPolicy::Neutral
    );
    assert!("?".parse::<TelemetryPolicy>().is_err());
  }

  #[test]
  fn test_telemetry_policy_default_is_off() {
    // Third-party packages default to telemetry off (FR-5.6).
    assert_eq!(TelemetryPolicy::default(), TelemetryPolicy::Off);
  }

  #[test]
  fn test_security_posture_from_npmrc() {
    let p = SecurityPosture::from_npmrc();
    assert!(p.audit);
    assert_eq!(p.audit_level, AuditLevel::High);
    assert!(p.engine_strict);
    assert!(p.frozen_lockfile);
    assert!(p.save_exact);
    assert!(p.provenance);
  }

  #[test]
  fn test_security_posture_serde_roundtrip() {
    let p = SecurityPosture::from_npmrc();
    let json = serde_json::to_string(&p).unwrap();
    let back: SecurityPosture = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
  }

  #[test]
  fn test_security_posture_default_matches_npmrc() {
    assert_eq!(SecurityPosture::default(), SecurityPosture::from_npmrc());
  }
}
