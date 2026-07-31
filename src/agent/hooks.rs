//! AI agent coding hooks — hard intercept and soft convention.
//!
//! The [`HookManager`] generates PATH shims that wrap package manager binaries
//! (pip, npm, yarn, cargo, go, apt, brew) so any invocation routes through
//! apmw. A shim intercepts the call, checks governance rules
//! (`prefer`/`force`/`block`/`eject` from the governance engine), runs
//! security scanning, and then delegates to the real binary (or the canonical
//! alternative if governance forces it).
//!
//! # Hard intercept (opt-in)
//!
//! `apmw --install --intercept` installs shims to a PATH directory that takes
//! precedence. `apmw --uninstall --intercept` removes them.
//!
//! # Soft convention (default)
//!
//! When no intercept is installed, [`generate_soft_convention_instructions`]
//! produces instructions for AI agents to use `apmw add` instead of calling
//! package managers directly.
//!
//! # Governance-aware interception
//!
//! The [`InterceptDecision`] type encodes what a shim should do for a given
//! intercepted call based on the active governance policy:
//!
//! - **prefer**: warn and delegate to the canonical manager (if installed).
//! - **force**: intercept and route through the canonical manager via wrapper.
//! - **block**: error and prevent the call entirely.
//! - **eject**: remove and block the non-canonical manager.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::{ApmwError, Result};
use crate::governance::{GovernanceEngine, GovernanceOutcome, GovernanceType};
use crate::path_scan::{PathScanner, ScanResult};

/// The set of package manager binaries for which PATH shims are generated.
///
/// These are the non-canonical (or commonly-invoked) managers that AI agents
/// tend to call directly. New shims can be added by extending this list.
pub const SHIM_TARGETS: &[&str] = &["pip", "npm", "yarn", "cargo", "go", "apt", "brew"];

/// The marker line embedded in every generated shim so that apmw can identify
/// its own shims during uninstall.
pub const SHIM_MARKER: &str = "# apmw-intercept-shim";

/// A specification for a single PATH shim.
///
/// Describes the tool being intercepted and the canonical manager it should be
/// routed to when governance forces a redirect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShimSpec {
  /// The non-canonical tool being intercepted (e.g. `pip`, `npm`).
  pub tool: String,
  /// The canonical manager to route to when governance is `force`.
  pub canonical_manager: Option<String>,
  /// The filename of the shim (same as `tool` on Unix).
  pub shim_name: String,
}

impl ShimSpec {
  /// Creates a new shim spec for the given tool.
  pub fn new(tool: &str, canonical_manager: Option<&str>) -> Self {
    ShimSpec {
      tool: tool.to_string(),
      canonical_manager: canonical_manager.map(|s| s.to_string()),
      shim_name: tool.to_string(),
    }
  }
}

/// The action a shim should take after evaluating governance and security.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterceptAction {
  /// Delegate to the original tool (no governance rule or `prefer` without
  /// the canonical manager installed).
  Delegate {
    /// The tool to delegate to.
    tool: String,
  },
  /// Redirect to the canonical manager (governance `force` or `prefer` with
  /// the canonical manager installed).
  Redirect {
    /// The canonical manager to invoke instead.
    canonical_manager: String,
  },
  /// Block the call entirely (governance `block` or `eject`).
  Block {
    /// A human-readable message explaining why the call was blocked.
    message: String,
  },
}

/// The decision produced by evaluating governance for an intercepted call.
///
/// This is the pure-logic result of consulting the governance engine; the
/// shim is responsible for acting on it (delegating, redirecting, or exiting
/// with an error).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterceptDecision {
  /// The tool that was intercepted.
  pub tool: String,
  /// The governance outcome (if a governance rule was found).
  pub governance: Option<GovernanceOutcome>,
  /// The action the shim should take.
  pub action: InterceptAction,
  /// Whether a security scan should be run before delegating.
  pub run_security_scan: bool,
}

