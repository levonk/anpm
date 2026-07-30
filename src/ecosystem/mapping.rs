//! Ecosystem mapping table — within-ecosystem command translations.
//!
//! This module defines the data structures and the static mapping table that
//! translates canonical `apmw` commands into per-package-manager commands.
//! The table is derived from the 2ndbrain research Table 2 ("Package Manager
//! Command Reference") in the All Package Manager Wrapper Tool Landscape
//! document.
//!
//! # Core principle: WITHIN-ecosystem only
//!
//! Mappings are **never** cross-ecosystem. A Python runner maps to a Python
//! runner; a Node runner maps to a Node runner. For example, `pip` maps to
//! `uv` (both Python), and `npm` maps to `pnpm` (both Node), but `uvx` (Python)
//! must NEVER map to `pnpm dlx` (Node). Each ecosystem's commands map within
//! that ecosystem only (PRD FR-2.1).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// A within-ecosystem mapping entry for a single source package manager.
#[derive(Debug, Clone)]
pub struct EcosystemMap {
  pub source_manager: PackageManager,
  pub canonical_manager: PackageManager,
  pub command_mapping: HashMap<ApmwCommand, Option<&'static str>>,
}

impl EcosystemMap {
  pub fn new(source: PackageManager) -> Self {
    let canonical = source.ecosystem().canonical_manager();
    Self {
      source_manager: source,
      canonical_manager: canonical,
      command_mapping: HashMap::new(),
    }
  }

  pub fn command(&self, cmd: ApmwCommand) -> Option<&'static str> {
    self.command_mapping.get(&cmd).copied().flatten()
  }
}

