---
story_id: "08-002"
story_title: "Add YAML custom project types"
story_name: "yaml-custom-types"
prd_name: "extract-apmw-core"
prd_file: "internal-docs/feature/todo/extract-apmw-core/feat-202609021309-extract-apmw-core.md"
phase: 8
parallel_id: 2
branch: "feature/current/extract-apmw-core/story-08-002-yaml-custom-types"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["08-001"]
parallel_safe: true
modules: ["crates/apmw-core/src/custom_types/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "yaml", "extensibility"]
due: ""
create-date: "2026-09-02"
update-date: "2026-09-02"
---

## Summary

Add a `custom_types` module to `apmw-core` that loads custom `ProjectType`
definitions from YAML config files, merges them with the built-in
`PackageManager` definitions, and allows custom types to override built-in
types with the same name. Config files are loaded from
`~/.config/apmw/project-types.yml` (user-wide) and
`.apmw/project-types.yml` (per-project), with per-project taking precedence.

## Sub-Tasks

- [ ] Add `serde_yaml` dependency to `crates/apmw-core/Cargo.toml` (or use
  `serde` with a YAML deserializer — check what's already available; the
  project uses `toml` already, may need to add a YAML crate).
- [ ] Create `crates/apmw-core/src/custom_types/mod.rs` with:
  - `CustomProjectType` struct (serde-deserializable from YAML):
    `name`, `display_name`, `ecosystem`, `hierarchy`, `primary_files`
    (glob patterns), `secondary_files` (glob patterns), `dir_markers`,
    `priority` (i32).
  - `CustomTypesConfig` struct: `types: Vec<CustomProjectType>`.
  - `load_custom_types()` function that reads from
    `~/.config/apmw/project-types.yml` and `.apmw/project-types.yml`,
    merges them (per-project overrides user-wide on name conflict), and
    returns a `Vec<CustomProjectType>`.
  - `merge_with_builtins()` function that takes custom types and the
    built-in `PackageManager` list, and produces a merged list where
    custom types with the same name as a built-in replace the built-in.
- [ ] Add a `from_custom()` constructor on `PackageManager` (or a conversion
  from `CustomProjectType` to `PackageManager`) so custom types can be used
  by the detection engine.
- [ ] Update the detection engine (`detect/mod.rs`) to accept an optional
  list of custom types and include them in the detection candidates.
- [ ] Add unit tests:
  - Loading a YAML config with one custom type.
  - Loading a YAML config with multiple custom types.
  - Merging custom types with built-in types (no conflicts).
  - Overriding a built-in type with a custom type of the same name.
  - Loading from both user-wide and per-project paths (per-project wins).
  - Missing config files (returns empty vec, no error).
  - Malformed YAML (returns error, does not panic).
- [ ] Run `cargo test --workspace` and `just validate`.

## Relevant Files

- `crates/apmw-core/Cargo.toml` — add YAML deserializer dependency
- `crates/apmw-core/src/custom_types/mod.rs` — new module
- `crates/apmw-core/src/lib.rs` — export `custom_types` module
- `crates/apmw-core/src/detect/mod.rs` — accept custom types in detection
- `crates/apmw-core/src/detect/managers.rs` — `PackageManager::from_custom()`

## Acceptance Criteria (Gherkin)

- Given a YAML config at `~/.config/apmw/project-types.yml` with a custom
  type `wgsl`, When `load_custom_types()` is called, Then it returns a vec
  containing the `wgsl` type.
- Given both `~/.config/apmw/project-types.yml` and
  `.apmw/project-types.yml` define a type with the same name, When
  `load_custom_types()` is called, Then the per-project version is used.
- Given a custom type with the same name as a built-in `PackageManager`,
  When `merge_with_builtins()` is called, Then the custom type replaces the
  built-in.
- Given no config files exist, When `load_custom_types()` is called, Then it
  returns an empty vec without error.
- Given a malformed YAML file, When `load_custom_types()` is called, Then it
  returns an error (does not panic).

## Test Plan

- Unit tests in `crates/apmw-core/src/custom_types/mod.rs` (`#[cfg(test)]`)
- `cargo test --workspace` — all tests pass
- `just validate` — all quality gates pass

## Risks & Mitigations

- **Risk**: Adding a YAML crate increases compile time.
  **Mitigation**: Use `serde_yaml` (lightweight) or check if an existing dep
  already provides YAML parsing.
- **Risk**: Glob pattern matching for `primary_files` / `secondary_files`.
  **Mitigation**: Use the `glob` crate if not already a dependency, or
  implement simple wildcard matching.

## Dependencies & Sequencing

- **Depends on**: 08-001 (workspace structure must exist).
- **Unblocks**: 08-005 (publish).
- **Parallel-safe with**: 08-003, 08-004 (different modules, no shared files).

## Definition of Done

- `custom_types` module exists in `crates/apmw-core/src/custom_types/`
- YAML config loading from both paths works
- Custom types can override built-in types
- All unit tests pass
- `just validate` passes
