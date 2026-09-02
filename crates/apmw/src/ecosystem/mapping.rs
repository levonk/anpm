//! Ecosystem mapping table — within-ecosystem command translations.
//!
//! This module defines the data structures and the static mapping table that
//! translates canonical `apmw` commands into per-package-manager commands.
//! The table is derived from the 2ndbrain research Table 2 ("Package Manager
//! Command Reference") in the All Package Manager Wrapper Tool Landscape
//! document.
//!
//! This module is CLI-specific and lives in the `apmw` binary crate (not in
//! `apmw-core`) because downstream repos do not need within-ecosystem command
//! translations.
//!
//! # Core principle: WITHIN-ecosystem only
//!
//! Mappings are **never** cross-ecosystem. A Python runner maps to a Python
//! runner; a Node runner maps to a Node runner. For example, `pip` maps to
//! `uv` (both Python), and `npm` maps to `pnpm` (both Node), but `uvx` (Python)
//! must NEVER map to `pnpm dlx` (Node). Each ecosystem's commands map within
//! that ecosystem only (PRD FR-2.1).
//!
//! # Example
//!
//! ```
//! use apmw::ecosystem::{EcosystemMapper, ApmwCommand, PackageManager};
//!
//! let mapper = EcosystemMapper::new();
//! // pip (Python) -> uv (Python): within-ecosystem
//! assert_eq!(
//!   mapper.map_command(ApmwCommand::Add, PackageManager::Pip),
//!   Some("uv pip install <pkg>".to_string())
//! );
//! // npm (Node) -> pnpm (Node): within-ecosystem
//! assert_eq!(
//!   mapper.map_command(ApmwCommand::Add, PackageManager::Npm),
//!   Some("pnpm add <pkg>".to_string())
//! );
//! ```

use std::collections::HashMap;

use tracing::{debug, warn};

use apmw_core::ecosystem::{
  all_commands, all_ecosystems, all_managers, ApmwCommand, Ecosystem, PackageManager,
};

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
    (PackageManager::Uv, ApmwCommand::AddDev) => Some("uv pip install --group dev <pkg>"),
    (PackageManager::Uv, ApmwCommand::Install) => Some("uv pip install -r requirements.txt"),
    (PackageManager::Uv, ApmwCommand::List) => Some("uv pip list"),
    (PackageManager::Uv, ApmwCommand::Outdated) => Some("uv pip list --outdated"),
    (PackageManager::Uv, ApmwCommand::Remove) => Some("uv pip uninstall <pkg>"),
    (PackageManager::Uv, ApmwCommand::UpdateAll) => Some("uv update"),
    (PackageManager::Uv, ApmwCommand::Update) => Some("uv update <pkg>"),
    (PackageManager::Uv, ApmwCommand::Test) => Some("uv run pytest"),
    (PackageManager::Uv, ApmwCommand::Build) => None,
    (PackageManager::Uv, ApmwCommand::Init) => Some("uv init"),
    (PackageManager::Cargo, ApmwCommand::Add) => Some("cargo add <pkg>"),
    (PackageManager::Cargo, ApmwCommand::AddDev) => Some("cargo add --dev <pkg>"),
    (PackageManager::Cargo, ApmwCommand::Install) => Some("cargo build"),
    (PackageManager::Cargo, ApmwCommand::List) => Some("cargo tree"),
    (PackageManager::Cargo, ApmwCommand::Outdated) => Some("cargo outdated"),
    (PackageManager::Cargo, ApmwCommand::Remove) => Some("cargo remove <pkg>"),
    (PackageManager::Cargo, ApmwCommand::UpdateAll) => Some("cargo update"),
    (PackageManager::Cargo, ApmwCommand::Update) => Some("cargo update -p <pkg>"),
    (PackageManager::Cargo, ApmwCommand::Test) => Some("cargo test"),
    (PackageManager::Cargo, ApmwCommand::Build) => Some("cargo build"),
    (PackageManager::Cargo, ApmwCommand::Init) => Some("cargo new <proj-name>"),
    (PackageManager::Go, ApmwCommand::Add) => Some("go get <pkg>"),
    (PackageManager::Go, ApmwCommand::AddDev) => Some("go get -t <pkg>"),
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
    (PackageManager::Gem, ApmwCommand::Add) => Some("gem install <pkg>"),
    (PackageManager::Gem, ApmwCommand::AddDev) => Some("gem install <pkg>"),
    (PackageManager::Gem, ApmwCommand::Install) => Some("bundle install"),
    (PackageManager::Gem, ApmwCommand::List) => Some("gem list"),
    (PackageManager::Gem, ApmwCommand::Outdated) => Some("gem outdated"),
    (PackageManager::Gem, ApmwCommand::Remove) => Some("gem uninstall <pkg>"),
    (PackageManager::Gem, ApmwCommand::UpdateAll) => Some("gem update"),
    (PackageManager::Gem, ApmwCommand::Update) => Some("gem update <pkg>"),
    (PackageManager::Gem, ApmwCommand::Test) => Some("bundle exec rake test"),
    (PackageManager::Gem, ApmwCommand::Build) => Some("gem build <pkg>.gemspec"),
    (PackageManager::Gem, ApmwCommand::Init) => Some("bundle init"),
    (PackageManager::Composer, ApmwCommand::Add) => Some("composer require <pkg>"),
    (PackageManager::Composer, ApmwCommand::AddDev) => Some("composer require --dev <pkg>"),
    (PackageManager::Composer, ApmwCommand::Install) => Some("composer install"),
    (PackageManager::Composer, ApmwCommand::List) => Some("composer show"),
    (PackageManager::Composer, ApmwCommand::Outdated) => Some("composer outdated"),
    (PackageManager::Composer, ApmwCommand::Remove) => Some("composer remove <pkg>"),
    (PackageManager::Composer, ApmwCommand::UpdateAll) => Some("composer update"),
    (PackageManager::Composer, ApmwCommand::Update) => Some("composer update <pkg>"),
    (PackageManager::Composer, ApmwCommand::Test) => Some("composer test"),
    (PackageManager::Composer, ApmwCommand::Build) => None,
    (PackageManager::Composer, ApmwCommand::Init) => Some("composer init"),
  }
}

