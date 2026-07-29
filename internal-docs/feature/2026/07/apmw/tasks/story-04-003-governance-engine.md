---
story_id: "04-003"
story_title: "Governance engine (reads prefer/force/block/eject from levonk-packages spec)"
story_name: "governance-engine"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 4
parallel_id: 3
branch: "feature/current/apmw/story-04-003-governance-engine"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001", "02-003"]
parallel_safe: true
modules: ["src/governance/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "governance", "policy", "levonk-packages"]
due: "2026-09-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Implement a governance engine that reads governance rules from the levonk-packages governance spec (https://github.com/levonk/levonk-packages, docs/SPEC.md). Support the four governance types: prefer (soft guidance, warn + delegate), eject (remove + block), force (strict replacement via wrapper), block (error-exiting wrapper). Config allows per-tool governance settings in TOML. Cache the spec locally with a 7-day TTL. Support `apmw governance refresh` to force-refresh. Support `devbox-rtk-*` naming convention.

## Current State

- **Relevant files and their roles:**
  - PRD FR-8 — Governance rules (prefer/force/block/eject)
  - `src/config/` — Config module (from story 01-001) for per-tool governance settings
  - `src/ecosystem/` — Ecosystem mapping (from story 02-003) for canonical manager lookup
  - levonk-packages spec at https://github.com/levonk/levonk-packages (docs/SPEC.md)
- **Repository conventions:** Module by feature/domain. Use serde for data structures. Use tokio for async HTTP.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/governance/mod.rs` — Governance engine
- Create `src/governance/spec.rs` — Spec loader (fetch from levonk-packages, parse, cache)
- Create `src/governance/rules.rs` — Rule definitions (prefer, force, block, eject)
- Create `src/governance/wrapper.rs` — Wrapper/shim for force and block types
- Implement the four governance types:
  - **prefer**: soft guidance — warn the user and delegate to the canonical manager if installed
  - **force**: strict replacement — intercept calls to the non-canonical manager and route through the canonical one via a wrapper
  - **block**: hard block — error-exiting wrapper that prevents use of the non-canonical manager entirely
  - **eject**: remove and block — remove the non-canonical manager from the project and block future use
- Support per-tool governance settings in TOML config (e.g., `[governance.pip] type = "force"`)
- Cache the spec locally with a 7-day TTL; re-fetch if cache is stale
- Support `apmw governance refresh` subcommand to force-refresh the cached spec
- Support `devbox-rtk-*` naming convention for governance-managed wrapper tools
- Add unit tests for each governance type
- Add integration tests for spec loading and caching

**Out of scope:**
- Ecosystem mapping table (story 02-003)
- Add engine (story 04-001)
- Agent hooks installation (story 06-002)

## Sub-Tasks

- [ ] Create `src/governance/mod.rs` with GovernanceEngine
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/governance/spec.rs` with spec loader (fetch, parse, cache with 7-day TTL)
  **Verify**: `cargo test spec_loader` → tests pass
- [ ] Create `src/governance/rules.rs` with rule definitions (prefer, force, block, eject)
  **Verify**: `cargo test governance_rules` → tests pass
- [ ] Implement prefer type (warn + delegate to canonical manager)
  **Verify**: `cargo test governance_prefer` → tests pass
- [ ] Implement force type (intercept and route through canonical via wrapper)
  **Verify**: `cargo test governance_force` → tests pass
- [ ] Implement block type (error-exiting wrapper)
  **Verify**: `cargo test governance_block` → tests pass
- [ ] Implement eject type (remove + block)
  **Verify**: `cargo test governance_eject` → tests pass
- [ ] Create `src/governance/wrapper.rs` with wrapper/shim for force and block
  **Verify**: `cargo test governance_wrapper` → tests pass
- [ ] Implement per-tool governance config parsing from TOML
  **Verify**: `cargo test governance_config` → tests pass
- [ ] Implement local cache with 7-day TTL
  **Verify**: `cargo test governance_cache_ttl` → tests pass
- [ ] Implement `apmw governance refresh` subcommand (force-refresh spec)
  **Verify**: `cargo test governance_refresh` → tests pass
- [ ] Support `devbox-rtk-*` naming convention
  **Verify**: `cargo test devbox_rtk_naming` → tests pass
- [ ] Add integration tests for spec loading and caching
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/governance/mod.rs` — Governance engine
- `src/governance/spec.rs` — Spec loader (fetch, parse, cache)
- `src/governance/rules.rs` — Rule definitions
- `src/governance/wrapper.rs` — Wrapper/shim for force and block
- `src/lib.rs` — Add `pub mod governance;`
- `src/config/` — Per-tool governance settings in TOML

## Acceptance Criteria

- [ ] Governance rules are loaded from the levonk-packages spec
- [ ] `force-pnpm` intercepts npm calls and routes through pnpm
- [ ] `block-pip` errors when pip is invoked
- [ ] `prefer-uv` warns and delegates to uv if installed
- [ ] `eject` removes the non-canonical manager and blocks future use
- [ ] Config TOML is parsed correctly for per-tool governance settings
- [ ] Local cache works with 7-day TTL (stale cache triggers re-fetch)
- [ ] `apmw governance refresh` force-refreshes the cached spec
- [ ] `devbox-rtk-*` naming convention is supported
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test governance` — all governance engine tests
- Integration: Spec loading and caching with mock HTTP responses
- Lint: `just lint`

## Observability

- Governance rule application should be logged at `info` level
- Spec fetch failures should be logged at `error` level
- Cache hits/misses should be logged at `debug` level
- Blocked manager invocations should be logged at `warn` level

## Compliance

- PRD FR-8 (governance rules: prefer/force/block/eject)
- levonk-packages governance spec (docs/SPEC.md)

## Risks & Mitigations

- Risk: levonk-packages spec may be unavailable or change format — Mitigation: Cache locally with TTL; if fetch fails, use cached version and warn; validate spec schema on parse
- Risk: Force/block wrappers may interfere with user workflows — Mitigation: Governance is opt-in via config; document the behavior clearly

## Dependencies & Sequencing

- Depends on: 01-001 (error types and config), 02-003 (ecosystem mapping)
- Unblocks: 06-002 (agent hooks — governance-aware)

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The levonk-packages spec is not accessible or its schema is incompatible
- The governance spec does not define the four required types (prefer/force/block/eject)

## Maintenance Notes

- Governance rules should be updated as the levonk-packages spec evolves
- Reviewers should check that force/block wrappers do not break existing workflows
- The cache TTL may need tuning based on spec update frequency

## Commit Conventions

- `feat(governance): add governance engine with prefer/force/block/eject from levonk-packages spec`
