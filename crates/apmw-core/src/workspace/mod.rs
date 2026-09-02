//! Workspace / monorepo detection.
//!
//! Detects workspace organizers (Cargo workspace, pnpm workspace, npm
//! workspace, Yarn workspace, Nx, Turborepo, Lerna, Gradle composite, Maven
//! multi-module) in a given directory and returns a [`WorkspaceResult`] with
//! the organizer type, the workspace root, and member project paths.
//!
//! ## Performance
//!
//! Detection scans only the top-level of the given directory (no recursion),
//! keeping it fast. Cargo workspace member globs are expanded with the `glob`
//! crate.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::debug;

/// A workspace / monorepo organizer.
///
/// Each variant corresponds to a tool or convention that groups multiple
/// packages into a single workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceOrganizer {
  /// Cargo workspace (`[workspace]` in `Cargo.toml`).
  Cargo,
  /// pnpm workspace (`pnpm-workspace.yaml`).
  Pnpm,
  /// npm workspace (`workspaces` field in `package.json`).
  Npm,
  /// Yarn (classic) workspace (`workspaces` field in `package.json` plus
  /// `yarn.lock`).
  Yarn,
  /// Nx monorepo (`nx.json`).
  Nx,
  /// Turborepo (`turbo.json`).
  Turborepo,
  /// Lerna (`lerna.json`).
  Lerna,
  /// Gradle composite build (`includeBuild` in `settings.gradle`).
  GradleComposite,
  /// Maven multi-module (`<modules>` in `pom.xml`).
  MavenMultiModule,
}

impl WorkspaceOrganizer {
  /// Returns the canonical string name for this organizer.
  pub fn as_str(self) -> &'static str {
    match self {
      WorkspaceOrganizer::Cargo => "cargo",
      WorkspaceOrganizer::Pnpm => "pnpm",
      WorkspaceOrganizer::Npm => "npm",
      WorkspaceOrganizer::Yarn => "yarn",
      WorkspaceOrganizer::Nx => "nx",
      WorkspaceOrganizer::Turborepo => "turborepo",
      WorkspaceOrganizer::Lerna => "lerna",
      WorkspaceOrganizer::GradleComposite => "gradle-composite",
      WorkspaceOrganizer::MavenMultiModule => "maven-multi-module",
    }
  }
}

impl std::fmt::Display for WorkspaceOrganizer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// The result of a workspace / monorepo detection scan.
///
/// Contains the detected organizer, the workspace root directory, and the
/// resolved member project paths (relative to the workspace root where
/// applicable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceResult {
  /// The detected workspace organizer.
  pub organizer: WorkspaceOrganizer,
  /// The workspace root directory (the directory that contains the marker
  /// file).
  pub root: PathBuf,
  /// Resolved member project paths. For Cargo workspaces these are
  /// glob-expanded absolute directory paths. For other organizers the list
  /// may be empty when member resolution is not supported by this module.
  pub members: Vec<PathBuf>,
}

/// Detects a workspace / monorepo organizer in the given directory.
///
/// Checks each organizer in a deterministic order and returns the first
/// match. Returns `None` when no workspace marker is found.
///
/// The detection order is:
/// 1. Cargo workspace
/// 2. pnpm workspace
/// 3. Yarn workspace (requires `yarn.lock`)
/// 4. npm workspace
/// 5. Nx
/// 6. Turborepo
/// 7. Lerna
/// 8. Gradle composite
/// 9. Maven multi-module
pub fn detect_workspace(path: &Path) -> Option<WorkspaceResult> {
  if let Some(result) = detect_cargo_workspace(path) {
    debug!(?result, "detected cargo workspace");
    return Some(result);
  }
  if let Some(result) = detect_pnpm_workspace(path) {
    debug!(?result, "detected pnpm workspace");
    return Some(result);
  }
  if let Some(result) = detect_yarn_workspace(path) {
    debug!(?result, "detected yarn workspace");
    return Some(result);
  }
  if let Some(result) = detect_npm_workspace(path) {
    debug!(?result, "detected npm workspace");
    return Some(result);
  }
  if let Some(result) = detect_nx(path) {
    debug!(?result, "detected nx workspace");
    return Some(result);
  }
  if let Some(result) = detect_turborepo(path) {
    debug!(?result, "detected turborepo workspace");
    return Some(result);
  }
  if let Some(result) = detect_lerna(path) {
    debug!(?result, "detected lerna workspace");
    return Some(result);
  }
  if let Some(result) = detect_gradle_composite(path) {
    debug!(?result, "detected gradle composite workspace");
    return Some(result);
  }
  if let Some(result) = detect_maven_multi_module(path) {
    debug!(?result, "detected maven multi-module workspace");
    return Some(result);
  }
  None
}

