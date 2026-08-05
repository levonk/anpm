# Product Requirements Document (PRD)

## Introduction / Overview

- **Feature name:** Sandboxed Integration Test Harness
- **Summary:** Wrap all apmw integration tests in a nono process sandbox so that
  test packages, cloned repos, and intercept shims cannot read the developer's
  personal files (`$HOME/.ssh`, `$HOME/.aws`, browser cookies, etc.) or contact
  arbitrary network endpoints. Tests continue to use `tempfile::TempDir` for
  path management (hybrid approach), but every `Command::cargo_bin("apmw")`
  invocation is wrapped through `nono run --profile <profile> --` with a bundled
  profile that restricts filesystem writes to the temp dir and network access to
  real package registries only.
- **Context:**
  - **Who:** apmw contributors and CI — anyone running `cargo test` or `just test`.
  - **Problem:** The current integration test suite (`tests/integration_tests.rs`,
    ~70 tests) runs `apmw` as a normal subprocess with full process privileges.
    Several tests execute real side-effecting paths: `test_manager_override_on_install`
    runs `apmw install express --manager pnpm` (no `--dry-run`), `test_clone_local_repo_succeeds`
    does a real `git clone`, and `test_install_intercept_creates_shims` /
    `test_uninstall_intercept_removes_shims` install/remove intercept shims on the
    host. A malicious test package or cloned repo could exfiltrate `~/.ssh/id_rsa`,
    `~/.aws/credentials`, browser cookies, or env vars to an arbitrary endpoint.
  - **Solution:** Use **nono** (the sandbox tool chosen by the skills-src ADR
    `adr-202608030953-skill-refresh-sandbox-selection`) to wrap every integration
    test subprocess. nono uses OS primitives (Seatbelt on macOS, Landlock on
    Linux) with zero-latency overhead, per-tool child sandboxes, and L7 network
    filtering. A bundled nono profile restricts fs writes to the test's TempDir,
    denies reads of `$HOME` outside explicitly allowed paths (cargo registry,
    pnpm store), and allows network only to real registries (npmjs.org,
    crates.io, github.com, static.crates.io). Tests that cannot run under
    sandbox restrictions (e.g., intercept shim install to the real PATH) are
    rewritten to target a sandbox-local PATH.
  - **Background:** The sandbox tool evaluation was done in
    `~/p/gh/levonk/skills-src/internal-docs/adr/2026/08/adr-202608030953-skill-refresh-sandbox-selection.md`.
    That ADR chose nono over zerobox (no nested sandboxing), bubblewrap
    (Linux-only), OpenShell (requires container runtime), and LavaMoat (different
    threat model). The ADR's rationale — nested sandboxing support, L7 filtering,
    Sigstore attestation, per-tool child sandboxes, zero-latency overhead —
    applies equally to apmw's test harness. This PRD adopts the same tool for
    consistency across the levonk toolchain.

## Goals

1. **Isolate** every integration test subprocess from the developer's personal
   files — no test may read `$HOME/.ssh`, `$HOME/.aws`, `$HOME/.config` (outside
   explicitly allowed package-manager paths), browser cookies, or env vars
   beyond a minimal allowlist.
2. **Restrict network** egress to real package registries only
   (`registry.npmjs.org`, `crates.io`, `static.crates.io`, `github.com`,
   `raw.githubusercontent.com`, `objects.githubusercontent.com`). All other
   network destinations are denied — a malicious test package cannot exfiltrate
   data to an attacker-controlled endpoint.
3. **Preserve test correctness** — all currently passing integration tests must
   continue to pass under the sandbox. Tests that genuinely require unrestricted
   access (e.g., writing intercept shims to the real `$HOME/.local/bin`) are
   rewritten to use a sandbox-local PATH so the test verifies the logic without
   touching the real host.
4. **Zero install friction on macOS** — nono is installed via `brew install nono`
   (or `curl -fsSL https://nono.sh/install.sh | sh`). If nono is not installed,
   tests fall back to unsandboxed execution with a clear warning, so `cargo test`
   never fails solely because nono is absent.