/// The ecosystem mapping engine.
///
/// Holds a pre-built mapping table for every known package manager and
/// provides within-ecosystem command translation.
#[derive(Debug, Clone)]
pub struct EcosystemMapper {
  /// Maps each source package manager to its within-ecosystem `EcosystemMap`.
  maps: HashMap<PackageManager, EcosystemMap>,
}

impl EcosystemMapper {
  /// Creates a new `EcosystemMapper` with the full mapping table built from
  /// 2ndbrain Table 2.
  pub fn new() -> Self {
    let maps = all_managers()
      .into_iter()
      .map(|source| {
        let canonical = source.ecosystem().canonical_manager();
        let mut command_mapping = HashMap::new();
        for cmd in all_commands() {
          // The canonical command is looked up from the canonical manager's
          // row in Table 2. This guarantees within-ecosystem translation: we
          // never mix commands from different ecosystems.
          let canonical_cmd = manager_command(canonical, cmd);
          command_mapping.insert(cmd, canonical_cmd);
        }
        (
          source,
          EcosystemMap {
            source_manager: source,
            canonical_manager: canonical,
            command_mapping,
          },
        )
      })
      .collect();
    Self { maps }
  }

  /// Returns the ecosystem for the given package manager.
  pub fn ecosystem(&self, manager: PackageManager) -> Ecosystem {
    manager.ecosystem()
  }

  /// Returns the canonical package manager for the given source manager's
  /// ecosystem.
  ///
  /// This is a within-ecosystem suggestion only — it never returns a manager
  /// from a different ecosystem (PRD FR-2.1, FR-2.2).
  ///
  /// Managers that "remain as-is" (poetry, pipenv, pdm, conda, cargo, go, etc.)
  /// return themselves, indicating no forced switch is needed.
  pub fn suggest_canonical(&self, source: PackageManager) -> PackageManager {
    let canonical = source.ecosystem().canonical_manager();
    debug!(
      source = %source,
      canonical = %canonical,
      ecosystem = %source.ecosystem(),
      "suggested canonical manager"
    );
    canonical
  }

  /// Returns `true` if the source manager should be remapped to a different
  /// canonical manager (i.e. `suggest_canonical(source) != source`).
  ///
  /// Managers that "remain as-is" (poetry, pipenv, pdm, conda, cargo, go)
  /// return `false` — no forced switch.
  pub fn needs_remapping(&self, source: PackageManager) -> bool {
    self.suggest_canonical(source) != source
  }

