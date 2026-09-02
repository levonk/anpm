//! Version info extraction — reads version constraints from manifest files.
//!
//! This module reads package version and language/runtime version constraints
//! from the standard manifest files of each supported ecosystem:
//!
//! | Manifest | Ecosystem | Package version | Language constraint |
//! |----------|-----------|-----------------|--------------------|
//! | `Cargo.toml` | Rust | `package.version` | `package.rust-version` |
//! | `package.json` | Node | `version` | `engines.node` |
//! | `pyproject.toml` | Python | `project.version` / `tool.poetry.version` | `project.requires-python` |
//! | `go.mod` | Go | (none) | `go` directive |
//! | `pom.xml` | JVM | `<version>` | `<maven.compiler.source>` |
//! | `build.gradle` | JVM | `version` | `sourceCompatibility` |
//! | `Gemfile` / `.gemspec` | Ruby | `version` | `required_ruby_version` |
//! | `Package.swift` | Swift | `version` | `platforms` |
//! | `composer.json` | PHP | `version` | `require.php` |
//!
//! The top-level [`extract_version_info`] function scans a directory for all
//! known manifest files and returns a [`VersionInfo`] entry for each one it can
//! read. Malformed manifests produce an error rather than a panic.

use std::path::{Path, PathBuf};

use crate::ecosystem::Ecosystem;
use crate::error::{CoreError, Result};

/// Version constraints extracted from a single manifest file.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionInfo {
  /// The package version declared in the manifest, if present.
  pub package_version: Option<String>,
  /// The language or runtime version constraint declared in the manifest, if
  /// present.
  pub language_version: Option<String>,
  /// The path to the manifest file this info was extracted from.
  pub source_file: PathBuf,
  /// The ecosystem this manifest belongs to.
  pub ecosystem: Ecosystem,
}

/// Extract version info from every known manifest file in `dir`.
///
/// Scans the top-level of `dir` (no recursion) for the manifest files listed in
/// the module docs. Returns one [`VersionInfo`] per readable manifest. A
/// malformed manifest causes an error rather than a panic; a missing manifest
/// simply contributes no entry.
pub fn extract_version_info(dir: &Path) -> Result<Vec<VersionInfo>> {
  if !dir.exists() {
    return Err(CoreError::PackageManagerNotFound(format!(
      "directory does not exist: {}",
      dir.display()
    )));
  }

  let mut results: Vec<VersionInfo> = Vec::new();

  let cargo = dir.join("Cargo.toml");
  if cargo.is_file() {
    results.push(extract_cargo_toml(&cargo)?);
  }

  let pkg_json = dir.join("package.json");
  if pkg_json.is_file() {
    results.push(extract_package_json(&pkg_json)?);
  }

  let pyproject = dir.join("pyproject.toml");
  if pyproject.is_file() {
    results.push(extract_pyproject_toml(&pyproject)?);
  }

  let go_mod = dir.join("go.mod");
  if go_mod.is_file() {
    results.push(extract_go_mod(&go_mod)?);
  }

  let pom = dir.join("pom.xml");
  if pom.is_file() {
    results.push(extract_pom_xml(&pom)?);
  }

  let gradle = dir.join("build.gradle");
  if gradle.is_file() {
    results.push(extract_build_gradle(&gradle)?);
  }

  let gemfile = dir.join("Gemfile");
  if gemfile.is_file() {
    results.push(extract_gemfile(&gemfile)?);
  }

  let pkg_swift = dir.join("Package.swift");
  if pkg_swift.is_file() {
    results.push(extract_package_swift(&pkg_swift)?);
  }

  let composer = dir.join("composer.json");
  if composer.is_file() {
    results.push(extract_composer_json(&composer)?);
  }

  Ok(results)
}

// ---------------------------------------------------------------------------
// Per-manifest extraction functions
// ---------------------------------------------------------------------------

