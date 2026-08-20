---
story_id: "01-002"
story_title: "Infra: devbox + CI + just doctor"
story_name: "infra-devbox-ci-doctor"
prd_name: "sandbox-test-harness"
prd_file: "internal-docs/feature/2026/08/sandbox-test-harness/feat-202608052241-sandbox-test-harness.md"
phase: 1
parallel_id: 2
branch: "feature/current/sandbox-test-harness/story-01-002-infra-devbox-ci-doctor"
status: "todo"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["devbox.json", ".github/workflows/ci.yml", "justfile"]
priority: "MUST"
risk_level: "low"
tags: ["infra", "ci", "devbox"]
due: "2026-08-06"
create-date: "2026-08-05"
update-date: "2026-08-05"
---

## Summary

Add nono to the development environment and CI pipeline. Update `devbox.json`
to include nono, update the CI workflow to install nono and set
`APMW_TEST_SANDBOX_REQUIRED=1`, and update `just doctor` to check for nono
presence.

## Sub-Tasks

- [ ] Add `nono` to `devbox.json` packages
- [ ] Update `.github/workflows/ci.yml` to install nono (via
      `curl -fsSL https://nono.sh/install.sh | sh`) in a setup step
- [ ] Set `APMW_TEST_SANDBOX_REQUIRED=1` env var in the CI test job
- [ ] Update `justfile` `doctor` recipe to check for nono and report
      presence/absence
- [ ] Verify `devbox shell` provides nono after the update

## Relevant Files

- `devbox.json` — add nono to packages
- `.github/workflows/ci.yml` — add nono install step + env var
- `justfile` — update doctor recipe

## Acceptance Criteria (Gherkin)

- Given `devbox.json`, When a contributor runs `devbox shell`, Then nono is
  available on PATH
- Given the CI workflow, When a CI job runs, Then nono is installed before
  tests execute
- Given the CI workflow, When the test job runs, Then `APMW_TEST_SANDBOX_REQUIRED`
  is set to `1`
- Given `just doctor`, When a contributor runs it, Then it reports whether nono
  is installed

## Test Plan

- Run `devbox run -- nono --version` and verify it succeeds
- Run `just doctor` and verify it reports nono status
- Review CI workflow YAML for correctness (nono install step, env var)

## Observability

- `just doctor` output includes nono status line

## Compliance

- devbox.json changes must be surgical (add nono, do not remove existing packages)
- CI workflow changes must not break existing jobs

## Risks & Mitigations

- **Risk**: nono may not be available in Nixpkgs (devbox uses Nix)
- **Mitigation**: If nono is not in Nixpkgs, use a postShellHook in devbox.json
  to install nono via curl, or document that contributors install it manually
  via `brew install nono` / curl
- **Risk**: nono install script may fail in CI (network issues)
- **Mitigation**: Add retry logic or use a pinned version

## Dependencies & Sequencing

- No dependencies — can start immediately
- Unblocks: nothing directly (other Phase 1 stories are parallel)

## Definition of Done

- `devbox.json` includes nono (or a postShellHook installs it)
- CI workflow installs nono and sets `APMW_TEST_SANDBOX_REQUIRED=1`
- `just doctor` reports nono status
- `devbox run -- nono --version` succeeds

## Commit Conventions

```
infra: add nono sandbox to devbox, CI, and just doctor

- Add nono to devbox.json packages
- Install nono in CI workflow and set APMW_TEST_SANDBOX_REQUIRED=1
- Update just doctor to check for nono presence
```
