---
story_id: "02-001"
story_title: "Package manager detection engine"
story_name: "detection-engine"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 2
parallel_id: 1
branch: "feature/current/apmw/story-02-001-detection-engine"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001", "01-002", "01-003", "01-004", "01-005"]
parallel_safe: true
modules: ["src/detect/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "detection", "package-manager"]
due: "2026-08-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the package manager detection engine that scans the current project directory for lockfiles, config files, and directory markers to identify the correct package manager. The detection logic delegates to the project-detection/project-adopter/project-configuration skills' attribute-based detection. The 2ndbrain research Table 1 defines detection attributes for 20+ package managers.

## Current State

- **Relevant files and their roles:**
  - `src/main.rs` (lines 61-63) — `Detect` command stub: `println!("Detecting package manager...");`
  - `src/error.rs` — Has `PackageManagerNotFound` error variant
  - PRD FR-1 (sections FR-1.1 through FR-1.4) — Detection requirements
  - 2ndbrain research Table 1 at `/Users/micro/Documents/2ndbrain/2ndbrain/Work/01 OandO/Self Improvement/Want To Build/Digital/All Package Manager Wrapper/All Package Manager Wrapper Tool Landscape.md`
- **Repository conventions:** Module by feature/domain. Use pub use in lib.rs. thiserror for errors.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/detect/mod.rs` — Detection engine orchestrator
- Create `src/detect/managers.rs` — Package manager definitions (name, lockfiles, config files, directory markers, detection priority)
- Create `src/detect/attributes.rs` — Detection attributes from 2ndbrain Table 1 (npm, yarn, pnpm, pip, poetry, pipenv, pdm, conda, uv, maven, gradle, sbt, go, cargo, flutter, dart, dotnet, xcode, cocoapods, carthage, spm, cmake, ant, brew, nix, snap, yum, apt, winget, apk)
- Implement `DetectionResult` struct: manager, confidence, evidence (Vec of file paths)
- Implement detection by scanning current directory for lockfiles and config files
- Implement confidence scoring (multiple matching files = higher confidence)
- Wire `apmw detect` command to the detection engine
- Output detection result in TOON format (agent mode) or human-readable (human mode)
- Write audit log entry for each detection operation
- Add unit tests with mock project directories (tempfile)
- Add integration tests with assert_cmd for `apmw detect`

**Out of scope:**
- Ecosystem mapping (story 02-003)
- PATH scanning (story 02-002)
- Install logic (story 04-001)

## Sub-Tasks

- [ ] Create `src/detect/managers.rs` with PackageManager definitions (name, lockfiles, config_files, dir_markers)
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/detect/attributes.rs` with detection attributes from 2ndbrain Table 1
  **Verify**: `cargo test attributes` → tests pass
- [ ] Create `src/detect/mod.rs` with DetectionEngine (scan directory, match attributes, score confidence)
  **Verify**: `cargo test detect` → tests pass
- [ ] Implement DetectionResult struct with serde
  **Verify**: `cargo test detection_result` → tests pass
- [ ] Wire `apmw detect` command to detection engine
  **Verify**: `apmw detect` in a Cargo.toml project → returns "cargo"
- [ ] Output detection result in TOON format (agent mode)
  **Verify**: `apmw detect` → valid TOON output
- [ ] Write audit log entry for detection operation
  **Verify**: audit log file contains entry after `apmw detect`
- [ ] Add unit tests with mock project directories (tempfile with Cargo.toml, package.json, etc.)
  **Verify**: `just test` → all pass
- [ ] Add integration tests with assert_cmd
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/detect/mod.rs` — Detection engine
- `src/detect/managers.rs` — Package manager definitions
- `src/detect/attributes.rs` — Detection attributes
- `src/lib.rs` — Add `pub mod detect;`
- `src/main.rs` — Wire Detect command

## Acceptance Criteria

- [ ] Detection correctly identifies cargo in a project with Cargo.toml
- [ ] Detection correctly identifies pnpm in a project with pnpm-lock.yaml
- [ ] Detection correctly identifies npm in a project with package-lock.json
- [ ] Detection correctly identifies pip/poetry in a project with pyproject.toml
- [ ] Detection correctly identifies go in a project with go.mod
- [ ] Detection returns manager, confidence, and evidence
- [ ] Output is in TOON format in agent mode
- [ ] Audit log entry is written for each detection
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test detect` — detection engine tests with mock directories
- Integration: `assert_cmd` tests for `apmw detect` in various project types
- Lint: `just lint`

## Observability

- Detection results should be logged at `info` level
- Detection failures should be logged at `warn` level
- Confidence scores should be logged at `debug` level

## Compliance

- PRD FR-1 (detection requirements)
- 2ndbrain research Table 1 (detection attributes)
- ADR-20260607001 sections 36-45 (AXI output)

## Risks & Mitigations

- Risk: Projects may have multiple package managers (e.g., Cargo.toml + package.json) — Mitigation: Return all detected managers with confidence scores, let the caller decide

## Dependencies & Sequencing

- Depends on: 01-001 (error types), 01-002 (CLI), 01-003 (AXI output), 01-004 (daemon), 01-005 (audit log)
- Unblocks: 03-001, 04-001, 06-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The 2ndbrain research tables are not accessible at the documented path
- Detection attributes do not cover the required 20+ package managers

## Maintenance Notes

- New package managers can be added by extending the managers.rs definitions
- Reviewers should check that confidence scoring is reasonable
- Detection should be fast (under 100ms per PRD NFR-1.1)

## Commit Conventions

- `feat(detect): add package manager detection engine with 20+ manager support`
