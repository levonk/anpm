//! Package manager detection engine.
//!
//! Scans the current project directory for lockfiles, config files, and
//! directory markers to identify the correct package manager. Detection
//! attributes are derived from the 2ndbrain research Table 1 (Package Manager
//! Project Detection Attributes).
//!
//! ## Confidence scoring
//!
//! Multiple matching files = higher confidence. Each primary file match
//! contributes a higher weight than a secondary file or directory marker.
//! The confidence is a normalized score in the range `[0.0, 1.0]`.
//!
//! ## Performance
//!
//! Detection scans only the top-level of the given directory (no recursion),
//! keeping it fast — under 100ms per PRD NFR-1.1.

pub mod attributes;
pub mod managers;

pub use attributes::{
  all_attributes, dir_marker_index, primary_file_index, secondary_file_index, DetectionAttributes,
  DetectionEvidence, EvidenceKind,
};
pub use managers::{
  all_managers, find_manager, Ecosystem, HierarchyLevel, PackageManager, PackageManagerOwned,
};

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{CoreError, Result};

/// Weight assigned to a primary file match in confidence scoring.
const WEIGHT_PRIMARY: f64 = 1.0;
/// Weight assigned to a secondary file match in confidence scoring.
const WEIGHT_SECONDARY: f64 = 0.5;
/// Weight assigned to a directory marker match in confidence scoring.
const WEIGHT_DIRECTORY: f64 = 0.3;

/// The result of a package manager detection scan.
///
/// Contains the detected manager, confidence score, and the evidence (file
/// paths that triggered the detection). Serialized with serde for TOON/JSON
/// output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionResult {
  /// The canonical name of the detected package manager.
  pub manager: String,
  /// Human-readable display name.
  pub display_name: String,
  /// The ecosystem this manager belongs to.
  pub ecosystem: String,
  /// The hierarchy level of this manager.
  pub hierarchy: String,
  /// Confidence score in the range `[0.0, 1.0]`.
  pub confidence: f64,
  /// Evidence — file and directory paths that triggered the detection.
  pub evidence: Vec<EvidenceEntry>,
}

/// A single piece of evidence in a detection result, serializable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceEntry {
  /// The file or directory path relative to the scanned directory.
  pub path: String,
  /// The kind of evidence: primary, secondary, or directory.
  pub kind: String,
}

impl From<&DetectionEvidence> for EvidenceEntry {
  fn from(e: &DetectionEvidence) -> Self {
    EvidenceEntry {
      path: e.path.clone(),
      kind: e.kind.to_string(),
    }
  }
}

impl DetectionResult {
  /// Create a synthetic detection result from a forced manager name.
  ///
  /// Used when `--manager <name>` is passed on the CLI to skip auto-detection.
  /// Looks up the manager in the registry for display name, ecosystem, and
  /// hierarchy. If the manager is not in the detection registry (e.g. `bun`,
  /// `dnf`, `pacman`, `snap`, `flatpak`, `podman`), sensible defaults are used.
  pub fn from_forced_manager(name: &str) -> Self {
    let (display_name, ecosystem, hierarchy) = match find_manager(name) {
      Some(m) => (
        m.display_name.to_string(),
        m.ecosystem.to_string(),
        m.hierarchy.to_string(),
      ),
      None => {
        // Managers not in the detection registry get default values.
        let ecosystem = ecosystem_for_manager(name);
        (
          name.to_string(),
          ecosystem.to_string(),
          "language".to_string(),
        )
      }
    };

    tracing::info!(
      manager = name,
      "Creating forced detection result (source: cli-override)"
    );

    DetectionResult {
      manager: name.to_string(),
      display_name,
      ecosystem,
      hierarchy,
      confidence: 1.0,
      evidence: vec![EvidenceEntry {
        path: "--manager CLI override".to_string(),
        kind: "cli-override".to_string(),
      }],
    }
  }
}

/// Determine the ecosystem for a manager that is not in the detection registry.
///
/// This covers managers like `bun`, `dnf`, `pacman`, `snap`, `flatpak`, and
/// `podman` which are valid `--manager` overrides but not in the detection
/// engine's `MANAGERS` array.
fn ecosystem_for_manager(name: &str) -> Ecosystem {
  match name {
    "bun" => Ecosystem::Node,
    "dnf" | "pacman" | "snap" | "flatpak" => Ecosystem::Os,
    "podman" => Ecosystem::Container,
    _ => Ecosystem::Unknown,
  }
}

