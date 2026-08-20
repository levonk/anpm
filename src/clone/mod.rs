//! Historyless clone engine for apmw.
//!
//! Clones a repository with `git clone --depth 1 --single-branch --no-tags`
//! (a shallow, single-branch, tag-less clone), writes a local `.gitignore`
//! that excludes AST index files and agent-specific files, and then invokes
//! the appropriate AST indexing tool (CodeGraph / Graphify / GitNexus) based
//! on the indexed-ast-tools decision tree.
//!
//! ## Workflow
//!
//! 1. `CloneEngine::clone_repo` — shallow clone via `git` subprocess.
//! 2. `gitignore::write_gitignore` — write local `.gitignore`.
//! 3. `ast_index::select_ast_tool` + `ast_index::create_index` — index.
//!
//! The clone and index steps can run as background jobs via the daemon's
//! [`JobManager`](crate::daemon::JobManager).

pub mod ast_index;
pub mod gitignore;

pub use ast_index::{
  count_files, create_index, is_tool_available, select_ast_tool, AstTool, IndexOptions,
};
pub use gitignore::{
  generate_gitignore_contents, write_gitignore, GITIGNORE_HEADER, IGNORED_PATTERNS,
};

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use crate::error::{ApmwError, Result};

/// The `git clone` flags used for a historyless clone.
pub const CLONE_FLAGS: &[&str] = &["--depth", "1", "--single-branch", "--no-tags"];

/// The result of a historyless clone operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneResult {
  /// The repository URL or package name that was cloned.
  pub repo: String,
  /// The local path where the repo was cloned to.
  pub path: String,
  /// The AST tool selected for indexing (or `Skip`).
  pub ast_tool: AstTool,
  /// Whether the AST index was created successfully.
  pub indexed: bool,
  /// The number of files detected in the cloned repo.
  pub file_count: usize,
}

/// The historyless clone engine.
///
/// Orchestrates the clone → gitignore → AST index pipeline. The engine is
/// cheap to construct and stateless; all state lives in the individual
/// operation results.
#[derive(Debug, Clone, Default)]
pub struct CloneEngine {
  /// Options that influence AST tool selection.
  pub index_options: IndexOptions,
}

impl CloneEngine {
  /// Create a new `CloneEngine` with default options.
  pub fn new() -> Self {
    Self::default()
  }

  /// Create a `CloneEngine` with custom index options.
  pub fn with_index_options(index_options: IndexOptions) -> Self {
    CloneEngine { index_options }
  }

  /// Perform a historyless clone of `repo` into `dest_dir`.
  ///
  /// This runs the full pipeline:
  /// 1. `git clone --depth 1 --single-branch --no-tags`
  /// 2. Write local `.gitignore`
  /// 3. Select and invoke AST indexing tool
  ///
  /// The destination directory is `dest_dir/repo_name` (the repo name is
  /// derived from the URL). If `dest_dir` does not exist, it is created.
  pub async fn clone_repo(&self, repo: &str, dest_dir: &Path) -> Result<CloneResult> {
    info!(repo = repo, dest = %dest_dir.display(), "Starting historyless clone");

    // Resolve the target directory.
    let repo_name = derive_repo_name(repo);
    let target = dest_dir.join(&repo_name);

    // Ensure the parent directory exists.
    if let Some(parent) = target.parent() {
      std::fs::create_dir_all(parent).map_err(ApmwError::from)?;
    }

    // Step 1: Shallow clone.
    self.run_git_clone(repo, &target).await?;

    // Step 2: Write local .gitignore.
    if let Err(e) = write_gitignore(&target) {
      warn!(error = %e, "Failed to write .gitignore — clone still usable");
    }

    // Step 3: AST indexing.
    let file_count = count_files(&target);
    let ast_tool = select_ast_tool(file_count, &self.index_options);
    let mut indexed = false;
    if ast_tool != AstTool::Skip {
      match create_index(&target, ast_tool).await {
        Ok(()) => {
          // Check if the tool was actually available (best-effort).
          let available = is_tool_available(ast_tool.binary_name()).await;
          indexed = available;
        }
        Err(e) => {
          warn!(error = %e, tool = %ast_tool, "AST indexing failed — clone remains usable");
        }
      }
    }

    info!(
      repo = repo,
      path = %target.display(),
      ast_tool = %ast_tool,
      file_count = file_count,
      "Historyless clone complete"
    );

    Ok(CloneResult {
      repo: repo.to_string(),
      path: target.to_string_lossy().into_owned(),
      ast_tool,
      indexed,
      file_count,
    })
  }

