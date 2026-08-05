//! Integration tests for apmw.
//!
//! Every test invokes the apmw binary through the sandbox harness in
//! `tests/sandbox/mod.rs` (`sandboxed_command` / `sandboxed_command_in`) so
//! that, when nono is installed, subprocesses run with restricted filesystem
//! and network access. When nono is absent the harness falls back to a direct
//! invocation of the cargo-built apmw binary, so the tests remain green on any
//! dev machine.

#[path = "sandbox/mod.rs"]
mod sandbox;

use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_version_flag() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("version-flag");
  cmd
    .arg("--version")
    .assert()
    .success()
    .stdout(predicates::str::contains("apmw"));
}

#[test]
fn test_help_flag() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("help-flag");
  cmd
    .arg("--help")
    .assert()
    .success()
    .stdout(predicates::str::contains(
      "Abstracts every package installer",
    ));
}

#[test]
fn test_no_args_prints_version() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("no-args");
  cmd
    .assert()
    .success()
    .stdout(predicates::str::contains("apmw"));
}

#[test]
fn test_list_jobs_no_daemon() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("list-jobs-no-daemon");
  cmd
    .arg("--no-daemon")
    .arg("--list-jobs")
    .assert()
    .success()
    .stdout(predicates::str::contains("--no-daemon mode"));
}

#[test]
fn test_list_jobs_daemon_not_running() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("list-jobs-daemon");
  cmd
    .arg("--list-jobs")
    .assert()
    .success()
    .stdout(predicates::str::contains("Daemon is not running"));
}

#[test]
fn test_cancel_job_no_daemon_errors() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("cancel-job-no-daemon");
  cmd
    .arg("--no-daemon")
    .arg("--cancel-job")
    .arg("abc123")
    .assert()
    .failure()
    .stderr(predicates::str::contains("--no-daemon"));
}

#[test]
fn test_cancel_job_daemon_not_running_errors() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("cancel-job-daemon");
  cmd
    .arg("--cancel-job")
    .arg("abc123")
    .assert()
    .failure()
    .stderr(predicates::str::contains("daemon is not running"));
}

#[test]
fn test_no_daemon_clone_errors() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("no-daemon-clone");
  cmd
    .arg("--no-daemon")
    .arg("clone")
    .arg("https://example.com/repo")
    .assert()
    .failure()
    .stderr(predicates::str::contains("--no-daemon"));
}

// ---------------------------------------------------------------------------
// Detection integration tests
// ---------------------------------------------------------------------------

/// Helper: create a sandbox temp directory, write the given files into it, and
/// return the path. The directory is deterministic for `test_name` so failures
/// leave a greppable path under `/tmp/apmw-tests/<test_name>/`.
fn make_project_dir(test_name: &str, files: &[&str]) -> TempDir {
  let dir = sandbox::sandbox_tempdir(test_name);
  for file in files {
    let path = dir.path().join(file);
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::write(&path, "// test").expect("failed to write file");
  }
  dir
}

#[test]
fn test_detect_cargo_project() {
  let dir = make_project_dir("detect-cargo", &["Cargo.toml", "Cargo.lock"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-cargo");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cargo"));
}

#[test]
fn test_detect_npm_project() {
  let dir = make_project_dir("detect-npm", &["package.json", "package-lock.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-npm");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("npm"));
}

#[test]
fn test_detect_pnpm_project() {
  let dir = make_project_dir("detect-pnpm", &["pnpm-lock.yaml", "package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-pnpm");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("pnpm"));
}

