---
modeline: "vim: set ft=markdown:"
title: "ADR: Test Sandbox Selection"
adr-id: "202608052241"
slug: "test-sandbox-selection"
url: "https://github.com/levonk/apmw/blob/main/internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md"
synopsis: "Select a lightweight process sandbox to wrap apmw integration test subprocesses, adopting nono (chosen by the skills-src ADR adr-202608030953) with a bundled profile that restricts filesystem writes to the test TempDir, denies reads of personal files, and allows network only to real package registries. Includes a fallback policy (unsandboxed + warning on local dev, mandatory in CI) and a hybrid TempDir-inside-sandbox approach with deterministic paths under /tmp/apmw-tests/<test-name>/."
author: "https://github.com/levonk"
date-created: "2026-08-05"
date-updated: "2026-08-05"
date-review: "2027-02-05"
date-triggers: ["2027-02-05"]
version: "1.0.0"
status: "accepted"
aliases: []
tags: [doc/architecture/adr, security, sandbox, testing, integration-tests]
supersedes: []
superseded-by: []
related-to:
  - "https://github.com/levonk/skills-src/blob/main/internal-docs/adr/2026/08/adr-202608030953-skill-refresh-sandbox-selection.md"
scope:
  impact-scope:
    - "tests/integration_tests.rs (all ~70 integration tests)"
    - "tests/sandbox/ (new harness module + nono profile)"
    - ".github/workflows/ci.yml (nono installation + APMW_TEST_SANDBOX_REQUIRED)"
    - "devbox.json (nono package)"
    - "justfile (just doctor nono check)"
  excluded-scope:
    - "src/ (no production code changes — sandbox wraps the test subprocess, not the daemon)"
    - "benches/performance.rs (benchmarks measure raw performance; sandbox overhead would skew results)"
    - "Unit tests (#[cfg(test)] inline tests are pure and do not touch the system)"
platforms:
  - "macOS x86_64"
  - "macOS arm64 (Apple Silicon)"
  - "Linux x86_64"
  - "Linux arm64"
---

# Decision Record: Test Sandbox Selection

**Filename:** `adr-202608052241-test-sandbox-selection.md`

- belongs in `internal-docs/adr/2026/08/*.md`

---

## Context

The apmw integration test suite (`tests/integration_tests.rs`, ~70 tests) runs
`apmw` as a normal subprocess via `assert_cmd::Command::cargo_bin("apmw")` with
full process privileges. Several tests execute real side-effecting paths:

| Test | Side effect |
|------|-------------|
| `test_manager_override_on_install` | Runs `apmw install express --manager pnpm` (no `--dry-run`) — real package install |
| `test_clone_local_repo_succeeds` | Performs a real `git clone` |
| `test_install_intercept_creates_shims` | Installs intercept shims onto the host PATH |
| `test_uninstall_intercept_removes_shims` | Removes intercept shims from the host PATH |

A malicious test package or cloned repo could exfiltrate `~/.ssh/id_rsa`,
`~/.aws/credentials`, browser cookies, or environment variables to an arbitrary
network endpoint. The current test harness provides no isolation between the
test subprocess and the developer's personal files.

### Prior evaluation

