---
story_id: "02-002"
story_title: "PATH scanner (cli-tool-discovery integration)"
story_name: "path-scanner"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 2
parallel_id: 2
branch: "feature/current/apmw/story-02-002-path-scanner"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-001"]
parallel_safe: true
modules: ["src/path_scan/"]
priority: "MUST"
risk_level: "low"
tags: ["feat", "path-scan", "cli-tool-discovery"]
due: "2026-08-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the PATH scanner that checks if a tool is already installed before attempting installation. The scanner integrates with `cli-tool-discovery.sh` for devbox-aware resolution (check DEVBOX_SHELL first), wrapper detection (mise, flox, direnv, nix), 30+ standard PATH locations, and repo-root fallback dirs.

## Current State

- **Relevant files and their roles:**
  - PRD FR-3 (sections FR-3.1 through FR-3.3) — PATH scanning requirements
  - cli-tool-discovery.md at `/Users/micro/p/gh/levonk/skills-src/build/current/includes/cli-tool-discovery.md`
- **Repository conventions:** Module by feature/domain. Use pub use in lib.rs.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/path_scan/mod.rs` — PATH scanner module
- Create `src/path_scan/discovery.rs` — cli-tool-discovery.sh integration (subprocess call or Rust reimplementation)
- Implement devbox-aware resolution: check DEVBOX_SHELL/IN_DEVBOX_SHELL env first
- Implement wrapper detection: mise, flox, direnv, nix
- Implement PATH location scanning: 30+ standard locations
- Implement repo-root fallback: $REPO_ROOT/bin, scripts/, .local/bin
- Return ScanResult: Found(path), Wrapper(cmd), NotFound
- Support JSON output mode for programmatic use
- Add unit tests with mock PATH environments
- Add integration tests with assert_cmd

**Out of scope:**
- Actual tool installation (story 04-001)
- Ad-hoc runner resolution (part of install engine, story 04-001)

## Sub-Tasks

- [ ] Create `src/path_scan/mod.rs` with PathScanner struct
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/path_scan/discovery.rs` with cli-tool-discovery integration
  **Verify**: `cargo test discovery` → tests pass
- [ ] Implement devbox-aware resolution (check DEVBOX_SHELL env first)
  **Verify**: `cargo test devbox_resolution` → tests pass
- [ ] Implement wrapper detection (mise, flox, direnv, nix)
  **Verify**: `cargo test wrapper_detection` → tests pass
- [ ] Implement PATH location scanning (30+ locations)
  **Verify**: `cargo test path_scan` → tests pass
- [ ] Implement repo-root fallback dirs
  **Verify**: `cargo test repo_fallback` → tests pass
- [ ] Implement ScanResult enum (Found, Wrapper, NotFound) with serde
  **Verify**: `cargo test scan_result` → tests pass
- [ ] Add unit tests with mock PATH environments
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/path_scan/mod.rs` — PATH scanner module
- `src/path_scan/discovery.rs` — cli-tool-discovery integration
- `src/lib.rs` — Add `pub mod path_scan;`

## Acceptance Criteria

- [ ] Scanner finds tools on PATH in under 50ms (PRD NFR-1.2)
- [ ] Devbox-aware resolution checks DEVBOX_SHELL first
- [ ] Wrapper detection covers mise, flox, direnv, nix
- [ ] PATH scanning covers 30+ standard locations
- [ ] Repo-root fallback checks $REPO_ROOT/bin, scripts/, .local/bin
- [ ] ScanResult correctly reports Found, Wrapper, or NotFound
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test path_scan` — scanner tests with mock environments
- Integration: `assert_cmd` tests
- Lint: `just lint`

## Observability

- PATH scan results should be logged at `debug` level
- NotFound results should be logged at `info` level

## Compliance

- PRD FR-3 (PATH scanning requirements)
- cli-tool-discovery.md contract

## Risks & Mitigations

- Risk: cli-tool-discovery.sh may not be available on all platforms — Mitigation: Implement the resolution logic in Rust directly, use the script as a reference

## Dependencies & Sequencing

- Depends on: 01-001 (error types)
- Unblocks: 04-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- cli-tool-discovery.sh's resolution flow does not work as documented
- PATH scanning cannot be implemented cross-platform

## Maintenance Notes

- New wrapper types can be added by extending the wrapper detection list
- Reviewers should check that the scanner is fast (under 50ms)

## Commit Conventions

- `feat(path_scan): add PATH scanner with cli-tool-discovery integration`
