---
story_id: "03-002"
story_title: "Security scanning orchestrator (two-phase)"
story_name: "security-scanning"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 3
parallel_id: 2
branch: "feature/current/apmw/story-03-002-security-scanning"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001"]
parallel_safe: true
modules: ["src/security/"]
priority: "MUST"
risk_level: "high"
tags: ["feat", "security", "scanning", "two-phase"]
due: "2026-09-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the security scanning orchestrator implementing the two-phase scan-all-then-install-all pattern from `executable_skill-install.sh`. The orchestrator loads scanner plugins following the scanner contract (`_available`, `_ensure`, `_scan`), uses OR semantics for risk aggregation, supports `--on-risk` modes (prompt, error, warn, skip), supports `--update-security-db`, and enforces the telemetry policy (off for third-party, on/neutral for levonk-owned).

## Current State

- **Relevant files and their roles:**
  - PRD FR-5 (sections FR-5.1 through FR-5.7) — Security scanning requirements
  - `executable_skill-install.sh` at `~/p/gh/levonk/dotfiles/home/current/dot_local/bin/executable_skill-install.sh` — Two-phase scan pattern reference
  - `npmrc.tmpl` at `~/p/gh/levonk/dotfiles/home/current/.chezmoitemplates/config/npm/npmrc.tmpl` — Security posture reference
  - `src/error.rs` — Has `SecurityScanFailed` error variant
- **Existing code excerpts (from skill-install.sh):**
  ```bash
  # Two-phase: scan all -> install all (lines 12-14)
  # Scanner plugin contract (lines 19-23):
  #   <name>_available  — can this scanner run?
  #   <name>_ensure     — prepare/install the scanner
  #   <name>_scan DIR   — scan a local directory, return 0=safe/1=risky/2=error
  # Risk aggregation: OR semantics (lines 25-27)
  # --on-risk modes: prompt, error, warn, skip (lines 55-59)
  # Telemetry: off for third-party, on for levonk-owned (lines 73-76)
  ```
- **Repository conventions:** Module by feature/domain. Use traits for plugin contracts.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/security/mod.rs` — Security scanning orchestrator
- Create `src/security/scanner.rs` — Scanner plugin trait (available, ensure, scan)
- Create `src/security/plugins/mod.rs` — Plugin registry
- Implement two-phase scan: scan all packages first, then install all (never interleave)
- Implement scanner plugin trait: `fn available(&self) -> bool`, `fn ensure(&mut self) -> Result<()>`, `fn scan(&self, dir: &Path) -> ScanResult`
- Implement ScanResult enum: Safe, Risky(findings), Error(message)
- Implement risk aggregation with OR semantics: if ANY scanner reports Risky, the package is risky
- Implement `--on-risk` modes: Prompt (ask in interactive TTY, error if not), Error (abort), Warn (continue), Skip (skip package)
- Implement `--update-security-db` flag: refresh security databases before scanning
- Implement `--no-scan` flag: skip all scanning (install immediately)
- Implement `--scan-only` flag: scan but do not install
- Implement telemetry policy: off for third-party, on/neutral for levonk-owned
- Implement security posture from npmrc.tmpl: audit=true, audit-level=high, engine-strict=true, frozen-lockfile=true, save-exact=true, provenance=true
- Add unit tests with mock scanners
- Add integration tests with assert_cmd

**Out of scope:**
- Individual scanner plugin implementations (story 03-003)
- Install engine (story 04-001)

## Sub-Tasks

- [ ] Create `src/security/scanner.rs` with Scanner trait (available, ensure, scan)
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/security/mod.rs` with ScanOrchestrator (two-phase, risk aggregation)
  **Verify**: `cargo test orchestrator` → tests pass
- [ ] Implement ScanResult enum (Safe, Risky, Error) with serde
  **Verify**: `cargo test scan_result` → tests pass
- [ ] Implement OR semantics risk aggregation
  **Verify**: `cargo test risk_aggregation` → tests pass
- [ ] Implement --on-risk modes (Prompt, Error, Warn, Skip)
  **Verify**: `cargo test on_risk_modes` → tests pass
- [ ] Implement --update-security-db flag
  **Verify**: `cargo test update_db` → tests pass
- [ ] Implement --no-scan and --scan-only flags
  **Verify**: `cargo test no_scan` → tests pass
- [ ] Implement telemetry policy (off for third-party, on for levonk-owned)
  **Verify**: `cargo test telemetry` → tests pass
- [ ] Create `src/security/plugins/mod.rs` with plugin registry
  **Verify**: `cargo test plugins` → tests pass
- [ ] Add unit tests with mock scanners
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/security/mod.rs` — Scan orchestrator
- `src/security/scanner.rs` — Scanner plugin trait
- `src/security/plugins/mod.rs` — Plugin registry
- `src/lib.rs` — Add `pub mod security;`

## Acceptance Criteria

- [ ] Two-phase scan: all packages scanned before any are installed
- [ ] Scanner plugin trait has available, ensure, scan methods
- [ ] OR semantics: any scanner reporting Risky makes the package risky
- [ ] --on-risk modes work correctly (prompt, error, warn, skip)
- [ ] --update-security-db refreshes databases before scanning
- [ ] --no-scan skips all scanning
- [ ] --scan-only scans but does not install
- [ ] Telemetry is off for third-party, on for levonk-owned
- [ ] Security posture mirrors npmrc.tmpl settings
- [ ] All unit tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test security` — all security module tests with mock scanners
- Integration: `assert_cmd` tests for `apmw scan`
- Lint: `just lint`

## Observability

- Scan results should be logged at `info` level
- Risky findings should be logged at `warn` level
- Scanner errors should be logged at `error` level
- Telemetry policy decisions should be logged at `debug` level

## Compliance

- PRD FR-5 (security scanning requirements)
- skill-install.sh two-phase scan pattern
- npmrc.tmpl security posture

## Risks & Mitigations

- Risk: Scanner plugins may have conflicting results — Mitigation: OR semantics ensures the most cautious result wins
- Risk: Two-phase scan may be slow for many packages — Mitigation: Run scanners in parallel within the scan phase

## Dependencies & Sequencing

- Depends on: 01-001 (error types)
- Unblocks: 04-001, 06-002

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The scanner plugin trait cannot be implemented as a Rust trait with dynamic dispatch
- The two-phase pattern cannot be enforced without interleaving

## Maintenance Notes

- New scanner plugins are added by implementing the Scanner trait and registering in the plugin registry
- Reviewers should verify that two-phase scan is never bypassed except via explicit --no-scan
- Security posture settings should be kept in sync with npmrc.tmpl

## Commit Conventions

- `feat(security): add two-phase scan orchestrator with plugin trait and on-risk modes`
