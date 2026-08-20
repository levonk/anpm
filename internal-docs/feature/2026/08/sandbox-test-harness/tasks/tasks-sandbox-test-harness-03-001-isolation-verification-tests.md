---
story_id: "03-001"
story_title: "Isolation verification tests"
story_name: "isolation-verification-tests"
prd_name: "sandbox-test-harness"
prd_file: "internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md"
phase: 3
parallel_id: 1
branch: "feature/current/sandbox-test-harness/story-03-001-isolation-verification-tests"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["02-001"]
parallel_safe: true
modules: ["tests/sandbox_tests.rs"]
priority: "MUST"
risk_level: "low"
tags: ["code", "tests", "security", "verification"]
due: "2026-08-07"
create-date: "2026-08-05"
update-date: "2026-08-05"
---

## Summary

Add two new verification tests that confirm the sandbox actually isolates
personal files and allows real registry access. These tests prove the sandbox
is effective, not just present.

## Sub-Tasks

- [ ] Create `tests/sandbox_tests.rs` (new file, separate from
      `integration_tests.rs`)
- [ ] Add `test_sandbox_denies_home_ssh`:
  - Creates a sandboxed command that runs `apmw` with an arg that causes it to
    read `$HOME/.ssh` (or use a direct `cat $HOME/.ssh/id_rsa` subprocess
    inside the sandbox)
  - Asserts the read is DENIED (nono blocks it, process exits with error or
    empty output)
  - If apmw has no command that reads arbitrary files, use a shell command
    inside the sandbox: `nono run --profile <profile> -- cat $HOME/.ssh/id_rsa`
    and assert it fails
- [ ] Add `test_sandbox_allows_real_registries`:
  - Creates a sandboxed command that runs `apmw install express --manager pnpm`
    (real install, no dry-run)
  - Asserts it succeeds (network to registry.npmjs.org is allowed)
  - Or: runs `nono run --profile <profile> -- curl -sI https://registry.npmjs.org/`
    inside the sandbox and asserts HTTP 200
- [ ] Run `cargo test --test sandbox_tests` and verify both tests pass
- [ ] Run `cargo clippy --all-targets -- -D warnings`

## Relevant Files

- `tests/sandbox_tests.rs` — new verification test file
- `tests/sandbox/mod.rs` — harness module (from 01-003, read-only)

## Acceptance Criteria (Gherkin)

- Given `test_sandbox_denies_home_ssh`, When it runs inside the sandbox, Then
  reading `$HOME/.ssh/id_rsa` is denied (nono blocks the read)
- Given `test_sandbox_allows_real_registries`, When it runs inside the sandbox,
  Then network access to `registry.npmjs.org` succeeds
- Given the test file, When `cargo test --test sandbox_tests` runs, Then both
  tests pass
- Given the test file, When `cargo clippy` runs, Then no warnings

## Test Plan

- `cargo test --test sandbox_tests` — both tests pass
- `cargo clippy --all-targets -- -D warnings` — no warnings
- `cargo fmt --check` — formatted correctly

## Observability

- Test names are self-documenting: `test_sandbox_denies_home_ssh`,
  `test_sandbox_allows_real_registries`

## Compliance

- No `unwrap()` (use `expect()` with context)
- Follow rustfmt and clippy
- No AI attribution boilerplate

## Risks & Mitigations

- **Risk**: `test_sandbox_denies_home_ssh` may fail if the test user has no
  `~/.ssh/id_rsa` file (nono can't block a read of a nonexistent file — the
  read fails for a different reason)
- **Mitigation**: Create a dummy `$HOME/.ssh/id_rsa` inside the test (set
  `HOME=<tempdir>` in the sandbox env, create `<tempdir>/.ssh/id_rsa`, then
  verify the sandbox denies reading it even though it exists). This tests the
  sandbox policy, not the file's existence.
- **Risk**: `test_sandbox_allows_real_registries` may fail in offline
  environments
- **Mitigation**: Mark the test as `#[ignore]` if network is unavailable, or
  skip if `APMW_TEST_OFFLINE=1` is set. In CI, network is available.

## Dependencies & Sequencing

- Depends on: 02-001 (migration must be complete so the harness is proven)
- Unblocks: nothing (this is a leaf story)

## Definition of Done

- `tests/sandbox_tests.rs` exists with both verification tests
- Both tests pass with nono installed
- `cargo clippy` and `cargo fmt --check` pass

## Commit Conventions

```
test: add sandbox isolation verification tests

Add test_sandbox_denies_home_ssh (confirms personal files are unreadable
inside the sandbox) and test_sandbox_allows_real_registries (confirms
npmjs/crates.io are reachable inside the sandbox).
```
