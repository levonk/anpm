//! Version info extraction — reads version constraints from manifest files.
//!
//! Given a project directory, [`extract_version_info`] detects which manifest
//! files are present (Cargo.toml, package.json, pyproject.toml, go.mod,
//! pom.xml, build.gradle, Gemfile / .gemspec, Package.swift, composer.json)
//! and extracts the package version and language/runtime version constraint
//! from each into a [`VersionInfo`] struct.
//!
//! TOML manifests are parsed with the `toml` crate, JSON manifests with
//! `serde_json`, and the remaining text-based manifests (go.mod, pom.xml,
//! build.gradle, Gemfile, Package.swift) are parsed with lightweight
//! string-search heuristics — no full grammar parser is required for version
//! extraction.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ecosystem::Ecosystem;
use crate::error::{CoreError, Result};

/// Version constraints extracted from a single manifest file.
///
/// Both `package_version` and `language_version` are optional — a manifest may
/// declare a package version without a language constraint, or vice versa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
  /// The package version declared in the manifest (e.g. `package.version` in
  /// Cargo.toml, `version` in package.json).
  pub package_version: Option<String>,
  /// The language/runtime version constraint declared in the manifest (e.g.
  /// `package.rust-version` in Cargo.toml, `engines.node` in package.json,
  /// `project.requires-python` in pyproject.toml).
  pub language_version: Option<String>,
  /// The manifest file the info was extracted from.
  pub source_file: PathBuf,
  /// The ecosystem this manifest belongs to.
  pub ecosystem: Ecosystem,
}

impl VersionInfo {
  /// Create a new `VersionInfo` with the given source file and ecosystem.
  fn new(source_file: PathBuf, ecosystem: Ecosystem) -> Self {
    VersionInfo {
      package_version: None,
      language_version: None,
      source_file,
      ecosystem,
    }
  }
}

/// Extract version info from all manifest files found in `dir`.
///
/// Scans the top level of `dir` for known manifest files and extracts version
/// info from each. Returns one [`VersionInfo`] per manifest found. If no
/// manifest files are present, returns an empty vec. If a manifest is present
/// but malformed, returns an error (does not panic).
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
  } else {
    // Fall back to a .gemspec file if present.
    if let Some(gemspec) = find_gemspec(dir) {
      results.push(extract_gemfile(&gemspec)?);
    }
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

/// Extract version info from a `Cargo.toml` manifest.
///
/// - `package_version` ← `package.version`
/// - `language_version` ← `package.rust-version`
fn extract_cargo_toml(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let value: toml::Value = toml::from_str(&content).map_err(CoreError::from)?;

  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Rust);
  if let Some(pkg) = value.get("package").and_then(|v| v.as_table()) {
    info.package_version = pkg.get("version").and_then(|v| v.as_str()).map(String::from);
    info.language_version = pkg
      .get("rust-version")
      .and_then(|v| v.as_str())
      .map(String::from);
  }
  Ok(info)
}

/// Extract version info from a `package.json` manifest.
///
/// - `package_version` ← `version`
/// - `language_version` ← `engines.node`
fn extract_package_json(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let value: serde_json::Value = serde_json::from_str(&content).map_err(CoreError::from)?;

  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Node);
  info.package_version = value
    .get("version")
    .and_then(|v| v.as_str())
    .map(String::from);
  info.language_version = value
    .get("engines")
    .and_then(|e| e.get("node"))
    .and_then(|v| v.as_str())
    .map(String::from);
  Ok(info)
}

/// Extract version info from a `pyproject.toml` manifest.
///
/// - `package_version` ← `project.version` (falls back to `tool.poetry.version`)
/// - `language_version` ← `project.requires-python`
fn extract_pyproject_toml(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let value: toml::Value = toml::from_str(&content).map_err(CoreError::from)?;

  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Python);

  // Package version: project.version, then tool.poetry.version.
  info.package_version = value
    .get("project")
    .and_then(|p| p.get("version"))
    .and_then(|v| v.as_str())
    .map(String::from)
    .or_else(|| {
      value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
    });

  info.language_version = value
    .get("project")
    .and_then(|p| p.get("requires-python"))
    .and_then(|v| v.as_str())
    .map(String::from);

  Ok(info)
}

/// Extract version info from a `go.mod` file.
///
/// - `package_version` ← `None` (go.mod has no package version)
/// - `language_version` ← the `go` directive (e.g. `go 1.21`)
fn extract_go_mod(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Go);

  for line in content.lines() {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("go ") {
      let version = rest.trim();
      if !version.is_empty() {
        info.language_version = Some(version.to_string());
        break;
      }
    }
  }
  Ok(info)
}

