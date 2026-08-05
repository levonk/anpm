# Task Index — sandbox-test-harness

| Story ID | Title | Branch | Dependencies | Parallel-safe | Modules | Status |
|----------|-------|--------|--------------|---------------|---------|--------|
| 01-001 | ADR: test sandbox selection | `feature/current/sandbox-test-harness/story-01-001-adr-test-sandbox-selection` | none | yes | `internal-docs/adr` | [ ] Todo |
| 01-002 | Infra: devbox + CI + just doctor | `feature/current/sandbox-test-harness/story-01-002-infra-devbox-ci-doctor` | none | yes | `devbox.json`, `.github/workflows/ci.yml`, `justfile` | [ ] Todo |
| 01-003 | Harness: sandbox module + nono profile | `feature/current/sandbox-test-harness/story-01-003-harness-sandbox-module-nono-profile` | none | yes | `tests/sandbox/` | [ ] Todo |
| 02-001 | Migrate all integration tests to sandbox | `feature/current/sandbox-test-harness/story-02-001-migrate-integration-tests-to-sandbox` | 01-003 | no | `tests/integration_tests.rs` | [ ] Todo |
| 03-001 | Isolation verification tests | `feature/current/sandbox-test-harness/story-03-001-isolation-verification-tests` | 02-001 | yes | `tests/sandbox_tests.rs` | [ ] Todo |
| 03-002 | Update AGENTS.md with sandbox notes | `feature/current/sandbox-test-harness/story-03-002-update-agents-md-sandbox-notes` | 02-001 | yes | `AGENTS.md` | [ ] Todo |

## Notes

- Phase 1 stories (01-001, 01-002, 01-003) are parallel-safe — they touch
  different files (ADR, devbox/CI, harness module) with no overlap.
- Phase 2 (02-001) is sequential — it depends on the harness module from
  01-003 and modifies `tests/integration_tests.rs` (the single largest file).
- Phase 3 stories (03-001, 03-002) are parallel-safe — 03-001 creates a new
  test file, 03-002 edits `AGENTS.md`. Both depend on 02-001 completing first.
