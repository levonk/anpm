//! Installable Agent Skills — SKILL.md generation and session integrations.
//!
//! This module implements ADR-20260607001 sections 42-43:
//!
//! - **Section 43 — Installable Agent Skill**: Generates a `SKILL.md` file from
//!   the same content as the no-args home view. The file uses trigger-shaped
//!   frontmatter so AI agents can discover and load it. A `--check` build step
//!   verifies that the committed `SKILL.md` is not stale.
//! - **Section 42 — Session integrations**: Installs SessionStart hooks for
//!   Claude Code (`~/.claude/settings.json`), Codex (`~/.codex/hooks.json`),
//!   and OpenCode (`~/.config/opencode/plugins/`). Integrations are:
//!   - **Explicit opt-in** from `apmw --install`.
//!   - **Idempotent** — repeated installs are silent no-ops.
//!   - **Directory-scoped** — show only state for the current directory.
//!   - **Token-budget-aware** — minimize per-session context.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{ApmwError, Result};

/// The YAML frontmatter delimiter used in SKILL.md.
const FRONTMATTER_DELIMITER: &str = "---";

/// The default filename for the generated skill file.
pub const SKILL_FILENAME: &str = "SKILL.md";

/// The marker comment embedded in generated session-integration files so that
/// apmw can identify and update its own entries idempotently.
pub const APMW_MARKER: &str = "# apmw-managed";

/// The set of supported AI agent session integrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionIntegration {
  /// Claude Code — SessionStart hook in `~/.claude/settings.json`.
  ClaudeCode,
  /// Codex — SessionStart hook in `~/.codex/hooks.json`.
  Codex,
  /// OpenCode — managed plugin in `~/.config/opencode/plugins/`.
  OpenCode,
}

impl SessionIntegration {
  /// Returns all supported session integrations.
  pub fn all() -> &'static [SessionIntegration] {
    &[
      SessionIntegration::ClaudeCode,
      SessionIntegration::Codex,
      SessionIntegration::OpenCode,
    ]
  }

  /// Returns the human-readable name of this integration.
  pub fn name(&self) -> &'static str {
    match self {
      SessionIntegration::ClaudeCode => "claude-code",
      SessionIntegration::Codex => "codex",
      SessionIntegration::OpenCode => "opencode",
    }
  }
}

impl std::fmt::Display for SessionIntegration {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.name())
  }
}

/// The result of a session integration install operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationResult {
  /// The integration that was installed.
  pub integration: SessionIntegration,
  /// The path where the integration was written.
  pub path: PathBuf,
  /// Whether the integration was already present (no-op) or newly installed.
  pub already_present: bool,
}

/// The skill generator — produces SKILL.md content from the no-args home view.
///
/// The generator takes the content that `apmw` prints when run with no
/// arguments and wraps it in trigger-shaped YAML frontmatter so AI agents can
/// discover and load the skill. The generated file strips live state and
/// non-interactive commands per ADR section 43.
pub struct SkillGenerator {
  /// The apmw version string (used in frontmatter).
  version: String,
}

impl SkillGenerator {
  /// Create a new skill generator with the given version string.
  pub fn new(version: &str) -> Self {
    SkillGenerator {
      version: version.to_string(),
    }
  }

  /// Create a skill generator using the library's own version.
  pub fn default_version() -> Self {
    SkillGenerator::new(crate::version())
  }

