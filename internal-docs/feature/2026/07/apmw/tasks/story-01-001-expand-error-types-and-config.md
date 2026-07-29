---
story_id: "01-001"
story_title: "Expand error types and config module"
story_name: "expand-error-types-and-config"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 1
parallel_id: 1
branch: "feature/current/apmw/story-01-001-expand-error-types-and-config"
status: "done"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["src/error.rs", "src/config/"]
priority: "MUST"
risk_level: "low"
tags: ["feat", "foundation", "error-handling", "config"]
due: "2026-08-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Expand the existing `src/error.rs` with new error variants for all apmw features (detection, version resolution, ecosystem mapping, clone, MCP, hooks). Create a `src/config/` module for configuration management following ADR-20260607001 section 2 (config precedence: CLI args > env vars > local project config > user config (XDG) > system config > defaults) and section 3 (config file initialization on first run).

## Current State

- **Relevant files and their roles:**
  - `src/error.rs` (lines 1-27) — Current error types with 5 variants: `Io`, `Config`, `PackageManagerNotFound`, `SecurityScanFailed`, `Daemon`. Uses `thiserror` with `#[from]` for `std::io::Error`.
  - `src/lib.rs` (lines 1-14) — Library root, exports `error::ApmwError` and `error::Result`.
  - `Cargo.toml` (lines 19-30) — Dependencies include `thiserror`, `anyhow`, `serde`, `clap`.
- **Existing code excerpts:**
  ```rust
  // src/error.rs:8-24
  #[derive(Error, Debug)]
  pub enum ApmwError {
      #[error("IO error: {0}")]
      Io(#[from] std::io::Error),
      #[error("Configuration error: {0}")]
      Config(String),
      #[error("Package manager not found: {0}")]
      PackageManagerNotFound(String),
      #[error("Security scan failed: {0}")]
      SecurityScanFailed(String),
      #[error("Daemon error: {0}")]
      Daemon(String),
  }
  ```
- **Repository conventions:** Error handling uses thiserror for library code, anyhow for binary code. Never panic! in library code. Implement From via #[from]. See AGENTS.md lines 156-162.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |
  | Validate  | `just validate`          | all gates pass  |

## Scope

**In scope:**
- Expand `ApmwError` with variants: `VersionResolution`, `EcosystemMapping`, `CloneFailed`, `PathScanFailed`, `McpError`, `HookError`, `AuditLogError`, `IndexError`
- Add `#[from]` implementations where applicable (e.g., `serde_json::Error`, `toml::de::Error`)
- Create `src/config/mod.rs` — Configuration struct with serde, config precedence chain, XDG path resolution
- Create `src/config/migration.rs` — Config auto-migration (ADR section 29: detect legacy, create .bak, validate)
- Implement config file initialization on first run (ADR section 3: create default config with all settings commented out)
- Add `toml` crate to Cargo.toml dependencies
- Add unit tests for config loading, precedence, and migration

**Out of scope:**
- CLI argument parsing (story 01-002)
- Daemon implementation (story 01-004)
- Actual feature logic (detection, install, etc.)

## Sub-Tasks

- [x] Expand `ApmwError` enum with new variants for all features
  **Verify**: `cargo check` → exit 0
- [x] Add `toml` crate to Cargo.toml dependencies
  **Verify**: `cargo build` → exit 0
- [x] Create `src/config/mod.rs` with `ApmwConfig` struct (serde), config precedence chain, XDG path resolution
  **Verify**: `cargo test config` → tests pass
- [x] Create `src/config/migration.rs` with auto-migration logic (detect legacy, .bak backup, validate)
  **Verify**: `cargo test migration` → tests pass
- [x] Implement config file initialization (create default config with commented-out settings on first run)
  **Verify**: `cargo test config_init` → tests pass
- [x] Add unit tests for all new error variants and config operations
  **Verify**: `just test` → all pass
- [x] Run `just validate` to confirm all quality gates pass
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/error.rs` — Expand with new error variants
- `src/config/mod.rs` — New config module
- `src/config/migration.rs` — New config migration module
- `src/lib.rs` — Add `pub mod config;` and re-exports
- `Cargo.toml` — Add `toml` dependency

## Acceptance Criteria

- [x] `ApmwError` has variants for all features listed in scope
- [x] Config module loads from XDG paths with correct precedence
- [x] Config file initialization creates a default config with commented-out settings
- [x] Config auto-migration creates .bak backup and validates migrated config
- [x] All unit tests pass
- [x] `just validate` passes with all 7 quality gates green

## Test Plan

- Unit: `cargo test config` — config loading, precedence, initialization, migration
- Unit: `cargo test error` — error variant construction and Display
- Lint: `just lint` — clippy with -D warnings
- Format: `just format` — rustfmt check

## Observability

- Config loading errors should be logged at `error` level via `tracing`
- Config migration should be logged at `info` level
- Config precedence resolution should be logged at `debug` level

## Compliance

- ADR-20260607001 section 2 (config precedence)
- ADR-20260607001 section 3 (config file initialization)
- ADR-20260607001 section 29 (config auto-migration)
- rust-development-practices error-handling.md

## Risks & Mitigations

- Risk: TOML parsing errors on malformed user configs — Mitigation: Validate config on load, report clear errors with line numbers (ADR section 21)

## Dependencies & Sequencing

- Depends on: None (foundation story)
- Unblocks: 02-001, 02-002, 02-003, 03-001, 05-001, 06-002

## Definition of Done

- [x] All verification commands from sub-tasks pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified (`git status`)

## STOP Conditions

Stop and report if:
- The code at `src/error.rs` doesn't match the excerpts (5 variants with thiserror)
- `toml` crate is not available for MSRV 1.70
- A dependency cannot be satisfied

## Maintenance Notes

- Future stories will add config fields as features are implemented
- Reviewers should check that error variants use `#[from]` where applicable and `String` where not
- Config migration logic should be extended as the config schema evolves

## Commit Conventions

- Use conventional commits: `feat(error): expand ApmwError with feature-specific variants`
- `feat(config): add config module with XDG precedence and auto-migration`
