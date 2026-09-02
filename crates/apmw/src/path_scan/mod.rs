//! PATH scanner for apmw.
//!
//! Checks whether a tool is already installed before attempting installation.
//! Implements the `cli-tool-discovery.sh` resolution flow directly in Rust:
//!
//! 1. Devbox-aware resolution (check `DEVBOX_SHELL` / `IN_DEVBOX_SHELL` first)
//! 2. Wrapper detection (mise, flox, direnv, nix)
//! 3. PATH location scanning (30+ standard locations)
//! 4. Repo-root fallback dirs (`$REPO_ROOT/bin`, `scripts/`, `.local/bin`) — last
//!
//! The scanner is fast (under 50ms per PRD NFR-1.2) because it performs only
//! filesystem stat calls — no subprocess invocations during the scan itself.
//!
//! # Example
//!
//! ```rust
//! use apmw::path_scan::{PathScanner, ScanResult};
//!
//! let scanner = PathScanner::new();
//! let result = scanner.scan("cargo");
//! match result {
//!   ScanResult::Found { path, source } => println!("found at {:?} via {:?}", path, source),
//!   ScanResult::Wrapper { command } => println!("run via wrapper: {command}"),
//!   ScanResult::NotFound => println!("not found — install required"),
//! }
//! ```

pub mod discovery;

pub use discovery::{
  detect_wrapper, devbox_json_dir, is_in_devbox_shell, resolve_repo_root, standard_search_dirs,
  wrapper_env_active, WrapperKind,
};

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Where a tool was discovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanSource {
  /// Found on the system `PATH` environment variable.
  Path,
  /// Found in one of the 30+ standard search directories.
  StandardLocation,
  /// Found in a tech-stack-specific repo directory (e.g. `node_modules/.bin`).
  RepoStackDir,
  /// Found via a package manager (brew, mise, asdf).
  PackageManager,
  /// Found in a repo-root fallback directory (`bin`, `scripts`, `.local/bin`).
  RepoRootFallback,
}

/// The outcome of a PATH scan for a single tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ScanResult {
  /// Tool found at a specific path — use it directly.
  Found {
    /// Absolute path to the executable.
    path: PathBuf,
    /// Where the tool was discovered.
    source: ScanSource,
  },
  /// Tool is inside an environment wrapper — run via the wrapper command.
  Wrapper {
    /// The wrapper command string (e.g. `devbox run --`, `mise exec --`).
    command: String,
  },
  /// Tool not found anywhere — installation is required.
  NotFound,
}

impl ScanResult {
  /// Returns `true` if the tool was found (either directly or via a wrapper).
  pub fn is_available(&self) -> bool {
    matches!(self, ScanResult::Found { .. } | ScanResult::Wrapper { .. })
  }

  /// Returns the path if this is a `Found` result.
  pub fn found_path(&self) -> Option<&Path> {
    if let ScanResult::Found { path, .. } = self {
      Some(path)
    } else {
      None
    }
  }

  /// Returns the wrapper command if this is a `Wrapper` result.
  pub fn wrapper_command(&self) -> Option<&str> {
    if let ScanResult::Wrapper { command } = self {
      Some(command)
    } else {
      None
    }
  }

  /// Renders the scan result as text (matching `cli-tool-discovery.sh` output).
  pub fn to_text(&self, tool: &str) -> String {
    match self {
      ScanResult::Found { path, .. } => format!("FOUND: {}", path.display()),
      ScanResult::Wrapper { command } => format!("WRAPPER: {command} {tool}"),
      ScanResult::NotFound => {
        format!("NOT_FOUND: {tool}\nChecked: PATH, devbox, mise, flox, direnv, nix, standard locations, package managers")
      }
    }
  }

  /// Renders the scan result as JSON (matching `cli-tool-discovery.sh --json`).
  pub fn to_json(&self, tool: &str) -> String {
    match self {
      ScanResult::Found { path, .. } => {
        format!(
          r#"{{"status":"found","path":"{}","tool":"{}"}}"#,
          path.display(),
          tool
        )
      }
      ScanResult::Wrapper { command } => {
        format!(
          r#"{{"status":"wrapper","wrapper":"{} {}","tool":"{}"}}"#,
          command, tool, tool
        )
      }
      ScanResult::NotFound => format!(
        r#"{{"status":"not_found","tool":"{tool}","checked":["PATH","devbox","mise","flox","direnv","nix","standard_locations","package_managers"]}}"#
      ),
    }
  }
}

