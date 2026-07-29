//! apmw — All Package Manager Wrapper
//!
//! Abstracts every package installer into one intelligent surface. Detects the
//! correct package manager, installs tools with install-on-use semantics, and
//! runs security scanning before install.

pub mod audit;
pub mod cli;
pub mod config;
pub mod daemon;
pub mod detect;
pub mod error;
pub mod output;
pub mod path_scan;

pub use audit::{AuditLogEntry, AuditLogWriter, TerminalType};
pub use cli::{Cli, Commands};
pub use config::ApmwConfig;
pub use daemon::{DaemonManager, DaemonStatus, JobId, JobManager, JobStatus};
pub use detect::{DetectionEngine, DetectionResult};
pub use error::{ApmwError, Result};
pub use output::{AgentFormat, OutputDispatcher, OutputMode, Schema, TruncationConfig};
pub use path_scan::{PathScanner, ScanConfig, ScanResult, ScanSource};

/// Returns the version of the apmw library.
pub fn version() -> &'static str {
  env!("CARGO_PKG_VERSION")
}
