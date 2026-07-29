---
story_id: "01-005"
story_title: "Audit log writer"
story_name: "audit-log-writer"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 1
parallel_id: 5
branch: "feature/current/apmw/story-01-005-audit-log-writer"
status: "done"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["src/audit/"]
priority: "MUST"
risk_level: "low"
tags: ["feat", "foundation", "audit", "logging"]
due: "2026-08-15"
created_at: "2026-07-29"
updated_at: "2026-07-30"
---

## Summary

Create the audit log writer that records every install/detect/clone/scan operation to `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/`. Each entry records: timestamp, what was requested, what was done, terminal type (login/interactive), caller program, and tools used. The log is append-only with configurable retention period (ADR section 34). Support an export command for external analysis.

## Current State

- **Relevant files and their roles:**
  - `Cargo.toml` — Has `serde` and `serde_json` for serialization
  - `src/lib.rs` — Library root
- **Repository conventions:** Use serde for serialization. Use XDG paths. See AGENTS.md.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/audit/mod.rs` — Audit log module
- Create `src/audit/log.rs` — Append-only log writer with retention
- Define `AuditLogEntry` struct (serde): timestamp, request, action, terminal_type (login/interactive), caller_program, tools_used
- Implement XDG path resolution: `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/audit.log`
- Implement append-only writing (entries flushed to disk before returning success)
- Implement configurable retention period (auto-prune old entries on startup per ADR section 34)
- Implement export command (JSONL format for external analysis)
- Detect terminal type (login shell, interactive shell, non-interactive)
- Detect caller program (from environment variables or parent process)
- Add unit tests for log writing, retention, export
- Add integration tests with tempfile for file operations

**Out of scope:**
- Audit log viewer UI (future story)
- Integration with specific operations (later stories will call the audit logger)

## Sub-Tasks

- [x] Create `src/audit/mod.rs` and `src/audit/log.rs`
  **Verify**: `cargo check` → exit 0
- [x] Define `AuditLogEntry` struct with serde derive
  **Verify**: `cargo test audit_entry` → tests pass
- [x] Implement XDG path resolution for audit log location
  **Verify**: `cargo test audit_path` → tests pass
- [x] Implement append-only writer with disk flush before success return
  **Verify**: `cargo test audit_write` → tests pass
- [x] Implement retention period with auto-prune on startup
  **Verify**: `cargo test audit_retention` → tests pass
- [x] Implement terminal type detection (login/interactive/non-interactive)
  **Verify**: `cargo test terminal_detection` → tests pass
- [x] Implement caller program detection
  **Verify**: `cargo test caller_detection` → tests pass
- [x] Implement export command (JSONL format)
  **Verify**: `cargo test audit_export` → tests pass
- [x] Add integration tests with tempfile
  **Verify**: `just test` → all pass
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/audit/mod.rs` — Audit log module
- `src/audit/log.rs` — Append-only log writer
- `src/lib.rs` — Add `pub mod audit;`

## Acceptance Criteria

- [x] Audit log entries are written to `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/audit.log`
- [x] Each entry records timestamp, request, action, terminal_type, caller_program, tools_used
- [x] Log is append-only and flushed to disk before returning success
- [x] Retention period auto-prunes old entries on startup
- [x] Export command produces JSONL output
- [x] Terminal type detection works (login/interactive/non-interactive)
- [x] Caller program detection works
- [x] All unit and integration tests pass
- [x] `just validate` passes

## Test Plan

- Unit: `cargo test audit` — all audit module tests
- Integration: tempfile-based file operations
- Lint: `just lint`

## Observability

- Audit log write failures should be logged at `error` level via tracing
- Retention pruning should be logged at `info` level

## Compliance

- ADR-20260607001 section 34 (audit logging with retention)
- ADR-20260607001 section 33 (privacy mode with anonymous lists)
- XDG Base Directory Specification

## Risks & Mitigations

- Risk: Concurrent writes from daemon and CLI may corrupt the log — Mitigation: Use file locking or append-only mode with O_APPEND

## Dependencies & Sequencing

- Depends on: None (foundation story)
- Unblocks: 02-001, 03-001, 04-001, 05-001

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- XDG path resolution fails on the target platform
- Append-only file operations are not supported

## Maintenance Notes

- Future stories will call the audit logger after each operation
- Reviewers should check that entries are flushed to disk before success
- Privacy mode (ADR section 33) should be integrated in a future update

## Commit Conventions

- `feat(audit): add append-only audit log writer with XDG paths and retention`
