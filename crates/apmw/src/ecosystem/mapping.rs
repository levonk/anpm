//! Ecosystem mapping table — re-exports from [`apmw_core::ecosystem`].
//!
//! The within-ecosystem command translations (pip -> uv, npm -> pnpm) are
//! defined in `apmw-core` and re-exported here for backwards compatibility.

pub use apmw_core::ecosystem::{
  all_commands, all_ecosystems, all_managers, manager_command, ApmwCommand, Ecosystem,
  EcosystemMap, PackageManager,
};
