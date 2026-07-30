//! grype container image scanner plugin.
//!
//! Runs `grype <image> -o json` against a container image and maps the
//! reported vulnerabilities to [`SecurityFinding`]s. Implements the
//! [`Scanner`] trait from [`crate::security::scanner`].
//!
//! Because the [`Scanner::scan`] trait method receives a `&Path`, the path is
//! interpreted as a container image reference (e.g. `nginx:1.21`). Callers
//! that scan directories should use the directory-based scanners; grype is for
//! container images only.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::error::{ApmwError, Result};
use crate::security::scanner::{ScanResult, Scanner, SecurityFinding, Severity};

/// Scanner plugin that wraps `grype` for container image scanning.
#[derive(Debug, Default, Clone)]
pub struct Grype;

impl Grype {
  /// Creates a new `grype` scanner plugin.
  pub fn new() -> Self {
    Self
  }

  /// The binary name to look for on `PATH`.
  const BINARY: &'static str = "grype";
}

impl Scanner for Grype {
  fn name(&self) -> &str {
    "grype"
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
    // grype is best installed via package manager or install script. We try
    // brew first, then surface an error with instructions.
    #[cfg(target_os = "macos")]
    {
      if which::which("brew").is_ok() {
        debug!(scanner = self.name(), "installing via brew install grype");
        let status = Command::new("brew")
          .args(["install", "grype"])
          .status()
          .map_err(|e| ApmwError::SecurityScanFailed(format!("grype install: {e}")))?;
        if status.success() {
          return Ok(());
        }
      }
    }
    Err(ApmwError::SecurityScanFailed(
      "grype not found and could not be installed automatically; \
       install grype manually (see https://github.com/anchore/grype)"
        .to_string(),
    ))
  }

  fn scan(&self, target: &Path) -> ScanResult {
    // The path is interpreted as a container image reference.
    let image = target.to_string_lossy();
    debug!(scanner = self.name(), image = %image, "running grype -o json");
    let output = Command::new("grype")
      .args([image.as_ref(), "-o", "json"])
      .output();

    let output = match output {
      Ok(o) => o,
      Err(e) => {
        error!(scanner = self.name(), error = %e, "failed to spawn grype");
        return ScanResult::Error {
          message: format!("failed to spawn grype: {e}"),
        };
      }
    };

    // grype may exit non-zero when vulnerabilities are found, but JSON is
    // still on stdout. Parse stdout regardless of exit code.
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_grype_json(&stdout).unwrap_or_else(|e| {
      error!(scanner = self.name(), error = %e, "failed to parse grype output");
      ScanResult::Error {
        message: format!("grype parse error: {e}"),
      }
    })
  }
}

// ---------------------------------------------------------------------------
// JSON schema (only the fields we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GrypeReport {
  #[serde(default)]
  matches: Vec<GrypeMatch>,
}

#[derive(Debug, Deserialize)]
struct GrypeMatch {
  vulnerability: GrypeVulnerability,
  artifact: GrypeArtifact,
}

#[derive(Debug, Deserialize)]
struct GrypeVulnerability {
  id: String,
  severity: String,
  #[serde(default)]
  description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GrypeArtifact {
  name: String,
  #[serde(default)]
  version: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parses the JSON stdout of `grype <image> -o json` into a [`ScanResult`].
fn parse_grype_json(stdout: &str) -> std::result::Result<ScanResult, String> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: GrypeReport =
    serde_json::from_str(stdout).map_err(|e| format!("invalid JSON: {e}"))?;

  if report.matches.is_empty() {
    return Ok(ScanResult::Safe);
  }

  let findings = report
    .matches
    .into_iter()
    .map(|m| {
      let severity = parse_severity(&m.vulnerability.severity);
      let version = m.artifact.version.as_deref().unwrap_or("?");
      let title = format!("{} in {}@{}", m.vulnerability.id, m.artifact.name, version);
      let description = m
        .vulnerability
        .description
        .unwrap_or_else(|| format!("{} in {}@{}", m.vulnerability.id, m.artifact.name, version));
      warn!(
        scanner = "grype",
        advisory = %m.vulnerability.id,
        package = %m.artifact.name,
        severity = %severity,
        "vulnerability found"
      );
      SecurityFinding::new("grype", severity, title, description)
    })
    .collect::<Vec<_>>();

  Ok(ScanResult::Risky { findings })
}

/// Maps grype severity strings to [`Severity`].
///
/// grype uses severity labels like "Critical", "High", "Medium", "Low",
/// "Negligible", and "Unknown".
fn parse_severity(s: &str) -> Severity {
  match s.to_ascii_lowercase().as_str() {
    "critical" => Severity::Critical,
    "high" => Severity::High,
    "medium" => Severity::Medium,
    "low" | "negligible" => Severity::Low,
    // grype may report "unknown" — default to Medium.
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
    let s = Grype::new();
    assert_eq!(s.name(), "grype");
  }

  #[test]
  fn test_parse_empty_output_is_safe() {
    let result = parse_grype_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_no_matches_is_safe() {
    let json = r#"{"matches": []}"#;
    let result = parse_grype_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_matches_is_risky() {
    let json = r#"{
      "matches": [
        {
          "vulnerability": {
            "id": "CVE-2021-3711",
            "severity": "High",
            "description": "OpenSSL SM2 Decryption Buffer Overflow"
          },
          "artifact": {
            "name": "openssl",
            "version": "1.1.1k-1"
          }
        },
        {
          "vulnerability": {
            "id": "CVE-2022-9999",
            "severity": "Critical",
            "description": "RCE in libc"
          },
          "artifact": {
            "name": "libc6",
            "version": "2.31-13"
          }
        }
      ]
    }"#;
    let result = parse_grype_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].severity, Severity::High);
    assert_eq!(findings[0].scanner_name, "grype");
    assert!(findings[0].title.contains("CVE-2021-3711"));
    assert_eq!(findings[1].severity, Severity::Critical);
  }

  #[test]
  fn test_parse_severity_mapping() {
    assert_eq!(parse_severity("Critical"), Severity::Critical);
    assert_eq!(parse_severity("High"), Severity::High);
    assert_eq!(parse_severity("Medium"), Severity::Medium);
    assert_eq!(parse_severity("Low"), Severity::Low);
    assert_eq!(parse_severity("Negligible"), Severity::Low);
    assert_eq!(parse_severity("Unknown"), Severity::Medium);
  }

  #[test]
  fn test_parse_invalid_json_is_error() {
    let result = parse_grype_json("{not json}").unwrap_err();
    assert!(result.contains("invalid JSON"));
  }
}
