//! Integration tests for apmw.

use assert_cmd::Command;

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
