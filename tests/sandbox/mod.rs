//! Sandbox test harness for apmw integration tests.
//!
//! Wraps `Command::cargo_bin("apmw")` through [nono] to isolate test
//! subprocesses from the developer's personal files and restrict network
//! access to real package registries only.
//!
//! [nono]: https://nono.sh

use assert_cmd::Command;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Nono sandbox profile. Serialized to JSON and passed to `nono run --profile`.
#[derive(Debug, Serialize, Deserialize)]
pub struct NonoProfile {
  /// Paths the sandboxed process may read.
  pub fs_read: Vec<String>,
  /// Paths the sandboxed process may write.
  pub fs_write: Vec<String>,
  /// Network hosts the sandboxed process may contact.
  pub net_allow: Vec<String>,
  /// Network hosts the sandboxed process may NOT contact (`*` denies all
  /// others).
  pub net_deny: Vec<String>,
  /// Environment variables passed through to the sandboxed process.
  pub env: Vec<String>,
}

/// Create a default nono profile for a test with the given TempDir.
///
/// - `fs_write`: restricted to the TempDir
/// - `fs_read`: TempDir + system paths + package manager stores
/// - `net_allow`: real registries only (npmjs, crates.io, github)
/// - `net_deny`: everything else (`*`)
/// - `env`: minimal allowlist (PATH, HOME, cargo/rustup paths, XDG, TMPDIR)
pub fn default_profile(tempdir: &Path) -> NonoProfile {
  let dir_str = tempdir.display().to_string();
  NonoProfile {
    fs_write: vec![dir_str.clone()],
    fs_read: vec![
      dir_str,
      "/usr".to_string(),
      "/lib".to_string(),
      "/lib64".to_string(),
      "/etc".to_string(),
      "/tmp".to_string(),
      "~/.cargo/registry".to_string(),
      "~/.local/share/pnpm".to_string(),
      "~/.config/pnpm".to_string(),
      "~/.rustup".to_string(),
    ],
    net_allow: vec![
      "registry.npmjs.org".to_string(),
      "crates.io".to_string(),
      "static.crates.io".to_string(),
      "index.crates.io".to_string(),
      "github.com".to_string(),
      "raw.githubusercontent.com".to_string(),
      "objects.githubusercontent.com".to_string(),
    ],
    net_deny: vec!["*".to_string()],
    env: vec![
      "PATH".to_string(),
      "HOME".to_string(),
      "USER".to_string(),
      "SHELL".to_string(),
      "TERM".to_string(),
      "LANG".to_string(),
      "LC_ALL".to_string(),
      "CARGO_HOME".to_string(),
      "RUSTUP_HOME".to_string(),
      "XDG_CACHE_HOME".to_string(),
      "XDG_CONFIG_HOME".to_string(),
      "XDG_DATA_HOME".to_string(),
      "TMPDIR".to_string(),
    ],
  }
}

/// Write a nono profile to a JSON file.
pub fn write_profile(profile: &NonoProfile, path: &Path) -> std::io::Result<()> {
  let json = serde_json::to_string_pretty(profile).expect("failed to serialize nono profile");
  fs::write(path, json)
}

/// Check if nono is installed. If not, either skip (local dev) or panic (CI).
///
/// Returns `true` if nono is available, `false` if not (and not required).
/// Panics if `APMW_TEST_SANDBOX_REQUIRED=1` and nono is not installed.
pub fn ensure_nono_or_skip() -> bool {
  let nono_available = which::which("nono").is_ok();
  if nono_available {
    return true;
  }
  let required = std::env::var("APMW_TEST_SANDBOX_REQUIRED")
    .map(|v| v == "1")
    .unwrap_or(false);
  if required {
    panic!(
      "nono is not installed but APMW_TEST_SANDBOX_REQUIRED=1. \
       Install with: brew install nono"
    );
  }
  eprintln!(
    "warning: nono not installed — running unsandboxed. \
     Install with: brew install nono"
  );
  false
}

/// Create a deterministic TempDir under `/tmp/apmw-tests/<test_name>/`.
///
/// Cleans up any existing dir at that path first. On failure, falls back to
/// [`TempDir::new`].
pub fn sandbox_tempdir(test_name: &str) -> TempDir {
  let base = PathBuf::from("/tmp/apmw-tests");
  let _ = fs::create_dir_all(&base);
  let dir = base.join(test_name);
  // Clean up any leftover dir from a prior run.
  let _ = fs::remove_dir_all(&dir);
  // Use rand_bytes(0) so the dir name is exactly <test_name> (no random
  // suffix), giving deterministic, greppable paths.
  match tempfile::Builder::new()
    .prefix(test_name)
    .rand_bytes(0)
    .tempdir_in(&base)
  {
    Ok(td) => td,
    Err(_) => TempDir::new().expect("failed to create temp dir"),
  }
}

/// Create a sandboxed [`Command`] for an existing TempDir.
///
/// Use this when the test needs to write files into the TempDir before running
/// the command. The TempDir must have been created via [`sandbox_tempdir`] (or
/// be a path the nono profile already covers).
pub fn sandboxed_command_in(tempdir: &TempDir, test_name: &str) -> Command {
  if !ensure_nono_or_skip() {
    let mut cmd = Command::cargo_bin("apmw").expect("failed to find apmw binary");
    cmd.current_dir(tempdir.path());
    return cmd;
  }
  let profile = default_profile(tempdir.path());
  let profile_path = tempdir.path().join("nono-profile.json");
  write_profile(&profile, &profile_path)
    .unwrap_or_else(|e| panic!("failed to write nono profile: {e}"));
  let apmw_bin = env!("CARGO_BIN_EXE_apmw");
  let mut cmd = Command::new("nono");
  cmd
    .arg("run")
    .arg("--profile")
    .arg(&profile_path)
    .arg("--")
    .arg(apmw_bin);
  cmd.current_dir(tempdir.path());
  let _ = test_name; // reserved for future per-test profile tuning
  cmd
}

/// Create a sandboxed [`Command`] that runs apmw through nono.
///
/// Returns `(TempDir, Command)` where the TempDir is the sandboxed working dir
/// and the Command invokes `nono run --profile <profile> -- <apmw-binary>`.
///
/// If nono is not installed and not required, falls back to a bare
/// `Command::cargo_bin("apmw")` with a TempDir.
pub fn sandboxed_command(test_name: &str) -> (TempDir, Command) {
  let tempdir = sandbox_tempdir(test_name);

  if !ensure_nono_or_skip() {
    // Fallback: unsandboxed
    let mut cmd = Command::cargo_bin("apmw").expect("failed to find apmw binary");
    cmd.current_dir(tempdir.path());
    return (tempdir, cmd);
  }

  // Generate per-test nono profile
  let profile = default_profile(tempdir.path());
  let profile_path = tempdir.path().join("nono-profile.json");
  write_profile(&profile, &profile_path)
    .unwrap_or_else(|e| panic!("failed to write nono profile: {e}"));

  // Resolve apmw binary path
  let apmw_bin = env!("CARGO_BIN_EXE_apmw");

  // Build nono command
  let mut cmd = Command::new("nono");
  cmd
    .arg("run")
    .arg("--profile")
    .arg(&profile_path)
    .arg("--")
    .arg(apmw_bin);
  cmd.current_dir(tempdir.path());

  (tempdir, cmd)
}
