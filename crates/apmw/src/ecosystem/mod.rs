//! Ecosystem module for the apmw binary.
//!
//! Re-exports the core ecosystem types from [`apmw_core::ecosystem`], and
//! provides [`mapping`] which contains the within-ecosystem command mapping
//! table (CLI-specific, not in `apmw-core`).

pub mod mapping;

// Re-export the core types from apmw-core.
pub use apmw_core::ecosystem::{
  all_commands, all_ecosystems, all_managers, ApmwCommand, Ecosystem, PackageManager,
};

// Re-export the CLI-specific mapping types from the binary's mapping module.
pub use mapping::{manager_command, EcosystemMap, EcosystemMapper};
