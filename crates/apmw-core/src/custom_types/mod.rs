//! Custom project type definitions loaded from YAML config files.
//!
//! This module allows users to extend `apmw` with custom project types beyond
//! the built-in [`PackageManager`](crate::detect::PackageManager) registry.
//! Custom types are defined in YAML config files and merged with the built-in
//! definitions, with custom types taking precedence on name conflicts.
//!
//! ## Config file locations
//!
//! Config files are loaded from two locations (in order of increasing
//! precedence):
//!
//! 1. **User-wide**: `~/.config/apmw/project-types.yml`
//! 2. **Per-project**: `.apmw/project-types.yml` (relative to the current
//!    project directory)
//!
//! When both files define a type with the same name, the per-project version
//! wins.
//!
//! ## YAML format
//!
//! ```yaml
//! types:
//!   - name: wgsl
//!     display_name: "WGSL Shaders"
//!     ecosystem: unknown
//!     hierarchy: build_system
//!     primary_files:
//!       - "*.wgsl"
//!     secondary_files:
//!       - "wgsl.toml"
//!     dir_markers:
//!       - "shaders"
//!     priority: 50
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::detect::{Ecosystem, HierarchyLevel, PackageManager, PackageManagerOwned};
use crate::error::{CoreError, Result};

/// A custom project type definition loaded from a YAML config file.
///
/// Mirrors the fields of [`PackageManager`](crate::detect::PackageManager) but
/// uses owned `String` types since the values come from runtime config rather
/// than compile-time constants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomProjectType {
  /// Canonical name (e.g. `wgsl`, `shaderc`). Must be unique — if it matches
  /// a built-in `PackageManager` name, the custom type overrides the built-in.
  pub name: String,
  /// Human-readable display name.
  pub display_name: String,
  /// Ecosystem this type belongs to (serde snake_case, e.g. `rust`, `node`).
  pub ecosystem: Ecosystem,
  /// Hierarchy level (serde snake_case, e.g. `language`, `build_system`).
  pub hierarchy: HierarchyLevel,
  /// Primary identifying files — glob patterns that strongly indicate this
  /// type is in use (e.g. `Cargo.toml`, `*.wgsl`).
  #[serde(default)]
  pub primary_files: Vec<String>,
  /// Secondary identifying files — glob patterns for supporting evidence.
  #[serde(default)]
  pub secondary_files: Vec<String>,
  /// Directory markers — directory names whose presence indicates this type.
  #[serde(default)]
  pub dir_markers: Vec<String>,
  /// Detection priority — higher numbers are checked first when multiple
  /// types share the same primary file.
  #[serde(default)]
  pub priority: i32,
}

/// The top-level YAML config structure for custom project types.
///
/// Contains a list of [`CustomProjectType`] definitions under the `types` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomTypesConfig {
  /// The list of custom project type definitions.
  #[serde(default)]
  pub types: Vec<CustomProjectType>,
}

/// Returns the user-wide config path: `~/.config/apmw/project-types.yml`.
fn user_config_path() -> Option<PathBuf> {
  let home = std::env::var_os("HOME")?;
  Some(
    PathBuf::from(home)
      .join(".config")
      .join("apmw")
      .join("project-types.yml"),
  )
}

/// Returns the per-project config path: `.apmw/project-types.yml` (relative
/// to the current working directory).
fn project_config_path() -> PathBuf {
  PathBuf::from(".apmw").join("project-types.yml")
}

/// Load custom project types from a single YAML config file.
///
/// Returns an empty vec if the file does not exist (not an error). Returns
/// an error if the file exists but cannot be read or parsed.
fn load_from_file(path: &Path) -> Result<Vec<CustomProjectType>> {
  if !path.exists() {
    return Ok(Vec::new());
  }

  let contents = std::fs::read_to_string(path).map_err(CoreError::Io)?;
  let config: CustomTypesConfig = serde_yaml::from_str(&contents)?;
  Ok(config.types)
}

/// Merge two lists of custom types, with the `override` list taking precedence
/// on name conflicts.
///
/// Types from `base` whose name does not appear in `override` are kept; all
/// types from `override` are included. The relative order of types within each
/// list is preserved.
fn merge_custom_type_lists(
  base: &[CustomProjectType],
  override_: &[CustomProjectType],
) -> Vec<CustomProjectType> {
  let override_names: HashMap<&str, ()> = override_.iter().map(|c| (c.name.as_str(), ())).collect();

  let mut result: Vec<CustomProjectType> = base
    .iter()
    .filter(|c| !override_names.contains_key(c.name.as_str()))
    .cloned()
    .collect();
  result.extend(override_.iter().cloned());
  result
}

