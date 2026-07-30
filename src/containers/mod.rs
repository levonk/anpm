//! Container package support engine.
//!
//! Provides container image pull (docker/podman), security scanning
//! (trivy/grype), local image listing, and container usage detection from
//! project files. Uses a trait-based executor so tests can mock subprocess
//! calls without requiring docker or podman to be installed.
//!
//! # Governance
//!
//! The [`GovernanceRule`] enum controls which container runtime is preferred
//! when both docker and podman are available. The `prefer-podman` rule routes
//! docker calls through podman, matching the governance spec from
//! levonk-packages.
//!
//! # Example
//!
//! ```no_run
//! use apmw::containers::{ContainerEngine, TokioExecutor, GovernanceRule};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let engine = ContainerEngine::new(TokioExecutor::new())
//!   .with_governance(GovernanceRule::PreferPodman);
//! let result = engine.pull("nginx:latest", false).await?;
//! println!("Pulled {} via {}", result.image, result.runtime);
//! # Ok(())
//! # }
//! ```

pub mod detect;
pub mod list;
pub mod pull;
pub mod scan;

pub use detect::{
  detect_container_usage, ContainerDetectionResult, ContainerUsage, ContainerUsageKind,
  DockerComposeInfo,
};
pub use list::{list_images, LocalImage};
pub use pull::{pull_image, PullResult};
pub use scan::{scan_image, ContainerScanResult};

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{ApmwError, Result};

/// A boxed future returned by the [`ContainerExecutor`] trait.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The output of a subprocess command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutput {
  /// Whether the command exited successfully (exit code 0).
  pub success: bool,
  /// The exit code, if available.
  pub exit_code: Option<i32>,
  /// stdout content as a string.
  pub stdout: String,
  /// stderr content as a string.
  pub stderr: String,
}

impl CommandOutput {
  /// Creates a successful command output with the given stdout.
  pub fn success(stdout: impl Into<String>) -> Self {
    Self {
      success: true,
      exit_code: Some(0),
      stdout: stdout.into(),
      stderr: String::new(),
    }
  }

  /// Creates a failed command output with the given stderr.
  pub fn failure(stderr: impl Into<String>) -> Self {
    Self {
      success: false,
      exit_code: Some(1),
      stdout: String::new(),
      stderr: stderr.into(),
    }
  }
}

/// Trait for executing subprocess commands.
///
/// Implementations include [`TokioExecutor`] (real subprocess calls via
/// tokio) and a mock executor for testing. This abstraction lets tests
/// mock docker/podman/trivy/grype calls without requiring those binaries.
pub trait ContainerExecutor: Send + Sync {
  /// Executes a command and returns its output.
  fn execute(&self, program: &str, args: &[&str]) -> BoxFuture<'_, std::io::Result<CommandOutput>>;

  /// Returns `true` if the given program is available on PATH.
  fn is_available(&self, program: &str) -> BoxFuture<'_, bool>;
}

/// Real executor that runs subprocesses via `tokio::process::Command`.
#[derive(Debug, Default, Clone)]
pub struct TokioExecutor;

impl TokioExecutor {
  /// Creates a new `TokioExecutor`.
  pub fn new() -> Self {
    Self
  }
}

impl ContainerExecutor for TokioExecutor {
  fn execute(&self, program: &str, args: &[&str]) -> BoxFuture<'_, std::io::Result<CommandOutput>> {
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    Box::pin(async move {
      let output = tokio::process::Command::new(&program)
        .args(&args)
        .output()
        .await?;
      Ok(CommandOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
      })
    })
  }

  fn is_available(&self, program: &str) -> BoxFuture<'_, bool> {
    let program = program.to_string();
    Box::pin(async move { which::which(&program).is_ok() })
  }
}

/// Mock executor for testing.
///
/// Returns pre-configured outputs for specific commands. Commands that
/// haven't been configured return an error.
#[cfg(test)]
#[derive(Debug, Default, Clone)]
pub struct MockExecutor {
  outputs: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, CommandOutput>>>,
  available: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

#[cfg(test)]
impl MockExecutor {
  /// Creates a new mock executor.
  pub fn new() -> Self {
    Self::default()
  }

  /// Configures the output for a specific command (program + args).
  pub fn with_output(self, program: &str, args: &[&str], output: CommandOutput) -> Self {
    let key = command_key(program, args);
    self.outputs.lock().unwrap().insert(key, output);
    self
  }

  /// Marks a program as available on PATH.
  pub fn with_available(self, program: &str) -> Self {
    self.available.lock().unwrap().insert(program.to_string());
    self
  }
}

#[cfg(test)]
fn command_key(program: &str, args: &[&str]) -> String {
  let mut key = program.to_string();
  for arg in args {
    key.push(' ');
    key.push_str(arg);
  }
  key
}

#[cfg(test)]
impl ContainerExecutor for MockExecutor {
  fn execute(&self, program: &str, args: &[&str]) -> BoxFuture<'_, std::io::Result<CommandOutput>> {
    let key = command_key(program, args);
    let output = self.outputs.lock().unwrap().get(&key).cloned();
    Box::pin(async move {
      output.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, format!("no mock for: {key}"))
      })
    })
  }

  fn is_available(&self, program: &str) -> BoxFuture<'_, bool> {
    let available = self.available.lock().unwrap().contains(program);
    Box::pin(async move { available })
  }
}

