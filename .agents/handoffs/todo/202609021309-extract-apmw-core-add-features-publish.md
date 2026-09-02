---
date:
  created: "2026-09-02"
  completed: ""
  last-activity: "2026-09-02"
---

# Extract detection + ecosystem + version modules into a reusable crate, add YAML custom types, workspace detection, and publish

**Date**: 2026-09-02
**Session**: Post-research comparing apmw's detection to three crates.io crates (project-detect, projectdetect, standarbuild-detect). apmw has the best detection mechanism (confidence scoring, shared-file disambiguation, hierarchy levels, ecosystem grouping) but lacks YAML extensibility, workspace/monorepo detection, and is not published. Three downstream repos (project-lint, skills-src/project-detection skill, levonk-base-boilerplate) duplicate the same manifest-to-ecosystem mapping.
**Status**: In progress — handoff created, awaiting execute-upsert execution.

## Current State

### Completed
- **Research complete** on the three existing crates.io crates and their capabilities vs apmw.
- **apmw detection module analyzed** — 33 package managers, confidence scoring, hierarchy levels, ecosystem grouping, shared-file priority disambiguation.
- **apmw version module analyzed** — lockfile pinning, engine constraints, latest/latest-minor resolution, supply-chain min-age-days defense, registry client trait.
- **Cargo workspace best practices researched** — path dependencies for local dev, version + path for publishing, workspace inheritance for shared metadata.

### Blocking Issues
1. **apmw's detection module is not reusable** — it's compiled into the `apmw` binary crate. project-lint, the project-detection bash skill, and the boilerplate Copier templates all duplicate the manifest-to-ecosystem mapping independently.
2. **apmw lacks YAML custom project types** — adding a new package manager requires editing `managers.rs` and recompiling. projectdetect (the Go port) supports YAML config files for custom types.
3. **apmw lacks workspace/monorepo detection** — it detects individual package managers but not workspace organizers (Cargo workspace, pnpm workspace, Nx, Turborepo, Lerna). standarbuild-detect has this.
4. **apmw is not published to crates.io** — downstream repos can't depend on it without a git dependency, which is fragile for version pinning.

## Git State

**Commit at handoff**: `10e3d319d619571c3a60b25f5182a5fd38ae3c92` (captured via `git rev-parse HEAD`)

## Required Reading

Before any other action, read `/Users/micro/p/gh/levonk/apmw/AGENTS.md` — it is the root of this project's progressively-disclosed informational files. Follow its Usage Protocol and re-read the chain for any path you touch.

## Project Overview

### Objective

1. **Extract `detect`, `ecosystem`, and `version` modules into a standalone crate** (`apmw-core` or similar) within the same repo using a Cargo workspace.
2. **Add YAML custom project types** — users can define new package managers/project types in a YAML config file without recompiling.
3. **Add workspace/monorepo detection** — detect workspace organizers (Cargo workspace, pnpm workspace, Nx, Turborepo, Lerna) in addition to individual package managers.
4. **Add version information extraction** — read version constraints from manifest files (Cargo.toml `version`, package.json `version`, pyproject.toml `version`, go.mod `go` directive, etc.) and expose them in detection results.
5. **Publish the crate to crates.io** so downstream repos (project-lint, etc.) can depend on it via version-pinned Cargo dependency.

### Crate packaging answer: same repo, Cargo workspace

**Standard practice is to keep the library crate in the same repo as the binary, using a Cargo workspace.** This is what `serde`, `tokio`, `clap`, `regex`, and most major Rust projects do. The pattern:

```
apmw/                          # repo root
├── Cargo.toml                 # workspace root (virtual manifest)
├── crates/
│   ├── apmw-core/             # the reusable library crate (detection + ecosystem + version)
│   │   ├── Cargo.toml         # package metadata, published to crates.io
│   │   └── src/
│   └── apmw/                  # the binary crate (CLI, agent, install, security, etc.)
│       ├── Cargo.toml         # depends on apmw-core via path + version
│       └── src/
```

The workspace root `Cargo.toml` is a virtual manifest (`[workspace]` section, no `[package]`). Member crates inherit shared metadata via `[workspace.dependencies]` and `[workspace.lints]`.

**Why same repo, not separate repo:**
- The binary and library are developed in tandem — changes to detection logic need to be tested against the CLI immediately.
- Cargo workspaces share a single `Cargo.lock` and `target/` directory, so builds are fast and dependency versions are unified.
- Path dependencies work for local dev; `version = "0.1.0"` alongside `path = "../apmw-core"` makes it publishable.
- A separate repo would require git dependencies for local dev, which are slower and more fragile than path dependencies.