/// Load custom project types from both user-wide and per-project config files.
///
/// Reads from:
/// - `~/.config/apmw/project-types.yml` (user-wide)
/// - `.apmw/project-types.yml` (per-project, takes precedence on name conflict)
///
/// Missing config files are not an error — they simply contribute no types.
/// Malformed YAML files produce an error.
///
/// # Examples
///
/// ```no_run
/// use apmw_core::custom_types::load_custom_types;
///
/// let types = load_custom_types().expect("failed to load custom types");
/// for t in &types {
///     println!("custom type: {}", t.name);
/// }
/// ```
pub fn load_custom_types() -> Result<Vec<CustomProjectType>> {
  let user_wide =
    user_config_path().unwrap_or_else(|| PathBuf::from("~/.config/apmw/project-types.yml"));
  let per_project = project_config_path();
  load_custom_types_from(&user_wide, &per_project)
}

/// Load custom project types from explicit file paths.
///
/// This is the testable core of [`load_custom_types`] — it accepts explicit
/// paths instead of relying on environment variables, making it easy to test
/// with temporary directories.
///
/// `user_wide` is loaded first, then `per_project` types override user-wide
/// types with the same name. Missing files are not an error.
pub fn load_custom_types_from(
  user_wide: &Path,
  per_project: &Path,
) -> Result<Vec<CustomProjectType>> {
  let base = load_from_file(user_wide)?;
  let project = load_from_file(per_project)?;
  Ok(merge_custom_type_lists(&base, &project))
}

/// Merge custom project types with built-in `PackageManager` definitions.
///
/// Custom types whose name matches a built-in manager **replace** the
/// built-in. Custom types with unique names are appended to the list.
/// The resulting list is sorted by priority (highest first) to match the
/// built-in registry ordering.
///
/// # Examples
///
/// ```
/// use apmw_core::custom_types::{merge_with_builtins, CustomProjectType};
/// use apmw_core::detect::{all_managers, Ecosystem, HierarchyLevel};
///
/// let custom = vec![CustomProjectType {
///     name: "wgsl".to_string(),
///     display_name: "WGSL".to_string(),
///     ecosystem: Ecosystem::Unknown,
///     hierarchy: HierarchyLevel::BuildSystem,
///     primary_files: vec!["*.wgsl".to_string()],
///     secondary_files: vec![],
///     dir_markers: vec![],
///     priority: 50,
/// }];
///
/// let merged = merge_with_builtins(&custom, all_managers());
/// assert!(merged.iter().any(|m| m.name == "wgsl"));
/// assert!(merged.iter().any(|m| m.name == "cargo"));
/// ```
pub fn merge_with_builtins(
  custom: &[CustomProjectType],
  builtins: &[PackageManager],
) -> Vec<PackageManagerOwned> {
  let custom_names: HashMap<&str, ()> = custom.iter().map(|c| (c.name.as_str(), ())).collect();

  // Start with builtins that are NOT overridden by custom types.
  let mut result: Vec<PackageManagerOwned> = builtins
    .iter()
    .filter(|m| !custom_names.contains_key(m.name))
    .map(PackageManagerOwned::from_builtin)
    .collect();

  // Append custom types (including overrides).
  for c in custom {
    result.push(PackageManagerOwned::from_custom(c));
  }

  // Sort by priority descending (highest first), matching built-in ordering.
  result.sort_by_key(|m| std::cmp::Reverse(m.priority));

  result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use crate::detect::all_managers;
  use std::fs;
  use std::io::Write;
  use tempfile::TempDir;

  /// Write content to a file, creating parent directories as needed.
  fn write_file(dir: &Path, rel_path: &str, content: &str) -> PathBuf {
    let path = dir.join(rel_path);
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::File::create(&path)
      .and_then(|mut f| f.write_all(content.as_bytes()))
      .expect("failed to write file");
    path
  }

  // --- Loading tests ---

  #[test]
  fn test_load_one_custom_type() {
    let dir = TempDir::new().unwrap();
    let yaml = r#"
types:
  - name: wgsl
    display_name: "WGSL Shaders"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "*.wgsl"
    secondary_files:
      - "wgsl.toml"
    dir_markers:
      - "shaders"
    priority: 50
"#;
    let config_path = write_file(dir.path(), "project-types.yml", yaml);

    let types = load_from_file(&config_path).expect("load failed");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].name, "wgsl");
    assert_eq!(types[0].display_name, "WGSL Shaders");
    assert_eq!(types[0].ecosystem, Ecosystem::Unknown);
    assert_eq!(types[0].hierarchy, HierarchyLevel::BuildSystem);
    assert_eq!(types[0].primary_files, vec!["*.wgsl"]);
    assert_eq!(types[0].secondary_files, vec!["wgsl.toml"]);
    assert_eq!(types[0].dir_markers, vec!["shaders"]);
    assert_eq!(types[0].priority, 50);
  }

  #[test]
  fn test_load_multiple_custom_types() {
    let dir = TempDir::new().unwrap();
    let yaml = r#"
types:
  - name: wgsl
    display_name: "WGSL Shaders"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "*.wgsl"
    priority: 50
  - name: shaderc
    display_name: "ShaderC"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "shader.toml"
    priority: 40
"#;
    let config_path = write_file(dir.path(), "project-types.yml", yaml);

    let types = load_from_file(&config_path).expect("load failed");
    assert_eq!(types.len(), 2);
    assert_eq!(types[0].name, "wgsl");
    assert_eq!(types[1].name, "shaderc");
  }

  #[test]
  fn test_load_missing_file_returns_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.yml");
    let types = load_from_file(&path).expect("should not error on missing file");
    assert!(types.is_empty());
  }

  #[test]
  fn test_load_malformed_yaml_returns_error() {
    let dir = TempDir::new().unwrap();
    let malformed = r#"
types:
  - name: wgsl
    display_name: "WGSL"
    ecosystem: [invalid
    hierarchy: build_system
"#;
    let config_path = write_file(dir.path(), "project-types.yml", malformed);

    let result = load_from_file(&config_path);
    assert!(result.is_err(), "malformed YAML should return an error");
    let err = result.unwrap_err();
    assert!(
      err.to_string().contains("YAML error"),
      "error should mention YAML: {}",
      err
    );
  }

  #[test]
  fn test_load_empty_config() {
    let dir = TempDir::new().unwrap();
    let yaml = "types: []\n";
    let config_path = write_file(dir.path(), "project-types.yml", yaml);

    let types = load_from_file(&config_path).expect("load failed");
    assert!(types.is_empty());
  }

  #[test]
  fn test_load_config_with_default_fields() {
    let dir = TempDir::new().unwrap();
    // Only required fields — secondary_files, dir_markers, priority should default.
    let yaml = r#"
types:
  - name: minimal
    display_name: "Minimal"
    ecosystem: rust
    hierarchy: language
    primary_files:
      - "minimal.toml"
"#;
    let config_path = write_file(dir.path(), "project-types.yml", yaml);

    let types = load_from_file(&config_path).expect("load failed");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].name, "minimal");
    assert!(
      types[0].secondary_files.is_empty(),
      "secondary_files should default to empty"
    );
    assert!(
      types[0].dir_markers.is_empty(),
      "dir_markers should default to empty"
    );
    assert_eq!(types[0].priority, 0, "priority should default to 0");
  }

  // --- Merge with both paths tests ---

  #[test]
  fn test_load_from_both_paths_per_project_wins() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    let user_yaml = r#"
