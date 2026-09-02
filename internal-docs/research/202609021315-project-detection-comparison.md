---
modeline: "vim: set ft=markdown:"
title: "Project Detection System Comparison"
adr-id: "202609021315"
slug: "project-detection-comparison"
url: "https://github.com/levonk/apmw/blob/main/internal-docs/research/202609021315-project-detection-comparison.md"
synopsis: "Comparison of apmw's detection engine against three existing crates.io crates (project-detect, projectdetect, standarbuild-detect) and the skills-src project-detection bash skill. Documents why we're not using any of them, which features we're borrowing, and the plan to extract apmw's detection into a reusable published crate."
author: "https://github.com/levonk"
date-created: "2026-09-02"
date-updated: "2026-09-02"
version: "1.0.0"
status: "accepted"
tags: [doc/research, detection, ecosystem, crate-extraction]
related-to:
  - "https://github.com/levonk/apmw/blob/main/.agents/handoffs/todo/202609021309-extract-apmw-core-add-features-publish.md"
---

# Project Detection System Comparison

## Context

Three downstream repos duplicate the same manifest-to-ecosystem mapping that
apmw already implements:

- **project-lint** (`~/p/gh/levonk/project-lint`) — the `dependabot` scanner
  and various language-specific scanners each hardcode which manifest files
  to look for
- **skills-src project-detection skill**
  (`~/p/gh/levonk/skills-src/src/current/skills/software-dev/project-detection/`)
  — a bash script with a ~40-entry associative array mapping build system
  names to manifest filenames
- **levonk-base-boilerplate**
  (`~/p/gh/levonk/levonk-base-boilerplate`) — Copier templates that
  conditionally generate config based on project type, with no shared
  detection logic

This document compares apmw's detection engine against the three existing
crates.io crates that solve the same problem, documents why we're not
adopting any of them, and records which features we're borrowing.

## The Four Systems

### 1. apmw (this repo)

**Code**: [`src/detect/`](../../src/detect/), [`src/ecosystem/`](../../src/ecosystem/)

- 33 package managers with structured `PackageManager` structs
- Each manager has: canonical name, display name, ecosystem, hierarchy
  level, primary files, secondary files, directory markers, detection
  priority
- **Confidence scoring**: weighted evidence (primary 1.0, secondary 0.5,
  directory 0.3), normalized to [0.0, 1.0]
- **Shared-file disambiguation**: priority field resolves conflicts when
  multiple managers share the same config file (e.g. `pyproject.toml` is
  used by poetry, pdm, and uv; uv wins as canonical runner)
- **Ecosystem grouping**: `Ecosystem` enum (Node, Python, Rust, Go, Jvm,
  Swift, Dotnet, Flutter, Polyglot) with canonical manager per ecosystem
- **Hierarchy levels**: OsWrapper, Os, Virtualization, Language, App,
  BuildSystem
- **Not published** to crates.io — downstream repos can't depend on it
- **No YAML extensibility** — adding a manager requires editing
  `managers.rs` and recompiling
- **No workspace/monorepo detection** — detects individual package
  managers but not workspace organizers (Cargo workspace, pnpm workspace,
  Nx, Turborepo, etc.)

### 2. project-detect

