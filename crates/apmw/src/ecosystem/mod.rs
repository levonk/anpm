//! Ecosystem module for the apmw binary.
//!
//! Re-exports the core ecosystem types and mapper from [`apmw_core::ecosystem`],
//! and provides [`mapping`] which re-exports the command mapping table.

pub mod mapping;

pub use apmw_core::ecosystem::{
  all_commands, all_ecosystems, all_managers, manager_command, ApmwCommand, Ecosystem,
  EcosystemMap, EcosystemMapper, PackageManager,
};
