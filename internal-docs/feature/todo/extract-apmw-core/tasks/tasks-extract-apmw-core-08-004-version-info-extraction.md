---
story_id: "08-004"
story_title: "Add version info extraction"
story_name: "version-info-extraction"
prd_name: "extract-apmw-core"
prd_file: "internal-docs/feature/todo/extract-apmw-core/feat-202609021309-extract-apmw-core.md"
phase: 8
parallel_id: 4
branch: "feature/current/extract-apmw-core/story-08-004-version-info-extraction"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["08-001"]
parallel_safe: true
modules: ["crates/apmw-core/src/version_info/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "version", "manifest", "extraction"]
due: ""
create-date: "2026-09-02"
update-date: "2026-09-02"
---

## Summary

Add a `version_info` module to `apmw-core` that reads version constraints
from manifest files (Cargo.toml, package.json, pyproject.toml, go.mod,
pom.xml, build.gradle, Gemfile, Package.swift, composer.json) and exposes
them in a `VersionInfo` struct with the package version, language/runtime
version constraints, and the source file.

## Sub-Tasks

- [ ] Create `crates/apmw-core/src/version_info/mod.rs` with:
  - `VersionInfo` struct: `package_version: Option<String>`,
    `language_version: Option<String>`, `source_file: PathBuf`,
    `ecosystem: Ecosystem`.
  - `extract_version_info(path: &Path) -> Vec<VersionInfo>` function that
    detects which manifest files exist in the directory and extracts
    version info from each.
  - Per-manifest extraction functions:
    - `extract_cargo_toml()` — parse `Cargo.toml`, extract `package.version`
      and `package.rust-version`.
    - `extract_package_json()` — parse `package.json`, extract `version`
      and `engines.node`.
    - `extract_pyproject_toml()` — parse `pyproject.toml`, extract
      `project.version` (or `tool.poetry.version`) and
      `project.requires-python`.
    - `extract_go_mod()` — parse `go.mod`, extract the `go` directive.
    - `extract_pom_xml()` — parse `pom.xml`, extract `<version>` and
      `<maven.compiler.source>`.
    - `extract_build_gradle()` — parse `build.gradle`, extract `version`
      and `sourceCompatibility` (string search or regex).
    - `extract_gemfile()` — parse `Gemfile` / `.gemspec`, extract
      `version` and `required_ruby_version`.
    - `extract_package_swift()` — parse `Package.swift`, extract `version`
      and `platforms`.
    - `extract_composer_json()` — parse `composer.json`, extract `version`
      and `require.php`.
- [ ] Export `version_info` module from `crates/apmw-core/src/lib.rs`.
- [ ] Add unit tests:
  - Each manifest type with valid version + language constraint.
  - Missing version field (returns `None` for package_version).
  - Missing language constraint (returns `None` for language_version).
  - Malformed manifest file (returns error, does not panic).
  - Directory with multiple manifests (returns a vec with multiple entries).
  - Directory with no manifests (returns empty vec).
- [ ] Run `cargo test --workspace` and `just validate`.

## Relevant Files

- `crates/apmw-core/src/version_info/mod.rs` — new module
- `crates/apmw-core/src/lib.rs` — export `version_info` module

## Acceptance Criteria (Gherkin)

- Given a `Cargo.toml` with `version = "0.1.0"` and
  `rust-version = "1.70"`, When `extract_version_info()` is called, Then it
  returns `VersionInfo` with `package_version == Some("0.1.0")` and
  `language_version == Some("1.70")`.
- Given a `package.json` with `"version": "1.0.0"` and
  `"engines": {"node": ">=18"}`, When `extract_version_info()` is called,
  Then it returns `VersionInfo` with the package and node versions.
- Given a `go.mod` with `go 1.21`, When `extract_version_info()` is called,
  Then it returns `VersionInfo` with `language_version == Some("1.21")`.
- Given a directory with no manifest files, When `extract_version_info()` is
  called, Then it returns an empty vec.
- Given a malformed `Cargo.toml`, When `extract_version_info()` is called,
  Then it returns an error (does not panic).

## Test Plan

- Unit tests in `crates/apmw-core/src/version_info/mod.rs` (`#[cfg(test)]`)
- Use `tempfile` to create test directories with manifest files.
- `cargo test --workspace` — all tests pass
- `just validate` — all quality gates pass

## Risks & Mitigations

- **Risk**: XML parsing for `pom.xml` adds a heavy dependency.
  **Mitigation**: Use string search / regex for `<version>` and
  `<maven.compiler.source>` instead of a full XML parser.
- **Risk**: `build.gradle` is Groovy and hard to parse.
  **Mitigation**: Use regex/string search for `version` and
  `sourceCompatibility` — not a full Groovy parser.
- **Risk**: `Package.swift` is Swift and hard to parse.
  **Mitigation**: Use regex/string search for `version` and `platforms`.

## Dependencies & Sequencing

- **Depends on**: 08-001 (workspace structure must exist).
- **Unblocks**: 08-005 (publish).
- **Parallel-safe with**: 08-002, 08-003 (different modules, no shared files).

## Definition of Done

- `version_info` module exists in `crates/apmw-core/src/version_info/`
- All 9 manifest types are parsed
- Package version + language constraints are extracted
- All unit tests pass
- `just validate` passes