// ---------------------------------------------------------------------------
// Per-organizer detection functions
// ---------------------------------------------------------------------------

/// Detects a Cargo workspace by parsing `Cargo.toml` for a `[workspace]`
/// section. Member globs (e.g. `crates/*`) are expanded to absolute directory
/// paths using the `glob` crate.
fn detect_cargo_workspace(path: &Path) -> Option<WorkspaceResult> {
  let cargo_toml = path.join("Cargo.toml");
  let contents = std::fs::read_to_string(&cargo_toml).ok()?;
  let parsed: toml::Value = toml::from_str(&contents).ok()?;
  let workspace = parsed.get("workspace")?;
  // A `[workspace]` table with either `members` or `exclude` (or neither,
  // which implies a virtual manifest) counts as a workspace.
  let has_members = workspace.get("members").is_some();
  let has_exclude = workspace.get("exclude").is_some();
  if !has_members && !has_exclude {
    // A bare `[workspace]` table (virtual manifest) is still a workspace.
    // Accept it as long as the table exists.
  }
  let members = expand_cargo_members(path, workspace);
  Some(WorkspaceResult {
    organizer: WorkspaceOrganizer::Cargo,
    root: path.to_path_buf(),
    members,
  })
}

/// Expands Cargo workspace `members` globs relative to the workspace root.
fn expand_cargo_members(root: &Path, workspace: &toml::Value) -> Vec<PathBuf> {
  let mut members = Vec::new();
  let Some(member_patterns) = workspace.get("members").and_then(|v| v.as_array()) else {
    return members;
  };
  for pattern_value in member_patterns {
    let Some(pattern) = pattern_value.as_str() else {
      continue;
    };
    let full_pattern = match root.join(pattern).to_str() {
      Some(p) => p.to_string(),
      None => continue,
    };
    for entry in glob::glob(&full_pattern).into_iter().flatten().flatten() {
      if entry.is_dir() {
        members.push(entry);
      }
    }
  }
  members.sort();
  members
}

