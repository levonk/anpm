//! Detection attributes derived from 2ndbrain research Table 1.
//!
//! This module provides the [`DetectionAttributes`] struct that maps a
//! detected file or directory to the package manager(s) it indicates. The
//! attributes are the static files and patterns `apmw` uses to detect which
//! package manager or build system is in use for a given project.
//!
//! Source: 2ndbrain research document "All Package Manager Wrapper Tool
//! Landscape" — Table 1 (Package Manager Project Detection Attributes).

use std::collections::HashMap;

use super::managers::{all_managers, PackageManager};

/// A single piece of evidence for a detection result — a file or directory
/// that was found on disk and matched a package manager's detection attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionEvidence {
  /// The file or directory path relative to the scanned directory.
  pub path: String,
  /// Whether this is a primary or secondary identifying file.
  pub kind: EvidenceKind,
  /// The name of the package manager this evidence points to.
  pub manager_name: &'static str,
}

/// The strength of a piece of detection evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
  /// A primary identifying file — strong evidence (lockfile or main config).
  Primary,
  /// A secondary identifying file — supporting evidence.
  Secondary,
  /// A directory marker — moderate evidence.
  Directory,
}

impl std::fmt::Display for EvidenceKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EvidenceKind::Primary => write!(f, "primary"),
      EvidenceKind::Secondary => write!(f, "secondary"),
      EvidenceKind::Directory => write!(f, "directory"),
    }
  }
}

/// Detection attributes for a single package manager.
///
/// Combines the primary files, secondary files, and directory markers from
/// 2ndbrain Table 1 into a single lookup structure.
#[derive(Debug, Clone)]
pub struct DetectionAttributes {
  /// The package manager these attributes belong to.
  pub manager: &'static PackageManager,
  /// Primary identifying files (strong evidence).
  pub primary_files: &'static [&'static str],
  /// Secondary identifying files (supporting evidence).
  pub secondary_files: &'static [&'static str],
  /// Directory markers (moderate evidence).
  pub dir_markers: &'static [&'static str],
}

impl DetectionAttributes {
  /// Create detection attributes from a [`PackageManager`].
  pub fn from_manager(manager: &'static PackageManager) -> Self {
    DetectionAttributes {
      manager,
      primary_files: manager.primary_files,
      secondary_files: manager.secondary_files,
      dir_markers: manager.dir_markers,
    }
  }
}

/// Returns the detection attributes for all registered package managers.
pub fn all_attributes() -> Vec<DetectionAttributes> {
  all_managers()
    .iter()
    .map(|m| DetectionAttributes::from_manager(m))
    .collect()
}

/// Build a lookup map from file name to list of manager names that use it as a
/// primary file.
pub fn primary_file_index() -> HashMap<&'static str, Vec<&'static str>> {
  let mut map: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
  for manager in all_managers() {
    for file in manager.primary_files {
      map.entry(file).or_default().push(manager.name);
    }
  }
  map
}

/// Build a lookup map from file name to list of manager names that use it as a
/// secondary file.
pub fn secondary_file_index() -> HashMap<&'static str, Vec<&'static str>> {
  let mut map: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
  for manager in all_managers() {
    for file in manager.secondary_files {
      map.entry(file).or_default().push(manager.name);
    }
  }
  map
}

/// Build a lookup map from directory name to list of manager names that use it
/// as a directory marker.
pub fn dir_marker_index() -> HashMap<&'static str, Vec<&'static str>> {
  let mut map: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
  for manager in all_managers() {
    for dir in manager.dir_markers {
      map.entry(dir).or_default().push(manager.name);
    }
  }
  map
}

