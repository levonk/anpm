//! AST indexing integration with the CodeGraph/Graphify/GitNexus decision tree.
//!
//! apmw does not implement its own AST indexer. Instead, it delegates to one
//! of three existing tools based on the indexed-ast-tools decision tree:
//!
//! 1. **Project < 20 files?** → Skip indexing (not worth the overhead).
//! 2. **Need multimodal / non-code knowledge?** → Use **Graphify** (MIT,
//!    multimodal).
//! 3. **Work across multiple repos?** → Use **GitNexus** (PolyForm
//!    Noncommercial, multi-repo graph).
//! 4. **Default (zero-maintenance, dynamic dispatch)?** → Use **CodeGraph**
//!    (MIT).
//! 5. **Power-user single-repo structural queries?** → Use **GitNexus** (17
//!    MCP tools).
//!
//! Each tool is invoked as a subprocess. If a tool is not installed, apmw
//! logs a warning and skips indexing — the clone itself is never failed by a
//! missing AST tool.

use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::error::{ApmwError, Result};

/// Projects with fewer than this many files skip indexing entirely.
pub const MIN_FILES_FOR_INDEXING: usize = 20;

/// The AST indexing tool selected by the decision tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AstTool {
  /// CodeGraph — default, zero-maintenance, dynamic dispatch (MIT).
  CodeGraph,
  /// Graphify — multimodal / non-code knowledge linking (MIT).
  Graphify,
  /// GitNexus — multi-repo graph + power-user structural queries (PolyForm
  /// Noncommercial).
  GitNexus,
  /// Indexing skipped (project too small or no tool available).
  Skip,
}

impl AstTool {
  /// Return the CLI binary name used to invoke this tool.
  pub fn binary_name(&self) -> &'static str {
    match self {
      AstTool::CodeGraph => "codegraph",
      AstTool::Graphify => "graphify",
      AstTool::GitNexus => "gitnexus",
      AstTool::Skip => "",
    }
  }

  /// Return a human-readable label for this tool.
  pub fn label(&self) -> &'static str {
    match self {
      AstTool::CodeGraph => "CodeGraph",
      AstTool::Graphify => "Graphify",
      AstTool::GitNexus => "GitNexus",
      AstTool::Skip => "skip",
    }
  }
}

impl std::fmt::Display for AstTool {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.label())
  }
}

/// Options that influence the AST tool decision tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexOptions {
  /// Whether multimodal / non-code knowledge linking is needed.
  pub multimodal: bool,
  /// Whether this is a multi-repo workflow (cloning across repos).
  pub multi_repo: bool,
  /// Whether the user explicitly requested power-user structural queries.
  pub structural_queries: bool,
}

/// Select the AST tool based on the indexed-ast-tools decision tree.
///
/// Decision order:
/// 1. Project < 20 files → `Skip`
/// 2. Multimodal needed → `Graphify`
/// 3. Multi-repo → `GitNexus`
/// 4. Default → `CodeGraph`
/// 5. Power-user structural queries (single repo) → `GitNexus`
pub fn select_ast_tool(file_count: usize, options: &IndexOptions) -> AstTool {
  if file_count < MIN_FILES_FOR_INDEXING {
    info!(
      file_count = file_count,
      threshold = MIN_FILES_FOR_INDEXING,
      "Skipping AST indexing — project below file threshold"
    );
    return AstTool::Skip;
  }
  if options.multimodal {
    info!(
      tool = "Graphify",
      reason = "multimodal",
      "Selected AST tool"
    );
    return AstTool::Graphify;
  }
  if options.multi_repo {
    info!(
      tool = "GitNexus",
      reason = "multi-repo",
      "Selected AST tool"
    );
    return AstTool::GitNexus;
  }
  if options.structural_queries {
    info!(
      tool = "GitNexus",
      reason = "power-user structural queries",
      "Selected AST tool"
    );
    return AstTool::GitNexus;
  }
  info!(
    tool = "CodeGraph",
    reason = "default (zero-maintenance)",
    "Selected AST tool"
  );
  AstTool::CodeGraph
}

