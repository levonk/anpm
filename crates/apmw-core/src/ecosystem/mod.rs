//! Ecosystem core types — package-manager ecosystems, canonical commands,
//! and package-manager identifiers.
//!
//! This module defines the shared core types used by detection, version
//! resolution, and the within-ecosystem command mapping table (which lives in
//! the `apmw` binary crate, not here).
//!
//! # Core principle: WITHIN-ecosystem only
//!
//! Each package manager belongs to exactly one ecosystem. Within-ecosystem
//! mapping never crosses ecosystem boundaries (PRD FR-2.1). A Python runner
//! maps to a Python runner; a Node runner maps to a Node runner. For example:
//!
//! | Source (ecosystem) | Canonical (same ecosystem) |
//! |-------------------|----------------------------|
//! | `pip` (Python)    | `uv` (Python)              |
//! | `npm` (Node)      | `pnpm` (Node)              |
//! | `yarn` (Node)     | `pnpm` (Node)              |
//! | `bun` (Node)      | `pnpm` (Node)              |
//! | `yarn2` (Node)    | `pnpm` (Node)              |

use serde::{Deserialize, Serialize};

/// A package-manager ecosystem.
///
/// Each package manager belongs to exactly one ecosystem. Within-ecosystem
/// mapping never crosses ecosystem boundaries (PRD FR-2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ecosystem {
  /// Python ecosystem — canonical manager: `uv`.
  Python,
  /// Node.js ecosystem — canonical manager: `pnpm`.
  Node,
  /// Rust ecosystem — canonical manager: `cargo`.
  Rust,
  /// Go ecosystem — canonical manager: `go`.
  Go,
  /// Ruby ecosystem — canonical manager: `gem`.
  Ruby,
  /// PHP ecosystem — canonical manager: `composer`.
  Php,
  /// JVM ecosystem (Java, Kotlin, Scala) — managers: maven, gradle, sbt.
  Jvm,
  /// Swift / Apple ecosystem — managers: swiftpm, cocoapods, carthage.
  Swift,
  /// .NET ecosystem — manager: dotnet.
  Dotnet,
  /// Flutter / Dart ecosystem — managers: flutter, dart.
  Flutter,
  /// Polyglot build systems — managers: bazel, ant.
  Polyglot,
}

impl Ecosystem {
  /// Returns the canonical package manager for this ecosystem.
  pub fn canonical_manager(self) -> PackageManager {
    match self {
      Ecosystem::Python => PackageManager::Uv,
      Ecosystem::Node => PackageManager::Pnpm,
      Ecosystem::Rust => PackageManager::Cargo,
      Ecosystem::Go => PackageManager::Go,
      Ecosystem::Ruby => PackageManager::Gem,
      Ecosystem::Php => PackageManager::Composer,
      Ecosystem::Jvm => PackageManager::Maven,
      Ecosystem::Swift => PackageManager::SwiftPm,
      Ecosystem::Dotnet => PackageManager::Dotnet,
      Ecosystem::Flutter => PackageManager::Flutter,
      Ecosystem::Polyglot => PackageManager::Bazel,
    }
  }
}

impl std::fmt::Display for Ecosystem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Ecosystem::Python => write!(f, "python"),
      Ecosystem::Node => write!(f, "node"),
      Ecosystem::Rust => write!(f, "rust"),
      Ecosystem::Go => write!(f, "go"),
      Ecosystem::Ruby => write!(f, "ruby"),
      Ecosystem::Php => write!(f, "php"),
      Ecosystem::Jvm => write!(f, "jvm"),
      Ecosystem::Swift => write!(f, "swift"),
      Ecosystem::Dotnet => write!(f, "dotnet"),
      Ecosystem::Flutter => write!(f, "flutter"),
      Ecosystem::Polyglot => write!(f, "polyglot"),
    }
  }
}

/// The canonical `apmw` commands (from 2ndbrain Table 2 row labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApmwCommand {
  Add,
  AddDev,
  Install,
  List,
  Outdated,
  Remove,
  UpdateAll,
  Update,
  Test,
  Build,
  Init,
}

impl ApmwCommand {
  pub fn label(self) -> &'static str {
    match self {
      ApmwCommand::Add => "add <pkg>",
      ApmwCommand::AddDev => "add-dev <pkg>",
      ApmwCommand::Install => "install",
      ApmwCommand::List => "list",
      ApmwCommand::Outdated => "outdated",
      ApmwCommand::Remove => "remove <pkg>",
      ApmwCommand::UpdateAll => "update-all",
      ApmwCommand::Update => "update <pkg>",
      ApmwCommand::Test => "test",
      ApmwCommand::Build => "build",
      ApmwCommand::Init => "init",
    }
  }
}

