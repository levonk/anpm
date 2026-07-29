---
story_id: "04-002"
story_title: "Alternative suggestions engine"
story_name: "alternative-suggestions"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 4
parallel_id: 2
branch: "feature/current/apmw/story-04-002-alternative-suggestions"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["02-003"]
parallel_safe: true
modules: ["src/install/suggest.rs"]
priority: "SHOULD"
risk_level: "low"
tags: ["feat", "suggest", "alternatives"]
due: "2026-09-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the alternative suggestions engine that suggests better alternatives for the current use case. When the user requests an install via a non-canonical manager (e.g., pip), apmw suggests the canonical alternative (e.g., uv) and offers to use it. Suggestions are based on the ecosystem mapping table.

## Current State

- **Relevant files and their roles:**
  - `src/ecosystem/` — Ecosystem mapping engine (from story 02-003)
  - PRD FR-2.2 — Suggest canonical alternative
  - PRD Goal 5 — Suggest better alternatives
- **Repository conventions:** Module by feature/domain.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/install/suggest.rs` — Alternative suggestions engine
- Implement suggest function: given a source manager, return the canonical alternative and a human-readable reason
- Implement offer-to-use: after suggesting, offer to use the canonical alternative (in human mode, prompt; in agent mode, include as a suggestion in help[])
- Wire suggestions into the install flow (when a non-canonical manager is detected, suggest before installing)
- Add unit tests for all suggestion pairs (pip->uv, npm->pnpm, yarn->pnpm, bun->pnpm, yarn2->pnpm, uvx->pnpm dlx)

**Out of scope:**
- Install engine (story 04-001)
- Ecosystem mapping table (story 02-003)

## Sub-Tasks

- [ ] Create `src/install/suggest.rs` with SuggestEngine
  **Verify**: `cargo check` → exit 0
- [ ] Implement suggest function (source manager -> canonical alternative + reason)
  **Verify**: `cargo test suggest` → tests pass
- [ ] Implement offer-to-use logic (prompt in human mode, help[] in agent mode)
  **Verify**: `cargo test offer` → tests pass
- [ ] Wire suggestions into install flow
  **Verify**: `cargo test suggest_integration` → tests pass
- [ ] Add unit tests for all suggestion pairs
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/install/suggest.rs` — Alternative suggestions engine
- `src/install/mod.rs` — Wire suggestions into install flow

## Acceptance Criteria

- [ ] Suggestions are produced for all non-canonical managers
- [ ] Suggestions include the canonical alternative and a reason
- [ ] In human mode, the user is prompted to use the canonical alternative
- [ ] In agent mode, the suggestion appears in the help[] array
- [ ] All unit tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test suggest` — all suggestion tests
- Lint: `just lint`

## Observability

- Suggestions should be logged at `info` level

## Compliance

- PRD FR-2.2 (suggest canonical alternative)
- PRD Goal 5 (suggest better alternatives)

## Risks & Mitigations

- Risk: Suggestions may be annoying if the user intentionally uses a non-canonical manager — Mitigation: Add a config option to disable suggestions

## Dependencies & Sequencing

- Depends on: 02-003 (ecosystem mapping)
- Unblocks: 06-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The ecosystem mapping table does not provide canonical alternatives

## Maintenance Notes

- Suggestion reasons should be updated as ecosystem best practices evolve
- Reviewers should check that suggestions are non-intrusive in agent mode

## Commit Conventions

- `feat(suggest): add alternative suggestions engine with canonical manager recommendations`
