---
story_id: "02-003"
story_title: "Ecosystem mapping engine (within-ecosystem only)"
story_name: "ecosystem-mapping"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 2
parallel_id: 3
branch: "feature/current/apmw/story-02-003-ecosystem-mapping"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001"]
parallel_safe: true
modules: ["src/ecosystem/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "ecosystem", "mapping"]
due: "2026-08-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the ecosystem mapping engine that maps non-canonical package managers to their canonical alternatives **within the same ecosystem only**: pip to uv within Python, npm/yarn/bun/yarn2 to pnpm within Node. Mapping is NEVER cross-ecosystem (e.g., uvx→pnpm dlx is WRONG — uvx is a Python-ecosystem tool and must not map to a Node-ecosystem tool). The mapping table is defined in the 2ndbrain research Table 2 (apmw command to per-package-manager command mapping with 200+ references).

**Key principle:** Ecosystem mapping is WITHIN-ecosystem only. pip→uv (Python), npm→pnpm (Node). Never map across ecosystems.

## Current State

- **Relevant files and their roles:**
  - PRD FR-2 (sections FR-2.1 through FR-2.3) — Ecosystem mapping requirements
  - 2ndbrain research Table 2 at `/Users/micro/Documents/2ndbrain/2ndbrain/Work/01 OandO/Self Improvement/Want To Build/Digital/All Package Manager Wrapper/All Package Manager Wrapper Tool Landscape.md`
- **Repository conventions:** Module by feature/domain. Use serde for data structures.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/ecosystem/mod.rs` — Ecosystem mapping engine
- Create `src/ecosystem/mapping.rs` — Mapping table (from 2ndbrain Table 2): pip->uv (Python), npm/yarn/bun/yarn2->pnpm (Node), and the canonical ad-hoc runner per ecosystem. **Mappings are within-ecosystem only — never cross-ecosystem.**
- Implement `EcosystemMap` struct: source_manager, canonical_manager, command_mapping (HashMap of apmw_command -> manager_command)
- Implement `map_command(apmw_command, source_manager) -> canonical_command` function
- Implement `suggest_canonical(source_manager) -> canonical_manager` function
- Support all package managers from 2ndbrain Table 2
- Add unit tests for all mapping entries
- Add property-based tests with proptest for mapping consistency

**Out of scope:**
- Alternative suggestion UI (story 04-002)
- Install engine (story 04-001)
- Ad-hoc runner execution (story 04-001)

## Sub-Tasks

- [x] Create `src/ecosystem/mod.rs` with EcosystemMapper struct
  **Verify**: `cargo check` → exit 0
- [x] Create `src/ecosystem/mapping.rs` with mapping table from 2ndbrain Table 2
  **Verify**: `cargo test mapping` → tests pass
- [x] Implement map_command function (apmw command + source manager -> canonical command)
  **Verify**: `cargo test map_command` → tests pass
- [x] Implement suggest_canonical function (source manager -> canonical manager)
  **Verify**: `cargo test suggest_canonical` → tests pass
- [x] Add unit tests for all mapping entries (pip->uv within Python, npm->pnpm, yarn->pnpm, bun->pnpm, yarn2->pnpm within Node — NO cross-ecosystem mappings)
  **Verify**: `just test` → all pass
- [x] Add property-based tests with proptest for mapping consistency
  **Verify**: `cargo test proptest` → tests pass
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/ecosystem/mod.rs` — Ecosystem mapping engine
- `src/ecosystem/mapping.rs` — Mapping table
- `src/lib.rs` — Add `pub mod ecosystem;`

## Acceptance Criteria

- [x] pip maps to uv (within Python ecosystem)
- [x] npm/yarn/bun/yarn2 map to pnpm (within Node ecosystem)
- [x] NO cross-ecosystem mappings exist (e.g., uvx does NOT map to pnpm dlx)
- [x] map_command returns the correct canonical command for each apmw command (within-ecosystem only)
- [x] suggest_canonical returns the correct canonical manager
- [x] All mapping entries from 2ndbrain Table 2 are implemented
- [x] All unit and property-based tests pass
- [x] `just validate` passes

## Test Plan

- Unit: `cargo test ecosystem` — all mapping tests
- Property: `cargo test proptest` — mapping consistency
- Lint: `just lint`

## Observability

- Mapping results should be logged at `debug` level
- Unmapped managers should be logged at `warn` level

## Compliance

- PRD FR-2 (ecosystem mapping requirements)
- 2ndbrain research Table 2 (command mapping)

## Risks & Mitigations

- Risk: 2ndbrain Table 2 may not cover all required package managers — Mitigation: Start with the core within-ecosystem mappings (pip->uv, npm->pnpm), extend as needed
- Risk: Cross-ecosystem mappings may be inadvertently introduced — Mitigation: Add a test that asserts no cross-ecosystem mappings exist; validate ecosystem boundaries in the mapping table

## Dependencies & Sequencing

- Depends on: 01-001 (error types)
- Unblocks: 04-001, 04-002

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The 2ndbrain research Table 2 is not accessible at the documented path
- The mapping table does not cover the required ecosystems

## Maintenance Notes

- New ecosystem mappings can be added by extending mapping.rs
- Reviewers should check that all canonical mappings match the 2ndbrain research

## Commit Conventions

- `feat(ecosystem): add ecosystem mapping engine with within-ecosystem pip->uv, npm->pnpm mappings`
