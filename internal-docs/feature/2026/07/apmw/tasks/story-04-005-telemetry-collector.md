---
story_id: "04-005"
story_title: "Anonymized telemetry collector (categorical usage, opt-out, non-blocking)"
story_name: "telemetry-collector"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 4
parallel_id: 5
branch: "feature/current/apmw/story-04-005-telemetry-collector"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001"]
parallel_safe: true
modules: ["src/telemetry/"]
priority: "SHOULD"
risk_level: "low"
tags: ["feat", "telemetry", "privacy", "opt-out"]
due: "2026-09-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Implement anonymized usage telemetry. Collect ONLY: terminal type (interactive/TUI/login/non-interactive), command name (add/detect/scan/clone/status/list — no args), manager name (pnpm/uv/cargo etc — no package names), success/failure + error category, duration in ms, apmw version. NO personal identifiers, NO package names, NO file paths, NO URLs. Opt-out via `--no-telemetry`, config `telemetry = false`, env `APMW_NO_TELEMETRY=1`. Non-blocking (silently drop if endpoint unreachable). `--telemetry-preview` prints payload without sending. Configurable endpoint URL.

## Current State

- **Relevant files and their roles:**
  - PRD FR-6 — Anonymized telemetry
  - `src/config/` — Config module (from story 01-001) for telemetry settings
  - `src/error.rs` — Error types (from story 01-001)
- **Repository conventions:** Module by feature/domain. Use tokio for non-blocking HTTP. Use serde for payload serialization.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/telemetry/mod.rs` — Telemetry collector
- Create `src/telemetry/event.rs` — Event payload definition and serialization
- Create `src/telemetry/sender.rs` — Non-blocking HTTP sender
- Define the telemetry payload schema (documented):
  - `terminal_type`: interactive | tui | login | non-interactive
  - `command`: add | detect | scan | clone | status | list (no arguments)
  - `manager`: pnpm | uv | cargo | npm | docker | etc. (no package names)
  - `outcome`: success | failure
  - `error_category`: network | permission | not-found | version | other (only on failure)
  - `duration_ms`: integer (command duration in milliseconds)
  - `apmw_version`: semver string
- Collect ONLY the fields above — NO personal identifiers, NO package names, NO file paths, NO URLs
- Implement opt-out mechanisms:
  - `--no-telemetry` CLI flag (per-invocation)
  - `telemetry = false` in config TOML (persistent)
  - `APMW_NO_TELEMETRY=1` environment variable
- Implement non-blocking sender: if the endpoint is unreachable, silently drop the event (never block operation)
- Implement `--telemetry-preview` flag: print the payload that would be sent without actually sending it
- Make the endpoint URL configurable via config (`telemetry_endpoint = "https://..."`)
- Add unit tests for payload construction, opt-out logic, and preview mode
- Add integration tests for non-blocking send behavior

**Out of scope:**
- Config module (story 01-001)
- Analytics dashboard or aggregation backend
- Telemetry from the daemon process itself (future)

## Sub-Tasks

- [ ] Create `src/telemetry/mod.rs` with TelemetryCollector
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/telemetry/event.rs` with payload schema and serialization
  **Verify**: `cargo test telemetry_event` → tests pass
- [ ] Implement payload construction (terminal type, command, manager, outcome, duration, version)
  **Verify**: `cargo test telemetry_payload` → tests pass
- [ ] Verify no personal identifiers, package names, file paths, or URLs are collected
  **Verify**: `cargo test telemetry_privacy` → tests pass
- [ ] Create `src/telemetry/sender.rs` with non-blocking HTTP sender
  **Verify**: `cargo test telemetry_sender` → tests pass
- [ ] Implement non-blocking send (silently drop if endpoint unreachable)
  **Verify**: `cargo test telemetry_nonblocking` → tests pass
- [ ] Implement `--no-telemetry` CLI flag opt-out
  **Verify**: `cargo test telemetry_no_flag` → tests pass
- [ ] Implement `telemetry = false` config opt-out
  **Verify**: `cargo test telemetry_config_optout` → tests pass
- [ ] Implement `APMW_NO_TELEMETRY=1` env var opt-out
  **Verify**: `cargo test telemetry_env_optout` → tests pass
- [ ] Implement `--telemetry-preview` (print payload without sending)
  **Verify**: `cargo test telemetry_preview` → tests pass
- [ ] Implement configurable endpoint URL
  **Verify**: `cargo test telemetry_endpoint` → tests pass
- [ ] Document the payload schema
  **Verify**: `cargo doc` → no warnings
- [ ] Add integration tests for non-blocking send behavior
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/telemetry/mod.rs` — Telemetry collector
- `src/telemetry/event.rs` — Event payload definition
- `src/telemetry/sender.rs` — Non-blocking HTTP sender
- `src/lib.rs` — Add `pub mod telemetry;`
- `src/config/` — Telemetry config settings
- `src/cli.rs` — `--no-telemetry` and `--telemetry-preview` flags

## Acceptance Criteria

- [ ] Telemetry event is captured on each command (add, detect, scan, clone, status, list)
- [ ] `--telemetry-preview` shows the payload with no personal data
- [ ] `--no-telemetry` disables collection for that invocation
- [ ] `telemetry = false` in config disables collection persistently
- [ ] `APMW_NO_TELEMETRY=1` env var disables collection
- [ ] Endpoint unreachable does not block operation (non-blocking)
- [ ] Payload contains only: terminal type, command, manager, outcome, error category, duration, version
- [ ] No personal identifiers, package names, file paths, or URLs are collected
- [ ] Endpoint URL is configurable via config
- [ ] Payload schema is documented
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test telemetry` — all telemetry tests
- Integration: Non-blocking send with mock endpoint
- Lint: `just lint`

## Observability

- Telemetry send attempts should be logged at `debug` level
- Telemetry opt-out should be logged at `debug` level
- Telemetry send failures should be logged at `debug` level (never `error` — non-blocking)

## Compliance

- PRD FR-6 (anonymized telemetry with opt-out)
- Privacy: no personal identifiers, no package names, no file paths, no URLs

## Risks & Mitigations

- Risk: Users may be concerned about privacy — Mitigation: Document the exact payload schema; provide multiple opt-out mechanisms; `--telemetry-preview` for transparency
- Risk: Telemetry send may block the main operation — Mitigation: Non-blocking sender with fire-and-forget; silently drop on failure

## Dependencies & Sequencing

- Depends on: 01-001 (error types and config)
- Unblocks: 07-002 (documentation — payload schema docs)

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The non-blocking sender cannot be implemented without blocking the main operation
- The payload schema does not match the PRD requirements

## Maintenance Notes

- The payload schema should be kept minimal and documented
- Reviewers should verify that no personal data is collected
- The endpoint URL may change; keep it configurable

## Commit Conventions

- `feat(telemetry): add anonymized telemetry collector with opt-out and non-blocking send`