/// Extract version info from a `pom.xml` file.
///
/// - `package_version` ← the first `<version>` element at the project level
///   (skipping any `<version>` inside a `<parent>` block)
/// - `language_version` ← `<maven.compiler.source>` value
fn extract_pom_xml(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Jvm);

  // Package version: first <version> outside a <parent> block.
  info.package_version = first_project_version(&content);

  // Language version: <maven.compiler.source>...</maven.compiler.source>.
  info.language_version = extract_tag_content(&content, "maven.compiler.source")
    .or_else(|| extract_property(&content, "maven.compiler.source"));

  Ok(info)
}

/// Extract version info from a `build.gradle` file.
///
/// - `package_version` ← a `version = '...'` / `version "..."` assignment
/// - `language_version` ← `sourceCompatibility = ...`
fn extract_build_gradle(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Jvm);

  info.package_version = extract_quoted_after(&content, "version");
  info.language_version = extract_assignment_value(&content, "sourceCompatibility");

  Ok(info)
}

/// Extract version info from a `Gemfile` or `.gemspec` file.
///
/// - `package_version` ← a `version = "..."` / `s.version = ...` assignment
/// - `language_version` ← `required_ruby_version` value
fn extract_gemfile(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Ruby);

  info.package_version = extract_quoted_after(&content, "version");
  info.language_version = extract_quoted_after(&content, "required_ruby_version");

  Ok(info)
}

/// Extract version info from a `Package.swift` file.
///
/// - `package_version` ← a `version: "..."` token (the `version` argument in
///   `.package(url:..., from:/"version":...)` or `.target`/`.executableTarget`)
/// - `language_version` ← the `platforms:` declaration (e.g.
///   `.macOS("13.0")`) — captured as the first platform string
fn extract_package_swift(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Swift);

  info.package_version = extract_quoted_after(&content, "version");
  info.language_version = extract_first_platform(&content);

  Ok(info)
}

/// Extract version info from a `composer.json` manifest.
///
/// - `package_version` ← `version`
/// - `language_version` ← `require.php`
fn extract_composer_json(path: &Path) -> Result<VersionInfo> {
  let content = std::fs::read_to_string(path)?;
  let value: serde_json::Value = serde_json::from_str(&content).map_err(CoreError::from)?;

  let mut info = VersionInfo::new(path.to_path_buf(), Ecosystem::Php);
  info.package_version = value
    .get("version")
    .and_then(|v| v.as_str())
    .map(String::from);
  info.language_version = value
    .get("require")
    .and_then(|r| r.get("php"))
    .and_then(|v| v.as_str())
    .map(String::from);
  Ok(info)
}

// ---------------------------------------------------------------------------
// String-search helpers (no regex dependency)
// ---------------------------------------------------------------------------

/// Find the first `.gemspec` file in a directory, if any.
fn find_gemspec(dir: &Path) -> Option<PathBuf> {
  let entries = std::fs::read_dir(dir).ok()?;
  for entry in entries.flatten() {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    if name.ends_with(".gemspec") {
      return Some(entry.path());
    }
  }
  None
}

/// Extract the content of the first `<tag>...</tag>` occurrence.
fn extract_tag_content(content: &str, tag: &str) -> Option<String> {
  let open = format!("<{tag}>");
  let close = format!("</{tag}>");
  let start = content.find(&open)? + open.len();
  let end = content[start..].find(&close)? + start;
  let text = content[start..end].trim();
  if text.is_empty() {
    None
  } else {
    Some(text.to_string())
  }
}

/// Extract a Maven property `<name>value</name>` where name may appear as a
/// property tag. This is a thin alias used for `maven.compiler.source`.
fn extract_property(content: &str, name: &str) -> Option<String> {
  extract_tag_content(content, name)
}

/// Find the first `<version>` at the project level, skipping any `<version>`
/// nested inside a `<parent>` element.
fn first_project_version(content: &str) -> Option<String> {
  let open = "<version>";
  let close = "</version>";
  let parent_open = "<parent>";
  let parent_close = "</parent>";

  let mut idx = 0usize;
  while let Some(rel) = content[idx..].find(open) {
    let start = idx + rel;
    // Determine whether we are currently inside a <parent> block.
    let last_parent_open = content[..start].rfind(parent_open);
    let last_parent_close = content[..start].rfind(parent_close);
    let inside_parent = match (last_parent_open, last_parent_close) {
      (Some(o), Some(c)) => o > c,
      (Some(_), None) => true,
      _ => false,
    };

    let value_start = start + open.len();
    if let Some(rel_end) = content[value_start..].find(close) {
      let value_end = value_start + rel_end;
      let text = content[value_start..value_end].trim();
      if !inside_parent && !text.is_empty() {
        return Some(text.to_string());
      }
      idx = value_end + close.len();
    } else {
      break;
    }
  }
  None
}