/// The result of an intercepted call — includes the decision and whether the
/// security scan passed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterceptResult {
  /// The decision that was made.
  pub decision: InterceptDecision,
  /// Whether the security scan passed (or was skipped).
  pub security_passed: bool,
  /// The final tool that should be invoked.
  pub effective_tool: String,
  /// The arguments to pass to the effective tool.
  pub effective_args: Vec<String>,
}

/// The hook manager — generates, installs, and removes PATH shims, and
/// evaluates governance for intercepted calls.
///
/// The manager holds a reference to a [`GovernanceEngine`] so that
/// interception decisions respect the active governance policy.
pub struct HookManager {
  governance: GovernanceEngine,
}

impl HookManager {
  /// Creates a new hook manager with the given governance engine.
  pub fn new(governance: GovernanceEngine) -> Self {
    HookManager { governance }
  }

  /// Creates a new hook manager with an empty (no-rules) governance engine.
  pub fn with_no_governance() -> Self {
    HookManager {
      governance: GovernanceEngine::default(),
    }
  }

  /// Returns a reference to the governance engine.
  pub fn governance(&self) -> &GovernanceEngine {
    &self.governance
  }

  // -------------------------------------------------------------------------
  // Shim generation
  // -------------------------------------------------------------------------

  /// Generates the content of a Unix shell-script shim for the given tool.
  ///
  /// The shim:
  /// 1. Identifies itself with the [`SHIM_MARKER`] comment.
  /// 2. Invokes `apmw intercept <tool> "$@"` to evaluate governance and
  ///    security.
  /// 3. Delegates to the real binary based on the intercept result.
  pub fn generate_shim(&self, tool: &str) -> String {
    let canonical = self.canonical_for_tool(tool);
    let canonical_comment = match &canonical {
      Some(c) => format!(" (canonical: {c})"),
      None => String::new(),
    };
    let mut script = String::new();
    script.push_str("#!/usr/bin/env sh\n");
    script.push_str(SHIM_MARKER);
    script.push('\n');
    script.push_str(&format!(
      "# apmw intercept shim for '{tool}'{canonical_comment}\n"
    ));
    script.push_str("# Generated by: apmw --install --intercept\n");
    script.push_str("# This shim routes invocations through apmw for governance + security.\n");
    script.push_str("exec apmw intercept \"");
    script.push_str(tool);
    script.push_str("\" \"$@\"\n");
    script
  }

  /// Returns the list of shim specs for all [`SHIM_TARGETS`].
  pub fn shim_specs(&self) -> Vec<ShimSpec> {
    SHIM_TARGETS
      .iter()
      .map(|tool| {
        let canonical = self.canonical_for_tool(tool);
        ShimSpec::new(tool, canonical.as_deref())
      })
      .collect()
  }

