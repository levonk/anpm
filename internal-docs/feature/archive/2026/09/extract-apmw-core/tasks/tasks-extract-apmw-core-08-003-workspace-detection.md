---
story_id: "08-003"
story_title: "Add workspace/monorepo detection"
story_name: "workspace-detection"
prd_name: "extract-apmw-core"
prd_file: "internal-docs/feature/todo/extract-apmw-core/feat-202609021309-extract-apmw-core.md"
phase: 8
parallel_id: 3
branch: "feature/current/extract-apmw-core/story-08-003-workspace-detection"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["08-001"]
parallel_safe: true
modules: ["crates/apmw-core/src/workspace/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "workspace", "monorepo", "detection"]
due: ""
create-date: "2026-09-02"
update-date: "2026-09-02"
---

## Summary

Add a `workspace` module to `apmw-core` that detects workspace organizers
(Cargo workspace, pnpm workspace, npm workspace, Yarn workspace, Nx,
Turborepo, Lerna, Gradle composite, Maven multi-module) in addition to
individual package managers. The detection returns a `WorkspaceResult` with
the organizer type, the workspace root, and member project paths.

## Sub-Tasks

- [ ] Create `crates/apmw-core/src/workspace/mod.rs` with:
  - `WorkspaceOrganizer` enum: `Cargo`, `Pnpm`, `Npm`, `Yarn`, `Nx`,
    `Turborepo`, `Lerna`, `GradleComposite`, `MavenMultiModule`.
  - `WorkspaceResult` struct: `organizer: WorkspaceOrganizer`,
    `root: PathBuf`, `members: Vec<PathBuf>`.
  - `detect_workspace(path: &Path) -> Option<WorkspaceResult>` function that
    checks for workspace marker files and returns the result if found.
  - Per-organizer detection functions:
    - `detect_cargo_workspace()` — `Cargo.toml` with `[workspace]` section
      (parse with `toml` crate, check for `workspace.members` or
      `workspace.exclude`).
    - `detect_pnpm_workspace()` — `pnpm-workspace.yaml` exists.
    - `detect_npm_workspace()` — `package.json` with `workspaces` field
      (parse with `serde_json`).
    - `detect_yarn_workspace()` — `package.json` with `workspaces` field AND
      `yarn.lock` exists.
    - `detect_nx()` — `nx.json` exists.
    - `detect_turborepo()` — `turbo.json` exists.
    - `detect_lerna()` — `lerna.json` exists.
    - `detect_gradle_composite()` — `settings.gradle` with `includeBuild:`
      (string search).
    - `detect_maven_multi_module()` — `pom.xml` with `<modules>` element
      (string search or XML parse).
- [ ] Export `workspace` module from `crates/apmw-core/src/lib.rs`.
- [ ] Add unit tests:
  - Cargo workspace detected from `Cargo.toml` with `[workspace]`.
  - pnpm workspace detected from `pnpm-workspace.yaml`.
  - npm workspace detected from `package.json` with `workspaces`.
  - Yarn workspace detected from `package.json` + `yarn.lock`.
  - Nx detected from `nx.json`.
  - Turborepo detected from `turbo.json`.
  - Lerna detected from `lerna.json`.
  - Gradle composite detected from `settings.gradle` with `includeBuild`.
  - Maven multi-module detected from `pom.xml` with `<modules>`.
  - No workspace detected (returns `None`).
  - Member paths are resolved correctly for Cargo workspace
    (`workspace.members` glob expansion).
- [ ] Run `cargo test --workspace` and `just validate`.

## Relevant Files

- `crates/apmw-core/src/workspace/mod.rs` — new module
- `crates/apmw-core/src/lib.rs` — export `workspace` module

## Acceptance Criteria (Gherkin)

- Given a directory with `Cargo.toml` containing `[workspace]`, When
  `detect_workspace()` is called, Then it returns `Some(WorkspaceResult)`
  with `organizer == Cargo` and the workspace root and member paths.
- Given a directory with `pnpm-workspace.yaml`, When `detect_workspace()` is
  called, Then it returns `Some` with `organizer == Pnpm`.
- Given a directory with `nx.json`, When `detect_workspace()` is called,
  Then it returns `Some` with `organizer == Nx`.
- Given a directory with no workspace marker files, When
  `detect_workspace()` is called, Then it returns `None`.
- Given a Cargo workspace with `members = ["crates/*"]`, When
  `detect_workspace()` is called, Then the member paths are glob-expanded
  to actual directory paths.

## Test Plan

- Unit tests in `crates/apmw-core/src/workspace/mod.rs` (`#[cfg(test)]`)
- Use `tempfile` to create test directories with marker files.
- `cargo test --workspace` — all tests pass
- `just validate` — all quality gates pass

## Risks & Mitigations

- **Risk**: Glob expansion for Cargo workspace members may not match
  Cargo's actual behavior.
  **Mitigation**: Use the `glob` crate for expansion and verify against
  real workspace layouts in tests.
- **Risk**: JSON/YAML parsing for `package.json` / `pnpm-workspace.yaml`.
  **Mitigation**: Use `serde_json` (already a dependency) and the YAML crate
  added in 08-002 (or add a minimal YAML parser if 08-002 hasn't landed yet
  — coordinate via the parallel-safe flag).

## Dependencies & Sequencing

- **Depends on**: 08-001 (workspace structure must exist).
- **Unblocks**: 08-005 (publish).
- **Parallel-safe with**: 08-002, 08-004 (different modules, no shared files).

## Definition of Done

- `workspace` module exists in `crates/apmw-core/src/workspace/`
- All 9 workspace organizers are detected
- Member paths are resolved for Cargo workspaces
- All unit tests pass
- `just validate` passes