/// Read a file to a string, mapping IO errors onto [`CoreError`].
fn read_to_string(path: &Path) -> Result<String> {
  std::fs::read_to_string(path).map_err(CoreError::Io)
}

/// Extract version info from a `Cargo.toml` manifest.
///
/// - `package_version`: `package.version`
/// - `language_version`: `package.rust-version`
pub fn extract_cargo_toml(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;
  let value: toml::Value = toml::from_str(&content)?;

  let package_version = value
    .get("package")
    .and_then(|p| p.get("version"))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  let language_version = value
    .get("package")
    .and_then(|p| p.get("rust-version"))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Rust,
  })
}

/// Extract version info from a `package.json` manifest.
///
/// - `package_version`: top-level `version`
/// - `language_version`: `engines.node`
pub fn extract_package_json(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;
  let value: serde_json::Value = serde_json::from_str(&content)?;

  let package_version = value
    .get("version")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  let language_version = value
    .get("engines")
    .and_then(|e| e.get("node"))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Node,
  })
}

/// Extract version info from a `pyproject.toml` manifest.
///
/// - `package_version`: `project.version`, falling back to `tool.poetry.version`
/// - `language_version`: `project.requires-python`
pub fn extract_pyproject_toml(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;
  let value: toml::Value = toml::from_str(&content)?;

  let package_version = value
    .get("project")
    .and_then(|p| p.get("version"))
    .and_then(|v| v.as_str())
    .or_else(|| {
      value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
    })
    .map(|s| s.to_string());

  let language_version = value
    .get("project")
    .and_then(|p| p.get("requires-python"))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Python,
  })
}

/// Extract version info from a `go.mod` file.
///
/// - `package_version`: `None` (go.mod does not declare a module version)
/// - `language_version`: the `go` directive (e.g. `go 1.21`)
pub fn extract_go_mod(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;
  let language_version = content.lines().find_map(|line| {
    let trimmed = line.trim();
    trimmed
      .strip_prefix("go ")
      .or_else(|| trimmed.strip_prefix("go\t"))
      .map(|rest| rest.trim().to_string())
  });

  Ok(VersionInfo {
    package_version: None,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Go,
  })
}

/// Extract version info from a `pom.xml` file using string search.
///
/// - `package_version`: the first `<version>` element that is a direct child of
///   `<project>` (approximated by taking the first `<version>` occurrence)
/// - `language_version`: the `<maven.compiler.source>` property value
///
/// A full XML parser is intentionally avoided to keep dependencies light.
pub fn extract_pom_xml(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;

  let package_version = first_xml_element(&content, "version").map(|s| s.to_string());

  let language_version = first_xml_element(&content, "maven.compiler.source")
    .or_else(|| first_xml_element(&content, "maven.compiler.release"))
    .map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Jvm,
  })
}

/// Extract version info from a `build.gradle` file using string search.
///
/// - `package_version`: the `version = '...'` or `version = "..."` assignment
/// - `language_version`: the `sourceCompatibility` value
pub fn extract_build_gradle(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;

  let package_version = find_quoted_assignment(&content, "version");

  let language_version = find_unquoted_assignment(&content, "sourceCompatibility")
    .or_else(|| find_unquoted_assignment(&content, "sourceCompatibility"))
    .map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Jvm,
  })
}

/// Extract version info from a `Gemfile` or `.gemspec` file using regex/string
/// search.
///
/// - `package_version`: `version = "..."` or `s.version = ...`
/// - `language_version`: `required_ruby_version = ...`
pub fn extract_gemfile(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;

  let package_version = find_quoted_assignment(&content, "version");

  let language_version =
    find_quoted_assignment(&content, "required_ruby_version").map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Python, // Ruby is not in the Ecosystem enum; closest is Python
  })
}