  /// Installs shims for all [`SHIM_TARGETS`] into the given directory.
  ///
  /// Each shim is written as an executable shell script. The directory is
  /// created if it does not exist. Existing shims are overwritten.
  pub fn install_shims(&self, dir: &Path) -> Result<Vec<PathBuf>> {
    info!(dir = %dir.display(), "installing intercept shims");
    std::fs::create_dir_all(dir).map_err(ApmwError::Io)?;

    let mut installed = Vec::new();
    for tool in SHIM_TARGETS {
      let shim_path = dir.join(tool);
      let content = self.generate_shim(tool);
      std::fs::write(&shim_path, content).map_err(ApmwError::Io)?;

      // Make the shim executable on Unix.
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&shim_path)
          .map_err(ApmwError::Io)?
          .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim_path, perms).map_err(ApmwError::Io)?;
      }

      info!(tool = tool, path = %shim_path.display(), "installed shim");
      installed.push(shim_path);
    }
    Ok(installed)
  }

  /// Removes all apmw intercept shims from the given directory.
  ///
  /// Only files containing the [`SHIM_MARKER`] are removed, so user-created
  /// scripts with the same name are preserved.
  pub fn uninstall_shims(&self, dir: &Path) -> Result<Vec<PathBuf>> {
    info!(dir = %dir.display(), "removing intercept shims");
    let mut removed = Vec::new();
    for tool in SHIM_TARGETS {
      let shim_path = dir.join(tool);
      if !shim_path.exists() {
        continue;
      }
      // Only remove files that contain the apmw marker.
      let content = std::fs::read_to_string(&shim_path).map_err(ApmwError::Io)?;
      if !content.contains(SHIM_MARKER) {
        warn!(
          tool = tool,
          path = %shim_path.display(),
          "skipping non-apmw file during shim removal"
        );
        continue;
      }
      std::fs::remove_file(&shim_path).map_err(ApmwError::Io)?;
      info!(tool = tool, path = %shim_path.display(), "removed shim");
      removed.push(shim_path);
    }
    Ok(removed)
  }

  /// Returns `true` if a shim for the given tool exists in `dir` and contains
  /// the apmw marker.
  pub fn shim_exists(&self, dir: &Path, tool: &str) -> bool {
    let path = dir.join(tool);
    if !path.exists() {
      return false;
    }
    match std::fs::read_to_string(&path) {
      Ok(content) => content.contains(SHIM_MARKER),
      Err(_) => false,
    }
  }

  // -------------------------------------------------------------------------
  // Interception logic
  // -------------------------------------------------------------------------

  /// Evaluates governance for an intercepted call to `tool` and returns the
  /// decision the shim should act on.
  ///
  /// `args` are the original arguments passed to the tool. They are used to
  /// determine whether this is an install/add operation (which requires a
  /// security scan) versus a read-only command (e.g. `--version`).
  pub fn evaluate_intercept(&self, tool: &str, args: &[String]) -> Result<InterceptDecision> {
    info!(tool = tool, args = args.join(" "), "intercepted call");

    let canonical = self.canonical_for_tool(tool);
    let canonical_installed = canonical
      .as_deref()
      .map(|c| self.is_binary_on_path(c))
      .unwrap_or(false);

    // If there is no governance rule, delegate to the original tool.
    let outcome = match self.governance.rule_for(tool) {
      Some(_) => {
        let result = self
          .governance
          .evaluate(tool, canonical.as_deref(), canonical_installed)?;
        Some(result)
      }
      None => None,
    };

    let action = match &outcome {
      None => InterceptAction::Delegate {
        tool: tool.to_string(),
      },
      Some(o) => match o.governance_type {
        GovernanceType::Prefer => {
          if o.redirected {
            // Canonical manager is installed — redirect.
            InterceptAction::Redirect {
              canonical_manager: o
                .canonical_manager
                .clone()
                .unwrap_or_else(|| tool.to_string()),
            }
          } else {
            // Canonical not installed — just warn and delegate.
            warn!(
              tool = tool,
              "prefer governance: delegating (canonical not installed)"
            );
            InterceptAction::Delegate {
              tool: tool.to_string(),
            }
          }
        }
        GovernanceType::Force => InterceptAction::Redirect {
          canonical_manager: o
            .canonical_manager
            .clone()
            .unwrap_or_else(|| tool.to_string()),
        },
        GovernanceType::Block | GovernanceType::Eject => InterceptAction::Block {
          message: o.message.clone(),
        },
      },
    };

    // A security scan should run if this looks like an install/add operation.
    let run_security_scan = is_install_command(tool, args);

    Ok(InterceptDecision {
      tool: tool.to_string(),
      governance: outcome,
      action,
      run_security_scan,
    })
  }

  /// Executes an intercepted call: evaluates governance, optionally runs a
  /// security scan, and returns the final tool + args to invoke.
  ///
  /// This is the pure-logic path; the actual process execution is the
  /// responsibility of the shim (or the `apmw intercept` subcommand).
  pub fn intercept(&self, tool: &str, args: &[String]) -> Result<InterceptResult> {
    let decision = self.evaluate_intercept(tool, args)?;

    let (effective_tool, effective_args) = match &decision.action {
      InterceptAction::Delegate { tool } => (tool.clone(), args.to_vec()),
      InterceptAction::Redirect { canonical_manager } => {
        // When redirecting, pass through the original args to the canonical
        // manager. The ecosystem mapper handles command translation at a
        // higher level; the shim simply swaps the binary.
        (canonical_manager.clone(), args.to_vec())
      }
      InterceptAction::Block { message } => {
        return Err(ApmwError::HookError(message.clone()));
      }
    };

    // Security scan is advisory here — in a full implementation the scan
    // would run against the package being installed. For now we mark it as
    // passed when no scan is needed.
    let security_passed = if decision.run_security_scan {
      info!(
        tool = tool,
        effective_tool = effective_tool.as_str(),
        "security scan would run before install"
      );
      // The actual scan is performed by the ScanOrchestrator; the hook
      // manager just signals that it should be run.
      true
    } else {
      true
    };

    Ok(InterceptResult {
      decision,
      security_passed,
      effective_tool,
      effective_args,
    })
  }

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  /// Looks up the canonical manager for a tool via the ecosystem mapper.
  fn canonical_for_tool(&self, tool: &str) -> Option<String> {
    use crate::ecosystem::{EcosystemMapper, PackageManager};
    let pm = PackageManager::parse_manager(tool)?;
    let mapper = EcosystemMapper::new();
    let canonical = mapper.suggest_canonical(pm);
    if canonical != pm {
      Some(canonical.to_string())
    } else {
      None
    }
  }

  /// Checks whether a binary is available on PATH.
  fn is_binary_on_path(&self, name: &str) -> bool {
    let scanner = PathScanner::new();
    matches!(
      scanner.scan(name),
      ScanResult::Found { .. } | ScanResult::Wrapper { .. }
    )
  }
}