/// Count the number of files in a directory tree (non-recursive at the top
/// level plus one level of common source directories). This is a lightweight
/// heuristic to decide whether indexing is worthwhile.
///
/// Hidden files/directories (starting with `.`) and the `.git` directory are
/// excluded.
pub fn count_files(dir: &Path) -> usize {
  if !dir.exists() {
    return 0;
  }
  let mut count = 0usize;
  count_dir_files(dir, &mut count);
  count
}

/// Recursively count files, skipping hidden entries and `.git`.
fn count_dir_files(dir: &Path, count: &mut usize) {
  let entries = match std::fs::read_dir(dir) {
    Ok(e) => e,
    Err(_) => return,
  };
  for entry in entries.flatten() {
    let name = entry.file_name();
    let name_str = name.to_string_lossy();
    // Skip hidden files/dirs and .git
    if name_str.starts_with('.') {
      continue;
    }
    let path = entry.path();
    if path.is_dir() {
      count_dir_files(&path, count);
    } else {
      *count += 1;
    }
  }
}

/// Check whether a given CLI binary is available on PATH.
pub async fn is_tool_available(binary: &str) -> bool {
  if binary.is_empty() {
    return false;
  }
  let result = Command::new("which")
    .arg(binary)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .await;
  match result {
    Ok(status) => status.success(),
    Err(_) => false,
  }
}