/// Extract a quoted value that follows a keyword, e.g. `version "1.0.0"`,
/// `version = "1.0.0"`, `version = '1.0.0'`, or `version: "1.0.0"`. Searches
/// for the keyword, then scans the remainder of that line for the next quoted
/// string (single or double).
fn extract_quoted_after(content: &str, keyword: &str) -> Option<String> {
  let mut search_from = 0usize;
  loop {
    let rel = content[search_from..].find(keyword)?;
    let after = search_from + rel + keyword.len();
    let rest = &content[after..];
    let line_end = rest.find('\n').unwrap_or(rest.len());
    let line = &rest[..line_end];
    if let Some(val) = next_quoted_string_anywhere(line) {
      return Some(val);
    }
    // No quoted string on this occurrence's line — continue searching.
    search_from = after;
  }
}

/// Extract the right-hand side of an `identifier = <value>` assignment,
/// trimming whitespace and a trailing comment. Returns the raw token (which
/// may be a number, a quoted string, or a bare identifier).
fn extract_assignment_value(content: &str, identifier: &str) -> Option<String> {
  let mut search_from = 0usize;
  loop {
    let rel = content[search_from..].find(identifier)?;
    let start = search_from + rel + identifier.len();
    let rest = &content[start..];
    // Skip whitespace then expect '='.
    let mut chars = rest.char_indices();
    let mut eq_pos = None;
    for (i, c) in chars.by_ref() {
      if c.is_whitespace() {
        continue;
      }
      if c == '=' {
        eq_pos = Some(i);
      }
      break;
    }
    let Some(eq_rel) = eq_pos else {
      search_from = start;
      continue;
    };
    let after_eq = &rest[eq_rel + 1..];
    let token = next_token(after_eq);
    if let Some(t) = token {
      return Some(t);
    }
    search_from = start;
  }
}

/// Return the next quoted string (single or double quoted) in `s`, skipping
/// leading whitespace.
fn next_quoted_string(s: &str) -> Option<String> {
  let trimmed = s.trim_start();
  let bytes = trimmed.as_bytes();
  if bytes.is_empty() {
    return None;
  }
  let quote = bytes[0];
  if quote != b'"' && quote != b'\'' {
    return None;
  }
  let inner = &trimmed[1..];
  let end = inner.find(quote as char)?;
  Some(inner[..end].to_string())
}

/// Return the next bare token in `s`: a quoted string, or a run of
/// non-whitespace characters (stopping at a comment `//` or `#`).
fn next_token(s: &str) -> Option<String> {
  let trimmed = s.trim_start();
  if trimmed.is_empty() {
    return None;
  }
  // Quoted string?
  let first = trimmed.as_bytes()[0];
  if first == b'"' || first == b'\'' {
    return next_quoted_string(trimmed);
  }
  // Bare token up to whitespace, comma, or comment.
  let mut end = trimmed.len();
  for (i, c) in trimmed.char_indices() {
    if c.is_whitespace() || c == ',' || c == ';' {
      end = i;
      break;
    }
    if c == '/' && trimmed[i..].starts_with("//") {
      end = i;
      break;
    }
    if c == '#' {
      end = i;
      break;
    }
  }
  let token = trimmed[..end].trim_end_matches(',');
  if token.is_empty() {
    None
  } else {
    Some(token.to_string())
  }
}

/// Extract the first platform version from a `platforms: [...]` declaration,
/// e.g. `.macOS("13.0")` → `Some("13.0")`.
fn extract_first_platform(content: &str) -> Option<String> {
  let keyword = "platforms";
  let rel = content.find(keyword)?;
  let after = &content[rel + keyword.len()..];
  // Find the first quoted string after `platforms`.
  next_quoted_string_anywhere(after)
}

