//! Ad-hoc runner resolution per ecosystem.
//!
//! Resolves the canonical ad-hoc runner command for a given ecosystem and
//! package manager. The runner is the command used to actually install a
//! package (e.g. `pnpm add`, `cargo add`, `uv pip install`).
//!
//! This module integrates with the ecosystem mapper (story 02-003) and the
//! `cli-tool-discovery --runner` concept: the runner is the canonical manager's
//! add command within the same ecosystem as the detected/source manager.
//!
//! # Trait-based approach
//!
//! The [`RunnerResolver`] trait allows tests to mock runner resolution. The
//! default implementation [`DefaultRunnerResolver`] uses the ecosystem mapper
//! to produce the canonical add command.

use crate::ecosystem::{ApmwCommand, EcosystemMapper, PackageManager};
use crate::error::{ApmwError, Result};

/// A resolved runner — the command used to install a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
  /// The canonical manager name (e.g. `pnpm`, `cargo`, `uv`).
  pub manager: String,
  /// The full add command with `<pkg>` placeholder (e.g. `pnpm add <pkg>`).
  pub add_command: String,
  /// The full dev-dep add command with `<pkg>` placeholder (e.g. `pnpm add -D <pkg>`).
  pub dev_command: Option<String>,
}

impl Runner {
  /// Build the final add command with the package name substituted.
  pub fn add(&self, package: &str) -> String {
    self.add_command.replace("<pkg>", package)
  }

  /// Build the final dev-dep add command with the package name substituted.
  ///
  /// Returns `None` if the manager does not support dev deps.
  pub fn add_dev(&self, package: &str) -> Option<String> {
    self
      .dev_command
      .as_ref()
      .map(|cmd| cmd.replace("<pkg>", package))
  }
}

/// Trait for resolving the ad-hoc runner for a package manager.
///
/// Implementations are expected to be cheap and deterministic. The trait
/// enables test mocking — real code uses [`DefaultRunnerResolver`].
pub trait RunnerResolver: Send + Sync {
  /// Resolve the runner for the given manager name.
  fn resolve(&self, manager: &str) -> Result<Runner>;
}

/// The default runner resolver — uses the ecosystem mapper to produce the
/// canonical add command within the source manager's ecosystem.
#[derive(Debug, Clone)]
pub struct DefaultRunnerResolver {
  mapper: EcosystemMapper,
}

impl DefaultRunnerResolver {
  /// Create a new default runner resolver.
  pub fn new() -> Self {
    DefaultRunnerResolver {
      mapper: EcosystemMapper::new(),
    }
  }
}

impl Default for DefaultRunnerResolver {
  fn default() -> Self {
    Self::new()
  }
}

impl RunnerResolver for DefaultRunnerResolver {
  fn resolve(&self, manager: &str) -> Result<Runner> {
    let pm = PackageManager::parse_manager(manager)
      .ok_or_else(|| ApmwError::EcosystemMapping(format!("unknown package manager: {manager}")))?;

    // Determine whether this manager should be remapped to its ecosystem's
    // canonical manager. Only pip→uv, npm→pnpm, yarn→pnpm, yarn2→pnpm, and
    // bun→pnpm are remapped. All other managers (poetry, pipenv, pdm, conda,
    // cargo, go, etc.) "remain as-is" and use their own commands directly.
    let should_remap = matches!(
      pm,
      PackageManager::Pip
        | PackageManager::Npm
        | PackageManager::Yarn
        | PackageManager::Yarn2
        | PackageManager::Bun
    );

    let (runner_manager, add_command, dev_command) = if should_remap {
      // Remap to canonical (e.g. pip→uv, npm→pnpm).
      let canonical = self.mapper.suggest_canonical(pm);
      let add = self
        .mapper
        .map_command(ApmwCommand::Add, pm)
        .ok_or_else(|| {
          ApmwError::EcosystemMapping(format!("no add command for manager: {manager}"))
        })?;
      let dev = self.mapper.map_command(ApmwCommand::AddDev, pm);
      (canonical, add, dev)
    } else {
      // Use the original manager's commands directly.
      let add = crate::ecosystem::manager_command(pm, ApmwCommand::Add)
        .ok_or_else(|| {
          ApmwError::EcosystemMapping(format!("no add command for manager: {manager}"))
        })?
        .to_string();
      let dev = crate::ecosystem::manager_command(pm, ApmwCommand::AddDev).map(|s| s.to_string());
      (pm, add, dev)
    };

    Ok(Runner {
      manager: runner_manager.to_string(),
      add_command,
      dev_command,
    })
  }
}

/// A mock runner resolver for unit tests.
///
/// Returns pre-configured runners without consulting the ecosystem mapper.
#[derive(Debug, Clone, Default)]
pub struct MockRunnerResolver {
  runners: std::collections::HashMap<String, Runner>,
}

impl MockRunnerResolver {
  /// Create an empty mock resolver.
  pub fn new() -> Self {
    MockRunnerResolver::default()
  }

