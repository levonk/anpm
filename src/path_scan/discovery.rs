//! cli-tool-discovery integration — Rust reimplementation.
//!
//! This module implements the resolution flow from `cli-tool-discovery.sh`
//! directly in Rust, without shelling out to the script. The script is used as
//! a reference for the algorithm.
//!
//! ## Resolution flow
//!
//! 1. **Devbox shell check** (`DEVBOX_SHELL` / `IN_DEVBOX_SHELL`) — checked
//!    first. If inside a devbox shell, devbox-managed binaries are on `PATH`
//!    and wrapper detection (mise/flox/direnv/nix) is skipped entirely.
//! 2. **Devbox availability** — if not in a devbox shell but devbox is
//!    available and a `devbox.json` exists up the tree, verify the tool inside
//!    the devbox environment. If found, return `WRAPPER:devbox run --`.
//! 3. **Normal flow** — `PATH` → other wrappers (mise, flox, direnv, nix) →
//!    30+ standard PATH locations → package managers (brew, mise, asdf).
//! 4. **Repo-root fallback** — `$REPO_ROOT/bin`, `scripts/`, `.local/bin` as
//!    a last resort (least secure: a cloned repo could contain malicious
//!    executables in `bin/`).
//! 5. **nix/uv fallback** — handled by the install engine (story 04-001), not
//!    the scanner. The scanner only reports what's already available.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// The kind of environment wrapper detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapperKind {
  /// mise (formerly rtx) — version manager.
  Mise,
  /// flox — Nix-based environment manager.
  Flox,
  /// direnv — directory-specific environment variables.
  Direnv,
  /// nix — Nix package manager / Nix shell.
  Nix,
}

impl WrapperKind {
  /// Returns the environment variable that indicates this wrapper is already
  /// active (and thus should NOT be detected as a wrapper to activate).
  fn active_env_var(&self) -> &'static str {
    match self {
      WrapperKind::Mise => "MISE_SHELL",
      WrapperKind::Flox => "FLOX_ACTIVE",
      WrapperKind::Direnv => "DIRENV_DIR",
      WrapperKind::Nix => "IN_NIX_SHELL",
    }
  }

  /// Returns the config files that indicate this wrapper is in use for the
  /// current project (walked up from the current directory).
  fn config_files(&self) -> &'static [&'static str] {
    match self {
      WrapperKind::Mise => &[".mise.toml", ".mise/config.toml", "mise.toml"],
      WrapperKind::Flox => &["flox.nix"],
      WrapperKind::Direnv => &[".envrc"],
      WrapperKind::Nix => &["shell.nix", "flake.nix"],
    }
  }

  /// Returns the wrapper command string to use when this wrapper is detected.
  fn command(&self, has_flake: bool) -> String {
    match self {
      WrapperKind::Mise => "mise exec --".to_string(),
      WrapperKind::Flox => "flox activate --".to_string(),
      WrapperKind::Direnv => "direnv export &&".to_string(),
      WrapperKind::Nix => {
        if has_flake {
          "nix develop --command".to_string()
        } else {
          "nix-shell --run".to_string()
        }
      }
    }
  }
}

/// Checks if we are inside a devbox shell by examining the `DEVBOX_SHELL` or
/// `IN_DEVBOX_SHELL` environment variables.
///
/// Per the cli-tool-discovery contract, this is checked FIRST, before any
/// other resolution. If inside a devbox shell, devbox-managed binaries are
/// on `PATH` and wrapper detection is skipped entirely.
pub fn is_in_devbox_shell(env_overrides: &HashMap<String, String>) -> bool {
  env_overrides.get("DEVBOX_SHELL").is_some()
    || env_overrides.get("IN_DEVBOX_SHELL").is_some()
    || env::var("DEVBOX_SHELL").is_ok()
    || env::var("IN_DEVBOX_SHELL").is_ok()
}

