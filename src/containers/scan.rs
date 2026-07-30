//! Container image security scanning.
//!
//! Runs `trivy image --format json <image>` and/or `grype <image> -o json`
//! against a container image and maps the reported vulnerabilities to
//! [`SecurityFinding`]s. Uses the
//! [`ContainerExecutor`] trait so tests can mock
//! the scanner subprocess calls.

use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::containers::ContainerExecutor;
use crate::error::{ApmwError, Result};
use crate::security::{ScanResult, SecurityFinding, Severity};

/// The result of scanning a container image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContainerScanResult {
  /// The image that was scanned.
  pub image: String,
  /// The scanners that were available and ran.
  pub scanners_used: Vec<String>,
  /// The aggregated scan result (Safe, Risky, or Error).
  pub result: ScanResult,
}

/// Scans a container image for vulnerabilities using trivy and/or grype.
///
/// Checks which scanners are available on PATH, runs each available scanner
/// against the image, and aggregates the results with OR semantics: any
/// `Risky` verdict makes the overall result `Risky`.
///
/// # Errors
///
/// Returns [`ApmwError::SecurityScanFailed`] if no scanner is available.
pub async fn scan_image<E: ContainerExecutor>(
  executor: &E,
  image: &str,
) -> Result<ContainerScanResult> {
  let trivy_available = executor.is_available("trivy").await;
  let grype_available = executor.is_available("grype").await;

  debug!(
    image,
    trivy_available, grype_available, "Checking scanner availability"
  );

  if !trivy_available && !grype_available {
    return Err(ApmwError::SecurityScanFailed(
      "no container image scanner available (neither trivy nor grype found on PATH)".to_string(),
    ));
  }

  let mut scanners_used: Vec<String> = Vec::new();
  let mut all_findings: Vec<SecurityFinding> = Vec::new();
  let mut errors: Vec<String> = Vec::new();

  if trivy_available {
    scanners_used.push("trivy".to_string());
    match run_trivy(executor, image).await {
      Ok(ScanResult::Safe) => {
        info!(scanner = "trivy", image, "trivy scan complete: no findings");
      }
      Ok(ScanResult::Risky { findings }) => {
        warn!(
          scanner = "trivy",
          image,
          finding_count = findings.len(),
          "trivy scan complete: vulnerabilities found"
        );
        all_findings.extend(findings);
      }
      Ok(ScanResult::Error { message }) => {
        warn!(scanner = "trivy", image, error = %message, "trivy scan error");
        errors.push(format!("trivy: {message}"));
      }
      Err(e) => {
        warn!(scanner = "trivy", image, error = %e, "trivy scan failed to execute");
        errors.push(format!("trivy: {e}"));
      }
    }
  }

  if grype_available {
    scanners_used.push("grype".to_string());
    match run_grype(executor, image).await {
      Ok(ScanResult::Safe) => {
        info!(scanner = "grype", image, "grype scan complete: no findings");
      }
      Ok(ScanResult::Risky { findings }) => {
        warn!(
          scanner = "grype",
          image,
          finding_count = findings.len(),
          "grype scan complete: vulnerabilities found"
        );
        all_findings.extend(findings);
      }
      Ok(ScanResult::Error { message }) => {
        warn!(scanner = "grype", image, error = %message, "grype scan error");
        errors.push(format!("grype: {message}"));
      }
      Err(e) => {
        warn!(scanner = "grype", image, error = %e, "grype scan failed to execute");
        errors.push(format!("grype: {e}"));
      }
    }
  }

  // Aggregate with OR semantics: any Risky → Risky.
  let result = if !all_findings.is_empty() {
    ScanResult::Risky {
      findings: all_findings,
    }
  } else if !errors.is_empty() {
    ScanResult::Error {
      message: errors.join("; "),
    }
  } else {
    ScanResult::Safe
  };

  Ok(ContainerScanResult {
    image: image.to_string(),
    scanners_used,
    result,
  })
}

