---
story_id: "03-003"
story_title: "Initial scanner plugins (cargo audit, npm audit, pip-audit, osv-scanner)"
story_name: "scanner-plugins"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 3
parallel_id: 3
branch: "feature/current/apmw/story-03-003-scanner-plugins"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["03-002"]
parallel_safe: true
modules: ["src/security/plugins/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "security", "scanners", "plugins"]
due: "2026-09-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Implement the initial set of scanner plugins that implement the Scanner trait from story 03-002: cargo audit (for Rust), npm audit (for Node), pip-audit (for Python), and osv-scanner (multi-ecosystem). Each plugin follows the scanner contract: available, ensure, scan.

## Current State

- **Relevant files and their roles:**
  - `src/security/scanner.rs` — Scanner trait (from story 03-002)
  - `src/security/plugins/mod.rs` — Plugin registry (from story 03-002)
- **Repository conventions:** Implement traits for plugins. Use subprocess calls for external scanners.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/security/plugins/cargo_audit.rs` — cargo audit scanner plugin
- Create `src/security/plugins/npm_audit.rs` — npm audit scanner plugin
- Create `src/security/plugins/pip_audit.rs` — pip-audit scanner plugin
- Create `src/security/plugins/osv_scanner.rs` — osv-scanner scanner plugin
- Each plugin implements the Scanner trait: available (check if binary is on PATH), ensure (install if missing), scan (run the scanner and parse results)
- Parse scanner output into ScanResult (Safe, Risky with findings, Error)
- Register all plugins in the plugin registry
- Add unit tests with mock scanner output
- Add integration tests that run scanners against test projects

**Out of scope:**
- Security scanning orchestrator (story 03-002)
- Additional scanner plugins (future)

## Sub-Tasks

- [ ] Create `src/security/plugins/cargo_audit.rs` implementing Scanner trait
  **Verify**: `cargo test cargo_audit` → tests pass
- [ ] Create `src/security/plugins/npm_audit.rs` implementing Scanner trait
  **Verify**: `cargo test npm_audit` → tests pass
- [ ] Create `src/security/plugins/pip_audit.rs` implementing Scanner trait
  **Verify**: `cargo test pip_audit` → tests pass
- [ ] Create `src/security/plugins/osv_scanner.rs` implementing Scanner trait
  **Verify**: `cargo test osv_scanner` → tests pass
- [ ] Register all plugins in the plugin registry
  **Verify**: `cargo test plugin_registry` → tests pass
- [ ] Add unit tests with mock scanner output for each plugin
  **Verify**: `just test` → all pass
- [ ] Add integration tests that run scanners against test projects
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/security/plugins/cargo_audit.rs` — cargo audit scanner
- `src/security/plugins/npm_audit.rs` — npm audit scanner
- `src/security/plugins/pip_audit.rs` — pip-audit scanner
- `src/security/plugins/osv_scanner.rs` — osv-scanner scanner
- `src/security/plugins/mod.rs` — Register plugins

## Acceptance Criteria

- [ ] cargo audit plugin correctly reports Safe/Risky/Error
- [ ] npm audit plugin correctly reports Safe/Risky/Error
- [ ] pip-audit plugin correctly reports Safe/Risky/Error
- [ ] osv-scanner plugin correctly reports Safe/Risky/Error
- [ ] Each plugin checks availability (binary on PATH)
- [ ] Each plugin can ensure (install if missing)
- [ ] All plugins are registered in the plugin registry
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test plugins` — all plugin tests with mock output
- Integration: Run scanners against test projects with known vulnerabilities
- Lint: `just lint`

## Observability

- Scanner availability should be logged at `debug` level
- Scanner findings should be logged at `warn` level
- Scanner errors should be logged at `error` level

## Compliance

- PRD FR-5.2 (scanner plugin contract)
- skill-install.sh scanner contract pattern

## Risks & Mitigations

- Risk: Scanners may not be installed on the system — Mitigation: The ensure() method installs missing scanners; if installation fails, the scanner is skipped with an incomplete-scan warning

## Dependencies & Sequencing

- Depends on: 03-002 (security scanning orchestrator)
- Unblocks: 04-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- A scanner binary is not available and cannot be installed
- Scanner output format is not parseable

## Maintenance Notes

- New scanner plugins can be added by implementing the Scanner trait and registering
- Reviewers should check that scanner output is parsed correctly

## Commit Conventions

- `feat(security): add cargo audit, npm audit, pip-audit, osv-scanner plugins`
