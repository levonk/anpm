//! Container usage detection from project files.
//!
//! Scans a project directory for files that indicate container usage:
//! - `Dockerfile` — container build definition
//! - `docker-compose.yml` — orchestration with docker-compose
//! - `docker-compose.*.yml` — override/environment-specific compose files
//! - `Containerfile` — Podman/Buildah container build definition
//! - `compose.yaml` — modern compose file format
//!
//! Also detects docker-compose as an orchestration package manager.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::debug;

/// The kind of container usage detected in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerUsageKind {
  /// A `Dockerfile` was found — container build definition.
  Dockerfile,
  /// A `Containerfile` was found — Podman/Buildah container build definition.
  Containerfile,
  /// A `docker-compose.yml` or `compose.yaml` was found — orchestration.
  Compose,
  /// A `docker-compose.*.yml` override file was found.
  ComposeOverride,
}

impl std::fmt::Display for ContainerUsageKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ContainerUsageKind::Dockerfile => write!(f, "dockerfile"),
      ContainerUsageKind::Containerfile => write!(f, "containerfile"),
      ContainerUsageKind::Compose => write!(f, "compose"),
      ContainerUsageKind::ComposeOverride => write!(f, "compose_override"),
    }
  }
}

/// A single detected container usage artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUsage {
  /// The kind of container usage.
  pub kind: ContainerUsageKind,
  /// The file path relative to the scanned directory.
  pub path: String,
}

/// Information about docker-compose orchestration usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerComposeInfo {
  /// Whether docker-compose orchestration is in use.
  pub in_use: bool,
  /// The compose files detected.
  pub files: Vec<String>,
}

/// The result of detecting container usage in a project directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerDetectionResult {
  /// All detected container usage artifacts.
  pub usages: Vec<ContainerUsage>,
  /// Whether any container usage was detected.
  pub has_containers: bool,
  /// Whether a Dockerfile was found.
  pub has_dockerfile: bool,
  /// Whether a Containerfile was found.
  pub has_containerfile: bool,
  /// Whether docker-compose orchestration was detected.
  pub has_compose: bool,
  /// Docker-compose orchestration details.
  pub compose: DockerComposeInfo,
}

impl ContainerDetectionResult {
  /// Returns `true` if no container usage was detected.
  pub fn is_empty(&self) -> bool {
    self.usages.is_empty()
  }

  /// Returns the detected runtime preference based on file types.
  ///
  /// If a `Containerfile` is found, podman is preferred. If only a
  /// `Dockerfile` is found, docker is preferred. If both are found,
  /// no preference is returned.
  pub fn preferred_runtime(&self) -> Option<crate::containers::ContainerRuntime> {
    match (self.has_containerfile, self.has_dockerfile) {
      (true, false) => Some(crate::containers::ContainerRuntime::Podman),
      (false, true) => Some(crate::containers::ContainerRuntime::Docker),
      _ => None,
    }
  }
}

/// Files that indicate a Dockerfile (container build definition).
const DOCKERFILE_NAMES: &[&str] = &["Dockerfile", "Dockerfile.dev", "Dockerfile.prod"];

/// Files that indicate a Containerfile (Podman/Buildah build definition).
const CONTAINERFILE_NAMES: &[&str] = &["Containerfile", "Containerfile.dev"];

/// Files that indicate docker-compose orchestration (exact matches).
const COMPOSE_EXACT_NAMES: &[&str] = &["docker-compose.yml", "compose.yaml", "compose.yml"];

/// Prefix for docker-compose override files (glob: `docker-compose.*.yml`).
const COMPOSE_OVERRIDE_PREFIX: &str = "docker-compose.";
const COMPOSE_OVERRIDE_SUFFIX: &str = ".yml";