/// The container runtime to use (docker or podman).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
  /// Docker runtime.
  Docker,
  /// Podman runtime.
  Podman,
}

impl ContainerRuntime {
  /// Returns the binary name for this runtime.
  pub fn binary(&self) -> &'static str {
    match self {
      ContainerRuntime::Docker => "docker",
      ContainerRuntime::Podman => "podman",
    }
  }

  /// Returns the display name.
  pub fn as_str(&self) -> &'static str {
    match self {
      ContainerRuntime::Docker => "docker",
      ContainerRuntime::Podman => "podman",
    }
  }
}

impl std::fmt::Display for ContainerRuntime {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

/// Governance rules for container runtimes.
///
/// Controls which runtime is preferred when both docker and podman are
/// available. The `prefer-podman` rule routes docker calls through podman,
/// matching the governance spec from levonk-packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GovernanceRule {
  /// No governance rule — use the first available runtime (docker preferred).
  #[default]
  Default,
  /// Prefer podman over docker.
  PreferPodman,
  /// Force podman — never use docker even if available.
  ForcePodman,
  /// Prefer docker over podman.
  PreferDocker,
  /// Force docker — never use podman even if available.
  ForceDocker,
}

/// The container engine — coordinates pull, scan, list, and detect operations.
///
/// Holds a reference to the executor and governance rules. All operations
/// go through the executor so they can be mocked in tests.
pub struct ContainerEngine<E: ContainerExecutor> {
  executor: E,
  governance: GovernanceRule,
}

impl<E: ContainerExecutor> ContainerEngine<E> {
  /// Creates a new container engine with the given executor.
  pub fn new(executor: E) -> Self {
    Self {
      executor,
      governance: GovernanceRule::Default,
    }
  }

  /// Sets the governance rule.
  pub fn with_governance(mut self, rule: GovernanceRule) -> Self {
    self.governance = rule;
    self
  }

  /// Returns a reference to the executor.
  pub fn executor(&self) -> &E {
    &self.executor
  }

  /// Returns the governance rule.
  pub fn governance(&self) -> GovernanceRule {
    self.governance
  }

  /// Detects the best available container runtime, applying governance rules.
  ///
  /// Checks for docker and podman availability, then applies the governance
  /// rule to select the preferred runtime. Returns an error if neither is
  /// available.
  pub async fn detect_runtime(&self) -> Result<ContainerRuntime> {
    let docker_available = self.executor.is_available("docker").await;
    let podman_available = self.executor.is_available("podman").await;

    debug!(
      docker_available,
      podman_available,
      governance = ?self.governance,
      "Detecting container runtime"
    );

    let runtime = match self.governance {
      GovernanceRule::ForcePodman => {
        if podman_available {
          info!(
            runtime = "podman",
            governance = "force-podman",
            "Selected container runtime"
          );
          ContainerRuntime::Podman
        } else {
          return Err(ApmwError::PackageManagerNotFound(
            "governance rule 'force-podman' is set but podman is not available".to_string(),
          ));
        }
      }
      GovernanceRule::ForceDocker => {
        if docker_available {
          info!(
            runtime = "docker",
            governance = "force-docker",
            "Selected container runtime"
          );
          ContainerRuntime::Docker
        } else {
          return Err(ApmwError::PackageManagerNotFound(
            "governance rule 'force-docker' is set but docker is not available".to_string(),
          ));
        }
      }
      GovernanceRule::PreferPodman => {
        if podman_available {
          info!(
            runtime = "podman",
            governance = "prefer-podman",
            "Selected container runtime (podman preferred over docker)"
          );
          ContainerRuntime::Podman
        } else if docker_available {
          warn!(
            runtime = "docker",
            governance = "prefer-podman",
            "podman not available, falling back to docker"
          );
          ContainerRuntime::Docker
        } else {
          return Err(ApmwError::PackageManagerNotFound(
            "no container runtime available (neither docker nor podman found on PATH)".to_string(),
          ));
        }
      }
      GovernanceRule::PreferDocker | GovernanceRule::Default => {
        if docker_available {
          info!(runtime = "docker", "Selected container runtime");
          ContainerRuntime::Docker
        } else if podman_available {
          info!(runtime = "podman", "docker not available, using podman");
          ContainerRuntime::Podman
        } else {
          return Err(ApmwError::PackageManagerNotFound(
            "no container runtime available (neither docker nor podman found on PATH)".to_string(),
          ));
        }
      }
    };

    Ok(runtime)
  }