5. **Record the decision** — create an ADR in `apmw/internal-docs/adr/` (which
   does not yet exist) documenting the sandbox tool choice, profile schema, and
   fallback policy, referencing the skills-src ADR as the prior evaluation.

## User Stories

1. **As an apmw contributor**, I want `just test` to run all integration tests
   inside a nono sandbox automatically — so a malicious test package cannot
   steal my SSH keys, AWS credentials, or browser cookies.

2. **As an apmw contributor**, I want the sandbox to allow real registry access
   (npmjs, crates.io, github) — so tests that verify real install behavior
   (`test_manager_override_on_install`) continue to work without mocking every
   registry.

3. **As an apmw contributor**, I want tests to use `tempfile::TempDir` for path
   management (hybrid approach) — so I get automatic cleanup and deterministic
   temp paths, with the sandbox restricting writes to that temp dir.

4. **As an apmw contributor**, if nono is not installed on my machine, I want
   `cargo test` to fall back to unsandboxed execution with a one-line warning —
   so I am not blocked from running tests on a machine without nono, but I know
   the sandbox is inactive.

5. **As a CI pipeline**, I want the sandbox to be mandatory (no fallback) — so
   CI never runs unsandboxed tests. The CI environment installs nono in the
   Dockerfile/devbox setup.

6. **As an apmw maintainer**, I want an ADR in the apmw repo documenting the
   sandbox tool choice and profile — so future contributors understand why nono
   was chosen and how to modify the profile.

## Functional Requirements

1. **FR-1: nono profile bundle** — A nono profile JSON file is committed at
   `tests/sandbox/nono-profile.json` with the following restrictions:
   - `fs_write`: the test's TempDir path (injected dynamically per-test)
   - `fs_read`: TempDir, `/usr`, `/lib`, `/lib64`, `/etc`, `/tmp`,
     `~/.cargo/registry` (for cargo tests), `~/.local/share/pnpm`,
     `~/.config/pnpm`, `~/.rustup` (for the toolchain)
   - `net_allow`: `registry.npmjs.org`, `crates.io`, `static.crates.io`,
     `index.crates.io`, `github.com`, `raw.githubusercontent.com`,
     `objects.githubusercontent.com`
   - `net_deny`: `*` (everything else)
   - `env`: `PATH`, `HOME`, `USER`, `SHELL`, `TERM`, `LANG`, `LC_ALL`,
     `CARGO_HOME`, `RUSTUP_HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`,
     `XDG_DATA_HOME`, `TMPDIR`

2. **FR-2: Test harness module** — A new module `tests/sandbox/mod.rs` (or
   `tests/common/sandbox.rs`) provides:
   - `fn sandboxed_command() -> Command` — returns a `Command` that invokes
     `nono run --profile <profile-path> -- <apmw-binary> [args]` instead of
     the bare `apmw` binary. The TempDir path is passed to nono via the
     profile's `fs_write` field (dynamically generated per-test, or via a
     nono env-var injection mechanism).
   - `fn ensure_nomo_or_skip() -> bool` — checks if nono is installed; if not,
     prints a warning and returns `false`. Tests call this at the top and
     `return` early if `false` (on local dev). In CI, this function panics
     instead of returning `false` (CI must be sandboxed).
   - `fn sandbox_tempdir() -> TempDir` — wraps `TempDir::new()` but creates
     the temp dir under `/tmp/apmw-tests/<test-name>/` for deterministic,
     greppable paths.

3. **FR-3: Migrate all integration tests** — Every test in
   `tests/integration_tests.rs` that calls `Command::cargo_bin("apmw")` is
   migrated to call `sandboxed_command()` instead. The test's TempDir is
   registered with the sandbox profile before the command runs.

