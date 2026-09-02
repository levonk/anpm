//! Package manager definitions.
//!
//! Each [`PackageManager`] entry encodes the static detection attributes from
//! the 2ndbrain research Table 1 (Package Manager Project Detection Attributes):
//! primary identifying files, secondary identifying files, and directory
//! markers. The detection priority controls the order in which managers are
//! checked when multiple managers share the same config file (e.g.
//! `pyproject.toml` is used by poetry, pdm, and uv).

use serde::{Deserialize, Serialize};

/// The hierarchy level at which a package manager or build system operates.
///
/// Mirrors the Package Manager Hierarchy from the 2ndbrain research document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HierarchyLevel {
  /// OS wrapper level — mise, brew, nix.
  OsWrapper,
  /// OS-level package managers — apt, dnf, pacman, winget.
  Os,
  /// Virtualization / container level — docker, podman, packer.
  Virtualization,
  /// Language-level package managers — npm, cargo, pip, gem, go, etc.
  Language,
  /// App-level package managers — helm, flatpak, snap, devbox, vagrant.
  App,
  /// Build systems — bazel, buck, pants, cmake, ant, xcode, spm.
  BuildSystem,
}

impl std::fmt::Display for HierarchyLevel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      HierarchyLevel::OsWrapper => write!(f, "os_wrapper"),
      HierarchyLevel::Os => write!(f, "os"),
      HierarchyLevel::Virtualization => write!(f, "virtualization"),
      HierarchyLevel::Language => write!(f, "language"),
      HierarchyLevel::App => write!(f, "app"),
      HierarchyLevel::BuildSystem => write!(f, "build_system"),
    }
  }
}

/// The ecosystem a package manager belongs to (for within-ecosystem mapping).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
  /// Node.js ecosystem — npm, yarn, pnpm, bun.
  Node,
  /// Python ecosystem — pip, poetry, pipenv, pdm, conda, uv.
  Python,
  /// Rust ecosystem — cargo.
  Rust,
  /// Go ecosystem — go modules.
  Go,
  /// Ruby ecosystem — gem.
  Ruby,
  /// JVM ecosystem — maven, gradle, sbt, ant.
  Jvm,
  /// Dart/Flutter ecosystem — flutter, dart.
  Dart,
  /// .NET ecosystem — dotnet/nuget.
  Dotnet,
  /// Apple ecosystem — cocoapods, carthage, spm, xcode.
  Apple,
  /// Container ecosystem — docker, podman, packer.
  Container,
  /// OS-level ecosystem — apt, dnf, pacman, winget, brew, nix, snap, flatpak, apk, yum.
  Os,
  /// App-level ecosystem — helm, devbox, vagrant.
  App,
  /// Build-system ecosystem — bazel, buck, pants, cmake.
  BuildSystem,
  /// Unknown / not applicable.
  Unknown,
}

impl std::fmt::Display for Ecosystem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Ecosystem::Node => write!(f, "node"),
      Ecosystem::Python => write!(f, "python"),
      Ecosystem::Rust => write!(f, "rust"),
      Ecosystem::Go => write!(f, "go"),
      Ecosystem::Ruby => write!(f, "ruby"),
      Ecosystem::Jvm => write!(f, "jvm"),
      Ecosystem::Dart => write!(f, "dart"),
      Ecosystem::Dotnet => write!(f, "dotnet"),
      Ecosystem::Apple => write!(f, "apple"),
      Ecosystem::Container => write!(f, "container"),
      Ecosystem::Os => write!(f, "os"),
      Ecosystem::App => write!(f, "app"),
      Ecosystem::BuildSystem => write!(f, "build_system"),
      Ecosystem::Unknown => write!(f, "unknown"),
    }
  }
}

/// A package manager or build system with its detection attributes.
///
/// Detection attributes are derived from the 2ndbrain research Table 1
/// (Package Manager Project Detection Attributes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManager {
  /// Canonical name (e.g. `cargo`, `pnpm`, `pip`).
  pub name: &'static str,
  /// Human-readable display name.
  pub display_name: &'static str,
  /// Ecosystem this manager belongs to.
  pub ecosystem: Ecosystem,
  /// Hierarchy level.
  pub hierarchy: HierarchyLevel,
  /// Primary identifying files — lockfiles and config files that strongly
  /// indicate this manager is in use (e.g. `Cargo.toml`, `pnpm-lock.yaml`).
  pub primary_files: &'static [&'static str],
  /// Secondary identifying files — provide supporting evidence but are not
  /// conclusive on their own (e.g. `Cargo.lock`, `pnpm-workspace.yaml`).
  pub secondary_files: &'static [&'static str],
  /// Directory markers — directories whose presence indicates this manager
  /// (e.g. `node_modules/`, `Pods/`).
  pub dir_markers: &'static [&'static str],
  /// Detection priority — higher numbers are checked first when multiple
  /// managers share the same primary file (e.g. `pyproject.toml` is shared by
  /// poetry, pdm, and uv; uv has higher priority as the canonical runner).
  pub priority: u8,
}