  /// Pulls a container image.
  ///
  /// When `dev` is `true`, the image is marked as a development/build-time
  /// dependency. The pull command itself is the same; the `dev` flag is
  /// recorded in the [`PullResult`] for audit and telemetry purposes.
  pub async fn pull(&self, image: &str, dev: bool) -> Result<PullResult> {
    let runtime = self.detect_runtime().await?;
    pull::pull_image(&self.executor, runtime, image, dev).await
  }

  /// Scans a container image for vulnerabilities using trivy and/or grype.
  pub async fn scan(&self, image: &str) -> Result<ContainerScanResult> {
    scan::scan_image(&self.executor, image).await
  }

  /// Lists local container images.
  ///
  /// If `runtime` is `None`, the runtime is auto-detected via governance rules.
  pub async fn list(&self, runtime: Option<ContainerRuntime>) -> Result<Vec<LocalImage>> {
    let runtime = match runtime {
      Some(r) => r,
      None => self.detect_runtime().await?,
    };
    list::list_images(&self.executor, runtime).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_detect_runtime_prefers_docker_by_default() {
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_available("podman");
    let engine = ContainerEngine::new(exec);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Docker);
  }

  #[tokio::test]
  async fn test_detect_runtime_prefer_podman() {
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_available("podman");
    let engine = ContainerEngine::new(exec).with_governance(GovernanceRule::PreferPodman);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Podman);
  }

  #[tokio::test]
  async fn test_detect_runtime_force_podman() {
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_available("podman");
    let engine = ContainerEngine::new(exec).with_governance(GovernanceRule::ForcePodman);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Podman);
  }

  #[tokio::test]
  async fn test_detect_runtime_force_podman_fails_without_podman() {
    let exec = MockExecutor::new().with_available("docker");
    let engine = ContainerEngine::new(exec).with_governance(GovernanceRule::ForcePodman);
    let result = engine.detect_runtime().await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_detect_runtime_force_docker() {
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_available("podman");
    let engine = ContainerEngine::new(exec).with_governance(GovernanceRule::ForceDocker);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Docker);
  }

  #[tokio::test]
  async fn test_detect_runtime_no_runtime_available() {
    let exec = MockExecutor::new();
    let engine = ContainerEngine::new(exec);
    let result = engine.detect_runtime().await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_detect_runtime_prefer_podman_falls_back_to_docker() {
    let exec = MockExecutor::new().with_available("docker");
    let engine = ContainerEngine::new(exec).with_governance(GovernanceRule::PreferPodman);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Docker);
  }

  #[tokio::test]
  async fn test_detect_runtime_default_falls_back_to_podman() {
    let exec = MockExecutor::new().with_available("podman");
    let engine = ContainerEngine::new(exec);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Podman);
  }

  #[tokio::test]
  async fn test_governance_podman_routes_docker_to_podman() {
    // With prefer-podman, even if docker is the only one available,
    // the governance rule is logged and podman is preferred when present.
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_available("podman");
    let engine = ContainerEngine::new(exec).with_governance(GovernanceRule::PreferPodman);
    let runtime = engine.detect_runtime().await.unwrap();
    assert_eq!(runtime, ContainerRuntime::Podman);
    assert_eq!(engine.governance(), GovernanceRule::PreferPodman);
  }

  #[test]
  fn test_container_runtime_binary() {
    assert_eq!(ContainerRuntime::Docker.binary(), "docker");
    assert_eq!(ContainerRuntime::Podman.binary(), "podman");
  }

  #[test]
  fn test_container_runtime_display() {
    assert_eq!(ContainerRuntime::Docker.to_string(), "docker");
    assert_eq!(ContainerRuntime::Podman.to_string(), "podman");
  }

  #[test]
  fn test_command_output_success() {
    let out = CommandOutput::success("hello");
    assert!(out.success);
    assert_eq!(out.stdout, "hello");
    assert_eq!(out.exit_code, Some(0));
  }

  #[test]
  fn test_command_output_failure() {
    let out = CommandOutput::failure("error");
    assert!(!out.success);
    assert_eq!(out.stderr, "error");
  }

  #[test]
  fn test_governance_rule_default() {
    assert_eq!(GovernanceRule::default(), GovernanceRule::Default);
  }

  #[tokio::test]
  async fn test_mock_executor_returns_configured_output() {
    let exec = MockExecutor::new().with_output(
      "docker",
      &["pull", "nginx:latest"],
      CommandOutput::success("Pull complete"),
    );
    let result = exec.execute("docker", &["pull", "nginx:latest"]).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert_eq!(output.stdout, "Pull complete");
  }

  #[tokio::test]
  async fn test_mock_executor_unconfigured_command_errors() {
    let exec = MockExecutor::new();
    let result = exec.execute("docker", &["pull", "nginx:latest"]).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_mock_executor_available() {
    let exec = MockExecutor::new().with_available("docker");
    assert!(exec.is_available("docker").await);
    assert!(!exec.is_available("podman").await);
  }
}