/// Find the first quoted string anywhere in `s` (single or double quoted).
fn next_quoted_string_anywhere(s: &str) -> Option<String> {
  let bytes = s.as_bytes();
  let mut i = 0usize;
  while i < bytes.len() {
    let c = bytes[i];
    if c == b'"' || c == b'\'' {
      let inner = &s[i + 1..];
      if let Some(end) = inner.find(c as char) {
        return Some(inner[..end].to_string());
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

  // --- Cargo.toml ---

  #[test]
  fn test_cargo_toml_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"[package]
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
      r#"[package]
name = "foo"
rust-version = "1.70"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some("1.70"));
  }

  #[test]
  fn test_cargo_toml_missing_language() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"[package]
name = "foo"
version = "0.1.0"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("0.1.0"));
    assert!(infos[0].language_version.is_none());
  }

  #[test]
  fn test_cargo_toml_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "Cargo.toml", "this is not = valid toml = [");
    let result = extract_version_info(dir.path());
    assert!(result.is_err(), "malformed Cargo.toml should error");
  }

  // --- package.json ---

  #[test]
  fn test_package_json_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "foo", "version": "1.0.0", "engines": {"node": ">=18"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">=18"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Node);
  }

  #[test]
  fn test_package_json_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "foo", "engines": {"node": ">=18"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some(">=18"));
  }

  #[test]
  fn test_package_json_missing_engines() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "package.json", r#"{"name": "foo", "version": "1.0.0"}"#);
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert!(infos[0].language_version.is_none());
  }

  #[test]
  fn test_package_json_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "package.json", "{not valid json}");
    let result = extract_version_info(dir.path());
    assert!(result.is_err());
  }

  // --- pyproject.toml ---

  #[test]
  fn test_pyproject_project_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pyproject.toml",
      r#"[project]
name = "foo"
version = "2.3.4"
requires-python = ">=3.10"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("2.3.4"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">=3.10"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Python);
  }

  #[test]
  fn test_pyproject_poetry_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pyproject.toml",
      r#"[tool.poetry]
name = "foo"
version = "0.9.0"

[project]
name = "foo"
requires-python = ">=3.8"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("0.9.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">=3.8"));
  }

  #[test]
  fn test_pyproject_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pyproject.toml",
      r#"[project]
name = "foo"
requires-python = ">=3.10"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some(">=3.10"));
  }

  #[test]
  fn test_pyproject_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "pyproject.toml", "not = = valid");
    let result = extract_version_info(dir.path());
    assert!(result.is_err());
  }

  // --- go.mod ---

  #[test]
  fn test_go_mod_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "go.mod",
      "module example.com/foo\n\ngo 1.21\n\nrequire (\n\tgithub.com/pkg/errors v0.9.1\n)\n",
    );
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
    assert!(infos[0].language_version.is_none());
  }

  // --- pom.xml ---

  #[test]
  fn test_pom_xml_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>foo</artifactId>
  <version>1.2.3</version>
  <properties>
    <maven.compiler.source>17</maven.compiler.source>
  </properties>
</project>"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.2.3"));
    assert_eq!(infos[0].language_version.as_deref(), Some("17"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Jvm);
  }

  #[test]
  fn test_pom_xml_skips_parent_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      r#"<project>
  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.1.0</version>
  </parent>
  <groupId>com.example</groupId>
  <artifactId>foo</artifactId>
  <version>0.4.2</version>
</project>"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("0.4.2"));
  }

  #[test]
  fn test_pom_xml_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      r#"<project>
  <groupId>com.example</groupId>
  <artifactId>foo</artifactId>
</project>"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos[0].package_version.is_none());
  }

  #[test]
  fn test_pom_xml_missing_compiler_source() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "pom.xml",
      r#"<project>
  <artifactId>foo</artifactId>
  <version>1.0.0</version>
</project>"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert!(infos[0].language_version.is_none());
  }

  // --- build.gradle ---

  #[test]
  fn test_build_gradle_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "build.gradle",
      r#"plugins { id 'java' }
version = '1.5.0'
sourceCompatibility = 17
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.5.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some("17"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Jvm);
  }

  #[test]
  fn test_build_gradle_double_quotes() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "build.gradle",
      r#"version "2.0.0"
sourceCompatibility = "11"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("2.0.0"));
    assert_eq!(infos[0].language_version.as_deref(), Some("11"));
  }

  #[test]
  fn test_build_gradle_missing_fields() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "build.gradle", "plugins { id 'java' }\n");
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos[0].package_version.is_none());
    assert!(infos[0].language_version.is_none());
  }

  // --- Gemfile / .gemspec ---

  #[test]
  fn test_gemfile_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Gemfile",
      r#"source "https://rubygems.org"
