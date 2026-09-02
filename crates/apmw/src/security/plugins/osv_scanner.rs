//! osv-scanner scanner plugin (multi-ecosystem).
//!
//! Runs `osv-scanner --json` against a project directory and maps the reported
//! vulnerabilities to [`SecurityFinding`]s. osv-scanner supports multiple
//! ecosystems (Rust, Node, Python, Go, etc.). Implements the [`Scanner`] trait
//! from [`crate::security::scanner`].

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::error::{ApmwError, Result};
use crate::security::scanner::{ScanResult, Scanner, SecurityFinding, Severity};

/// Scanner plugin that wraps `osv-scanner` for multi-ecosystem scanning.
#[derive(Debug, Default, Clone)]
pub struct OsvScanner;

impl OsvScanner {
  /// Creates a new `osv-scanner` scanner plugin.
  pub fn new() -> Self {
    Self
  }

  /// The binary name to look for on `PATH`.
  const BINARY: &'static str = "osv-scanner";
}

impl Scanner for OsvScanner {
  fn name(&self) -> &str {
    "osv-scanner"
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
    // osv-scanner is a Go binary — install via `go install`.
    debug!(scanner = self.name(), "installing via go install");
    let status = Command::new("go")
      .args([
        "install",
        "github.com/google/osv-scanner/cmd/osv-scanner@latest",
      ])
      .status()
      .map_err(|e| ApmwError::SecurityScanFailed(format!("osv-scanner install: {e}")))?;
    if !status.success() {
      return Err(ApmwError::SecurityScanFailed(format!(
        "osv-scanner install exited with {status}; ensure Go is installed"
      )));
    }
    Ok(())
  }

  fn scan(&self, dir: &Path) -> ScanResult {
    debug!(scanner = self.name(), dir = %dir.display(), "running osv-scanner --json");
    let output = Command::new("osv-scanner")
      .args(["--json", "-r", dir.to_string_lossy().as_ref()])
      .output();

    let output = match output {
      Ok(o) => o,
      Err(e) => {
        error!(scanner = self.name(), error = %e, "failed to spawn osv-scanner");
        return ScanResult::Error {
          message: format!("failed to spawn osv-scanner: {e}"),
        };
      }
    };

    // osv-scanner exits non-zero when vulnerabilities are found, but JSON is
    // still on stdout. Parse stdout regardless of exit code.
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_osv_scanner_json(&stdout).unwrap_or_else(|e| {
      error!(scanner = self.name(), error = %e, "failed to parse osv-scanner output");
      ScanResult::Error {
        message: format!("osv-scanner parse error: {e}"),
      }
    })
  }
}

// ---------------------------------------------------------------------------
// JSON schema (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OsvReport {
  #[serde(default)]
  results: Vec<OsvResult>,
}

#[derive(Debug, Deserialize)]
struct OsvResult {
  #[serde(default)]
  packages: Vec<OsvPackageEntry>,
}

#[derive(Debug, Deserialize)]
struct OsvPackageEntry {
  package: OsvPackage,
  #[serde(default)]
  vulnerabilities: Vec<OsvVulnerability>,
}

#[derive(Debug, Deserialize)]
struct OsvPackage {
  name: String,
  #[serde(default)]
  version: Option<String>,
  #[serde(default)]
  ecosystem: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OsvVulnerability {
  id: String,
  #[serde(default)]
  summary: Option<String>,
  #[serde(default)]
  severity: Vec<OsvSeverity>,
}

#[derive(Debug, Deserialize)]
struct OsvSeverity {
  #[serde(default)]
  score: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parses the JSON stdout of `osv-scanner --json` into a [`ScanResult`].
fn parse_osv_scanner_json(stdout: &str) -> std::result::Result<ScanResult, String> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: OsvReport = serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

