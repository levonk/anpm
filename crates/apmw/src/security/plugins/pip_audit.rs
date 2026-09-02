//! pip-audit scanner plugin (Python ecosystem).
//!
//! Runs `pip-audit --format json` against a Python project directory and maps
//! the reported vulnerabilities to [`SecurityFinding`]s. Implements the
//! [`Scanner`] trait from [`crate::security::scanner`].

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::error::{ApmwError, Result};
use crate::security::scanner::{ScanResult, Scanner, SecurityFinding, Severity};

/// Scanner plugin that wraps `pip-audit` for Python projects.
#[derive(Debug, Default, Clone)]
pub struct PipAudit;

impl PipAudit {
  /// Creates a new `pip-audit` scanner plugin.
  pub fn new() -> Self {
    Self
  }

  /// The binary name to look for on `PATH`.
  const BINARY: &'static str = "pip-audit";
}

impl Scanner for PipAudit {
  fn name(&self) -> &str {
    "pip-audit"
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
    debug!(scanner = self.name(), "installing via pip install");
    let status = Command::new("pip")
      .args(["install", "pip-audit"])
      .status()
      .map_err(|e| ApmwError::SecurityScanFailed(format!("pip-audit install: {e}")))?;
    if !status.success() {
      return Err(ApmwError::SecurityScanFailed(format!(
        "pip-audit install exited with {status}"
      )));
    }
    Ok(())
  }

  fn scan(&self, dir: &Path) -> ScanResult {
    debug!(scanner = self.name(), dir = %dir.display(), "running pip-audit --format json");
    let output = Command::new("pip-audit")
      .args(["--format", "json"])
      .current_dir(dir)
      .output();

    let output = match output {
      Ok(o) => o,
      Err(e) => {
        error!(scanner = self.name(), error = %e, "failed to spawn pip-audit");
        return ScanResult::Error {
          message: format!("failed to spawn pip-audit: {e}"),
        };
      }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pip_audit_json(&stdout).unwrap_or_else(|e| {
      error!(scanner = self.name(), error = %e, "failed to parse pip-audit output");
      ScanResult::Error {
        message: format!("pip-audit parse error: {e}"),
      }
    })
  }
}

// ---------------------------------------------------------------------------
// JSON schema (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct PipAuditReport {
  #[serde(default)]
  dependencies: Vec<PipDependency>,
}

#[derive(Debug, Deserialize)]
struct PipDependency {
  name: String,
  version: String,
  #[serde(default)]
  vulns: Vec<PipVuln>,
}

#[derive(Debug, Deserialize)]
struct PipVuln {
  id: String,
  #[serde(default)]
  fix_versions: Vec<String>,
  #[serde(default)]
  description: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parses the JSON stdout of `pip-audit --format json` into a [`ScanResult`].
fn parse_pip_audit_json(stdout: &str) -> std::result::Result<ScanResult, String> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: PipAuditReport =
    serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

  let mut findings = Vec::new();
  for dep in &report.dependencies {
    for vuln in &dep.vulns {
      let fix = if vuln.fix_versions.is_empty() {
        "no fix available".to_string()
      } else {
        format!("fix: upgrade to {}", vuln.fix_versions.join(", "))
      };
      let description = match &vuln.description {
        Some(desc) => format!("{desc} ({fix})"),
        None => format!("{} in {}@{}: {}", vuln.id, dep.name, dep.version, fix),
      };
      let title = format!("{} in {}=={}", vuln.id, dep.name, dep.version);
      warn!(
        scanner = "pip-audit",
        advisory = %vuln.id,
        package = %dep.name,
        version = %dep.version,
        "vulnerability found"
      );
      // pip-audit does not provide severity levels — default to Medium.
      findings.push(SecurityFinding::new(
        "pip-audit",
        Severity::Medium,
        title,
        description,
      ));
    }
  }

  if findings.is_empty() {
    Ok(ScanResult::Safe)
  } else {
    Ok(ScanResult::Risky { findings })
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
    let s = PipAudit::new();
    assert_eq!(s.name(), "pip-audit");
  }

  #[test]
  fn test_parse_empty_output_is_safe() {
    let result = parse_pip_audit_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_no_vulnerabilities_is_safe() {
    let json = r#"{
      "dependencies": [
        {"name": "requests", "version": "2.31.0", "vulns": []}
      ]
    }"#;
    let result = parse_pip_audit_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_vulnerabilities_is_risky() {
    let json = r#"{
      "dependencies": [
        {
          "name": "requests",
          "version": "2.31.0",
          "vulns": [
            {
              "id": "PYSEC-2023-1234",
              "fix_versions": ["2.32.0"],
              "description": "SSRF vulnerability in requests"
            },
            {
              "id": "PYSEC-2023-5678",
              "fix_versions": [],
              "description": "DoS in requests"
            }
          ]
        }
      ]
    }"#;
    let result = parse_pip_audit_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].severity, Severity::Medium);
    assert_eq!(findings[0].scanner_name, "pip-audit");
    assert!(findings[0].title.contains("PYSEC-2023-1234"));
    assert!(findings[0].description.contains("2.32.0"));
    assert!(findings[1].description.contains("no fix available"));
  }

  #[test]
  fn test_parse_invalid_json_is_error() {
    let result = parse_pip_audit_json("{not json}").unwrap_err();
    assert!(result.contains("invalid JSON"));
  }
}
