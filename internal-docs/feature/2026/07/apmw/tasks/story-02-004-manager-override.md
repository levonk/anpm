---
story_id: "02-004"
story_title: "`--manager <name>` CLI override"
story_name: "manager-override"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 2
parallel_id: 4
branch: "feature/current/apmw/story-02-004-manager-override"
status: "done"
assignee: ""
reviewer: ""
dependencies: ["01-002", "02-001"]
parallel_safe: true
modules: ["src/cli.rs"]
priority: "MUST"
risk_level: "low"
tags: ["feat", "cli", "manager", "override"]
due: "2026-08-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Add a `--manager <name>` (alias `--use <name>`) CLI flag that overrides auto-detection and forces a specific package manager. When passed, detection is skipped entirely and the specified manager is used. The override is recorded in the audit log. Valid values cover the full set of supported package managers across all ecosystems.

## Current State

- **Relevant files and their roles:**
  - `src/cli.rs` — CLI argument parsing (from story 01-002)
  - `src/detect/` — Detection engine (from story 02-001)
  - PRD FR-1.3 — `--manager <name>` override
- **Repository conventions:** Module by feature/domain. Use clap for CLI parsing.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Add `--manager <name>` (alias `--use <name>`) global CLI flag to `src/cli.rs`
- Define valid manager values: pnpm, npm, yarn, bun, uv, pip, poetry, pipenv, pdm, conda, cargo, go, gem, brew, nix, devbox, apt, dnf, pacman, winget, snap, flatpak, helm, docker, podman, maven, gradle, sbt, dotnet
- When `--manager` is passed, skip auto-detection and use the specified manager directly
- Validate the manager name; error with a clear message listing valid options if invalid
- Record the override in the audit log (manager name, source: `cli-override`)
- Wire the override into the detection flow so downstream consumers receive the forced manager
- Add unit tests for valid overrides, invalid names, and audit log recording

**Out of scope:**
- Detection engine internals (story 02-001)
- Install/add engine (story 04-001)
- Governance enforcement (story 04-003)

## Sub-Tasks

- [x] Add `--manager <name>` (alias `--use <name>`) flag to CLI definition
  **Verify**: `cargo check` → exit 0
- [x] Define valid manager value set and validation logic
  **Verify**: `cargo test manager_validation` → tests pass
- [x] Wire override into detection flow (skip detection when flag present)
  **Verify**: `cargo test manager_override` → tests pass
- [x] Record override in audit log
  **Verify**: `cargo test manager_override_audit` → tests pass
- [x] Add unit tests for valid overrides (pnpm, uv, cargo, docker, etc.)
  **Verify**: `just test` → all pass
- [x] Add unit test for invalid manager name (error lists valid options)
  **Verify**: `cargo test invalid_manager` → tests pass
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/cli.rs` — CLI flag definition and validation
- `src/detect/mod.rs` — Detection flow respects override
- `src/audit/` — Audit log records override

## Acceptance Criteria

- [x] `--manager pnpm` forces pnpm as the package manager (detection skipped)
- [x] `--manager uv` forces uv as the package manager (detection skipped)
- [x] `--use <name>` alias works identically to `--manager <name>`
- [x] Invalid manager name errors with a message listing all valid options
- [x] Override appears in the audit log with source `cli-override`
- [x] All unit tests pass
- [x] `just validate` passes

## Test Plan

- Unit: `cargo test manager_override` — override and validation tests
- Lint: `just lint`

## Observability

- Manager override should be logged at `info` level
- Invalid manager names should be logged at `warn` level

## Compliance

- PRD FR-1.3 (`--manager <name>` override)

## Risks & Mitigations

- Risk: Forcing a manager that is not installed may cause confusing downstream errors — Mitigation: Validate that the forced manager binary exists on PATH before proceeding; if not, error with a clear message

## Dependencies & Sequencing

- Depends on: 01-002 (CLI subcommands), 02-001 (detection engine)
- Unblocks: 04-001 (add engine)

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The detection engine does not support skipping auto-detection
- The valid manager value set does not match the PRD

## Maintenance Notes

- New package managers can be added by extending the valid value set
- Reviewers should check that the override is recorded in the audit log

## Commit Conventions

- `feat(cli): add --manager <name> override flag with validation and audit logging`
