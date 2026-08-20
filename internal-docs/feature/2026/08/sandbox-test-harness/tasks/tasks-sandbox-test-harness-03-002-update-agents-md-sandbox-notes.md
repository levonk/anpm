---
story_id: "03-002"
story_title: "Update AGENTS.md with sandbox notes"
story_name: "update-agents-md-sandbox-notes"
prd_name: "sandbox-test-harness"
prd_file: "internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md"
phase: 3
parallel_id: 2
branch: "feature/current/sandbox-test-harness/story-03-002-update-agents-md-sandbox-notes"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["02-001"]
parallel_safe: true
modules: ["AGENTS.md"]
priority: "SHOULD"
risk_level: "low"
tags: ["docs", "agents-md"]
due: "2026-08-07"
create-date: "2026-08-05"
update-date: "2026-08-05"
---

## Summary

Update the project's `AGENTS.md` to document the sandboxed test harness: how
it works, how to run tests with the sandbox, how to install nono, and the
fallback policy. This ensures future contributors and AI agents know the tests
are sandboxed and how to work with the sandbox.

## Sub-Tasks

- [ ] Add a "Sandboxed Testing" subsection to the Testing section of AGENTS.md
- [ ] Document:
  - [ ] Tests run inside nono sandbox by default
  - [ ] `brew install nono` or `curl -fsSL https://nono.sh/install.sh | sh` to install
  - [ ] If nono is absent, tests fall back to unsandboxed with a warning (local dev)
  - [ ] CI requires the sandbox (`APMW_TEST_SANDBOX_REQUIRED=1`)
  - [ ] The nono profile restricts fs writes to TempDir and network to real registries
  - [ ] New integration tests must use `sandboxed_command()` from `tests/sandbox/mod.rs`
  - [ ] Reference the ADR at `internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md`
- [ ] Update the Quality Gates section if needed (sandbox is now part of `just test`)

## Relevant Files

- `AGENTS.md` — the project's binding contract for AI agents

## Acceptance Criteria (Gherkin)

- Given `AGENTS.md`, When a contributor reads the Testing section, Then they
  find a "Sandboxed Testing" subsection explaining nono, the profile, and the
  fallback policy
- Given `AGENTS.md`, When a contributor reads about new tests, Then they see
  that new integration tests must use `sandboxed_command()`
- Given `AGENTS.md`, When a contributor looks for the ADR reference, Then they
  find a link to `internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md`

## Test Plan

- Read the updated `AGENTS.md` and verify all Sub-Task items are documented
- Verify no existing AGENTS.md content was removed (additive change only)

## Observability

- N/A (documentation only)

## Compliance

- AGENTS.md changes must be additive (do not remove existing sections)
- No AI attribution boilerplate
- Follow the existing AGENTS.md format (markdown tables, code blocks)

## Risks & Mitigations

- **Risk**: AGENTS.md may become too long
- **Mitigation**: Keep the sandbox section concise (~20 lines). Link to the
  ADR for details.

## Dependencies & Sequencing

- Depends on: 02-001 (migration must be complete so the documentation is accurate)
- Unblocks: nothing (this is a leaf story)

## Definition of Done

- `AGENTS.md` has a "Sandboxed Testing" subsection
- All Sub-Task items are documented
- No existing content removed
- ADR is referenced

## Commit Conventions

```
docs(agents): add sandboxed testing section to AGENTS.md

Document the nono sandbox harness, installation, fallback policy, and the
requirement that new integration tests use sandboxed_command().
```