**Why not keep it as a single crate:**
- crates.io doesn't allow publishing crates with path dependencies on code outside crates.io (except dev-dependencies). If `apmw` (binary) depends on `apmw-core` (library) via path, `apmw-core` must be published first, then `apmw` can be published with `apmw-core = { path = "../apmw-core", version = "0.1.0" }`.
- Downstream repos (project-lint) only need the detection/ecosystem/version logic, not the CLI/agent/install/security code. A separate crate lets them depend on just what they need.
- The library crate has a different release cadence than the binary — detection logic is stable and changes infrequently; the CLI changes more often.

### Current Status

apmw is a single crate with `src/lib.rs` exporting all modules and `src/main.rs` as the binary. The `detect`, `ecosystem`, and `version` modules are self-contained (no dependencies on CLI, agent, install, or security modules), which makes extraction straightforward.

Module sizes (lines of code):
- `src/detect/mod.rs` — 920 lines (detection engine, confidence scoring)
- `src/detect/managers.rs` — 584 lines (33 PackageManager definitions)
- `src/detect/attributes.rs` — 283 lines (evidence collection)
- `src/ecosystem/mod.rs` — 601 lines (ecosystem definitions, canonical managers)
- `src/ecosystem/mapping.rs` — 688 lines (within-ecosystem command translations)
- `src/version/mod.rs` — 584 lines (resolution strategy, supply-chain defense)
- `src/version/resolver.rs` — 1250 lines (lockfile parsing, registry client, engine constraints)
- **Total: ~4,910 lines to extract**

## Key Decisions Made

### Crate name: `apmw-core`

The crate is named `apmw-core` (not `apmw-detect`) because it contains three modules (detect, ecosystem, version), not just detection. The binary stays as `apmw`. If the name `apmw-core` is taken on crates.io, fall back to `apmw_detect` or `project-detect-core`.

### What goes into `apmw-core`

| Module | Goes in | Reason |
|--------|---------|--------|
| `detect` | Yes | Core detection logic — reusable by project-lint, project-detection skill |
| `ecosystem` | Yes | Ecosystem grouping + canonical managers — needed by detect and by downstream Dependabot mapping |
| `version` | Yes | Version resolution + lockfile parsing + supply-chain defense — reusable by project-lint's dependency_checker scanner |
| `ecosystem/mapping.rs` | **No** — stays in binary | Within-ecosystem command translations (`pip` → `uv`, `npm` → `pnpm`) are CLI-specific, not needed by downstream |
| `agent` | No — stays in binary | CLI/agent integration |
| `install` | No — stays in binary | Install-on-use logic |
| `security` | No — stays in binary | Security scanning |
| `cli` | No — stays in binary | CLI parsing |
| `containers` | No — stays in binary | Container management |
| `config` | No — stays in binary | User config |
| `clone` | No — stays in binary | Repo cloning |
| `daemon` | No — stays in binary | Daemon mode |
| `governance` | No — stays in binary | Governance |
| `output` | No — stays in binary | Output formatting |
| `path_scan` | No — stays in binary | Path scanning |
| `telemetry` | No — stays in binary | Telemetry |

### YAML custom project types

Borrow the design from `projectdetect` (the Go port): a YAML config file that defines custom `ProjectType` entries with `Indicator`s. The YAML schema:

```yaml
# ~/.config/apmw/project-types.yml
types:
  - name: wgsl
    display_name: "WGSL Shader"
    ecosystem: custom
    hierarchy: language
    primary_files:
      - "*.wgsl"
    secondary_files:
      - "shaders.toml"
    dir_markers:
      - "shaders/"
    priority: 50
```

The crate loads custom types from:
1. `~/.config/apmw/project-types.yml` (user-wide)
2. `.apmw/project-types.yml` (per-project)
3. Custom path via `--types-file` CLI flag

Custom types are merged with built-in types. Built-in types can be overridden by custom types with the same name.

### Workspace/monorepo detection

Add a `workspace` module to `apmw-core` that detects workspace organizers:

| Organizer | Detection file(s) |
|-----------|-------------------|
| Cargo workspace | `Cargo.toml` with `[workspace]` section |
| pnpm workspace | `pnpm-workspace.yaml` |
| npm workspace | `package.json` with `workspaces` field |
| Yarn workspace | `package.json` with `workspaces` field + `yarn.lock` |
| Nx | `nx.json` |
| Turborepo | `turbo.json` |
| Lerna | `lerna.json` |
| Gradle composite | `settings.gradle` with `includeBuild:` |
| Maven multi-module | `pom.xml` with `<modules>` |