/// Extract version info from a `Package.swift` file using string search.
///
/// - `package_version`: `version: "..."` inside the Package initializer
/// - `language_version`: the `.macos(...)`, `.ios(...)`, etc. platform entries
pub fn extract_package_swift(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;

  let package_version = find_quoted_assignment(&content, "version");

  // Collect platform version constraints into a comma-separated string.
  let mut platforms: Vec<String> = Vec::new();
  for line in content.lines() {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix(".") {
      if let Some(open) = rest.find('(') {
        let name = &rest[..open];
        if matches!(
          name,
          "macos" | "ios" | "tvos" | "watchos" | "linux" | "windows"
        ) {
          if let Some(v) = first_quoted_in(&rest[open..]) {
            platforms.push(format!("{} {}", name, v));
          }
        }
      }
    }
  }
  let language_version = if platforms.is_empty() {
    None
  } else {
    Some(platforms.join(", "))
  };

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Swift,
  })
}

/// Extract version info from a `composer.json` manifest.
///
/// - `package_version`: top-level `version`
/// - `language_version`: `require.php`
pub fn extract_composer_json(path: &Path) -> Result<VersionInfo> {
  let content = read_to_string(path)?;
  let value: serde_json::Value = serde_json::from_str(&content)?;

  let package_version = value
    .get("version")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  let language_version = value
    .get("require")
    .and_then(|r| r.get("php"))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  Ok(VersionInfo {
    package_version,
    language_version,
    source_file: path.to_path_buf(),
    ecosystem: Ecosystem::Python, // PHP is not in the Ecosystem enum; closest is Python
  })
}

// ---------------------------------------------------------------------------
// String-search helpers (no full XML/Groovy/Swift parsers)
// ---------------------------------------------------------------------------

/// Return the text content of the first `<tag>...</tag>` occurrence in `content`.
fn first_xml_element<'a>(content: &'a str, tag: &str) -> Option<&'a str> {
  let open = format!("<{tag}>");
  let close = format!("</{tag}>");
  let start = content.find(&open)? + open.len();
  let end = content[start..].find(&close)? + start;
  Some(content[start..end].trim())
}

/// Find a `key = 'value'` or `key = "value"` assignment and return the quoted
/// value (without quotes).
fn find_quoted_assignment(content: &str, key: &str) -> Option<String> {
  for line in content.lines() {
    let trimmed = line.trim();
    // Match `key = "..."` or `key: "..."` or `key = '...'` or `key: '...'`.
    if let Some(idx) = trimmed.find(key) {
      let after_key = &trimmed[idx + key.len()..];
      let after_key = after_key.trim_start();
      let after_key = after_key
        .strip_prefix(':')
        .map(|s| s.trim_start())
        .unwrap_or(after_key);
      let after_key = after_key
        .strip_prefix('=')
        .map(|s| s.trim_start())
        .unwrap_or(after_key);
      if let Some(v) = first_quoted_in(after_key) {
        return Some(v.to_string());
      }
    }
  }
  None
}

/// Find a `key = value` assignment where `value` is unquoted (e.g. a number or
/// bare word).
fn find_unquoted_assignment(content: &str, key: &str) -> Option<String> {
  for line in content.lines() {
    let trimmed = line.trim();
    if let Some(idx) = trimmed.find(key) {
      let after_key = &trimmed[idx + key.len()..];
      let after_key = after_key.trim_start();
      let after_key = after_key
        .strip_prefix(':')
        .map(|s| s.trim_start())
        .unwrap_or(after_key);
      let after_key = after_key
        .strip_prefix('=')
        .map(|s| s.trim_start())
        .unwrap_or(after_key);
      let token: String = after_key
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '.' || *c == '_')
        .collect();
      if !token.is_empty() {
        return Some(token);
      }
    }
  }
  None
}