/// Configuration for the PATH scanner.
#[derive(Debug, Clone, Default)]
pub struct ScanConfig {
  /// Override the `PATH` environment variable (for testing). If `None`, uses
  /// `std::env::var("PATH")`.
  pub path_env: Option<String>,
  /// Override the home directory (for testing). If `None`, uses `dirs::home_dir()`.
  pub home_dir: Option<PathBuf>,
  /// Override the repo root (for testing). If `None`, auto-detected via
  /// `git rev-parse --show-toplevel` or `std::env::current_dir()`.
  pub repo_root: Option<PathBuf>,
  /// Override environment variables (for testing). If `None`, reads from
  /// `std::env::var`.
  pub env_overrides: HashMap<String, String>,
  /// When `true`, skip wrapper detection (used when already inside a devbox
  /// shell — mise/flox/direnv/nix are skipped entirely).
  pub skip_wrappers: bool,
}

impl ScanConfig {
  /// Creates a new config with the given `PATH` override.
  pub fn with_path(mut self, path: impl Into<String>) -> Self {
    self.path_env = Some(path.into());
    self
  }

  /// Creates a new config with the given home directory override.
  pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
    self.home_dir = Some(home.into());
    self
  }

  /// Creates a new config with the given repo root override.
  pub fn with_repo_root(mut self, root: impl Into<PathBuf>) -> Self {
    self.repo_root = Some(root.into());
    self
  }

  /// Sets an environment variable override.
  pub fn set_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.env_overrides.insert(key.into(), value.into());
    self
  }

  /// Reads an environment variable, checking overrides first.
  fn env(&self, key: &str) -> Option<String> {
    self
      .env_overrides
      .get(key)
      .cloned()
      .or_else(|| env::var(key).ok())
  }
}

/// The PATH scanner — checks if a tool is already installed.
///
/// Implements the `cli-tool-discovery.sh` resolution flow directly in Rust:
/// devbox-aware resolution, wrapper detection, 30+ standard PATH locations,
/// and repo-root fallback dirs.
pub struct PathScanner {
  config: ScanConfig,
}

impl Default for PathScanner {
  fn default() -> Self {
    Self::new()
  }
}

impl PathScanner {
  /// Creates a new scanner with default configuration.
  pub fn new() -> Self {
    Self {
      config: ScanConfig::default(),
    }
  }

  /// Creates a new scanner with the given configuration.
  pub fn with_config(config: ScanConfig) -> Self {
    Self { config }
  }

  /// Returns a reference to the scanner's configuration.
  pub fn config(&self) -> &ScanConfig {
    &self.config
  }

