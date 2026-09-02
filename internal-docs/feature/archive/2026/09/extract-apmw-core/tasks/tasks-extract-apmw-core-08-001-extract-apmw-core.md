---
story_id: "08-001"
story_title: "Extract detect+ecosystem+version into apmw-core crate (Cargo workspace)"
story_name: "extract-apmw-core"
prd_name: "extract-apmw-core"
prd_file: "internal-docs/feature/todo/extract-apmw-core/feat-202609021309-extract-apmw-core.md"
phase: 8
parallel_id: 1
branch: "feature/current/extract-apmw-core/story-08-001-extract-apmw-core"
status: "todo"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: false
modules: ["crates/apmw-core/", "crates/apmw/", "Cargo.toml"]
priority: "MUST"
risk_level: "high"
tags: ["feat", "refactor", "workspace"]
due: ""
create-date: "2026-09-02"
update-date: "2026-09-02"
---

## Summary

Convert the single-crate `apmw` repo into a Cargo workspace with two members:
`crates/apmw-core` (the reusable library) and `crates/apmw` (the binary).
Move the `detect`, `ecosystem` (mod.rs only, NOT mapping.rs), and `version`
modules from `src/` into `crates/apmw-core/src/`. Create a crate-local error
type in `apmw-core`. Update the binary to depend on `apmw-core` via
`path + version`. Keep `ecosystem/mapping.rs` in the binary crate. The binary
must compile and all existing tests must pass after the extraction.

## Sub-Tasks

- [ ] Create Cargo workspace structure: root `Cargo.toml` becomes a virtual
  manifest (`[workspace]` section, no `[package]`). Add `[workspace.dependencies]`
  for shared deps and `[workspace.lints]` if applicable.
- [ ] Create `crates/apmw-core/` directory with `Cargo.toml` (package metadata:
  name=apmw-core, version=0.1.0, edition=2021, rust-version=1.70, license=MIT,
  description, repository).
- [ ] Create `crates/apmw/` directory with `Cargo.toml` (package metadata
  matching the original, depends on `apmw-core = { path = "../apmw-core",
  version = "0.1.0" }`).
- [ ] Move `src/detect/` → `crates/apmw-core/src/detect/`
- [ ] Move `src/ecosystem/mod.rs` → `crates/apmw-core/src/ecosystem/mod.rs`
  (do NOT move `mapping.rs`)
- [ ] Move `src/version/` → `crates/apmw-core/src/version/`
- [ ] Create `crates/apmw-core/src/error.rs` with a `CoreError` type (thiserror)
  that covers the error variants used by detect, ecosystem, and version modules.
- [ ] Create `crates/apmw-core/src/lib.rs` that exports `detect`, `ecosystem`,
  `version`, and `error` modules.
- [ ] Update all `crate::error::ApmwError` references in the moved modules to
  `crate::error::CoreError` (or re-export as needed).
- [ ] Update all `crate::detect`, `crate::ecosystem`, `crate::version`
  references in the binary's remaining modules to `apmw_core::detect`, etc.
- [ ] Keep `ecosystem/mapping.rs` in the binary crate at
  `crates/apmw/src/ecosystem/mapping.rs`. Update the binary's `ecosystem`
  module to re-export from `apmw_core::ecosystem` and add `mapping.rs` locally.
- [ ] Move binary source files to `crates/apmw/src/` (main.rs, lib.rs, cli.rs,
  agent/, audit/, clone/, config/, containers/, daemon/, governance/, install/,
  output/, path_scan/, security/, telemetry/).
- [ ] Update `crates/apmw/src/lib.rs` to re-export from `apmw-core` where the
  binary's modules need it.
- [ ] Update `justfile` if any paths reference `src/` directly (they should
  still work since cargo resolves workspace members automatically).
- [ ] Update `tests/integration_tests.rs` and `benches/performance.rs` paths
  if needed (they reference `Command::cargo_bin("apmw")` which still works).
- [ ] Update `Dockerfile` if it references `src/` paths.
- [ ] Update `.github/workflows/ci.yml` if it references `src/` paths.
- [ ] Run `cargo test --workspace` and fix all compilation errors.
- [ ] Run `just validate` and fix all warnings/errors.