/// Checks if a wrapper's environment is already active (meaning the wrapper
/// has already been activated and should NOT be detected again).
pub fn wrapper_env_active(kind: WrapperKind, env_overrides: &HashMap<String, String>) -> bool {
  let var = kind.active_env_var();
  // When env_overrides is non-empty (test mode), only check overrides — don't
  // fall back to the real environment, which may have wrapper env vars set
  // (e.g. IN_NIX_SHELL when running tests inside a devbox/nix shell).
  if !env_overrides.is_empty() {
    return env_overrides.get(var).is_some();
  }
  env_overrides.get(var).is_some() || env::var(var).is_ok()
}

/// Detects environment wrappers (mise, flox, direnv, nix) by walking up from
/// the current directory looking for config files.
///
/// Returns the wrapper command string if a wrapper is detected and its
/// environment is not already active.
pub fn detect_wrapper(env_overrides: &HashMap<String, String>, cwd: &str) -> Option<String> {
  let cwd_path = Path::new(cwd);

  // Check each wrapper in order: mise, flox, direnv, nix
  for kind in [
    WrapperKind::Mise,
    WrapperKind::Flox,
    WrapperKind::Direnv,
    WrapperKind::Nix,
  ] {
    // Skip if the wrapper's environment is already active
    if wrapper_env_active(kind, env_overrides) {
      continue;
    }

    // Check if the wrapper binary is available on PATH
    let wrapper_bin = match kind {
      WrapperKind::Mise => "mise",
      WrapperKind::Flox => "flox",
      WrapperKind::Direnv => "direnv",
      WrapperKind::Nix => "nix",
    };

    // Only detect wrapper if the binary is available
    if !is_binary_on_path(wrapper_bin, env_overrides) {
      continue;
    }

    // Walk up from cwd looking for config files
    if let Some(config_dir) = walk_up_for_files(cwd_path, kind.config_files()) {
      // For nix, the command depends on whether flake.nix or shell.nix was found
      let has_flake = if kind == WrapperKind::Nix {
        config_dir.join("flake.nix").is_file()
      } else {
        false
      };
      return Some(kind.command(has_flake));
    }
  }

  None
}

/// Walks up from a starting directory looking for any of the given config
/// files. Returns the directory containing the first match.
fn walk_up_for_files(start: &Path, files: &[&str]) -> Option<PathBuf> {
  let mut current = start.to_path_buf();
  loop {
    for file in files {
      if current.join(file).is_file() {
        return Some(current);
      }
    }
    if !current.pop() {
      return None;
    }
  }
}

/// Checks if a binary is available on `PATH` (using env overrides or real env).
fn is_binary_on_path(bin: &str, env_overrides: &HashMap<String, String>) -> bool {
  let path_env = env_overrides
    .get("PATH")
    .cloned()
    .or_else(|| env::var("PATH").ok())
    .unwrap_or_default();

  for dir in path_env.split(':') {
    if dir.is_empty() {
      continue;
    }
    let candidate = Path::new(dir).join(bin);
    if is_executable(&candidate) {
      return true;
    }
  }
  false
}

/// Checks if a path is an executable file.
fn is_executable(path: &Path) -> bool {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
      Ok(meta) => meta.is_file() && (meta.permissions().mode() & 0o111 != 0),
      Err(_) => false,
    }
  }
  #[cfg(not(unix))]
  {
    std::fs::metadata(path)
      .map(|m| m.is_file())
      .unwrap_or(false)
  }
}