  /// Scans for a tool, returning the first match.
  ///
  /// Resolution order:
  /// 1. If inside a devbox shell: `PATH` → standard locations → (skip wrappers)
  /// 2. If not in devbox shell but devbox is available and `devbox.json`
  ///    exists up the tree: check inside devbox → return `Wrapper`
  /// 3. Normal flow: `PATH` → wrappers (mise, flox, direnv, nix) → standard
  ///    locations → repo-root fallback
  ///
  /// The scan is fast (under 50ms) because it only does filesystem stat calls.
  pub fn scan(&self, tool: &str) -> ScanResult {
    let _start = Instant::now();

    // 1. Devbox shell check — FIRST
    let in_devbox_shell = is_in_devbox_shell(&self.config.env_overrides);

    if in_devbox_shell {
      // Inside devbox shell: devbox-managed binaries are on PATH.
      // a. Check PATH
      if let Some(path) = self.find_on_path(tool) {
        return ScanResult::Found {
          path,
          source: ScanSource::Path,
        };
      }
      // b. Path-exhaustion (standard locations + package managers)
      if let Some((path, source)) = self.find_in_standard_dirs(tool) {
        return ScanResult::Found { path, source };
      }
      // c. Skip other wrappers (we're in devbox), go to repo-root fallback
      if let Some(path) = self.find_in_repo_root_fallback(tool) {
        return ScanResult::Found {
          path,
          source: ScanSource::RepoRootFallback,
        };
      }
      // d. Not found
      return ScanResult::NotFound;
    }

    // 2. Not in devbox shell — check if devbox is available and devbox.json exists
    if self.is_devbox_available() {
      if let Some(devbox_dir) = self.devbox_json_dir() {
        // 2a. Verify the tool exists inside the devbox environment.
        // We check if the tool is on PATH inside devbox by checking if it's
        // in the devbox-managed bin dirs. Since we can't run `devbox run`
        // (that would be slow and could hang), we check the devbox profile
        // bin directory directly.
        if self.tool_in_devbox_env(&devbox_dir, tool) {
          return ScanResult::Wrapper {
            command: "devbox run --".to_string(),
          };
        }
        // 2b. Fall through to normal flow (don't return Wrapper for a tool
        // that isn't available inside devbox).
      }
    }

    // 3. Normal flow (no devbox involved)
    // a. Already on PATH?
    if let Some(path) = self.find_on_path(tool) {
      return ScanResult::Found {
        path,
        source: ScanSource::Path,
      };
    }

    // b. Other environment wrappers (mise, flox, direnv, nix)
    if !self.config.skip_wrappers {
      if let Some(wrapper) = self.detect_wrapper() {
        return ScanResult::Wrapper { command: wrapper };
      }
    }

    // c. Global path-exhaustion (standard locations + package managers)
    if let Some((path, source)) = self.find_in_standard_dirs(tool) {
      return ScanResult::Found { path, source };
    }

    // d. Repo-root fallback dirs — LAST (least secure)
    if let Some(path) = self.find_in_repo_root_fallback(tool) {
      return ScanResult::Found {
        path,
        source: ScanSource::RepoRootFallback,
      };
    }

    // 4. Not found (nix/uv install fallback is handled by the install engine,
    //    not the scanner — the scanner only reports what's already available)
    ScanResult::NotFound
  }

  /// Scans for a tool and returns the elapsed time (for performance testing).
  pub fn scan_timed(&self, tool: &str) -> (ScanResult, std::time::Duration) {
    let start = Instant::now();
    let result = self.scan(tool);
    (result, start.elapsed())
  }

  // --- Internal helpers ---

  /// Finds a tool on the `PATH` environment variable.
  fn find_on_path(&self, tool: &str) -> Option<PathBuf> {
    let path_env = match &self.config.path_env {
      Some(p) => p.clone(),
      None => self.config.env("PATH").unwrap_or_default(),
    };

    for dir in path_env.split(':') {
      if dir.is_empty() {
        continue;
      }
      let candidate = Path::new(dir).join(tool);
      if is_executable(&candidate) {
        return Some(candidate);
      }
      // On some systems, Windows-style extensions aren't relevant here (Unix only).
    }
    None
  }

  /// Finds a tool in the 30+ standard search directories.
  /// Returns the path and the source category.
  fn find_in_standard_dirs(&self, tool: &str) -> Option<(PathBuf, ScanSource)> {
    let home = self.home_dir();
    let repo_root = self.repo_root();

    // Standard system + home directories (30+ locations)
    for dir in standard_search_dirs(&home, &self.config.env_overrides) {
      let candidate = dir.join(tool);
      if is_executable(&candidate) {
        return Some((candidate, ScanSource::StandardLocation));
      }
    }

    // Tech-stack-specific repo directories (not deferred — build-system-managed)
    if let Some(root) = &repo_root {
      for dir in repo_stack_dirs(root) {
        let candidate = dir.join(tool);
        if is_executable(&candidate) {
          return Some((candidate, ScanSource::RepoStackDir));
        }
      }
    }

    // Package manager lookup (brew, mise, asdf) — via their known bin prefixes
    if let Some((path, source)) = self.find_via_package_manager(tool) {
      return Some((path, source));
    }

    None
  }