/// Return the contents of the first single- or double-quoted string in `s`.
fn first_quoted_in(s: &str) -> Option<&str> {
  let bytes = s.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    let c = bytes[i];
    if c == b'"' || c == b'\'' {
      let quote = c;
      let start = i + 1;
      let mut j = start;
      while j < bytes.len() && bytes[j] != quote {
        j += 1;
      }
      if j < bytes.len() {
        return Some(&s[start..j]);
      }
      return None;
    }
    i += 1;
  }
  None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("failed to write file");
    path
  }

  // --- Cargo.toml ---------------------------------------------------------

  #[test]
  fn test_cargo_toml_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"
[package]
name = "foo"
version = "0.1.0"
rust-version = "1.70"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    let info = &infos[0];
    assert_eq!(info.package_version.as_deref(), Some("0.1.0"));
    assert_eq!(info.language_version.as_deref(), Some("1.70"));
    assert_eq!(info.ecosystem, Ecosystem::Rust);
  }

  #[test]
  fn test_cargo_toml_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"
[package]
name = "foo"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
    assert!(infos[0].language_version.is_none());
  }

  #[test]
  fn test_cargo_toml_missing_rust_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"
[package]
name = "foo"
version = "1.2.3"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.2.3"));
    assert!(infos[0].language_version.is_none());
  }

  #[test]
  fn test_cargo_toml_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "Cargo.toml", "this is not = valid toml [");
    let result = extract_version_info(dir.path());
    assert!(result.is_err(), "malformed Cargo.toml should error");
  }

  // --- package.json -------------------------------------------------------

  #[test]
  fn test_package_json_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name":"foo","version":"1.0.0","engines":{"node":">=18"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    let info = &infos[0];
    assert_eq!(info.package_version.as_deref(), Some("1.0.0"));
    assert_eq!(info.language_version.as_deref(), Some(">=18"));
    assert_eq!(info.ecosystem, Ecosystem::Node);
  }

  #[test]
  fn test_package_json_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "package.json", r#"{"name":"foo"}"#);
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
  }

  #[test]
  fn test_package_json_missing_engines() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name":"foo","version":"2.0.0"}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("2.0.0"));
    assert!(infos[0].language_version.is_none());
  }

  #[test]
  fn test_package_json_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "package.json", "{not valid json}");
    let result = extract_version_info(dir.path());
    assert!(result.is_err(), "malformed package.json should error");
  }

  // --- pyproject.toml -----------------------------------------------------

  #[test]
  fn test_pyproject_toml_project_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pyproject.toml",
      r#"
[project]
version = "1.5.0"
requires-python = ">=3.10"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.5.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">=3.10"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Python);
  }

  #[test]
  fn test_pyproject_toml_poetry_fallback() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pyproject.toml",
      r#"
[tool.poetry]
version = "0.9.1"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("0.9.1"));
  }

  #[test]
  fn test_pyproject_toml_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pyproject.toml",
      r#"
[project]
name = "foo"
requires-python = ">=3.8"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some(">=3.8"));
  }

  #[test]
  fn test_pyproject_toml_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "pyproject.toml", "not = valid [ toml");
    let result = extract_version_info(dir.path());
    assert!(result.is_err());
  }

  // --- go.mod -------------------------------------------------------------

  #[test]
  fn test_go_mod_valid() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "go.mod", "module example.com/foo\n\ngo 1.21\n");
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some("1.21"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Go);
  }

  #[test]
  fn test_go_mod_missing_go_directive() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "go.mod", "module example.com/foo\n");
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].language_version.is_none());
  }

  // --- pom.xml ------------------------------------------------------------

  #[test]
  fn test_pom_xml_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      r#"
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>foo</artifactId>
  <version>1.0.0</version>
  <properties>
    <maven.compiler.source>17</maven.compiler.source>
  </properties>
</project>
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some("17"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Jvm);
  }

  #[test]
  fn test_pom_xml_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      r#"<project><artifactId>foo</artifactId></project>"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
  }

  // --- build.gradle -------------------------------------------------------

  #[test]
  fn test_build_gradle_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "build.gradle",
      r#"