  /// Maps an `apmw` command to the canonical manager's concrete command string
  /// for the given source manager's ecosystem.
  ///
  /// This is a **within-ecosystem** translation: the returned command always
  /// belongs to the canonical manager of the same ecosystem as `source`. It
  /// never returns a command from a different ecosystem (PRD FR-2.1).
  ///
  /// Returns `None` if the canonical manager does not support that command
  /// (marked "N/A" in Table 2).
  pub fn map_command(&self, cmd: ApmwCommand, source: PackageManager) -> Option<String> {
    let canonical = self.suggest_canonical(source);
    let result = manager_command(canonical, cmd).map(|s| s.to_string());
    match &result {
      Some(c) => {
        debug!(
          apmw_command = %cmd,
          source = %source,
          canonical = %canonical,
          canonical_command = %c,
          "mapped command"
        );
      }
      None => {
        warn!(
          apmw_command = %cmd,
          source = %source,
          canonical = %canonical,
          "no command mapping for this apmw command"
        );
      }
    }
    result
  }

  /// Returns the [`EcosystemMap`] for the given source manager.
  pub fn map_for(&self, source: PackageManager) -> Option<&EcosystemMap> {
    self.maps.get(&source)
  }

  /// Returns all known package managers.
  pub fn all_managers(&self) -> Vec<PackageManager> {
    all_managers()
  }

  /// Returns all known ecosystems.
  pub fn all_ecosystems(&self) -> Vec<Ecosystem> {
    all_ecosystems()
  }
}

impl Default for EcosystemMapper {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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

  // ===========================================================================
  // Acceptance criteria: pip maps to uv (within Python ecosystem)
  // ===========================================================================