Status conventions: mark in-progress with `[~]`, done with `[x]`, blocked with `[!]`.

## Relevant Files

- `Cargo.toml` — becomes virtual manifest (workspace root)
- `crates/apmw-core/Cargo.toml` — new library crate manifest
- `crates/apmw-core/src/lib.rs` — new library root
- `crates/apmw-core/src/error.rs` — new crate-local error type
- `crates/apmw-core/src/detect/` — moved from `src/detect/`
- `crates/apmw-core/src/ecosystem/mod.rs` — moved from `src/ecosystem/mod.rs`
- `crates/apmw-core/src/version/` — moved from `src/version/`
- `crates/apmw/Cargo.toml` — binary crate manifest (depends on apmw-core)
- `crates/apmw/src/lib.rs` — binary lib root (re-exports from apmw-core)
- `crates/apmw/src/main.rs` — binary entry
- `crates/apmw/src/ecosystem/mapping.rs` — stays in binary
- `justfile` — may need path updates
- `Dockerfile` — may need path updates
- `.github/workflows/ci.yml` — may need path updates

## Acceptance Criteria (Gherkin)

- Given the repo is a Cargo workspace, When `cargo build --workspace` is run,
  Then both `apmw-core` and `apmw` compile without errors.
- Given the extraction is complete, When `cargo test --workspace` is run,
  Then all existing tests pass with 0 failures.
- Given the extraction is complete, When `just validate` is run,
  Then all quality gates pass (check, test, clippy, fmt, doc, build, audit).
- Given `ecosystem/mapping.rs` exists, When inspecting `crates/apmw-core/`,
  Then `mapping.rs` is NOT present in `apmw-core` (it stays in the binary).
- Given the binary depends on `apmw-core`, When inspecting `crates/apmw/Cargo.toml`,
  Then the dependency is declared as `{ path = "../apmw-core", version = "0.1.0" }`.

## Test Plan

- `cargo test --workspace` — all existing tests pass
- `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
- `cargo fmt --check` — formatting passes
- `cargo doc --workspace` — docs build without warnings
- `just validate` — all 7 quality gates pass

## Observability

- No new logging/telemetry needed (this is a structural refactor).

## Compliance

- N/A (no regulatory concerns for a crate extraction).

## Risks & Mitigations

- **Risk**: Circular dependencies between core and binary.
  **Mitigation**: `apmw-core` must not depend on `apmw`. The binary depends on
  core, never the reverse. Verify with `cargo tree`.
- **Risk**: Missing `pub use` re-exports break the binary's modules.
  **Mitigation**: After moving, grep for all `crate::detect`, `crate::ecosystem`,
  `crate::version` references in the binary and update them to
  `apmw_core::detect`, etc.
- **Risk**: `ecosystem/mapping.rs` accidentally moved to core.
  **Mitigation**: Explicitly verify `crates/apmw-core/src/ecosystem/` does not
  contain `mapping.rs` after the move.
- **Risk**: CI/Docker paths break.
  **Mitigation**: Update `Dockerfile` and `.github/workflows/ci.yml` to
  reference the new `crates/` layout if they hardcode `src/` paths.

## Dependencies & Sequencing

- **Depends on**: nothing (this is the first story).
- **Unblocks**: 08-002 (YAML custom types), 08-003 (workspace detection),
  08-004 (version info extraction), 08-005 (publish).

## Definition of Done

- `cargo test --workspace` passes with 0 failures
- `just validate` passes (all quality gates)
- `crates/apmw-core/` exists with `detect`, `ecosystem` (no mapping.rs),
  `version`, and `error` modules
- `crates/apmw/` exists and depends on `apmw-core` via `path + version`
- `ecosystem/mapping.rs` is in `crates/apmw/src/ecosystem/mapping.rs`, NOT in
  `apmw-core`

## Commit Conventions

- `refactor(workspace): extract apmw-core crate from apmw binary`
- `refactor(detect): move detection module to apmw-core`
- `refactor(ecosystem): move ecosystem module to apmw-core (mapping.rs stays in binary)`
- `refactor(version): move version module to apmw-core`