The detection returns a `WorkspaceResult` with the organizer type, the workspace root, and member project paths. This is inspired by `standarbuild-detect`'s `discover()` but integrated with apmw's existing detection model (confidence scoring, ecosystem grouping).

### Version information extraction

Add a `version_info` module to `apmw-core` that reads version constraints from manifest files:

| Manifest | Version field | What to extract |
|----------|--------------|-----------------|
| `Cargo.toml` | `version`, `rust-version` | Package version + MSRV |
| `package.json` | `version`, `engines` | Package version + engine constraints |
| `pyproject.toml` | `version`, `requires-python` | Package version + Python version constraint |
| `go.mod` | `go` directive | Go version |
| `pom.xml` | `<version>`, `<maven.compiler.source>` | Artifact version + Java version |
| `build.gradle` | `version`, `sourceCompatibility` | Project version + Java version |
| `Gemfile` / `.gemspec` | `version`, `required_ruby_version` | Gem version + Ruby version |
| `Package.swift` | `version`, `platforms` | Package version + platform constraints |
| `composer.json` | `version`, `require.php` | Package version + PHP version |

The extraction returns a `VersionInfo` struct with the package version, language/runtime version constraints, and the source file. This is useful for:
- project-lint's `dependency_version_checker` scanner (currently reimplements lockfile parsing)
- Dependabot config generation (knowing which ecosystems need version entries)
- The `apmw` CLI's version resolution engine (already has this, but the extraction makes it available to downstream)

### Publishing

Publish `apmw-core` to crates.io. The binary `apmw` depends on it via `path + version`:

```toml
# crates/apmw/Cargo.toml
[dependencies]
apmw-core = { path = "../apmw-core", version = "0.1.0" }
```

Downstream repos depend on it via version:

```toml
# project-lint's Cargo.toml
[dependencies]
apmw-core = "0.1"
```

Publish `apmw-core` first, then `apmw` (the binary). Use `cargo publish --dry-run` to verify before publishing.

### Rust version

Stay on Rust edition 2021, MSRV 1.70. Latest stable is 1.98.0 (August 2026) but there's no compelling reason to bump — the detection logic doesn't benefit from edition 2024 features. Bumping MSRV would force downstream consumers to update their toolchains for no functional gain.

## Technical Context

### Stack/Tools
- Rust 2021 Edition, MSRV 1.70
- `serde` for serialization
- `tracing` for logging
- `thiserror` for errors
- `tempfile` + `assert_fs` for tests
- Cargo workspace for multi-crate structure

### Current module dependencies

The `detect` module depends on:
- `ecosystem` (for `Ecosystem` enum and `PackageManager` ecosystem field)
- `error` (for `ApmwError`, `Result`)

The `version` module depends on:
- `error` (for `ApmwError`, `Result`)

The `ecosystem` module has no internal dependencies.

**Extraction plan**: Create a new `error` module inside `apmw-core` (or use `thiserror` directly with a crate-specific error type). The `detect` and `version` modules switch from `crate::error::ApmwError` to `apmw_core::error::CoreError`.

### Important Files
- `Cargo.toml` — current single-crate manifest, will become workspace root
- `src/lib.rs` — current lib entry, will be split
- `src/detect/mod.rs` — detection engine (920 lines)
- `src/detect/managers.rs` — 33 PackageManager definitions (584 lines)
- `src/detect/attributes.rs` — evidence collection (283 lines)
- `src/ecosystem/mod.rs` — ecosystem definitions (601 lines)
- `src/ecosystem/mapping.rs` — command translations (688 lines, stays in binary)
- `src/version/mod.rs` — version resolution strategy (584 lines)
- `src/version/resolver.rs` — lockfile parsing, registry client (1250 lines)
- `src/error.rs` — error types (needs to be split between core and binary)

### Environment Notes
- Run tests with `cargo test --workspace`
- Run quality checks with `just validate` (runs cargo audit + cargo outdated + clippy)
- The repo uses devbox — run commands through `devbox run --`

## Next Steps (Priority Order)

1. Create the Cargo workspace structure: `crates/apmw-core/` and `crates/apmw/`.
2. Move `detect`, `ecosystem` (minus `mapping.rs`), and `version` modules into `crates/apmw-core/src/`.
3. Create a crate-local error type in `apmw-core` (or re-export from a shared error module).
4. Update `crates/apmw/Cargo.toml` to depend on `apmw-core` via `path + version`.
5. Update the binary's `src/lib.rs` to re-export from `apmw-core` where needed.
6. Add YAML custom project types support (config file loading, merging with built-in types).
7. Add workspace/monorepo detection module.
8. Add version information extraction module.
9. Add unit tests for all new features.
10. Run `cargo test --workspace` and `just validate`.
11. Run `cargo publish --dry-run` for `apmw-core`.
12. Publish `apmw-core` to crates.io.
13. Update `apmw` binary's `Cargo.toml` with the published version.
14. Publish `apmw` binary (optional — the binary doesn't need to be on crates.io if it's installed via other means).