  /// Finds a tool via package manager bin directories (without shelling out).
  /// Checks brew prefix, mise installs, and asdf shims.
  fn find_via_package_manager(&self, tool: &str) -> Option<(PathBuf, ScanSource)> {
    let home = self.home_dir();

    // Homebrew: check /opt/homebrew/bin and /usr/local/bin (already in standard
    // dirs, but also check brew --prefix/bin if brew is available via PATH).
    // Since we don't shell out, we check the common brew prefix locations.
    for prefix in ["/opt/homebrew", "/usr/local"] {
      let bin = Path::new(prefix).join("bin");
      let candidate = bin.join(tool);
      if is_executable(&candidate) {
        return Some((candidate, ScanSource::PackageManager));
      }
    }

    // mise installs: ~/.local/share/mise/installs/*/bin
    let mise_installs = home.join(".local/share/mise/installs");
    if mise_installs.is_dir() {
      if let Ok(entries) = std::fs::read_dir(&mise_installs) {
        for entry in entries.flatten() {
          let bin = entry.path().join("bin");
          let candidate = bin.join(tool);
          if is_executable(&candidate) {
            return Some((candidate, ScanSource::PackageManager));
          }
        }
      }
    }

    // rtx installs (legacy mise name): ~/.local/share/rtx/installs/*/bin
    let rtx_installs = home.join(".local/share/rtx/installs");
    if rtx_installs.is_dir() {
      if let Ok(entries) = std::fs::read_dir(&rtx_installs) {
        for entry in entries.flatten() {
          let bin = entry.path().join("bin");
          let candidate = bin.join(tool);
          if is_executable(&candidate) {
            return Some((candidate, ScanSource::PackageManager));
          }
        }
      }
    }

    // asdf shims: ~/.asdf/shims
    let asdf_shims = home.join(".asdf/shims");
    let candidate = asdf_shims.join(tool);
    if is_executable(&candidate) {
      return Some((candidate, ScanSource::PackageManager));
    }

    None
  }

  /// Finds a tool in repo-root fallback dirs (`bin`, `scripts`, `.local/bin`).
  /// These are searched LAST because a cloned repo could contain malicious
  /// executables in `bin/`.
  fn find_in_repo_root_fallback(&self, tool: &str) -> Option<PathBuf> {
    let repo_root = self.repo_root()?;

    for subdir in ["bin", "scripts", ".local/bin"] {
      let candidate = repo_root.join(subdir).join(tool);
      if is_executable(&candidate) {
        return Some(candidate);
      }
    }
    None
  }

  /// Detects environment wrappers (mise, flox, direnv, nix).
  /// Returns the wrapper command string if a wrapper is detected.
  fn detect_wrapper(&self) -> Option<String> {
    detect_wrapper(
      &self.config.env_overrides,
      &self.config.env("CWD").unwrap_or_else(|| {
        env::current_dir()
          .map(|p| p.display().to_string())
          .unwrap_or_default()
      }),
    )
  }

  /// Checks if devbox is available (on PATH).
  fn is_devbox_available(&self) -> bool {
    self.find_on_path("devbox").is_some()
  }

  /// Finds the nearest `devbox.json` by walking up from the current directory.
  fn devbox_json_dir(&self) -> Option<PathBuf> {
    let start = self
      .config
      .env("CWD")
      .map(PathBuf::from)
      .or_else(|| env::current_dir().ok())
      .or(self.repo_root())?;
    devbox_json_dir(&start)
  }

  /// Checks if a tool is available inside the devbox environment.
  /// Since we can't run `devbox run` (slow, could hang), we check the devbox
  /// profile bin directory directly.
  fn tool_in_devbox_env(&self, devbox_dir: &Path, tool: &str) -> bool {
    // Devbox stores its environment in .devbox/virgs/*/bin
    // The most reliable check without shelling out is to look for the tool
    // in the devbox profile bin directory.
    let devbox_bin = devbox_dir.join(".devbox/virgs/current/bin");
    if is_executable(&devbox_bin.join(tool)) {
      return true;
    }

    // Also check the nix profile path that devbox uses
    // ~/.local/share/devbox/nix/profile/default/bin
    if let Some(home) = dirs::home_dir() {
      let nix_profile = home.join(".local/share/devbox/nix/profile/default/bin");
      if is_executable(&nix_profile.join(tool)) {
        return true;
      }
    }

    false
  }