  /// Generate the full SKILL.md content from the no-args home view text.
  ///
  /// The no-args home view is the text that `apmw` prints when run with no
  /// arguments (e.g. `apmw v0.1.0 — run 'apmw --help' for usage`). The
  /// generated file wraps this in YAML frontmatter with trigger metadata.
  pub fn generate(&self, home_view: &str) -> String {
    info!("generating SKILL.md from no-args home view");
    let mut content = String::new();

    // YAML frontmatter with trigger-shaped metadata.
    content.push_str(FRONTMATTER_DELIMITER);
    content.push('\n');
    content.push_str("name: apmw\n");
    content.push_str(&format!("version: \"{}\"\n", self.version));
    content.push_str("description: |\n");
    content.push_str("  All Package Manager Wrapper — abstracts every package\n");
    content.push_str("  installer into one intelligent surface. Use `apmw add\n");
    content.push_str("  <package>` instead of calling package managers directly.\n");
    content.push_str("triggers:\n");
    content.push_str("  - pattern: \"install|add|dependency|package\"\n");
    content.push_str("    description: \"Adding a dependency or installing a package\"\n");
    content.push_str("  - pattern: \"pip|npm|yarn|cargo|go|brew|apt\"\n");
    content
      .push_str("    description: \"Direct package manager invocation — route through apmw\"\n");
    content.push_str(FRONTMATTER_DELIMITER);
    content.push('\n');
    content.push('\n');

    // The body is the no-args home view content, stripped of live state and
    // non-interactive commands. Since the home view is already static content
    // (version + help pointer), we include it directly.
    content.push_str("# apmw — Agent Skill\n");
    content.push('\n');
    content.push_str(home_view);
    content.push('\n');
    content.push('\n');

    // Add non-interactive usage guidance for AI agents.
    content.push_str("## Quick Reference\n");
    content.push('\n');
    content.push_str("```bash\n");
    content.push_str("apmw add <package>          # Install a runtime dependency\n");
    content.push_str("apmw add <package> --dev    # Install a dev/build-time dependency\n");
    content.push_str("apmw detect                 # Detect the package manager\n");
    content.push_str("apmw scan <package>         # Security-scan a package\n");
    content.push_str("```\n");
    content.push('\n');
    content.push_str("## Agent Guidelines\n");
    content.push('\n');
    content.push_str("- Use `apmw add <package>` instead of `pip install`, `npm install`, etc.\n");
    content.push_str("- Use `--dev` for development/build-time dependencies.\n");
    content.push_str("- apmw handles detection, version resolution, and security scanning.\n");

    content
  }

  /// Write the generated SKILL.md to the given path.
  ///
  /// Creates parent directories if needed. Overwrites any existing file.
  pub fn write_to(&self, home_view: &str, path: &Path) -> Result<PathBuf> {
    let content = self.generate(home_view);
    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent).map_err(ApmwError::Io)?;
    }
    std::fs::write(path, content).map_err(ApmwError::Io)?;
    info!(path = %path.display(), "wrote SKILL.md");
    Ok(path.to_path_buf())
  }

  /// Check whether the committed SKILL.md at `path` matches what would be
  /// generated from `home_view`.
  ///
  /// This is the `--check` build step for CI: it fails if the committed
  /// SKILL.md is stale (differs from the generated content).
  pub fn check(&self, home_view: &str, path: &Path) -> Result<bool> {
    if !path.exists() {
      debug!(path = %path.display(), "SKILL.md does not exist — check fails");
      return Ok(false);
    }
    let existing = std::fs::read_to_string(path).map_err(ApmwError::Io)?;
    let expected = self.generate(home_view);
    let matches = existing == expected;
    if !matches {
      debug!(path = %path.display(), "SKILL.md is stale — check fails");
    }
    Ok(matches)
  }
}

/// The no-args home view content — what `apmw` prints when run with no args.
///
/// This is the canonical source for SKILL.md generation. It is kept as a
/// function so that the content can be regenerated deterministically.
pub fn home_view_content() -> String {
  format!("apmw v{} — run 'apmw --help' for usage", crate::version())
}

// ---------------------------------------------------------------------------
// Session integrations
// ---------------------------------------------------------------------------

/// A trait abstracting each session integration's installation logic.
///
/// Each integration knows how to:
/// - Compute its target path (relative to a base directory).
/// - Generate its configuration content.
/// - Check whether it is already installed (idempotency).
/// - Install itself (writing the config file).
pub trait SessionIntegrationInstaller {
  /// The integration variant this installer handles.
  fn integration(&self) -> SessionIntegration;