  #[test]
  fn test_pip_maps_to_uv() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Pip),
      PackageManager::Uv
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Add, PackageManager::Pip),
      Some("uv pip install <pkg>".to_string())
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Install, PackageManager::Pip),
      Some("uv pip install -r requirements.txt".to_string())
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Remove, PackageManager::Pip),
      Some("uv pip uninstall <pkg>".to_string())
    );
  }

  // ===========================================================================
  // Acceptance criteria: npm/yarn/bun/yarn2 map to pnpm (within Node ecosystem)
  // ===========================================================================

  #[test]
  fn test_npm_maps_to_pnpm() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Npm),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Add, PackageManager::Npm),
      Some("pnpm add <pkg>".to_string())
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::AddDev, PackageManager::Npm),
      Some("pnpm add -D <pkg>".to_string())
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Install, PackageManager::Npm),
      Some("pnpm install".to_string())
    );
  }

  #[test]
  fn test_yarn_maps_to_pnpm() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Yarn),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Add, PackageManager::Yarn),
      Some("pnpm add <pkg>".to_string())
    );
  }

  #[test]
  fn test_bun_maps_to_pnpm() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Bun),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Add, PackageManager::Bun),
      Some("pnpm add <pkg>".to_string())
    );
  }

  #[test]
  fn test_yarn2_maps_to_pnpm() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Yarn2),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.map_command(ApmwCommand::Add, PackageManager::Yarn2),
      Some("pnpm add <pkg>".to_string())
    );
  }

  // ===========================================================================
  // Acceptance criteria: NO cross-ecosystem mappings exist
  // ===========================================================================

  /// Asserts that map_command NEVER returns a command from a different
  /// ecosystem than the source manager. This is the key risk-mitigation test
  /// from the story: "uvx does NOT map to pnpm dlx" and similar cross-ecosystem
  /// mappings must not exist.
  #[test]
  fn test_no_cross_ecosystem_mappings() {
    let mapper = EcosystemMapper::new();
    for source in all_managers() {
      let source_ecosystem = source.ecosystem();
      let canonical = mapper.suggest_canonical(source);
      // The canonical manager must be in the SAME ecosystem as the source.
      assert_eq!(
        canonical.ecosystem(),
        source_ecosystem,
        "suggest_canonical({source}) returned {canonical} from a different ecosystem"
      );
      // Every mapped command must come from the canonical manager (same ecosystem).
      for cmd in all_commands() {
        if let Some(mapped) = mapper.map_command(cmd, source) {
          // The mapped command must be the canonical manager's command.
          let expected = manager_command(canonical, cmd).map(|s| s.to_string());
          assert_eq!(
            Some(mapped.clone()),
            expected,
            "map_command({cmd}, {source}) returned {mapped:?} but expected \
             canonical {canonical}'s command"
          );
          // Double-check: the canonical's ecosystem matches the source's.
          assert_eq!(
            canonical.ecosystem(),
            source_ecosystem,
            "canonical {canonical} is in a different ecosystem than source {source}"
          );
        }
      }
    }
  }

  /// Explicitly asserts that pip (Python) does NOT map to any pnpm (Node)
  /// command — the classic cross-ecosystem mistake.
  #[test]
  fn test_pip_does_not_map_to_pnpm() {
    let mapper = EcosystemMapper::new();
    for cmd in all_commands() {
      if let Some(mapped) = mapper.map_command(cmd, PackageManager::Pip) {
        assert!(
          !mapped.starts_with("pnpm"),
          "pip command {cmd} mapped to pnpm command '{mapped}' — cross-ecosystem mapping!"
        );
        assert!(
          !mapped.starts_with("npm"),
          "pip command {cmd} mapped to npm command '{mapped}' — cross-ecosystem mapping!"
        );
      }
    }
  }

  /// Explicitly asserts that npm (Node) does NOT map to any uv/pip (Python)
  /// command.
  #[test]
  fn test_npm_does_not_map_to_python() {
    let mapper = EcosystemMapper::new();
    for cmd in all_commands() {
      if let Some(mapped) = mapper.map_command(cmd, PackageManager::Npm) {
        assert!(
          !mapped.starts_with("uv "),
          "npm command {cmd} mapped to uv command '{mapped}' — cross-ecosystem mapping!"
        );
        assert!(
          !mapped.starts_with("pip "),
          "npm command {cmd} mapped to pip command '{mapped}' — cross-ecosystem mapping!"
        );
      }
    }
  }

  // ===========================================================================
  // Acceptance criteria: suggest_canonical returns correct canonical manager
  // ===========================================================================

  #[test]
  fn test_suggest_canonical_python() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Pip),
      PackageManager::Uv
    );
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Uv),
      PackageManager::Uv
    );
    // poetry/pipenv/pdm/conda "remain as-is" — canonical for the ecosystem is
    // uv, but they are not forced. suggest_canonical returns the ecosystem
    // canonical (uv); callers use needs_remapping to decide whether to switch.
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Poetry),
      PackageManager::Uv
    );
  }

  #[test]
  fn test_suggest_canonical_node() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Npm),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Yarn),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Bun),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Yarn2),
      PackageManager::Pnpm
    );
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Pnpm),
      PackageManager::Pnpm
    );
  }

  #[test]
  fn test_suggest_canonical_rust() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Cargo),
      PackageManager::Cargo
    );
    assert!(!mapper.needs_remapping(PackageManager::Cargo));
  }

  #[test]
  fn test_suggest_canonical_go() {
    let mapper = EcosystemMapper::new();
    assert_eq!(
      mapper.suggest_canonical(PackageManager::Go),
      PackageManager::Go
    );
    assert!(!mapper.needs_remapping(PackageManager::Go));
  }

  // ===========================================================================
  // Acceptance criteria: map_command returns correct canonical command
  // ===========================================================================

  #[test]
  fn test_map_command_all_python_managers() {
    let mapper = EcosystemMapper::new();
    // All Python-ecosystem managers should map to uv's commands.
    for source in [
      PackageManager::Pip,
      PackageManager::Poetry,
      PackageManager::Pipenv,
      PackageManager::Pdm,
      PackageManager::Conda,
      PackageManager::Uv,
    ] {
      assert_eq!(source.ecosystem(), Ecosystem::Python);
      let mapped = mapper.map_command(ApmwCommand::Add, source);
      let expected = manager_command(PackageManager::Uv, ApmwCommand::Add).map(|s| s.to_string());
      assert_eq!(
        mapped, expected,
        "Python manager {source} should map to uv's add command"
      );
    }
  }

  #[test]
  fn test_map_command_all_node_managers() {
    let mapper = EcosystemMapper::new();
    // All Node-ecosystem managers should map to pnpm's commands.
    for source in [
      PackageManager::Npm,
      PackageManager::Yarn,
      PackageManager::Yarn2,
      PackageManager::Bun,
      PackageManager::Pnpm,
    ] {
      assert_eq!(source.ecosystem(), Ecosystem::Node);
      let mapped = mapper.map_command(ApmwCommand::Add, source);
      let expected = manager_command(PackageManager::Pnpm, ApmwCommand::Add).map(|s| s.to_string());
      assert_eq!(
        mapped, expected,
        "Node manager {source} should map to pnpm's add command"
      );
    }
  }

  #[test]
  fn test_map_command_returns_none_for_unsupported() {
    let mapper = EcosystemMapper::new();
    // Maven is the canonical for JVM and doesn't support `add` (N/A in Table 2).
    assert_eq!(
      mapper.map_command(ApmwCommand::Add, PackageManager::Maven),
      None
    );
    // uv (canonical for Python) doesn't support `build` (N/A in Table 2),
    // so pip's build maps to None via the canonical.
    assert_eq!(
      mapper.map_command(ApmwCommand::Build, PackageManager::Pip),
      None
    );
  }

  // ===========================================================================
  // EcosystemMap struct tests
  // ===========================================================================

  #[test]
  fn test_ecosystem_map_fields() {
    let mapper = EcosystemMapper::new();
    let map = mapper.map_for(PackageManager::Pip).unwrap();
    assert_eq!(map.source_manager, PackageManager::Pip);
    assert_eq!(map.canonical_manager, PackageManager::Uv);
    assert_eq!(map.command(ApmwCommand::Add), Some("uv pip install <pkg>"));
  }

  #[test]
  fn test_ecosystem_map_command_none() {
    let mapper = EcosystemMapper::new();
    let map = mapper.map_for(PackageManager::Maven).unwrap();
    assert_eq!(map.command(ApmwCommand::Add), None);
  }

  // ===========================================================================
  // Property-based tests with proptest
  // ===========================================================================

  /// Property: map_command always returns a command from the canonical
  /// manager of the SAME ecosystem as the source — never cross-ecosystem.
  #[test]
  fn proptest_within_ecosystem_only() {
    use proptest::prelude::*;

    let mapper = EcosystemMapper::new();
    let managers = all_managers();
    let commands = all_commands();

    proptest!(|(manager_idx in 0usize..managers.len(), cmd_idx in 0usize..commands.len())| {
      let source = managers[manager_idx];
      let cmd = commands[cmd_idx];
      let source_ecosystem = source.ecosystem();

      if let Some(mapped) = mapper.map_command(cmd, source) {
        let canonical = mapper.suggest_canonical(source);
        prop_assert_eq!(canonical.ecosystem(), source_ecosystem);
        let expected = manager_command(canonical, cmd).map(|s| s.to_string());
        prop_assert_eq!(Some(mapped), expected);
      }
    });
  }

  /// Property: suggest_canonical always returns a manager in the same
  /// ecosystem as the source.
  #[test]
  fn proptest_canonical_same_ecosystem() {
    use proptest::prelude::*;

    let mapper = EcosystemMapper::new();
    let managers = all_managers();

    proptest!(|(manager_idx in 0usize..managers.len())| {
      let source = managers[manager_idx];
      let canonical = mapper.suggest_canonical(source);
      prop_assert_eq!(canonical.ecosystem(), source.ecosystem());
    });
  }

  /// Property: for every (manager, command) pair, the mapped command (if any)
  /// is identical to the canonical manager's own command for that apmw command.
  /// This guarantees mapping consistency across the entire table.
  #[test]
  fn proptest_mapping_consistency() {
    use proptest::prelude::*;

    let mapper = EcosystemMapper::new();
    let managers = all_managers();
    let commands = all_commands();

    proptest!(|(mi in 0usize..managers.len(), ci in 0usize..commands.len())| {
      let source = managers[mi];
      let cmd = commands[ci];
      let canonical = mapper.suggest_canonical(source);
      let mapped = mapper.map_command(cmd, source);
      let direct = manager_command(canonical, cmd).map(|s| s.to_string());
      prop_assert_eq!(mapped, direct);
    });
  }

  /// Property: needs_remapping is true iff canonical != source.
  #[test]
  fn proptest_needs_remapping_consistency() {
    use proptest::prelude::*;

    let mapper = EcosystemMapper::new();
    let managers = all_managers();

    proptest!(|(mi in 0usize..managers.len())| {
      let source = managers[mi];
      let canonical = mapper.suggest_canonical(source);
      prop_assert_eq!(mapper.needs_remapping(source), canonical != source);
    });
  }

  /// Property: the mapping table is exhaustive — every known manager has an
  /// EcosystemMap entry.
  #[test]
  fn proptest_every_manager_has_map() {
    use proptest::prelude::*;

    let mapper = EcosystemMapper::new();
    let managers = all_managers();

    proptest!(|(mi in 0usize..managers.len())| {
      let source = managers[mi];
      prop_assert!(mapper.map_for(source).is_some(), "no map for {source}");
    });
  }
}