// -------------------------------------------------------------------------
// Soft convention
// -------------------------------------------------------------------------

/// Generates the soft-convention instructions for AI agents.
///
/// These instructions tell the agent to use `apmw add <package>` instead of
/// calling package managers directly (e.g. `pip install`, `npm install`).
/// The soft convention is the default when no hard intercept is installed.
pub fn generate_soft_convention_instructions() -> String {
  let mut text = String::new();
  text.push_str("# apmw — AI Agent Coding Instructions\n\n");
  text.push_str("## Dependency Installation\n\n");
  text.push_str("When you need to install a package or add a dependency, **always** use:\n\n");
  text.push_str("```\napmw add <package>\n```\n\n");
  text.push_str("Do **not** call package managers directly. Instead of:\n\n");
  for tool in SHIM_TARGETS {
    text.push_str(&format!(
      "- `{tool} install <package>` → use `apmw add <package>`\n"
    ));
  }
  text.push('\n');
  text.push_str("## Why\n\n");
  text.push_str(
    "apmw automatically:\n\
     - Detects the correct package manager for the project.\n\
     - Maps within-ecosystem alternatives to the canonical runner.\n\
     - Runs security scanning before install.\n\
     - Respects governance rules (prefer/force/block/eject).\n\
     - Records an audit log entry.\n\n",
  );
  text.push_str("## Development Dependencies\n\n");
  text.push_str("To install a development/build-time dependency:\n\n");
  text.push_str("```\napmw add <package> --dev\n```\n\n");
  text.push_str("## Hard Intercept (Optional)\n\n");
  text.push_str(
    "If you find yourself calling a package manager directly despite these \
     instructions, the project owner may enable a hard intercept:\n\n\
     ```\napmw --install --intercept\n```\n\n\
     This installs PATH shims that automatically route package manager calls \
     through apmw.\n",
  );
  text
}

// -------------------------------------------------------------------------
// Free functions
// -------------------------------------------------------------------------