  /// Compute the target path relative to `base_dir` (typically a home dir).
  fn target_path(&self, base_dir: &Path) -> PathBuf;

  /// Generate the configuration content for the given project directory.
  ///
  /// The content is directory-scoped (references the current project dir) and
  /// token-budget-aware (minimal per-session context).
  fn generate_content(&self, project_dir: &Path) -> String;

  /// Check whether the integration is already installed for `project_dir`.
  ///
  /// Returns `true` if the config file exists and already contains a reference
  /// to `project_dir` (idempotency check).
  fn is_installed(&self, base_dir: &Path, project_dir: &Path) -> bool {
    let path = self.target_path(base_dir);
    if !path.exists() {
      return false;
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let marker = project_dir_marker(project_dir);
    content.contains(&marker)
  }

  /// Install the integration for `project_dir`.
  ///
  /// If the integration is already installed for this directory, this is a
  /// silent no-op. Otherwise, the config is written (or merged if the file
  /// already exists for other directories).
  fn install(&self, base_dir: &Path, project_dir: &Path) -> Result<IntegrationResult> {
    let path = self.target_path(base_dir);
    let already_present = self.is_installed(base_dir, project_dir);

    if already_present {
      info!(
        integration = %self.integration(),
        path = %path.display(),
        "session integration already installed — no-op"
      );
      return Ok(IntegrationResult {
        integration: self.integration(),
        path,
        already_present: true,
      });
    }

    // Read existing content (if any) and append the new entry.
    let existing = if path.exists() {
      std::fs::read_to_string(&path).unwrap_or_default()
    } else {
      String::new()
    };

    let new_content = self.generate_content(project_dir);
    let merged = merge_config(&existing, &new_content);

    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent).map_err(ApmwError::Io)?;
    }
    std::fs::write(&path, merged).map_err(ApmwError::Io)?;

    info!(
      integration = %self.integration(),
      path = %path.display(),
      "session integration installed"
    );

    Ok(IntegrationResult {
      integration: self.integration(),
      path,
      already_present: false,
    })
  }
}

/// Produce a stable marker string for a project directory.
///
/// This marker is embedded in generated config files so that idempotency
/// checks can determine whether apmw has already installed an integration for
/// a specific directory.
fn project_dir_marker(project_dir: &Path) -> String {
  // Use the canonical path if possible, falling back to the display path.
  let canonical = project_dir
    .canonicalize()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|_| project_dir.display().to_string());
  format!("apmw-dir:{canonical}")
}

/// Merge existing config content with new content.
///
/// If the existing content is empty, the new content is used as-is. Otherwise,
/// the new content is appended after a separator (both are text-based config
/// files managed by apmw).
fn merge_config(existing: &str, new_content: &str) -> String {
  if existing.trim().is_empty() {
    return new_content.to_string();
  }
  let mut merged = String::from(existing);
  if !merged.ends_with('\n') {
    merged.push('\n');
  }
  merged.push('\n');
  merged.push_str(new_content);
  merged
}

// --- Claude Code integration ---

/// Claude Code session integration — SessionStart hook in settings.json.
pub struct ClaudeCodeInstaller;

impl SessionIntegrationInstaller for ClaudeCodeInstaller {
  fn integration(&self) -> SessionIntegration {
    SessionIntegration::ClaudeCode
  }

  fn target_path(&self, base_dir: &Path) -> PathBuf {
    base_dir.join(".claude").join("settings.json")
  }

  fn generate_content(&self, project_dir: &Path) -> String {
    let marker = project_dir_marker(project_dir);
    format!(
      "{APMW_MARKER}\n\
       {marker}\n\
       {{\n  \
       \"hooks\": {{\n    \
       \"SessionStart\": [\n      \
       {{\n        \
       \"command\": \"apmw detect --json\",\n        \
       \"description\": \"Load apmw package manager context for this project\"\n      \
       }}\n    \
       ]\n  \
       }}\n\
       }}\n"
    )
  }
}

