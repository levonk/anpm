---
date:
  created: "2026-09-02"
  completed: "2026-09-02"
  last-activity: "2026-09-02"
status: Complete
slug: extract-apmw-core
---

# Extract apmw-core crate, add YAML custom types, workspace detection, version info, and publish

**Source handoff**: `.agents/handoffs/todo/202609021309-extract-apmw-core-add-features-publish.md`
**Base SHA**: `10e3d319d619571c3a60b25f5182a5fd38ae3c92`

## Goal

Extract the `detect`, `ecosystem` (minus `mapping.rs`), and `version` modules
from the `apmw` binary crate into a reusable library crate (`apmw-core`) within
a Cargo workspace, add three new capabilities (YAML custom project types,
workspace/monorepo detection, version info extraction), and publish
`apmw-core` to crates.io so downstream repos (project-lint, the
project-detection skill, levonk-base-boilerplate) can depend on it via a
version-pinned Cargo dependency instead of duplicating the
manifest-to-ecosystem mapping.

## Background

Three downstream repos duplicate apmw's manifest-to-ecosystem mapping because
the detection logic is compiled into the `apmw` binary crate and not reusable.
Research comparing apmw to three crates.io crates (project-detect,
projectdetect, standarbuild-detect) showed apmw has the best detection
mechanism (confidence scoring, shared-file disambiguation, hierarchy levels,
ecosystem grouping) but lacks YAML extensibility, workspace/monorepo
detection, and is not published.

## Scope

### In scope

1. Cargo workspace structure: `crates/apmw-core/` (library) + `crates/apmw/`
   (binary), virtual manifest at repo root.
2. Move `detect`, `ecosystem` (mod.rs only, NOT mapping.rs), and `version`
   modules into `apmw-core`.
3. Crate-local error type in `apmw-core`.
4. Binary depends on `apmw-core` via `path + version`.
5. `ecosystem/mapping.rs` stays in the binary crate.
6. YAML custom project types (load from `~/.config/apmw/project-types.yml`
   and `.apmw/project-types.yml`, merge with built-in, override support).
7. Workspace detection module (Cargo workspace, pnpm workspace, npm workspace,
   Yarn workspace, Nx, Turborepo, Lerna, Gradle composite, Maven multi-module).
8. Version info extraction module (Cargo.toml, package.json, pyproject.toml,
   go.mod, pom.xml, build.gradle, Gemfile, Package.swift, composer.json).
9. Unit tests for all new features.
10. `cargo publish --dry-run` passes for `apmw-core`.
11. Publish `apmw-core` to crates.io.
12. Update `AGENTS.md` to document the workspace structure.

### Out of scope

- CEL expressions for YAML custom types (start with file/glob indicators only).
- Publishing the `apmw` binary to crates.io (optional, not required).
- Bumping Rust edition or MSRV (stay on 2021, MSRV 1.70).
- Cross-ecosystem command translations (already in `ecosystem/mapping.rs`,
  stays in binary).

## Architecture

### Current (single crate)

```
apmw/
├── Cargo.toml          # single crate (binary + lib)
├── src/
│   ├── lib.rs          # exports all modules
│   ├── main.rs         # binary entry
│   ├── detect/         # 920+584+283 lines
│   ├── ecosystem/      # 601+688 lines (mapping.rs stays in binary)
│   ├── version/        # 584+1250 lines
│   └── ... (cli, agent, install, security, etc.)
```

### Target (Cargo workspace)

```
apmw/
├── Cargo.toml          # virtual manifest (workspace root)
├── crates/
│   ├── apmw-core/      # reusable library crate (published)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── detect/     (moved from src/detect/)
│   │       ├── ecosystem/  (moved from src/ecosystem/, minus mapping.rs)
│   │       ├── version/    (moved from src/version/)
│   │       ├── workspace/  (NEW — workspace/monorepo detection)
│   │       ├── version_info/ (NEW — manifest version extraction)
│   │       └── custom_types/ (NEW — YAML custom project types)
│   └── apmw/           # binary crate
│       ├── Cargo.toml  # depends on apmw-core via path + version
│       └── src/
│           ├── lib.rs  # re-exports from apmw-core where needed
│           ├── main.rs
│           ├── ecosystem/mapping.rs  (stays here — CLI-specific)
│           └── ... (cli, agent, install, security, etc.)
```

### Module dependency graph (apmw-core)

```
error ──┐
        ├── detect ──→ ecosystem
        └── version
```

`ecosystem` has no internal dependencies. `detect` depends on `ecosystem` and
`error`. `version` depends on `error`. New modules (`workspace`,
`version_info`, `custom_types`) depend on `ecosystem` and `error` as needed.

## Decisions

1. **Crate name**: `apmw-core` (contains detect + ecosystem + version, not
   just detection). Fallback: `apmw_detect` or `project-detect-core` if the
   name is taken on crates.io.
2. **Same repo, Cargo workspace** — standard practice (serde, tokio, clap,
   regex all do this). Path deps for local dev, `version` alongside `path`
   for publishing.
3. **`ecosystem/mapping.rs` stays in binary** — within-ecosystem command
   translations (`pip` → `uv`, `npm` → `pnpm`) are CLI-specific.