/// Detects container usage in the given project directory.
///
/// Scans the top-level of the directory for container-related files:
/// `Dockerfile`, `docker-compose.yml`, `docker-compose.*.yml`, `Containerfile`,
/// and `compose.yaml`.
///
/// # Errors
///
/// Returns an error if the directory doesn't exist or cannot be read.
pub fn detect_container_usage(dir: &Path) -> crate::error::Result<ContainerDetectionResult> {
  if !dir.exists() {
    return Err(crate::error::ApmwError::PackageManagerNotFound(format!(
      "directory does not exist: {}",
      dir.display()
    )));
  }

  let entries = std::fs::read_dir(dir).map_err(crate::error::ApmwError::Io)?;

  let mut usages: Vec<ContainerUsage> = Vec::new();
  let mut compose_files: Vec<String> = Vec::new();

  for entry in entries {
    let entry = match entry {
      Ok(e) => e,
      Err(err) => {
        debug!(error = %err, "Failed to read directory entry during container detection");
        continue;
      }
    };

    // Only check files, not directories.
    if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
      continue;
    }

    let name = entry.file_name().to_string_lossy().to_string();

    if DOCKERFILE_NAMES.contains(&name.as_str()) {
      debug!(file = %name, "Detected Dockerfile");
      usages.push(ContainerUsage {
        kind: ContainerUsageKind::Dockerfile,
        path: name.clone(),
      });
    } else if CONTAINERFILE_NAMES.contains(&name.as_str()) {
      debug!(file = %name, "Detected Containerfile");
      usages.push(ContainerUsage {
        kind: ContainerUsageKind::Containerfile,
        path: name.clone(),
      });
    } else if COMPOSE_EXACT_NAMES.contains(&name.as_str()) {
      debug!(file = %name, "Detected compose file");
      usages.push(ContainerUsage {
        kind: ContainerUsageKind::Compose,
        path: name.clone(),
      });
      compose_files.push(name);
    } else if is_compose_override(&name) {
      debug!(file = %name, "Detected compose override file");
      usages.push(ContainerUsage {
        kind: ContainerUsageKind::ComposeOverride,
        path: name.clone(),
      });
      compose_files.push(name);
    }
  }

  // Sort for deterministic output.
  usages.sort_by(|a, b| a.path.cmp(&b.path));
  compose_files.sort();

  let has_dockerfile = usages
    .iter()
    .any(|u| u.kind == ContainerUsageKind::Dockerfile);
  let has_containerfile = usages
    .iter()
    .any(|u| u.kind == ContainerUsageKind::Containerfile);
  let has_compose = usages.iter().any(|u| {
    matches!(
      u.kind,
      ContainerUsageKind::Compose | ContainerUsageKind::ComposeOverride
    )
  });

  let result = ContainerDetectionResult {
    has_containers: !usages.is_empty(),
    has_dockerfile,
    has_containerfile,
    has_compose,
    compose: DockerComposeInfo {
      in_use: !compose_files.is_empty(),
      files: compose_files,
    },
    usages,
  };

  debug!(
    has_containers = result.has_containers,
    has_dockerfile = result.has_dockerfile,
    has_containerfile = result.has_containerfile,
    has_compose = result.has_compose,
    "Container usage detection complete"
  );

  Ok(result)
}

/// Checks if a file name is a docker-compose override file (`docker-compose.*.yml`).
///
/// Excludes the exact `docker-compose.yml` which is handled separately.
fn is_compose_override(name: &str) -> bool {
  if name == "docker-compose.yml" {
    return false;
  }
  name.starts_with(COMPOSE_OVERRIDE_PREFIX)
    && name.ends_with(COMPOSE_OVERRIDE_SUFFIX)
    && name.len() > COMPOSE_OVERRIDE_PREFIX.len() + COMPOSE_OVERRIDE_SUFFIX.len()
}

