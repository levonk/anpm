//! AI agent docs notification — notify the agent where to find docs.
//!
//! After `apmw add <package>` installs a package, the [`DocsNotifier`] produces
//! a notification that tells the AI agent where to find the documentation for
//! the added package. The notification includes:
//!
//! - **Package name** — the package that was added.
//! - **Docs URL** — a documentation URL derived from package metadata or a
//!   heuristic based on the ecosystem/manager.
//! - **Local README/docs path** — the local path to the package's README or
//!   docs directory (if it can be determined).
//!
//! This implements PRD FR-11 — AI agent docs notification.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::Result;

/// A docs notification — tells the AI agent where to find docs for a package.
///
/// Produced by [`DocsNotifier`] after a successful `apmw add`. The notification
/// is intended to be emitted to the agent (via stdout in AXI/TOON mode) so the
/// agent knows where to look up the package's documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsNotification {
  /// The package that was added.
  pub package: String,
  /// The package manager that was used (e.g. `pnpm`, `uv`).
  pub manager: String,
  /// The canonical manager (after ecosystem mapping).
  pub canonical_manager: String,
  /// The resolved version (if known).
  pub version: Option<String>,
  /// The documentation URL for the package (from metadata or heuristic).
  pub docs_url: String,
  /// The local path to the package's README or docs (if determinable).
  pub local_docs_path: Option<String>,
}

impl DocsNotification {
  /// Render the notification as a single-line summary suitable for agent
  /// consumption (token-budget-aware).
  pub fn to_summary(&self) -> String {
    let version_part = self
      .version
      .as_ref()
      .map(|v| format!("@{v}"))
      .unwrap_or_default();
    let local_part = self
      .local_docs_path
      .as_ref()
      .map(|p| format!(" | local: {p}"))
      .unwrap_or_default();
    format!(
      "docs: {}{} → {}{}",
      self.package, version_part, self.docs_url, local_part
    )
  }
}

impl std::fmt::Display for DocsNotification {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_summary())
  }
}

/// The docs notifier — produces [`DocsNotification`] values after a package is
/// added.
///
/// The notifier uses a combination of package metadata (when available) and
/// ecosystem-specific heuristics to determine the docs URL. Local docs paths
/// are resolved relative to the project directory and the package manager's
/// installation layout.
pub struct DocsNotifier {
  /// The project directory where the package was added.
  project_dir: PathBuf,
}

impl DocsNotifier {
  /// Create a new docs notifier for the given project directory.
  pub fn new(project_dir: &Path) -> Self {
    DocsNotifier {
      project_dir: project_dir.to_path_buf(),
    }
  }

  /// Build a docs notification for a package that was just added.
  ///
  /// `manager` is the canonical manager used (e.g. `pnpm`, `uv`). `version` is
  /// the resolved version, if known.
  pub fn notify(
    &self,
    package: &str,
    manager: &str,
    canonical_manager: &str,
    version: Option<&str>,
  ) -> Result<DocsNotification> {
    info!(
      package = package,
      manager = manager,
      "building docs notification"
    );

    let docs_url = resolve_docs_url(package, canonical_manager);
    let local_docs_path = self.resolve_local_docs_path(package, canonical_manager);

    debug!(
      package = package,
      docs_url = %docs_url,
      local = ?local_docs_path,
      "docs notification resolved"
    );

    Ok(DocsNotification {
      package: package.to_string(),
      manager: manager.to_string(),
      canonical_manager: canonical_manager.to_string(),
      version: version.map(|s| s.to_string()),
      docs_url,
      local_docs_path,
    })
  }