ruby "3.2.0"
gem "rails", "7.1.0"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].ecosystem, Ecosystem::Ruby);
    // Gemfile has no package version; required_ruby_version absent here.
    assert!(infos[0].package_version.is_none());
  }

  #[test]
  fn test_gemspec_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "foo.gemspec",
      r#"Gem::Specification.new do |s|
  s.name = "foo"
  s.version = "0.3.1"
  s.required_ruby_version = ">= 3.0"
end
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("0.3.1"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">= 3.0"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Ruby);
  }

  #[test]
  fn test_gemfile_with_required_ruby_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Gemfile",
      r#"source "https://rubygems.org"
required_ruby_version = ">= 3.1"
"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].language_version.as_deref(), Some(">= 3.1"));
  }

  // --- Package.swift ---

  #[test]
  fn test_package_swift_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Package.swift",
      r#"let package = Package(
    name: "foo",
    platforms: [.macOS("13.0"), .iOS("16.0")],
    products: [],
    targets: []
)"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].ecosystem, Ecosystem::Swift);
    assert_eq!(infos[0].language_version.as_deref(), Some("13.0"));
  }

  #[test]
  fn test_package_swift_with_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Package.swift",
      r#"let package = Package(
    name: "foo",
    dependencies: [
        .package(url: "https://example.com/lib", version: "1.2.0"),
    ]
)"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("1.2.0"));
  }

  #[test]
  fn test_package_swift_missing_platforms() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Package.swift",
      r#"let package = Package(name: "foo")"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos[0].language_version.is_none());
  }

  // --- composer.json ---

  #[test]
  fn test_composer_json_valid() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "composer.json",
      r#"{"name": "foo/bar", "version": "1.4.2", "require": {"php": ">=8.1"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].package_version.as_deref(), Some("1.4.2"));
    assert_eq!(infos[0].language_version.as_deref(), Some(">=8.1"));
    assert_eq!(infos[0].ecosystem, Ecosystem::Php);
  }

  #[test]
  fn test_composer_json_missing_version() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "composer.json",
      r#"{"name": "foo/bar", "require": {"php": ">=8.1"}}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos[0].package_version.is_none());
    assert_eq!(infos[0].language_version.as_deref(), Some(">=8.1"));
  }

  #[test]
  fn test_composer_json_missing_php() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "composer.json",
      r#"{"name": "foo/bar", "version": "1.0.0"}"#,
    );
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos[0].package_version.as_deref(), Some("1.0.0"));
    assert!(infos[0].language_version.is_none());
  }

  #[test]
  fn test_composer_json_malformed() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "composer.json", "{bad json}");
    let result = extract_version_info(dir.path());
    assert!(result.is_err());
  }

  // --- Multiple manifests / no manifests ---

  #[test]
  fn test_multiple_manifests() {
    let dir = TempDir::new().unwrap();
    write_file(
      dir.path(),
      "Cargo.toml",
      r#"[package]
name = "foo"
version = "0.1.0"
rust-version = "1.70"
"#,
    );
    write_file(
      dir.path(),
      "package.json",
      r#"{"name": "foo", "version": "1.0.0", "engines": {"node": ">=18"}}"#,
    );
    write_file(dir.path(), "go.mod", "module example.com/foo\n\ngo 1.21\n");
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert_eq!(infos.len(), 3);
    let ecosystems: Vec<Ecosystem> = infos.iter().map(|i| i.ecosystem).collect();
    assert!(ecosystems.contains(&Ecosystem::Rust));
    assert!(ecosystems.contains(&Ecosystem::Node));
    assert!(ecosystems.contains(&Ecosystem::Go));
  }

  #[test]
  fn test_no_manifests() {
    let dir = TempDir::new().unwrap();
    write_file(dir.path(), "README.md", "# hello");
    let infos = extract_version_info(dir.path()).expect("extraction failed");
    assert!(infos.is_empty());
  }

  #[test]
  fn test_nonexistent_dir_errors() {
    let result = extract_version_info(Path::new("/nonexistent/path/that/does/not/exist"));
    assert!(result.is_err());
  }

  #[test]
  fn test_version_info_serde_roundtrip() {
    let info = VersionInfo {
      package_version: Some("1.0.0".to_string()),
      language_version: Some(">=18".to_string()),
      source_file: PathBuf::from("package.json"),
      ecosystem: Ecosystem::Node,
    };
    let json = serde_json::to_string(&info).expect("serialize");
    let back: VersionInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(info, back);
  }
}