/// Detects a pnpm workspace by checking for the existence of
/// `pnpm-workspace.yaml`.
fn detect_pnpm_workspace(path: &Path) -> Option<WorkspaceResult> {
  let marker = path.join("pnpm-workspace.yaml");
  if marker.exists() {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::Pnpm,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects an npm workspace by parsing `package.json` for a `workspaces`
/// field.
fn detect_npm_workspace(path: &Path) -> Option<WorkspaceResult> {
  let package_json = path.join("package.json");
  let contents = std::fs::read_to_string(&package_json).ok()?;
  let parsed: serde_json::Value = serde_json::from_str(&contents).ok()?;
  if parsed.get("workspaces").is_some() {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::Npm,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects a Yarn (classic) workspace by parsing `package.json` for a
/// `workspaces` field **and** confirming that `yarn.lock` exists.
fn detect_yarn_workspace(path: &Path) -> Option<WorkspaceResult> {
  if !path.join("yarn.lock").exists() {
    return None;
  }
  let package_json = path.join("package.json");
  let contents = std::fs::read_to_string(&package_json).ok()?;
  let parsed: serde_json::Value = serde_json::from_str(&contents).ok()?;
  if parsed.get("workspaces").is_some() {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::Yarn,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects an Nx monorepo by checking for the existence of `nx.json`.
fn detect_nx(path: &Path) -> Option<WorkspaceResult> {
  if path.join("nx.json").exists() {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::Nx,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects a Turborepo by checking for the existence of `turbo.json`.
fn detect_turborepo(path: &Path) -> Option<WorkspaceResult> {
  if path.join("turbo.json").exists() {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::Turborepo,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects a Lerna monorepo by checking for the existence of `lerna.json`.
fn detect_lerna(path: &Path) -> Option<WorkspaceResult> {
  if path.join("lerna.json").exists() {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::Lerna,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects a Gradle composite build by searching `settings.gradle` for the
/// `includeBuild` keyword.
fn detect_gradle_composite(path: &Path) -> Option<WorkspaceResult> {
  let settings_gradle = path.join("settings.gradle");
  let contents = std::fs::read_to_string(&settings_gradle).ok()?;
  if contents.contains("includeBuild") {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::GradleComposite,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

/// Detects a Maven multi-module project by searching `pom.xml` for a
/// `<modules>` element.
fn detect_maven_multi_module(path: &Path) -> Option<WorkspaceResult> {
  let pom_xml = path.join("pom.xml");
  let contents = std::fs::read_to_string(&pom_xml).ok()?;
  if contents.contains("<modules>") {
    Some(WorkspaceResult {
      organizer: WorkspaceOrganizer::MavenMultiModule,
      root: path.to_path_buf(),
      members: Vec::new(),
    })
  } else {
    None
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  fn write_file(dir: &Path, name: &str, contents: &str) {
    fs::write(dir.join(name), contents).expect("write file");
  }

  #[test]
  fn test_detect_cargo_workspace() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    let result = detect_workspace(dir.path()).expect("cargo workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Cargo);
    assert_eq!(result.root, dir.path());
    assert!(result.members.is_empty());
  }

  #[test]
  fn test_detect_cargo_workspace_virtual_manifest() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "Cargo.toml", "[workspace]\n");
    let result = detect_workspace(dir.path()).expect("cargo workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Cargo);
  }

  #[test]
  fn test_detect_cargo_workspace_member_glob_expansion() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("crates/foo")).unwrap();
    fs::create_dir_all(root.join("crates/bar")).unwrap();
    fs::create_dir_all(root.join("crates/baz")).unwrap();
    // Create a non-directory entry that should be filtered out.
    fs::write(root.join("crates/README.md"), "readme").unwrap();
    write_file(
      root,
      "Cargo.toml",
      "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    let result = detect_workspace(dir.path()).expect("cargo workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Cargo);
    let member_names: Vec<String> = result
      .members
      .iter()
      .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
      .map(|s| s.to_string())
      .collect();
    assert!(member_names.contains(&"foo".to_string()));
    assert!(member_names.contains(&"bar".to_string()));
    assert!(member_names.contains(&"baz".to_string()));
    // Non-directory entries must not appear.
    assert!(!member_names.contains(&"README.md".to_string()));
    // All members must be directories.
    for m in &result.members {
      assert!(m.is_dir(), "{:?} is not a directory", m);
    }
  }

  #[test]
  fn test_detect_pnpm_workspace() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pnpm-workspace.yaml",
      "packages:\n  - packages/*\n",
    );
    let result = detect_workspace(dir.path()).expect("pnpm workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Pnpm);
    assert_eq!(result.root, dir.path());
  }

  #[test]
  fn test_detect_npm_workspace() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "root", "workspaces": ["packages/*"]}"#,
    );
    let result = detect_workspace(dir.path()).expect("npm workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Npm);
  }

  #[test]
  fn test_detect_yarn_workspace() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "root", "workspaces": ["packages/*"]}"#,
    );
    write_file(dir.path(), "yarn.lock", "# yarn lockfile\n");
    let result = detect_workspace(dir.path()).expect("yarn workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Yarn);
  }

  #[test]
  fn test_detect_yarn_workspace_without_lock_returns_none() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "root", "workspaces": ["packages/*"]}"#,
    );
    // No yarn.lock — npm detection should win instead (no yarn.lock).
    let result = detect_workspace(dir.path()).expect("npm workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Npm);
  }

  #[test]
  fn test_detect_nx() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "nx.json",
      r#"{"extends": "nx/presets/core.json"}"#,
    );
    let result = detect_workspace(dir.path()).expect("nx workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Nx);
  }

  #[test]
  fn test_detect_turborepo() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "turbo.json", r#"{"pipeline": {"build": {}}}"#);
    let result = detect_workspace(dir.path()).expect("turborepo workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Turborepo);
  }

  #[test]
  fn test_detect_lerna() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "lerna.json", r#"{"packages": ["packages/*"]}"#);
    let result = detect_workspace(dir.path()).expect("lerna workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Lerna);
  }

  #[test]
  fn test_detect_gradle_composite() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "settings.gradle",
      "rootProject.name = 'root'\nincludeBuild '../other'\n",
    );
    let result = detect_workspace(dir.path()).expect("gradle composite");
    assert_eq!(result.organizer, WorkspaceOrganizer::GradleComposite);
  }

  #[test]
  fn test_detect_gradle_composite_without_include_build_returns_none() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "settings.gradle", "rootProject.name = 'root'\n");
    assert!(detect_gradle_composite(dir.path()).is_none());
  }

  #[test]
  fn test_detect_maven_multi_module() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      "<project>\n  <modules>\n    <module>core</module>\n  </modules>\n</project>\n",
    );
    let result = detect_workspace(dir.path()).expect("maven multi-module");
    assert_eq!(result.organizer, WorkspaceOrganizer::MavenMultiModule);
  }

  #[test]
  fn test_detect_maven_without_modules_returns_none() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      "<project>\n  <groupId>com.example</groupId>\n</project>\n",
    );
    assert!(detect_maven_multi_module(dir.path()).is_none());
  }

  #[test]
  fn test_detect_no_workspace_returns_none() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "README.md", "# nothing here\n");
    assert!(detect_workspace(dir.path()).is_none());
  }

  #[test]
  fn test_cargo_takes_precedence_over_npm() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "root", "workspaces": ["packages/*"]}"#,
    );
    let result = detect_workspace(dir.path()).expect("workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Cargo);
  }

  #[test]
  fn test_yarn_takes_precedence_over_npm() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "root", "workspaces": ["packages/*"]}"#,
    );
    write_file(dir.path(), "yarn.lock", "# yarn\n");
    let result = detect_workspace(dir.path()).expect("workspace");
    assert_eq!(result.organizer, WorkspaceOrganizer::Yarn);
  }

  #[test]
  fn test_organizer_as_str_and_display() {
    assert_eq!(WorkspaceOrganizer::Cargo.as_str(), "cargo");
    assert_eq!(WorkspaceOrganizer::Pnpm.as_str(), "pnpm");
    assert_eq!(WorkspaceOrganizer::Npm.as_str(), "npm");
    assert_eq!(WorkspaceOrganizer::Yarn.as_str(), "yarn");
    assert_eq!(WorkspaceOrganizer::Nx.as_str(), "nx");
    assert_eq!(WorkspaceOrganizer::Turborepo.as_str(), "turborepo");
    assert_eq!(WorkspaceOrganizer::Lerna.as_str(), "lerna");
    assert_eq!(
      WorkspaceOrganizer::GradleComposite.as_str(),
      "gradle-composite"
    );
    assert_eq!(
      WorkspaceOrganizer::MavenMultiModule.as_str(),
      "maven-multi-module"
    );
    assert_eq!(format!("{}", WorkspaceOrganizer::Cargo), "cargo");
  }

  #[test]
  fn test_workspace_result_serde_roundtrip() {
    let result = WorkspaceResult {
      organizer: WorkspaceOrganizer::Cargo,
      root: PathBuf::from("/tmp/ws"),
      members: vec![PathBuf::from("/tmp/ws/crates/a")],
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: WorkspaceResult = serde_json::from_str(&json).unwrap();
    assert_eq!(result, back);
  }
}