/// Create an AST index for the given directory using the selected tool.
///
/// If the tool is not installed, a warning is logged and `Ok(())` is returned
/// — the clone is not failed by a missing AST tool.
pub async fn create_index(dir: &Path, tool: AstTool) -> Result<()> {
  if tool == AstTool::Skip {
    info!(dir = %dir.display(), "AST indexing skipped");
    return Ok(());
  }

  let binary = tool.binary_name();
  debug!(dir = %dir.display(), tool = %tool, binary = binary, "Creating AST index");

  if !is_tool_available(binary).await {
    warn!(
      tool = %tool,
      binary = binary,
      "AST tool not installed — skipping indexing. \
       Install {tool} to enable AST indexing."
    );
    return Ok(());
  }

  let output = Command::new(binary)
    .arg("index")
    .arg(dir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .await
    .map_err(|e| ApmwError::IndexError(format!("failed to invoke {binary}: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    warn!(
      tool = %tool,
      exit_code = output.status.code(),
      stderr = stderr.trim(),
      "AST indexing failed — clone remains usable without index"
    );
    // We do NOT fail the clone — indexing is best-effort.
    return Ok(());
  }

  info!(tool = %tool, dir = %dir.display(), "AST index created");
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  // --- Decision tree tests ---

  #[test]
  fn test_select_skip_below_threshold() {
    let opts = IndexOptions::default();
    assert_eq!(select_ast_tool(0, &opts), AstTool::Skip);
    assert_eq!(select_ast_tool(19, &opts), AstTool::Skip);
  }

  #[test]
  fn test_select_skip_takes_priority_over_multimodal() {
    let opts = IndexOptions {
      multimodal: true,
      ..Default::default()
    };
    assert_eq!(select_ast_tool(5, &opts), AstTool::Skip);
  }

  #[test]
  fn test_select_codegraph_default() {
    let opts = IndexOptions::default();
    assert_eq!(select_ast_tool(20, &opts), AstTool::CodeGraph);
    assert_eq!(select_ast_tool(100, &opts), AstTool::CodeGraph);
  }

  #[test]
  fn test_select_graphify_multimodal() {
    let opts = IndexOptions {
      multimodal: true,
      ..Default::default()
    };
    assert_eq!(select_ast_tool(20, &opts), AstTool::Graphify);
  }

  #[test]
  fn test_select_gitnexus_multi_repo() {
    let opts = IndexOptions {
      multi_repo: true,
      ..Default::default()
    };
    assert_eq!(select_ast_tool(20, &opts), AstTool::GitNexus);
  }

  #[test]
  fn test_select_gitnexus_structural_queries() {
    let opts = IndexOptions {
      structural_queries: true,
      ..Default::default()
    };
    assert_eq!(select_ast_tool(20, &opts), AstTool::GitNexus);
  }

  #[test]
  fn test_multimodal_takes_priority_over_multi_repo() {
    let opts = IndexOptions {
      multimodal: true,
      multi_repo: true,
      ..Default::default()
    };
    assert_eq!(select_ast_tool(20, &opts), AstTool::Graphify);
  }

  #[test]
  fn test_multi_repo_takes_priority_over_structural() {
    let opts = IndexOptions {
      multi_repo: true,
      structural_queries: true,
      ..Default::default()
    };
    assert_eq!(select_ast_tool(20, &opts), AstTool::GitNexus);
  }

  // --- Tool metadata tests ---

  #[test]
  fn test_tool_binary_names() {
    assert_eq!(AstTool::CodeGraph.binary_name(), "codegraph");
    assert_eq!(AstTool::Graphify.binary_name(), "graphify");
    assert_eq!(AstTool::GitNexus.binary_name(), "gitnexus");
    assert_eq!(AstTool::Skip.binary_name(), "");
  }

  #[test]
  fn test_tool_labels() {
    assert_eq!(AstTool::CodeGraph.label(), "CodeGraph");
    assert_eq!(AstTool::Graphify.label(), "Graphify");
    assert_eq!(AstTool::GitNexus.label(), "GitNexus");
    assert_eq!(AstTool::Skip.label(), "skip");
  }

  #[test]
  fn test_tool_display() {
    assert_eq!(AstTool::CodeGraph.to_string(), "CodeGraph");
    assert_eq!(AstTool::Skip.to_string(), "skip");
  }

  // --- File counting tests ---

  #[test]
  fn test_count_files_empty_dir() {
    let dir = TempDir::new().unwrap();
    assert_eq!(count_files(dir.path()), 0);
  }

  #[test]
  fn test_count_files_nonexistent_dir() {
    assert_eq!(count_files(Path::new("/nonexistent/path/abc")), 0);
  }

  #[test]
  fn test_count_files_with_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), "").unwrap();
    fs::write(dir.path().join("b.rs"), "").unwrap();
    fs::write(dir.path().join("c.txt"), "").unwrap();
    assert_eq!(count_files(dir.path()), 3);
  }

  #[test]
  fn test_count_files_skips_hidden() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), "").unwrap();
    fs::write(dir.path().join(".hidden"), "").unwrap();
    assert_eq!(count_files(dir.path()), 1);
  }

  #[test]
  fn test_count_files_recursive() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), "").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src").join("b.rs"), "").unwrap();
    fs::write(dir.path().join("src").join("c.rs"), "").unwrap();
    assert_eq!(count_files(dir.path()), 3);
  }

  #[test]
  fn test_count_files_excludes_git_dir() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), "").unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".git").join("config"), "").unwrap();
    fs::write(dir.path().join(".git").join("HEAD"), "").unwrap();
    assert_eq!(count_files(dir.path()), 1);
  }

  // --- create_index tests ---

  #[tokio::test]
  async fn test_create_index_skip_is_noop() {
    let dir = TempDir::new().unwrap();
    let result = create_index(dir.path(), AstTool::Skip).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_create_index_missing_tool_skips_gracefully() {
    let dir = TempDir::new().unwrap();
    // CodeGraph is almost certainly not installed in the test environment.
    let result = create_index(dir.path(), AstTool::CodeGraph).await;
    assert!(result.is_ok(), "missing AST tool should not fail");
  }

  // --- is_tool_available tests ---

  #[tokio::test]
  async fn test_is_tool_available_empty_binary() {
    assert!(!is_tool_available("").await);
  }

  #[tokio::test]
  async fn test_is_tool_available_known_binary() {
    // `ls` should be available on any Unix system.
    assert!(is_tool_available("ls").await);
  }

  #[tokio::test]
  async fn test_is_tool_available_nonexistent_binary() {
    assert!(!is_tool_available("definitely_not_a_real_binary_xyz123").await);
  }
}
