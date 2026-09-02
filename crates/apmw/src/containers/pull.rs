//! Container image pull operations.
//!
//! Implements `docker pull` / `podman pull` for fetching container images.
//! The pull goes through the [`ContainerExecutor`]
//! trait so it can be mocked in tests.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::containers::{CommandOutput, ContainerExecutor, ContainerRuntime};
use crate::error::{ApmwError, Result};

/// The result of pulling a container image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullResult {
  /// The image reference that was pulled (e.g. `nginx:latest`).
  pub image: String,
  /// The runtime used to pull the image (docker or podman).
  pub runtime: ContainerRuntime,
  /// Whether this image was pulled as a development/build-time dependency.
  pub dev: bool,
  /// stdout from the pull command.
  pub output: String,
}

/// Pulls a container image using the given runtime.
///
/// Runs `<runtime> pull <image>` via the executor. When `dev` is `true`, the
/// image is marked as a development/build-time dependency — the pull command
/// itself is unchanged, but the `dev` flag is recorded in the [`PullResult`]
/// for audit and telemetry purposes.
///
/// # Errors
///
/// Returns [`ApmwError::Io`] if the subprocess cannot be spawned.
/// Returns [`ApmwError::Config`] if the pull command fails (non-zero exit).
pub async fn pull_image<E: ContainerExecutor>(
  executor: &E,
  runtime: ContainerRuntime,
  image: &str,
  dev: bool,
) -> Result<PullResult> {
  let binary = runtime.binary();
  let args = ["pull", image];

  info!(runtime = binary, image, dev, "Pulling container image");

  debug!(
    runtime = binary,
    image, "executing: {} pull {}", binary, image
  );

  let output: CommandOutput = executor
    .execute(binary, &args)
    .await
    .map_err(ApmwError::Io)?;

  if !output.success {
    warn!(
      runtime = binary,
      image,
      exit_code = ?output.exit_code,
      stderr = %output.stderr,
      "Container image pull failed"
    );
    return Err(ApmwError::Config(format!(
      "failed to pull image '{image}' via {binary}: {}",
      output.stderr.trim()
    )));
  }

  info!(
    runtime = binary,
    image, dev, "Container image pulled successfully"
  );

  Ok(PullResult {
    image: image.to_string(),
    runtime,
    dev,
    output: output.stdout,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::containers::{CommandOutput, MockExecutor};

  #[tokio::test]
  async fn test_pull_image_docker_success() {
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_output(
        "docker",
        &["pull", "nginx:latest"],
        CommandOutput::success("latest: Pulling from library/nginx\nDigest: sha256:abc\nStatus: Image is up to date for nginx:latest"),
      );
    let result = pull_image(&exec, ContainerRuntime::Docker, "nginx:latest", false)
      .await
      .unwrap();
    assert_eq!(result.image, "nginx:latest");
    assert_eq!(result.runtime, ContainerRuntime::Docker);
    assert!(!result.dev);
    assert!(result.output.contains("nginx:latest"));
  }

  #[tokio::test]
  async fn test_pull_image_podman_success() {
    let exec = MockExecutor::new().with_output(
      "podman",
      &["pull", "alpine:3.18"],
      CommandOutput::success("Trying to pull docker.io/library/alpine:3.18\nGetting image source signatures\nCopying blob sha256 done\nCopying config sha256 done\nWriting manifest to image destination\nStoring signatures"),
    );
    let result = pull_image(&exec, ContainerRuntime::Podman, "alpine:3.18", false)
      .await
      .unwrap();
    assert_eq!(result.image, "alpine:3.18");
    assert_eq!(result.runtime, ContainerRuntime::Podman);
    assert!(!result.dev);
  }

  #[tokio::test]
  async fn test_pull_image_dev_flag() {
    let exec = MockExecutor::new().with_output(
      "docker",
      &["pull", "node:20"],
      CommandOutput::success("Pull complete"),
    );
    let result = pull_image(&exec, ContainerRuntime::Docker, "node:20", true)
      .await
      .unwrap();
    assert!(result.dev);
    assert_eq!(result.image, "node:20");
  }

  #[tokio::test]
  async fn test_pull_image_failure() {
    let exec = MockExecutor::new().with_output(
      "docker",
      &["pull", "nonexistent:tag"],
      CommandOutput::failure("Error: manifest for nonexistent:tag not found"),
    );
    let result = pull_image(&exec, ContainerRuntime::Docker, "nonexistent:tag", false).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("failed to pull image"));
    assert!(err.contains("nonexistent:tag"));
  }

  #[tokio::test]
  async fn test_pull_image_executor_error() {
    let exec = MockExecutor::new();
    let result = pull_image(&exec, ContainerRuntime::Docker, "nginx:latest", false).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_pull_image_with_full_reference() {
    let image = "gcr.io/my-project/my-image:v1.2.3";
    let exec = MockExecutor::new().with_output(
      "docker",
      &["pull", image],
      CommandOutput::success("Pull complete"),
    );
    let result = pull_image(&exec, ContainerRuntime::Docker, image, false)
      .await
      .unwrap();
    assert_eq!(result.image, image);
  }

  #[tokio::test]
  async fn test_add_container_via_engine() {
    let exec = MockExecutor::new().with_available("docker").with_output(
      "docker",
      &["pull", "nginx:latest"],
      CommandOutput::success("Pull complete"),
    );
    let engine = crate::containers::ContainerEngine::new(exec);
    let result = engine.pull("nginx:latest", false).await.unwrap();
    assert_eq!(result.image, "nginx:latest");
    assert_eq!(result.runtime, ContainerRuntime::Docker);
    assert!(!result.dev);
  }

  #[tokio::test]
  async fn test_add_container_dev_via_engine() {
    let exec = MockExecutor::new().with_available("docker").with_output(
      "docker",
      &["pull", "node:20"],
      CommandOutput::success("Pull complete"),
    );
    let engine = crate::containers::ContainerEngine::new(exec);
    let result = engine.pull("node:20", true).await.unwrap();
    assert!(result.dev);
    assert_eq!(result.image, "node:20");
  }

  #[tokio::test]
  async fn test_add_container_podman_via_governance() {
    let exec = MockExecutor::new()
      .with_available("docker")
      .with_available("podman")
      .with_output(
        "podman",
        &["pull", "nginx:latest"],
        CommandOutput::success("Pull complete"),
      );
    let engine = crate::containers::ContainerEngine::new(exec)
      .with_governance(crate::containers::GovernanceRule::PreferPodman);
    let result = engine.pull("nginx:latest", false).await.unwrap();
    assert_eq!(result.runtime, ContainerRuntime::Podman);
  }
}