types:
  - name: wgsl
    display_name: "WGSL (user)"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "*.wgsl"
    priority: 30
  - name: shaderc
    display_name: "ShaderC (user)"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "shader.toml"
    priority: 20
"#;
    let user_path = write_file(user_dir.path(), "project-types.yml", user_yaml);

    let project_yaml = r#"
types:
  - name: wgsl
    display_name: "WGSL (project)"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "*.wgsl"
      - "*.comp"
    priority: 50
"#;
    let project_path = write_file(project_dir.path(), "project-types.yml", project_yaml);

    let types = load_custom_types_from(&user_path, &project_path).expect("load failed");

    // Should have 2 types: shaderc (from user) + wgsl (from project, overriding user).
    assert_eq!(types.len(), 2);

    let wgsl = types
      .iter()
      .find(|t| t.name == "wgsl")
      .expect("wgsl must exist");
    assert_eq!(
      wgsl.display_name, "WGSL (project)",
      "per-project should override user-wide"
    );
    assert_eq!(wgsl.priority, 50);
    assert!(wgsl.primary_files.contains(&"*.comp".to_string()));

    let shaderc = types
      .iter()
      .find(|t| t.name == "shaderc")
      .expect("shaderc must exist");
    assert_eq!(shaderc.display_name, "ShaderC (user)");
  }

  #[test]
  fn test_load_from_both_paths_both_missing() {
    let user_path = PathBuf::from("/nonexistent/user/project-types.yml");
    let project_path = PathBuf::from("/nonexistent/project/project-types.yml");

    let types = load_custom_types_from(&user_path, &project_path).expect("should not error");
    assert!(types.is_empty());
  }

  #[test]
  fn test_load_from_both_paths_only_user() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    let user_yaml = r#"
