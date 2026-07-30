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
pub mod ecosystem;
pub mod error;
pub mod output;
pub mod path_scan;
pub mod security;
pub mod version;

pub use audit::{AuditLogEntry, AuditLogWriter, TerminalType};
pub use cli::{Cli, Commands};
pub use config::ApmwConfig;
pub use daemon::{DaemonManager, DaemonStatus, JobId, JobManager, JobStatus};
pub use detect::{DetectionEngine, DetectionResult};
pub use ecosystem::{ApmwCommand, EcosystemMap, EcosystemMapper};
pub use error::{ApmwError, Result};
pub use output::{AgentFormat, OutputDispatcher, OutputMode, Schema, TruncationConfig};
pub use path_scan::{PathScanner, ScanConfig, ScanResult, ScanSource};
pub use security::{
  AggregatedVerdict, InstallAction, OnRiskMode, PackageScanOutcome, PackageScanRequest,
  ScanConfig as SecurityScanConfig, ScanOrchestrator, ScanReport, ScanResult as SecurityScanResult,
  Scanner, SecurityFinding, SecurityPosture, Severity, TelemetryPolicy,
};
pub use version::{
  MinAgeDaysConfig, RegistryClient, RegistryVersion, ResolutionStrategy, VersionResolution,
  VersionResolver,
};

/// Returns the version of the apmw library.
pub fn version() -> &'static str {
  env!("CARGO_PKG_VERSION")
}