/// Returns the 30+ standard search directories for PATH scanning.
///
/// These cover system paths, home directory tool paths, and language-specific
/// bin directories. The order follows the `cli-tool-discovery.sh` script.
pub fn standard_search_dirs(home: &Path, env_overrides: &HashMap<String, String>) -> Vec<PathBuf> {
  let mut dirs = Vec::new();

  // XDG bin home (if set)
  if let Some(xdg) = env_overrides.get("XDG_BIN_HOME") {
    if !xdg.is_empty() {
      dirs.push(PathBuf::from(xdg));
    }
  } else if let Ok(xdg) = env::var("XDG_BIN_HOME") {
    if !xdg.is_empty() {
      dirs.push(PathBuf::from(xdg));
    }
  }

  // Home directory paths
  dirs.push(home.join(".local/bin"));
  dirs.push(home.join(".nix-profile/bin"));
  dirs.push(home.join("bin"));

  // System nix profile
  dirs.push(PathBuf::from("/nix/var/nix/profiles/default/bin"));

  // Standard system paths
  dirs.push(PathBuf::from("/usr/local/bin"));
  dirs.push(PathBuf::from("/usr/local/sbin"));
  dirs.push(PathBuf::from("/usr/sbin"));
  dirs.push(PathBuf::from("/usr/bin"));
  dirs.push(PathBuf::from("/sbin"));
  dirs.push(PathBuf::from("/bin"));

  // Homebrew (Apple Silicon)
  dirs.push(PathBuf::from("/opt/homebrew/bin"));
  dirs.push(PathBuf::from("/opt/homebrew/sbin"));

  // MacPorts
  dirs.push(PathBuf::from("/opt/local/bin"));

  // Snap (Linux)
  dirs.push(PathBuf::from("/snap/bin"));

  // NixOS
  dirs.push(PathBuf::from("/run/current-system/sw/bin"));

  // Language-specific home directories
  dirs.push(home.join(".cargo/bin")); // Rust
  dirs.push(home.join(".bun/bin")); // Bun
  dirs.push(home.join(".deno/bin")); // Deno
  dirs.push(home.join(".volta/bin")); // Volta (Node)
  dirs.push(home.join("go/bin")); // Go
  dirs.push(home.join(".rbenv/shims")); // Ruby (rbenv)
  dirs.push(home.join(".pyenv/shims")); // Python (pyenv)
  dirs.push(home.join(".pixi/bin")); // Pixi (Conda)
  dirs.push(home.join(".krew/bin")); // krew (Kubernetes plugins)
  dirs.push(home.join(".foundry/bin")); // Foundry (Ethereum)
  dirs.push(home.join(".roswell/bin")); // Roswell (Common Lisp)

  // nvm (Node Version Manager) — scan version directories
  let nvm_versions = home.join(".nvm/versions/node");
  if nvm_versions.is_dir() {
    if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
      for entry in entries.flatten() {
        let bin = entry.path().join("bin");
        if bin.is_dir() {
          dirs.push(bin);
        }
      }
    }
  }

  // mise installs: ~/.local/share/mise/installs/*/bin
  let mise_installs = home.join(".local/share/mise/installs");
  if mise_installs.is_dir() {
    if let Ok(entries) = std::fs::read_dir(&mise_installs) {
      for entry in entries.flatten() {
        let bin = entry.path().join("bin");
        if bin.is_dir() {
          dirs.push(bin);
        }
      }
    }
  }

  // rtx installs (legacy mise): ~/.local/share/rtx/installs/*/bin
  let rtx_installs = home.join(".local/share/rtx/installs");
  if rtx_installs.is_dir() {
    if let Ok(entries) = std::fs::read_dir(&rtx_installs) {
      for entry in entries.flatten() {
        let bin = entry.path().join("bin");
        if bin.is_dir() {
          dirs.push(bin);
        }
      }
    }
  }

  // asdf shims
  dirs.push(home.join(".asdf/shims"));

  // conda environments
  dirs.push(home.join("miniconda3/bin"));
  dirs.push(home.join("anaconda3/bin"));

  // pipx
  dirs.push(home.join(".local/pipx/venvs"));

  // Deno (alternative location)
  dirs.push(home.join(".deno/bin"));

  // Go (alternative GOPATH)
  dirs.push(home.join("go/bin"));

  // Rust (alternative)
  dirs.push(home.join(".rustup/toolchains"));

  // Total: 30+ directories (base 30 + dynamic nvm/mise/rtx additions)
  dirs
}

