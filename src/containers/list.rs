//! Local container image listing.
//!
//! Implements `docker images` / `podman images` for listing local container
//! images. Uses the [`ContainerExecutor`] trait so
//! tests can mock the subprocess calls.
//!
//! Both docker and podman support the `--format` Go template syntax. We use
//! a tab-separated template for consistent parsing across runtimes:
//! `{{.Repository}}\t{{.Tag}}\t{{.ID}}\t{{.Size}}`

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::containers::{CommandOutput, ContainerExecutor, ContainerRuntime};
use crate::error::{ApmwError, Result};

/// A local container image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalImage {
  /// The repository name (e.g. `nginx`, `docker.io/library/nginx`).
  pub repository: String,
  /// The tag (e.g. `latest`, `1.21`).
  pub tag: String,
  /// The image ID (short hash).
  pub image_id: String,
  /// The human-readable size (e.g. `142MB`).
  pub size: String,
}

/// Go template for consistent image listing across docker and podman.
const IMAGE_FORMAT: &str = "{{.Repository}}\t{{.Tag}}\t{{.ID}}\t{{.Size}}";

/// Lists local container images using the given runtime.
///
/// Runs `<runtime> images --format <template>` via the executor and parses
/// the tab-separated output into [`LocalImage`] entries.
///
/// # Errors
///
/// Returns [`ApmwError::Io`] if the subprocess cannot be spawned.
/// Returns [`ApmwError::Config`] if the command fails (non-zero exit).
pub async fn list_images<E: ContainerExecutor>(
  executor: &E,
  runtime: ContainerRuntime,
) -> Result<Vec<LocalImage>> {
  let binary = runtime.binary();
  let args = ["images", "--format", IMAGE_FORMAT];

  info!(runtime = binary, "Listing local container images");

  debug!(
    runtime = binary,
    "executing: {} images --format ...", binary
  );

  let output: CommandOutput = executor
    .execute(binary, &args)
    .await
    .map_err(ApmwError::Io)?;

  if !output.success {
    warn!(
      runtime = binary,
      exit_code = ?output.exit_code,
      stderr = %output.stderr,
      "Failed to list container images"
    );
    return Err(ApmwError::Config(format!(
      "failed to list images via {binary}: {}",
      output.stderr.trim()
    )));
  }

  let images = parse_image_list(&output.stdout);

  debug!(
    runtime = binary,
    count = images.len(),
    "Listed local container images"
  );

  Ok(images)
}

/// Parses tab-separated image list output into [`LocalImage`] entries.
///
/// Each line is expected to be: `repository\ttag\timage_id\tsize`.
/// Lines that don't have exactly 4 fields are skipped.
fn parse_image_list(stdout: &str) -> Vec<LocalImage> {
  stdout
    .lines()
    .filter(|line| !line.trim().is_empty())
    .filter_map(|line| {
      let parts: Vec<&str> = line.split('\t').collect();
      if parts.len() != 4 {
        return None;
      }
      Some(LocalImage {
        repository: parts[0].to_string(),
        tag: parts[1].to_string(),
        image_id: parts[2].to_string(),
        size: parts[3].to_string(),
      })
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::containers::{CommandOutput, MockExecutor};

  #[test]
  fn test_parse_image_list_empty() {
    let images = parse_image_list("");
    assert!(images.is_empty());
  }

  #[test]
  fn test_parse_image_list_single() {
    let stdout = "nginx\tlatest\t1234567890ab\t142MB";
    let images = parse_image_list(stdout);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].repository, "nginx");
    assert_eq!(images[0].tag, "latest");
    assert_eq!(images[0].image_id, "1234567890ab");
    assert_eq!(images[0].size, "142MB");
  }

  #[test]
  fn test_parse_image_list_multiple() {
    let stdout = "nginx\tlatest\t1234567890ab\t142MB\nalpine\t3.18\tabcdef123456\t7.8MB\nredis\t7.0\tfedcba654321\t117MB";
    let images = parse_image_list(stdout);
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].repository, "nginx");
    assert_eq!(images[1].repository, "alpine");
    assert_eq!(images[2].repository, "redis");
  }

  #[test]
  fn test_parse_image_list_skips_malformed() {
    let stdout = "nginx\tlatest\t1234567890ab\t142MB\nbad_line\nalpine\t3.18\tabcdef123456\t7.8MB";
    let images = parse_image_list(stdout);
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].repository, "nginx");
    assert_eq!(images[1].repository, "alpine");
  }

  #[test]
  fn test_parse_image_list_skips_blank_lines() {
    let stdout = "\nnginx\tlatest\t1234567890ab\t142MB\n\n";
    let images = parse_image_list(stdout);
    assert_eq!(images.len(), 1);
  }

  #[tokio::test]
  async fn test_list_images_docker() {
    let stdout = "nginx\tlatest\t1234567890ab\t142MB\nalpine\t3.18\tabcdef123456\t7.8MB";
    let exec = MockExecutor::new().with_available("docker").with_output(
      "docker",
      &["images", "--format", IMAGE_FORMAT],
      CommandOutput::success(stdout),
    );
    let images = list_images(&exec, ContainerRuntime::Docker).await.unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0].repository, "nginx");
    assert_eq!(images[1].repository, "alpine");
  }

  #[tokio::test]
  async fn test_list_images_podman() {
    let stdout = "localhost/nginx\tlatest\t1234567890ab\t142MB";
    let exec = MockExecutor::new().with_available("podman").with_output(
      "podman",
      &["images", "--format", IMAGE_FORMAT],
      CommandOutput::success(stdout),
    );
    let images = list_images(&exec, ContainerRuntime::Podman).await.unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].repository, "localhost/nginx");
  }

  #[tokio::test]
  async fn test_list_images_empty() {
    let exec = MockExecutor::new().with_output(
      "docker",
      &["images", "--format", IMAGE_FORMAT],
      CommandOutput::success(""),
    );
    let images = list_images(&exec, ContainerRuntime::Docker).await.unwrap();
    assert!(images.is_empty());
  }

  #[tokio::test]
  async fn test_list_images_failure() {
    let exec = MockExecutor::new().with_output(
      "docker",
      &["images", "--format", IMAGE_FORMAT],
      CommandOutput::failure("Cannot connect to the Docker daemon"),
    );
    let result = list_images(&exec, ContainerRuntime::Docker).await;
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("failed to list images"));
  }

  #[tokio::test]
  async fn test_list_containers_via_engine() {
    let stdout = "nginx\tlatest\t1234567890ab\t142MB";
    let exec = MockExecutor::new().with_available("docker").with_output(
      "docker",
      &["images", "--format", IMAGE_FORMAT],
      CommandOutput::success(stdout),
    );
    let engine = crate::containers::ContainerEngine::new(exec);
    let images = engine.list(Some(ContainerRuntime::Docker)).await.unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].repository, "nginx");
  }

  #[tokio::test]
  async fn test_list_containers_via_engine_auto_detect() {
    let stdout = "nginx\tlatest\t1234567890ab\t142MB";
    let exec = MockExecutor::new().with_available("docker").with_output(
      "docker",
      &["images", "--format", IMAGE_FORMAT],
      CommandOutput::success(stdout),
    );
    let engine = crate::containers::ContainerEngine::new(exec);
    let images = engine.list(None).await.unwrap();
    assert_eq!(images.len(), 1);
  }
}
