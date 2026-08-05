---
story_id: "02-001"
story_title: "Migrate all integration tests to sandbox"
story_name: "migrate-integration-tests-to-sandbox"
prd_name: "sandbox-test-harness"
prd_file: "internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md"
phase: 2
parallel_id: 1
branch: "feature/current/sandbox-test-harness/story-02-001-migrate-integration-tests-to-sandbox"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-003"]
parallel_safe: false
modules: ["tests/integration_tests.rs"]
priority: "MUST"
risk_level: "medium"
tags: ["code", "tests", "migration", "sandbox"]
due: "2026-08-07"
create-date: "2026-08-05"
update-date: "2026-08-05"
---

## Summary

Migrate all ~70 integration tests in `tests/integration_tests.rs` from
`Command::cargo_bin("apmw")` to `sandboxed_command()` from the harness module
(01-003). Rewrite the intercept shim tests to use a sandbox-local PATH instead
of the real host PATH. All tests must pass under the sandbox.

## Sub-Tasks

- [ ] Add `mod sandbox;` import to `tests/integration_tests.rs`
- [ ] Migrate version/help/no-args tests (lines 8-37) to `sandboxed_command()`
- [ ] Migrate daemon/list-jobs tests (lines 40-93) to `sandboxed_command()`
- [ ] Migrate detection tests (lines 113-371) to `sandboxed_command()` —
      update `make_project_dir` to use `sandbox_tempdir()`
- [ ] Migrate manager override tests (lines 377-534) to `sandboxed_command()`
- [ ] Migrate intercept hook tests (lines 541-593):
  - [ ] Rewrite `test_install_intercept_creates_shims` to use a sandbox-local
        PATH (`<tempdir>/bin`) instead of the real host PATH
  - [ ] Rewrite `test_uninstall_intercept_removes_shims` similarly
  - [ ] Rewrite `test_intercept_subcommand_delegates_no_governance` to use
        sandboxed command
  - [ ] Rewrite `test_intercept_subcommand_with_version_flag` similarly
- [ ] Migrate install engine tests (lines 599-697) to `sandboxed_command()`
- [ ] Migrate clone tests (lines 703-921) to `sandboxed_command()` —
      update `make_cloneable_repo` to use `sandbox_tempdir()`
- [ ] Migrate usage/man/pager/help tests (lines 924-984) to `sandboxed_command()`
- [ ] Run `cargo test --test integration_tests` and verify all tests pass
- [ ] Run `cargo clippy --all-targets -- -D warnings` and verify no warnings

## Relevant Files

- `tests/integration_tests.rs` — the test file to migrate (984 lines)
- `tests/sandbox/mod.rs` — the harness module (from 01-003, read-only for this story)

## Acceptance Criteria (Gherkin)

- Given the migrated test file, When `cargo test --test integration_tests` runs
  with nono installed, Then all ~70 tests pass
- Given the migrated test file, When any test calls a command, Then it uses
  `sandboxed_command()` (grep for `Command::cargo_bin` should return zero
  matches in integration_tests.rs)
- Given `test_install_intercept_creates_shims`, When it runs, Then it writes
  shims to a sandbox-local PATH dir (inside TempDir), not the real host PATH
- Given the migrated test file, When `cargo clippy` runs, Then no warnings

## Test Plan

- `cargo test --test integration_tests` — all tests pass
- `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
- `cargo fmt --check` — formatted correctly
- Grep `tests/integration_tests.rs` for `Command::cargo_bin` — expect zero
  matches (all migrated to `sandboxed_command()`)

## Observability

- Test failures leave `/tmp/apmw-tests/<test-name>/` dirs for inspection
- The generated nono profile is at `<tempdir>/nono-profile.json` for debugging

## Compliance

- No `unwrap()` added (use `expect()` with context)
- Follow rustfmt (2-space, 100 char)
- Follow clippy (no warnings)
- No AI attribution boilerplate

## Risks & Mitigations

- **Risk**: Some tests may fail under sandbox (e.g., tests that read files
  outside the TempDir)
- **Mitigation**: The nono profile allows reads of `/usr`, `/lib`, `/etc`,
  `~/.cargo/registry`, `~/.rustup`, `~/.local/share/pnpm`. If a test needs
  an additional read path, add it to the profile template in the harness
  module (01-003). Document any additions in the ADR.
- **Risk**: Intercept shim rewrite may break if apmw's intercept logic
  hardcodes the real PATH
- **Mitigation**: Check apmw's intercept implementation. If it respects a
  `--prefix` or `--path` flag, use it. If not, set `PATH=<tempdir>/bin` in
  the test's env and verify apmw writes there.
- **Risk**: Clone tests may fail if nono blocks `git clone` from file:// URLs
- **Mitigation**: The nono profile allows fs_read of `/tmp` (where the source
  repo lives). If nono blocks file:// URLs, add the source repo path to
  fs_read explicitly.

## Dependencies & Sequencing

- Depends on: 01-003 (harness module must exist)
- Unblocks: 03-001, 03-002 (verification and docs depend on migration)

## Definition of Done

- All ~70 integration tests pass under the sandbox (with nono installed)
- Zero `Command::cargo_bin` calls remain in `tests/integration_tests.rs`
- Intercept shim tests use sandbox-local PATH
- `cargo clippy` and `cargo fmt --check` pass

## Commit Conventions

```
test: migrate all integration tests to nono sandbox

Replace all Command::cargo_bin("apmw") calls with sandboxed_command() from
the harness module. Rewrite intercept shim tests to use sandbox-local PATH.
All ~70 tests now run inside nono with fs/network restrictions.
```