impl std::fmt::Display for ApmwCommand {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.label())
  }
}

/// A package manager identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum PackageManager {
  Npm,
  Yarn,
  Yarn2,
  Pnpm,
  Bun,
  Pip,
  Poetry,
  Pipenv,
  Pdm,
  Conda,
  Uv,
  Cargo,
  Go,
  Gem,
  Composer,
  Maven,
  Gradle,
  Sbt,
  SwiftPm,
  CocoaPods,
  Carthage,
  Dotnet,
  Flutter,
  Dart,
  Bazel,
  Ant,
}

impl PackageManager {
  pub fn ecosystem(self) -> Ecosystem {
    match self {
      PackageManager::Npm
      | PackageManager::Yarn
      | PackageManager::Yarn2
      | PackageManager::Pnpm
      | PackageManager::Bun => Ecosystem::Node,
      PackageManager::Pip
      | PackageManager::Poetry
      | PackageManager::Pipenv
      | PackageManager::Pdm
      | PackageManager::Conda
      | PackageManager::Uv => Ecosystem::Python,
      PackageManager::Cargo => Ecosystem::Rust,
      PackageManager::Go => Ecosystem::Go,
      PackageManager::Gem => Ecosystem::Ruby,
      PackageManager::Composer => Ecosystem::Php,
      PackageManager::Maven | PackageManager::Gradle | PackageManager::Sbt => Ecosystem::Jvm,
      PackageManager::SwiftPm | PackageManager::CocoaPods | PackageManager::Carthage => {
        Ecosystem::Swift
      }
      PackageManager::Dotnet => Ecosystem::Dotnet,
      PackageManager::Flutter | PackageManager::Dart => Ecosystem::Flutter,
      PackageManager::Bazel | PackageManager::Ant => Ecosystem::Polyglot,
    }
  }

  pub fn is_canonical(self) -> bool {
    self.ecosystem().canonical_manager() == self
  }

  pub fn as_str(self) -> &'static str {
    match self {
      PackageManager::Npm => "npm",
      PackageManager::Yarn => "yarn",
      PackageManager::Yarn2 => "yarn2",
      PackageManager::Pnpm => "pnpm",
      PackageManager::Bun => "bun",
      PackageManager::Pip => "pip",
      PackageManager::Poetry => "poetry",
      PackageManager::Pipenv => "pipenv",
      PackageManager::Pdm => "pdm",
      PackageManager::Conda => "conda",
      PackageManager::Uv => "uv",
      PackageManager::Cargo => "cargo",
      PackageManager::Go => "go",
      PackageManager::Gem => "gem",
      PackageManager::Composer => "composer",
      PackageManager::Maven => "maven",
      PackageManager::Gradle => "gradle",
      PackageManager::Sbt => "sbt",
      PackageManager::SwiftPm => "swiftpm",
      PackageManager::CocoaPods => "cocoapods",
      PackageManager::Carthage => "carthage",
      PackageManager::Dotnet => "dotnet",
      PackageManager::Flutter => "flutter",
      PackageManager::Dart => "dart",
      PackageManager::Bazel => "bazel",
      PackageManager::Ant => "ant",
    }
  }

  pub fn parse_manager(s: &str) -> Option<Self> {
    match s.to_ascii_lowercase().as_str() {
      "npm" => Some(PackageManager::Npm),
      "yarn" => Some(PackageManager::Yarn),
      "yarn2" | "yarn-berry" | "yarnberry" => Some(PackageManager::Yarn2),
      "pnpm" => Some(PackageManager::Pnpm),
      "bun" => Some(PackageManager::Bun),
      "pip" => Some(PackageManager::Pip),
      "poetry" => Some(PackageManager::Poetry),
      "pipenv" => Some(PackageManager::Pipenv),
      "pdm" => Some(PackageManager::Pdm),
      "conda" => Some(PackageManager::Conda),
      "uv" => Some(PackageManager::Uv),
      "cargo" => Some(PackageManager::Cargo),
      "go" => Some(PackageManager::Go),
      "gem" | "bundler" => Some(PackageManager::Gem),
      "composer" => Some(PackageManager::Composer),
      "maven" | "mvn" => Some(PackageManager::Maven),
      "gradle" => Some(PackageManager::Gradle),
      "sbt" => Some(PackageManager::Sbt),
      "swiftpm" | "swift-pm" | "swift" => Some(PackageManager::SwiftPm),
      "cocoapods" | "pod" | "cocoapod" => Some(PackageManager::CocoaPods),
      "carthage" => Some(PackageManager::Carthage),
      "dotnet" => Some(PackageManager::Dotnet),
      "flutter" => Some(PackageManager::Flutter),
      "dart" => Some(PackageManager::Dart),
      "bazel" => Some(PackageManager::Bazel),
      "ant" => Some(PackageManager::Ant),
      _ => None,
    }
  }
}

