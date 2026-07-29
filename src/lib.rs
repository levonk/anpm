//! apmw — All Package Manager Wrapper
//!
//! Abstracts every package installer into one intelligent surface. Detects the
//! correct package manager, installs tools with install-on-use semantics, and
//! runs security scanning before install.

pub mod audit;
pub mod cli;
pub mod config;
pub mod error;

pub use audit::{AuditLogEntry, AuditLogWriter, TerminalType};
pub use cli::{Cli, Commands};
pub use config::ApmwConfig;
pub use error::{ApmwError, Result};

/// Returns the version of the apmw library.
pub fn version() -> &'static str {
  env!("CARGO_PKG_VERSION")
}