types:
  - name: wgsl
    display_name: "WGSL"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "*.wgsl"
    priority: 30
"#;
    let user_path = write_file(user_dir.path(), "project-types.yml", user_yaml);
    let project_path = project_dir.path().join("project-types.yml"); // does not exist

    let types = load_custom_types_from(&user_path, &project_path).expect("load failed");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].name, "wgsl");
  }

  #[test]
  fn test_load_from_both_paths_only_project() {
    let user_dir = TempDir::new().unwrap();
    let project_dir = TempDir::new().unwrap();

    let user_path = user_dir.path().join("project-types.yml"); // does not exist

    let project_yaml = r#"
types:
  - name: wgsl
    display_name: "WGSL"
    ecosystem: unknown
    hierarchy: build_system
    primary_files:
      - "*.wgsl"
    priority: 50
"#;
    let project_path = write_file(project_dir.path(), "project-types.yml", project_yaml);

    let types = load_custom_types_from(&user_path, &project_path).expect("load failed");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].name, "wgsl");
  }

  // --- merge_with_builtins tests ---

  #[test]
  fn test_merge_with_builtins_no_conflicts() {
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

    let merged = merge_with_builtins(&custom, all_managers());

    // Should contain both the custom type and all built-ins.
    assert!(merged.iter().any(|m| m.name == "wgsl"));
    assert!(merged.iter().any(|m| m.name == "cargo"));
    assert!(merged.iter().any(|m| m.name == "pnpm"));

    // Total count = built-ins + 1 custom.
    assert_eq!(merged.len(), all_managers().len() + 1);
  }

  #[test]
  fn test_merge_with_builtins_override_same_name() {
    let custom = vec![CustomProjectType {
      name: "cargo".to_string(),
      display_name: "Custom Cargo".to_string(),
      ecosystem: Ecosystem::Rust,
      hierarchy: HierarchyLevel::Language,
      primary_files: vec!["Cargo.toml".to_string(), "Cargo-custom.toml".to_string()],
      secondary_files: vec![],
      dir_markers: vec!["target".to_string()],
      priority: 100,
    }];

    let merged = merge_with_builtins(&custom, all_managers());

    // Should have exactly one "cargo" entry (the custom one, not the built-in).
    let cargo_entries: Vec<&PackageManagerOwned> =
      merged.iter().filter(|m| m.name == "cargo").collect();
    assert_eq!(
      cargo_entries.len(),
      1,
      "should have exactly one cargo entry"
    );
    assert_eq!(cargo_entries[0].display_name, "Custom Cargo");
    assert_eq!(cargo_entries[0].priority, 100);
    assert!(cargo_entries[0]
      .primary_files
      .contains(&"Cargo-custom.toml".to_string()));

    // Total count = built-ins (one removed) + 1 custom = same as built-ins.
    assert_eq!(merged.len(), all_managers().len());
  }

  #[test]
  fn test_merge_with_builtins_sorted_by_priority() {
    let custom = vec![
      CustomProjectType {
        name: "low".to_string(),
        display_name: "Low".to_string(),
        ecosystem: Ecosystem::Unknown,
        hierarchy: HierarchyLevel::BuildSystem,
        primary_files: vec!["low.toml".to_string()],
        secondary_files: vec![],
        dir_markers: vec![],
        priority: 1,
      },
      CustomProjectType {
        name: "high".to_string(),
        display_name: "High".to_string(),
        ecosystem: Ecosystem::Unknown,
        hierarchy: HierarchyLevel::BuildSystem,
        primary_files: vec!["high.toml".to_string()],
        secondary_files: vec![],
        dir_markers: vec![],
        priority: 100,
      },
    ];

    let merged = merge_with_builtins(&custom, all_managers());

    // Verify sorted by priority descending.
    for window in merged.windows(2) {
      assert!(
        window[0].priority >= window[1].priority,
        "merged list must be sorted by priority descending: {} >= {}",
        window[0].priority,
        window[1].priority
      );
    }
  }

  #[test]
  fn test_merge_with_builtins_empty_custom() {
    let merged = merge_with_builtins(&[], all_managers());
    assert_eq!(merged.len(), all_managers().len());
    // All built-ins should be present.
    assert!(merged.iter().any(|m| m.name == "cargo"));
    assert!(merged.iter().any(|m| m.name == "pnpm"));
  }

  #[test]
  fn test_merge_with_builtins_multiple_overrides() {
    let custom = vec![
      CustomProjectType {
        name: "cargo".to_string(),
        display_name: "Custom Cargo".to_string(),
        ecosystem: Ecosystem::Rust,
        hierarchy: HierarchyLevel::Language,
        primary_files: vec!["Cargo.toml".to_string()],
        secondary_files: vec![],
        dir_markers: vec![],
        priority: 100,
      },
      CustomProjectType {
        name: "pnpm".to_string(),
        display_name: "Custom pnpm".to_string(),
        ecosystem: Ecosystem::Node,
        hierarchy: HierarchyLevel::Language,
        primary_files: vec!["pnpm-lock.yaml".to_string()],
        secondary_files: vec![],
        dir_markers: vec![],
        priority: 99,
      },
    ];

    let merged = merge_with_builtins(&custom, all_managers());

    // Both overrides should replace their built-ins.
    let cargo = merged.iter().find(|m| m.name == "cargo").unwrap();
    assert_eq!(cargo.display_name, "Custom Cargo");
    let pnpm = merged.iter().find(|m| m.name == "pnpm").unwrap();
    assert_eq!(pnpm.display_name, "Custom pnpm");

    // Count = built-ins - 2 overridden + 2 custom = built-ins.
    assert_eq!(merged.len(), all_managers().len());
  }

  // --- merge_custom_type_lists tests ---

  #[test]
  fn test_merge_custom_type_lists_no_conflicts() {
    let base = vec![CustomProjectType {
      name: "a".to_string(),
      display_name: "A".to_string(),
      ecosystem: Ecosystem::Unknown,
      hierarchy: HierarchyLevel::BuildSystem,
      primary_files: vec![],
      secondary_files: vec![],
      dir_markers: vec![],
      priority: 10,
    }];
    let override_ = vec![CustomProjectType {
      name: "b".to_string(),
      display_name: "B".to_string(),
      ecosystem: Ecosystem::Unknown,
      hierarchy: HierarchyLevel::BuildSystem,
      primary_files: vec![],
      secondary_files: vec![],
      dir_markers: vec![],
      priority: 20,
    }];

    let merged = merge_custom_type_lists(&base, &override_);
    assert_eq!(merged.len(), 2);
    assert!(merged.iter().any(|c| c.name == "a"));
    assert!(merged.iter().any(|c| c.name == "b"));
  }

  #[test]
  fn test_merge_custom_type_lists_override_wins() {
    let base = vec![
      CustomProjectType {
        name: "a".to_string(),
        display_name: "A (base)".to_string(),
        ecosystem: Ecosystem::Unknown,
        hierarchy: HierarchyLevel::BuildSystem,
        primary_files: vec![],
        secondary_files: vec![],
        dir_markers: vec![],
        priority: 10,
      },
      CustomProjectType {
        name: "b".to_string(),
        display_name: "B (base)".to_string(),
        ecosystem: Ecosystem::Unknown,
        hierarchy: HierarchyLevel::BuildSystem,
        primary_files: vec![],
        secondary_files: vec![],
        dir_markers: vec![],
        priority: 20,
      },
    ];
    let override_ = vec![CustomProjectType {
      name: "a".to_string(),
      display_name: "A (override)".to_string(),
      ecosystem: Ecosystem::Unknown,
      hierarchy: HierarchyLevel::BuildSystem,
      primary_files: vec![],
      secondary_files: vec![],
      dir_markers: vec![],
      priority: 30,
    }];

    let merged = merge_custom_type_lists(&base, &override_);
    assert_eq!(merged.len(), 2);

    let a = merged.iter().find(|c| c.name == "a").unwrap();
    assert_eq!(a.display_name, "A (override)", "override should win");
    let b = merged.iter().find(|c| c.name == "b").unwrap();
    assert_eq!(b.display_name, "B (base)");
  }

  // --- Serialization round-trip test ---

  #[test]
  fn test_custom_project_type_serde_roundtrip() {
    let original = CustomProjectType {
      name: "wgsl".to_string(),
      display_name: "WGSL Shaders".to_string(),
      ecosystem: Ecosystem::Unknown,
      hierarchy: HierarchyLevel::BuildSystem,
      primary_files: vec!["*.wgsl".to_string()],
      secondary_files: vec!["wgsl.toml".to_string()],
      dir_markers: vec!["shaders".to_string()],
      priority: 50,
    };

    let yaml = serde_yaml::to_string(&original).expect("serialize failed");
    let deserialized: CustomProjectType = serde_yaml::from_str(&yaml).expect("deserialize failed");
    assert_eq!(original, deserialized);
  }
}
