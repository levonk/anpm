---
story_id: "06-002"
story_title: "AI agent coding hooks (hard intercept + soft convention, governance-aware)"
story_name: "agent-hooks"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 6
parallel_id: 2
branch: "feature/current/apmw/story-06-002-agent-hooks"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001", "03-002", "04-003"]
parallel_safe: true
modules: ["src/agent/hooks.rs"]
priority: "MUST"
risk_level: "high"
tags: ["feat", "hooks", "intercept", "agent", "ai"]
due: "2026-10-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the AI agent coding hooks that force AI agents through apmw's install-on-use hook for adding dependencies and running tools. Implement both a hard intercept (PATH shim that wraps package manager binaries like pip, npm, cargo) and a soft convention (instructions for agents to use `apmw add`). The hard intercept is opt-in via `apmw --install --intercept`; the soft convention is the default. Hooks are governance-aware: they respect governance rules (prefer/force/block/eject) from the governance engine (story 04-003), so intercepted calls are routed or blocked according to the active governance policy.

## Current State

- **Relevant files and their roles:**
  - PRD FR-10.2 — AI agent coding hooks (hard intercept + soft convention)
  - PRD FR-8 — Governance rules (prefer/force/block/eject) — hooks must respect governance
  - PRD Open Question 3 — Hard intercept vs soft convention
  - `src/security/` — Security scanning (from story 03-002) must run on all intercepted adds
  - `src/governance/` — Governance engine (from story 04-003) provides prefer/force/block/eject rules
- **Repository conventions:** Module by feature/domain.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/agent/hooks.rs` — AI agent coding hooks module
- Implement hard intercept: PATH shim that wraps package manager binaries (pip, npm, yarn, cargo, go, apt, brew) so any invocation routes through apmw
- The PATH shim intercepts the call, checks governance rules (prefer/force/block/eject from 04-003), runs security scanning, then delegates to the real binary (or the canonical alternative if governance forces it)
- Hard intercept is opt-in via `apmw --install --intercept` (installs shims to a PATH dir that takes precedence)
- Implement `apmw --uninstall --intercept` to remove shims
- Implement soft convention: generate instructions for AI agents to use `apmw add` instead of calling package managers directly
- Soft convention is the default (no intercept installed)
- Hooks are governance-aware: intercepted calls respect the active governance policy:
  - **prefer**: warn and delegate to canonical manager (if installed)
  - **force**: intercept and route through the canonical manager via wrapper
  - **block**: error and prevent the call entirely
  - **eject**: remove and block the non-canonical manager
- Add unit tests for shim generation and interception logic
- Add integration tests for intercepted installs

**Out of scope:**
- MCP server (story 06-001)
- Installable Agent Skills (story 06-003)

## Sub-Tasks

- [ ] Create `src/agent/hooks.rs` with HookManager
  **Verify**: `cargo check` → exit 0
- [ ] Implement hard intercept: PATH shim generation for pip, npm, yarn, cargo, go, apt, brew
  **Verify**: `cargo test shim_generation` → tests pass
- [ ] Implement shim interception logic (intercept -> governance check -> security scan -> delegate)
  **Verify**: `cargo test interception` → tests pass
- [ ] Implement governance-aware interception (respect prefer/force/block/eject from 04-003)
  **Verify**: `cargo test governance_hooks` → tests pass
- [ ] Implement `apmw --install --intercept` (install shims to PATH)
  **Verify**: `apmw --install --intercept` → shims installed
- [ ] Implement `apmw --uninstall --intercept` (remove shims)
  **Verify**: `apmw --uninstall --intercept` → shims removed
- [ ] Implement soft convention: generate agent instructions
  **Verify**: `cargo test soft_convention` → tests pass
- [ ] Add unit tests for shim generation and interception
  **Verify**: `just test` → all pass
- [ ] Add integration tests for intercepted adds
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/agent/hooks.rs` — Hook manager
- `src/agent/mod.rs` — Agent module root

## Acceptance Criteria

- [ ] Hard intercept generates PATH shims for pip, npm, yarn, cargo, go, apt, brew
- [ ] Shims intercept calls, check governance rules, run security scanning, then delegate to the real binary (or canonical alternative if governance forces)
- [ ] Governance-aware hooks respect prefer/force/block/eject rules from the governance engine (04-003)
- [ ] `apmw --install --intercept` installs shims
- [ ] `apmw --uninstall --intercept` removes shims
- [ ] Soft convention generates agent instructions
- [ ] Soft convention is the default (no intercept installed)
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test hooks` — hook and shim tests
- Integration: Intercepted add tests
- Lint: `just lint`

## Observability

- Shim installations and removals should be logged at `info` level
- Intercepted calls should be logged at `info` level
- Security scan results from intercepted calls should be logged at `warn` level for risky packages

## Compliance

- PRD FR-10.2 (AI agent coding hooks)
- PRD FR-8 (governance rules — hooks are governance-aware)
- PRD Open Question 3 (hard intercept vs soft convention — implement both)

## Risks & Mitigations

- Risk: PATH shims may break existing workflows — Mitigation: Hard intercept is opt-in only; soft convention is default
- Risk: Shims may not work on Windows — Mitigation: Use batch file shims on Windows, shell scripts on Unix

## Dependencies & Sequencing

- Depends on: 01-001 (error types), 03-002 (security scanning), 04-003 (governance engine)
- Unblocks: 07-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- PATH shims cannot intercept package manager calls reliably
- Security scanning cannot be run from within a shim

## Maintenance Notes

- New package manager shims can be added by extending the shim list
- Reviewers should verify that shims are opt-in only and do not break existing workflows
- Shims should be tested on Linux, macOS, and Windows
- Reviewers should verify that governance rules (prefer/force/block/eject) are respected by intercepted calls

## Commit Conventions

- `feat(hooks): add governance-aware AI agent coding hooks with hard intercept and soft convention`