/// The detection engine — scans a directory and returns detection results.
#[derive(Debug, Clone)]
pub struct DetectionEngine;

impl DetectionEngine {
  /// Create a new detection engine.
  pub fn new() -> Self {
    DetectionEngine
  }

  /// Detect package managers in the given directory.
  ///
  /// Returns all detected managers sorted by confidence (highest first).
  /// The caller decides which manager to use based on the confidence scores.
  pub fn detect(&self, dir: &Path) -> Result<Vec<DetectionResult>> {
    self.detect_with_custom(dir, &[])
  }

  /// Detect package managers in the given directory, including custom types.
  ///
  /// Custom types are merged with the built-in [`PackageManager`] registry:
  /// a custom type with the same name as a built-in replaces the built-in.
  /// Custom types with unique names are added as additional detection
  /// candidates.
  ///
  /// Returns all detected managers sorted by confidence (highest first).
  pub fn detect_with_custom(
    &self,
    dir: &Path,
    custom: &[crate::custom_types::CustomProjectType],
  ) -> Result<Vec<DetectionResult>> {
    let start = std::time::Instant::now();

    if !dir.exists() {
      return Err(CoreError::PackageManagerNotFound(format!(
        "directory does not exist: {}",
        dir.display()
      )));
    }

    let entries = match std::fs::read_dir(dir) {
      Ok(e) => e,
      Err(err) => {
        warn!(error = %err, dir = %dir.display(), "Failed to read directory for detection");
        return Err(CoreError::Io(err));
      }
    };

    let mut file_names: Vec<String> = Vec::new();
    let mut dir_names: Vec<String> = Vec::new();

    for entry in entries {
      let entry = match entry {
        Ok(e) => e,
        Err(err) => {
          warn!(error = %err, "Failed to read directory entry");
          continue;
        }
      };
      let name = entry.file_name().to_string_lossy().to_string();
      if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
        dir_names.push(name);
      } else {
        file_names.push(name);
      }
    }

    // Build the merged candidate list: built-ins (minus overridden) + custom.
    let candidates = crate::custom_types::merge_with_builtins(custom, all_managers());

    let mut results: Vec<DetectionResult> = Vec::new();

    for manager in &candidates {
      let mut evidence: Vec<DetectionEvidence> = Vec::new();
      let mut score: f64 = 0.0;

      for pattern in &manager.primary_files {
        for name in &file_names {
          if attributes::matches_glob(pattern, name) {
            evidence.push(DetectionEvidence {
              path: name.clone(),
              kind: EvidenceKind::Primary,
              manager_name: "",
            });
            score += WEIGHT_PRIMARY;
            break;
          }
        }
      }

      for pattern in &manager.secondary_files {
        for name in &file_names {
          if attributes::matches_glob(pattern, name) {
            evidence.push(DetectionEvidence {
              path: name.clone(),
              kind: EvidenceKind::Secondary,
              manager_name: "",
            });
            score += WEIGHT_SECONDARY;
            break;
          }
        }
      }

      for marker in &manager.dir_markers {
        if dir_names.iter().any(|d| d == marker) {
          evidence.push(DetectionEvidence {
            path: marker.clone(),
            kind: EvidenceKind::Directory,
            manager_name: "",
          });
          score += WEIGHT_DIRECTORY;
        }
      }

      if !evidence.is_empty() {
        let max_possible = (manager.primary_files.len() as f64 * WEIGHT_PRIMARY)
          + (manager.secondary_files.len() as f64 * WEIGHT_SECONDARY)
          + (manager.dir_markers.len() as f64 * WEIGHT_DIRECTORY);
        let confidence = if max_possible > 0.0 {
          (score / max_possible).min(1.0)
        } else {
          0.0
        };

        debug!(
          manager = %manager.name,
          score,
          max_possible,
          confidence,
          evidence_count = evidence.len(),
          "Detected package manager"
        );

        results.push(DetectionResult {
          manager: manager.name.clone(),
          display_name: manager.display_name.clone(),
          ecosystem: manager.ecosystem.to_string(),
          hierarchy: manager.hierarchy.to_string(),
          confidence,
          evidence: evidence.iter().map(EvidenceEntry::from).collect(),
        });
      }
    }

