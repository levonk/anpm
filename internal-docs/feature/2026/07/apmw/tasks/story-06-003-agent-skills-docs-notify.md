---
story_id: "06-003"
story_title: "Installable Agent Skills + session integrations + docs notification"
story_name: "agent-skills-docs-notify"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 6
parallel_id: 3
branch: "feature/current/apmw/story-06-003-agent-skills-docs-notify"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-002", "01-003", "04-001"]
parallel_safe: true
modules: ["src/agent/skills.rs", "src/agent/docs_notify.rs"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "skills", "session", "docs", "agent", "ai"]
due: "2026-10-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the installable Agent Skills (SKILL.md generation from the no-args home view), session integrations (Claude Code, Codex, OpenCode hooks), and AI agent docs notification (notify the agent where to find docs for an added package). Follow ADR-20260607001 sections 42-43 for session integrations and installable skills.

## Current State

- **Relevant files and their roles:**
  - PRD FR-9.10 — Session integrations (Claude Code, Codex, OpenCode)
  - PRD FR-9.11 — Installable Agent Skill
  - PRD FR-11 — AI agent docs notification
  - ADR-20260607001 sections 42-43 — Session integrations and installable skills
- **Existing code excerpts (from ADR-20260607001):**
  ```
  Section 42 — Session integrations:
  - Claude Code: ~/.claude/settings.json SessionStart hook
  - Codex: ~/.codex/hooks.json SessionStart hook
  - OpenCode: ~/.config/opencode/plugins/ managed plugin
  - Explicit opt-in from user-invoked setup command
  - Idempotent, directory-scoped, token-budget-aware

  Section 43 — Installable Agent Skill:
  - Generate SKILL.md from same content as no-args home view
  - Add --check build step to CI
  - Strip live state, non-interactive commands
  - Trigger-shaped frontmatter
  ```
- **Repository conventions:** Module by feature/domain.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/agent/skills.rs` — Installable Agent Skills generation
- Create `src/agent/docs_notify.rs` — AI agent docs notification
- Implement SKILL.md generation from the no-args home view content (ADR section 43)
- Add `--check` build step to CI that fails if committed SKILL.md is stale
- Implement session integrations:
  - Claude Code: SessionStart hook in `~/.claude/settings.json`
  - Codex: SessionStart hook in `~/.codex/hooks.json`
  - OpenCode: managed plugin in `~/.config/opencode/plugins/`
- Session integrations are explicit opt-in from `apmw --install` command
- Session integrations are idempotent (repeated installs are silent no-ops)
- Session integrations are directory-scoped (show only state for current directory)
- Session integrations are token-budget-aware (minimize per-session context)
- Implement docs notification: after adding a package, notify the AI agent where to find docs
- Docs notification includes: package name, docs URL (from package metadata), local path to README/docs
- Add unit tests for SKILL.md generation, session integration, docs notification

**Out of scope:**
- MCP server (story 06-001)
- AI agent coding hooks (story 06-002)

## Sub-Tasks

- [x] Create `src/agent/skills.rs` with SkillGenerator
  **Verify**: `cargo check` → exit 0
- [x] Implement SKILL.md generation from no-args home view content
  **Verify**: `cargo test skill_generation` → tests pass
- [x] Implement --check build step for CI (fail if SKILL.md is stale)
  **Verify**: `cargo test skill_check` → tests pass
- [x] Implement Claude Code session integration (SessionStart hook)
  **Verify**: `cargo test claude_integration` → tests pass
- [x] Implement Codex session integration (SessionStart hook)
  **Verify**: `cargo test codex_integration` → tests pass
- [x] Implement OpenCode session integration (managed plugin)
  **Verify**: `cargo test opencode_integration` → tests pass
- [x] Implement idempotent install (repeated installs are no-ops)
  **Verify**: `cargo test idempotent` → tests pass
- [x] Implement directory-scoped context (show only current directory state)
  **Verify**: `cargo test directory_scoped` → tests pass
- [x] Create `src/agent/docs_notify.rs` with DocsNotifier
  **Verify**: `cargo check` → exit 0
- [x] Implement docs notification (package name, docs URL, local README path)
  **Verify**: `cargo test docs_notify` → tests pass
- [x] Wire docs notification into add flow
  **Verify**: `cargo test docs_integration` → tests pass
- [x] Add unit tests for all components
  **Verify**: `just test` → all pass
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/agent/skills.rs` — Agent Skills generation (SkillGenerator, session integrations)
- `src/agent/docs_notify.rs` — Docs notification (DocsNotifier, DocsNotification)
- `src/agent/mod.rs` — Agent module root (updated with submodule declarations and exports)
- `src/lib.rs` — Library root (updated with pub use exports for new types)
- `src/install/on_use.rs` — Install-on-use engine (updated with docs notification step 9)

## Acceptance Criteria

- [x] SKILL.md is generated from the no-args home view content
- [x] --check build step fails if committed SKILL.md is stale
- [x] Claude Code session integration installs SessionStart hook
- [x] Codex session integration installs SessionStart hook
- [x] OpenCode session integration installs managed plugin
- [x] Session integrations are idempotent
- [x] Session integrations are directory-scoped
- [x] Docs notification includes package name, docs URL, local README path
- [x] Docs notification fires after add
- [x] All unit tests pass
- [x] `just validate` passes

## Test Plan

- Unit: `cargo test agent` — all agent module tests
- Lint: `just lint`

## Observability

- Session integration installs should be logged at `info` level
- SKILL.md generation should be logged at `info` level
- Docs notifications should be logged at `debug` level

## Compliance

- ADR-20260607001 section 42 (session integrations)
- ADR-20260607001 section 43 (installable agent skill)
- PRD FR-9.10, FR-9.11, FR-11

## Risks & Mitigations

- Risk: Agent session hook formats may change — Mitigation: Abstract each integration behind a trait, update as needed
- Risk: SKILL.md may drift from no-args output — Mitigation: --check build step in CI catches drift

## Dependencies & Sequencing

- Depends on: 01-002 (CLI), 01-003 (AXI output), 04-001 (add engine)
- Unblocks: 07-001

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- Agent session hook formats are not as documented in the ADR
- SKILL.md generation cannot be automated from the no-args output

## Maintenance Notes

- Session integrations should be updated as agent platforms evolve
- Reviewers should check that session integrations are idempotent and token-budget-aware
- SKILL.md should be kept in sync with the no-args home view

## Commit Conventions

- `feat(agent): add installable skills, session integrations, and docs notification`
