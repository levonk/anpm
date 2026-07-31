//! `--dev` flag mapping to per-manager dev-dep mechanisms.
//!
//! Maps the `apmw add --dev` flag to each package manager's development/build-time
//! dependency mechanism. The mapping is within-ecosystem: each manager gets its
//! own dev-dep flag(s) appended to the canonical add command.
//!
//! # Mapping table (PRD FR-1.2)
//!
//! | Manager | Dev-dep command                          |
//! |---------|------------------------------------------|
//! | pnpm    | `pnpm add -D <pkg>`                      |
//! | npm     | `npm install --save-dev <pkg>`           |
//! | yarn    | `yarn add --dev <pkg>`                   |
//! | cargo   | `cargo add --dev <pkg>`                  |
//! | uv      | `uv pip install --group dev <pkg>`       |
//! | pip     | `pip install --group dev <pkg>`          |
//! | poetry  | `poetry add --group dev <pkg>`           |
//! | go      | `go get -t <pkg>`                        |

use crate::ecosystem::PackageManager;

/// The extra arguments to insert into a manager's add command when `--dev` is set.
///
/// Returns the flag tokens that transform a runtime add into a dev-dep add. For
/// example, pnpm gets `["-D"]`, npm gets `["--save-dev"]`, cargo gets `["--dev"]`.
/// Managers without a distinct dev-dep mechanism return an empty slice.
pub fn dev_flags(manager: &str) -> Vec<&'static str> {
  match manager {
    "pnpm" => vec!["-D"],
    "npm" => vec!["--save-dev"],
    "yarn" | "yarn2" => vec!["--dev"],
    "bun" => vec!["--dev"],
    "cargo" => vec!["--dev"],
    "uv" => vec!["--group", "dev"],
    "pip" => vec!["--group", "dev"],
    "poetry" => vec!["--group", "dev"],
    "pipenv" => vec!["--dev"],
    "pdm" => vec!["--dev"],
    "conda" => vec!["--dev"],
    "go" => vec!["-t"],
    // Managers without a dev-dep concept: brew, nix, devbox, apt, gem, etc.
    _ => Vec::new(),
  }
}

/// Build the full dev-dep add command for a manager and package.
///
/// Combines the canonical add command (from the ecosystem mapper) with the
/// dev-dep flags. Returns the complete command string with `<pkg>` replaced
/// by the actual package name.
pub fn dev_add_command(manager: &str, package: &str) -> Option<String> {
  let pm = PackageManager::parse_manager(manager)?;
  let mapper = crate::ecosystem::EcosystemMapper::new();
  // Only pip, npm, yarn, yarn2, bun are remapped to their canonical.
  // All other managers (poetry, pipenv, pdm, conda, cargo, go, etc.) use
  // their own commands directly.
  let should_remap = matches!(
    pm,
    PackageManager::Pip
      | PackageManager::Npm
      | PackageManager::Yarn
      | PackageManager::Yarn2
      | PackageManager::Bun
  );
  let base = if should_remap {
    mapper.map_command(crate::ecosystem::ApmwCommand::AddDev, pm)?
  } else {
    crate::ecosystem::manager_command(pm, crate::ecosystem::ApmwCommand::AddDev)?.to_string()
  };
  let result = base.replace("<pkg>", package);
  Some(result)
}

/// Returns `true` if the manager supports a distinct dev-dep mechanism.
pub fn supports_dev(manager: &str) -> bool {
  !dev_flags(manager).is_empty()
}

#[cfg(test)]
mod tests {
  use super::*;

  // ===========================================================================
  // Acceptance criteria: --dev maps to the correct dev-dep mechanism per manager
  // ===========================================================================

  #[test]
  fn test_dev_flags_pnpm() {
    assert_eq!(dev_flags("pnpm"), vec!["-D"]);
  }

  #[test]
  fn test_dev_flags_npm() {
    assert_eq!(dev_flags("npm"), vec!["--save-dev"]);
  }

  #[test]
  fn test_dev_flags_yarn() {
    assert_eq!(dev_flags("yarn"), vec!["--dev"]);
  }

  #[test]
  fn test_dev_flags_cargo() {
    assert_eq!(dev_flags("cargo"), vec!["--dev"]);
  }

  #[test]
  fn test_dev_flags_uv() {
    assert_eq!(dev_flags("uv"), vec!["--group", "dev"]);
  }

  #[test]
  fn test_dev_flags_pip() {
    assert_eq!(dev_flags("pip"), vec!["--group", "dev"]);
  }

  #[test]
  fn test_dev_flags_poetry() {
    assert_eq!(dev_flags("poetry"), vec!["--group", "dev"]);
  }

  #[test]
  fn test_dev_flags_go() {
    assert_eq!(dev_flags("go"), vec!["-t"]);
  }

  #[test]
  fn test_dev_flags_unknown_manager_empty() {
    assert!(dev_flags("brew").is_empty());
    assert!(dev_flags("apt").is_empty());
    assert!(dev_flags("nonexistent").is_empty());
  }

  // ===========================================================================
  // Full dev-dep command construction
  // ===========================================================================

  #[test]
  fn test_dev_add_command_pnpm() {
    let cmd = dev_add_command("pnpm", "jest").unwrap();
    assert_eq!(cmd, "pnpm add -D jest");
  }

  #[test]
  fn test_dev_add_command_npm() {
    // npm maps to pnpm within the Node ecosystem.
    let cmd = dev_add_command("npm", "jest").unwrap();
    assert_eq!(cmd, "pnpm add -D jest");
  }

  #[test]
  fn test_dev_add_command_cargo() {
    let cmd = dev_add_command("cargo", "criterion").unwrap();
    assert_eq!(cmd, "cargo add --dev criterion");
  }

  #[test]
  fn test_dev_add_command_uv() {
    // pip maps to uv within the Python ecosystem.
    let cmd = dev_add_command("pip", "pytest").unwrap();
    assert_eq!(cmd, "uv pip install --group dev pytest");
  }

  #[test]
  fn test_dev_add_command_poetry() {
    let cmd = dev_add_command("poetry", "pytest").unwrap();
    assert_eq!(cmd, "poetry add --group dev pytest");
  }

  #[test]
  fn test_dev_add_command_go() {
    let cmd = dev_add_command("go", "github.com/stretchr/testify").unwrap();
    assert_eq!(cmd, "go get -t github.com/stretchr/testify");
  }

  // ===========================================================================
  // supports_dev
  // ===========================================================================

  #[test]
  fn test_supports_dev_known_managers() {
    assert!(supports_dev("pnpm"));
    assert!(supports_dev("npm"));
    assert!(supports_dev("yarn"));
    assert!(supports_dev("cargo"));
    assert!(supports_dev("uv"));
    assert!(supports_dev("pip"));
    assert!(supports_dev("poetry"));
    assert!(supports_dev("go"));
  }

  #[test]
  fn test_supports_dev_unknown_managers() {
    assert!(!supports_dev("brew"));
    assert!(!supports_dev("apt"));
    assert!(!supports_dev("nix"));
    assert!(!supports_dev("nonexistent"));
  }
}