  let mut findings = Vec::new();
  for result in &report.results {
    for pkg_entry in &result.packages {
      for vuln in &pkg_entry.vulnerabilities {
        let version = pkg_entry.package.version.as_deref().unwrap_or("?");
        let ecosystem = pkg_entry.package.ecosystem.as_deref().unwrap_or("?");
        let title = format!(
          "{} in {}@{} ({})",
          vuln.id, pkg_entry.package.name, version, ecosystem
        );
        let description = vuln.summary.clone().unwrap_or_else(|| vuln.id.clone());
        let severity = severity_from_cvss(&vuln.severity);
        warn!(
          scanner = "osv-scanner",
          advisory = %vuln.id,
          package = %pkg_entry.package.name,
          severity = %severity,
          "vulnerability found"
        );
        findings.push(SecurityFinding::new(
          "osv-scanner",
          severity,
          title,
          description,
        ));
      }
    }
  }

  if findings.is_empty() {
    Ok(ScanResult::Safe)
  } else {
    Ok(ScanResult::Risky { findings })
  }
}

/// Derives a [`Severity`] from a CVSS v3 score string.
///
/// CVSS v3 severity bands:
/// - 0.1–3.9 → Low
/// - 4.0–6.9 → Medium
/// - 7.0–8.9 → High
/// - 9.0–10.0 → Critical
fn severity_from_cvss(severities: &[OsvSeverity]) -> Severity {
  let mut max_severity = Severity::Low;
  for sev in severities {
    if let Some(score_str) = &sev.score {
      // CVSS strings look like "CVSS:3.1/AV:N/AC:L/..." — extract the numeric
      // vector score if present at the end, otherwise try to parse the whole
      // string as a float.
      let parsed = score_str
        .rsplit('/')
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| score_str.parse::<f64>().ok());
      if let Some(score) = parsed {
        let s = if score >= 9.0 {
          Severity::Critical
        } else if score >= 7.0 {
          Severity::High
        } else if score >= 4.0 {
          Severity::Medium
        } else {
          Severity::Low
        };
        if s > max_severity {
          max_severity = s;
        }
      }
    }
  }
  max_severity
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_name() {
    let s = OsvScanner::new();
    assert_eq!(s.name(), "osv-scanner");
  }

  #[test]
  fn test_parse_empty_output_is_safe() {
    let result = parse_osv_scanner_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_no_vulnerabilities_is_safe() {
    let json = r#"{
      "results": [
        {
          "packages": [
            {"package": {"name": "left-pad", "version": "1.0.0", "ecosystem": "npm"}, "vulnerabilities": []}
          ]
        }
      ]
    }"#;
    let result = parse_osv_scanner_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_vulnerabilities_is_risky() {
    let json = r#"{
      "results": [
        {
          "packages": [
            {
              "package": {"name": "left-pad", "version": "1.0.0", "ecosystem": "npm"},
              "vulnerabilities": [
                {
                  "id": "GHSA-1234-5678-9abc",
                  "summary": "Prototype pollution in left-pad",
                  "severity": [
                    {"type": "CVSS_V3", "score": "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"}
                  ]
                }
              ]
            }
          ]
        }
      ]
    }"#;
    let result = parse_osv_scanner_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].scanner_name, "osv-scanner");
    assert!(findings[0].title.contains("GHSA-1234-5678-9abc"));
    assert_eq!(findings[0].description, "Prototype pollution in left-pad");
  }

  #[test]
  fn test_severity_from_cvss() {
    let critical = [OsvSeverity {
      score: Some("9.5".to_string()),
    }];
    assert_eq!(severity_from_cvss(&critical), Severity::Critical);

    let high = [OsvSeverity {
      score: Some("7.5".to_string()),
    }];
    assert_eq!(severity_from_cvss(&high), Severity::High);

    let medium = [OsvSeverity {
      score: Some("5.0".to_string()),
    }];
    assert_eq!(severity_from_cvss(&medium), Severity::Medium);

    let low = [OsvSeverity {
      score: Some("2.0".to_string()),
    }];
    assert_eq!(severity_from_cvss(&low), Severity::Low);

    let empty: [OsvSeverity; 0] = [];
    assert_eq!(severity_from_cvss(&empty), Severity::Low);
  }

  #[test]
  fn test_parse_invalid_json_is_error() {
    let result = parse_osv_scanner_json("{not json}").unwrap_err();
    assert!(result.contains("invalid JSON"));
  }
}