#[test]
fn test_detect_go_project() {
  let dir = make_project_dir("detect-go", &["go.mod", "go.sum"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-go");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("go"));
}

#[test]
fn test_detect_pip_project() {
  let dir = make_project_dir("detect-pip", &["requirements.txt"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-pip");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("pip"));
}

#[test]
fn test_detect_docker_project() {
  let dir = make_project_dir("detect-docker", &["Dockerfile"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-docker");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("docker"));
}

#[test]
fn test_detect_empty_dir() {
  let dir = sandbox::sandbox_tempdir("detect-empty");
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-empty");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("\"total\":0"));
}

#[test]
fn test_detect_human_mode() {
  let dir = make_project_dir("detect-human", &["Cargo.toml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-human");
  cmd.arg("detect").arg("--human").assert().success();
}

#[test]
fn test_detect_multiple_managers() {
  let dir = make_project_dir(
    "detect-multi",
    &["Cargo.toml", "package.json", "package-lock.json"],
  );
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-multi");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cargo"))
    .stdout(contains("npm"));
}

#[test]
fn test_detect_maven_project() {
  let dir = make_project_dir("detect-maven", &["pom.xml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-maven");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("maven"));
}

#[test]
fn test_detect_gradle_project() {
  let dir = make_project_dir("detect-gradle", &["build.gradle"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-gradle");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("gradle"));
}

#[test]
fn test_detect_dotnet_project() {
  let dir = make_project_dir("detect-dotnet", &["MyApp.csproj"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-dotnet");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("dotnet"));
}

#[test]
fn test_detect_helm_project() {
  let dir = make_project_dir("detect-helm", &["Chart.yaml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-helm");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("helm"));
}

#[test]
fn test_detect_poetry_project() {
  let dir = make_project_dir("detect-poetry", &["poetry.lock"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-poetry");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("poetry"));
}

#[test]
fn test_detect_uv_project() {
  let dir = make_project_dir("detect-uv", &["uv.lock"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-uv");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("uv"));
}

#[test]
fn test_detect_gem_project() {
  let dir = make_project_dir("detect-gem", &["Gemfile"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-gem");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("gem"));
}

#[test]
fn test_detect_nix_project() {
  let dir = make_project_dir("detect-nix", &["flake.nix"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-nix");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("nix"));
}

#[test]
fn test_detect_brew_project() {
  let dir = make_project_dir("detect-brew", &["Brewfile"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-brew");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("brew"));
}

#[test]
fn test_detect_cmake_project() {
  let dir = make_project_dir("detect-cmake", &["CMakeLists.txt"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-cmake");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cmake"));
}

#[test]
fn test_detect_compose_yaml() {
  let dir = make_project_dir("detect-compose", &["compose.yaml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "detect-compose");
  cmd
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("docker"));
}

// ---------------------------------------------------------------------------
// Manager override integration tests (story 02-004)
// ---------------------------------------------------------------------------

#[test]
fn test_manager_override_pnpm_skips_detection() {
  // Create a cargo project but force pnpm — detection should be skipped.
  let dir = make_project_dir("override-pnpm", &["Cargo.toml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "override-pnpm");
  cmd
    .arg("--manager")
    .arg("pnpm")
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("pnpm"));
}

#[test]
fn test_manager_override_uv_skips_detection() {
  let dir = make_project_dir("override-uv", &["Cargo.toml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "override-uv");
  cmd
    .arg("--manager")
    .arg("uv")
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("uv"));
}

#[test]
fn test_manager_override_cargo_skips_detection() {
  let dir = make_project_dir("override-cargo", &["package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "override-cargo");
  cmd
    .arg("--manager")
    .arg("cargo")
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cargo"));
}

#[test]
fn test_manager_override_docker_skips_detection() {
  let dir = make_project_dir("override-docker", &["Cargo.toml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "override-docker");
  cmd
    .arg("--manager")
    .arg("docker")
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("docker"));
}

#[test]
fn test_use_alias_works_identically() {
  let dir = make_project_dir("use-alias", &["Cargo.toml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "use-alias");
  cmd
    .arg("--use")
    .arg("pnpm")
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("pnpm"));
}

#[test]
fn test_invalid_manager_name_errors() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("invalid-manager");
  cmd
    .arg("--manager")
    .arg("nonexistent")
    .arg("status")
    .assert()
    .failure()
    .stderr(contains("invalid manager"));
}

#[test]
fn test_invalid_manager_use_alias_errors() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("invalid-use-alias");
  cmd
    .arg("--use")
    .arg("badmgr")
    .arg("status")
    .assert()
    .failure()
    .stderr(contains("invalid manager"));
}

#[test]
fn test_invalid_manager_error_lists_valid_options() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("invalid-manager-list");
  let output = cmd
    .arg("--manager")
    .arg("foobar")
    .arg("status")
    .assert()
    .failure()
    .get_output()
    .clone();
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("pnpm"), "error should list pnpm");
  assert!(stderr.contains("cargo"), "error should list cargo");
  assert!(stderr.contains("docker"), "error should list docker");
  assert!(stderr.contains("uv"), "error should list uv");
}

#[test]
fn test_manager_override_on_install() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("override-install");
  cmd
    .arg("install")
    .arg("express")
    .arg("--manager")
    .arg("pnpm")
    .assert()
    .success()
    .stdout(contains("via pnpm"));
}

#[test]
fn test_manager_use_alias_on_install() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("use-alias-install");
  cmd
    .arg("install")
    .arg("express")
    .arg("--use")
    .arg("npm")
    .assert()
    .success()
    .stdout(contains("via npm"));
}

#[test]
fn test_manager_override_confidence_is_max() {
  let dir = make_project_dir("override-confidence", &["Cargo.toml"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "override-confidence");
  cmd
    .arg("--manager")
    .arg("pnpm")
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("\"confidence\":1.0"));
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Intercept hooks integration tests (story 06-002)
// ---------------------------------------------------------------------------
//
// These tests redirect the intercept shim directory into the sandbox TempDir
// by overriding `HOME` (which `dirs::data_local_dir()` derives from on both
// macOS and Linux). This keeps the real host's `~/Library/Application Support`
// or `~/.local/share` untouched.

#[test]
fn test_install_intercept_creates_shims() {
  let (dir, mut cmd) = sandbox::sandboxed_command("install-intercept");
  cmd
    .env("HOME", dir.path())
    .arg("--install")
    .arg("--intercept")
    .assert()
    .success()
    .stdout(contains("Installed"))
    .stdout(contains("intercept shim"));
  // Verify the shims landed inside the sandbox, not the real host.
  let shims_dir = intercept_shims_dir(dir.path());
  assert!(
    shims_dir.exists(),
    "intercept shims should be written inside the sandbox tempdir at {}",
    shims_dir.display()
  );
}

#[test]
fn test_uninstall_intercept_removes_shims() {
  // Install first, then uninstall, using the same sandbox dir + HOME override.
  let (dir, mut cmd) = sandbox::sandboxed_command("uninstall-intercept");
  cmd
    .env("HOME", dir.path())
    .arg("--install")
    .arg("--intercept")
    .assert()
    .success();

  let mut cmd = sandbox::sandboxed_command_in(&dir, "uninstall-intercept");
  cmd
    .env("HOME", dir.path())
    .arg("--uninstall")
    .arg("--intercept")
    .assert()
    .success()
    .stdout(contains("Removed"));

  let shims_dir = intercept_shims_dir(dir.path());
  let remaining = fs::read_dir(&shims_dir)
    .map(|mut entries| entries.next().is_some())
    .unwrap_or(false);
  assert!(
    !remaining,
    "intercept shims should be removed from {}",
    shims_dir.display()
  );
}

#[test]
fn test_intercept_subcommand_delegates_no_governance() {
  // With no governance rules, intercept should delegate to the original tool.
  let (_dir, mut cmd) = sandbox::sandboxed_command("intercept-delegate");
  cmd
    .arg("intercept")
    .arg("pip")
    .arg("install")
    .arg("foo")
    .assert()
    .success()
    .stdout(contains("pip"));
}

#[test]
fn test_intercept_subcommand_with_version_flag() {
  // --version is a read-only command — should still succeed and delegate.
  let (_dir, mut cmd) = sandbox::sandboxed_command("intercept-version");
  cmd
    .arg("intercept")
    .arg("npm")
    .arg("--version")
    .assert()
    .success()
    .stdout(contains("npm"));
}

/// Resolve the intercept shim directory that apmw will use for the given
/// `HOME` override. Mirrors `default_shim_dir()` in `src/agent/hooks.rs`.
fn intercept_shims_dir(home: &std::path::Path) -> std::path::PathBuf {
  if cfg!(target_os = "macos") {
    home
      .join("Library")
      .join("Application Support")
      .join("apmw")
      .join("shims")
  } else {
    home.join(".local").join("share").join("apmw").join("shims")
  }
}

// ---------------------------------------------------------------------------
// Install (add engine) integration tests (story 04-001)
// ---------------------------------------------------------------------------

#[test]
fn test_install_with_dry_run() {
  let dir = make_project_dir("install-dry-run", &["package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "install-dry-run");
  cmd
    .arg("install")
    .arg("express")
    .arg("--manager")
    .arg("pnpm")
    .arg("--dry-run")
    .arg("--no-scan")
    .assert()
    .success()
    .stdout(contains("via pnpm"));
}

#[test]
fn test_install_dev_flag() {
  let dir = make_project_dir("install-dev", &["package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "install-dev");
  cmd
    .arg("install")
    .arg("express")
    .arg("--dev")
    .arg("--manager")
    .arg("pnpm")
    .arg("--dry-run")
    .arg("--no-scan")
    .assert()
    .success()
    .stdout(contains("(dev)"));
}

#[test]
fn test_install_manager_override_cargo() {
  let dir = make_project_dir("install-override-cargo", &["package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "install-override-cargo");
  cmd
    .arg("install")
    .arg("serde")
    .arg("--manager")
    .arg("cargo")
    .arg("--dry-run")
    .arg("--no-scan")
    .assert()
    .success()
    .stdout(contains("via cargo"));
}

#[test]
fn test_install_dry_run_outputs_toon() {
  let dir = make_project_dir("install-toon", &["package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "install-toon");
  cmd
    .arg("install")
    .arg("express")
    .arg("--manager")
    .arg("pnpm")
    .arg("--dry-run")
    .arg("--no-scan")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("\"item\""));
}

#[test]
fn test_install_no_manager_detected_errors() {
  let dir = sandbox::sandbox_tempdir("install-no-manager");
  let mut cmd = sandbox::sandboxed_command_in(&dir, "install-no-manager");
  cmd
    .arg("install")
    .arg("some-tool")
    .arg("--no-scan")
    .assert()
    .failure();
}

#[test]
fn test_install_path_scan_skips() {
  // "cargo" is on PATH in the test environment — should skip.
  let dir = make_project_dir("install-path-scan", &["package.json"]);
  let mut cmd = sandbox::sandboxed_command_in(&dir, "install-path-scan");
  cmd
    .arg("install")
    .arg("cargo")
    .arg("--manager")
    .arg("pnpm")
    .arg("--no-scan")
    .assert()
    .success()
    .stdout(contains("already"));
}

// ---------------------------------------------------------------------------
// Clone (historyless clone engine) integration tests (story 05-001)
// ---------------------------------------------------------------------------

/// Helper: create a bare git repo in a sandbox temp dir that can be cloned.
fn make_cloneable_repo(test_name: &str) -> TempDir {
  let dir = sandbox::sandbox_tempdir(test_name);
  std::process::Command::new("git")
    .args(["init"])
    .current_dir(dir.path())
    .output()
    .expect("git init");
  std::process::Command::new("git")
    .args(["config", "user.email", "test@test.com"])
    .current_dir(dir.path())
    .output()
    .expect("git config");
  std::process::Command::new("git")
    .args(["config", "user.name", "Test"])
    .current_dir(dir.path())
    .output()
    .expect("git config");
  fs::write(dir.path().join("README.md"), "# test repo").expect("write readme");
  std::process::Command::new("git")
    .args(["add", "."])
    .current_dir(dir.path())
    .output()
    .expect("git add");
  std::process::Command::new("git")
    .args(["commit", "-m", "initial commit"])
    .current_dir(dir.path())
    .output()
    .expect("git commit");
  dir
}

#[test]
fn test_clone_help_shows_subcommand() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("clone-help");
  cmd
    .arg("clone")
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("Repository URL or package name to clone"));
}

#[test]
fn test_clone_no_daemon_errors() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("clone-no-daemon");
  cmd
    .arg("--no-daemon")
    .arg("clone")
    .arg("https://example.com/repo")
    .assert()
    .failure()
    .stderr(contains("--no-daemon"));
}

#[test]
fn test_clone_invalid_repo_fails() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("clone-invalid");
  cmd
    .arg("clone")
    .arg("https://invalid-host-nonexistent-xyz.invalid/repo")
    .assert()
    .failure();
}

#[test]
fn test_clone_local_repo_succeeds() {
  let source = make_cloneable_repo("clone-source-local");
  let (sandbox_dir, mut cmd) = sandbox::sandboxed_command("clone-local");
  let dest = sandbox_dir.path().join("cache");
  fs::create_dir_all(&dest).expect("create cache dir");
  let source_url = format!("file://{}", source.path().display());

  cmd
    .env("XDG_CACHE_HOME", &dest)
    .arg("clone")
    .arg(&source_url)
    .assert()
    .success()
    .stdout(contains("\"repo\""))
    .stdout(contains("\"ast_tool\""));
}

#[test]
fn test_clone_writes_gitignore() {
  let source = make_cloneable_repo("clone-source-gitignore");
  let (sandbox_dir, mut cmd) = sandbox::sandboxed_command("clone-gitignore");
  let dest = sandbox_dir.path().join("cache");
  fs::create_dir_all(&dest).expect("create cache dir");
  let source_url = format!("file://{}", source.path().display());

  cmd
    .env("XDG_CACHE_HOME", &dest)
    .arg("clone")
    .arg(&source_url)
    .assert()
    .success();

  // The .gitignore should be in the cloned repo directory. The repo name
  // is derived from the source path's last segment.
  let clones_dir = dest.join("apmw").join("clones");
  let entries: Vec<_> = fs::read_dir(&clones_dir)
    .expect("clones dir should exist")
    .flatten()
    .collect();
  assert!(
    !entries.is_empty(),
    "clones dir should have at least one entry"
  );
  let cloned_dir = entries[0].path();
  let gitignore = cloned_dir.join(".gitignore");
  assert!(gitignore.exists(), ".gitignore should exist after clone");
  let contents = fs::read_to_string(&gitignore).expect("read gitignore");
  assert!(contents.contains("devbox.json"));
  assert!(contents.contains("AGENTS.md"));
  assert!(contents.contains("*.codegraph"));
}

#[test]
fn test_clone_outputs_toon_format() {
  let source = make_cloneable_repo("clone-source-toon");
  let (sandbox_dir, mut cmd) = sandbox::sandboxed_command("clone-toon");
  let dest = sandbox_dir.path().join("cache");
  fs::create_dir_all(&dest).expect("create cache dir");
  let source_url = format!("file://{}", source.path().display());

  let output = cmd
    .env("XDG_CACHE_HOME", &dest)
    .arg("clone")
    .arg(&source_url)
    .assert()
    .success()
    .get_output()
    .clone();
  let stdout = String::from_utf8_lossy(&output.stdout);
  // TOON format uses = instead of : and no commas.
  assert!(stdout.contains("\"item\""), "stdout: {stdout}");
  assert!(stdout.contains("\"help\""), "stdout: {stdout}");
}

#[test]
fn test_clone_outputs_json_format() {
  let source = make_cloneable_repo("clone-source-json");
  let (sandbox_dir, mut cmd) = sandbox::sandboxed_command("clone-json");
  let dest = sandbox_dir.path().join("cache");
  fs::create_dir_all(&dest).expect("create cache dir");
  let source_url = format!("file://{}", source.path().display());

  cmd
    .env("XDG_CACHE_HOME", &dest)
    .arg("--json")
    .arg("clone")
    .arg(&source_url)
    .assert()
    .success()
    .stdout(contains("\"item\""))
    .stdout(contains("\"ast_tool\""));
}

#[test]
fn test_clone_small_repo_skips_indexing() {
  let source = make_cloneable_repo("clone-source-small");
  let (sandbox_dir, mut cmd) = sandbox::sandboxed_command("clone-small");
  let dest = sandbox_dir.path().join("cache");
  fs::create_dir_all(&dest).expect("create cache dir");
  let source_url = format!("file://{}", source.path().display());

  cmd
    .env("XDG_CACHE_HOME", &dest)
    .arg("--json")
    .arg("clone")
    .arg(&source_url)
    .assert()
    .success()
    .stdout(contains("\"ast_tool\""))
    .stdout(contains("skip"));
}

#[test]
fn test_clone_creates_shallow_clone() {
  let source = make_cloneable_repo("clone-source-shallow");
  // Add a second commit.
  fs::write(source.path().join("second.txt"), "second").expect("write");
  std::process::Command::new("git")
    .args(["add", "."])
    .current_dir(source.path())
    .output()
    .expect("git add");
  std::process::Command::new("git")
    .args(["commit", "-m", "second"])
    .current_dir(source.path())
    .output()
    .expect("git commit");

  let (sandbox_dir, mut cmd) = sandbox::sandboxed_command("clone-shallow");
  let dest = sandbox_dir.path().join("cache");
  fs::create_dir_all(&dest).expect("create cache dir");
  let source_url = format!("file://{}", source.path().display());

  cmd
    .env("XDG_CACHE_HOME", &dest)
    .arg("clone")
    .arg(&source_url)
    .assert()
    .success();

  // Verify the clone is shallow (depth 1).
  let clones_dir = dest.join("apmw").join("clones");
  let entries: Vec<_> = fs::read_dir(&clones_dir)
    .expect("clones dir should exist")
    .flatten()
    .collect();
  let cloned_dir = entries[0].path();
  let log_output = std::process::Command::new("git")
    .args(["log", "--oneline"])
    .current_dir(&cloned_dir)
    .output()
    .expect("git log");
  let log_str = String::from_utf8_lossy(&log_output.stdout);
  let commit_count = log_str.lines().filter(|l| !l.is_empty()).count();
  assert_eq!(
    commit_count, 1,
    "shallow clone should have exactly 1 commit"
  );
}

#[test]
fn test_usage_flag() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("usage");
  cmd
    .arg("--usage")
    .assert()
    .success()
    .stdout(contains("USAGE:"))
    .stdout(contains("COMMANDS:"))
    .stdout(contains("install <package>"))
    .stdout(contains("detect"))
    .stdout(contains("--help"));
}

#[test]
fn test_man_flag() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("man");
  cmd
    .arg("--man")
    .assert()
    .success()
    // groff/troff man page format markers
    .stdout(contains(".TH apmw"))
    .stdout(contains(".SH NAME"))
    .stdout(contains(".SH SYNOPSIS"))
    .stdout(contains("All Package Manager Wrapper"));
}

#[test]
fn test_no_pager_flag_with_status() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("no-pager");
  cmd
    .arg("--no-pager")
    .arg("status")
    .assert()
    .success()
    .stdout(contains("apmw"));
}

#[test]
fn test_help_lists_all_commands() {
  let (_dir, mut cmd) = sandbox::sandboxed_command("help-lists");
  cmd
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("install"))
    .stdout(contains("detect"))
    .stdout(contains("status"))
    .stdout(contains("clone"))
    .stdout(contains("scan"))
    .stdout(contains("suggest"))
    .stdout(contains("info"))
    .stdout(contains("audit-log"))
    .stdout(contains("config"))
    .stdout(contains("governance"))
    .stdout(contains("intercept"))
    .stdout(contains("mcp"))
    .stdout(contains("--man"))
    .stdout(contains("--usage"))
    .stdout(contains("--no-pager"));
}
