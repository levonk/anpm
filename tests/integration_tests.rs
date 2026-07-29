//! Integration tests for apmw.

use assert_cmd::Command;

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("apmw").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("apmw"));
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("apmw").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Abstracts every package installer"));
}

#[test]
fn test_no_args_prints_version() {
    let mut cmd = Command::cargo_bin("apmw").unwrap();
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("apmw"));
}
