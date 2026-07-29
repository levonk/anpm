---
story_id: "03-001"
story_title: "Version resolution engine (pinned/engine/latest/latest-minor + min-age-days supply-chain defense)"
story_name: "version-resolution"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 3
parallel_id: 1
branch: "feature/current/apmw/story-03-001-version-resolution"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001", "01-005", "02-001"]
parallel_safe: true
modules: ["src/version/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "version", "resolution"]
due: "2026-09-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the version resolution engine that resolves package versions using this priority: pinned version if locked (from lockfile), latest version compatible with the engine in use, latest if unspecified, latest minor if only a major version is specified. The engine respects engine constraints from the project's config (e.g., `engines` in package.json, `python-requires` in setup.py, `rust-version` in Cargo.toml).

**Supply-chain defense (min-age-days):** By default, the engine refuses versions published less than 2 days ago (default `min-age-days=2`). If the latest version is too new, the engine falls back to the most recent version published >= 2 days ago. This check applies only to auto-resolved versions (priority 3 "latest" and priority 4 "latest-minor"), NOT to pinned versions (priority 1) or engine-constrained versions (priority 2). The min-age-days threshold is overrideable via:
- `--min-age-days 0` CLI flag (set to 0 to disable)
- `--allow-new` CLI flag (alias for `--min-age-days 0`)
- `min_age_days` config setting in TOML
- `APMW_MIN_AGE_DAYS` environment variable

## Current State

- **Relevant files and their roles:**
  - PRD FR-4 (sections FR-4.1 through FR-4.2) — Version resolution requirements
  - `src/error.rs` — Will need VersionResolution error variant (from story 01-001)
  - `src/detect/` — Detection engine provides the detected manager (from story 02-001)
- **Repository conventions:** Module by feature/domain. Use serde for data structures.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/version/mod.rs` — Version resolution engine
- Create `src/version/resolver.rs` — Resolution logic (pinned, engine-compatible, latest, latest-minor)
- Implement `VersionResolution` struct: requested, resolved, resolution_strategy (Pinned, EngineCompatible, Latest, LatestMinor)
- Implement pinned version resolution (read from lockfile: Cargo.lock, package-lock.json, pnpm-lock.yaml, poetry.lock, etc.)
- Implement engine-compatible resolution (read engine constraints: engines in package.json, python-requires, rust-version in Cargo.toml)
- Implement latest version resolution (query the package registry for the latest version)
- Implement latest-minor resolution (query registry for latest minor of a specified major)
- Implement min-age-days supply-chain defense:
  - Default: refuse versions published < 2 days ago (min-age-days=2)
  - Fall back to the most recent version published >= 2 days old
  - Applies only to auto-resolved versions (priority 3 "latest" and priority 4 "latest-minor")
  - Does NOT apply to pinned versions (priority 1) or engine-constrained versions (priority 2)
  - Overrideable via `--min-age-days 0`, `--allow-new`, config `min_age_days`, env `APMW_MIN_AGE_DAYS`
- Add unit tests for each resolution strategy
- Add unit tests for min-age-days defense (refuse too-new, fall back to older, override behavior)
- Add property-based tests with proptest for version comparison

**Out of scope:**
- Install engine (story 04-001)
- Registry API implementation (use HTTP calls to package registries)

## Sub-Tasks

- [ ] Create `src/version/mod.rs` with VersionResolver struct
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/version/resolver.rs` with resolution strategies
  **Verify**: `cargo test resolver` → tests pass
- [ ] Implement pinned version resolution (read from lockfiles)
  **Verify**: `cargo test pinned` → tests pass
- [ ] Implement engine-compatible resolution (read engine constraints)
  **Verify**: `cargo test engine_compatible` → tests pass
- [ ] Implement latest version resolution (query registry)
  **Verify**: `cargo test latest` → tests pass
- [ ] Implement latest-minor resolution (query registry for latest minor of major)
  **Verify**: `cargo test latest_minor` → tests pass
- [ ] Implement min-age-days supply-chain defense (default 2 days, refuse too-new, fall back)
  **Verify**: `cargo test min_age_days` → tests pass
- [ ] Implement min-age-days overrides (`--min-age-days 0`, `--allow-new`, config, env var)
  **Verify**: `cargo test min_age_overrides` → tests pass
- [ ] Verify min-age-days does not apply to pinned or engine-constrained versions
  **Verify**: `cargo test min_age_exclusions` → tests pass
- [ ] Implement VersionResolution struct with serde
  **Verify**: `cargo test version_resolution` → tests pass
- [ ] Add unit tests for each strategy with mock lockfiles and engine constraints
  **Verify**: `just test` → all pass
- [ ] Add property-based tests with proptest for version comparison
  **Verify**: `cargo test proptest` → tests pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/version/mod.rs` — Version resolution engine
- `src/version/resolver.rs` — Resolution logic
- `src/lib.rs` — Add `pub mod version;`
- `Cargo.toml` — May need `reqwest` or `ureq` for registry queries

## Acceptance Criteria

- [ ] Pinned version is read from lockfiles (Cargo.lock, package-lock.json, pnpm-lock.yaml, poetry.lock)
- [ ] Engine-compatible version respects engine constraints
- [ ] Latest version is queried from the registry
- [ ] Latest-minor version is queried for a specified major
- [ ] min-age-days defense refuses versions published < 2 days ago (default)
- [ ] min-age-days falls back to the most recent version >= 2 days old
- [ ] min-age-days applies only to auto-resolved versions (priority 3 and 4), not pinned or engine-constrained
- [ ] `--min-age-days 0` and `--allow-new` override the defense
- [ ] Config `min_age_days` and env `APMW_MIN_AGE_DAYS` override the defense
- [ ] VersionResolution records the strategy used
- [ ] All unit and property-based tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test version` — all version resolution tests
- Property: `cargo test proptest` — version comparison
- Lint: `just lint`

## Observability

- Version resolution results should be logged at `info` level
- Registry query failures should be logged at `error` level

## Compliance

- PRD FR-4 (version resolution requirements)

## Risks & Mitigations

- Risk: Registry API may be rate-limited or unavailable — Mitigation: Cache results, fall back to lockfile version if registry is unavailable
- Risk: min-age-days may block a critical freshly-published security patch — Mitigation: Override via `--allow-new` or `--min-age-days 0`; document the override mechanisms clearly

## Dependencies & Sequencing

- Depends on: 01-001 (error types), 01-005 (audit log), 02-001 (detection engine)
- Unblocks: 04-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- Registry APIs do not provide version information in a parseable format
- Lockfile formats are not as expected

## Maintenance Notes

- New lockfile formats can be added by extending the lockfile parsers
- Reviewers should check that engine constraints are properly enforced

## Commit Conventions

- `feat(version): add version resolution engine with pinned/engine/latest/latest-minor strategies and min-age-days supply-chain defense`