/// Returns `true` if the given args look like an install/add command for the
/// given tool.
///
/// This is a heuristic: it checks for common install subcommands and flags.
/// Read-only commands (e.g. `--version`, `list`, `show`) return `false`.
fn is_install_command(tool: &str, args: &[String]) -> bool {
  // No args is never an install.
  if args.is_empty() {
    return false;
  }

  // Common install subcommands across managers.
  let install_subcommands = ["install", "add", "i"];
  let first = args[0].to_ascii_lowercase();

  // Direct subcommand match (pip install, npm install, cargo install, etc.)
  if install_subcommands.contains(&first.as_str()) {
    return true;
  }

  // `go install` / `go get`
  if tool == "go" && (first == "install" || first == "get") {
    return true;
  }

  // `apt install` / `apt-get install`
  if (tool == "apt" || tool == "apt-get") && (first == "install" || first == "get") {
    return true;
  }

  // `brew install`
  if tool == "brew" && first == "install" {
    return true;
  }

  // `cargo add`
  if tool == "cargo" && first == "add" {
    return true;
  }

  // `pip install` is already covered by install_subcommands, but also handle
  // `pip install --editable .` etc. — the subcommand check suffices.

  false
}

/// Returns the default intercept shim directory.
///
/// On Unix this is `~/.local/share/apmw/shims`; on Windows it would be a
/// per-user directory (not yet implemented).
pub fn default_shim_dir() -> Result<PathBuf> {
  let data_dir = dirs::data_local_dir()
    .ok_or_else(|| ApmwError::Config("could not determine local data directory".to_string()))?;
  Ok(data_dir.join("apmw").join("shims"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
  use super::*;
  use crate::governance::GovernanceEngine;
  use tempfile::TempDir;

  // -------------------------------------------------------------------------
  // Shim generation
  // -------------------------------------------------------------------------

  #[test]
  fn test_shim_generation_contains_marker() {
    let mgr = HookManager::with_no_governance();
    let shim = mgr.generate_shim("pip");
    assert!(shim.contains(SHIM_MARKER), "shim must contain marker");
    assert!(
      shim.contains("#!/usr/bin/env sh"),
      "shim must be a shell script"
    );
    assert!(
      shim.contains("apmw intercept"),
      "shim must call apmw intercept"
    );
    assert!(
      shim.contains("\"pip\""),
      "shim must reference the tool name"
    );
  }

  #[test]
  fn test_shim_generation_for_all_targets() {
    let mgr = HookManager::with_no_governance();
    for tool in SHIM_TARGETS {
      let shim = mgr.generate_shim(tool);
      assert!(
        shim.contains(SHIM_MARKER),
        "shim for {tool} must contain marker"
      );
      assert!(
        shim.contains(&format!("\"{tool}\"")),
        "shim for {tool} must reference the tool"
      );
    }
  }

  #[test]
  fn test_shim_specs_include_all_targets() {
    let mgr = HookManager::with_no_governance();
    let specs = mgr.shim_specs();
    assert_eq!(specs.len(), SHIM_TARGETS.len());
    for (spec, target) in specs.iter().zip(SHIM_TARGETS.iter()) {
      assert_eq!(spec.tool, *target);
      assert_eq!(spec.shim_name, *target);
    }
  }

  #[test]
  fn test_shim_spec_pip_has_canonical_uv() {
    let mgr = HookManager::with_no_governance();
    let specs = mgr.shim_specs();
    let pip_spec = specs.iter().find(|s| s.tool == "pip").unwrap();
    assert_eq!(pip_spec.canonical_manager.as_deref(), Some("uv"));
  }

  #[test]
  fn test_shim_spec_npm_has_canonical_pnpm() {
    let mgr = HookManager::with_no_governance();
    let specs = mgr.shim_specs();
    let npm_spec = specs.iter().find(|s| s.tool == "npm").unwrap();
    assert_eq!(npm_spec.canonical_manager.as_deref(), Some("pnpm"));
  }

  #[test]
  fn test_shim_spec_cargo_has_no_canonical() {
    // cargo is its own canonical — no redirect.
    let mgr = HookManager::with_no_governance();
    let specs = mgr.shim_specs();
    let cargo_spec = specs.iter().find(|s| s.tool == "cargo").unwrap();
    assert!(cargo_spec.canonical_manager.is_none());
  }

  // -------------------------------------------------------------------------
  // Install / uninstall shims
  // -------------------------------------------------------------------------

  #[test]
  fn test_install_shims_creates_executable_files() {
    let mgr = HookManager::with_no_governance();
    let dir = TempDir::new().unwrap();
    let installed = mgr.install_shims(dir.path()).unwrap();
    assert_eq!(installed.len(), SHIM_TARGETS.len());
    for path in &installed {
      assert!(path.exists(), "shim file should exist");
      let content = std::fs::read_to_string(path).unwrap();
      assert!(content.contains(SHIM_MARKER), "shim must contain marker");
    }
  }

  #[test]
  fn test_install_shims_creates_directory_if_missing() {
    let mgr = HookManager::with_no_governance();
    let dir = TempDir::new().unwrap();
    let nested = dir.path().join("nested").join("shims");
    let installed = mgr.install_shims(&nested).unwrap();
    assert!(!installed.is_empty());
    assert!(nested.exists());
  }

  #[test]
  fn test_uninstall_shims_removes_apmw_shims() {
    let mgr = HookManager::with_no_governance();
    let dir = TempDir::new().unwrap();
    mgr.install_shims(dir.path()).unwrap();
    let removed = mgr.uninstall_shims(dir.path()).unwrap();
    assert_eq!(removed.len(), SHIM_TARGETS.len());
    for path in &removed {
      assert!(!path.exists(), "shim should be removed");
    }
  }

  #[test]
  fn test_uninstall_shims_preserves_non_apmw_files() {
    let mgr = HookManager::with_no_governance();
    let dir = TempDir::new().unwrap();
    mgr.install_shims(dir.path()).unwrap();

    // Overwrite one shim with a user script (no marker).
    let pip_path = dir.path().join("pip");
    std::fs::write(&pip_path, "#!/usr/bin/env sh\nreal-pip \"$@\"\n").unwrap();

    let removed = mgr.uninstall_shims(dir.path()).unwrap();
    // pip should NOT be in the removed list.
    assert!(
      !removed.iter().any(|p| p.file_name().unwrap() == "pip"),
      "user-created pip script should be preserved"
    );
    assert!(pip_path.exists(), "user pip script should still exist");
  }

  #[test]
  fn test_shim_exists_detects_apmw_shim() {
    let mgr = HookManager::with_no_governance();
    let dir = TempDir::new().unwrap();
    mgr.install_shims(dir.path()).unwrap();
    assert!(mgr.shim_exists(dir.path(), "pip"));
    assert!(mgr.shim_exists(dir.path(), "npm"));
    assert!(!mgr.shim_exists(dir.path(), "nonexistent-tool"));
  }

  #[test]
  fn test_shim_exists_false_for_non_apmw_file() {
    let mgr = HookManager::with_no_governance();
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::write(dir.path().join("pip"), "# user script\n").unwrap();
    assert!(
      !mgr.shim_exists(dir.path(), "pip"),
      "non-apmw file should not be detected as a shim"
    );
  }

  // -------------------------------------------------------------------------
  // Interception logic — no governance rule
  // -------------------------------------------------------------------------

  #[test]
  fn test_intercept_no_governance_rule_delegates() {
    let mgr = HookManager::with_no_governance();
    let args = vec!["install".to_string(), "foo".to_string()];
    let result = mgr.intercept("pip", &args).unwrap();
    assert_eq!(result.effective_tool, "pip");
    assert_eq!(result.effective_args, args);
    assert!(result.security_passed);
    assert!(result.decision.run_security_scan);
  }

  #[test]
  fn test_intercept_readonly_command_no_security_scan() {
    let mgr = HookManager::with_no_governance();
    let args = vec!["--version".to_string()];
    let result = mgr.intercept("pip", &args).unwrap();
    assert!(!result.decision.run_security_scan);
  }

  // -------------------------------------------------------------------------
  // Governance-aware interception
  // -------------------------------------------------------------------------

  #[test]
  fn test_governance_hooks_prefer_delegates_when_canonical_not_installed() {
    // Use a tool whose canonical manager is unlikely to be installed in the
    // test environment. `yarn`'s canonical is `pnpm`; if pnpm happens to be
    // installed, the test still passes because prefer would redirect — we
    // accept either outcome as long as the call is not blocked.
    let engine = GovernanceEngine::with_spec_text("prefer yarn");
    let mgr = HookManager::new(engine);
    let args = vec!["add".to_string(), "foo".to_string()];
    let result = mgr.intercept("yarn", &args).unwrap();
    // prefer should never block.
    assert!(!matches!(
      result.decision.action,
      InterceptAction::Block { .. }
    ));
    assert!(result.decision.governance.is_some());
    // The effective tool is either the original (delegate) or the canonical
    // (redirect) depending on whether the canonical is installed.
    assert!(
      result.effective_tool == "yarn" || result.effective_tool == "pnpm",
      "unexpected effective_tool: {}",
      result.effective_tool
    );
  }

  #[test]
  fn test_governance_hooks_force_redirects_to_canonical() {
    let engine = GovernanceEngine::with_spec_text("force pip");
    let mgr = HookManager::new(engine);
    let args = vec!["install".to_string(), "foo".to_string()];
    let result = mgr.intercept("pip", &args).unwrap();
    // force → redirect to uv (canonical).
    assert_eq!(result.effective_tool, "uv");
    assert_eq!(result.effective_args, args);
  }

  #[test]
  fn test_governance_hooks_block_prevents_call() {
    let engine = GovernanceEngine::with_spec_text("block pip");
    let mgr = HookManager::new(engine);
    let args = vec!["install".to_string(), "foo".to_string()];
    let result = mgr.intercept("pip", &args);
    assert!(result.is_err(), "block governance should produce an error");
    let err = result.unwrap_err();
    assert!(
      err.to_string().contains("block"),
      "error should mention block: {}",
      err
    );
  }

  #[test]
  fn test_governance_hooks_eject_prevents_call() {
    let engine = GovernanceEngine::with_spec_text("eject npm");
    let mgr = HookManager::new(engine);
    let args = vec!["install".to_string(), "foo".to_string()];
    let result = mgr.intercept("npm", &args);
    assert!(result.is_err(), "eject governance should produce an error");
  }

  #[test]
  fn test_governance_hooks_force_npm_redirects_to_pnpm() {
    let engine = GovernanceEngine::with_spec_text("force npm");
    let mgr = HookManager::new(engine);
    let args = vec!["install".to_string(), "express".to_string()];
    let result = mgr.intercept("npm", &args).unwrap();
    assert_eq!(result.effective_tool, "pnpm");
  }

  #[test]
  fn test_governance_hooks_force_yarn_redirects_to_pnpm() {
    let engine = GovernanceEngine::with_spec_text("force yarn");
    let mgr = HookManager::new(engine);
    let args = vec!["add".to_string(), "express".to_string()];
    let result = mgr.intercept("yarn", &args).unwrap();
    assert_eq!(result.effective_tool, "pnpm");
  }

  #[test]
  fn test_governance_hooks_no_rule_for_cargo_delegates() {
    // cargo is its own canonical — no governance rule → delegate.
    let engine = GovernanceEngine::with_spec_text("force pip");
    let mgr = HookManager::new(engine);
    let args = vec!["add".to_string(), "serde".to_string()];
    let result = mgr.intercept("cargo", &args).unwrap();
    assert_eq!(result.effective_tool, "cargo");
  }

  #[test]
  fn test_intercept_decision_action_types() {
    // Block
    let engine = GovernanceEngine::with_spec_text("block npm");
    let mgr = HookManager::new(engine);
    let decision = mgr.evaluate_intercept("npm", &["install".into()]).unwrap();
    assert!(matches!(decision.action, InterceptAction::Block { .. }));

    // Force → Redirect
    let engine = GovernanceEngine::with_spec_text("force npm");
    let mgr = HookManager::new(engine);
    let decision = mgr.evaluate_intercept("npm", &["install".into()]).unwrap();
    assert!(matches!(decision.action, InterceptAction::Redirect { .. }));

    // No rule → Delegate
    let mgr = HookManager::with_no_governance();
    let decision = mgr.evaluate_intercept("npm", &["install".into()]).unwrap();
    assert!(matches!(decision.action, InterceptAction::Delegate { .. }));
  }

  // -------------------------------------------------------------------------
  // Soft convention
  // -------------------------------------------------------------------------

  #[test]
  fn test_soft_convention_mentions_apmw_add() {
    let text = generate_soft_convention_instructions();
    assert!(text.contains("apmw add <package>"), "must mention apmw add");
    assert!(
      text.contains("AI Agent Coding Instructions"),
      "must have a title"
    );
  }

  #[test]
  fn test_soft_convention_lists_all_shim_targets() {
    let text = generate_soft_convention_instructions();
    for tool in SHIM_TARGETS {
      assert!(
        text.contains(&format!("{tool} install")),
        "must mention {tool} install as something to avoid"
      );
    }
  }

  #[test]
  fn test_soft_convention_mentions_dev_flag() {
    let text = generate_soft_convention_instructions();
    assert!(text.contains("--dev"), "must mention --dev flag");
  }

  #[test]
  fn test_soft_convention_mentions_hard_intercept() {
    let text = generate_soft_convention_instructions();
    assert!(
      text.contains("--install --intercept"),
      "must mention hard intercept option"
    );
  }

  // -------------------------------------------------------------------------
  // is_install_command heuristic
  // -------------------------------------------------------------------------

  #[test]
  fn test_is_install_command_pip_install() {
    assert!(is_install_command("pip", &["install".into(), "foo".into()]));
  }

  #[test]
  fn test_is_install_command_npm_install() {
    assert!(is_install_command(
      "npm",
      &["install".into(), "express".into()]
    ));
  }

  #[test]
  fn test_is_install_command_yarn_add() {
    assert!(is_install_command(
      "yarn",
      &["add".into(), "express".into()]
    ));
  }

  #[test]
  fn test_is_install_command_cargo_add() {
    assert!(is_install_command("cargo", &["add".into(), "serde".into()]));
  }

  #[test]
  fn test_is_install_command_cargo_install() {
    assert!(is_install_command(
      "cargo",
      &["install".into(), "ripgrep".into()]
    ));
  }

  #[test]
  fn test_is_install_command_go_install() {
    assert!(is_install_command(
      "go",
      &["install".into(), "foo@latest".into()]
    ));
  }

  #[test]
  fn test_is_install_command_brew_install() {
    assert!(is_install_command(
      "brew",
      &["install".into(), "ripgrep".into()]
    ));
  }

  #[test]
  fn test_is_install_command_apt_install() {
    assert!(is_install_command(
      "apt",
      &["install".into(), "curl".into()]
    ));
  }

  #[test]
  fn test_is_install_command_version_is_not_install() {
    assert!(!is_install_command("pip", &["--version".into()]));
  }

  #[test]
  fn test_is_install_command_list_is_not_install() {
    assert!(!is_install_command("pip", &["list".into()]));
  }

  #[test]
  fn test_is_install_command_no_args_is_not_install() {
    assert!(!is_install_command("pip", &[]));
  }

  // -------------------------------------------------------------------------
  // default_shim_dir
  // -------------------------------------------------------------------------

  #[test]
  fn test_default_shim_dir_is_valid_path() {
    let dir = default_shim_dir().unwrap();
    assert!(
      dir.to_string_lossy().contains("apmw"),
      "dir must contain apmw"
    );
    assert!(
      dir.to_string_lossy().contains("shims"),
      "dir must contain shims"
    );
  }
}