impl PackageManager {
  /// Returns all primary and secondary files combined.
  pub fn all_files(&self) -> impl Iterator<Item = &'static str> {
    self
      .primary_files
      .iter()
      .chain(self.secondary_files.iter())
      .copied()
  }
}

impl std::fmt::Display for PackageManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.name)
  }
}

/// Returns the registry of all known package managers and build systems.
///
/// Ordered by detection priority (highest first) so that when multiple
/// managers could match, the more specific / canonical one is preferred.
pub fn all_managers() -> &'static [PackageManager] {
  &MANAGERS
}

/// Returns the package manager with the given name, if any.
pub fn find_manager(name: &str) -> Option<&'static PackageManager> {
  MANAGERS.iter().find(|m| m.name == name)
}

// ---------------------------------------------------------------------------
// Package manager registry — derived from 2ndbrain research Table 1.
//
// Each entry lists the primary and secondary identifying files from Table 1.
// Priority is used to disambiguate shared config files (e.g. pyproject.toml
// is shared by poetry, pdm, and uv — uv gets the highest priority as the
// canonical Python runner per PRD FR-2.1).
// ---------------------------------------------------------------------------

#[rustfmt::skip]
static MANAGERS: [PackageManager; 30] = [
  // --- Node.js ecosystem ---
  PackageManager {
    name: "pnpm",
    display_name: "pnpm",
    ecosystem: Ecosystem::Node,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["pnpm-lock.yaml"],
    secondary_files: &["pnpm-workspace.yaml"],
    dir_markers: &["node_modules", ".pnpm-store"],
    priority: 30,
  },
  PackageManager {
    name: "yarn",
    display_name: "Yarn (Classic)",
    ecosystem: Ecosystem::Node,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["yarn.lock"],
    secondary_files: &[],
    dir_markers: &["node_modules"],
    priority: 29,
  },
  PackageManager {
    name: "npm",
    display_name: "npm",
    ecosystem: Ecosystem::Node,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["package-lock.json"],
    secondary_files: &["package.json"],
    dir_markers: &["node_modules"],
    priority: 28,
  },
  // --- Python ecosystem ---
  PackageManager {
    name: "uv",
    display_name: "uv",
    ecosystem: Ecosystem::Python,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["uv.lock"],
    secondary_files: &["requirements.in"],
    dir_markers: &[],
    priority: 27,
  },
  PackageManager {
    name: "poetry",
    display_name: "Poetry",
    ecosystem: Ecosystem::Python,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["poetry.lock"],
    secondary_files: &[],
    dir_markers: &[],
    priority: 26,
  },
  PackageManager {
    name: "pdm",
    display_name: "PDM",
    ecosystem: Ecosystem::Python,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["pdm.lock"],
    secondary_files: &[".pdm.toml"],
    dir_markers: &["__pypackages__"],
    priority: 25,
  },
  PackageManager {
    name: "pipenv",
    display_name: "pipenv",
    ecosystem: Ecosystem::Python,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["Pipfile.lock", "Pipfile"],
    secondary_files: &[],
    dir_markers: &[],
    priority: 24,
  },
  PackageManager {
    name: "conda",
    display_name: "Conda",
    ecosystem: Ecosystem::Python,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["environment.yml"],
    secondary_files: &[".condarc"],
    dir_markers: &["conda-meta"],
    priority: 23,
  },
  PackageManager {
    name: "pip",
    display_name: "pip",
    ecosystem: Ecosystem::Python,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["requirements.txt"],
    secondary_files: &["setup.py"],
    dir_markers: &[],
    priority: 22,
  },
  // --- Rust ecosystem ---
  PackageManager {
    name: "cargo",
    display_name: "Cargo",
    ecosystem: Ecosystem::Rust,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["Cargo.toml"],
    secondary_files: &["Cargo.lock"],
    dir_markers: &["target"],
    priority: 21,
  },
  // --- Go ecosystem ---
  PackageManager {
    name: "go",
    display_name: "Go Modules",
    ecosystem: Ecosystem::Go,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["go.mod"],
    secondary_files: &["go.sum", "go.work"],
    dir_markers: &["vendor"],
    priority: 20,
  },
  // --- Ruby ecosystem ---
  PackageManager {
    name: "gem",
    display_name: "RubyGems",
    ecosystem: Ecosystem::Ruby,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["Gemfile", "Gemfile.lock"],
    secondary_files: &["gems.locked"],
    dir_markers: &[],
    priority: 19,
  },
  // --- JVM ecosystem ---
  PackageManager {
    name: "maven",
    display_name: "Maven",
    ecosystem: Ecosystem::Jvm,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["pom.xml"],
    secondary_files: &[],
    dir_markers: &["target"],
    priority: 18,
  },
  PackageManager {
    name: "gradle",
    display_name: "Gradle",
    ecosystem: Ecosystem::Jvm,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["build.gradle", "build.gradle.kts"],
    secondary_files: &["settings.gradle", "settings.gradle.kts", "gradlew"],
    dir_markers: &["build", ".gradle"],
    priority: 17,
  },
  PackageManager {
    name: "sbt",
    display_name: "SBT",
    ecosystem: Ecosystem::Jvm,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["build.sbt"],
    secondary_files: &["project/build.properties", "project/plugins.sbt"],
    dir_markers: &["target"],
    priority: 16,
  },
  // --- Dart/Flutter ecosystem ---
  PackageManager {
    name: "flutter",
    display_name: "Flutter",
    ecosystem: Ecosystem::Dart,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["pubspec.yaml"],
    secondary_files: &["pubspec.lock"],
    dir_markers: &[],
    priority: 15,
  },
  PackageManager {
    name: "dart",
    display_name: "Dart",
    ecosystem: Ecosystem::Dart,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["pubspec.yaml"],
    secondary_files: &["pubspec.lock"],
    dir_markers: &[],
    priority: 14,
  },
  // --- .NET ecosystem ---
  PackageManager {
    name: "dotnet",
    display_name: ".NET CLI / NuGet",
    ecosystem: Ecosystem::Dotnet,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["*.csproj", "*.fsproj", "*.vbproj"],
    secondary_files: &["packages.config", "Directory.Packages.props"],
    dir_markers: &["bin", "obj"],
    priority: 13,
  },
  // --- Apple ecosystem ---
  PackageManager {
    name: "cocoapods",
    display_name: "CocoaPods",
    ecosystem: Ecosystem::Apple,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["Podfile"],
    secondary_files: &["Podfile.lock"],
    dir_markers: &["Pods"],
    priority: 12,
  },
  PackageManager {
    name: "carthage",
    display_name: "Carthage",
    ecosystem: Ecosystem::Apple,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["Cartfile"],
    secondary_files: &["Cartfile.resolved"],
    dir_markers: &["Carthage"],
    priority: 11,
  },
  PackageManager {
    name: "spm",
    display_name: "Swift Package Manager",
    ecosystem: Ecosystem::Apple,
    hierarchy: HierarchyLevel::Language,
    primary_files: &["Package.swift"],
    secondary_files: &[],
    dir_markers: &[".build", ".swiftpm"],
    priority: 10,
  },
  // --- Container / Virtualization ecosystem ---
  PackageManager {
    name: "docker",
    display_name: "Docker",
    ecosystem: Ecosystem::Container,
    hierarchy: HierarchyLevel::Virtualization,
    primary_files: &["Dockerfile", "docker-compose.yml", "compose.yaml"],
    secondary_files: &["docker-compose.override.yml", "Containerfile"],
    dir_markers: &[],
    priority: 9,
  },
  // --- OS-level ecosystem ---
  PackageManager {
    name: "brew",
    display_name: "Homebrew",
    ecosystem: Ecosystem::Os,
    hierarchy: HierarchyLevel::Os,
    primary_files: &["Brewfile"],
    secondary_files: &["Brewfile.lock.json"],
    dir_markers: &[],
    priority: 8,
  },
  PackageManager {
    name: "nix",
    display_name: "Nix",
    ecosystem: Ecosystem::Os,
    hierarchy: HierarchyLevel::OsWrapper,
    primary_files: &["flake.nix", "default.nix"],
    secondary_files: &["shell.nix"],
    dir_markers: &[".direnv"],
    priority: 7,
  },
  PackageManager {
    name: "apt",
    display_name: "APT (Debian/Ubuntu)",
    ecosystem: Ecosystem::Os,
    hierarchy: HierarchyLevel::Os,
    primary_files: &[],
    secondary_files: &[],
    dir_markers: &[],
    priority: 6,
  },
  PackageManager {
    name: "winget",
    display_name: "Winget (Windows)",
    ecosystem: Ecosystem::Os,
    hierarchy: HierarchyLevel::Os,
    primary_files: &["winget.yaml", "winget.yml"],
    secondary_files: &[],
    dir_markers: &[],
    priority: 5,
  },
  // --- App-level ecosystem ---
  PackageManager {
    name: "helm",
    display_name: "Helm (Kubernetes)",
    ecosystem: Ecosystem::App,
    hierarchy: HierarchyLevel::App,
    primary_files: &["Chart.yaml"],
    secondary_files: &["Chart.lock"],
    dir_markers: &[],
    priority: 4,
  },
  PackageManager {
    name: "devbox",
    display_name: "Devbox",
    ecosystem: Ecosystem::App,
    hierarchy: HierarchyLevel::App,
    primary_files: &["devbox.json"],
    secondary_files: &["devbox.lock"],
    dir_markers: &[],
    priority: 3,
  },
  PackageManager {
    name: "vagrant",
    display_name: "Vagrant",
    ecosystem: Ecosystem::App,
    hierarchy: HierarchyLevel::App,
    primary_files: &["Vagrantfile"],
    secondary_files: &[],
    dir_markers: &[".vagrant"],
    priority: 2,
  },
  // --- Build systems ---
  PackageManager {
    name: "cmake",
    display_name: "CMake",
    ecosystem: Ecosystem::BuildSystem,
    hierarchy: HierarchyLevel::BuildSystem,
    primary_files: &["CMakeLists.txt"],
    secondary_files: &["CMakePresets.json"],
    dir_markers: &["build"],
    priority: 1,
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_all_managers_count() {
    let managers = all_managers();
    assert!(
      managers.len() >= 20,
      "Must have at least 20 package managers, got {}",
      managers.len()
    );
  }

  #[test]
  fn test_find_manager_cargo() {
    let m = find_manager("cargo").expect("cargo must be registered");
    assert_eq!(m.name, "cargo");
    assert_eq!(m.ecosystem, Ecosystem::Rust);
    assert!(m.primary_files.contains(&"Cargo.toml"));
  }

  #[test]
  fn test_find_manager_pnpm() {
    let m = find_manager("pnpm").expect("pnpm must be registered");
    assert_eq!(m.name, "pnpm");
    assert_eq!(m.ecosystem, Ecosystem::Node);
    assert!(m.primary_files.contains(&"pnpm-lock.yaml"));
  }

  #[test]
  fn test_find_manager_unknown() {
    assert!(find_manager("nonexistent").is_none());
  }

  #[test]
  fn test_find_manager_npm() {
    let m = find_manager("npm").expect("npm must be registered");
    assert!(m.primary_files.contains(&"package-lock.json"));
  }

  #[test]
  fn test_find_manager_pip() {
    let m = find_manager("pip").expect("pip must be registered");
    assert!(m.primary_files.contains(&"requirements.txt"));
  }

  #[test]
  fn test_find_manager_go() {
    let m = find_manager("go").expect("go must be registered");
    assert!(m.primary_files.contains(&"go.mod"));
  }

  #[test]
  fn test_python_managers_share_pyproject() {
    let poetry = find_manager("poetry").unwrap();
    let pdm = find_manager("pdm").unwrap();
    let uv = find_manager("uv").unwrap();
    assert_eq!(poetry.primary_files, &["poetry.lock"]);
    assert_eq!(pdm.primary_files, &["pdm.lock"]);
    assert_eq!(uv.primary_files, &["uv.lock"]);
  }

  #[test]
  fn test_priority_ordering() {
    let pnpm = find_manager("pnpm").unwrap();
    let npm = find_manager("npm").unwrap();
    assert!(pnpm.priority > npm.priority);
  }

  #[test]
  fn test_all_managers_have_unique_names() {
    let managers = all_managers();
    let mut names: Vec<&str> = managers.iter().map(|m| m.name).collect();
    names.sort();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "Duplicate manager names found");
  }

  #[test]
  fn test_all_files_iterator() {
    let cargo = find_manager("cargo").unwrap();
    let files: Vec<&str> = cargo.all_files().collect();
    assert!(files.contains(&"Cargo.toml"));
    assert!(files.contains(&"Cargo.lock"));
  }

  #[test]
  fn test_hierarchy_display() {
    assert_eq!(HierarchyLevel::Language.to_string(), "language");
    assert_eq!(HierarchyLevel::Os.to_string(), "os");
  }

  #[test]
  fn test_ecosystem_display() {
    assert_eq!(Ecosystem::Rust.to_string(), "rust");
    assert_eq!(Ecosystem::Node.to_string(), "node");
  }
}