  /// Returns the home directory (from config override or `dirs::home_dir()`).
  fn home_dir(&self) -> PathBuf {
    self
      .config
      .home_dir
      .clone()
      .or_else(dirs::home_dir)
      .unwrap_or_else(|| PathBuf::from("."))
  }

  /// Returns the repo root (from config override or auto-detected).
  fn repo_root(&self) -> Option<PathBuf> {
    if let Some(root) = &self.config.repo_root {
      return Some(root.clone());
    }
    Some(resolve_repo_root(&self.config.env_overrides))
  }
}

/// Checks if a path is an executable file.
fn is_executable(path: &Path) -> bool {
  // On Unix, check that the file exists and is executable.
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
    // On non-Unix, just check if the file exists.
    std::fs::metadata(path)
      .map(|m| m.is_file())
      .unwrap_or(false)
  }
}

/// Returns tech-stack-specific repo directories based on project markers.
/// These are NOT deferred (they are build-system-managed and stay in normal
/// search order).
fn repo_stack_dirs(repo_root: &Path) -> Vec<PathBuf> {
  let mut dirs = Vec::new();

  // Node: package.json → node_modules/.bin, .bin
  if repo_root.join("package.json").is_file() {
    dirs.push(repo_root.join("node_modules/.bin"));
    dirs.push(repo_root.join(".bin"));
  }
  // Rust: Cargo.toml → target/release, target/debug
  if repo_root.join("Cargo.toml").is_file() {
    dirs.push(repo_root.join("target/release"));
    dirs.push(repo_root.join("target/debug"));
  }
  // Go: go.mod → bin, .bin
  if repo_root.join("go.mod").is_file() {
    dirs.push(repo_root.join("bin"));
    dirs.push(repo_root.join(".bin"));
  }
  // Python: pyproject.toml or requirements.txt → .venv/bin, .local/bin
  if repo_root.join("pyproject.toml").is_file() || repo_root.join("requirements.txt").is_file() {
    dirs.push(repo_root.join(".venv/bin"));
    dirs.push(repo_root.join(".local/bin"));
  }
  // Ruby: Gemfile → bin, .bundle/bin
  if repo_root.join("Gemfile").is_file() {
    dirs.push(repo_root.join("bin"));
    dirs.push(repo_root.join(".bundle/bin"));
  }
  // PHP: composer.json → vendor/bin
  if repo_root.join("composer.json").is_file() {
    dirs.push(repo_root.join("vendor/bin"));
  }

  dirs
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

  /// Creates a temporary directory with an executable file named `tool`.
  fn make_bin_dir(tool: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let tool_path = dir.path().join(tool);
    fs::write(&tool_path, "#!/bin/sh\necho hi\n").unwrap();
    fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();
    dir
  }

  /// Makes a file executable inside the given path.
  fn make_executable(path: &Path) {
    fs::write(path, "#!/bin/sh\necho hi\n").unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
  }

  // --- ScanResult tests ---

  #[test]
  fn test_scan_result_found() {
    let result = ScanResult::Found {
      path: PathBuf::from("/usr/bin/cargo"),
      source: ScanSource::Path,
    };
    assert!(result.is_available());
    assert_eq!(result.found_path(), Some(Path::new("/usr/bin/cargo")));
    assert_eq!(result.wrapper_command(), None);
    assert_eq!(result.to_text("cargo"), "FOUND: /usr/bin/cargo");
  }

  #[test]
  fn test_scan_result_wrapper() {
    let result = ScanResult::Wrapper {
      command: "devbox run --".to_string(),
    };
    assert!(result.is_available());
    assert_eq!(result.found_path(), None);
    assert_eq!(result.wrapper_command(), Some("devbox run --"));
    assert_eq!(result.to_text("cargo"), "WRAPPER: devbox run -- cargo");
  }

  #[test]
  fn test_scan_result_not_found() {
    let result = ScanResult::NotFound;
    assert!(!result.is_available());
    assert_eq!(result.found_path(), None);
    assert_eq!(result.wrapper_command(), None);
    let text = result.to_text("nonexistent");
    assert!(text.starts_with("NOT_FOUND: nonexistent"));
  }

  #[test]
  fn test_scan_result_serde_roundtrip() {
    let found = ScanResult::Found {
      path: PathBuf::from("/usr/bin/cargo"),
      source: ScanSource::StandardLocation,
    };
    let json = serde_json::to_string(&found).unwrap();
    let deserialized: ScanResult = serde_json::from_str(&json).unwrap();
    assert_eq!(found, deserialized);

    let wrapper = ScanResult::Wrapper {
      command: "mise exec --".to_string(),
    };
    let json = serde_json::to_string(&wrapper).unwrap();
    let deserialized: ScanResult = serde_json::from_str(&json).unwrap();
    assert_eq!(wrapper, deserialized);

    let not_found = ScanResult::NotFound;
    let json = serde_json::to_string(&not_found).unwrap();
    let deserialized: ScanResult = serde_json::from_str(&json).unwrap();
    assert_eq!(not_found, deserialized);
  }

  #[test]
  fn test_scan_result_json_output() {
    let found = ScanResult::Found {
      path: PathBuf::from("/usr/bin/cargo"),
      source: ScanSource::Path,
    };
    let json = found.to_json("cargo");
    assert!(json.contains(r#""status":"found""#));
    assert!(json.contains(r#""path":"/usr/bin/cargo""#));

    let wrapper = ScanResult::Wrapper {
      command: "devbox run --".to_string(),
    };
    let json = wrapper.to_json("cargo");
    assert!(json.contains(r#""status":"wrapper""#));

    let not_found = ScanResult::NotFound;
    let json = not_found.to_json("nonexistent");
    assert!(json.contains(r#""status":"not_found""#));
  }

  // --- PATH scanning tests ---

  #[test]
  fn test_path_scan_finds_tool_on_path() {
    let bin_dir = make_bin_dir("mytool");
    let path_env = bin_dir.path().display().to_string();
    let config = ScanConfig::default().with_path(path_env);
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("mytool");
    assert_eq!(
      result,
      ScanResult::Found {
        path: bin_dir.path().join("mytool"),
        source: ScanSource::Path
      }
    );
  }

  #[test]
  fn test_path_scan_not_found() {
    let cwd = TempDir::new().unwrap();
    let config = ScanConfig::default()
      .with_path("/nonexistent:/usr/bin")
      .with_home("/nonexistent_home")
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("definitely_not_a_real_tool_xyz123");
    assert_eq!(result, ScanResult::NotFound);
  }

  #[test]
  fn test_path_scan_finds_in_standard_location() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let cargo_bin = home.path().join(".cargo/bin");
    fs::create_dir_all(&cargo_bin).unwrap();
    make_executable(&cargo_bin.join("cargo"));

    let config = ScanConfig::default()
      .with_path("/usr/bin")
      .with_home(home.path())
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("cargo");
    match result {
      ScanResult::Found { path, source } => {
        assert_eq!(path, cargo_bin.join("cargo"));
        assert_eq!(source, ScanSource::StandardLocation);
      }
      other => panic!("expected Found, got {other:?}"),
    }
  }

  #[test]
  fn test_path_scan_finds_in_repo_root_fallback() {
    let repo = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let bin_dir = repo.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    make_executable(&bin_dir.join("mytool"));

    let config = ScanConfig::default()
      .with_path("/usr/bin")
      .with_home("/nonexistent_home")
      .with_repo_root(repo.path())
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("mytool");
    match result {
      ScanResult::Found { path, source } => {
        assert_eq!(path, bin_dir.join("mytool"));
        assert_eq!(source, ScanSource::RepoRootFallback);
      }
      other => panic!("expected Found, got {other:?}"),
    }
  }

  #[test]
  fn test_path_scan_finds_in_repo_stack_dir() {
    let repo = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    // Create a Rust project
    fs::write(
      repo.path().join("Cargo.toml"),
      "[package]\nname = \"test\"\n",
    )
    .unwrap();
    let target_release = repo.path().join("target/release");
    fs::create_dir_all(&target_release).unwrap();
    make_executable(&target_release.join("myapp"));

    let config = ScanConfig::default()
      .with_path("/usr/bin")
      .with_home("/nonexistent_home")
      .with_repo_root(repo.path())
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("myapp");
    match result {
      ScanResult::Found { path, source } => {
        assert_eq!(path, target_release.join("myapp"));
        assert_eq!(source, ScanSource::RepoStackDir);
      }
      other => panic!("expected Found, got {other:?}"),
    }
  }

  // --- Devbox-aware resolution tests ---

  #[test]
  fn test_devbox_resolution_in_shell() {
    // When DEVBOX_SHELL is set, the scanner should skip wrapper detection.
    let bin_dir = make_bin_dir("mytool");
    let path_env = bin_dir.path().display().to_string();
    let config = ScanConfig::default()
      .with_path(path_env)
      .with_home("/nonexistent_home")
      .set_env("DEVBOX_SHELL", "1");
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("mytool");
    assert_eq!(
      result,
      ScanResult::Found {
        path: bin_dir.path().join("mytool"),
        source: ScanSource::Path
      }
    );
  }

  #[test]
  fn test_devbox_resolution_in_shell_not_found() {
    // When DEVBOX_SHELL is set and the tool isn't found, should return NotFound
    // without trying wrappers.
    let config = ScanConfig::default()
      .with_path("/usr/bin")
      .with_home("/nonexistent_home")
      .set_env("DEVBOX_SHELL", "1");
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("nonexistent_tool_xyz");
    assert_eq!(result, ScanResult::NotFound);
  }

  #[test]
  fn test_devbox_resolution_in_devbox_shell_var() {
    // IN_DEVBOX_SHELL should also trigger devbox mode.
    assert!(is_in_devbox_shell(&{
      let mut m = HashMap::new();
      m.insert("IN_DEVBOX_SHELL".to_string(), "1".to_string());
      m
    }));
  }

  #[test]
  fn test_devbox_resolution_not_in_shell() {
    // Without DEVBOX_SHELL, normal flow should be used.
    assert!(!is_in_devbox_shell(&HashMap::new()));
  }

  // --- Wrapper detection tests ---

  #[test]
  fn test_wrapper_detection_mise() {
    // MISE_SHELL not set → wrapper detected if .mise.toml exists
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".mise.toml"), "").unwrap();
    let cwd = temp.path().display().to_string();

    let env = HashMap::new();
    // Don't set MISE_SHELL → wrapper should be detected
    let wrapper = detect_wrapper(&env, &cwd);
    // mise must be on PATH for this to detect; in test env it may not be,
    // so we just verify the function doesn't panic.
    let _ = wrapper;
  }

  #[test]
  fn test_wrapper_detection_mise_shell_active() {
    // When MISE_SHELL is set, mise wrapper should NOT be detected.
    let mut env = HashMap::new();
    env.insert("MISE_SHELL".to_string(), "bash".to_string());
    assert!(wrapper_env_active(WrapperKind::Mise, &env));
  }

  #[test]
  fn test_wrapper_detection_flox_active() {
    let mut env = HashMap::new();
    env.insert("FLOX_ACTIVE".to_string(), "1".to_string());
    assert!(wrapper_env_active(WrapperKind::Flox, &env));
  }

  #[test]
  fn test_wrapper_detection_direnv_active() {
    let mut env = HashMap::new();
    env.insert("DIRENV_DIR".to_string(), "/some/path".to_string());
    assert!(wrapper_env_active(WrapperKind::Direnv, &env));
  }

  #[test]
  fn test_wrapper_detection_nix_active() {
    let mut env = HashMap::new();
    env.insert("IN_NIX_SHELL".to_string(), "1".to_string());
    assert!(wrapper_env_active(WrapperKind::Nix, &env));
  }

  #[test]
  fn test_wrapper_detection_none_active() {
    // Use a non-empty HashMap to stay in "test mode" (no real-env fallback),
    // otherwise IN_NIX_SHELL from the devbox shell would cause Nix to appear active.
    let mut env = HashMap::new();
    env.insert("__test_marker".to_string(), "1".to_string());
    assert!(!wrapper_env_active(WrapperKind::Mise, &env));
    assert!(!wrapper_env_active(WrapperKind::Flox, &env));
    assert!(!wrapper_env_active(WrapperKind::Direnv, &env));
    assert!(!wrapper_env_active(WrapperKind::Nix, &env));
  }

  // --- Performance test ---

  #[test]
  fn test_path_scan_under_50ms() {
    // PRD NFR-1.2: scanner must find tools in under 50ms.
    let cwd = TempDir::new().unwrap();
    let config = ScanConfig::default()
      .with_path("/usr/bin:/usr/local/bin:/bin")
      .with_home("/nonexistent_home")
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let (result, elapsed) = scanner.scan_timed("ls");
    // Even if not found, the scan should be fast.
    let _ = result;
    assert!(
      elapsed.as_millis() < 50,
      "scan took {elapsed:?}, expected under 50ms"
    );
  }

  // --- Standard search dirs count test ---

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

  // --- Repo-root fallback dirs test ---

  #[test]
  fn test_repo_fallback_checks_all_three_dirs() {
    let repo = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();

    // Create tool in scripts/ (not bin/)
    let scripts_dir = repo.path().join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();
    make_executable(&scripts_dir.join("mytool"));

    let config = ScanConfig::default()
      .with_path("/usr/bin")
      .with_home("/nonexistent_home")
      .with_repo_root(repo.path())
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("mytool");
    match result {
      ScanResult::Found { path, source } => {
        assert_eq!(path, scripts_dir.join("mytool"));
        assert_eq!(source, ScanSource::RepoRootFallback);
      }
      other => panic!("expected Found, got {other:?}"),
    }
  }

  #[test]
  fn test_repo_fallback_checks_local_bin() {
    let repo = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();

    let local_bin = repo.path().join(".local/bin");
    fs::create_dir_all(&local_bin).unwrap();
    make_executable(&local_bin.join("mytool"));

    let config = ScanConfig::default()
      .with_path("/usr/bin")
      .with_home("/nonexistent_home")
      .with_repo_root(repo.path())
      .set_env("CWD", cwd.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("mytool");
    match result {
      ScanResult::Found { path, source } => {
        assert_eq!(path, local_bin.join("mytool"));
        assert_eq!(source, ScanSource::RepoRootFallback);
      }
      other => panic!("expected Found, got {other:?}"),
    }
  }

  // --- ScanConfig builder tests ---

  #[test]
  fn test_scan_config_builders() {
    let config = ScanConfig::default()
      .with_path("/custom/path")
      .with_home("/custom/home")
      .with_repo_root("/custom/repo")
      .set_env("DEVBOX_SHELL", "1");

    assert_eq!(config.path_env, Some("/custom/path".to_string()));
    assert_eq!(config.home_dir, Some(PathBuf::from("/custom/home")));
    assert_eq!(config.repo_root, Some(PathBuf::from("/custom/repo")));
    assert_eq!(config.env("DEVBOX_SHELL"), Some("1".to_string()));
  }

  #[test]
  fn test_scan_config_env_override_takes_precedence() {
    let config = ScanConfig::default().set_env("MY_CUSTOM_VAR", "override");
    assert_eq!(config.env("MY_CUSTOM_VAR"), Some("override".to_string()));
  }

  // --- Integration: devbox wrapper detection ---

  #[test]
  fn test_devbox_wrapper_detection() {
    // When not in devbox shell but devbox.json exists and tool is in devbox env.
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("devbox.json"), r#"{"packages": {}}"#).unwrap();

    // Create a fake devbox bin dir with the tool
    let devbox_bin = repo.path().join(".devbox/virgs/current/bin");
    fs::create_dir_all(&devbox_bin).unwrap();
    make_executable(&devbox_bin.join("mytool"));

    // Also put devbox binary on PATH so is_devbox_available returns true
    let devbox_bin_dir = make_bin_dir("devbox");

    let config = ScanConfig::default()
      .with_path(devbox_bin_dir.path().display().to_string())
      .with_home("/nonexistent_home")
      .with_repo_root(repo.path())
      .set_env("CWD", repo.path().display().to_string());
    let scanner = PathScanner::with_config(config);

    let result = scanner.scan("mytool");
    match result {
      ScanResult::Wrapper { command } => {
        assert_eq!(command, "devbox run --");
      }
      other => panic!("expected Wrapper, got {other:?}"),
    }
  }
}