  /// Attempt to resolve the local path to the package's README or docs.
  ///
  /// This checks common installation layouts per ecosystem:
  /// - **Node (pnpm/npm)**: `node_modules/<package>/README.md`
  /// - **Python (uv/pip)**: `<package>/README.md` in site-packages (best-effort)
  /// - **Rust (cargo)**: not resolvable locally (docs are on docs.rs)
  /// - **Go**: not resolvable locally (docs are on pkg.go.dev)
  fn resolve_local_docs_path(&self, package: &str, manager: &str) -> Option<String> {
    match manager {
      "pnpm" | "npm" | "yarn" | "bun" => {
        let readme = self
          .project_dir
          .join("node_modules")
          .join(package)
          .join("README.md");
        if readme.exists() {
          return Some(readme.display().to_string());
        }
        // Also check for a docs directory.
        let docs_dir = self
          .project_dir
          .join("node_modules")
          .join(package)
          .join("docs");
        if docs_dir.is_dir() {
          return Some(docs_dir.display().to_string());
        }
        None
      }
      "uv" | "pip" | "poetry" | "pipenv" | "pdm" => {
        // Python packages install into site-packages, which varies by
        // environment. We do a best-effort check of common venv locations.
        let candidates = [
          self.project_dir.join(".venv").join("lib"),
          self.project_dir.join("venv").join("lib"),
        ];
        for base in &candidates {
          if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
              let readme = entry
                .path()
                .join("site-packages")
                .join(package)
                .join("README.md");
              if readme.exists() {
                return Some(readme.display().to_string());
              }
              // Also try the package name with hyphens replaced.
              let readme_alt = entry
                .path()
                .join("site-packages")
                .join(package.replace('-', "_"))
                .join("README.md");
              if readme_alt.exists() {
                return Some(readme_alt.display().to_string());
              }
            }
          }
        }
        None
      }
      // For ecosystems where docs are primarily online, return None.
      _ => None,
    }
  }
}

/// Resolve a documentation URL for a package using ecosystem-specific
/// heuristics.
///
/// This function does not make network calls — it constructs a URL based on the
/// canonical manager and package name. When real package metadata is available
/// (e.g. from a registry response), it should be preferred over this
/// heuristic.
pub fn resolve_docs_url(package: &str, canonical_manager: &str) -> String {
  match canonical_manager {
    "pnpm" | "npm" | "yarn" | "bun" => format!("https://www.npmjs.com/package/{package}"),
    "uv" | "pip" | "poetry" | "pipenv" | "pdm" => format!("https://pypi.org/project/{package}/"),
    "cargo" => format!("https://docs.rs/{package}"),
    "go" => format!("https://pkg.go.dev/{package}"),
    "gem" => format!("https://rubygems.org/gems/{package}"),
    "brew" => format!("https://formulae.brew.sh/formula/{package}"),
    "apt" => format!("https://packages.debian.org/search?keywords={package}"),
    "maven" | "gradle" | "sbt" => format!("https://search.maven.org/search?q={package}"),
    "dotnet" => format!("https://www.nuget.org/packages/{package}"),
    "helm" => format!("https://artifacthub.io/packages/helm/{package}"),
    "docker" | "podman" => format!("https://hub.docker.com/_/{package}"),
    _ => format!("https://www.google.com/search?q={package}+documentation"),
  }
}

/// Build and emit a docs notification for an add result.
///
/// This is a convenience function that creates a [`DocsNotifier`] for the given
/// project directory and produces a notification. It is intended to be called
/// after a successful `apmw add` operation.
pub fn notify_docs(
  project_dir: &Path,
  package: &str,
  manager: &str,
  canonical_manager: &str,
  version: Option<&str>,
) -> Result<DocsNotification> {
  let notifier = DocsNotifier::new(project_dir);
  notifier.notify(package, manager, canonical_manager, version)
}

/// Check whether a docs notification should be emitted for the given add
/// status.
///
/// Notifications are emitted only for successful adds (not dry-runs, skips, or
/// security aborts).
pub fn should_notify(status: &crate::install::AddStatus) -> bool {
  matches!(status, crate::install::AddStatus::Added)
}