/// Returns the path to the first detected compose file, if any.
///
/// Useful for callers that want to run `docker-compose -f <file>` commands.
pub fn find_compose_file(dir: &Path) -> Option<PathBuf> {
  let result = detect_container_usage(dir).ok()?;
  result.compose.files.first().map(|f| dir.join(f))
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  fn create_file(dir: &Path, name: &str) {
    std::fs::write(dir.join(name), "# test file\n").unwrap();
  }

  #[test]
  fn test_detect_dockerfile() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile");
    create_file(tmp.path(), "README.md");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_containers);
    assert!(result.has_dockerfile);
    assert!(!result.has_containerfile);
    assert!(!result.has_compose);
    assert_eq!(result.usages.len(), 1);
    assert_eq!(result.usages[0].kind, ContainerUsageKind::Dockerfile);
    assert_eq!(result.usages[0].path, "Dockerfile");
  }

  #[test]
  fn test_detect_dockerfile_variants() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile.dev");
    create_file(tmp.path(), "Dockerfile.prod");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_dockerfile);
    assert_eq!(
      result
        .usages
        .iter()
        .filter(|u| u.kind == ContainerUsageKind::Dockerfile)
        .count(),
      2
    );
  }

  #[test]
  fn test_detect_containerfile() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Containerfile");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_containers);
    assert!(result.has_containerfile);
    assert!(!result.has_dockerfile);
    assert_eq!(result.usages.len(), 1);
    assert_eq!(result.usages[0].kind, ContainerUsageKind::Containerfile);
  }

  #[test]
  fn test_detect_compose_yaml() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "docker-compose.yml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_compose);
    assert!(result.compose.in_use);
    assert_eq!(result.compose.files, vec!["docker-compose.yml"]);
    assert_eq!(result.usages[0].kind, ContainerUsageKind::Compose);
  }

  #[test]
  fn test_detect_compose_yaml_modern() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "compose.yaml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_compose);
    assert!(result.compose.in_use);
    assert_eq!(result.compose.files, vec!["compose.yaml"]);
  }

  #[test]
  fn test_detect_compose_yml_modern() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "compose.yml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_compose);
    assert_eq!(result.compose.files, vec!["compose.yml"]);
  }

  #[test]
  fn test_detect_compose_override() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "docker-compose.yml");
    create_file(tmp.path(), "docker-compose.override.yml");
    create_file(tmp.path(), "docker-compose.prod.yml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_compose);
    assert_eq!(result.compose.files.len(), 3);
    assert!(result
      .compose
      .files
      .contains(&"docker-compose.yml".to_string()));
    assert!(result
      .compose
      .files
      .contains(&"docker-compose.override.yml".to_string()));
    assert!(result
      .compose
      .files
      .contains(&"docker-compose.prod.yml".to_string()));

    // Check kinds
    let compose_count = result
      .usages
      .iter()
      .filter(|u| u.kind == ContainerUsageKind::Compose)
      .count();
    let override_count = result
      .usages
      .iter()
      .filter(|u| u.kind == ContainerUsageKind::ComposeOverride)
      .count();
    assert_eq!(compose_count, 1);
    assert_eq!(override_count, 2);
  }

  #[test]
  fn test_detect_all_container_files() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile");
    create_file(tmp.path(), "docker-compose.yml");
    create_file(tmp.path(), "docker-compose.prod.yml");
    create_file(tmp.path(), "Containerfile");
    create_file(tmp.path(), "compose.yaml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.has_containers);
    assert!(result.has_dockerfile);
    assert!(result.has_containerfile);
    assert!(result.has_compose);
    assert_eq!(result.usages.len(), 5);
  }

  #[test]
  fn test_detect_no_container_files() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "package.json");
    create_file(tmp.path(), "README.md");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(!result.has_containers);
    assert!(result.is_empty());
    assert!(!result.has_dockerfile);
    assert!(!result.has_containerfile);
    assert!(!result.has_compose);
  }

  #[test]
  fn test_detect_nonexistent_directory() {
    let result = detect_container_usage(Path::new("/nonexistent/path/that/does/not/exist"));
    assert!(result.is_err());
  }

  #[test]
  fn test_preferred_runtime_dockerfile_only() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert_eq!(
      result.preferred_runtime(),
      Some(crate::containers::ContainerRuntime::Docker)
    );
  }

  #[test]
  fn test_preferred_runtime_containerfile_only() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Containerfile");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert_eq!(
      result.preferred_runtime(),
      Some(crate::containers::ContainerRuntime::Podman)
    );
  }

  #[test]
  fn test_preferred_runtime_both() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile");
    create_file(tmp.path(), "Containerfile");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert_eq!(result.preferred_runtime(), None);
  }

  #[test]
  fn test_preferred_runtime_neither() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "docker-compose.yml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert_eq!(result.preferred_runtime(), None);
  }

  #[test]
  fn test_is_compose_override() {
    assert!(!is_compose_override("docker-compose.yml"));
    assert!(is_compose_override("docker-compose.override.yml"));
    assert!(is_compose_override("docker-compose.prod.yml"));
    assert!(is_compose_override("docker-compose.dev.yml"));
    assert!(!is_compose_override("docker-compose.yml.bak"));
    assert!(!is_compose_override("Dockerfile"));
    assert!(!is_compose_override("docker-compose.")); // too short, no suffix
  }

  #[test]
  fn test_find_compose_file() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "docker-compose.yml");
    create_file(tmp.path(), "Dockerfile");

    let compose_file = find_compose_file(tmp.path()).unwrap();
    assert_eq!(compose_file, tmp.path().join("docker-compose.yml"));
  }

  #[test]
  fn test_find_compose_file_none() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile");

    let result = find_compose_file(tmp.path());
    assert!(result.is_none());
  }

  #[test]
  fn test_docker_compose_orchestration_info() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "docker-compose.yml");
    create_file(tmp.path(), "docker-compose.override.yml");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.compose.in_use);
    assert_eq!(result.compose.files.len(), 2);
  }

  #[test]
  fn test_container_usage_kind_display() {
    assert_eq!(ContainerUsageKind::Dockerfile.to_string(), "dockerfile");
    assert_eq!(
      ContainerUsageKind::Containerfile.to_string(),
      "containerfile"
    );
    assert_eq!(ContainerUsageKind::Compose.to_string(), "compose");
    assert_eq!(
      ContainerUsageKind::ComposeOverride.to_string(),
      "compose_override"
    );
  }

  #[test]
  fn test_detection_result_is_empty() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "README.md");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn test_detection_result_not_empty() {
    let tmp = TempDir::new().unwrap();
    create_file(tmp.path(), "Dockerfile");

    let result = detect_container_usage(tmp.path()).unwrap();
    assert!(!result.is_empty());
  }
}