**Repository**: [github.com/teh33/project-detect](https://github.com/teh33/project-detect)
**crates.io**: [crates.io/crates/project-detect](https://crates.io/crates/project-detect)
**Version**: 0.1.2 (published 2026-03-26)
**Downloads**: 591 total, 3 dependents

- 32 project types as a flat `ProjectKind` enum
- Single file existence check, priority-ordered (language-specific first,
  then build systems)
- Returns at most one `ProjectKind` with metadata: label, detected_file,
  artifact_dirs
- `detect_walk` walks up the directory tree for monorepo support
- Zero dependencies beyond `serde_json`
- **No confidence scoring** — first match wins by priority order
- **No ecosystem grouping** — flat enum, no way to know that npm/yarn/pnpm
  are all Node.js
- **No shared-file disambiguation** — `pyproject.toml` matches Python
  without distinguishing poetry/pdm/uv
- **No hierarchy levels** — no concept of OS wrapper vs language-level vs
  build system
- **No extensibility** — static enum, recompile to add types
- **No workspace detection**

### 3. projectdetect

**Repository**: [github.com/richardwooding/projectdetect-rs](https://github.com/richardwooding/projectdetect-rs)
**crates.io**: [crates.io/crates/projectdetect](https://crates.io/crates/projectdetect)
**Version**: 0.1.0 (published 2026-06-16)
**Downloads**: 21 total, 0 dependents

- 20+ project types as `ProjectType` entries in a `Registry`
- **YAML extensibility** — custom types load from YAML config files
  (`~/.config/projectdetect/types.yml`, `.projectdetect/types.yml`)
- **CEL expressions** (optional `cel` feature) — custom indicators can
  use CEL expressions over the directory's files/subdirs
- `Registry::find` recursively walks a tree for project roots
- `Resolver` — caching file-to-project walk-up resolver
- `collect_build_excludes` — union of artifact directories under a tree
- A directory can match multiple types simultaneously (e.g. a Go module
  with a docker-compose.yml matches both `go` and `docker-compose`)
- **No confidence scoring** — returns all matches without ranking
- **No ecosystem grouping** — flat type names, no ecosystem concept
- **No shared-file disambiguation** — returns all matches
- **No hierarchy levels**
- **No workspace detection**
- 8 dependencies (cel, dirs, glob, ignore, once_cell, serde, serde_norway,
  thiserror) — heavier than the others

### 4. standarbuild-detect

**Repository**: [github.com/miralabs-tech/standarbuild](https://github.com/miralabs-tech/standarbuild)
**crates.io**: [crates.io/crates/standarbuild-detect](https://crates.io/crates/standarbuild-detect)
**Version**: 0.3.0 (published 2026-05-17)
**Downloads**: 281 total, 0 dependents

- 8 project types (Rust, Node, Bun, Deno, Python, Lua, C, C++) + 11
  workspace organizers (Cargo, Npm, Pnpm, Yarn, Bun, Deno, Go, Lerna, Nx,
  Turborepo, Mira)
- **Workspace/monorepo detection** — `discover()` recursively scans a
  polyglot monorepo and returns both individual projects and workspace
  manifests with member relationships
- `Detector` trait + `DetectorRegistry` — implement `Detector` for custom
  kinds, add/remove from registry at runtime
- `KindId::custom("wgsl")` — custom kinds without touching the library
- `DetectionResult { projects, workspaces }` — structured output with
  `member_of` relationships
- **No confidence scoring** — returns all matches
- **No ecosystem grouping** — flat KindId, no ecosystem concept
- **No shared-file disambiguation**
- **No hierarchy levels**
- **No YAML extensibility** — custom types require implementing the
  `Detector` trait in Rust (recompile)
- 3 dependencies (serde, serde_json, toml)

## Feature Matrix

| Feature | apmw | project-detect | projectdetect | standarbuild-detect |
|---------|:----:|:--------------:|:-------------:|:-------------------:|
| Ecosystems/managers | 33 | 32 | 20+ | 8+11 |
| Confidence scoring | Yes | No | No | No |
| Shared-file disambiguation | Yes | No | No | No |
| Ecosystem grouping | Yes | No | No | No |
| Hierarchy levels | Yes | No | No | No |
| YAML custom types | No | No | Yes | No |
| CEL expressions | No | No | Yes (opt) | No |
| Trait-based extensibility | No | No | No | Yes |
| Workspace/monorepo detection | No | No | No | Yes |
| Directory walk-up | No | Yes | Yes | Yes |
| Recursive tree scan | No | No | Yes | Yes |
| Artifact directories | No | Yes | Yes | No |
| File-to-project resolution | No | Yes | Yes | Yes (member_of) |
| Published to crates.io | No | Yes | Yes | Yes |
| Dependency count | serde, tracing | serde, serde_json | 8 deps | 3 deps |

## Why We're Not Using Any of Them

### Supply chain risk

All three crates are very new, very low-download, single-author crates:

| Crate | Age at time of research | Total downloads | Dependents |
|-------|------------------------|-----------------|------------|
| project-detect | 6 months | 591 | 3 |
| projectdetect | 3 months | 21 | 0 |
| standarbuild-detect | 4 months | 281 | 0 |

apmw is a private repo with strict security posture (SHA-pinned GitHub
Actions, zizmor in blocking CI, supply-chain min-age-days defense in the
version resolver). Pulling in an unvetted 6-month-old crate with 591
downloads is a real risk. Any of these crates could be abandoned or yanked.

### Detection quality

apmw's detection engine is genuinely more sophisticated than all three.
The confidence scoring with weighted primary/secondary/directory evidence
and the priority-based disambiguation for shared config files
(`pyproject.toml` → uv beats poetry beats pdm) solve real problems that
none of the three crates address:

- **project-detect** returns the first match by priority order. If
  `pyproject.toml` exists, it returns `Python` without knowing whether
  the project uses poetry, pdm, or uv. apmw distinguishes all three and
  ranks them by priority.

- **projectdetect** returns all matches simultaneously. A directory with
  `pyproject.toml` matches `python` (via file indicator), but there's no
  way to know which Python package manager is in use. apmw's confidence
  scoring ranks the candidates.

- **standarbuild-detect** detects 8 project types — it doesn't know about
  Elixir, Haskell, Clojure, Erlang, OCaml, Perl, Julia, R, Nim, Crystal,
  V, Gleam, Lua, or any of the 25+ ecosystems apmw covers.

### Ecosystem grouping

None of the three crates has the concept of an ecosystem. apmw groups
package managers into ecosystems (Node, Python, Rust, Go, Jvm, Swift,
Dotnet, Flutter, Polyglot) with a canonical manager per ecosystem. This
is essential for:
- Within-ecosystem command mapping (`pip` → `uv`, `npm` → `pnpm`) — never
  cross-ecosystem
- Dependabot ecosystem mapping (`Cargo.toml` → `cargo`, `package.json` →
  `npm`, `go.mod` → `gomod`)
- project-lint's language-specific scanners (knowing that a project is
  "Node ecosystem" activates the typescript, config_validation, and
  runtime_guards scanners)

### Hierarchy levels

None of the three crates models hierarchy levels. apmw distinguishes:
- OS wrappers (mise, brew, nix)
- OS-level (apt, dnf, pacman, winget)
- Virtualization (docker, podman, packer)
- Language-level (npm, cargo, pip, gem, go)
- App-level (helm, flatpak, snap, devbox, vagrant)
- Build systems (bazel, buck, pants, cmake, ant, xcode, spm)

This matters for apmw's install-on-use semantics — an OS wrapper like
`mise` can install a language-level manager like `pnpm`, but not vice
versa.

## What We're Borrowing

Despite not adopting any of the three crates, two features are worth
incorporating into apmw:

### YAML custom project types (from projectdetect)

projectdetect's YAML config system lets users define custom project types
without recompiling. The schema:

```yaml
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

We're adopting this pattern (without CEL expressions — start simple) in
the `apmw-core` extraction. Custom types load from
`~/.config/apmw/project-types.yml` and `.apmw/project-types.yml`, merge
with built-in types, and can override built-in types with the same name.

**Source**: [projectdetect YAML config](https://github.com/richardwooding/projectdetect-rs)

### Workspace/monorepo detection (from standarbuild-detect)

standarbuild-detect's `discover()` recursively scans a polyglot monorepo
and returns both individual projects and workspace organizers with
member relationships. apmw currently has no workspace detection — it
detects individual package managers but not workspace structures (Cargo
workspace, pnpm workspace, Nx, Turborepo, Lerna, etc.).

We're adding a `workspace` module to `apmw-core` that detects 9 workspace
organizers, integrated with apmw's existing detection model (confidence
scoring, ecosystem grouping).

**Source**: [standarbuild-detect workspace discovery](https://github.com/miralabs-tech/standarbuild)

## The Plan: Extract and Publish apmw-core

Rather than depending on an external crate, we're extracting apmw's
detection, ecosystem, and version modules into a standalone crate
(`apmw-core`) within the same repo using a Cargo workspace, adding the
two borrowed features (YAML custom types, workspace detection), adding
version info extraction, and publishing to crates.io.

This gives downstream repos (project-lint, the project-detection skill,
the boilerplate templates) a single, vetted, owned dependency for project
detection instead of three independent duplications.

**Handoff**: [202609021309-extract-apmw-core-add-features-publish.md](../../.agents/handoffs/todo/202609021309-extract-apmw-core-add-features-publish.md)

### Crate structure

```
apmw/
├── Cargo.toml                 # workspace root (virtual manifest)
├── crates/
│   ├── apmw-core/             # reusable library, published to crates.io
│   │   └── src/
│   │       ├── detect/        # detection engine + confidence scoring
│   │       ├── ecosystem/     # ecosystem grouping + canonical managers
│   │       ├── version/       # version resolution + lockfile parsing
│   │       ├── workspace/     # workspace/monorepo detection (new)
│   │       ├── version_info/  # version extraction from manifests (new)
│   │       └── custom_types/  # YAML custom project types (new)
│   └── apmw/                  # binary crate (CLI, agent, install, security)
│       └── src/
│           ├── ecosystem/mapping.rs  # command translations (stays in binary)
│           └── ...                   # all other CLI-specific modules
```

### Why same repo, not separate repo

Standard practice for Rust libraries with an associated binary (serde,
tokio, clap, regex all do this):
- Binary and library are developed in tandem
- Cargo workspace shares one `Cargo.lock` and `target/` directory
- Path dependencies for local dev; `path + version` for publishing
- A separate repo would require git dependencies for local dev, which are
  slower and more fragile than path dependencies

### Downstream adoption

Once `apmw-core` is published:

| Downstream | Current state | After apmw-core |
|------------|--------------|-----------------|
| project-lint | Hardcodes manifest mappings in each scanner | `apmw-core = "0.1"` in Cargo.toml, scanners call `apmw_core::detect()` |
| project-detection skill | 40-entry bash associative array | Bash script calls `apmw detect --json`, falls back to bash array if apmw not installed |
| levonk-base-boilerplate | No detection logic, Copier conditionals hardcoded | Copier template references apmw-core's ecosystem mapping for conditional generation |

## Rust Version Note

Latest stable Rust is 1.98.0 (released 2026-08-20). apmw stays on edition
2021, MSRV 1.70. No compelling reason to bump — the detection logic
doesn't benefit from edition 2024 features (`gen` blocks, `unsafe`
changes, `async fn` in traits). Bumping MSRV would force downstream
consumers to update their toolchains for no functional gain.