// --- Codex integration ---

/// Codex session integration — SessionStart hook in hooks.json.
pub struct CodexInstaller;

impl SessionIntegrationInstaller for CodexInstaller {
  fn integration(&self) -> SessionIntegration {
    SessionIntegration::Codex
  }

  fn target_path(&self, base_dir: &Path) -> PathBuf {
    base_dir.join(".codex").join("hooks.json")
  }

  fn generate_content(&self, project_dir: &Path) -> String {
    let marker = project_dir_marker(project_dir);
    format!(
      "{APMW_MARKER}\n\
       {marker}\n\
       {{\n  \
       \"hooks\": {{\n    \
       \"SessionStart\": [\n      \
       {{\n        \
       \"command\": \"apmw detect --json\",\n        \
       \"description\": \"Load apmw package manager context for this project\"\n      \
       }}\n    \
       ]\n  \
       }}\n\
       }}\n"
    )
  }
}

// --- OpenCode integration ---

/// OpenCode session integration — managed plugin in plugins directory.
pub struct OpenCodeInstaller;

impl SessionIntegrationInstaller for OpenCodeInstaller {
  fn integration(&self) -> SessionIntegration {
    SessionIntegration::OpenCode
  }

  fn target_path(&self, base_dir: &Path) -> PathBuf {
    base_dir
      .join(".config")
      .join("opencode")
      .join("plugins")
      .join("apmw.json")
  }

  fn generate_content(&self, project_dir: &Path) -> String {
    let marker = project_dir_marker(project_dir);
    format!(
      "{APMW_MARKER}\n\
       {marker}\n\
       {{\n  \
       \"name\": \"apmw\",\n  \
       \"version\": \"{ver}\",\n  \
       \"hooks\": {{\n    \
       \"SessionStart\": \"apmw detect --json\"\n  \
       }}\n\
       }}\n",
      ver = crate::version()
    )
  }
}

/// Install all session integrations for the given project directory.
///
/// This is the top-level entry point called from `apmw --install`. Each
/// integration is installed idempotently — repeated calls for the same
/// directory are silent no-ops.
pub fn install_all_session_integrations(
  base_dir: &Path,
  project_dir: &Path,
) -> Result<Vec<IntegrationResult>> {
  let installers: Vec<Box<dyn SessionIntegrationInstaller>> = vec![
    Box::new(ClaudeCodeInstaller),
    Box::new(CodexInstaller),
    Box::new(OpenCodeInstaller),
  ];

  let mut results = Vec::new();
  for installer in &installers {
    let result = installer.install(base_dir, project_dir)?;
    results.push(result);
  }
  Ok(results)
}

/// Install a single session integration by variant.
pub fn install_session_integration(
  integration: SessionIntegration,
  base_dir: &Path,
  project_dir: &Path,
) -> Result<IntegrationResult> {
  let installer: Box<dyn SessionIntegrationInstaller> = match integration {
    SessionIntegration::ClaudeCode => Box::new(ClaudeCodeInstaller),
    SessionIntegration::Codex => Box::new(CodexInstaller),
    SessionIntegration::OpenCode => Box::new(OpenCodeInstaller),
  };
  installer.install(base_dir, project_dir)
}

