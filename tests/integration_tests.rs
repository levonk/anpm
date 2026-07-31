//! Integration tests for apmw.

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_version_flag() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("--version")
    .assert()
    .success()
    .stdout(predicates::str::contains("apmw"));
}

#[test]
fn test_help_flag() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .assert()
    .success()
    .stdout(predicates::str::contains("apmw"));
}

#[test]
fn test_list_jobs_no_daemon() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("--no-daemon")
    .arg("--list-jobs")
    .assert()
    .success()
    .stdout(predicates::str::contains("--no-daemon mode"));
}

#[test]
fn test_list_jobs_daemon_not_running() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("--list-jobs")
    .assert()
    .success()
    .stdout(predicates::str::contains("Daemon is not running"));
}

#[test]
fn test_cancel_job_no_daemon_errors() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("--cancel-job")
    .arg("abc123")
    .assert()
    .failure()
    .stderr(predicates::str::contains("daemon is not running"));
}

#[test]
fn test_no_daemon_clone_errors() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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

/// Helper: create a temp directory, write the given files into it, and return
/// the path.
fn make_project_dir(files: &[&str]) -> TempDir {
  let dir = TempDir::new().expect("failed to create temp dir");
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
  let dir = make_project_dir(&["Cargo.toml", "Cargo.lock"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cargo"));
}

#[test]
fn test_detect_npm_project() {
  let dir = make_project_dir(&["package.json", "package-lock.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("npm"));
}

#[test]
fn test_detect_pnpm_project() {
  let dir = make_project_dir(&["pnpm-lock.yaml", "package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("pnpm"));
}

#[test]
fn test_detect_go_project() {
  let dir = make_project_dir(&["go.mod", "go.sum"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("go"));
}

#[test]
fn test_detect_pip_project() {
  let dir = make_project_dir(&["requirements.txt"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("pip"));
}

#[test]
fn test_detect_docker_project() {
  let dir = make_project_dir(&["Dockerfile"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("docker"));
}

#[test]
fn test_detect_empty_dir() {
  let dir = TempDir::new().expect("failed to create temp dir");
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("\"total\":0"));
}

#[test]
fn test_detect_human_mode() {
  let dir = make_project_dir(&["Cargo.toml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--human")
    .assert()
    .success();
}

#[test]
fn test_detect_multiple_managers() {
  let dir = make_project_dir(&["Cargo.toml", "package.json", "package-lock.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cargo"))
    .stdout(contains("npm"));
}

#[test]
fn test_detect_maven_project() {
  let dir = make_project_dir(&["pom.xml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("maven"));
}

#[test]
fn test_detect_gradle_project() {
  let dir = make_project_dir(&["build.gradle"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("gradle"));
}

#[test]
fn test_detect_dotnet_project() {
  let dir = make_project_dir(&["MyApp.csproj"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("dotnet"));
}

#[test]
fn test_detect_helm_project() {
  let dir = make_project_dir(&["Chart.yaml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("helm"));
}

#[test]
fn test_detect_poetry_project() {
  let dir = make_project_dir(&["poetry.lock"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("poetry"));
}

#[test]
fn test_detect_uv_project() {
  let dir = make_project_dir(&["uv.lock"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("uv"));
}

#[test]
fn test_detect_gem_project() {
  let dir = make_project_dir(&["Gemfile"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("gem"));
}

#[test]
fn test_detect_nix_project() {
  let dir = make_project_dir(&["flake.nix"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("nix"));
}

#[test]
fn test_detect_brew_project() {
  let dir = make_project_dir(&["Brewfile"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("brew"));
}

#[test]
fn test_detect_cmake_project() {
  let dir = make_project_dir(&["CMakeLists.txt"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("detect")
    .arg("--json")
    .assert()
    .success()
    .stdout(contains("cmake"));
}

#[test]
fn test_detect_compose_yaml() {
  let dir = make_project_dir(&["compose.yaml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["Cargo.toml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["Cargo.toml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["Cargo.toml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["Cargo.toml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let dir = make_project_dir(&["Cargo.toml"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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

#[test]
fn test_install_intercept_creates_shims() {
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("--install")
    .arg("--intercept")
    .assert()
    .success()
    .stdout(contains("Installed"))
    .stdout(contains("intercept shim"));
}

#[test]
fn test_uninstall_intercept_removes_shims() {
  // Install first, then uninstall.
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd.arg("--install").arg("--intercept").assert().success();

  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("--uninstall")
    .arg("--intercept")
    .assert()
    .success()
    .stdout(contains("Removed"));
}

#[test]
fn test_intercept_subcommand_delegates_no_governance() {
  // With no governance rules, intercept should delegate to the original tool.
  let mut cmd = Command::cargo_bin("apmw").unwrap();
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
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .arg("intercept")
    .arg("npm")
    .arg("--version")
    .assert()
    .success()
    .stdout(contains("npm"));
}

// ---------------------------------------------------------------------------
// Install (add engine) integration tests (story 04-001)
// ---------------------------------------------------------------------------

#[test]
fn test_install_with_dry_run() {
  let dir = make_project_dir(&["package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = make_project_dir(&["package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
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
  let dir = TempDir::new().expect("failed to create temp dir");
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("install")
    .arg("some-tool")
    .arg("--no-scan")
    .assert()
    .failure();
}

#[test]
fn test_install_path_scan_skips() {
  // "cargo" is on PATH in the test environment — should skip.
  let dir = make_project_dir(&["package.json"]);
  let mut cmd = Command::cargo_bin("apmw").unwrap();
  cmd
    .current_dir(dir.path())
    .arg("install")
    .arg("cargo")
    .arg("--manager")
    .arg("pnpm")
    .arg("--no-scan")
    .assert()
    .success()
    .stdout(contains("already"));
}