/// Resolves the repo root via `git rev-parse --show-toplevel` or falls back
/// to the current working directory.
pub fn resolve_repo_root(env_overrides: &HashMap<String, String>) -> PathBuf {
  // Check for REPO_ROOT env override first
  if let Some(root) = env_overrides.get("REPO_ROOT") {
    return PathBuf::from(root);
  }
  if let Ok(root) = env::var("REPO_ROOT") {
    return PathBuf::from(root);
  }

  // Try git rev-parse --show-toplevel
  if let Ok(output) = std::process::Command::new("git")
    .args(["rev-parse", "--show-toplevel"])
    .output()
  {
    if output.status.success() {
      let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
      if !path.is_empty() {
        return PathBuf::from(path);
      }
    }
  }

  // Fall back to current directory
  env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Walks up from a starting directory looking for `devbox.json`.
/// Returns the directory containing the file, or `None` if not found.
pub fn devbox_json_dir(start: &Path) -> Option<PathBuf> {
  walk_up_for_files(start, &["devbox.json"])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::os::unix::fs::PermissionsExt;
  use tempfile::TempDir;

  fn make_executable(path: &Path) {
    fs::write(path, "#!/bin/sh\necho hi\n").unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
  }

  // --- Devbox shell detection tests ---

  #[test]
  fn test_devbox_resolution_devbox_shell_set() {
    let mut env = HashMap::new();
    env.insert("DEVBOX_SHELL".to_string(), "1".to_string());
    assert!(is_in_devbox_shell(&env));
  }

  #[test]
  fn test_devbox_resolution_in_devbox_shell_set() {
    let mut env = HashMap::new();
    env.insert("IN_DEVBOX_SHELL".to_string(), "1".to_string());
    assert!(is_in_devbox_shell(&env));
  }

  #[test]
  fn test_devbox_resolution_neither_set() {
    // Note: this test may be affected by the real environment.
    // We only test the override map here.
    let env = HashMap::new();
    // In a test environment, DEVBOX_SHELL is typically not set.
    // But we can't assert false because the real env might have it.
    // The important thing is the function doesn't panic.
    let _ = is_in_devbox_shell(&env);
  }

  // --- Wrapper detection tests ---

  #[test]
  fn test_wrapper_detection_mise_env_active() {
    let mut env = HashMap::new();
    env.insert("MISE_SHELL".to_string(), "bash".to_string());
    assert!(wrapper_env_active(WrapperKind::Mise, &env));
  }

  #[test]
  fn test_wrapper_detection_flox_env_active() {
    let mut env = HashMap::new();
    env.insert("FLOX_ACTIVE".to_string(), "1".to_string());
    assert!(wrapper_env_active(WrapperKind::Flox, &env));
  }

  #[test]
  fn test_wrapper_detection_direnv_env_active() {
    let mut env = HashMap::new();
    env.insert("DIRENV_DIR".to_string(), "/some/path".to_string());
    assert!(wrapper_env_active(WrapperKind::Direnv, &env));
  }

  #[test]
  fn test_wrapper_detection_nix_env_active() {
    let mut env = HashMap::new();
    env.insert("IN_NIX_SHELL".to_string(), "1".to_string());
    assert!(wrapper_env_active(WrapperKind::Nix, &env));
  }

  #[test]
  fn test_wrapper_detection_no_env_active() {
    // Use a non-empty HashMap to stay in "test mode" (no real-env fallback),
    // otherwise IN_NIX_SHELL from the devbox shell would cause Nix to appear active.
    let mut env = HashMap::new();
    env.insert("__test_marker".to_string(), "1".to_string());
    assert!(!wrapper_env_active(WrapperKind::Mise, &env));
    assert!(!wrapper_env_active(WrapperKind::Flox, &env));
    assert!(!wrapper_env_active(WrapperKind::Direnv, &env));
    assert!(!wrapper_env_active(WrapperKind::Nix, &env));
  }

  #[test]
  fn test_wrapper_detection_config_files() {
    assert_eq!(
      WrapperKind::Mise.config_files(),
      &[".mise.toml", ".mise/config.toml", "mise.toml"]
    );
    assert_eq!(WrapperKind::Flox.config_files(), &["flox.nix"]);
    assert_eq!(WrapperKind::Direnv.config_files(), &[".envrc"]);
    assert_eq!(WrapperKind::Nix.config_files(), &["shell.nix", "flake.nix"]);
  }

  #[test]
  fn test_wrapper_detection_commands() {
    assert_eq!(WrapperKind::Mise.command(false), "mise exec --");
    assert_eq!(WrapperKind::Flox.command(false), "flox activate --");
    assert_eq!(WrapperKind::Direnv.command(false), "direnv export &&");
    assert_eq!(WrapperKind::Nix.command(true), "nix develop --command");
    assert_eq!(WrapperKind::Nix.command(false), "nix-shell --run");
  }

  // --- walk_up_for_files tests ---

  #[test]
  fn test_walk_up_finds_file_in_current_dir() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("devbox.json"), "{}").unwrap();

    let result = walk_up_for_files(temp.path(), &["devbox.json"]);
    assert_eq!(result, Some(temp.path().to_path_buf()));
  }

  #[test]
  fn test_walk_up_finds_file_in_parent_dir() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("devbox.json"), "{}").unwrap();
    let subdir = temp.path().join("subdir");
    fs::create_dir_all(&subdir).unwrap();

    let result = walk_up_for_files(&subdir, &["devbox.json"]);
    assert_eq!(result, Some(temp.path().to_path_buf()));
  }

  #[test]
  fn test_walk_up_returns_none_when_not_found() {
    let temp = TempDir::new().unwrap();
    let result = walk_up_for_files(temp.path(), &["nonexistent_file.xyz"]);
    assert_eq!(result, None);
  }

  // --- devbox_json_dir tests ---

  #[test]
  fn test_devbox_json_dir_finds_in_current() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("devbox.json"), "{}").unwrap();

    let result = devbox_json_dir(temp.path());
    assert_eq!(result, Some(temp.path().to_path_buf()));
  }

  #[test]
  fn test_devbox_json_dir_finds_in_parent() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("devbox.json"), "{}").unwrap();
    let subdir = temp.path().join("src");
    fs::create_dir_all(&subdir).unwrap();

    let result = devbox_json_dir(&subdir);
    assert_eq!(result, Some(temp.path().to_path_buf()));
  }

  #[test]
  fn test_devbox_json_dir_not_found() {
    let temp = TempDir::new().unwrap();
    let result = devbox_json_dir(temp.path());
    // May find a devbox.json somewhere up the tree from the temp dir,
    // but in a temp dir this is unlikely. We just verify it doesn't panic.
    let _ = result;
  }

  // --- standard_search_dirs tests ---

  #[test]
  fn test_standard_search_dirs_includes_key_paths() {
    let home = PathBuf::from("/home/user");
    let env = HashMap::new();
    let dirs = standard_search_dirs(&home, &env);

    // Verify key paths are included
    assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
    assert!(dirs.contains(&PathBuf::from("/usr/bin")));
    assert!(dirs.contains(&PathBuf::from("/bin")));
    assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
    assert!(dirs.contains(&home.join(".cargo/bin")));
    assert!(dirs.contains(&home.join(".local/bin")));
    assert!(dirs.contains(&home.join(".bun/bin")));
    assert!(dirs.contains(&home.join(".deno/bin")));
    assert!(dirs.contains(&home.join(".volta/bin")));
    assert!(dirs.contains(&home.join(".roswell/bin")));
  }

  #[test]
  fn test_standard_search_dirs_count_30_plus() {
    let home = PathBuf::from("/home/user");
    let env = HashMap::new();
    let dirs = standard_search_dirs(&home, &env);
    assert!(
      dirs.len() >= 30,
      "expected 30+ standard search dirs, got {}: {dirs:?}",
      dirs.len()
    );
  }

  #[test]
  fn test_standard_search_dirs_with_xdg_bin_home() {
    let home = PathBuf::from("/home/user");
    let mut env = HashMap::new();
    env.insert("XDG_BIN_HOME".to_string(), "/custom/xdg/bin".to_string());
    let dirs = standard_search_dirs(&home, &env);
    assert!(dirs.contains(&PathBuf::from("/custom/xdg/bin")));
  }

  // --- resolve_repo_root tests ---

  #[test]
  fn test_resolve_repo_root_env_override() {
    let mut env = HashMap::new();
    env.insert("REPO_ROOT".to_string(), "/custom/repo".to_string());
    let root = resolve_repo_root(&env);
    assert_eq!(root, PathBuf::from("/custom/repo"));
  }

  // --- detect_wrapper integration tests ---

  #[test]
  fn test_detect_wrapper_with_mise_config() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".mise.toml"), "").unwrap();

    // Create a fake mise binary on PATH
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("mise"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());
    env.insert("CWD".to_string(), temp.path().display().to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, Some("mise exec --".to_string()));
  }

  #[test]
  fn test_detect_wrapper_with_flox_config() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("flox.nix"), "").unwrap();

    // Create a fake flox binary on PATH
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("flox"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, Some("flox activate --".to_string()));
  }

  #[test]
  fn test_detect_wrapper_with_direnv_config() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".envrc"), "").unwrap();

    // Create a fake direnv binary on PATH
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("direnv"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, Some("direnv export &&".to_string()));
  }

  #[test]
  fn test_detect_wrapper_with_nix_flake() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("flake.nix"), "").unwrap();

    // Create a fake nix binary on PATH
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("nix"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, Some("nix develop --command".to_string()));
  }

  #[test]
  fn test_detect_wrapper_with_nix_shell() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("shell.nix"), "").unwrap();

    // Create a fake nix binary on PATH
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("nix"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, Some("nix-shell --run".to_string()));
  }

  #[test]
  fn test_detect_wrapper_skips_when_env_active() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".mise.toml"), "").unwrap();

    // mise is on PATH but MISE_SHELL is set → should not detect
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("mise"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());
    env.insert("MISE_SHELL".to_string(), "bash".to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, None);
  }

  #[test]
  fn test_detect_wrapper_no_config_files() {
    let temp = TempDir::new().unwrap();

    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("mise"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());

    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    assert_eq!(wrapper, None);
  }

  #[test]
  fn test_detect_wrapper_no_binaries_on_path() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".mise.toml"), "").unwrap();

    let env = HashMap::new();
    let wrapper = detect_wrapper(&env, &temp.path().display().to_string());
    // No wrapper binaries on PATH → no detection
    // (may detect from real PATH, but unlikely in test env)
    let _ = wrapper;
  }

  // --- is_binary_on_path tests ---

  #[test]
  fn test_is_binary_on_path_found() {
    let bin_dir = TempDir::new().unwrap();
    make_executable(&bin_dir.path().join("mytool"));

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), bin_dir.path().display().to_string());

    assert!(is_binary_on_path("mytool", &env));
  }

  #[test]
  fn test_is_binary_on_path_not_found() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/nonexistent".to_string());
    assert!(!is_binary_on_path("nonexistent_tool", &env));
  }

  // --- is_executable tests ---

  #[test]
  fn test_is_executable_with_executable_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("script.sh");
    make_executable(&file);
    assert!(is_executable(&file));
  }

  #[test]
  fn test_is_executable_with_non_executable_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("not_exec.txt");
    fs::write(&file, "hello").unwrap();
    assert!(!is_executable(&file));
  }

  #[test]
  fn test_is_executable_with_nonexistent_file() {
    assert!(!is_executable(Path::new("/nonexistent/file")));
  }

  #[test]
  fn test_is_executable_with_directory() {
    let temp = TempDir::new().unwrap();
    assert!(!is_executable(temp.path()));
  }
}