/// Runs `trivy image --format json <image>` and parses the output.
async fn run_trivy<E: ContainerExecutor>(executor: &E, image: &str) -> Result<ScanResult> {
  let output = executor
    .execute("trivy", &["image", "--format", "json", image])
    .await
    .map_err(ApmwError::Io)?;

  if !output.success && output.stdout.trim().is_empty() {
    return Ok(ScanResult::Error {
      message: format!(
        "trivy exited with code {:?}: {}",
        output.exit_code,
        output.stderr.trim()
      ),
    });
  }

  parse_trivy_json(&output.stdout)
}

/// Runs `grype <image> -o json` and parses the output.
async fn run_grype<E: ContainerExecutor>(executor: &E, image: &str) -> Result<ScanResult> {
  let output = executor
    .execute("grype", &[image, "-o", "json"])
    .await
    .map_err(ApmwError::Io)?;

  if !output.success && output.stdout.trim().is_empty() {
    return Ok(ScanResult::Error {
      message: format!(
        "grype exited with code {:?}: {}",
        output.exit_code,
        output.stderr.trim()
      ),
    });
  }

  parse_grype_json(&output.stdout)
}

// ---------------------------------------------------------------------------
// trivy JSON parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TrivyReport {
  #[serde(default, rename = "Results")]
  results: Vec<TrivyResult>,
}

#[derive(Debug, Deserialize)]
struct TrivyResult {
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

fn parse_trivy_json(stdout: &str) -> Result<ScanResult> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: TrivyReport = serde_json::from_str(stdout)
    .map_err(|e| ApmwError::SecurityScanFailed(format!("trivy parse error: {e}")))?;

  let mut findings = Vec::new();
  for result in &report.results {
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
          "{} in {} (installed: {}, fixed: {})",
          vuln.vulnerability_id, vuln.pkg_name, installed, fixed
        )
      });
      findings.push(SecurityFinding::new("trivy", severity, title, description));
    }
  }

  if findings.is_empty() {
    Ok(ScanResult::Safe)
  } else {
    Ok(ScanResult::Risky { findings })
  }
}

// ---------------------------------------------------------------------------
// grype JSON parsing
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

fn parse_grype_json(stdout: &str) -> Result<ScanResult> {
  if stdout.trim().is_empty() {
    return Ok(ScanResult::Safe);
  }
  let report: GrypeReport = serde_json::from_str(stdout)
    .map_err(|e| ApmwError::SecurityScanFailed(format!("grype parse error: {e}")))?;

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
      SecurityFinding::new("grype", severity, title, description)
    })
    .collect::<Vec<_>>();

  Ok(ScanResult::Risky { findings })
}