    // Sort by confidence (descending), then by priority (descending).
    let priority_map: HashMap<String, i32> = candidates
      .iter()
      .map(|m| (m.name.clone(), m.priority))
      .collect();
    results.sort_by(|a, b| {
      b.confidence
        .partial_cmp(&a.confidence)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
          priority_map
            .get(b.manager.as_str())
            .cmp(&priority_map.get(a.manager.as_str()))
        })
    });

    let elapsed = start.elapsed();
    info!(
      results_count = results.len(),
      elapsed_ms = elapsed.as_millis(),
      "Detection complete"
    );

    Ok(results)
  }

  /// Detect the single best (highest-confidence) package manager.
  ///
  /// Returns `None` if no managers were detected.
  pub fn detect_best(&self, dir: &Path) -> Result<Option<DetectionResult>> {
    let results = self.detect(dir)?;
    Ok(results.into_iter().next())
  }
}

impl Default for DetectionEngine {
  fn default() -> Self {
    DetectionEngine::new()
  }
}

/// Convenience function to detect the current directory.
pub fn detect_current_dir() -> Result<Vec<DetectionResult>> {
  let engine = DetectionEngine::new();
  let dir = std::env::current_dir()
    .map_err(|e| CoreError::PackageManagerNotFound(format!("cannot get current dir: {e}")))?;
  engine.detect(&dir)
}

/// Convenience function to detect in a specific directory.
pub fn detect_in(dir: &Path) -> Result<Vec<DetectionResult>> {
  let engine = DetectionEngine::new();
  engine.detect(dir)
}