4. **FR-4: Intercept shim tests rewritten** — `test_install_intercept_creates_shims`
   and `test_uninstall_intercept_removes_shims` are rewritten to use a
   sandbox-local `PATH` (a dir inside the TempDir) instead of the real host
   PATH. The test verifies that apmw writes/removes shims in the specified
   PATH dir, not the real `$HOME/.local/bin`.

5. **FR-5: CI mandatory sandbox** — The GitHub Actions workflow
   (`.github/workflows/ci.yml`) installs nono in the CI environment and sets
   `APMW_TEST_SANDBOX_REQUIRED=1` so the harness panics if nono is absent.

6. **FR-6: ADR** — Create `internal-docs/adr/2026/08/` directory and write
   `adr-202608052241-test-sandbox-selection.md` documenting:
   - The sandbox tool choice (nono, referencing the skills-src ADR)
   - The nono profile schema and allowed paths/domains
   - The fallback policy (unsandboxed on local dev with warning, mandatory in CI)
   - The TempDir-inside-sandbox hybrid approach
   - Alternatives considered (sandbox-exec, devbox isolation, Docker)

7. **FR-7: devbox.json update** — Add `nono` to `devbox.json` so `devbox shell`
   provides it. The `just doctor` command checks for nono and reports its
   presence/absence.

## Non-Functional Requirements

1. **NFR-1: Performance** — The sandbox must add < 50ms overhead per test
   invocation (nono claims zero-latency). The full integration test suite
   (~70 tests) must complete within 1.5x the current unsandboxed runtime.

2. **NFR-2: Portability** — The sandbox works on macOS (x86_64, arm64) and
   Linux (x86_64, arm64). On platforms where nono is unavailable, tests fall
   back to unsandboxed with a warning (local dev) or fail (CI).

3. **NFR-3: No secret leakage** — The nono profile must deny reads of
   `$HOME/.ssh`, `$HOME/.aws`, `$HOME/.gnupg`, `$HOME/.config` (outside
   package-manager paths), `$HOME/Library/Cookies` (macOS),
   `$HOME/.config/google-chrome` (Linux). This is verified by a test that
   attempts to read these paths from inside the sandbox and asserts denial.

4. **NFR-4: Deterministic paths** — TempDirs are created under
   `/tmp/apmw-tests/<test-name>/` (not random `/tmp/.tmpXXXXXX`) so test
   failures leave greppable artifacts for debugging. Each test cleans up its
   dir on success; on failure, the dir is left for inspection.

5. **NFR-5: Code quality** — The harness module passes `cargo clippy -- -D warnings`
   and `cargo fmt --check`. No `unwrap()` in the harness except in test-only
   code paths.

## Current State

- **Relevant files and their roles:**
  - `tests/integration_tests.rs` (984 lines, ~70 tests) — the integration test
    suite. Uses `assert_cmd::Command::cargo_bin("apmw")` for every test. Helper
    `make_project_dir()` creates TempDirs with project marker files. Helper
    `make_cloneable_repo()` creates a bare git repo for clone tests.
  - `Cargo.toml` — dev-dependencies include `assert_cmd`, `predicates`,
    `serial_test`, `tempfile`, `proptest`. No nono-related deps.
  - `devbox.json` — Nix dev environment. Does not include nono.
  - `.github/workflows/ci.yml` — CI pipeline (fmt, clippy, test, build, audit).
    Does not install nono.
  - `justfile` — `just test` runs `cargo test`. `just test_impl` runs it inside
    devbox.
  - `AGENTS.md` — binding contract. Documents `just test` as the test command,
    `just validate` as the full quality gate.

- **Existing code excerpts:**
  - Current unsandboxed command pattern (lines 9-16):
    ```rust
    let mut cmd = Command::cargo_bin("apmw").unwrap();
    cmd.arg("--version").assert().success();
    ```
  - Current TempDir helper (lines 101-111):
    ```rust
    fn make_project_dir(files: &[&str]) -> TempDir {
      let dir = TempDir::new().expect("failed to create temp dir");
      // ... writes files into dir ...
      dir
    }
    ```
  - Real install test without dry-run (lines 496-506):
    ```rust
    let mut cmd = Command::cargo_bin("apmw").unwrap();
    cmd.arg("install").arg("express").arg("--manager").arg("pnpm")
      .assert().success().stdout(contains("via pnpm"));
    ```
  - Intercept shim install on host (lines 541-551):
    ```rust
    let mut cmd = Command::cargo_bin("apmw").unwrap();
    cmd.arg("--install").arg("--intercept").assert().success();
    ```