/// Check if a file name matches a glob pattern (supports `*` wildcard).
pub fn matches_glob(pattern: &str, name: &str) -> bool {
  if !pattern.contains('*') {
    return pattern == name;
  }
  let parts: Vec<&str> = pattern.split('*').collect();
  if parts.is_empty() {
    return true;
  }
  let mut search_from = 0;
  for (i, part) in parts.iter().enumerate() {
    if part.is_empty() {
      continue;
    }
    if i == 0 {
      if !name.starts_with(part) {
        return false;
      }
      search_from = part.len();
    } else if let Some(found_at) = name[search_from..].find(part) {
      search_from += found_at + part.len();
    } else {
      return false;
    }
  }
  if !pattern.ends_with('*') {
    let last = parts.last().unwrap();
    if !last.is_empty() && !name.ends_with(last) {
      return false;
    }
  }
  true
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_all_attributes_count() {
    let attrs = all_attributes();
    assert!(
      attrs.len() >= 20,
      "Must have at least 20 sets of attributes, got {}",
      attrs.len()
    );
  }

  #[test]
  fn test_attributes_cargo() {
    let attrs = all_attributes();
    let cargo = attrs
      .iter()
      .find(|a| a.manager.name == "cargo")
      .expect("cargo attributes must exist");
    assert!(cargo.primary_files.contains(&"Cargo.toml"));
    assert!(cargo.secondary_files.contains(&"Cargo.lock"));
    assert!(cargo.dir_markers.contains(&"target"));
  }

  #[test]
  fn test_attributes_pnpm() {
    let attrs = all_attributes();
    let pnpm = attrs
      .iter()
      .find(|a| a.manager.name == "pnpm")
      .expect("pnpm attributes must exist");
    assert!(pnpm.primary_files.contains(&"pnpm-lock.yaml"));
    assert!(pnpm.secondary_files.contains(&"pnpm-workspace.yaml"));
  }

  #[test]
  fn test_primary_file_index() {
    let index = primary_file_index();
    let cargo_managers = index.get("Cargo.toml").expect("Cargo.toml must be indexed");
    assert!(cargo_managers.contains(&"cargo"));
    let go_managers = index.get("go.mod").expect("go.mod must be indexed");
    assert!(go_managers.contains(&"go"));
  }

  #[test]
  fn test_secondary_file_index() {
    let index = secondary_file_index();
    let cargo_managers = index
      .get("Cargo.lock")
      .expect("Cargo.lock must be indexed as secondary");
    assert!(cargo_managers.contains(&"cargo"));
  }

  #[test]
  fn test_dir_marker_index() {
    let index = dir_marker_index();
    let node_managers = index
      .get("node_modules")
      .expect("node_modules must be indexed");
    assert!(node_managers.contains(&"npm"));
    assert!(node_managers.contains(&"pnpm"));
    assert!(node_managers.contains(&"yarn"));
    let target_managers = index.get("target").expect("target must be indexed");
    assert!(target_managers.contains(&"cargo"));
  }

  #[test]
  fn test_matches_glob_exact() {
    assert!(matches_glob("Cargo.toml", "Cargo.toml"));
    assert!(!matches_glob("Cargo.toml", "Cargo.lock"));
  }

  #[test]
  fn test_matches_glob_wildcard() {
    assert!(matches_glob("*.csproj", "MyApp.csproj"));
    assert!(matches_glob("*.csproj", "App.csproj"));
    assert!(!matches_glob("*.csproj", "MyApp.fsproj"));
  }

  #[test]
  fn test_matches_glob_no_wildcard() {
    assert!(matches_glob("go.mod", "go.mod"));
    assert!(!matches_glob("go.mod", "go.sum"));
  }

  #[test]
  fn test_evidence_kind_display() {
    assert_eq!(EvidenceKind::Primary.to_string(), "primary");
    assert_eq!(EvidenceKind::Secondary.to_string(), "secondary");
    assert_eq!(EvidenceKind::Directory.to_string(), "directory");
  }

  #[test]
  fn test_detection_attributes_from_manager() {
    let manager = all_managers()
      .iter()
      .find(|m| m.name == "npm")
      .expect("npm must exist");
    let attrs = DetectionAttributes::from_manager(manager);
    assert_eq!(attrs.manager.name, "npm");
    assert!(attrs.primary_files.contains(&"package-lock.json"));
  }

  #[test]
  fn test_python_managers_have_distinct_lockfiles() {
    let index = primary_file_index();
    assert!(index.get("poetry.lock").map(|v| v.contains(&"poetry")).unwrap_or(false));
    assert!(index.get("pdm.lock").map(|v| v.contains(&"pdm")).unwrap_or(false));
    assert!(index.get("uv.lock").map(|v| v.contains(&"uv")).unwrap_or(false));
    assert!(index.get("Pipfile").map(|v| v.contains(&"pipenv")).unwrap_or(false));
    assert!(index.get("Pipfile.lock").map(|v| v.contains(&"pipenv")).unwrap_or(false));
  }
}