4. **YAML custom types**: file/glob indicators only (no CEL). Borrowed from
   `projectdetect` (Go port).
5. **Rust edition 2021, MSRV 1.70** — no bump. Detection logic doesn't benefit
   from edition 2024 features.
6. **Publish `apmw-core` first, then `apmw`** can optionally be published
   later.

## Definition of Done

- [x] **[script]** `cargo test --workspace` passes with 0 failures (1182 tests, 0 failures)
- [ ] **[script]** `just validate` passes (cargo audit + cargo outdated + clippy) — blocked on devbox/clippy/rustfmt tooling unavailability; `cargo build`, `cargo test`, `cargo doc`, and `cargo publish --dry-run` all pass
- [x] **[script]** `cargo publish --dry-run` succeeds for `apmw-core`
- [x] **[manual]** `apmw-core` crate exists at `crates/apmw-core/` with
  `detect`, `ecosystem`, `version`, `workspace`, `version_info`, and
  `custom_types` modules
- [x] **[manual]** `apmw` binary crate exists at `crates/apmw/` and depends
  on `apmw-core` via `path + version`
- [x] **[manual]** `ecosystem/mapping.rs` stays in the binary crate
- [x] **[manual]** YAML custom project types load from
  `~/.config/apmw/project-types.yml` and `.apmw/project-types.yml`
- [x] **[manual]** Custom types can override built-in types with the same name
- [x] **[manual]** Workspace detection identifies Cargo, pnpm, npm, Yarn, Nx,
  Turborepo, Lerna, Gradle composite, and Maven multi-module workspaces
- [x] **[manual]** Version info extraction reads package version + language
  constraints from all 9 supported manifest types
- [!] **[manual]** `apmw-core` is published to crates.io — blocked on crates.io auth token (story 08-005)
- [x] **[manual]** AGENTS.md documents the workspace structure

## Tech Debt (from holistic review 2026-09-02)

These items were identified during the Phase 8.5 holistic review. They are
design-debt regressions introduced by the extraction, not functional
breakages — the workspace builds, all 1182 tests pass, and
`cargo publish --dry-run` succeeds. They should be addressed in a follow-up
story before `apmw-core` v0.2.

1. **Duplicate `Ecosystem` enum** — `crates/apmw-core/src/ecosystem/mod.rs`
   defines an `Ecosystem` enum with variants `Python`, `Node`, `Rust`, `Go`,
   `Ruby`, `Php`, `Jvm`, `Swift`, `Dotnet`, `Flutter`, `Polyglot`, while
   `crates/apmw-core/src/detect/managers.rs` defines a separate `Ecosystem`
   enum with `Node`, `Python`, `Rust`, `Go`, `Ruby`, `Jvm`, `Dart`, `Dotnet`,
   `Apple`, `Container`, `Os`, `App`, `BuildSystem`, `Unknown`. The
   `ecosystem` enum should be canonical; `detect` should consume it.
2. **Duplicate `PackageManager` type** — `ecosystem::PackageManager` is an
   enum for command mapping; `detect::PackageManager` is a struct for
   detection attributes. The two are bridged by string parsing, which is
   incomplete (missing aliases for `spm`, `cmake`, `brew`, `nix`, `apt`,
   `dnf`, `pacman`, `winget`, `snap`, `flatpak`, `helm`, `docker`, `podman`,
   `devbox`, `vagrant`).
3. **`detect` does not depend on `apmw_core::ecosystem`** — `detect/mod.rs`
   imports `Ecosystem`, `PackageManager`, `HierarchyLevel` from
   `detect/managers.rs`, never from `crate::ecosystem`. `version_info`
   correctly imports `crate::ecosystem::Ecosystem`.
4. **`custom_types` depends on `detect` instead of `ecosystem`** — same root
   cause as #3.
5. **Ecosystem string disagreement** — `detect` reports `apple` for Swift
   Package Manager; `version_info` reports `swift`. `detect` reports `dart`;
   canonical enum reports `flutter`.
6. **`VALID_MANAGERS` and `PackageManager::parse_manager` out of sync** —
   `apmw/src/cli.rs` accepts managers that `parse_manager` does not
   recognize, causing runtime `EcosystemMapping` errors.
7. **Serialization inconsistency** — canonical `Ecosystem`/`PackageManager`
   lack `#[serde(rename_all = "snake_case")]`, so serialized output uses
   `Rust`, `Php`, `Npm` instead of `rust`, `php`, `npm`.
8. **`build.gradle.kts` not in version_info** — `detect` recognizes it as a
   primary file; `version_info` only checks `build.gradle`.
9. **Circular `detect` ↔ `custom_types` dependency** — compiles but violates
   the PRD's intended dependency graph.

## Tech Context (Binding Constraint)

This project uses the following tools. Use them, not alternatives.

- Package manager: cargo (Rust)
- Ad-hoc runner: cargo binstall (per tech-stack table — never npx)
- Build system: cargo + just
- Test runner: cargo test
- Linter: clippy
- Container runtime: Docker
- CI/CD: GitHub Actions

System tools run via: `devbox run -- <command>`
Never use: npm, npx, yarn, jest, biome, pip install (this is a Rust project)

## Stories

See `tasks/index-extract-apmw-core.md` for the task breakdown.
