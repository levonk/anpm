//! cargo-audit scanner plugin (Rust ecosystem).
//!
//! Runs `cargo audit --json` against a Rust project directory and maps the
//! reported advisories to [`SecurityFinding`]s. Implements the
//! [`Scanner`] trait from [`crate::security::scanner`].

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::error::{ApmwError, Result};
use crate::security::scanner::{ScanResult, Scanner, SecurityFinding, Severity};

/// Scanner plugin that wraps `cargo audit` for Rust projects.
#[derive(Debug, Default, Clone)]
pub struct CargoAudit;

impl CargoAudit {
  /// Creates a new `cargo-audit` scanner plugin.
  pub fn new() -> Self {
    Self
  }

  /// The binary name to look for on `PATH`.
  const BINARY: &'static str = "cargo-audit";
}

impl Scanner for CargoAudit {
  fn name(&self) -> &str {
    "cargo-audit"
  }

  fn available(&self) -> bool {
    let found = which::which(Self::BINARY).is_ok();
    debug!(scanner = self.name(), found, "checking availability");
    found
  }

  fn ensure(&mut self) -> Result<()> {
    if self.available() {
      debug!(scanner = self.name(), "already available, skipping install");
      return Ok(());
    }
    debug!(scanner = self.name(), "installing via cargo install");
    let status = Command::new("cargo")
      .args(["install", "cargo-audit"])
      .status()
      .map_err(|e| ApmwError::SecurityScanFailed(format!("cargo-audit install: {e}")))?;
    if !status.success() {
      return Err(ApmwError::SecurityScanFailed(format!(
        "cargo-audit install exited with {status}"
      )));
    }
    Ok(())
  }

  fn scan(&self, dir: &Path) -> ScanResult {
    debug!(scanner = self.name(), dir = %dir.display(), "running cargo audit --json");
    let output = Command::new("cargo")
      .args(["audit", "--json"])
      .current_dir(dir)
      .output();

    let output = match output {
      Ok(o) => o,
      Err(e) => {
        error!(scanner = self.name(), error = %e, "failed to spawn cargo audit");
        return ScanResult::Error {
          message: format!("failed to spawn cargo audit: {e}"),
        };
      }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_cargo_audit_json(&stdout).unwrap_or_else(|e| {
      error!(scanner = self.name(), error = %e, "failed to parse cargo audit output");
      ScanResult::Error {
        message: format!("cargo audit parse error: {e}"),
      }
    })
  }
}

// ---------------------------------------------------------------------------
// JSON schema (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CargoAuditReport {
  vulnerabilities: CargoVulnerabilities,
}

#[derive(Debug, Deserialize)]
struct CargoVulnerabilities {
  #[serde(default)]
  list: Vec<CargoAdvisoryEntry>,
}

#[derive(Debug, Deserialize)]
struct CargoAdvisoryEntry {
  advisory: CargoAdvisory,
}

#[derive(Debug, Deserialize)]
struct CargoAdvisory {
  id: String,
  package: String,
  #[serde(default)]
  title: Option<String>,
  #[serde(default)]
  description: Option<String>,
  #[serde(default)]
  severity: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parses the JSON stdout of `cargo audit --json` into a [`ScanResult`].
fn parse_cargo_audit_json(stdout: &str) -> std::result::Result<ScanResult, String> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: CargoAuditReport =
    serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

  if report.vulnerabilities.list.is_empty() {
    return Ok(ScanResult::Safe);
  }

  let findings = report
    .vulnerabilities
    .list
    .into_iter()
    .map(|entry| {
      let adv = entry.advisory;
      let severity = parse_severity(adv.severity.as_deref());
      let title = adv
        .title
        .unwrap_or_else(|| format!("{} in {}", adv.id, adv.package));
      let description = adv.description.unwrap_or_else(|| adv.id.clone());
      warn!(
        scanner = "cargo-audit",
        advisory = %adv.id,
        package = %adv.package,
        severity = %severity,
        "vulnerability found"
      );
      SecurityFinding::new("cargo-audit", severity, title, description)
    })
    .collect::<Vec<_>>();

  Ok(ScanResult::Risky { findings })
}

/// Maps cargo-audit severity strings to [`Severity`].
fn parse_severity(s: Option<&str>) -> Severity {
  match s.map(str::to_ascii_lowercase).as_deref() {
    Some("critical") => Severity::Critical,
    Some("high") => Severity::High,
    Some("medium") | Some("moderate") => Severity::Medium,
    Some("low") => Severity::Low,
    // cargo-audit sometimes omits severity — default to Medium.
    _ => Severity::Medium,
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_name() {
    let s = CargoAudit::new();
    assert_eq!(s.name(), "cargo-audit");
  }

  #[test]
  fn test_parse_empty_output_is_safe() {
    let result = parse_cargo_audit_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_no_vulnerabilities_is_safe() {
    let json = r#"{"vulnerabilities":{"count":0,"found":false,"list":[]}}"#;
    let result = parse_cargo_audit_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_vulnerabilities_is_risky() {
    let json = r#"{
      "vulnerabilities": {
        "count": 2,
        "found": true,
        "list": [
          {
            "advisory": {
              "id": "RUSTSEC-2020-0159",
              "package": "chrono",
              "title": "chrono is y2048 vulnerable",
              "description": "chrono has a security issue",
              "severity": "high"
            }
          },
          {
            "advisory": {
              "id": "RUSTSEC-2021-0001",
              "package": "some-crate",
              "title": "Another issue",
              "description": "Bad bug",
              "severity": "critical"
            }
          }
        ]
      }
    }"#;
    let result = parse_cargo_audit_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].severity, Severity::High);
    assert_eq!(findings[0].scanner_name, "cargo-audit");
    assert_eq!(findings[1].severity, Severity::Critical);
  }

  #[test]
  fn test_parse_missing_severity_defaults_medium() {
    let json = r#"{
      "vulnerabilities": {
        "list": [
          {
            "advisory": {
              "id": "RUSTSEC-2020-0001",
              "package": "foo"
            }
          }
        ]
      }
    }"#;
    let result = parse_cargo_audit_json(json).unwrap();
    assert!(result.is_risky());
    assert_eq!(result.findings()[0].severity, Severity::Medium);
  }

  #[test]
  fn test_parse_invalid_json_is_error() {
    let result = parse_cargo_audit_json("{not json}").unwrap_err();
    assert!(result.contains("invalid JSON"));
  }

  #[test]
  fn test_parse_severity_mapping() {
    assert_eq!(parse_severity(Some("critical")), Severity::Critical);
    assert_eq!(parse_severity(Some("HIGH")), Severity::High);
    assert_eq!(parse_severity(Some("medium")), Severity::Medium);
    assert_eq!(parse_severity(Some("moderate")), Severity::Medium);
    assert_eq!(parse_severity(Some("low")), Severity::Low);
    assert_eq!(parse_severity(None), Severity::Medium);
    assert_eq!(parse_severity(Some("unknown")), Severity::Medium);
  }
}