/// Run the docs notification step after an add operation.
///
/// This is the main entry point for wiring docs notification into the add
/// flow. It checks whether notification is appropriate (the add succeeded),
/// builds the notification, and logs it at `debug` level per the story's
/// observability requirements.
///
/// Returns `Ok(Some(notification))` if a notification was produced, or
/// `Ok(None)` if notification was skipped (e.g. dry-run or already-installed).
pub fn run_docs_notification(
  project_dir: &Path,
  package: &str,
  manager: &str,
  canonical_manager: &str,
  version: Option<&str>,
  status: &crate::install::AddStatus,
) -> Result<Option<DocsNotification>> {
  if !should_notify(status) {
    debug!(
      package = package,
      status = ?status,
      "skipping docs notification (add did not succeed)"
    );
    return Ok(None);
  }
  let notification = notify_docs(project_dir, package, manager, canonical_manager, version)?;
  debug!(notification = %notification, "docs notification emitted");
  Ok(Some(notification))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::install::AddStatus;
  use tempfile::TempDir;

  // -------------------------------------------------------------------------
  // DocsNotification
  // -------------------------------------------------------------------------

  #[test]
  fn test_docs_notification_to_summary_with_version() {
    let n = DocsNotification {
      package: "express".to_string(),
      manager: "pnpm".to_string(),
      canonical_manager: "pnpm".to_string(),
      version: Some("4.18.2".to_string()),
      docs_url: "https://www.npmjs.com/package/express".to_string(),
      local_docs_path: Some("/tmp/node_modules/express/README.md".to_string()),
    };
    let summary = n.to_summary();
    assert!(summary.contains("express@4.18.2"));
    assert!(summary.contains("npmjs.com"));
    assert!(summary.contains("local:"));
  }

  #[test]
  fn test_docs_notification_to_summary_without_version() {
    let n = DocsNotification {
      package: "react".to_string(),
      manager: "pnpm".to_string(),
      canonical_manager: "pnpm".to_string(),
      version: None,
      docs_url: "https://www.npmjs.com/package/react".to_string(),
      local_docs_path: None,
    };
    let summary = n.to_summary();
    assert!(summary.contains("react"));
    assert!(!summary.contains('@'));
    assert!(!summary.contains("local:"));
  }

  #[test]
  fn test_docs_notification_display() {
    let n = DocsNotification {
      package: "foo".to_string(),
      manager: "uv".to_string(),
      canonical_manager: "uv".to_string(),
      version: None,
      docs_url: "https://pypi.org/project/foo/".to_string(),
      local_docs_path: None,
    };
    let s = format!("{n}");
    assert!(s.contains("foo"));
    assert!(s.contains("pypi.org"));
  }

  #[test]
  fn test_docs_notification_serialization() {
    let n = DocsNotification {
      package: "foo".to_string(),
      manager: "pnpm".to_string(),
      canonical_manager: "pnpm".to_string(),
      version: Some("1.0.0".to_string()),
      docs_url: "https://www.npmjs.com/package/foo".to_string(),
      local_docs_path: None,
    };
    let json = serde_json::to_string(&n).unwrap();
    assert!(json.contains("\"package\":\"foo\""));
    assert!(json.contains("\"docs_url\""));
    let back: DocsNotification = serde_json::from_str(&json).unwrap();
    assert_eq!(back, n);
  }

  // -------------------------------------------------------------------------
  // resolve_docs_url
  // -------------------------------------------------------------------------

  #[test]
  fn test_resolve_docs_url_npm() {
    let url = resolve_docs_url("express", "pnpm");
    assert_eq!(url, "https://www.npmjs.com/package/express");
  }

  #[test]
  fn test_resolve_docs_url_pip() {
    let url = resolve_docs_url("requests", "uv");
    assert_eq!(url, "https://pypi.org/project/requests/");
  }

  #[test]
  fn test_resolve_docs_url_cargo() {
    let url = resolve_docs_url("serde", "cargo");
    assert_eq!(url, "https://docs.rs/serde");
  }

  #[test]
  fn test_resolve_docs_url_go() {
    let url = resolve_docs_url("fmt", "go");
    assert_eq!(url, "https://pkg.go.dev/fmt");
  }

  #[test]
  fn test_resolve_docs_url_gem() {
    let url = resolve_docs_url("rails", "gem");
    assert_eq!(url, "https://rubygems.org/gems/rails");
  }

  #[test]
  fn test_resolve_docs_url_brew() {
    let url = resolve_docs_url("ripgrep", "brew");
    assert_eq!(url, "https://formulae.brew.sh/formula/ripgrep");
  }

  #[test]
  fn test_resolve_docs_url_unknown_manager_fallback() {
    let url = resolve_docs_url("foo", "unknown-manager");
    assert!(url.contains("google.com"));
    assert!(url.contains("foo"));
  }

  // -------------------------------------------------------------------------
  // DocsNotifier — local docs path resolution
  // -------------------------------------------------------------------------

  #[test]
  fn test_docs_notifier_node_modules_readme() {
    let dir = TempDir::new().unwrap();
    let pkg_dir = dir.path().join("node_modules").join("express");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("README.md"), "# express").unwrap();

    let notifier = DocsNotifier::new(dir.path());
    let path = notifier.resolve_local_docs_path("express", "pnpm");
    assert!(path.is_some(), "should find README in node_modules");
    assert!(path.unwrap().contains("README.md"));
  }

  #[test]
  fn test_docs_notifier_node_modules_docs_dir() {
    let dir = TempDir::new().unwrap();
    let pkg_dir = dir.path().join("node_modules").join("lib");
    let docs_dir = pkg_dir.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();

    let notifier = DocsNotifier::new(dir.path());
    let path = notifier.resolve_local_docs_path("lib", "npm");
    assert!(path.is_some(), "should find docs directory");
    assert!(path.unwrap().ends_with("docs"));
  }

  #[test]
  fn test_docs_notifier_no_local_docs() {
    let dir = TempDir::new().unwrap();
    let notifier = DocsNotifier::new(dir.path());
    let path = notifier.resolve_local_docs_path("nonexistent-pkg", "pnpm");
    assert!(
      path.is_none(),
      "should return None when no local docs exist"
    );
  }

  #[test]
  fn test_docs_notifier_cargo_returns_none() {
    let dir = TempDir::new().unwrap();
    let notifier = DocsNotifier::new(dir.path());
    let path = notifier.resolve_local_docs_path("serde", "cargo");
    assert!(
      path.is_none(),
      "cargo docs are online — local path should be None"
    );
  }

  #[test]
  fn test_docs_notifier_python_venv_readme() {
    let dir = TempDir::new().unwrap();
    // Simulate a .venv/lib/python3.x/site-packages/requests/README.md
    let pkg_dir = dir
      .path()
      .join(".venv")
      .join("lib")
      .join("python3.11")
      .join("site-packages")
      .join("requests");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("README.md"), "# requests").unwrap();

    let notifier = DocsNotifier::new(dir.path());
    let path = notifier.resolve_local_docs_path("requests", "uv");
    assert!(path.is_some(), "should find README in venv site-packages");
  }

  #[test]
  fn test_docs_notifier_python_hyphen_to_underscore() {
    let dir = TempDir::new().unwrap();
    // Python normalizes hyphens to underscores in package names.
    let pkg_dir = dir
      .path()
      .join(".venv")
      .join("lib")
      .join("python3.11")
      .join("site-packages")
      .join("my_package");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("README.md"), "# my-package").unwrap();

    let notifier = DocsNotifier::new(dir.path());
    let path = notifier.resolve_local_docs_path("my-package", "pip");
    assert!(path.is_some(), "should find README with underscore variant");
  }

  // -------------------------------------------------------------------------
  // DocsNotifier::notify
  // -------------------------------------------------------------------------

  #[test]
  fn test_docs_notifier_notify_npm() {
    let dir = TempDir::new().unwrap();
    let notifier = DocsNotifier::new(dir.path());
    let n = notifier
      .notify("express", "pnpm", "pnpm", Some("4.18.2"))
      .unwrap();
    assert_eq!(n.package, "express");
    assert_eq!(n.manager, "pnpm");
    assert_eq!(n.canonical_manager, "pnpm");
    assert_eq!(n.version.as_deref(), Some("4.18.2"));
    assert_eq!(n.docs_url, "https://www.npmjs.com/package/express");
  }

  #[test]
  fn test_docs_notifier_notify_cargo() {
    let dir = TempDir::new().unwrap();
    let notifier = DocsNotifier::new(dir.path());
    let n = notifier.notify("serde", "cargo", "cargo", None).unwrap();
    assert_eq!(n.docs_url, "https://docs.rs/serde");
    assert!(n.local_docs_path.is_none());
  }

  // -------------------------------------------------------------------------
  // notify_docs convenience function
  // -------------------------------------------------------------------------

  #[test]
  fn test_notify_docs_convenience() {
    let dir = TempDir::new().unwrap();
    let n = notify_docs(dir.path(), "express", "pnpm", "pnpm", Some("4.0.0")).unwrap();
    assert_eq!(n.package, "express");
    assert_eq!(n.version.as_deref(), Some("4.0.0"));
  }

  // -------------------------------------------------------------------------
  // should_notify
  // -------------------------------------------------------------------------

  #[test]
  fn test_should_notify_added() {
    assert!(should_notify(&AddStatus::Added));
  }

  #[test]
  fn test_should_not_notify_dry_run() {
    assert!(!should_notify(&AddStatus::DryRun));
  }

  #[test]
  fn test_should_not_notify_already_installed() {
    assert!(!should_notify(&AddStatus::AlreadyInstalled));
  }

  #[test]
  fn test_should_not_notify_security_abort() {
    assert!(!should_notify(&AddStatus::SecurityAbort));
  }

  #[test]
  fn test_should_not_notify_security_skip() {
    assert!(!should_notify(&AddStatus::SecuritySkip));
  }

  #[test]
  fn test_should_not_notify_failed() {
    assert!(!should_notify(&AddStatus::Failed));
  }

  // -------------------------------------------------------------------------
  // run_docs_notification
  // -------------------------------------------------------------------------

  #[test]
  fn test_run_docs_notification_on_success() {
    let dir = TempDir::new().unwrap();
    let result = run_docs_notification(
      dir.path(),
      "express",
      "pnpm",
      "pnpm",
      Some("4.18.2"),
      &AddStatus::Added,
    )
    .unwrap();
    assert!(result.is_some(), "should notify on successful add");
    let n = result.unwrap();
    assert_eq!(n.package, "express");
  }

  #[test]
  fn test_run_docs_notification_skips_dry_run() {
    let dir = TempDir::new().unwrap();
    let result = run_docs_notification(
      dir.path(),
      "express",
      "pnpm",
      "pnpm",
      None,
      &AddStatus::DryRun,
    )
    .unwrap();
    assert!(result.is_none(), "should not notify on dry-run");
  }

  #[test]
  fn test_run_docs_notification_skips_already_installed() {
    let dir = TempDir::new().unwrap();
    let result = run_docs_notification(
      dir.path(),
      "cargo",
      "pnpm",
      "pnpm",
      None,
      &AddStatus::AlreadyInstalled,
    )
    .unwrap();
    assert!(result.is_none(), "should not notify when already installed");
  }

  #[test]
  fn test_run_docs_notification_skips_security_abort() {
    let dir = TempDir::new().unwrap();
    let result = run_docs_notification(
      dir.path(),
      "bad-pkg",
      "pnpm",
      "pnpm",
      None,
      &AddStatus::SecurityAbort,
    )
    .unwrap();
    assert!(result.is_none(), "should not notify on security abort");
  }

  // -------------------------------------------------------------------------
  // Integration: full notification with local docs
  // -------------------------------------------------------------------------

  #[test]
  fn test_docs_integration_with_local_readme() {
    let dir = TempDir::new().unwrap();
    let pkg_dir = dir.path().join("node_modules").join("express");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("README.md"), "# express").unwrap();

    let result = run_docs_notification(
      dir.path(),
      "express",
      "pnpm",
      "pnpm",
      Some("4.18.2"),
      &AddStatus::Added,
    )
    .unwrap();
    let n = result.expect("should have notification");
    assert!(n.local_docs_path.is_some(), "should find local README");
    assert!(n.local_docs_path.unwrap().contains("README.md"));
    assert_eq!(n.docs_url, "https://www.npmjs.com/package/express");
  }
}