  /// Register a runner for a manager name.
  pub fn with(mut self, manager: &str, runner: Runner) -> Self {
    self.runners.insert(manager.to_string(), runner);
    self
  }
}

impl RunnerResolver for MockRunnerResolver {
  fn resolve(&self, manager: &str) -> Result<Runner> {
    self
      .runners
      .get(manager)
      .cloned()
      .ok_or_else(|| ApmwError::EcosystemMapping(format!("no mock runner for: {manager}")))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ===========================================================================
  // DefaultRunnerResolver — canonical runner per ecosystem
  // ===========================================================================

  #[test]
  fn test_resolve_pnpm_runner() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("pnpm").unwrap();
    assert_eq!(runner.manager, "pnpm");
    assert_eq!(runner.add_command, "pnpm add <pkg>");
    assert_eq!(runner.add("express"), "pnpm add express");
  }

  #[test]
  fn test_resolve_npm_maps_to_pnpm() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("npm").unwrap();
    assert_eq!(runner.manager, "pnpm");
    assert_eq!(runner.add_command, "pnpm add <pkg>");
  }

  #[test]
  fn test_resolve_pip_maps_to_uv() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("pip").unwrap();
    assert_eq!(runner.manager, "uv");
    assert_eq!(runner.add_command, "uv pip install <pkg>");
  }

  #[test]
  fn test_resolve_cargo_runner() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("cargo").unwrap();
    assert_eq!(runner.manager, "cargo");
    assert_eq!(runner.add_command, "cargo add <pkg>");
  }

  #[test]
  fn test_resolve_go_runner() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("go").unwrap();
    assert_eq!(runner.manager, "go");
    assert_eq!(runner.add_command, "go get <pkg>");
  }

  #[test]
  fn test_resolve_yarn_maps_to_pnpm() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("yarn").unwrap();
    assert_eq!(runner.manager, "pnpm");
    assert_eq!(runner.add_command, "pnpm add <pkg>");
  }

  #[test]
  fn test_resolve_poetry_runner() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("poetry").unwrap();
    assert_eq!(runner.manager, "poetry");
    assert_eq!(runner.add_command, "poetry add <pkg>");
  }

  // ===========================================================================
  // Dev-dep command resolution
  // ===========================================================================

  #[test]
  fn test_resolve_pnpm_dev_command() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("pnpm").unwrap();
    assert_eq!(runner.dev_command.as_deref(), Some("pnpm add -D <pkg>"));
    assert_eq!(runner.add_dev("jest"), Some("pnpm add -D jest".to_string()));
  }

  #[test]
  fn test_resolve_cargo_dev_command() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("cargo").unwrap();
    assert_eq!(runner.dev_command.as_deref(), Some("cargo add --dev <pkg>"));
  }

  #[test]
  fn test_resolve_pip_dev_command() {
    let resolver = DefaultRunnerResolver::new();
    let runner = resolver.resolve("pip").unwrap();
    assert_eq!(
      runner.dev_command.as_deref(),
      Some("uv pip install --group dev <pkg>")
    );
  }

  // ===========================================================================
  // Error cases
  // ===========================================================================

  #[test]
  fn test_resolve_unknown_manager_errors() {
    let resolver = DefaultRunnerResolver::new();
    let result = resolver.resolve("nonexistent");
    assert!(result.is_err());
  }

  // ===========================================================================
  // MockRunnerResolver
  // ===========================================================================

  #[test]
  fn test_mock_resolver_returns_configured_runner() {
    let runner = Runner {
      manager: "pnpm".to_string(),
      add_command: "pnpm add <pkg>".to_string(),
      dev_command: Some("pnpm add -D <pkg>".to_string()),
    };
    let resolver = MockRunnerResolver::new().with("pnpm", runner.clone());
    let result = resolver.resolve("pnpm").unwrap();
    assert_eq!(result, runner);
  }

  #[test]
  fn test_mock_resolver_unknown_errors() {
    let resolver = MockRunnerResolver::new();
    assert!(resolver.resolve("pnpm").is_err());
  }

  // ===========================================================================
  // Runner command substitution
  // ===========================================================================

  #[test]
  fn test_runner_add_substitutes_package() {
    let runner = Runner {
      manager: "cargo".to_string(),
      add_command: "cargo add <pkg>".to_string(),
      dev_command: Some("cargo add --dev <pkg>".to_string()),
    };
    assert_eq!(runner.add("serde"), "cargo add serde");
  }

  #[test]
  fn test_runner_add_dev_substitutes_package() {
    let runner = Runner {
      manager: "cargo".to_string(),
      add_command: "cargo add <pkg>".to_string(),
      dev_command: Some("cargo add --dev <pkg>".to_string()),
    };
    assert_eq!(
      runner.add_dev("criterion"),
      Some("cargo add --dev criterion".to_string())
    );
  }

  #[test]
  fn test_runner_add_dev_none_when_no_dev_command() {
    let runner = Runner {
      manager: "brew".to_string(),
      add_command: "brew install <pkg>".to_string(),
      dev_command: None,
    };
    assert_eq!(runner.add_dev("wget"), None);
  }
}