impl std::fmt::Display for PackageManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// Returns all known package managers.
pub fn all_managers() -> Vec<PackageManager> {
  vec![
    PackageManager::Npm,
    PackageManager::Yarn,
    PackageManager::Yarn2,
    PackageManager::Pnpm,
    PackageManager::Bun,
    PackageManager::Pip,
    PackageManager::Poetry,
    PackageManager::Pipenv,
    PackageManager::Pdm,
    PackageManager::Conda,
    PackageManager::Uv,
    PackageManager::Cargo,
    PackageManager::Go,
    PackageManager::Gem,
    PackageManager::Composer,
    PackageManager::Maven,
    PackageManager::Gradle,
    PackageManager::Sbt,
    PackageManager::SwiftPm,
    PackageManager::CocoaPods,
    PackageManager::Carthage,
    PackageManager::Dotnet,
    PackageManager::Flutter,
    PackageManager::Dart,
    PackageManager::Bazel,
    PackageManager::Ant,
  ]
}

/// Returns all known ecosystems.
pub fn all_ecosystems() -> Vec<Ecosystem> {
  vec![
    Ecosystem::Python,
    Ecosystem::Node,
    Ecosystem::Rust,
    Ecosystem::Go,
    Ecosystem::Ruby,
    Ecosystem::Php,
    Ecosystem::Jvm,
    Ecosystem::Swift,
    Ecosystem::Dotnet,
    Ecosystem::Flutter,
    Ecosystem::Polyglot,
  ]
}

/// Returns all canonical `apmw` commands.
pub fn all_commands() -> Vec<ApmwCommand> {
  vec![
    ApmwCommand::Add,
    ApmwCommand::AddDev,
    ApmwCommand::Install,
    ApmwCommand::List,
    ApmwCommand::Outdated,
    ApmwCommand::Remove,
    ApmwCommand::UpdateAll,
    ApmwCommand::Update,
    ApmwCommand::Test,
    ApmwCommand::Build,
    ApmwCommand::Init,
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_every_manager_has_ecosystem() {
    for m in all_managers() {
      let eco = m.ecosystem();
      assert_eq!(eco.canonical_manager().ecosystem(), eco);
    }
  }

  #[test]
  fn test_python_ecosystem_boundaries() {
    assert_eq!(PackageManager::Pip.ecosystem(), Ecosystem::Python);
    assert_eq!(PackageManager::Uv.ecosystem(), Ecosystem::Python);
    assert_ne!(PackageManager::Pip.ecosystem(), Ecosystem::Node);
  }

  #[test]
  fn test_node_ecosystem_boundaries() {
    for m in [
      PackageManager::Npm,
      PackageManager::Yarn,
      PackageManager::Yarn2,
      PackageManager::Bun,
      PackageManager::Pnpm,
    ] {
      assert_eq!(m.ecosystem(), Ecosystem::Node);
      assert_ne!(m.ecosystem(), Ecosystem::Python);
    }
  }

  #[test]
  fn test_python_canonical_is_uv() {
    assert_eq!(Ecosystem::Python.canonical_manager(), PackageManager::Uv);
    assert_ne!(Ecosystem::Python.canonical_manager(), PackageManager::Pnpm);
  }

  #[test]
  fn test_node_canonical_is_pnpm() {
    assert_eq!(Ecosystem::Node.canonical_manager(), PackageManager::Pnpm);
    assert_ne!(Ecosystem::Node.canonical_manager(), PackageManager::Uv);
  }

  #[test]
  fn test_is_canonical() {
    assert!(PackageManager::Uv.is_canonical());
    assert!(PackageManager::Pnpm.is_canonical());
    assert!(PackageManager::Cargo.is_canonical());
    assert!(!PackageManager::Pip.is_canonical());
    assert!(!PackageManager::Npm.is_canonical());
  }

  #[test]
  fn test_from_str_roundtrip() {
    for m in all_managers() {
      let s = m.as_str();
      let parsed = PackageManager::parse_manager(s).unwrap();
      assert_eq!(parsed, m);
    }
  }

  #[test]
  fn test_from_str_case_insensitive() {
    assert_eq!(
      PackageManager::parse_manager("NPM"),
      Some(PackageManager::Npm)
    );
    assert_eq!(
      PackageManager::parse_manager("UV"),
      Some(PackageManager::Uv)
    );
  }

  #[test]
  fn test_from_str_unknown() {
    assert_eq!(PackageManager::parse_manager("unknown-pm"), None);
    assert_eq!(PackageManager::parse_manager(""), None);
  }
}