  /// Run `git clone --depth 1 --single-branch --no-tags` as a subprocess.
  async fn run_git_clone(&self, repo: &str, target: &Path) -> Result<()> {
    debug!(
      repo = repo,
      target = %target.display(),
      flags = CLONE_FLAGS.join(" "),
      "Running git clone"
    );

    let output = Command::new("git")
      .arg("clone")
      .args(CLONE_FLAGS)
      .arg(repo)
      .arg(target)
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .output()
      .await
      .map_err(|e| ApmwError::CloneFailed(format!("failed to invoke git: {e}")))?;

    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      error!(
        repo = repo,
        exit_code = output.status.code(),
        stderr = stderr.trim(),
        "git clone failed"
      );
      return Err(ApmwError::CloneFailed(format!(
        "git clone failed (exit {:?}): {}",
        output.status.code(),
        stderr.trim()
      )));
    }

    info!(repo = repo, target = %target.display(), "git clone succeeded");
    Ok(())
  }
}

/// Derive a local directory name from a repository URL or package name.
///
/// For URLs like `https://github.com/owner/repo.git`, the name is `repo`.
/// For bare names like `repo`, the name is used as-is.
pub fn derive_repo_name(repo: &str) -> String {
  // Try to parse as a URL and take the last path segment.
  let trimmed = repo.trim().trim_end_matches('/');
  if let Some(after_scheme) = trimmed.split("://").nth(1) {
    let last = after_scheme.rsplit('/').next().unwrap_or(after_scheme);
    return last.trim_end_matches(".git").to_string();
  }
  // For `git@host:owner/repo` style SSH URLs.
  if let Some(after_colon) = trimmed.split(':').nth(1) {
    if after_colon.contains('/') {
      let last = after_colon.rsplit('/').next().unwrap_or(after_colon);
      return last.trim_end_matches(".git").to_string();
    }
  }
  // Bare name — use the last segment if it contains a slash.
  if let Some(last) = trimmed.rsplit('/').next() {
    return last.trim_end_matches(".git").to_string();
  }
  trimmed.to_string()
}