## Task List

**Mark legend:**
- `[ ]` — task pending
- `[~]` — task in progress
- `[x]` — task done (verified complete)
- `[!]` — task blocked (note the blocker inline)

```markdown
- [ ] Create Cargo workspace structure: root Cargo.toml (virtual manifest), crates/apmw-core/, crates/apmw/
- [ ] Move detect module (mod.rs, managers.rs, attributes.rs) into crates/apmw-core/src/detect/
- [ ] Move ecosystem module (mod.rs only, NOT mapping.rs) into crates/apmw-core/src/ecosystem/
- [ ] Move version module (mod.rs, resolver.rs) into crates/apmw-core/src/version/
- [ ] Create crate-local error type in apmw-core (CoreError or re-export thiserror)
- [ ] Update crates/apmw/Cargo.toml to depend on apmw-core via { path = "../apmw-core", version = "0.1.0" }
- [ ] Update binary src/lib.rs to re-export from apmw-core where needed
- [ ] Keep ecosystem/mapping.rs in the binary crate (CLI-specific command translations)
- [ ] Add YAML custom project types: config file loading (~/.config/apmw/project-types.yml + .apmw/project-types.yml), merging with built-in types, override support
- [ ] Add workspace detection module: Cargo workspace, pnpm workspace, npm workspace, Yarn workspace, Nx, Turborepo, Lerna, Gradle composite, Maven multi-module
- [ ] Add version info extraction module: read version + language constraints from Cargo.toml, package.json, pyproject.toml, go.mod, pom.xml, build.gradle, Gemfile, Package.swift, composer.json
- [ ] Add unit tests for YAML custom types (loading, merging, overriding built-in)
- [ ] Add unit tests for workspace detection (each organizer type, nested workspaces, no workspace)
- [ ] Add unit tests for version info extraction (each manifest type, missing fields, malformed files)
- [ ] Add apmw-core public API docs (crate-level rustdoc, module-level docs, examples)
- [ ] Run cargo test --workspace and just validate to verify
- [ ] Run cargo publish --dry-run for apmw-core and fix any packaging issues
- [ ] Publish apmw-core to crates.io
- [ ] Update crates/apmw/Cargo.toml with published apmw-core version
- [ ] Update AGENTS.md to document the workspace structure
```

**Maintenance protocol (receiving session):**
1. Verify in-progress marks before starting.
2. Defer to execute-upsert for execution.
3. Mark done only when verified.
4. Record blockers inline.
5. Update the list as work reveals new tasks.

## Definition of Done

- [ ] **[script]** `cargo test --workspace` passes with 0 failures
- [ ] **[script]** `just validate` passes (cargo audit + cargo outdated + clippy)
- [ ] **[script]** `cargo publish --dry-run` succeeds for `apmw-core` (no errors, no warnings)
- [ ] **[manual]** `apmw-core` crate exists at `crates/apmw-core/` with `detect`, `ecosystem`, `version`, `workspace`, `version_info`, and `custom_types` modules
- [ ] **[manual]** `apmw` binary crate exists at `crates/apmw/` and depends on `apmw-core` via `path + version`
- [ ] **[manual]** `ecosystem/mapping.rs` stays in the binary crate (not in `apmw-core`)
- [ ] **[manual]** YAML custom project types load from `~/.config/apmw/project-types.yml` and `.apmw/project-types.yml`
- [ ] **[manual]** Custom types can override built-in types with the same name
- [ ] **[manual]** Workspace detection identifies Cargo, pnpm, npm, Yarn, Nx, Turborepo, Lerna, Gradle composite, and Maven multi-module workspaces
- [ ] **[manual]** Version info extraction reads package version + language constraints from all 9 supported manifest types
- [ ] **[manual]** `apmw-core` is published to crates.io and downloadable via `cargo add apmw-core`
- [ ] **[manual]** AGENTS.md documents the workspace structure and the crate split

**Not Done (common false-completion signals):**
- Tests pass but `cargo publish --dry-run` fails (path dependency not allowed in published crates — need `version` alongside `path`)
- YAML custom types load but can't override built-in types
- Workspace detection works for Cargo but not for Nx/Turborepo (these need JSON parsing, not just file existence)
- Version info extraction reads package version but not language constraints (e.g., reads `Cargo.toml` `version` but not `rust-version`)
- `apmw-core` published but `apmw` binary can't compile because it still references `crate::detect` instead of `apmw_core::detect`