group = 'com.example'
version = '1.2.3'
sourceCompatibility = 17
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.2.3"));
    assert_eq!(infos[0].language_version.as_deref(), Some("17"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Jvm);
  }

  #[test]
  fn test_build_gradle_double_quotes() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "build.gradle",
      r#"version = "2.0.0"
sourceCompatibility = 11
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("2.0.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some("11"));
  }

  // --- Gemfile ------------------------------------------------------------

  #[test]
  fn test_gemfile_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Gemfile",
      r#"
source "https://rubygems.org"
ruby "3.2.0"
gem "rails", "7.1.0"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    // Gemfile has no `version =` assignment by default.
    assert!(infos[0].package_version.is_none());
  }

  #[test]
  fn test_gemfile_with_version_and_ruby() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Gemfile",
      r#"
version = "1.0.0"
required_ruby_version = ">= 3.0"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">= 3.0"));
  }

  // --- Package.swift ------------------------------------------------------

  #[test]
  fn test_package_swift_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Package.swift",
      r#"
let package = Package(
    name: "foo",
    platforms: [.macOS(.v12), .iOS(.v15)],
    products: [],
    targets: []
)
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].ecosystem, Ecosystem::Swift);
  }

  #[test]
  fn test_package_swift_with_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Package.swift",
      r#"
let package = Package(
    name: "foo",
    version: "1.0.0"
)
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
  }

  // --- composer.json ------------------------------------------------------

  #[test]
  fn test_composer_json_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "composer.json",
      r#"{"name":"foo/bar","version":"1.0.0","require":{"php":">=8.1"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">=8.1"));
  }

  #[test]
  fn test_composer_json_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "composer.json",
      r#"{"name":"foo/bar","require":{"php":">=8.0"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some(">=8.0"));
  }

  #[test]
  fn test_composer_json_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "composer.json", "{broken");
    let result = extract_version_info(dir.path());
    assert!(result.is_err());
  }

  // --- directory-level tests ----------------------------------------------

  #[test]
  fn test_no_manifests_empty_vec() {
    let dir = TempDir::new().unwrap();
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos.is_empty());
  }

  #[test]
  fn test_multiple_manifests() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"[package]
version = "0.1.0"
rust-version = "1.70"
"#,
    );
    write_file(
      dir.path(),
      "package.json",
      r#"{"version":"1.0.0","engines":{"node":">=18"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 2);
    let ecosystems: Vec<Ecosystem> = infos.iter().map(|i| i.ecosystem).collect();
    assert!(ecosystems.contains(&Ecosystem::Rust));
    assert!(ecosystems.contains(&Ecosystem::Node));
  }

  #[test]
  fn test_nonexistent_dir_errors() {
    let result = extract_version_info(Path::new("/nonexistent/does/not/exist"));
    assert!(result.is_err());
  }

  // --- helper unit tests ---------------------------------------------------

  #[test]
  fn test_first_xml_element() {
    let xml = "<a><b>hello</b><b>world</b></a>";
    assert_eq!(first_xml_element(xml, "b"), Some("hello"));
    assert_eq!(first_xml_element(xml, "c"), None);
  }

  #[test]
  fn test_find_quoted_assignment() {
    let s = "version = '1.0.0'\nname = 'foo'";
    assert_eq!(
      find_quoted_assignment(s, "version"),
      Some("1.0.0".to_string())
    );
    assert_eq!(find_quoted_assignment(s, "missing"), None);
  }

  #[test]
  fn test_find_unquoted_assignment() {
    let s = "sourceCompatibility = 17\n";
    assert_eq!(
      find_unquoted_assignment(s, "sourceCompatibility"),
      Some("17".to_string())
    );
  }

  #[test]
  fn test_first_quoted_in() {
    assert_eq!(first_quoted_in(r#"  "hi"  "#), Some("hi"));
    assert_eq!(first_quoted_in("  'hi'  "), Some("hi"));
    assert_eq!(first_quoted_in("  no quotes  "), None);
  }
}