/// Returns the concrete command string for a (manager, apmw_command) pair.
pub fn manager_command(manager: PackageManager, cmd: ApmwCommand) -> Option<&'static str> {
  match (manager, cmd) {
    (PackageManager::Npm, ApmwCommand::Add) => Some("npm install <pkg>"),
    (PackageManager::Npm, ApmwCommand::AddDev) => Some("npm install --save-dev <pkg>"),
    (PackageManager::Npm, ApmwCommand::Install) => Some("npm install"),
    (PackageManager::Npm, ApmwCommand::List) => Some("npm list"),
    (PackageManager::Npm, ApmwCommand::Outdated) => Some("npm outdated"),
    (PackageManager::Npm, ApmwCommand::Remove) => Some("npm uninstall <pkg>"),
    (PackageManager::Npm, ApmwCommand::UpdateAll) => Some("npm update"),
    (PackageManager::Npm, ApmwCommand::Update) => Some("npm update <pkg>"),
    (PackageManager::Npm, ApmwCommand::Test) => Some("npm test"),
    (PackageManager::Npm, ApmwCommand::Build) => Some("npm run build"),
    (PackageManager::Npm, ApmwCommand::Init) => Some("npm init"),
    (PackageManager::Yarn, ApmwCommand::Add) => Some("yarn add <pkg>"),
    (PackageManager::Yarn, ApmwCommand::AddDev) => Some("yarn add --dev <pkg>"),
    (PackageManager::Yarn, ApmwCommand::Install) => Some("yarn install"),
    (PackageManager::Yarn, ApmwCommand::List) => Some("yarn list"),
    (PackageManager::Yarn, ApmwCommand::Outdated) => Some("yarn outdated"),
    (PackageManager::Yarn, ApmwCommand::Remove) => Some("yarn remove <pkg>"),
    (PackageManager::Yarn, ApmwCommand::UpdateAll) => Some("yarn upgrade"),
    (PackageManager::Yarn, ApmwCommand::Update) => Some("yarn upgrade <pkg>"),
    (PackageManager::Yarn, ApmwCommand::Test) => Some("yarn test"),
    (PackageManager::Yarn, ApmwCommand::Build) => Some("yarn build"),
    (PackageManager::Yarn, ApmwCommand::Init) => Some("yarn init"),
    (PackageManager::Yarn2, ApmwCommand::Add) => Some("yarn add <pkg>"),
    (PackageManager::Yarn2, ApmwCommand::AddDev) => Some("yarn add --dev <pkg>"),
    (PackageManager::Yarn2, ApmwCommand::Install) => Some("yarn install"),
    (PackageManager::Yarn2, ApmwCommand::List) => Some("yarn list"),
    (PackageManager::Yarn2, ApmwCommand::Outdated) => Some("yarn outdated"),
    (PackageManager::Yarn2, ApmwCommand::Remove) => Some("yarn remove <pkg>"),
    (PackageManager::Yarn2, ApmwCommand::UpdateAll) => Some("yarn upgrade"),
    (PackageManager::Yarn2, ApmwCommand::Update) => Some("yarn upgrade <pkg>"),
    (PackageManager::Yarn2, ApmwCommand::Test) => Some("yarn test"),
    (PackageManager::Yarn2, ApmwCommand::Build) => Some("yarn build"),
    (PackageManager::Yarn2, ApmwCommand::Init) => Some("yarn init"),
    (PackageManager::Pnpm, ApmwCommand::Add) => Some("pnpm add <pkg>"),
    (PackageManager::Pnpm, ApmwCommand::AddDev) => Some("pnpm add -D <pkg>"),
    (PackageManager::Pnpm, ApmwCommand::Install) => Some("pnpm install"),
    (PackageManager::Pnpm, ApmwCommand::List) => Some("pnpm list"),
    (PackageManager::Pnpm, ApmwCommand::Outdated) => Some("pnpm outdated"),
    (PackageManager::Pnpm, ApmwCommand::Remove) => Some("pnpm remove <pkg>"),
    (PackageManager::Pnpm, ApmwCommand::UpdateAll) => Some("pnpm update"),
    (PackageManager::Pnpm, ApmwCommand::Update) => Some("pnpm update <pkg>"),
    (PackageManager::Pnpm, ApmwCommand::Test) => Some("pnpm test"),
    (PackageManager::Pnpm, ApmwCommand::Build) => Some("pnpm build"),
    (PackageManager::Pnpm, ApmwCommand::Init) => Some("pnpm init"),
    (PackageManager::Bun, ApmwCommand::Add) => Some("bun add <pkg>"),
    (PackageManager::Bun, ApmwCommand::AddDev) => Some("bun add --dev <pkg>"),
    (PackageManager::Bun, ApmwCommand::Install) => Some("bun install"),
    (PackageManager::Bun, ApmwCommand::List) => Some("bun pm ls"),
    (PackageManager::Bun, ApmwCommand::Outdated) => Some("bun pm outdated"),
    (PackageManager::Bun, ApmwCommand::Remove) => Some("bun remove <pkg>"),
    (PackageManager::Bun, ApmwCommand::UpdateAll) => Some("bun update"),
    (PackageManager::Bun, ApmwCommand::Update) => Some("bun update <pkg>"),
    (PackageManager::Bun, ApmwCommand::Test) => Some("bun test"),
    (PackageManager::Bun, ApmwCommand::Build) => Some("bun build"),
    (PackageManager::Bun, ApmwCommand::Init) => Some("bun init"),
    (PackageManager::Pip, ApmwCommand::Add) => Some("pip install <pkg>"),
    (PackageManager::Pip, ApmwCommand::AddDev) => Some("pip install <pkg>"),
    (PackageManager::Pip, ApmwCommand::Install) => Some("pip install -r requirements.txt"),
    (PackageManager::Pip, ApmwCommand::List) => Some("pip list"),
    (PackageManager::Pip, ApmwCommand::Outdated) => Some("pip list --outdated"),
    (PackageManager::Pip, ApmwCommand::Remove) => Some("pip uninstall <pkg>"),
    (PackageManager::Pip, ApmwCommand::UpdateAll) => {
      Some("pip install --upgrade -r requirements.txt")
    }
    (PackageManager::Pip, ApmwCommand::Update) => Some("pip install --upgrade <pkg>"),
    (PackageManager::Pip, ApmwCommand::Test) => Some("python -m pytest"),
    (PackageManager::Pip, ApmwCommand::Build) => Some("python -m build"),
    (PackageManager::Pip, ApmwCommand::Init) => None,
    (PackageManager::Poetry, ApmwCommand::Add) => Some("poetry add <pkg>"),
    (PackageManager::Poetry, ApmwCommand::AddDev) => Some("poetry add --group dev <pkg>"),
    (PackageManager::Poetry, ApmwCommand::Install) => Some("poetry install"),
    (PackageManager::Poetry, ApmwCommand::List) => Some("poetry show"),
    (PackageManager::Poetry, ApmwCommand::Outdated) => Some("poetry show --outdated"),
    (PackageManager::Poetry, ApmwCommand::Remove) => Some("poetry remove <pkg>"),
    (PackageManager::Poetry, ApmwCommand::UpdateAll) => Some("poetry update"),
    (PackageManager::Poetry, ApmwCommand::Update) => Some("poetry update <pkg>"),
    (PackageManager::Poetry, ApmwCommand::Test) => Some("poetry run pytest"),
    (PackageManager::Poetry, ApmwCommand::Build) => Some("poetry build"),
    (PackageManager::Poetry, ApmwCommand::Init) => Some("poetry new <proj-name>"),
    (PackageManager::Pipenv, ApmwCommand::Add) => Some("pipenv install <pkg>"),
    (PackageManager::Pipenv, ApmwCommand::AddDev) => Some("pipenv install --dev <pkg>"),
    (PackageManager::Pipenv, ApmwCommand::Install) => Some("pipenv install"),
    (PackageManager::Pipenv, ApmwCommand::List) => Some("pipenv graph"),
    (PackageManager::Pipenv, ApmwCommand::Outdated) => Some("pipenv update --outdated"),
    (PackageManager::Pipenv, ApmwCommand::Remove) => Some("pipenv uninstall <pkg>"),
    (PackageManager::Pipenv, ApmwCommand::UpdateAll) => Some("pipenv update"),
    (PackageManager::Pipenv, ApmwCommand::Update) => Some("pipenv update <pkg>"),
    (PackageManager::Pipenv, ApmwCommand::Test) => Some("pipenv run pytest"),
    (PackageManager::Pipenv, ApmwCommand::Build) => {
      Some("pipenv run python setup.py sdist bdist_wheel")
    }
    (PackageManager::Pipenv, ApmwCommand::Init) => Some("pipenv install"),
    (PackageManager::Pdm, ApmwCommand::Add) => Some("pdm add <pkg>"),
    (PackageManager::Pdm, ApmwCommand::AddDev) => Some("pdm add -d <pkg>"),
    (PackageManager::Pdm, ApmwCommand::Install) => Some("pdm install"),
    (PackageManager::Pdm, ApmwCommand::List) => Some("pdm list"),
    (PackageManager::Pdm, ApmwCommand::Outdated) => Some("pdm update --dry-run"),
    (PackageManager::Pdm, ApmwCommand::Remove) => Some("pdm remove <pkg>"),
    (PackageManager::Pdm, ApmwCommand::UpdateAll) => Some("pdm update"),
    (PackageManager::Pdm, ApmwCommand::Update) => Some("pdm update <pkg>"),
    (PackageManager::Pdm, ApmwCommand::Test) => Some("pdm run pytest"),
    (PackageManager::Pdm, ApmwCommand::Build) => Some("pdm build"),
    (PackageManager::Pdm, ApmwCommand::Init) => Some("pdm init"),
    (PackageManager::Conda, ApmwCommand::Add) => Some("conda install <pkg>"),
    (PackageManager::Conda, ApmwCommand::AddDev) => Some("conda install -c conda-forge <pkg>"),
    (PackageManager::Conda, ApmwCommand::Install) => Some("conda env create -f environment.yml"),
    (PackageManager::Conda, ApmwCommand::List) => Some("conda list"),
    (PackageManager::Conda, ApmwCommand::Outdated) => Some("conda update --all --dry-run"),
    (PackageManager::Conda, ApmwCommand::Remove) => Some("conda remove <pkg>"),
    (PackageManager::Conda, ApmwCommand::UpdateAll) => Some("conda update --all"),
    (PackageManager::Conda, ApmwCommand::Update) => Some("conda update <pkg>"),
    (PackageManager::Conda, ApmwCommand::Test) => Some("conda run pytest"),
    (PackageManager::Conda, ApmwCommand::Build) => None,
    (PackageManager::Conda, ApmwCommand::Init) => Some("conda create -n <env_name>"),
    (PackageManager::Uv, ApmwCommand::Add) => Some("uv pip install <pkg>"),
    (PackageManager::Uv, ApmwCommand::AddDev) => Some("uv pip install --dev <pkg>"),
    (PackageManager::Uv, ApmwCommand::Install) => Some("uv pip install -r requirements.txt"),
    (PackageManager::Uv, ApmwCommand::List) => Some("uv pip list"),
    (PackageManager::Uv, ApmwCommand::Outdated) => Some("uv pip list --outdated"),
    (PackageManager::Uv, ApmwCommand::Remove) => Some("uv pip uninstall <pkg>"),
    (PackageManager::Uv, ApmwCommand::UpdateAll) => Some("uv update"),
    (PackageManager::Uv, ApmwCommand::Update) => Some("uv update <pkg>"),
    (PackageManager::Uv, ApmwCommand::Test) => Some("uv run pytest"),
    (PackageManager::Uv, ApmwCommand::Build) => None,
    (PackageManager::Uv, ApmwCommand::Init) => Some("uv init"),
    (PackageManager::Cargo, ApmwCommand::Add) => Some("cargo add <crate>"),
    (PackageManager::Cargo, ApmwCommand::AddDev) => Some("cargo add --dev <crate>"),
    (PackageManager::Cargo, ApmwCommand::Install) => Some("cargo build"),
    (PackageManager::Cargo, ApmwCommand::List) => Some("cargo tree"),
    (PackageManager::Cargo, ApmwCommand::Outdated) => Some("cargo outdated"),
    (PackageManager::Cargo, ApmwCommand::Remove) => Some("cargo remove <crate>"),
    (PackageManager::Cargo, ApmwCommand::UpdateAll) => Some("cargo update"),
    (PackageManager::Cargo, ApmwCommand::Update) => Some("cargo update -p <crate>"),
    (PackageManager::Cargo, ApmwCommand::Test) => Some("cargo test"),
    (PackageManager::Cargo, ApmwCommand::Build) => Some("cargo build"),
    (PackageManager::Cargo, ApmwCommand::Init) => Some("cargo new <proj-name>"),
    (PackageManager::Go, ApmwCommand::Add) => Some("go get <pkg>"),
    (PackageManager::Go, ApmwCommand::AddDev) => Some("go get -d <pkg>"),
    (PackageManager::Go, ApmwCommand::Install) => Some("go mod tidy"),
    (PackageManager::Go, ApmwCommand::List) => Some("go list -m all"),
    (PackageManager::Go, ApmwCommand::Outdated) => Some("go list -m -u all"),
    (PackageManager::Go, ApmwCommand::Remove) => Some("go mod tidy"),
    (PackageManager::Go, ApmwCommand::UpdateAll) => Some("go get -u ./..."),
    (PackageManager::Go, ApmwCommand::Update) => Some("go get -u <pkg>"),
    (PackageManager::Go, ApmwCommand::Test) => Some("go test ./..."),
    (PackageManager::Go, ApmwCommand::Build) => Some("go build ./..."),
    (PackageManager::Go, ApmwCommand::Init) => Some("go mod init <module-path>"),
    (PackageManager::Maven, ApmwCommand::Add) => None,
    (PackageManager::Maven, ApmwCommand::AddDev) => None,
    (PackageManager::Maven, ApmwCommand::Install) => Some("mvn install"),
    (PackageManager::Maven, ApmwCommand::List) => Some("mvn dependency:list"),
    (PackageManager::Maven, ApmwCommand::Outdated) => {
      Some("mvn versions:display-dependency-updates")
    }
    (PackageManager::Maven, ApmwCommand::Remove) => None,
    (PackageManager::Maven, ApmwCommand::UpdateAll) => Some("mvn versions:update-properties"),
    (PackageManager::Maven, ApmwCommand::Update) => None,
    (PackageManager::Maven, ApmwCommand::Test) => Some("mvn test"),
    (PackageManager::Maven, ApmwCommand::Build) => Some("mvn package"),
    (PackageManager::Maven, ApmwCommand::Init) => Some("mvn archetype:generate"),
    (PackageManager::Gradle, ApmwCommand::Add) => None,
    (PackageManager::Gradle, ApmwCommand::AddDev) => None,
    (PackageManager::Gradle, ApmwCommand::Install) => Some("gradle build"),
    (PackageManager::Gradle, ApmwCommand::List) => Some("gradle dependencies"),
    (PackageManager::Gradle, ApmwCommand::Outdated) => Some("gradle dependencyUpdates"),
    (PackageManager::Gradle, ApmwCommand::Remove) => None,
    (PackageManager::Gradle, ApmwCommand::UpdateAll) => Some("gradle dependencyUpdates"),
    (PackageManager::Gradle, ApmwCommand::Update) => None,
    (PackageManager::Gradle, ApmwCommand::Test) => Some("gradle test"),
    (PackageManager::Gradle, ApmwCommand::Build) => Some("gradle build"),
    (PackageManager::Gradle, ApmwCommand::Init) => Some("gradle init"),
    (PackageManager::Sbt, ApmwCommand::Add) => None,
    (PackageManager::Sbt, ApmwCommand::AddDev) => None,
    (PackageManager::Sbt, ApmwCommand::Install) => Some("sbt update"),
    (PackageManager::Sbt, ApmwCommand::List) => Some("sbt libraryDependencies"),
    (PackageManager::Sbt, ApmwCommand::Outdated) => Some("sbt dependencyUpdates"),
    (PackageManager::Sbt, ApmwCommand::Remove) => None,
    (PackageManager::Sbt, ApmwCommand::UpdateAll) => Some("sbt dependencyUpdates"),
    (PackageManager::Sbt, ApmwCommand::Update) => None,
    (PackageManager::Sbt, ApmwCommand::Test) => Some("sbt test"),
    (PackageManager::Sbt, ApmwCommand::Build) => Some("sbt package"),
    (PackageManager::Sbt, ApmwCommand::Init) => Some("sbt new"),
    (PackageManager::SwiftPm, ApmwCommand::Add) => None,
    (PackageManager::SwiftPm, ApmwCommand::AddDev) => None,
    (PackageManager::SwiftPm, ApmwCommand::Install) => Some("swift package resolve"),
    (PackageManager::SwiftPm, ApmwCommand::List) => Some("swift package show-dependencies"),
    (PackageManager::SwiftPm, ApmwCommand::Outdated) => None,
    (PackageManager::SwiftPm, ApmwCommand::Remove) => None,
    (PackageManager::SwiftPm, ApmwCommand::UpdateAll) => Some("swift package update"),
    (PackageManager::SwiftPm, ApmwCommand::Update) => None,
    (PackageManager::SwiftPm, ApmwCommand::Test) => Some("swift test"),
    (PackageManager::SwiftPm, ApmwCommand::Build) => Some("swift build"),
    (PackageManager::SwiftPm, ApmwCommand::Init) => Some("swift package init"),
    (PackageManager::CocoaPods, ApmwCommand::Add) => Some("pod install"),
    (PackageManager::CocoaPods, ApmwCommand::AddDev) => None,
    (PackageManager::CocoaPods, ApmwCommand::Install) => Some("pod install"),
    (PackageManager::CocoaPods, ApmwCommand::List) => Some("pod list"),
    (PackageManager::CocoaPods, ApmwCommand::Outdated) => Some("pod outdated"),
    (PackageManager::CocoaPods, ApmwCommand::Remove) => Some("pod install"),
    (PackageManager::CocoaPods, ApmwCommand::UpdateAll) => Some("pod update"),
    (PackageManager::CocoaPods, ApmwCommand::Update) => None,
    (PackageManager::CocoaPods, ApmwCommand::Test) => Some("pod spec lint"),
    (PackageManager::CocoaPods, ApmwCommand::Build) => Some("pod lib lint"),
    (PackageManager::CocoaPods, ApmwCommand::Init) => Some("pod lib create <project_name>"),
    (PackageManager::Carthage, ApmwCommand::Add) => None,
    (PackageManager::Carthage, ApmwCommand::AddDev) => None,
    (PackageManager::Carthage, ApmwCommand::Install) => Some("carthage bootstrap"),
    (PackageManager::Carthage, ApmwCommand::List) => Some("carthage version"),
    (PackageManager::Carthage, ApmwCommand::Outdated) => None,
    (PackageManager::Carthage, ApmwCommand::Remove) => None,
    (PackageManager::Carthage, ApmwCommand::UpdateAll) => Some("carthage update"),
    (PackageManager::Carthage, ApmwCommand::Update) => None,
    (PackageManager::Carthage, ApmwCommand::Test) => Some("carthage build"),
    (PackageManager::Carthage, ApmwCommand::Build) => Some("carthage build"),
    (PackageManager::Carthage, ApmwCommand::Init) => None,
    (PackageManager::Dotnet, ApmwCommand::Add) => Some("dotnet add package <package>"),
    (PackageManager::Dotnet, ApmwCommand::AddDev) => Some("dotnet add package <package>"),
    (PackageManager::Dotnet, ApmwCommand::Install) => Some("dotnet restore"),
    (PackageManager::Dotnet, ApmwCommand::List) => Some("dotnet list package"),
    (PackageManager::Dotnet, ApmwCommand::Outdated) => Some("dotnet list package --outdated"),
    (PackageManager::Dotnet, ApmwCommand::Remove) => Some("dotnet remove package <package>"),
    (PackageManager::Dotnet, ApmwCommand::UpdateAll) => Some("dotnet nuget update"),
    (PackageManager::Dotnet, ApmwCommand::Update) => Some("dotnet nuget update <package>"),
    (PackageManager::Dotnet, ApmwCommand::Test) => Some("dotnet test"),
    (PackageManager::Dotnet, ApmwCommand::Build) => Some("dotnet build"),
    (PackageManager::Dotnet, ApmwCommand::Init) => Some("dotnet new <template>"),
    (PackageManager::Flutter, ApmwCommand::Add) => Some("flutter pub add <package>"),
    (PackageManager::Flutter, ApmwCommand::AddDev) => Some("flutter pub add --dev <package>"),
    (PackageManager::Flutter, ApmwCommand::Install) => Some("flutter pub get"),
    (PackageManager::Flutter, ApmwCommand::List) => Some("flutter pub deps"),
    (PackageManager::Flutter, ApmwCommand::Outdated) => Some("flutter pub outdated"),
    (PackageManager::Flutter, ApmwCommand::Remove) => Some("flutter pub remove <package>"),
    (PackageManager::Flutter, ApmwCommand::UpdateAll) => Some("flutter pub upgrade"),
    (PackageManager::Flutter, ApmwCommand::Update) => Some("flutter pub upgrade <package>"),
    (PackageManager::Flutter, ApmwCommand::Test) => Some("flutter test"),
    (PackageManager::Flutter, ApmwCommand::Build) => Some("flutter build <target>"),
    (PackageManager::Flutter, ApmwCommand::Init) => Some("flutter create <project_name>"),
    (PackageManager::Dart, ApmwCommand::Add) => Some("dart pub add <package>"),
    (PackageManager::Dart, ApmwCommand::AddDev) => Some("dart pub add --dev <package>"),
    (PackageManager::Dart, ApmwCommand::Install) => Some("dart pub get"),
    (PackageManager::Dart, ApmwCommand::List) => Some("dart pub deps"),
    (PackageManager::Dart, ApmwCommand::Outdated) => Some("dart pub outdated"),
    (PackageManager::Dart, ApmwCommand::Remove) => Some("dart pub remove <package>"),
    (PackageManager::Dart, ApmwCommand::UpdateAll) => Some("dart pub upgrade"),
    (PackageManager::Dart, ApmwCommand::Update) => Some("dart pub upgrade <package>"),
    (PackageManager::Dart, ApmwCommand::Test) => Some("dart test"),
    (PackageManager::Dart, ApmwCommand::Build) => Some("dart compile exe <file.dart>"),
    (PackageManager::Dart, ApmwCommand::Init) => Some("dart create <project_name>"),
    (PackageManager::Bazel, _) => None,
    (PackageManager::Ant, ApmwCommand::Install) => Some("ant"),
    (PackageManager::Ant, _) => None,
  }
}

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

pub fn all_ecosystems() -> Vec<Ecosystem> {
  vec![
    Ecosystem::Python,
    Ecosystem::Node,
    Ecosystem::Rust,
    Ecosystem::Go,
    Ecosystem::Jvm,
    Ecosystem::Swift,
    Ecosystem::Dotnet,
    Ecosystem::Flutter,
    Ecosystem::Polyglot,
  ]
}

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
  fn test_manager_command_add() {
    assert_eq!(
      manager_command(PackageManager::Npm, ApmwCommand::Add),
      Some("npm install <pkg>")
    );
    assert_eq!(
      manager_command(PackageManager::Pip, ApmwCommand::Add),
      Some("pip install <pkg>")
    );
    assert_eq!(
      manager_command(PackageManager::Uv, ApmwCommand::Add),
      Some("uv pip install <pkg>")
    );
    assert_eq!(
      manager_command(PackageManager::Pnpm, ApmwCommand::Add),
      Some("pnpm add <pkg>")
    );
    assert_eq!(
      manager_command(PackageManager::Maven, ApmwCommand::Add),
      None
    );
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