/// Resolve the default clone destination directory.
///
/// Uses `XDG_CACHE_HOME` if set, otherwise `$HOME/.cache`, falling back to
/// the current directory.
pub fn default_clone_dest() -> PathBuf {
  if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
    if !xdg.is_empty() {
      return PathBuf::from(xdg).join("apmw").join("clones");
    }
  }
  if let Some(home) = std::env::var_os("HOME") {
    if !home.is_empty() {
      return PathBuf::from(home)
        .join(".cache")
        .join("apmw")
        .join("clones");
    }
  }
  PathBuf::from(".apmw").join("clones")
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  // --- derive_repo_name tests ---

  #[test]
  fn test_derive_repo_name_https_url() {
    assert_eq!(
      derive_repo_name("https://github.com/owner/repo.git"),
      "repo"
    );
  }

  #[test]
  fn test_derive_repo_name_https_url_no_git_suffix() {
    assert_eq!(derive_repo_name("https://github.com/owner/repo"), "repo");
  }

  #[test]
  fn test_derive_repo_name_ssh_url() {
    assert_eq!(derive_repo_name("git@github.com:owner/repo.git"), "repo");
  }

  #[test]
  fn test_derive_repo_name_bare_name() {
    assert_eq!(derive_repo_name("my-repo"), "my-repo");
  }

  #[test]
  fn test_derive_repo_name_trailing_slash() {
    assert_eq!(derive_repo_name("https://github.com/owner/repo/"), "repo");
  }

  #[test]
  fn test_derive_repo_name_nested_path() {
    assert_eq!(
      derive_repo_name("https://gitlab.com/group/subgroup/project.git"),
      "project"
    );
  }

  // --- CloneEngine tests ---

  #[test]
  fn test_clone_engine_default() {
    let engine = CloneEngine::new();
    assert_eq!(engine.index_options, IndexOptions::default());
  }

  #[test]
  fn test_clone_engine_with_index_options() {
    let opts = IndexOptions {
      multimodal: true,
      ..Default::default()
    };
    let engine = CloneEngine::with_index_options(opts);
    assert!(engine.index_options.multimodal);
  }

  #[tokio::test]
  async fn test_clone_repo_invalid_url_fails() {
    let dir = TempDir::new().unwrap();
    let engine = CloneEngine::new();
    let result = engine
      .clone_repo(
        "https://invalid-host-that-does-not-exist.invalid/repo",
        dir.path(),
      )
      .await;
    assert!(result.is_err(), "clone of invalid URL should fail");
  }

  #[tokio::test]
  async fn test_clone_repo_local_repo() {
    // Create a bare repo to clone from.
    let source = TempDir::new().unwrap();
    let dest_parent = TempDir::new().unwrap();

    // Initialize a git repo with a commit.
    std::process::Command::new("git")
      .args(["init"])
      .current_dir(source.path())
      .output()
      .expect("git init");
    std::process::Command::new("git")
      .args(["config", "user.email", "test@test.com"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    std::process::Command::new("git")
      .args(["config", "user.name", "Test"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    fs::write(source.path().join("README.md"), "# test").unwrap();
    std::process::Command::new("git")
      .args(["add", "."])
      .current_dir(source.path())
      .output()
      .expect("git add");
    std::process::Command::new("git")
      .args(["commit", "-m", "init"])
      .current_dir(source.path())
      .output()
      .expect("git commit");

    let engine = CloneEngine::new();
    let source_url = source.path().to_string_lossy().to_string();
    let result = engine
      .clone_repo(&source_url, dest_parent.path())
      .await
      .expect("clone should succeed");

    assert_eq!(result.repo, source_url);
    // The cloned dir should exist.
    let cloned_path = PathBuf::from(&result.path);
    assert!(
      cloned_path.exists(),
      "cloned path should exist: {:?}",
      cloned_path
    );
    // The .gitignore should be written.
    assert!(cloned_path.join(".gitignore").exists());
    // The README should be present.
    assert!(cloned_path.join("README.md").exists());
  }

  #[tokio::test]
  async fn test_clone_repo_writes_gitignore_with_correct_patterns() {
    let source = TempDir::new().unwrap();
    let dest_parent = TempDir::new().unwrap();

    // Create a minimal git repo.
    std::process::Command::new("git")
      .args(["init"])
      .current_dir(source.path())
      .output()
      .expect("git init");
    std::process::Command::new("git")
      .args(["config", "user.email", "t@t.com"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    std::process::Command::new("git")
      .args(["config", "user.name", "T"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    fs::write(source.path().join("file.txt"), "content").unwrap();
    std::process::Command::new("git")
      .args(["add", "."])
      .current_dir(source.path())
      .output()
      .expect("git add");
    std::process::Command::new("git")
      .args(["commit", "-m", "x"])
      .current_dir(source.path())
      .output()
      .expect("git commit");

    let engine = CloneEngine::new();
    let source_url = source.path().to_string_lossy().to_string();
    let result = engine
      .clone_repo(&source_url, dest_parent.path())
      .await
      .expect("clone should succeed");

    let gitignore_path = PathBuf::from(&result.path).join(".gitignore");
    let contents = fs::read_to_string(&gitignore_path).unwrap();
    assert!(contents.contains("devbox.json"));
    assert!(contents.contains("AGENTS.md"));
    assert!(contents.contains("*.codegraph"));
    assert!(contents.contains("*.graphify"));
    assert!(contents.contains("*.gitnexus"));
  }

  #[tokio::test]
  async fn test_clone_repo_small_project_skips_indexing() {
    let source = TempDir::new().unwrap();
    let dest_parent = TempDir::new().unwrap();

    std::process::Command::new("git")
      .args(["init"])
      .current_dir(source.path())
      .output()
      .expect("git init");
    std::process::Command::new("git")
      .args(["config", "user.email", "t@t.com"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    std::process::Command::new("git")
      .args(["config", "user.name", "T"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    // Only 1 file — below the 20-file threshold.
    fs::write(source.path().join("only.txt"), "x").unwrap();
    std::process::Command::new("git")
      .args(["add", "."])
      .current_dir(source.path())
      .output()
      .expect("git add");
    std::process::Command::new("git")
      .args(["commit", "-m", "x"])
      .current_dir(source.path())
      .output()
      .expect("git commit");

    let engine = CloneEngine::new();
    let source_url = source.path().to_string_lossy().to_string();
    let result = engine
      .clone_repo(&source_url, dest_parent.path())
      .await
      .expect("clone should succeed");

    assert_eq!(result.ast_tool, AstTool::Skip);
    assert!(!result.indexed);
  }

  #[tokio::test]
  async fn test_clone_repo_uses_shallow_clone() {
    let source = TempDir::new().unwrap();
    let dest_parent = TempDir::new().unwrap();

    std::process::Command::new("git")
      .args(["init"])
      .current_dir(source.path())
      .output()
      .expect("git init");
    std::process::Command::new("git")
      .args(["config", "user.email", "t@t.com"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    std::process::Command::new("git")
      .args(["config", "user.name", "T"])
      .current_dir(source.path())
      .output()
      .expect("git config");
    fs::write(source.path().join("f.txt"), "x").unwrap();
    std::process::Command::new("git")
      .args(["add", "."])
      .current_dir(source.path())
      .output()
      .expect("git add");
    std::process::Command::new("git")
      .args(["commit", "-m", "c1"])
      .current_dir(source.path())
      .output()
      .expect("git commit");
    // Second commit to ensure depth-1 only fetches one.
    fs::write(source.path().join("f2.txt"), "y").unwrap();
    std::process::Command::new("git")
      .args(["add", "."])
      .current_dir(source.path())
      .output()
      .expect("git add");
    std::process::Command::new("git")
      .args(["commit", "-m", "c2"])
      .current_dir(source.path())
      .output()
      .expect("git commit");

    let engine = CloneEngine::new();
    // Use file:// protocol to force a real (non-hardlink) clone so that
    // --depth 1 actually limits the commit history.
    let source_url = format!("file://{}", source.path().display());
    let result = engine
      .clone_repo(&source_url, dest_parent.path())
      .await
      .expect("clone should succeed");

    // Verify the clone is shallow (depth 1).
    let cloned = PathBuf::from(&result.path);
    let log_output = std::process::Command::new("git")
      .args(["log", "--oneline"])
      .current_dir(&cloned)
      .output()
      .expect("git log");
    let log_str = String::from_utf8_lossy(&log_output.stdout);
    let commit_count = log_str.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(
      commit_count, 1,
      "shallow clone should have exactly 1 commit, got {commit_count}: {log_str}"
    );
  }

  // --- default_clone_dest tests ---

  #[test]
  fn test_default_clone_dest_fallback() {
    let path = default_clone_dest();
    assert!(path.to_string_lossy().contains("clones"));
  }
}
