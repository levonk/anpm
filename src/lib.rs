//! apmw — All Package Manager Wrapper
//!
//! Abstracts every package installer into one intelligent surface. Detects the
//! correct package manager, installs tools with install-on-use semantics, and
//! runs security scanning before install.

pub mod agent;
pub mod audit;
pub mod cli;
pub mod config;
pub mod containers;
pub mod daemon;
pub mod detect;
pub mod ecosystem;
pub mod error;
pub mod governance;
pub mod install;
pub mod output;
pub mod path_scan;
pub mod security;
pub mod telemetry;
pub mod version;

pub use agent::{
  default_shim_dir, generate_soft_convention_instructions, home_dir, home_view_content,
  install_all_session_integrations, install_session_integration, notify_docs,
  run_docs_notification, should_notify, ClaudeCodeInstaller, CodexInstaller, DocsNotification,
  DocsNotifier, HookManager, IntegrationResult, InterceptAction, InterceptDecision,
  InterceptResult, OpenCodeInstaller, SessionIntegration, SessionIntegrationInstaller, ShimSpec,
  SkillGenerator, APMW_MARKER, SHIM_MARKER, SHIM_TARGETS, SKILL_FILENAME,
};
pub use audit::{AuditLogEntry, AuditLogWriter, TerminalType};
pub use cli::{Cli, Commands};
pub use config::ApmwConfig;
pub use containers::{
  detect_container_usage, list_images, pull_image, scan_image, CommandOutput,
  ContainerDetectionResult, ContainerEngine, ContainerExecutor, ContainerRuntime,
  ContainerScanResult, ContainerUsage, ContainerUsageKind, DockerComposeInfo,
  GovernanceRule as ContainerGovernanceRule, LocalImage, PullResult, TokioExecutor,
};
pub use daemon::{DaemonManager, DaemonStatus, JobId, JobManager, JobStatus};
pub use detect::{DetectionEngine, DetectionResult};
pub use ecosystem::{ApmwCommand, EcosystemMap, EcosystemMapper};
pub use error::{ApmwError, Result};
pub use governance::{
  GovernanceConfig, GovernanceEngine, GovernanceOutcome, GovernanceRule, GovernanceSpec,
  GovernanceType, MockSpecClient, ReqwestSpecClient, SpecLoader, WrapperKind, WrapperSpec,
};
pub use install::{
  dev_add_command, dev_flags, supports_dev, AddEngine, AddEngineConfig, AddResult, AddStatus,
  DefaultRunnerResolver, MockRunnerResolver, OnUseConfig, OnUseEngine, RoutingDecision, Runner,
  RunnerResolver,
};
pub use output::{AgentFormat, OutputDispatcher, OutputMode, Schema, TruncationConfig};
pub use path_scan::{PathScanner, ScanConfig, ScanResult, ScanSource};
pub use security::{
  AggregatedVerdict, InstallAction, OnRiskMode, PackageScanOutcome, PackageScanRequest,
  ScanConfig as SecurityScanConfig, ScanOrchestrator, ScanReport, ScanResult as SecurityScanResult,
  Scanner, SecurityFinding, SecurityPosture, Severity, TelemetryPolicy,
};
pub use telemetry::{
  ErrorCategory, HttpSender, MockSender, NoopSender, Outcome, TelemetryCollector, TelemetryCommand,
  TelemetryEvent, TelemetryEventBuilder, TelemetrySender, TelemetryTerminalType, Timer,
};
pub use version::{
  MinAgeDaysConfig, RegistryClient, RegistryVersion, ResolutionStrategy, VersionResolution,
  VersionResolver,
};

/// Returns the version of the apmw library.
pub fn version() -> &'static str {
  env!("CARGO_PKG_VERSION")
}
