---
story_id: "01-003"
story_title: "Harness: sandbox module + nono profile"
story_name: "harness-sandbox-module-nono-profile"
prd_name: "sandbox-test-harness"
prd_file: "internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md"
phase: 1
parallel_id: 3
branch: "feature/current/sandbox-test-harness/story-01-003-harness-sandbox-module-nono-profile"
status: "todo"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["tests/sandbox/"]
priority: "MUST"
risk_level: "medium"
tags: ["code", "tests", "sandbox", "security"]
due: "2026-08-06"
create-date: "2026-08-05"
update-date: "2026-08-05"
---

## Summary

Create the sandbox test harness module at `tests/sandbox/mod.rs` that provides
`sandboxed_command()`, `ensure_nono_or_skip()`, and `sandbox_tempdir()`. The
module generates a per-test nono profile JSON dynamically (with the test's
TempDir path in `fs_write`) and wraps `Command::cargo_bin("apmw")` through
`nono run --profile <profile> -- <apmw-binary>`.

## Sub-Tasks

- [ ] Create `tests/sandbox/` directory
- [ ] Create `tests/sandbox/mod.rs` with:
  - [ ] `struct NonoProfile` — serializable profile with fs_read, fs_write,
        net_allow, net_deny, env fields
  - [ ] `fn default_profile(tempdir: &Path) -> NonoProfile` — returns a profile
        with the TempDir in fs_write, standard allowed paths in fs_read,
        registry domains in net_allow, `*` in net_deny, minimal env allowlist
  - [ ] `fn write_profile(profile: &NonoProfile, path: &Path) -> Result<()>` —
        serializes to JSON
  - [ ] `fn ensure_nono_or_skip() -> bool` — checks `command -v nono`; if absent,
        prints warning and returns false (or panics if
        `APMW_TEST_SANDBOX_REQUIRED=1`)
  - [ ] `fn sandbox_tempdir(test_name: &str) -> TempDir` — creates
        `/tmp/apmw-tests/<test-name>/` deterministically (cleans up existing
        dir first)
  - [ ] `fn sandboxed_command(test_name: &str) -> (TempDir, Command)` —
        creates a TempDir, generates a per-test nono profile, writes it to
        `<tempdir>/nono-profile.json`, and returns a `Command` invoking
        `nono run --profile <profile> -- <apmw-binary>`
- [ ] Create `tests/sandbox/nono-profile.json` — a static reference profile
      (for documentation; the actual profiles are generated per-test)
- [ ] Add `serde_json` to `[dev-dependencies]` in `Cargo.toml` if not present
  (for profile serialization)
- [ ] Verify the module compiles with `cargo check --tests`

## Relevant Files

- `tests/sandbox/mod.rs` — the harness module (new)
- `tests/sandbox/nono-profile.json` — static reference profile (new, for docs)
- `Cargo.toml` — add `serde_json` to dev-dependencies if not present
- `tests/integration_tests.rs` — NOT modified in this story (migration is 02-001)

## Acceptance Criteria (Gherkin)

- Given `tests/sandbox/mod.rs`, When `cargo check --tests` runs, Then it
  compiles without errors
- Given `sandbox_tempdir("my_test")`, When called, Then it creates
  `/tmp/apmw-tests/my_test/` and returns a TempDir pointing to it
- Given `sandboxed_command("my_test")`, When the returned Command is run, Then
  it invokes `nono run --profile <path>/nono-profile.json -- <apmw-binary>`
- Given `ensure_nono_or_skip()`, When nono is not installed and
  `APMW_TEST_SANDBOX_REQUIRED` is not set, Then it prints a warning and returns
  false
- Given `ensure_nono_or_skip()`, When nono is not installed and
  `APMW_TEST_SANDBOX_REQUIRED=1`, Then it panics
- Given the generated nono profile, When inspected, Then fs_write contains the
  TempDir path, net_allow contains registry.npmjs.org and crates.io, net_deny
  contains `*`, and env contains PATH/HOME/CARGO_HOME/RUSTUP_HOME

## Test Plan

- `cargo check --tests` — module compiles
- `cargo clippy --all-targets -- -D warnings` — no warnings
- `cargo fmt --check` — formatted correctly
- Manual: call `sandbox_tempdir("test")` and verify `/tmp/apmw-tests/test/`
  exists
- Manual: call `sandboxed_command("test")` and verify the Command's program is
  `nono` with correct args

## Observability

- The generated nono profile JSON is written to `<tempdir>/nono-profile.json`
  for debugging (visible if a test fails and the TempDir is preserved)

## Compliance

- No `unwrap()` in harness functions (use `expect()` with context or `?`)
- Follow rustfmt (2-space, 100 char)
- Follow clippy (no warnings)

## Risks & Mitigations

- **Risk**: nono profile schema may differ from the assumed format
  (fs_read/fs_write/net_allow/net_deny/env)
- **Mitigation**: Verify against nono docs (docs.nono.sh). If the schema
  differs, update the `NonoProfile` struct. The ADR (01-001) documents this
  risk.
- **Risk**: Dynamic profile generation per-test may be slow
- **Mitigation**: Profile JSON is small (~500 bytes), serialization is
  sub-millisecond. Negligible overhead.
- **Risk**: `/tmp/apmw-tests/` may have permission issues on some systems
- **Mitigation**: Use `std::fs::create_dir_all` with explicit error context.
  Fall back to `TempDir::new()` if `/tmp/apmw-tests/` cannot be created.

## Dependencies & Sequencing

- No dependencies — can start immediately
- Unblocks: 02-001 (test migration depends on this harness module)

## Definition of Done

- `tests/sandbox/mod.rs` exists with all functions listed in Sub-Tasks
- `cargo check --tests` passes
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo fmt --check` passes
- The generated profile has correct fs_read/fs_write/net_allow/net_deny/env

## Commit Conventions

```
feat(test): add sandbox test harness module

Create tests/sandbox/mod.rs with sandboxed_command(), ensure_nono_or_skip(),
and sandbox_tempdir(). The harness generates per-test nono profiles that
restrict fs writes to the TempDir and network to real registries only.
```