The sandbox tool evaluation was performed in the skills-src ADR
[`adr-202608030953-skill-refresh-sandbox-selection`](https://github.com/levonk/skills-src/blob/main/internal-docs/adr/2026/08/adr-202608030953-skill-refresh-sandbox-selection.md).
That ADR evaluated six candidates (nono, zerobox, bubblewrap, OpenShell,
LavaMoat, vpod) and chose **nono** for the following reasons:

1. **Nested sandboxing** — nono's broker manages per-tool child sandboxes,
   designed for nesting. An agent already running inside a nono session can
   spawn child sandboxes with independent policies. zerobox has no nesting
   concept; bubblewrap namespaces can nest but parent limits apply.
2. **L7 network filtering** — nono filters at the method + path level, not just
   domain level. A compromised process cannot POST exfiltrated data to a path
   on an allowed domain.
3. **Zero-latency overhead** — nono uses OS primitives (Seatbelt on macOS,
   Landlock on Linux) with no daemon and no container runtime.
4. **Per-tool child sandboxes** — each tool call gets its own child sandbox
   with separate filesystem grants, network rules, and credentials.
5. **Sigstore-signed profiles** — cryptographic assurance that the profile
   itself has not been tampered with.
6. **Composable JSON profiles** — a profile system with a registry, vs.
   zerobox's CLI flags only.

This ADR adopts the same tool for apmw's integration test harness for
consistency across the levonk toolchain. The rationale from the skills-src ADR
applies equally: the test subprocess executes arbitrary package code (real
installs, real clones), the sandbox must be lightweight (under 50ms overhead
per test), and the sandbox must work on macOS and Linux.

### Candidates (summary from skills-src ADR)

| Tool | macOS | Linux | Overhead | Nested sandbox | AI-agent focused |
|------|-------|-------|----------|----------------|-----------------|
| [nono](https://github.com/nolabs-ai/nono) | Seatbelt | Landlock | Zero latency | Per-tool child sandboxes | Yes |
| [zerobox](https://github.com/afshinm/zerobox) | Seatbelt | Bubblewrap+seccomp | ~10ms, ~7MB | No nesting concept | Yes |
| [bubblewrap](https://github.com/containers/bubblewrap) | No | User namespaces | ~10ms | Parent limits apply | No |
| [OpenShell](https://github.com/NVIDIA/OpenShell) | Docker/Podman | Docker/Podman | Container startup | Container nesting | Yes |
| [sandbox-exec](https://developer.apple.com/library/archive/documentation/Security/Conceptual/AppSandboxDesignGuide/AppSandboxDesignGuide.pdf) | Seatbelt | No | Zero latency | N/A | No |
| Docker | Docker | Docker | Container startup | Container nesting | No |

## Constraints

1. **macOS and Linux must be supported.** apmw contributors develop on macOS
   (x86_64, arm64) and CI runs on Linux (x86_64). The sandbox must work on all
   four combinations. Windows is out of scope (nono does not support native
   Windows; WSL2 uses the Linux path).

2. **Lightweight.** The sandbox adds to every test invocation's startup cost.
   Target: under 50ms overhead per test. The full integration test suite
   (~70 tests) must complete within 1.5x the current unsandboxed runtime. nono
   claims zero-latency via OS primitives (no daemon, no container runtime).

3. **Filesystem restriction.** The sandboxed test subprocess may only write to
   the test's TempDir. Reads of system paths (`/usr`, `/lib`, `/etc`, cargo
   registry, pnpm store, rustup toolchain) are allowed. Reads of personal
   files (`$HOME/.ssh`, `$HOME/.aws`, `$HOME/.gnupg`, `$HOME/.config` outside
   package-manager paths, browser cookies) are denied.

4. **Network restriction.** The sandboxed test subprocess may only contact
   real package registries: `registry.npmjs.org`, `crates.io`,
   `static.crates.io`, `index.crates.io`, `github.com`,
   `raw.githubusercontent.com`, `objects.githubusercontent.com`. All other
   network access is denied — a malicious test package cannot exfiltrate data
   to an attacker-controlled endpoint.

5. **Real registry access required.** The user explicitly wants tests to
   verify real install behavior (not mocked registries). The profile must
   allow network access to the registries listed in constraint 4.

6. **Fallback on local dev.** If nono is not installed on a contributor's
   machine, `cargo test` must not fail solely because nono is absent. Tests
   fall back to unsandboxed execution with a one-line warning. The sandbox is
   defense-in-depth on local dev, not a hard gate.

7. **Mandatory in CI.** CI must never run unsandboxed tests. The CI
   environment installs nono and sets `APMW_TEST_SANDBOX_REQUIRED=1` so the
   harness panics if nono is absent.

8. **Hybrid TempDir approach.** Tests continue to use `tempfile::TempDir` for
   path management (automatic cleanup, deterministic paths). The sandbox
   restricts writes to that TempDir. TempDirs are created under
   `/tmp/apmw-tests/<test-name>/` for deterministic, greppable paths.

9. **Devbox compatibility.** Tests run inside `devbox run --` per the apmw
   AGENTS.md. The sandbox must function inside a devbox (Nix) environment.
   nono is added to `devbox.json` so `devbox shell` provides it.

10. **No test regressions.** All currently passing integration tests must
    continue to pass under the sandbox. Tests that genuinely require
    unrestricted access (e.g., writing intercept shims to the real
    `$HOME/.local/bin`) are rewritten to use a sandbox-local PATH.

## Decision

Use **nono** as the sandbox for all apmw integration tests on macOS and Linux.
Ship a **bundled nono profile** at `tests/sandbox/nono-profile.json` (with
per-test dynamic `fs_write` generation). On local dev without nono, fall back
to unsandboxed execution with a warning. In CI, the sandbox is mandatory
(`APMW_TEST_SANDBOX_REQUIRED=1`).

This decision aligns with the skills-src ADR
`adr-202608030953-skill-refresh-sandbox-selection`, which performed the full
candidate evaluation and chose nono. apmw adopts the same tool for
consistency.

**Sandbox invocation from the test harness:**

```rust
let mut cmd = Command::new("nono");
cmd.arg("run")
    .arg("--profile")
    .arg(&profile_path)
    .arg("--")
    .arg(apmw_binary_path)
    .args(args);
```

The apmw binary path is resolved via `env!("CARGO_BIN_EXE_apmw")`.

### nono profile schema

The bundled profile (`tests/sandbox/nono-profile.json`) restricts the test
subprocess:

```json
{
  "fs_read": [
    "TempDir",
    "/usr",
    "/lib",
    "/lib64",
    "/etc",
    "/tmp",
    "~/.cargo/registry",
    "~/.local/share/pnpm",
    "~/.config/pnpm",
    "~/.rustup"
  ],
  "fs_write": [
    "TempDir"
  ],
  "net_allow": [
    "registry.npmjs.org",
    "crates.io",
    "static.crates.io",
    "index.crates.io",
    "github.com",
    "raw.githubusercontent.com",
    "objects.githubusercontent.com"
  ],
  "net_deny": [
    "*"
  ],
  "env": [
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "TERM",
    "LANG",
    "LC_ALL",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "TMPDIR"
  ]
}
```

**Field semantics:**

| Field | Purpose |
|-------|---------|
| `fs_read` | Paths the sandboxed process may read. `TempDir` is replaced with the test's actual temp dir path at profile generation time. |
| `fs_write` | Paths the sandboxed process may write. Only the test's TempDir — no writes to `$HOME`, system paths, or other test dirs. |
| `net_allow` | Domains the sandboxed process may contact. Real package registries only. |
| `net_deny` | `*` — deny all network not in `net_allow`. |
| `env` | Environment variables passed through to the sandboxed process. Minimal allowlist — no `AWS_*`, `GITHUB_TOKEN`, or other secret-bearing vars. |

**Dynamic `fs_write` generation:** nono profiles are static JSON, but each
test needs a different TempDir path in `fs_write`. The harness generates a
per-test profile from a template struct (serialize to JSON, write to a temp
file, pass to `nono run --profile <temp-profile>`). This is more portable than
relying on nono env-var expansion in profiles.

### Fallback policy

```
┌──────────────────────────────────────────────────────────────────┐
│ Environment + nono detection                                      │
├──────────────────────────────────────────────────────────────────┤
│ CI (APMW_TEST_SANDBOX_REQUIRED=1) + nono present → sandboxed     │
│ CI (APMW_TEST_SANDBOX_REQUIRED=1) + nono absent  → PANIC         │
│ Local dev + nono present                          → sandboxed     │
│ Local dev + nono absent                           → unsandboxed   │
│                                                     + warning      │
│ Already in nono session (NONO_SESSION set)        → skip sandbox  │
│                                                     (parent handles)│
└──────────────────────────────────────────────────────────────────┘
```

The harness function `ensure_nono_or_skip()` checks if nono is installed. If
not, it prints a warning and returns `false` (local dev) or panics (CI, when
`APMW_TEST_SANDBOX_REQUIRED=1` is set). Tests call this at the top and return
early if `false`.

### Hybrid TempDir approach

Tests use `tempfile::TempDir` for path management, but the TempDir is created
under a deterministic path:

```
/tmp/apmw-tests/<test-name>/
```

This provides:

- **Automatic cleanup** — `TempDir`'s `Drop` impl removes the dir on test
  completion.
- **Deterministic paths** — greppable artifacts when a test fails and leaves
  the dir behind for inspection.
- **Sandbox write restriction** — the nono profile's `fs_write` is set to this
  path, so the sandboxed process can only write to the test's own dir.

The harness function `sandbox_tempdir(test_name: &str) -> TempDir` creates the
dir under `/tmp/apmw-tests/<test-name>/`.

## Rationale

### Why nono (adopting the skills-src ADR)

The skills-src ADR `adr-202608030953` performed a thorough evaluation of six
sandbox tools. The key differentiators that favor nono for apmw's test harness:

| Factor | nono | zerobox | bubblewrap | OpenShell | sandbox-exec | Docker |
|--------|------|---------|------------|-----------|--------------|--------|
| macOS + Linux | Both | Both | Linux only | Both (containers) | macOS only | Both |
| Nested sandboxing | Per-tool child sandboxes | No | Parent limits | Container nesting | N/A | Container nesting |
| L7 network filtering | Method + path level | Domain only | N/A | Yes | Domain level | N/A |
| Overhead | Zero latency | ~10ms | ~10ms | Container startup | Zero latency | Container startup |
| Per-tool sandboxing | Yes | No | No | Yes | No | No |
| Sigstore attestation | Yes | No | No | No | No | No |
| Profile system | Composable JSON | CLI flags | CLI flags | YAML | Seatbelt profile | N/A |
| Install | `brew install nono` / curl | `npm install -g` / cargo | distro package | Docker required | macOS built-in | Docker required |

**The nested sandboxing issue is decisive for the test harness.** When a
contributor runs `cargo test` inside a nono session (e.g., they launched their
editor via `nono run --profile agent-profile -- code`), nono's broker manages
child sandboxes for the test subprocesses. Each test gets its own child sandbox
with its own profile — separate filesystem grants, separate network rules.
The parent agent's broad grants don't leak into the test subprocess. zerobox
has no equivalent.

**nono's L7 filtering matters for the test use case.** Tests contact
`registry.npmjs.org` for package metadata and `github.com` for clone
operations. With nono, we can restrict to specific HTTP methods (GET only) and
paths — a compromised test package cannot POST exfiltrated data to a path on
the same domain.

**nono's zero-latency overhead is critical for ~70 tests.** The test suite
runs `apmw` as a subprocess for each test. A 10ms overhead per test (zerobox)
adds ~700ms to the suite; container startup (Docker, OpenShell) adds seconds
per test. nono's zero-latency OS primitives keep the suite fast.

### Why not sandbox-exec (macOS-only)

`sandbox-exec` is macOS's built-in Seatbelt sandbox launcher. It is
zero-latency and pre-installed on every macOS system. However:

1. **macOS-only.** CI runs on Linux (Ubuntu). Using sandbox-exec would mean
   one sandbox on macOS and a different sandbox on Linux — two profiles, two
   invocation patterns, two test behaviors. nono uses Seatbelt on macOS and
   Landlock on Linux with the same profile and invocation.

2. **No profile system.** sandbox-exec uses a Seatbelt profile language (SBPL)
   that is verbose, poorly documented, and macOS-specific. nono's JSON profiles
   are portable across macOS and Linux.

3. **No nested sandboxing support.** sandbox-exec does not have a nesting
   concept. If a contributor is already inside a nono session, sandbox-exec
   inside nono is untested and likely to conflict.

4. **Apple deprecated the public Seatbelt API.** `sandbox-exec` is present but
   not officially supported for third-party use. The Seatbelt library headers
   are private. nono wraps the same primitives with a stable interface.

### Why not devbox isolation (not a security sandbox)

devbox provides a Nix-based reproducible development environment. It isolates
the toolchain (rustc, cargo, nono) from the host, but it is **not a security
sandbox**:

1. **No filesystem restriction.** A process inside `devbox shell` can read
   `$HOME/.ssh`, `$HOME/.aws`, and any other host path. devbox isolates the
   toolchain, not the process's filesystem access.

2. **No network restriction.** devbox does not filter network access. A
   process inside devbox can contact any endpoint.

3. **No per-process sandboxing.** devbox provides an environment, not a
   process wrapper. There is no equivalent of `nono run --profile ... -- cmd`.

devbox is the environment provider (it supplies nono itself via
`devbox.json`), but it is not the sandbox. The two are complementary: devbox
ensures nono is installed; nono does the sandboxing.

### Why not Docker (too heavy)

Docker provides strong isolation via container namespaces. However:

1. **Container startup overhead.** Each `docker run` invocation takes 0.5–2
   seconds for container creation. With ~70 tests, this adds 35–140 seconds to
   the test suite — unacceptable for a fast feedback loop.

2. **Docker daemon dependency.** Docker requires a running daemon
   (`dockerd`). On macOS, Docker Desktop is a heavy VM. On CI, Docker is
   available but adds setup complexity. nono requires no daemon.

3. **Wrong granularity.** Docker sandboxes an entire container environment.
   We need to wrap a single command (`apmw <args>`) with filesystem and
   network restrictions. Docker's container model is overkill for this.

4. **Image management.** Docker requires building and maintaining a container
   image with the apmw binary inside. nono wraps the already-built binary
   directly.

Docker is a strong candidate for sandboxing the entire apmw daemon in
production. It is not the right tool for wrapping individual test subprocesses.

## Technical Approach

### Test harness module

A new module `tests/sandbox/mod.rs` (or `tests/common/sandbox.rs`) provides
three functions:

1. **`fn sandboxed_command(test_name: &str) -> Command`** — returns a
   `Command` that invokes `nono run --profile <generated-profile> -- <apmw-binary>`.
   The TempDir path is injected into the profile's `fs_write` field. The
   profile is generated per-test from a template struct, serialized to JSON,
   and written to a temp file.

2. **`fn ensure_nono_or_skip() -> bool`** — checks if nono is installed. If
   not, prints a warning and returns `false` (local dev) or panics (CI, when
   `APMW_TEST_SANDBOX_REQUIRED=1` is set). Tests call this at the top and
   return early if `false`.

3. **`fn sandbox_tempdir(test_name: &str) -> TempDir`** — wraps
   `TempDir::new()` but creates the temp dir under
   `/tmp/apmw-tests/<test-name>/` for deterministic, greppable paths.

### Per-test profile generation

```rust
#[derive(Serialize)]
struct NonoProfile {
    fs_read: Vec<String>,
    fs_write: Vec<String>,
    net_allow: Vec<String>,
    net_deny: Vec<String>,
    env: Vec<String>,
}

fn generate_profile(tempdir_path: &Path) -> PathBuf {
    let profile = NonoProfile {
        fs_read: vec![
            tempdir_path.display().to_string(),
            "/usr".into(), "/lib".into(), "/lib64".into(),
            "/etc".into(), "/tmp".into(),
            "~/.cargo/registry".into(),
            "~/.local/share/pnpm".into(),
            "~/.config/pnpm".into(),
            "~/.rustup".into(),
        ],
        fs_write: vec![tempdir_path.display().to_string()],
        net_allow: vec![
            "registry.npmjs.org".into(),
            "crates.io".into(),
            "static.crates.io".into(),
            "index.crates.io".into(),
            "github.com".into(),
            "raw.githubusercontent.com".into(),
            "objects.githubusercontent.com".into(),
        ],
        net_deny: vec!["*".into()],
        env: vec![
            "PATH".into(), "HOME".into(), "USER".into(),
            "SHELL".into(), "TERM".into(), "LANG".into(),
            "LC_ALL".into(), "CARGO_HOME".into(),
            "RUSTUP_HOME".into(),
            "XDG_CACHE_HOME".into(),
            "XDG_CONFIG_HOME".into(),
            "XDG_DATA_HOME".into(),
            "TMPDIR".into(),
        ],
    };
    // Serialize to JSON, write to a temp file, return the path.
    // ...
}
```

### CI integration

The GitHub Actions workflow (`.github/workflows/ci.yml`) installs nono in a
setup step:

```yaml
- name: Install nono
  run: curl -fsSL https://nono.sh/install.sh | sh

- name: Run tests (sandboxed)
  env:
    APMW_TEST_SANDBOX_REQUIRED: "1"
  run: just test_impl
```

### Intercept shim test rewrite

The current `test_install_intercept_creates_shims` installs shims to the host
PATH. Under the sandbox, the host PATH is restricted. The test is rewritten
to set `PATH=<tempdir>/bin` and verify apmw writes shims there. This tests the
logic without touching the real host.

### Nested session detection

If a contributor is already running inside a nono session
(`NONO_SESSION` env var is set), the harness skips its own sandbox layer —
the parent session's per-tool policy handles sandboxing. Running nono inside
nono is supported (nono's broker manages child sandboxes), but the parent's
policy may already restrict the test subprocess.

## Affected Components

| Component | Change |
|-----------|--------|
| `tests/integration_tests.rs` | All ~70 tests migrated from `Command::cargo_bin("apmw")` to `sandboxed_command()` |
| `tests/sandbox/mod.rs` | New harness module with `sandboxed_command()`, `ensure_nono_or_skip()`, `sandbox_tempdir()` |
| `tests/sandbox/nono-profile.json` | Bundled profile template (base schema) |
| `.github/workflows/ci.yml` | Install nono, set `APMW_TEST_SANDBOX_REQUIRED=1` |
| `devbox.json` | Add `nono` package |
| `justfile` | `just doctor` checks for nono presence/absence |
| `AGENTS.md` | Add sandbox testing notes |

## Consequences

### Positive

- **Personal files are isolated.** No integration test can read
  `$HOME/.ssh`, `$HOME/.aws`, `$HOME/.gnupg`, browser cookies, or env vars
  beyond the minimal allowlist. A malicious test package cannot exfiltrate
  secrets.
- **Network is restricted.** Tests can only contact real package registries.
  A compromised test package cannot phone home to an attacker-controlled
  endpoint.
- **Consistency with skills-src.** Both repos use nono with the same profile
  schema. Contributors familiar with one repo's sandbox understand the other.
- **CI enforces sandboxing.** CI never runs unsandboxed tests
  (`APMW_TEST_SANDBOX_REQUIRED=1`). No silent unsandboxed CI.
- **Deterministic temp paths.** `/tmp/apmw-tests/<test-name>/` makes test
  failures debuggable — artifacts are greppable and inspectable.
- **Zero-latency overhead.** nono's OS primitives add no measurable overhead
  to the test suite.

### Negative

- **nono is pre-1.0.** The profile schema may change. The harness's profile
  template struct must be updated if nono's schema evolves. This is
  documented in the maintenance notes.
- **Additional dev dependency.** Contributors must install nono
  (`brew install nono` or `curl -fsSL https://nono.sh/install.sh | sh`) for
  sandboxed tests. Without nono, tests run unsandboxed with a warning — not
  blocked, but not isolated.
- **Per-test profile generation.** Each test generates a profile JSON file
  with the TempDir path. This adds a small amount of I/O per test (writing the
  profile to a temp file). The overhead is negligible vs. the test itself.
- **Intercept shim tests require rewrite.** Tests that install shims to the
  real host PATH must be rewritten to use a sandbox-local PATH. This is
  additional work but produces better tests (no host side effects).
- **No native Windows support.** nono does not support native Windows. Windows
  tests run unsandboxed (consistent with the skills-src ADR's Windows policy).
  WSL2 uses the Linux/nono path.

### Neutral

- **Hybrid TempDir approach.** Tests continue to use `tempfile::TempDir` —
  the sandbox wraps the subprocess, TempDir manages the paths. No change to
  the test's path management logic, only to the command invocation.
- **Profile is intentionally minimal.** The profile covers only the
  integration test use case. New network endpoints (e.g., `pypi.org` for a
  future Python ecosystem test) require updating the profile template.

## Alternatives Considered

### sandbox-exec (macOS built-in)

**Rejected:** macOS-only. CI runs on Linux. Using sandbox-exec would require
two sandbox implementations (macOS + Linux) with different profiles and
invocation patterns. nono wraps the same OS primitives (Seatbelt on macOS,
Landlock on Linux) with a single profile and invocation. See Rationale §"Why
not sandbox-exec" for details.

### devbox isolation (Nix environment)

**Rejected:** Not a security sandbox. devbox isolates the toolchain, not the
process's filesystem or network access. A process inside `devbox shell` can
read `$HOME/.ssh` and contact any endpoint. devbox is the environment provider
(it supplies nono via `devbox.json`), but it is not the sandbox. See
Rationale §"Why not devbox isolation" for details.

### Docker (container isolation)

**Rejected:** Too heavy. Container startup adds 0.5–2 seconds per test
invocation. With ~70 tests, this adds 35–140 seconds to the suite. Docker
requires a running daemon and image management. nono wraps the already-built
binary with zero-latency OS primitives. See Rationale §"Why not Docker" for
details.

### zerobox (process sandbox)

**Rejected:** No nested sandboxing. If a contributor runs tests inside a nono
session, zerobox inside nono is untested. zerobox's domain-level network
filtering (no L7 method/path filtering) is weaker than nono's. The skills-src
ADR already evaluated and rejected zerobox in favor of nono. See the skills-src
ADR §"Why not zerobox (revised)" for the full rationale.

### bubblewrap (Linux namespaces)

**Rejected:** Linux-only. No macOS support. Already used as the Linux backend
by zerobox and available to nono. Using bubblewrap directly would require a
separate macOS solution. nono provides a unified cross-platform interface.

### OpenShell (NVIDIA, container/MicroVM)

**Rejected:** Requires Docker/Podman/MicroVM. Container startup overhead
violates the lightweight constraint. OpenShell is designed for sandboxing
entire agent sessions with credential injection and inference routing —
overkill for wrapping individual test subprocesses. Alpha software. See the
skills-src ADR §"Why not OpenShell" for details.

## Rollout / Migration

1. **ADR creation** (this document) — record the decision.
2. **Harness module** — implement `tests/sandbox/mod.rs` with
   `sandboxed_command()`, `ensure_nono_or_skip()`, `sandbox_tempdir()`.
3. **nono profile** — implement the profile template struct and per-test
   profile generation.
4. **Test migration** — migrate all ~70 integration tests from
   `Command::cargo_bin("apmw")` to `sandboxed_command()`. Tests continue to
   pass unsandboxed if nono is absent (fallback policy).
5. **Intercept shim rewrite** — rewrite `test_install_intercept_creates_shims`
   and `test_uninstall_intercept_removes_shims` to use a sandbox-local PATH.
6. **Isolation verification tests** — add `test_sandbox_denies_home_ssh`
   (confirms personal paths are denied) and `test_sandbox_allows_real_registries`
   (confirms registries are reachable).
7. **CI + devbox update** — add nono to `devbox.json`, CI workflow
   (install nono, set `APMW_TEST_SANDBOX_REQUIRED=1`), `just doctor`.
8. **Documentation** — update `AGENTS.md` with sandbox testing notes.

The migration is incremental — tests can be migrated one at a time. The
harness's fallback policy ensures `cargo test` never breaks during migration
(even without nono installed).

## To Investigate

- **nono env-var expansion in profiles.** If nono supports env-var expansion
  in profile JSON (e.g., `"fs_write": ["${APMW_TEST_TEMPDIR}"]`), the harness
  could pass the TempDir path via an env var instead of generating a per-test
  profile file. This would simplify the harness. Verify in nono docs.
- **nono L7 filtering for test profile.** The current profile uses domain-level
  `net_allow`. nono supports method + path level filtering. Investigate
  restricting to GET-only for registry metadata and specific paths for GitHub
  clone operations.
- **nono Sigstore attestation for test profile.** The skills-src ADR notes
  nono's Sigstore-signed profiles. Investigate signing the test profile for
  additional tamper resistance.
- **Parallel test execution under sandbox.** `cargo test` runs tests in
  parallel by default. Verify that multiple nono sessions can run concurrently
  without interference (each test gets its own TempDir and profile).

## Validation

| Check | Method | Expected result |
|-------|--------|-----------------|
| ADR exists | `test -f internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md` | File present |
| ADR references skills-src ADR | `grep adr-202608030953 <adr-file>` | Match found |
| ADR decision is nono | `grep -i nono <adr-file>` | Match found |
| ADR has profile schema | `grep fs_read <adr-file>` | Match found |
| ADR has fallback policy | `grep APMW_TEST_SANDBOX_REQUIRED <adr-file>` | Match found |
| ADR has hybrid approach | `grep /tmp/apmw-tests <adr-file>` | Match found |
| No AI attribution | `grep -iE 'Generated by|Co-Authored-By' <adr-file>` | No match |

## Review Schedule

- **Next review:** 2027-02-05 (6 months from acceptance)
- **Triggers for earlier review:**
  - nono releases a 1.0 with schema changes
  - A test fails due to sandbox restrictions that cannot be resolved within
    the current profile
  - A new platform is added to the CI matrix (e.g., Windows native)
  - A security incident related to test isolation

## Notes

- This ADR is the first ADR in the apmw repository. The
  `internal-docs/adr/` directory did not exist before this ADR. The directory
  structure follows the skills-src convention: `internal-docs/adr/YYYY/MM/`.
- The ADR format (YAML frontmatter + sections) follows the skills-src ADR
  `adr-202608030953` as the template.
- The nono profile schema may evolve as nono is pre-1.0. The harness's profile
  template struct is the single point of update if the schema changes.
- New integration tests must use `sandboxed_command()` — not
  `Command::cargo_bin("apmw")` directly. A code review convention enforces
  this. A future lint check (custom clippy lint or CI grep) could automate
  enforcement.
- New network endpoints (e.g., `pypi.org` for a Python ecosystem test) require
  updating the profile template struct's `net_allow` list. Reviewers should
  scrutinize the `fs_read` allowlist — every allowed path is a potential leak
  vector. The `env` allowlist — every allowed env var is accessible to the
  sandboxed process.

## References

- [skills-src ADR: Skill Refresh Sandbox Selection](https://github.com/levonk/skills-src/blob/main/internal-docs/adr/2026/08/adr-202608030953-skill-refresh-sandbox-selection.md) — the prior evaluation that chose nono
- [nono](https://github.com/nolabs-ai/nono) — lightweight process sandbox (Seatbelt on macOS, Landlock on Linux)
- [apmw PRD: Sandboxed Integration Test Harness](https://github.com/levonk/apmw/blob/main/internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md) — feature requirements
- [apmw AGENTS.md](https://github.com/levonk/apmw/blob/main/AGENTS.md) — project conventions
- [Seatbelt (macOS sandbox)](https://developer.apple.com/library/archive/documentation/Security/Conceptual/AppSandboxDesignGuide/AppSandboxDesignGuide.pdf) — macOS sandbox primitive
- [Landlock (Linux sandbox)](https://docs.kernel.org/userspace-api/landlock.html) — Linux sandbox primitive
