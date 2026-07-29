---
story_id: "04-001"
story_title: "Add engine (runtime + --dev deps, install-on-use, devbox+rtk routing, --manager override)"
story_name: "install-engine"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 4
parallel_id: 1
branch: "feature/current/apmw/story-04-001-add-engine"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-002", "01-003", "01-005", "02-001", "02-002", "02-003", "02-004", "03-001", "03-002", "03-003"]
parallel_safe: false
modules: ["src/install/"]
priority: "MUST"
risk_level: "high"
tags: ["feat", "add", "install-on-use", "devbox", "dev-deps"]
due: "2026-09-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the add engine that implements install-on-use semantics. The engine detects the package manager, resolves the version, scans PATH (skip if already installed), runs security scanning (two-phase), routes through devbox + rtk when no impossible barriers exist, uses the canonical ad-hoc runner per ecosystem (via cli-tool-discovery --runner), and writes an audit log entry.

The primary command is `apmw add` (not `apmw install`). The engine supports:
- `--dev` flag for development/build-time dependencies (mapped to each package manager's dev-dep mechanism: pnpm `-D`, cargo `--dev`, pip `--group dev`, npm `--save-dev`, etc.)
- `--manager <name>` override to force a specific package manager (from story 02-004), skipping auto-detection
- min-age-days supply-chain defense integration (from story 03-001): refuse versions published < 2 days ago during auto-resolution

## Current State

- **Relevant files and their roles:**
  - `src/main.rs` (lines 57-60) — `Install` command stub: `println!("Installing: {package}");` (to be renamed to `Add`)
  - PRD FR-1.1 — `apmw add` primary command
  - PRD FR-1.2 — `--dev` flag for development/build-time deps
  - PRD FR-1.3 — `--manager <name>` override
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
- Create `src/install/mod.rs` — Add engine orchestrator
- Create `src/install/on_use.rs` — Install-on-use semantics (detect -> scan PATH -> resolve version -> security scan -> add)
- Create `src/install/runner.rs` — Ad-hoc runner resolution (cli-tool-discovery --runner integration)
- Create `src/install/dev.rs` — `--dev` flag mapping to per-manager dev-dep mechanisms
- Wire the add engine to: detection engine (02-001), PATH scanner (02-002), ecosystem mapper (02-003), manager override (02-004), version resolver (03-001), security orchestrator (03-002/03-003), audit logger (01-005)
- Implement devbox + rtk routing: when no impossible barriers exist, route through devbox + rtk
- Implement the canonical ad-hoc runner per ecosystem (cli-tool-discovery --runner <ecosystem>)
- Implement `--dev` flag: map to each package manager's dev-dep mechanism:
  - pnpm: `pnpm add -D <pkg>`
  - npm: `npm install --save-dev <pkg>`
  - yarn: `yarn add --dev <pkg>`
  - cargo: `cargo add --dev <pkg>`
  - pip/uv: `uv pip install --group dev <pkg>` / `pip install --group dev <pkg>`
  - poetry: `poetry add --group dev <pkg>`
  - go: `go get -t <pkg>` (test/dev deps)
- Implement `--manager <name>` override support: when the flag is passed, skip detection and use the forced manager
- Integrate min-age-days supply-chain defense from version resolver (03-001)
- Wire `apmw add <package>` command to the add engine
- Output add result in TOON format (agent mode) or human-readable (human mode)
- Write audit log entry for each add operation
- Support daemon mode: long-running adds run as background jobs
- Add integration tests with assert_cmd for `apmw add`

**Out of scope:**
- Alternative suggestions UI (story 04-002)
- Historyless clone (story 05-001)
- MCP server (story 06-001)

## Sub-Tasks

- [ ] Create `src/install/mod.rs` with AddEngine orchestrator
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/install/on_use.rs` with install-on-use flow (detect -> scan -> resolve -> security -> add)
  **Verify**: `cargo test on_use` → tests pass
- [ ] Create `src/install/runner.rs` with ad-hoc runner resolution (cli-tool-discovery --runner)
  **Verify**: `cargo test runner` → tests pass
- [ ] Create `src/install/dev.rs` with `--dev` flag mapping to per-manager dev-dep mechanisms
  **Verify**: `cargo test dev_deps` → tests pass
- [ ] Wire add engine to detection, PATH scan, ecosystem mapping, manager override, version resolution, security scanning
  **Verify**: `cargo test integration` → tests pass
- [ ] Implement devbox + rtk routing (route through devbox when available)
  **Verify**: `cargo test devbox_routing` → tests pass
- [ ] Implement `--manager <name>` override support (skip detection when flag present)
  **Verify**: `cargo test manager_override_integration` → tests pass
- [ ] Integrate min-age-days supply-chain defense from version resolver
  **Verify**: `cargo test min_age_integration` → tests pass
- [ ] Wire `apmw add <package>` command
  **Verify**: `apmw add <test-package>` → adds correctly
- [ ] Wire `apmw add <package> --dev` command
  **Verify**: `apmw add <test-package> --dev` → adds as dev dependency
- [ ] Output add result in TOON format
  **Verify**: `apmw add <test-package>` → valid TOON output
- [ ] Write audit log entry for each add
  **Verify**: audit log contains entry after add
- [ ] Support daemon mode for long-running adds (background jobs)
  **Verify**: `apmw --daemon add <large-package>` → returns job ID
- [ ] Add integration tests with assert_cmd
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/install/mod.rs` — Add engine
- `src/install/on_use.rs` — Install-on-use semantics
- `src/install/runner.rs` — Ad-hoc runner resolution
- `src/install/dev.rs` — `--dev` flag mapping to per-manager dev-dep mechanisms
- `src/lib.rs` — Add `pub mod install;`
- `src/main.rs` — Wire Add command

## Acceptance Criteria

- [ ] Add engine follows the install-on-use flow: detect -> scan PATH -> resolve version -> security scan -> add
- [ ] `apmw add` is the primary command (not `apmw install`)
- [ ] `--dev` flag maps to the correct dev-dep mechanism for each package manager
- [ ] `--manager <name>` override skips detection and forces the specified manager
- [ ] min-age-days supply-chain defense is integrated (refuses too-new versions)
- [ ] PATH scan skips installation if tool is already present
- [ ] Security scanning runs before add (two-phase)
- [ ] Version resolution uses the correct strategy
- [ ] Ecosystem mapping routes to the canonical manager
- [ ] devbox + rtk routing works when available
- [ ] Ad-hoc runner resolution uses cli-tool-discovery --runner
- [ ] Audit log entry is written for each add
- [ ] Output is in TOON format in agent mode
- [ ] Daemon mode runs long adds as background jobs
- [ ] All integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test install` — add engine tests
- Integration: `assert_cmd` tests for `apmw add` in various project types
- Lint: `just lint`

## Observability

- Add lifecycle events should be logged at `info` level
- PATH scan skips should be logged at `info` level
- Security scan results should be logged at `warn` level for risky packages
- Add failures should be logged at `error` level
- `--dev` flag usage should be logged at `info` level
- `--manager` override usage should be logged at `info` level
- min-age-days refusals should be logged at `warn` level

## Compliance

- PRD FR-1.1 (`apmw add` primary command), FR-1.2 (`--dev` flag), FR-1.3 (`--manager` override), FR-2, FR-3, FR-4, FR-5 (all add-related requirements)
- ADR-20260607001 section 13 (daemon mode for long operations)

## Risks & Mitigations

- Risk: Add may fail due to missing system dependencies — Mitigation: Report clear errors with suggestions, do not leave partial installs
- Risk: devbox routing may not work in all environments — Mitigation: Fall back to direct installation if devbox is unavailable
- Risk: `--dev` mapping may differ across package manager versions — Mitigation: Test against current versions of each manager; document the mapping table

## Dependencies & Sequencing

- Depends on: 01-002 (CLI), 01-003 (AXI output), 01-005 (audit), 02-001 (detection), 02-002 (PATH scan), 02-003 (ecosystem), 02-004 (manager override), 03-001 (version + min-age-days), 03-002 (security), 03-003 (scanners)
- Unblocks: 05-001, 06-001, 06-003

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The install-on-use flow cannot be implemented without interleaving scan and add
- devbox + rtk routing has impossible barriers that cannot be bypassed
- cli-tool-discovery --runner does not work as documented
- `--dev` mapping cannot be determined for a supported package manager

## Maintenance Notes

- New package manager add commands can be added by extending the ecosystem mapping
- Reviewers should verify that security scanning is never bypassed except via --no-scan
- Add engine should handle concurrent adds safely in daemon mode
- The `--dev` mapping table should be kept in sync as package managers evolve

## Commit Conventions

- `feat(add): add install-on-use engine with --dev deps, --manager override, devbox routing, and security scanning`
