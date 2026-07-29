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
