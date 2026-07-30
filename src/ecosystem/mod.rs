//! Ecosystem mapping engine — within-ecosystem command translation.
//!
//! The [`EcosystemMapper`] translates canonical `apmw` commands into the
//! concrete command string for the canonical package manager within the same
//! ecosystem as the detected source manager.
//!
//! # Core principle: WITHIN-ecosystem only
//!
//! Mapping is **never** cross-ecosystem (PRD FR-2.1). A Python runner maps to
//! a Python runner; a Node runner maps to a Node runner. For example:
//!
//! | Source (ecosystem) | Canonical (same ecosystem) |
//! |-------------------|----------------------------|
//! | `pip` (Python)    | `uv` (Python)              |
//! | `npm` (Node)      | `pnpm` (Node)              |
//! | `yarn` (Node)     | `pnpm` (Node)              |
//! | `bun` (Node)      | `pnpm` (Node)              |
//! | `yarn2` (Node)    | `pnpm` (Node)              |
//!
//! Managers that "remain as-is" per PRD FR-2.1 (poetry, pipenv, pdm, conda in
//! Python; cargo in Rust; go in Go) are their own canonical — they are not
//! forced to a different runner. `suggest_canonical` returns the source
//! manager itself in those cases (no suggestion to switch).
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

pub mod mapping;

pub use mapping::{
  all_commands, all_ecosystems, all_managers, manager_command, ApmwCommand, Ecosystem,
  EcosystemMap, PackageManager,
};

use std::collections::HashMap;

use tracing::{debug, warn};

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
    let maps = mapping::all_managers()
      .into_iter()
      .map(|source| {
        let canonical = source.ecosystem().canonical_manager();
        let mut command_mapping = HashMap::new();
        for cmd in mapping::all_commands() {
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
    mapping::all_managers()
  }

  /// Returns all known ecosystems.
  pub fn all_ecosystems(&self) -> Vec<Ecosystem> {
    mapping::all_ecosystems()
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
    for source in mapping::all_managers() {
      let source_ecosystem = source.ecosystem();
      let canonical = mapper.suggest_canonical(source);
      // The canonical manager must be in the SAME ecosystem as the source.
      assert_eq!(
        canonical.ecosystem(),
        source_ecosystem,
        "suggest_canonical({source}) returned {canonical} from a different ecosystem"
      );
      // Every mapped command must come from the canonical manager (same ecosystem).
      for cmd in mapping::all_commands() {
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
    for cmd in mapping::all_commands() {
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
    for cmd in mapping::all_commands() {
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
    let managers = mapping::all_managers();
    let commands = mapping::all_commands();

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
    let managers = mapping::all_managers();

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
    let managers = mapping::all_managers();
    let commands = mapping::all_commands();

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
    let managers = mapping::all_managers();

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
    let managers = mapping::all_managers();

    proptest!(|(mi in 0usize..managers.len())| {
      let source = managers[mi];
      prop_assert!(mapper.map_for(source).is_some(), "no map for {source}");
    });
  }
}
