---
story_id: "01-003"
story_title: "AXI output module (TOON encoder, minimal schemas, truncation)"
story_name: "axi-output-module"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 1
parallel_id: 3
branch: "feature/current/apmw/story-01-003-axi-output-module"
status: "done"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["src/output/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "foundation", "axi", "toon", "output"]
due: "2026-08-15"
created_at: "2026-07-29"
updated_at: "2026-07-29T12:00:00Z"
---

## Summary

Create the AXI output module implementing TOON (Token-Oriented Object Notation) encoding, minimal default schemas (3-4 fields), content truncation (500-1500 chars with total size and --full escape hatch), pre-computed aggregates (total count in list output), definitive empty states, structured errors on stdout, and contextual disclosure (2-4 next steps in help[] array). This module is the output boundary — internal logic stays on JSON, conversion to TOON happens here.

## Current State

- **Relevant files and their roles:**
  - `Cargo.toml` — Has `serde` and `serde_json` dependencies
  - `src/lib.rs` — Library root
- **Repository conventions:** Module organization by feature/domain. Use pub use in lib.rs. See AGENTS.md lines 109-120 for AXI agent mode contract.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/output/mod.rs` — Output dispatcher (agent mode vs human mode)
- Create `src/output/toon.rs` — TOON encoder (convert serde_json::Value to TOON string)
- Create `src/output/human.rs` — Human-readable output (colors, formatting, pager)
- Implement minimal default schemas: 3-4 fields in list output, --fields flag for additional fields (ADR section 37)
- Implement content truncation: 500-1500 char preview, total size display, --full escape hatch (ADR section 38)
- Implement pre-computed aggregates: total count in list output, derived status fields (ADR section 39)
- Implement definitive empty states: explicit "0 results found" with context (ADR section 40)
- Implement structured errors on stdout: error + suggestion, no raw stack traces (ADR section 41)
- Implement contextual disclosure: 2-4 next steps in help[] array (ADR section 45)
- Implement content-first no-args: live content output, not usage manual (ADR section 44)
- Add unit tests for TOON encoding, truncation, empty states, error formatting
- Add property-based tests with proptest for TOON encoder

**Out of scope:**
- CLI argument parsing (story 01-002)
- Actual feature data (detection results, install results — later stories)
- Human mode TUI (ratatui/cursive integration — future story)

## Sub-Tasks

- [x] Create `src/output/mod.rs` with OutputDispatcher trait (agent mode vs human mode)
  **Verify**: `cargo check` → exit 0
- [x] Create `src/output/toon.rs` with TOON encoder (serde_json::Value -> TOON string)
  **Verify**: `cargo test toon` → encoder tests pass
- [x] Implement minimal schema filtering (select 3-4 fields, --fields flag support)
  **Verify**: `cargo test schema` → tests pass
- [x] Implement content truncation (preview + total size + --full hint)
  **Verify**: `cargo test truncation` → tests pass
- [x] Implement pre-computed aggregates (total count in list output)
  **Verify**: `cargo test aggregates` → tests pass
- [x] Implement definitive empty states
  **Verify**: `cargo test empty_states` → tests pass
- [x] Implement structured errors on stdout (error + suggestion format)
  **Verify**: `cargo test structured_errors` → tests pass
- [x] Implement contextual disclosure (help[] array with 2-4 next steps)
  **Verify**: `cargo test contextual_disclosure` → tests pass
- [x] Create `src/output/human.rs` with human-readable output (colors, formatting)
  **Verify**: `cargo test human_output` → tests pass
- [x] Add property-based tests with proptest for TOON encoder
  **Verify**: `cargo test proptest` → tests pass
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/output/mod.rs` — Output dispatcher
- `src/output/toon.rs` — TOON encoder
- `src/output/human.rs` — Human-readable output
- `src/lib.rs` — Add `pub mod output;`

## Acceptance Criteria

- [x] TOON encoder produces valid TOON from serde_json::Value
- [x] Minimal schemas default to 3-4 fields
- [x] Content truncation shows preview, total size, and --full hint
- [x] Pre-computed aggregates include total count
- [x] Empty states are definitive with context
- [x] Structured errors include description and suggestion
- [x] Contextual disclosure includes 2-4 next steps as complete commands
- [x] Agent mode is the default; human mode is opt-in
- [x] All unit and property-based tests pass
- [x] `just validate` passes

## Test Plan

- Unit: `cargo test output` — all output module tests
- Property: `cargo test proptest` — TOON encoder property tests
- Lint: `just lint`

## Observability

- Output mode selection (agent vs human) should be logged at `debug` level
- Truncation events should be logged at `debug` level

## Compliance

- ADR-20260607001 sections 36-45 (AXI agent mode)
- TOON format specification (https://toonformat.dev/reference/spec.html)

## Risks & Mitigations

- Risk: TOON format specification may have edge cases not covered — Mitigation: Start with a subset covering common patterns (lists, objects, scalars), expand as needed
- Risk: Truncation may break structured output — Mitigation: Truncate field values, not structure

## Dependencies & Sequencing

- Depends on: None (foundation story)
- Unblocks: 02-001, 04-001, 06-001

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- TOON format specification is not accessible or is ambiguous
- serde_json::Value cannot be converted to TOON without losing type information

## Maintenance Notes

- TOON encoder should be extended as new output patterns are needed
- Reviewers should verify TOON output against the specification examples
- Human mode formatting should be extended with ratatui/cursive in a future story

## Commit Conventions

- `feat(output): add AXI output module with TOON encoder and truncation`
