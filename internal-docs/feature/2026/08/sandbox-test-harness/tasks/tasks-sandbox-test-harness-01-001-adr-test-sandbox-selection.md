---
story_id: "01-001"
story_title: "ADR: test sandbox selection"
story_name: "adr-test-sandbox-selection"
prd_name: "sandbox-test-harness"
prd_file: "internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md"
phase: 1
parallel_id: 1
branch: "feature/current/sandbox-test-harness/story-01-001-adr-test-sandbox-selection"
status: "todo"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["internal-docs/adr"]
priority: "MUST"
risk_level: "low"
tags: ["docs", "adr", "security"]
due: "2026-08-06"
create-date: "2026-08-05"
update-date: "2026-08-05"
---

## Summary

Create the apmw ADR directory (`internal-docs/adr/`) and write the ADR
documenting the sandbox tool choice (nono), the nono profile schema, the
fallback policy, and the TempDir-inside-sandbox hybrid approach. This ADR
references the skills-src ADR `adr-202608030953-skill-refresh-sandbox-selection`
as the prior evaluation.

## Sub-Tasks

- [ ] Create `internal-docs/adr/2026/08/` directory
- [ ] Write `adr-202608052241-test-sandbox-selection.md` with:
  - [ ] Frontmatter (adr-id, slug, status: accepted, date, tags, scope)
  - [ ] Context: why apmw tests need sandboxing (current unsandboxed state)
  - [ ] Decision: nono, referencing skills-src ADR
  - [ ] nono profile schema (fs_read, fs_write, net_allow, net_deny, env)
  - [ ] Fallback policy (unsandboxed + warning on dev, mandatory in CI)
  - [ ] Hybrid approach (TempDir inside sandbox, /tmp/apmw-tests/<test>/)
  - [ ] Alternatives considered (sandbox-exec, devbox isolation, Docker)
  - [ ] Consequences (positive, negative, neutral)
  - [ ] Rollout / migration notes
  - [ ] References to skills-src ADR and nono docs

## Relevant Files

- `internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md` — the ADR (new)
- `~/p/gh/levonk/skills-src/internal-docs/adr/2026/08/adr-202608030953-skill-refresh-sandbox-selection.md` — reference ADR (read-only, cite it)

## Acceptance Criteria (Gherkin)

- Given the apmw repo, When a contributor looks for the sandbox decision, Then
  they find `internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md`
- Given the ADR file, When a reader checks the decision, Then it says "nono" and
  references the skills-src ADR as the prior evaluation
- Given the ADR file, When a reader checks the profile schema, Then it lists
  fs_read, fs_write, net_allow, net_deny, and env keys with the allowed
  paths/domains from the PRD
- Given the ADR file, When a reader checks the fallback policy, Then it
  documents: unsandboxed + warning on local dev, mandatory in CI via
  APMW_TEST_SANDBOX_REQUIRED=1

## Test Plan

- No code tests — this is a documentation story
- Verify the file exists at the expected path
- Verify the ADR frontmatter has required fields (adr-id, status, date)
- Verify the ADR references the skills-src ADR by filename

## Observability

- N/A (documentation only)

## Compliance

- The ADR must not contain AI attribution boilerplate (per AGENTS.md)
- The ADR follows the date-embedded naming convention

## Risks & Mitigations

- **Risk**: ADR format may not match apmw's conventions (apmw has no prior ADRs)
- **Mitigation**: Follow the skills-src ADR format as the template

## Dependencies & Sequencing

- No dependencies — can start immediately
- Unblocks: nothing directly (other Phase 1 stories are parallel)

## Definition of Done

- ADR file exists at `internal-docs/adr/2026/08/adr-202608052241-test-sandbox-selection.md`
- ADR covers all sections listed in Sub-Tasks
- ADR references the skills-src ADR
- No AI attribution boilerplate

## Commit Conventions

```
docs(adr): add test sandbox selection ADR

Document the decision to use nono for sandboxing apmw integration tests,
referencing the skills-src ADR adr-202608030953 as the prior evaluation.
Covers profile schema, fallback policy, and hybrid TempDir approach.
```
