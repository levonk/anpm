//! npm audit scanner plugin (Node ecosystem).
//!
//! Runs `npm audit --json` against a Node project directory and maps the
//! reported vulnerabilities to [`SecurityFinding`]s. Implements the
//! [`Scanner`] trait from [`crate::security::scanner`].

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::error::{ApmwError, Result};
use crate::security::scanner::{ScanResult, Scanner, SecurityFinding, Severity};

/// Scanner plugin that wraps `npm audit` for Node projects.
#[derive(Debug, Default, Clone)]
pub struct NpmAudit;

impl NpmAudit {
  /// Creates a new `npm-audit` scanner plugin.
  pub fn new() -> Self {
    Self
  }

  /// The binary name to look for on `PATH`.
  const BINARY: &'static str = "npm";
}

impl Scanner for NpmAudit {
  fn name(&self) -> &str {
    "npm-audit"
  }

  fn available(&self) -> bool {
    let found = which::which(Self::BINARY).is_ok();
    debug!(scanner = self.name(), found, "checking availability");
    found
  }

  fn ensure(&mut self) -> Result<()> {
    if self.available() {
      debug!(scanner = self.name(), "npm already available");
      return Ok(());
    }
    // npm ships with Node.js — if it's missing, Node itself is missing.
    Err(ApmwError::SecurityScanFailed(
      "npm not found on PATH; install Node.js to enable npm-audit".to_string(),
    ))
  }

  fn scan(&self, dir: &Path) -> ScanResult {
    debug!(scanner = self.name(), dir = %dir.display(), "running npm audit --json");
    let output = Command::new("npm")
      .args(["audit", "--json"])
      .current_dir(dir)
      .output();

    let output = match output {
      Ok(o) => o,
      Err(e) => {
        error!(scanner = self.name(), error = %e, "failed to spawn npm audit");
        return ScanResult::Error {
          message: format!("failed to spawn npm audit: {e}"),
        };
      }
    };

    // npm audit exits non-zero when vulnerabilities are found, but the JSON
    // is still on stdout. We parse the stdout regardless of exit code.
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_npm_audit_json(&stdout).unwrap_or_else(|e| {
      error!(scanner = self.name(), error = %e, "failed to parse npm audit output");
      ScanResult::Error {
        message: format!("npm audit parse error: {e}"),
      }
    })
  }
}

// ---------------------------------------------------------------------------
// JSON schema (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct NpmAuditReport {
  #[serde(default)]
  vulnerabilities: std::collections::BTreeMap<String, NpmVulnerability>,
}

#[derive(Debug, Deserialize)]
struct NpmVulnerability {
  name: String,
  severity: String,
  #[serde(default)]
  via: Vec<NpmVia>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmVia {
  /// A string advisory reference (e.g. "GHSA-xxxx").
  #[allow(dead_code)]
  Ref(String),
  /// An inline advisory object.
  Advisory(NpmAdvisory),
}

#[derive(Debug, Deserialize)]
struct NpmAdvisory {
  #[serde(default)]
  #[allow(dead_code)]
  source: Option<u64>,
  #[serde(default)]
  title: Option<String>,
  #[serde(default)]
  url: Option<String>,
  #[serde(default)]
  #[allow(dead_code)]
  severity: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parses the JSON stdout of `npm audit --json` into a [`ScanResult`].
fn parse_npm_audit_json(stdout: &str) -> std::result::Result<ScanResult, String> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: NpmAuditReport =
    serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

  if report.vulnerabilities.is_empty() {
    return Ok(ScanResult::Safe);
  }

  let mut findings = Vec::new();
  for (_key, vuln) in report.vulnerabilities {
    let severity = parse_severity(&vuln.severity);
    // Try to extract a title from the first inline advisory in `via`.
    let title = vuln
      .via
      .iter()
      .find_map(|via| match via {
        NpmVia::Advisory(adv) => adv.title.clone(),
        _ => None,
      })
      .unwrap_or_else(|| format!("{} in {}", vuln.severity, vuln.name));

    let description = vuln
      .via
      .iter()
      .find_map(|via| match via {
        NpmVia::Advisory(adv) => adv.url.clone(),
        _ => None,
      })
      .unwrap_or_else(|| format!("vulnerability in npm package '{}'", vuln.name));

    warn!(
      scanner = "npm-audit",
      package = %vuln.name,
      severity = %severity,
      "vulnerability found"
    );
    findings.push(SecurityFinding::new(
      "npm-audit",
      severity,
      title,
      description,
    ));
  }

  Ok(ScanResult::Risky { findings })
}

/// Maps npm severity strings to [`Severity`].
fn parse_severity(s: &str) -> Severity {
  match s.to_ascii_lowercase().as_str() {
    "critical" => Severity::Critical,
    "high" => Severity::High,
    "moderate" | "medium" => Severity::Medium,
    "low" | "info" => Severity::Low,
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
    let s = NpmAudit::new();
    assert_eq!(s.name(), "npm-audit");
  }

  #[test]
  fn test_parse_empty_output_is_safe() {
    let result = parse_npm_audit_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_no_vulnerabilities_is_safe() {
    let json = r#"{"vulnerabilities":{},"metadata":{"vulnerabilities":{"total":0}}}"#;
    let result = parse_npm_audit_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_vulnerabilities_is_risky() {
    let json = r#"{
      "auditReportVersion": 2,
      "vulnerabilities": {
        "left-pad": {
          "name": "left-pad",
          "severity": "high",
          "via": [
            {
              "source": 123,
              "title": "Prototype pollution",
              "url": "https://example.com/advisory",
              "severity": "high"
            }
          ],
          "effects": [],
          "range": ">=1.0.0",
          "nodes": [],
          "fixAvailable": true
        },
        "lodash": {
          "name": "lodash",
          "severity": "critical",
          "via": ["GHSA-1234"],
          "effects": [],
          "range": ">=4.0.0",
          "nodes": [],
          "fixAvailable": false
        }
      },
      "metadata": {
        "vulnerabilities": {"info":0,"low":0,"moderate":0,"high":1,"critical":1,"total":2}
      }
    }"#;
    let result = parse_npm_audit_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].severity, Severity::High);
    assert_eq!(findings[0].scanner_name, "npm-audit");
    assert_eq!(findings[1].severity, Severity::Critical);
  }

  #[test]
  fn test_parse_invalid_json_is_error() {
    let result = parse_npm_audit_json("{not json}").unwrap_err();
    assert!(result.contains("invalid JSON"));
  }

  #[test]
  fn test_parse_severity_mapping() {
    assert_eq!(parse_severity("critical"), Severity::Critical);
    assert_eq!(parse_severity("HIGH"), Severity::High);
    assert_eq!(parse_severity("moderate"), Severity::Medium);
    assert_eq!(parse_severity("low"), Severity::Low);
    assert_eq!(parse_severity("info"), Severity::Low);
    assert_eq!(parse_severity("unknown"), Severity::Medium);
  }
}