/// Convenience function to detect in a specific directory with custom types.
pub fn detect_in_with_custom(
  dir: &Path,
  custom: &[crate::custom_types::CustomProjectType],
) -> Result<Vec<DetectionResult>> {
  let engine = DetectionEngine::new();
  engine.detect_with_custom(dir, custom)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::io::Write;
  use tempfile::TempDir;

  fn make_project(files: &[&str]) -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    for file in files {
      let path = dir.path().join(file);
      if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir");
      }
      fs::File::create(&path)
        .and_then(|mut f| f.write_all(b"// test"))
        .expect("failed to write file");
    }
    dir
  }

  fn make_project_with_dirs(files: &[&str], dirs: &[&str]) -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    for file in files {
      let path = dir.path().join(file);
      fs::File::create(&path)
        .and_then(|mut f| f.write_all(b"// test"))
        .expect("failed to write file");
    }
    for d in dirs {
      fs::create_dir_all(dir.path().join(d)).expect("failed to create dir");
    }
    dir
  }

  #[test]
  fn test_detect_cargo() {
    let dir = make_project(&["Cargo.toml", "Cargo.lock"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let cargo = results
      .iter()
      .find(|r| r.manager == "cargo")
      .expect("cargo should be detected");
    assert!(cargo.confidence > 0.0);
    assert!(cargo.evidence.iter().any(|e| e.path == "Cargo.toml"));
    assert!(cargo.evidence.iter().any(|e| e.path == "Cargo.lock"));
  }

  #[test]
  fn test_detect_pnpm() {
    let dir = make_project(&["pnpm-lock.yaml", "package.json"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let pnpm = results
      .iter()
      .find(|r| r.manager == "pnpm")
      .expect("pnpm should be detected");
    assert!(pnpm.confidence > 0.0);
    assert!(pnpm.evidence.iter().any(|e| e.path == "pnpm-lock.yaml"));
  }

  #[test]
  fn test_detect_npm() {
    let dir = make_project(&["package-lock.json", "package.json"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let npm = results
      .iter()
      .find(|r| r.manager == "npm")
      .expect("npm should be detected");
    assert!(npm.confidence > 0.0);
    assert!(npm.evidence.iter().any(|e| e.path == "package-lock.json"));
  }

  #[test]
  fn test_detect_pip() {
    let dir = make_project(&["requirements.txt"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let pip = results
      .iter()
      .find(|r| r.manager == "pip")
      .expect("pip should be detected");
    assert!(pip.confidence > 0.0);
    assert!(pip.evidence.iter().any(|e| e.path == "requirements.txt"));
  }

  #[test]
  fn test_detect_poetry() {
    let dir = make_project(&["poetry.lock"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let poetry = results
      .iter()
      .find(|r| r.manager == "poetry")
      .expect("poetry should be detected");
    assert!(poetry.confidence > 0.0);
  }

  #[test]
  fn test_detect_go() {
    let dir = make_project(&["go.mod", "go.sum"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let go = results
      .iter()
      .find(|r| r.manager == "go")
      .expect("go should be detected");
    assert!(go.confidence > 0.0);
    assert!(go.evidence.iter().any(|e| e.path == "go.mod"));
  }

  #[test]
  fn test_detect_gem() {
    let dir = make_project(&["Gemfile", "Gemfile.lock"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let gem = results
      .iter()
      .find(|r| r.manager == "gem")
      .expect("gem should be detected");
    assert!(gem.confidence > 0.0);
  }

  #[test]
  fn test_detect_maven() {
    let dir = make_project(&["pom.xml"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let maven = results
      .iter()
      .find(|r| r.manager == "maven")
      .expect("maven should be detected");
    assert!(maven.confidence > 0.0);
  }

  #[test]
  fn test_detect_gradle() {
    let dir = make_project(&["build.gradle", "settings.gradle"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let gradle = results
      .iter()
      .find(|r| r.manager == "gradle")
      .expect("gradle should be detected");
    assert!(gradle.confidence > 0.0);
  }

  #[test]
  fn test_detect_docker() {
    let dir = make_project(&["Dockerfile"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let docker = results
      .iter()
      .find(|r| r.manager == "docker")
      .expect("docker should be detected");
    assert!(docker.confidence > 0.0);
  }

  #[test]
  fn test_detect_dotnet() {
    let dir = make_project(&["MyApp.csproj"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let dotnet = results
      .iter()
      .find(|r| r.manager == "dotnet")
      .expect("dotnet should be detected");
    assert!(dotnet.confidence > 0.0);
    assert!(dotnet.evidence.iter().any(|e| e.path == "MyApp.csproj"));
  }

  #[test]
  fn test_detect_helm() {
    let dir = make_project(&["Chart.yaml"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let helm = results
      .iter()
      .find(|r| r.manager == "helm")
      .expect("helm should be detected");
    assert!(helm.confidence > 0.0);
  }

  #[test]
  fn test_detect_empty_dir() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    assert!(results.is_empty(), "empty dir should detect nothing");
  }

  #[test]
  fn test_detect_nonexistent_dir() {
    let engine = DetectionEngine::new();
    let result = engine.detect(Path::new("/nonexistent/path/that/does/not/exist"));
    assert!(result.is_err());
  }

  #[test]
  fn test_detect_multiple_managers() {
    let dir = make_project(&["Cargo.toml", "package.json", "package-lock.json"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let managers: Vec<&str> = results.iter().map(|r| r.manager.as_str()).collect();
    assert!(managers.contains(&"cargo"));
    assert!(managers.contains(&"npm"));
  }

  #[test]
  fn test_confidence_higher_with_more_evidence() {
    let dir1 = make_project(&["Cargo.toml"]);
    let dir2 = make_project_with_dirs(&["Cargo.toml", "Cargo.lock"], &["target"]);
    let engine = DetectionEngine::new();
    let results1 = engine.detect(dir1.path()).expect("detection failed");
    let results2 = engine.detect(dir2.path()).expect("detection failed");
    let cargo1 = results1
      .iter()
      .find(|r| r.manager == "cargo")
      .expect("cargo should be detected in dir1");
    let cargo2 = results2
      .iter()
      .find(|r| r.manager == "cargo")
      .expect("cargo should be detected in dir2");
    assert!(
      cargo2.confidence > cargo1.confidence,
      "more evidence should yield higher confidence: {} > {}",
      cargo2.confidence,
      cargo1.confidence
    );
  }

  #[test]
  fn test_results_sorted_by_confidence() {
    let dir = make_project(&[
      "Cargo.toml",
      "Cargo.lock",
      "package.json",
      "package-lock.json",
    ]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    for window in results.windows(2) {
      assert!(
        window[0].confidence >= window[1].confidence,
        "results must be sorted by confidence descending: {} >= {}",
        window[0].confidence,
        window[1].confidence
      );
    }
  }

  #[test]
  fn test_detect_best() {
    let dir = make_project(&["Cargo.toml", "Cargo.lock"]);
    let engine = DetectionEngine::new();
    let best = engine
      .detect_best(dir.path())
      .expect("detection failed")
      .expect("should detect at least one manager");
    assert_eq!(best.manager, "cargo");
  }

  #[test]
  fn test_detect_best_empty() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let engine = DetectionEngine::new();
    let best = engine.detect_best(dir.path()).expect("detection failed");
    assert!(best.is_none());
  }

  #[test]
  fn test_detection_result_serde() {
    let result = DetectionResult {
      manager: "cargo".to_string(),
      display_name: "Cargo".to_string(),
      ecosystem: "rust".to_string(),
      hierarchy: "language".to_string(),
      confidence: 0.8,
      evidence: vec![EvidenceEntry {
        path: "Cargo.toml".to_string(),
        kind: "primary".to_string(),
      }],
    };
    let json = serde_json::to_string(&result).expect("serialize failed");
    let deserialized: DetectionResult = serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(result, deserialized);
  }

  #[test]
  fn test_evidence_entry_from_detection_evidence() {
    let evidence = DetectionEvidence {
      path: "Cargo.toml".to_string(),
      kind: EvidenceKind::Primary,
      manager_name: "cargo",
    };
    let entry = EvidenceEntry::from(&evidence);
    assert_eq!(entry.path, "Cargo.toml");
    assert_eq!(entry.kind, "primary");
  }

  #[test]
  fn test_detect_performance_under_100ms() {
    let dir = make_project(&[
      "Cargo.toml",
      "Cargo.lock",
      "package.json",
      "package-lock.json",
      "go.mod",
      "go.sum",
      "requirements.txt",
      "Dockerfile",
      "pom.xml",
      "build.gradle",
    ]);
    let engine = DetectionEngine::new();
    let start = std::time::Instant::now();
    let _results = engine.detect(dir.path()).expect("detection failed");
    let elapsed = start.elapsed();
    assert!(
      elapsed.as_millis() < 100,
      "detection should be under 100ms, took {}ms",
      elapsed.as_millis()
    );
  }

  #[test]
  fn test_detect_with_node_modules_dir() {
    let dir = make_project_with_dirs(&["package.json", "package-lock.json"], &["node_modules"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let npm = results
      .iter()
      .find(|r| r.manager == "npm")
      .expect("npm should be detected");
    assert!(npm
      .evidence
      .iter()
      .any(|e| e.path == "node_modules" && e.kind == "directory"));
  }

  #[test]
  fn test_detect_pyproject_poetry() {
    let dir = make_project(&["poetry.lock", "pyproject.toml"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let poetry = results
      .iter()
      .find(|r| r.manager == "poetry")
      .expect("poetry should be detected");
    assert!(poetry.confidence > 0.0);
  }

  #[test]
  fn test_detect_uv() {
    let dir = make_project(&["uv.lock"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let uv = results
      .iter()
      .find(|r| r.manager == "uv")
      .expect("uv should be detected");
    assert!(uv.confidence > 0.0);
  }

  #[test]
  fn test_detect_pipenv() {
    let dir = make_project(&["Pipfile", "Pipfile.lock"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let pipenv = results
      .iter()
      .find(|r| r.manager == "pipenv")
      .expect("pipenv should be detected");
    assert!(pipenv.confidence > 0.0);
  }

  #[test]
  fn test_detect_conda() {
    let dir = make_project(&["environment.yml"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let conda = results
      .iter()
      .find(|r| r.manager == "conda")
      .expect("conda should be detected");
    assert!(conda.confidence > 0.0);
  }

  #[test]
  fn test_detect_sbt() {
    let dir = make_project(&["build.sbt"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let sbt = results
      .iter()
      .find(|r| r.manager == "sbt")
      .expect("sbt should be detected");
    assert!(sbt.confidence > 0.0);
  }

  #[test]
  fn test_detect_flutter() {
    let dir = make_project(&["pubspec.yaml"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let flutter = results
      .iter()
      .find(|r| r.manager == "flutter")
      .expect("flutter should be detected");
    assert!(flutter.confidence > 0.0);
  }

  #[test]
  fn test_detect_cocoapods() {
    let dir = make_project(&["Podfile"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let cocoapods = results
      .iter()
      .find(|r| r.manager == "cocoapods")
      .expect("cocoapods should be detected");
    assert!(cocoapods.confidence > 0.0);
  }

  #[test]
  fn test_detect_carthage() {
    let dir = make_project(&["Cartfile"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let carthage = results
      .iter()
      .find(|r| r.manager == "carthage")
      .expect("carthage should be detected");
    assert!(carthage.confidence > 0.0);
  }

  #[test]
  fn test_detect_spm() {
    let dir = make_project(&["Package.swift"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let spm = results
      .iter()
      .find(|r| r.manager == "spm")
      .expect("spm should be detected");
    assert!(spm.confidence > 0.0);
  }

  #[test]
  fn test_detect_brew() {
    let dir = make_project(&["Brewfile"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let brew = results
      .iter()
      .find(|r| r.manager == "brew")
      .expect("brew should be detected");
    assert!(brew.confidence > 0.0);
  }

  #[test]
  fn test_detect_nix() {
    let dir = make_project(&["flake.nix"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let nix = results
      .iter()
      .find(|r| r.manager == "nix")
      .expect("nix should be detected");
    assert!(nix.confidence > 0.0);
  }

  #[test]
  fn test_detect_devbox() {
    let dir = make_project(&["devbox.json"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let devbox = results
      .iter()
      .find(|r| r.manager == "devbox")
      .expect("devbox should be detected");
    assert!(devbox.confidence > 0.0);
  }

  #[test]
  fn test_detect_vagrant() {
    let dir = make_project(&["Vagrantfile"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let vagrant = results
      .iter()
      .find(|r| r.manager == "vagrant")
      .expect("vagrant should be detected");
    assert!(vagrant.confidence > 0.0);
  }

  #[test]
  fn test_detect_cmake() {
    let dir = make_project(&["CMakeLists.txt"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let cmake = results
      .iter()
      .find(|r| r.manager == "cmake")
      .expect("cmake should be detected");
    assert!(cmake.confidence > 0.0);
  }

  #[test]
  fn test_detect_compose_yaml() {
    let dir = make_project(&["compose.yaml"]);
    let engine = DetectionEngine::new();
    let results = engine.detect(dir.path()).expect("detection failed");
    let docker = results
      .iter()
      .find(|r| r.manager == "docker")
      .expect("docker should be detected from compose.yaml");
    assert!(docker.confidence > 0.0);
  }

  // --- Manager override tests (story 02-004) ---

  #[test]
  fn test_from_forced_manager_pnpm() {
    let result = DetectionResult::from_forced_manager("pnpm");
    assert_eq!(result.manager, "pnpm");
    assert_eq!(result.ecosystem, "node");
    assert_eq!(result.confidence, 1.0);
    assert!(result.evidence.iter().any(|e| e.kind == "cli-override"));
  }

  #[test]
  fn test_from_forced_manager_uv() {
    let result = DetectionResult::from_forced_manager("uv");
    assert_eq!(result.manager, "uv");
    assert_eq!(result.ecosystem, "python");
    assert_eq!(result.confidence, 1.0);
  }

  #[test]
  fn test_from_forced_manager_cargo() {
    let result = DetectionResult::from_forced_manager("cargo");
    assert_eq!(result.manager, "cargo");
    assert_eq!(result.ecosystem, "rust");
    assert_eq!(result.confidence, 1.0);
  }

  #[test]
  fn test_from_forced_manager_docker() {
    let result = DetectionResult::from_forced_manager("docker");
    assert_eq!(result.manager, "docker");
    assert_eq!(result.ecosystem, "container");
    assert_eq!(result.confidence, 1.0);
  }

  #[test]
  fn test_from_forced_manager_bun_not_in_registry() {
    // bun is a valid --manager value but not in the detection registry.
    let result = DetectionResult::from_forced_manager("bun");
    assert_eq!(result.manager, "bun");
    assert_eq!(result.ecosystem, "node");
    assert_eq!(result.confidence, 1.0);
  }

  #[test]
  fn test_from_forced_manager_podman_not_in_registry() {
    let result = DetectionResult::from_forced_manager("podman");
    assert_eq!(result.manager, "podman");
    assert_eq!(result.ecosystem, "container");
    assert_eq!(result.confidence, 1.0);
  }

  #[test]
  fn test_from_forced_manager_dnf_not_in_registry() {
    let result = DetectionResult::from_forced_manager("dnf");
    assert_eq!(result.manager, "dnf");
    assert_eq!(result.ecosystem, "os");
    assert_eq!(result.confidence, 1.0);
  }

  #[test]
  fn test_from_forced_manager_has_cli_override_evidence() {
    let result = DetectionResult::from_forced_manager("pnpm");
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(result.evidence[0].path, "--manager CLI override");
    assert_eq!(result.evidence[0].kind, "cli-override");
  }

  #[test]
  fn test_from_forced_manager_skips_detection() {
    // Verify that from_forced_manager produces a result without scanning a dir.
    let result = DetectionResult::from_forced_manager("pnpm");
    // The forced result should have confidence 1.0 (max), unlike real detection.
    assert_eq!(result.confidence, 1.0);
    // No file-based evidence — only the CLI override marker.
    assert!(result.evidence.iter().all(|e| e.kind == "cli-override"));
  }

  // --- Custom type detection tests (story 08-002) ---

  #[test]
  fn test_detect_custom_type() {
    use crate::custom_types::CustomProjectType;

    let dir = make_project(&["shader.wgsl"]);
    let engine = DetectionEngine::new();
    let custom = vec![CustomProjectType {
      name: "wgsl".to_string(),
      display_name: "WGSL Shaders".to_string(),
      ecosystem: Ecosystem::Unknown,
      hierarchy: HierarchyLevel::BuildSystem,
      primary_files: vec!["*.wgsl".to_string()],
      secondary_files: vec![],
      dir_markers: vec![],
      priority: 50,
    }];
    let results = engine
      .detect_with_custom(dir.path(), &custom)
      .expect("detection failed");
    let wgsl = results
      .iter()
      .find(|r| r.manager == "wgsl")
      .expect("wgsl should be detected");
    assert!(wgsl.confidence > 0.0);
    assert!(wgsl.evidence.iter().any(|e| e.path == "shader.wgsl"));
  }

  #[test]
  fn test_detect_custom_type_overrides_builtin() {
    use crate::custom_types::CustomProjectType;

    let dir = make_project(&["Cargo.toml", "Cargo-custom.toml"]);
    let engine = DetectionEngine::new();

    // Custom cargo with an extra primary file.
    let custom = vec![CustomProjectType {
      name: "cargo".to_string(),
      display_name: "Custom Cargo".to_string(),
      ecosystem: Ecosystem::Rust,
      hierarchy: HierarchyLevel::Language,
      primary_files: vec!["Cargo.toml".to_string(), "Cargo-custom.toml".to_string()],
      secondary_files: vec![],
      dir_markers: vec![],
      priority: 100,
    }];

    let results_builtin = engine.detect(dir.path()).expect("detection failed");
    let results_custom = engine
      .detect_with_custom(dir.path(), &custom)
      .expect("detection failed");

    // With the override, cargo should have a different display name.
    let cargo_custom = results_custom
      .iter()
      .find(|r| r.manager == "cargo")
      .expect("cargo should be detected");
    assert_eq!(cargo_custom.display_name, "Custom Cargo");

    // The custom cargo should have higher confidence (2 primary files matched).
    let cargo_builtin = results_builtin
      .iter()
      .find(|r| r.manager == "cargo")
      .expect("cargo should be detected");
    assert!(
      cargo_custom.confidence > cargo_builtin.confidence,
      "custom cargo with more evidence should have higher confidence: {} > {}",
      cargo_custom.confidence,
      cargo_builtin.confidence
    );
  }

  #[test]
  fn test_detect_with_custom_empty_is_same_as_detect() {
    let dir = make_project(&["Cargo.toml", "Cargo.lock"]);
    let engine = DetectionEngine::new();

    let results_default = engine.detect(dir.path()).expect("detection failed");
    let results_empty_custom = engine
      .detect_with_custom(dir.path(), &[])
      .expect("detection failed");

    assert_eq!(results_default, results_empty_custom);
  }

  #[test]
  fn test_detect_with_custom_dir_marker() {
    use crate::custom_types::CustomProjectType;

    let dir = make_project_with_dirs(&["wgsl.toml"], &["shaders"]);
    let engine = DetectionEngine::new();
    let custom = vec![CustomProjectType {
      name: "wgsl".to_string(),
      display_name: "WGSL Shaders".to_string(),
      ecosystem: Ecosystem::Unknown,
      hierarchy: HierarchyLevel::BuildSystem,
      primary_files: vec![],
      secondary_files: vec!["wgsl.toml".to_string()],
      dir_markers: vec!["shaders".to_string()],
      priority: 50,
    }];
    let results = engine
      .detect_with_custom(dir.path(), &custom)
      .expect("detection failed");
    let wgsl = results
      .iter()
      .find(|r| r.manager == "wgsl")
      .expect("wgsl should be detected");
    assert!(wgsl.confidence > 0.0);
    assert!(wgsl
      .evidence
      .iter()
      .any(|e| e.path == "shaders" && e.kind == "directory"));
  }
}
