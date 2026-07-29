---
story_id: "07-002"
story_title: "Documentation finalization (man pages, shell completion, README)"
story_name: "documentation-finalization"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 7
parallel_id: 2
branch: "feature/current/apmw/story-07-002-documentation-finalization"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["06-001", "06-002", "06-003"]
parallel_safe: true
modules: ["docs/", "README.md"]
priority: "SHOULD"
risk_level: "low"
tags: ["feat", "docs", "man-pages", "completion", "readme"]
due: "2026-11-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Finalize all documentation: man pages (accessible via `man apmw` or `--man` flag), shell completion scripts (bash, zsh, fish), and a comprehensive README. Follow ADR-20260607001 sections 17-19 (shell completion, man pages, pager integration).

## Current State

- **Relevant files and their roles:**
  - `README.md` — Existing README (may need updates)
  - `AGENTS.md` — Project conventions (already comprehensive)
  - PRD FR-13.1 — CLI standards compliance including shell completion, man pages
  - ADR-20260607001 sections 17-19 — Shell completion, man pages, pager integration
- **Repository conventions:** Documentation in docs/ directory. Man pages in man/ directory.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Generate man pages from CLI help text (accessible via `man apmw` or `--man` flag)
- Generate shell completion scripts for bash, zsh, and fish (auto-generated from clap)
- Update README with comprehensive documentation: installation, usage, configuration, examples
- Create `docs/architecture.md` — Architecture overview
- Create `docs/examples/` — Example usage scenarios
- Ensure `--help` output is comprehensive and follows ADR standards
- Ensure `--usage` shows brief usage summary
- Add `--no-pager` flag support (ADR section 19)
- Verify all ADR-mandated documentation is present

**Out of scope:**
- Multi-ecosystem packaging (story 07-001)
- Feature implementation (all features complete)

## Sub-Tasks

- [ ] Generate man pages from CLI help text
  **Verify**: `man apmw` → shows man page
- [ ] Generate shell completion scripts for bash, zsh, fish
  **Verify**: Completion scripts work in each shell
- [ ] Update README with installation, usage, configuration, examples
  **Verify**: README covers all features
- [ ] Create `docs/architecture.md` with architecture overview
  **Verify**: `docs/architecture.md` → complete
- [ ] Create `docs/examples/` with example usage scenarios
  **Verify**: `docs/examples/` → has examples
- [ ] Verify --help output is comprehensive
  **Verify**: `apmw --help` → shows all commands and flags
- [ ] Verify --usage shows brief usage summary
  **Verify**: `apmw --usage` → brief summary
- [ ] Add --no-pager flag support
  **Verify**: `apmw --no-pager status` → no pager
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `README.md` — Comprehensive documentation
- `docs/architecture.md` — Architecture overview
- `docs/examples/` — Example usage
- `man/` — Man pages
- `completions/` — Shell completion scripts

## Acceptance Criteria

- [ ] Man pages are accessible via `man apmw` and `--man` flag
- [ ] Shell completion scripts work in bash, zish, and fish
- [ ] README covers installation, usage, configuration, and examples
- [ ] Architecture documentation is complete
- [ ] Example usage scenarios are documented
- [ ] --help output is comprehensive
- [ ] --usage shows brief summary
- [ ] --no-pager flag works
- [ ] `just validate` passes

## Test Plan

- Manual: Verify man page displays correctly
- Manual: Verify shell completion works in each shell
- Lint: `just lint`

## Observability

- N/A (documentation story)

## Compliance

- ADR-20260607001 sections 17-19 (shell completion, man pages, pager)
- ADR-20260607001 section 1 (standard arguments: --help, --version, --usage)

## Risks & Mitigations

- Risk: Man page generation may require additional tooling — Mitigation: Use clap_mangen for auto-generation

## Dependencies & Sequencing

- Depends on: 06-001 (MCP), 06-002 (hooks), 06-003 (skills/docs)
- Unblocks: None (final phase)

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- Man page generation tooling is not available
- Shell completion auto-generation does not work with clap

## Maintenance Notes

- Documentation should be updated as features are added or changed
- Reviewers should check that README covers all features
- Man pages and completion scripts should be auto-generated, not hand-maintained

## Commit Conventions

- `docs: finalize man pages, shell completion, and comprehensive README`
