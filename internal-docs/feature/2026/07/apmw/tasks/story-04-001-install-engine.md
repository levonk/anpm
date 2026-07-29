---
story_id: "04-001"
story_title: "Install engine (install-on-use, devbox+rtk routing)"
story_name: "install-engine"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 4
parallel_id: 1
branch: "feature/current/apmw/story-04-001-install-engine"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-002", "01-003", "01-005", "02-001", "02-002", "02-003", "03-001", "03-002", "03-003"]
parallel_safe: false
modules: ["src/install/"]
priority: "MUST"
risk_level: "high"
tags: ["feat", "install", "install-on-use", "devbox"]
due: "2026-09-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the install engine that implements install-on-use semantics. The engine detects the package manager, resolves the version, scans PATH (skip if already installed), runs security scanning (two-phase), routes through devbox + rtk when no impossible barriers exist, uses the canonical ad-hoc runner per ecosystem (via cli-tool-discovery --runner), and writes an audit log entry.

## Current State

- **Relevant files and their roles:**
  - `src/main.rs` (lines 57-60) — `Install` command stub: `println!("Installing: {package}");`
  - PRD FR-2, FR-3, FR-4, FR-5 — Ecosystem mapping, PATH scan, version resolution, security scanning
  - All Phase 02 and 03 modules provide the building blocks
- **Repository conventions:** Module by feature/domain. Use tokio for async operations.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/install/mod.rs` — Install engine orchestrator
- Create `src/install/on_use.rs` — Install-on-use semantics (detect -> scan PATH -> resolve version -> security scan -> install)
- Create `src/install/runner.rs` — Ad-hoc runner resolution (cli-tool-discovery --runner integration)
- Wire the install engine to: detection engine (02-001), PATH scanner (02-002), ecosystem mapper (02-003), version resolver (03-001), security orchestrator (03-002/03-003), audit logger (01-005)
- Implement devbox + rtk routing: when no impossible barriers exist, route through devbox + rtk
- Implement the canonical ad-hoc runner per ecosystem (cli-tool-discovery --runner <ecosystem>)
- Wire `apmw install <package>` command to the install engine
- Output install result in TOON format (agent mode) or human-readable (human mode)
- Write audit log entry for each install operation
- Support daemon mode: long-running installs run as background jobs
- Add integration tests with assert_cmd for `apmw install`

**Out of scope:**
- Alternative suggestions UI (story 04-002)
- Historyless clone (story 05-001)
- MCP server (story 06-001)

## Sub-Tasks

- [ ] Create `src/install/mod.rs` with InstallEngine orchestrator
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/install/on_use.rs` with install-on-use flow (detect -> scan -> resolve -> security -> install)
  **Verify**: `cargo test on_use` → tests pass
- [ ] Create `src/install/runner.rs` with ad-hoc runner resolution (cli-tool-discovery --runner)
  **Verify**: `cargo test runner` → tests pass
- [ ] Wire install engine to detection, PATH scan, ecosystem mapping, version resolution, security scanning
  **Verify**: `cargo test integration` → tests pass
- [ ] Implement devbox + rtk routing (route through devbox when available)
  **Verify**: `cargo test devbox_routing` → tests pass
- [ ] Wire `apmw install <package>` command
  **Verify**: `apmw install <test-package>` → installs correctly
- [ ] Output install result in TOON format
  **Verify**: `apmw install <test-package>` → valid TOON output
- [ ] Write audit log entry for each install
  **Verify**: audit log contains entry after install
- [ ] Support daemon mode for long-running installs (background jobs)
  **Verify**: `apmw --daemon install <large-package>` → returns job ID
- [ ] Add integration tests with assert_cmd
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/install/mod.rs` — Install engine
- `src/install/on_use.rs` — Install-on-use semantics
- `src/install/runner.rs` — Ad-hoc runner resolution
- `src/lib.rs` — Add `pub mod install;`
- `src/main.rs` — Wire Install command

## Acceptance Criteria

- [ ] Install engine follows the install-on-use flow: detect -> scan PATH -> resolve version -> security scan -> install
- [ ] PATH scan skips installation if tool is already present
- [ ] Security scanning runs before install (two-phase)
- [ ] Version resolution uses the correct strategy
- [ ] Ecosystem mapping routes to the canonical manager
- [ ] devbox + rtk routing works when available
- [ ] Ad-hoc runner resolution uses cli-tool-discovery --runner
- [ ] Audit log entry is written for each install
- [ ] Output is in TOON format in agent mode
- [ ] Daemon mode runs long installs as background jobs
- [ ] All integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test install` — install engine tests
- Integration: `assert_cmd` tests for `apmw install` in various project types
- Lint: `just lint`

## Observability

- Install lifecycle events should be logged at `info` level
- PATH scan skips should be logged at `info` level
- Security scan results should be logged at `warn` level for risky packages
- Install failures should be logged at `error` level

## Compliance

- PRD FR-2, FR-3, FR-4, FR-5 (all install-related requirements)
- ADR-20260607001 section 13 (daemon mode for long operations)

## Risks & Mitigations

- Risk: Install may fail due to missing system dependencies — Mitigation: Report clear errors with suggestions, do not leave partial installs
- Risk: devbox routing may not work in all environments — Mitigation: Fall back to direct installation if devbox is unavailable

## Dependencies & Sequencing

- Depends on: 01-002 (CLI), 01-003 (AXI output), 01-005 (audit), 02-001 (detection), 02-002 (PATH scan), 02-003 (ecosystem), 03-001 (version), 03-002 (security), 03-003 (scanners)
- Unblocks: 05-001, 06-001, 06-003

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The install-on-use flow cannot be implemented without interleaving scan and install
- devbox + rtk routing has impossible barriers that cannot be bypassed
- cli-tool-discovery --runner does not work as documented

## Maintenance Notes

- New package manager install commands can be added by extending the ecosystem mapping
- Reviewers should verify that security scanning is never bypassed except via --no-scan
- Install engine should handle concurrent installs safely in daemon mode

## Commit Conventions

- `feat(install): add install-on-use engine with devbox routing and security scanning`