- **Repository conventions:**
  - Rust edition 2021, MSRV 1.70. Errors via `thiserror` (lib) / `anyhow` (bin).
  - `just <task>` runs tasks; `just <task>_impl` runs inside devbox.
  - `cargo clippy --all-targets --all-features -- -D warnings` is the lint gate.
  - `cargo fmt --check` is the format gate (2-space, 100 char).
  - Tests use `assert_cmd` + `predicates` + `tempfile` + `serial_test`.
  - No AI attribution boilerplate in commits or files (per AGENTS.md).

- **Design constraints:**
  - The sandbox tool must be nono (aligned with skills-src ADR
    `adr-202608030953`). The ADR's rationale (nested sandboxing, L7 filtering,
    zero-latency, per-tool child sandboxes) is binding.
  - The nono profile must allow real registry access (user requirement: "it
    should have access to pull from real registries").
  - The nono profile must deny reads of personal files (user requirement: "any
    choice needs to isolate my files from packages").
  - Tests must use TempDir inside the sandbox (user requirement: "Hybrid:
    TempDir inside sandbox").

## Technical Considerations

- **nono profile dynamic fs_write**: nono profiles are static JSON, but each
  test needs a different TempDir path in `fs_write`. Two approaches:
  1. Generate the profile JSON per-test in Rust (serialize a struct to JSON,
     write to a temp file, pass to `nono run --profile <temp-profile>`).
  2. Use nono's env-var expansion in profiles (if supported — verify in nono
     docs) and pass the TempDir path via an env var.
  Approach 1 is more portable and does not depend on nono profile features.
  The harness generates a per-test profile from a template struct.

- **nono invocation from Rust**: `Command::cargo_bin("apmw")` returns a
  `Command` pointing at the apmw binary. The harness wraps this:
  ```rust
  let mut cmd = Command::new("nono");
  cmd.arg("run").arg("--profile").arg(&profile_path)
     .arg("--").arg(apmw_binary_path).args(args);
  ```
  The apmw binary path is resolved via `env!("CARGO_BIN_EXE_apmw")` (the
  canonical way to find the built binary in integration tests).

- **CI nono installation**: The CI workflow (Ubuntu-based) installs nono via
  `curl -fsSL https://nono.sh/install.sh | sh` in a setup step. The Dockerfile
  may also install nono for container-based CI.

- **Intercept shim tests**: The current `test_install_intercept_creates_shims`
  installs shims to the host PATH. Under sandbox, the host PATH is restricted.
  The test is rewritten to set `PATH=<tempdir>/bin` and verify apmw writes
  shims there. This tests the logic without touching the real host.

## Verification Approach

| Purpose   | Command                          | Expected Result                    |
|-----------|----------------------------------|------------------------------------|
| Build     | `just build`                     | exit 0                             |
| Tests     | `just test`                      | all pass (sandboxed if nono present)|
| Lint      | `just lint`                      | exit 0, no warnings                |
| Format    | `just format` (check mode)       | `cargo fmt --check` passes         |
| Audit     | `just audit`                     | no vulnerabilities                 |
| Validate  | `just validate`                  | all gates pass                     |
| Sandbox   | `cargo test --test integration_tests -- sandbox_isolation` | the isolation verification test passes (confirms personal paths are denied) |
| CI        | GitHub Actions workflow          | all jobs green with nono installed |

## Success Criteria (Machine-Checkable)

- [ ] `just test` passes with all ~70 integration tests sandboxed under nono
- [ ] A new test `test_sandbox_denies_home_ssh` confirms that reading
      `$HOME/.ssh` from inside the sandbox is denied
- [ ] A new test `test_sandbox_allows_real_registries` confirms that
      `registry.npmjs.org` and `crates.io` are reachable from inside the sandbox
- [ ] `just lint` passes with the new harness module
- [ ] `just validate` passes (all quality gates)
- [ ] CI workflow installs nono and runs tests sandboxed (no fallback)
- [ ] ADR file exists at `internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md`
- [ ] `devbox.json` includes nono
- [ ] `just doctor` reports nono presence/absence

## Out of Scope

- **Sandboxing unit tests** (`#[cfg(test)]` inline tests) — unit tests are pure
  and do not touch the system. Sandboxing them adds overhead with no benefit.
- **Sandboxing benchmarks** (`benches/performance.rs`) — benchmarks measure
  performance; sandbox overhead would skew results.
- **Mocking registries** — the user explicitly wants real registry access. A
  local mock registry server is not needed.
- **Windows support** — nono does not support native Windows. Windows tests
  run unsandboxed (consistent with the skills-src ADR's Windows policy). WSL2
  uses the Linux/nono path.
- **Sandboxing the daemon** — the daemon (`--daemon` mode) is not exercised by
  integration tests in a way that requires sandboxing beyond the CLI wrapper.
  Daemon sandboxing is a future concern.

## Risk Assessment

- **Priority:** P2
- **Effort:** M
- **Risk:** MED

## Success Metrics

- **Zero secret leakage** — the `test_sandbox_denies_home_ssh` test confirms
  personal files are unreadable from inside the sandbox.
- **No test regressions** — all ~70 integration tests pass under the sandbox.
- **CI enforced** — CI fails if nono is absent (no silent unsandboxed CI).

## Open Questions

- None remaining. All clarifying questions answered:
  - Sandbox tool: nono (align with skills-src ADR)
  - Scope: all integration tests
  - Fixture layout: hybrid (TempDir inside sandbox)
  - Network: allow real registries, deny everything else

## Dependencies

- **nono** must be installed (`brew install nono` or
  `curl -fsSL https://nono.sh/install.sh | sh`). Added to `devbox.json`.
- **skills-src ADR `adr-202608030953`** — referenced as the prior evaluation.
  Not a runtime dependency; the ADR is cited in apmw's own ADR.

## Timeline / Milestones

1. **ADR creation** — write the apmw ADR documenting the sandbox choice.
2. **Harness module** — implement `tests/sandbox/mod.rs` with
   `sandboxed_command()`, `ensure_nono_or_skip()`, `sandbox_tempdir()`.
3. **nono profile** — implement `tests/sandbox/nono-profile.json` template and
   per-test profile generation.
4. **Test migration** — migrate all ~70 integration tests to use the harness.
5. **Intercept shim rewrite** — rewrite intercept tests to use sandbox-local PATH.
6. **Isolation verification tests** — add `test_sandbox_denies_home_ssh` and
   `test_sandbox_allows_real_registries`.
7. **CI + devbox update** — add nono to devbox.json, CI workflow, `just doctor`.
8. **Documentation** — update AGENTS.md with sandbox testing notes.

## Maintenance Notes

- **Future nono profile changes**: if nono's profile schema changes (nono is
  pre-1.0), the profile template struct in `tests/sandbox/mod.rs` must be
  updated. The ADR documents this risk.
- **New integration tests**: any new test in `tests/integration_tests.rs` must
  use `sandboxed_command()` — not `Command::cargo_bin("apmw")` directly. A
  lint check or code review convention enforces this.
- **New network endpoints**: if a test needs to contact a new registry (e.g.,
  `pypi.org` for a Python ecosystem test), the nono profile's `net_allow` list
  must be updated in the harness template struct.
- **Reviewers should scrutinize**: the nono profile's `fs_read` allowlist —
  every allowed path is a potential leak vector. The `env` allowlist — every
  allowed env var is accessible to the sandboxed process.