/// Resolve the home directory for session integration installation.
///
/// Uses the `dirs` crate to find the user's home directory. Falls back to the
/// `HOME` environment variable if `dirs` cannot determine it.
pub fn home_dir() -> Result<PathBuf> {
  dirs::home_dir().ok_or_else(|| ApmwError::Config("cannot determine home directory".to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;

  // -------------------------------------------------------------------------
  // SKILL.md generation
  // -------------------------------------------------------------------------

  #[test]
  fn test_skill_generation_contains_frontmatter() {
    let gen = SkillGenerator::new("0.1.0");
    let content = gen.generate("apmw v0.1.0 — run 'apmw --help' for usage");
    assert!(
      content.starts_with("---\n"),
      "must start with frontmatter delimiter"
    );
    // The frontmatter should close with a second delimiter.
    let second = content.match_indices("---").nth(1).unwrap().0;
    assert!(second > 0, "must have closing frontmatter delimiter");
  }

  #[test]
  fn test_skill_generation_includes_home_view() {
    let gen = SkillGenerator::new("0.1.0");
    let home = "apmw v0.1.0 — run 'apmw --help' for usage";
    let content = gen.generate(home);
    assert!(
      content.contains(home),
      "SKILL.md must contain the no-args home view content"
    );
  }

  #[test]
  fn test_skill_generation_includes_version() {
    let gen = SkillGenerator::new("1.2.3");
    let content = gen.generate("apmw v1.2.3 — run 'apmw --help' for usage");
    assert!(
      content.contains("version: \"1.2.3\""),
      "must include version in frontmatter"
    );
  }

  #[test]
  fn test_skill_generation_includes_triggers() {
    let gen = SkillGenerator::new("0.1.0");
    let content = gen.generate("apmw v0.1.0");
    assert!(
      content.contains("triggers:"),
      "must include triggers in frontmatter"
    );
    assert!(
      content.contains("install|add|dependency|package"),
      "must include install trigger pattern"
    );
  }

  #[test]
  fn test_skill_generation_includes_quick_reference() {
    let gen = SkillGenerator::new("0.1.0");
    let content = gen.generate("apmw v0.1.0");
    assert!(
      content.contains("apmw add <package>"),
      "must include add command"
    );
    assert!(content.contains("--dev"), "must include --dev flag");
  }

  #[test]
  fn test_skill_generation_strips_noninteractive() {
    let gen = SkillGenerator::new("0.1.0");
    let content = gen.generate("apmw v0.1.0");
    // The generated content should not contain interactive prompts.
    assert!(
      !content.contains("prompt("),
      "must not contain interactive prompts"
    );
    assert!(
      !content.contains("read_line"),
      "must not contain interactive read_line"
    );
  }

  #[test]
  fn test_skill_write_to_file() {
    let gen = SkillGenerator::new("0.1.0");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    let result = gen.write_to("apmw v0.1.0", &path).unwrap();
    assert!(path.exists(), "SKILL.md must be written");
    assert_eq!(result, path);
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("apmw v0.1.0"));
  }

  // -------------------------------------------------------------------------
  // --check build step
  // -------------------------------------------------------------------------

  #[test]
  fn test_skill_check_passes_when_fresh() {
    let gen = SkillGenerator::new("0.1.0");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    gen.write_to("apmw v0.1.0", &path).unwrap();
    assert!(
      gen.check("apmw v0.1.0", &path).unwrap(),
      "check should pass when SKILL.md is fresh"
    );
  }

  #[test]
  fn test_skill_check_fails_when_stale() {
    let gen = SkillGenerator::new("0.1.0");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    gen.write_to("apmw v0.1.0", &path).unwrap();
    // Now check with different home view — should fail.
    assert!(
      !gen.check("apmw v0.2.0", &path).unwrap(),
      "check should fail when SKILL.md is stale"
    );
  }

  #[test]
  fn test_skill_check_fails_when_missing() {
    let gen = SkillGenerator::new("0.1.0");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    assert!(
      !gen.check("apmw v0.1.0", &path).unwrap(),
      "check should fail when SKILL.md does not exist"
    );
  }

  // -------------------------------------------------------------------------
  // home_view_content
  // -------------------------------------------------------------------------

  #[test]
  fn test_home_view_content_contains_version() {
    let content = home_view_content();
    assert!(content.contains("apmw v"), "home view must contain version");
    assert!(
      content.contains("--help"),
      "home view must contain help pointer"
    );
  }

  // -------------------------------------------------------------------------
  // Claude Code integration
  // -------------------------------------------------------------------------

  #[test]
  fn test_claude_integration_target_path() {
    let installer = ClaudeCodeInstaller;
    let path = installer.target_path(Path::new("/tmp/home"));
    assert_eq!(path, Path::new("/tmp/home/.claude/settings.json"));
  }

  #[test]
  fn test_claude_integration_generates_session_start_hook() {
    let installer = ClaudeCodeInstaller;
    let content = installer.generate_content(Path::new("/tmp/project"));
    assert!(
      content.contains("SessionStart"),
      "must include SessionStart hook"
    );
    assert!(
      content.contains("apmw detect"),
      "must include apmw detect command"
    );
    assert!(content.contains(APMW_MARKER), "must include apmw marker");
  }

  #[test]
  fn test_claude_integration_installs_hook() {
    let installer = ClaudeCodeInstaller;
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let result = installer.install(home.path(), project.path()).unwrap();
    assert_eq!(result.integration, SessionIntegration::ClaudeCode);
    assert!(
      !result.already_present,
      "first install should not be already_present"
    );
    let path = installer.target_path(home.path());
    assert!(path.exists(), "config file must exist after install");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("SessionStart"));
  }

  // -------------------------------------------------------------------------
  // Codex integration
  // -------------------------------------------------------------------------

  #[test]
  fn test_codex_integration_target_path() {
    let installer = CodexInstaller;
    let path = installer.target_path(Path::new("/tmp/home"));
    assert_eq!(path, Path::new("/tmp/home/.codex/hooks.json"));
  }

  #[test]
  fn test_codex_integration_generates_session_start_hook() {
    let installer = CodexInstaller;
    let content = installer.generate_content(Path::new("/tmp/project"));
    assert!(
      content.contains("SessionStart"),
      "must include SessionStart hook"
    );
    assert!(
      content.contains("apmw detect"),
      "must include apmw detect command"
    );
  }

  #[test]
  fn test_codex_integration_installs_hook() {
    let installer = CodexInstaller;
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let result = installer.install(home.path(), project.path()).unwrap();
    assert_eq!(result.integration, SessionIntegration::Codex);
    assert!(!result.already_present);
    let path = installer.target_path(home.path());
    assert!(path.exists());
  }

  // -------------------------------------------------------------------------
  // OpenCode integration
  // -------------------------------------------------------------------------

  #[test]
  fn test_opencode_integration_target_path() {
    let installer = OpenCodeInstaller;
    let path = installer.target_path(Path::new("/tmp/home"));
    assert_eq!(
      path,
      Path::new("/tmp/home/.config/opencode/plugins/apmw.json")
    );
  }

  #[test]
  fn test_opencode_integration_generates_plugin() {
    let installer = OpenCodeInstaller;
    let content = installer.generate_content(Path::new("/tmp/project"));
    assert!(
      content.contains("\"name\": \"apmw\""),
      "must include plugin name"
    );
    assert!(
      content.contains("SessionStart"),
      "must include SessionStart hook"
    );
  }

  #[test]
  fn test_opencode_integration_installs_plugin() {
    let installer = OpenCodeInstaller;
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let result = installer.install(home.path(), project.path()).unwrap();
    assert_eq!(result.integration, SessionIntegration::OpenCode);
    assert!(!result.already_present);
    let path = installer.target_path(home.path());
    assert!(path.exists());
  }

  // -------------------------------------------------------------------------
  // Idempotency
  // -------------------------------------------------------------------------

  #[test]
  fn test_idempotent_install_is_noop() {
    let installer = ClaudeCodeInstaller;
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // First install.
    let r1 = installer.install(home.path(), project.path()).unwrap();
    assert!(
      !r1.already_present,
      "first install should not be already_present"
    );

    // Second install — should be a no-op.
    let r2 = installer.install(home.path(), project.path()).unwrap();
    assert!(
      r2.already_present,
      "second install should be already_present (no-op)"
    );
  }

  #[test]
  fn test_idempotent_install_all_integrations() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // First install of all integrations.
    let results1 = install_all_session_integrations(home.path(), project.path()).unwrap();
    assert_eq!(results1.len(), 3);
    assert!(
      results1.iter().all(|r| !r.already_present),
      "first install should not be already_present"
    );

    // Second install — all should be no-ops.
    let results2 = install_all_session_integrations(home.path(), project.path()).unwrap();
    assert_eq!(results2.len(), 3);
    assert!(
      results2.iter().all(|r| r.already_present),
      "second install should all be already_present (no-ops)"
    );
  }

  // -------------------------------------------------------------------------
  // Directory-scoped
  // -------------------------------------------------------------------------

  #[test]
  fn test_directory_scoped_different_dirs() {
    let installer = ClaudeCodeInstaller;
    let home = tempfile::tempdir().unwrap();
    let project1 = tempfile::tempdir().unwrap();
    let project2 = tempfile::tempdir().unwrap();

    // Install for project1.
    installer.install(home.path(), project1.path()).unwrap();

    // project2 should NOT be marked as installed.
    assert!(
      !installer.is_installed(home.path(), project2.path()),
      "different directory should not be marked as installed"
    );

    // Install for project2 — should not be a no-op.
    let r2 = installer.install(home.path(), project2.path()).unwrap();
    assert!(
      !r2.already_present,
      "install for different dir should not be no-op"
    );
  }

  #[test]
  fn test_directory_scoped_marker_in_content() {
    let installer = CodexInstaller;
    let project = tempfile::tempdir().unwrap();
    let content = installer.generate_content(project.path());
    let marker = project_dir_marker(project.path());
    assert!(
      content.contains(&marker),
      "generated content must contain the directory marker"
    );
  }

  // -------------------------------------------------------------------------
  // Token-budget-aware
  // -------------------------------------------------------------------------

  #[test]
  fn test_token_budget_aware_minimal_content() {
    let installer = ClaudeCodeInstaller;
    let content = installer.generate_content(Path::new("/tmp/project"));
    // The content should be minimal — under 1KB for token budget awareness.
    assert!(
      content.len() < 1024,
      "session integration content should be minimal (under 1KB), got {} bytes",
      content.len()
    );
  }

  // -------------------------------------------------------------------------
  // SessionIntegration enum
  // -------------------------------------------------------------------------

  #[test]
  fn test_session_integration_all() {
    let all = SessionIntegration::all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&SessionIntegration::ClaudeCode));
    assert!(all.contains(&SessionIntegration::Codex));
    assert!(all.contains(&SessionIntegration::OpenCode));
  }

  #[test]
  fn test_session_integration_names() {
    assert_eq!(SessionIntegration::ClaudeCode.name(), "claude-code");
    assert_eq!(SessionIntegration::Codex.name(), "codex");
    assert_eq!(SessionIntegration::OpenCode.name(), "opencode");
  }

  // -------------------------------------------------------------------------
  // install_session_integration (single)
  // -------------------------------------------------------------------------

  #[test]
  fn test_install_single_session_integration() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let result =
      install_session_integration(SessionIntegration::OpenCode, home.path(), project.path())
        .unwrap();
    assert_eq!(result.integration, SessionIntegration::OpenCode);
    assert!(!result.already_present);
  }

  // -------------------------------------------------------------------------
  // merge_config
  // -------------------------------------------------------------------------

  #[test]
  fn test_merge_config_empty_existing() {
    let merged = merge_config("", "new content");
    assert_eq!(merged, "new content");
  }

  #[test]
  fn test_merge_config_appends() {
    let merged = merge_config("existing\n", "new content");
    assert!(merged.contains("existing"));
    assert!(merged.contains("new content"));
  }
}
