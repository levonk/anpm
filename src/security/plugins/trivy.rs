//! trivy container image scanner plugin.
//!
//! Runs `trivy image --format json <image>` against a container image and maps
//! the reported vulnerabilities to [`SecurityFinding`]s. Implements the
//! [`Scanner`] trait from [`crate::security::scanner`].
//!
//! Because the [`Scanner::scan`] trait method receives a `&Path`, the path is
//! interpreted as a container image reference (e.g. `nginx:1.21`). Callers
//! that scan directories should use the directory-based scanners; trivy is for
//! container images only.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::error::{ApmwError, Result};
use crate::security::scanner::{ScanResult, Scanner, SecurityFinding, Severity};

/// Scanner plugin that wraps `trivy image` for container image scanning.
#[derive(Debug, Default, Clone)]
pub struct Trivy;

impl Trivy {
  /// Creates a new `trivy` scanner plugin.
  pub fn new() -> Self {
    Self
  }

  /// The binary name to look for on `PATH`.
  const BINARY: &'static str = "trivy";
}

impl Scanner for Trivy {
  fn name(&self) -> &str {
    "trivy"
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
    // trivy is best installed via package manager or install script. We try
    // brew first, then surface an error with instructions.
    #[cfg(target_os = "macos")]
    {
      if which::which("brew").is_ok() {
        debug!(scanner = self.name(), "installing via brew install trivy");
        let status = Command::new("brew")
          .args(["install", "trivy"])
          .status()
          .map_err(|e| ApmwError::SecurityScanFailed(format!("trivy install: {e}")))?;
        if status.success() {
          return Ok(());
        }
      }
    }
    Err(ApmwError::SecurityScanFailed(
      "trivy not found and could not be installed automatically; \
       install trivy manually (see https://aquasecurity.github.io/trivy/)"
        .to_string(),
    ))
  }

  fn scan(&self, target: &Path) -> ScanResult {
    // The path is interpreted as a container image reference.
    let image = target.to_string_lossy();
    debug!(scanner = self.name(), image = %image, "running trivy image --format json");
    let output = Command::new("trivy")
      .args(["image", "--format", "json", image.as_ref()])
      .output();

    let output = match output {
      Ok(o) => o,
      Err(e) => {
        error!(scanner = self.name(), error = %e, "failed to spawn trivy");
        return ScanResult::Error {
          message: format!("failed to spawn trivy: {e}"),
        };
      }
    };

    // trivy exits non-zero when vulnerabilities are found, but JSON is still
    // on stdout. Parse stdout regardless of exit code.
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_trivy_json(&stdout).unwrap_or_else(|e| {
      error!(scanner = self.name(), error = %e, "failed to parse trivy output");
      ScanResult::Error {
        message: format!("trivy parse error: {e}"),
      }
    })
  }
}

// ---------------------------------------------------------------------------
// JSON schema (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TrivyReport {
  #[serde(default, rename = "Results")]
  results: Vec<TrivyResult>,
}

#[derive(Debug, Deserialize)]
struct TrivyResult {
  #[serde(default, rename = "Target")]
  target: Option<String>,
  #[serde(default, rename = "Vulnerabilities")]
  vulnerabilities: Vec<TrivyVulnerability>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TrivyVulnerability {
  #[serde(rename = "VulnerabilityID")]
  vulnerability_id: String,
  pkg_name: String,
  #[serde(default)]
  installed_version: Option<String>,
  #[serde(default)]
  fixed_version: Option<String>,
  severity: String,
  #[serde(default)]
  title: Option<String>,
  #[serde(default)]
  description: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parses the JSON stdout of `trivy image --format json` into a [`ScanResult`].
fn parse_trivy_json(stdout: &str) -> std::result::Result<ScanResult, String> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: TrivyReport =
    serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

  let mut findings = Vec::new();
  for result in &report.results {
    let target = result.target.as_deref().unwrap_or("unknown");
    for vuln in &result.vulnerabilities {
      let severity = parse_severity(&vuln.severity);
      let title = vuln
        .title
        .clone()
        .unwrap_or_else(|| format!("{} in {}", vuln.vulnerability_id, vuln.pkg_name));
      let installed = vuln.installed_version.as_deref().unwrap_or("?");
      let fixed = vuln.fixed_version.as_deref().unwrap_or("no fix");
      let description = vuln.description.clone().unwrap_or_else(|| {
        format!(
          "{} in {} (installed: {}, fixed: {}) [target: {}]",
          vuln.vulnerability_id, vuln.pkg_name, installed, fixed, target
        )
      });
      warn!(
        scanner = "trivy",
        advisory = %vuln.vulnerability_id,
        package = %vuln.pkg_name,
        severity = %severity,
        "vulnerability found"
      );
      findings.push(SecurityFinding::new("trivy", severity, title, description));
    }
  }

  if findings.is_empty() {
    Ok(ScanResult::Safe)
  } else {
    Ok(ScanResult::Risky { findings })
  }
}

/// Maps trivy severity strings to [`Severity`].
fn parse_severity(s: &str) -> Severity {
  match s.to_ascii_lowercase().as_str() {
    "critical" => Severity::Critical,
    "high" => Severity::High,
    "medium" => Severity::Medium,
    "low" => Severity::Low,
    // trivy may report "unknown" — default to Medium.
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
    let s = Trivy::new();
    assert_eq!(s.name(), "trivy");
  }

  #[test]
  fn test_parse_empty_output_is_safe() {
    let result = parse_trivy_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_no_vulnerabilities_is_safe() {
    let json = r#"{
      "SchemaVersion": 2,
      "Results": [
        {"Target": "nginx:1.21", "Class": "os-pkgs", "Vulnerabilities": []}
      ]
    }"#;
    let result = parse_trivy_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_vulnerabilities_is_risky() {
    let json = r#"{
      "SchemaVersion": 2,
      "Results": [
        {
          "Target": "nginx:1.21 (debian 11.3)",
          "Class": "os-pkgs",
          "Vulnerabilities": [
            {
              "VulnerabilityID": "CVE-2021-3711",
              "PkgName": "openssl",
              "InstalledVersion": "1.1.1k-1",
              "FixedVersion": "1.1.1l-1",
              "Severity": "HIGH",
              "Title": "OpenSSL SM2 Decryption Buffer Overflow",
              "Description": "There is a buffer overflow in OpenSSL..."
            },
            {
              "VulnerabilityID": "CVE-2022-1234",
              "PkgName": "libc6",
              "InstalledVersion": "2.31-13",
              "FixedVersion": "",
              "Severity": "CRITICAL",
              "Title": "libc6 heap overflow"
            }
          ]
        }
      ]
    }"#;
    let result = parse_trivy_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].severity, Severity::High);
    assert_eq!(findings[0].scanner_name, "trivy");
    assert_eq!(findings[1].severity, Severity::Critical);
  }

  #[test]
  fn test_parse_severity_mapping() {
    assert_eq!(parse_severity("CRITICAL"), Severity::Critical);
    assert_eq!(parse_severity("HIGH"), Severity::High);
    assert_eq!(parse_severity("MEDIUM"), Severity::Medium);
    assert_eq!(parse_severity("LOW"), Severity::Low);
    assert_eq!(parse_severity("UNKNOWN"), Severity::Medium);
  }

  #[test]
  fn test_parse_invalid_json_is_error() {
    let result = parse_trivy_json("{not json}").unwrap_err();
    assert!(result.contains("invalid JSON"));
  }
}