## Execution Plan

Every task below is executed via the `execute-upsert` skill.

| Story slug | Type | Base SHA | DoD |
|------------|------|----------|-----|
| extract-apmw-core | standard | 10e3d31 | Workspace structure created, detect+ecosystem+version extracted, binary compiles, tests pass |
| add-yaml-custom-types | standard | (after extract) | YAML config loading, merging, override, tests pass |
| add-workspace-detection | standard | (after extract) | 9 workspace organizers detected, tests pass |
| add-version-info-extraction | standard | (after extract) | 9 manifest types parsed, tests pass |
| publish-apmw-core | standard | (after all features) | cargo publish --dry-run passes, published to crates.io |

## Open Questions

1. Should `apmw-core` include the `ecosystem/mapping.rs` (command translations) or keep it in the binary? (Recommendation: keep in binary — command translations are CLI-specific, downstream repos don't need them.)
2. Should the YAML custom types support CEL expressions (like projectdetect) or just file/glob indicators? (Recommendation: start with file/glob indicators only — CEL adds a heavy dependency and complexity. Add CEL later if needed.)
3. Should `apmw-core` publish as `apmw-core` or `apmw_detect`? (Recommendation: `apmw-core` — it contains more than just detection, and the `apmw` brand is consistent. Check crates.io for name availability first.)
4. Should version info extraction use `toml` crate for Cargo.toml/pyproject.toml parsing or stick with the existing line-based parser? (Recommendation: use `toml` crate — it's already a dependency via `standarbuild-detect` pattern and is more robust than line-based parsing. The existing `version/resolver.rs` already uses some parsing; consolidate on `toml`.)

## Do Not

- Do not bump the Rust edition or MSRV — stay on 2021/1.70
- Do not move `ecosystem/mapping.rs` into `apmw-core` — it's CLI-specific
- Do not add CEL expression support in the first version of YAML custom types — start simple
- Do not publish the `apmw` binary to crates.io unless explicitly requested — only `apmw-core` needs publishing for downstream use
- Do not create a separate repo for `apmw-core` — same repo, Cargo workspace is standard practice
- Do not remove the `proptest-regressions/` directory — it's test state

## Suggested Skills

- `execute-upsert` — for executing each story with worktree-per-story discipline
- `unit-test-writing` — for writing tests in Roy Osherove style
- `code-review-guidance` — for reviewing the crate extraction before publishing

## Additional Context

### Research: Existing crates.io crates

| Crate | Ecosystems | Downloads | Key advantage over apmw | Key disadvantage vs apmw |
|-------|-----------|-----------|------------------------|--------------------------|
| `project-detect` | 32 | 591 | Zero dependencies | No confidence scoring, no ecosystem grouping, no shared-file disambiguation |
| `projectdetect` | 20+ | 21 | YAML + CEL extensibility | No confidence scoring, no ecosystem grouping, flat type model |
| `standarbuild-detect` | 8+11 | 281 | Monorepo workspace detection | Fewer ecosystems, no confidence scoring, no ecosystem grouping |

None of the three has confidence scoring, hierarchy levels, or shared-file priority disambiguation. apmw's detection model is genuinely more sophisticated. The only features worth borrowing are YAML extensibility (from projectdetect) and workspace detection (from standarbuild-detect).

### Research: Cargo workspace best practices

- **Same repo, workspace structure** is standard practice (serde, tokio, clap, regex all do this)
- Virtual manifest at root (`[workspace]` only, no `[package]`)
- Member crates in `crates/` subdirectory
- Path dependencies for local dev, `path + version` for publishing
- `[workspace.dependencies]` for shared dependency versions
- `[workspace.lints]` for shared clippy/rustfmt config
- Publish library crate first, then binary crate

### Research: Rust version

Latest stable: 1.98.0 (August 20, 2026). No compelling reason to bump from 2021/1.70 — detection logic doesn't benefit from edition 2024 features. Bumping MSRV would force downstream consumers to update for no functional gain.

### Related Handoffs

- **skills-src handoff**: `202609021249-add-dependabot-config-and-update-project-adopter.md` — project-adopter should use apmw-core for ecosystem detection when generating dependabot.yml
- **project-lint handoff**: `202609021249-dependabot-scanner-modernization.md` — project-lint's dependabot scanner should use apmw-core for manifest detection instead of hardcoding mappings
- **levonk-base-boilerplate handoff**: `202609021249-add-dependabot-template-to-boilerplate.md` — Copier template should reference apmw-core's ecosystem mapping for conditional dependabot.yml generation