/// Maps severity strings (from trivy or grype) to [`Severity`].
fn parse_severity(s: &str) -> Severity {
  match s.to_ascii_lowercase().as_str() {
    "critical" => Severity::Critical,
    "high" => Severity::High,
    "medium" => Severity::Medium,
    "low" | "negligible" => Severity::Low,
    _ => Severity::Medium,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::containers::{CommandOutput, MockExecutor};

  #[test]
  fn test_parse_trivy_empty_is_safe() {
    let result = parse_trivy_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_trivy_no_vulns_is_safe() {
    let json = r#"{"Results": [{"Vulnerabilities": []}]}"#;
    let result = parse_trivy_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_trivy_with_vulns_is_risky() {
    let json = r#"{
      "Results": [
        {
          "Vulnerabilities": [
            {
              "VulnerabilityID": "CVE-2021-3711",
              "PkgName": "openssl",
              "InstalledVersion": "1.1.1k-1",
              "FixedVersion": "1.1.1l-1",
              "Severity": "HIGH",
              "Title": "OpenSSL Overflow",
              "Description": "Buffer overflow in OpenSSL"
            }
          ]
        }
      ]
    }"#;
    let result = parse_trivy_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert_eq!(findings[0].scanner_name, "trivy");
  }

  #[test]
  fn test_parse_grype_empty_is_safe() {
    let result = parse_grype_json("").unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_grype_no_matches_is_safe() {
    let json = r#"{"matches": []}"#;
    let result = parse_grype_json(json).unwrap();
    assert_eq!(result, ScanResult::Safe);
  }

  #[test]
  fn test_parse_grype_with_matches_is_risky() {
    let json = r#"{
      "matches": [
        {
          "vulnerability": {"id": "CVE-2022-9999", "severity": "Critical", "description": "RCE"},
          "artifact": {"name": "libc6", "version": "2.31-13"}
        }
      ]
    }"#;
    let result = parse_grype_json(json).unwrap();
    assert!(result.is_risky());
    let findings = result.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Critical);
    assert_eq!(findings[0].scanner_name, "grype");
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

  #[tokio::test]
  async fn test_scan_image_trivy_safe() {
    let exec = MockExecutor::new().with_available("trivy").with_output(
      "trivy",
      &["image", "--format", "json", "nginx:latest"],
      CommandOutput::success(r#"{"Results": [{"Vulnerabilities": []}]}"#),
    );
    let result = scan_image(&exec, "nginx:latest").await.unwrap();
    assert_eq!(result.image, "nginx:latest");
    assert!(result.scanners_used.contains(&"trivy".to_string()));
    assert!(result.result.is_safe());
  }

  #[tokio::test]
  async fn test_scan_image_trivy_risky() {
    let json = r#"{
      "Results": [
        {"Vulnerabilities": [
          {"VulnerabilityID": "CVE-2021-3711", "PkgName": "openssl", "Severity": "HIGH", "Title": "Overflow", "Description": "Buffer overflow"}
        ]}
      ]
    }"#;
    let exec = MockExecutor::new().with_available("trivy").with_output(
      "trivy",
      &["image", "--format", "json", "nginx:latest"],
      CommandOutput::success(json),
    );
    let result = scan_image(&exec, "nginx:latest").await.unwrap();
    assert!(result.result.is_risky());
    assert_eq!(result.result.findings().len(), 1);
  }

  #[tokio::test]
  async fn test_scan_image_grype_safe() {
    let exec = MockExecutor::new().with_available("grype").with_output(
      "grype",
      &["nginx:latest", "-o", "json"],
      CommandOutput::success(r#"{"matches": []}"#),
    );
    let result = scan_image(&exec, "nginx:latest").await.unwrap();
    assert!(result.scanners_used.contains(&"grype".to_string()));
    assert!(result.result.is_safe());
  }

  #[tokio::test]
  async fn test_scan_image_both_scanners_or_semantics() {
    let trivy_json = r#"{"Results": [{"Vulnerabilities": []}]}"#;
    let grype_json = r#"{
      "matches": [
        {"vulnerability": {"id": "CVE-2022-9999", "severity": "Critical", "description": "RCE"}, "artifact": {"name": "libc6", "version": "2.31"}}
      ]
    }"#;
    let exec = MockExecutor::new()
      .with_available("trivy")
      .with_available("grype")
      .with_output(
        "trivy",
        &["image", "--format", "json", "nginx:latest"],
        CommandOutput::success(trivy_json),
      )
      .with_output(
        "grype",
        &["nginx:latest", "-o", "json"],
        CommandOutput::success(grype_json),
      );
    let result = scan_image(&exec, "nginx:latest").await.unwrap();
    assert!(result.result.is_risky());
    assert_eq!(result.scanners_used.len(), 2);
    assert_eq!(result.result.findings().len(), 1);
  }

  #[tokio::test]
  async fn test_scan_image_no_scanner_available() {
    let exec = MockExecutor::new();
    let result = scan_image(&exec, "nginx:latest").await;
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("no container image scanner available"));
  }

  #[tokio::test]
  async fn test_scan_container_via_engine() {
    let exec = MockExecutor::new().with_available("trivy").with_output(
      "trivy",
      &["image", "--format", "json", "alpine:3.18"],
      CommandOutput::success(r#"{"Results": []}"#),
    );
    let engine = crate::containers::ContainerEngine::new(exec);
    let result = engine.scan("alpine:3.18").await.unwrap();
    assert_eq!(result.image, "alpine:3.18");
    assert!(result.result.is_safe());
  }
}
