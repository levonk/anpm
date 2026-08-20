---
story_id: "07-001"
story_title: "Multi-ecosystem packaging and release pipeline"
story_name: "multi-ecosystem-release"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 7
parallel_id: 1
branch: "feature/current/apmw/story-07-001-multi-ecosystem-release"
status: "done"
assignee: ""
reviewer: ""
dependencies: ["06-001", "06-002", "06-003"]
parallel_safe: false
modules: ["packaging/", ".github/workflows/"]
priority: "MUST"
risk_level: "high"
tags: ["feat", "release", "packaging", "ci-cd"]
due: "2026-11-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the multi-ecosystem packaging and release pipeline that ships apmw as npm/pnpm, PyPI/uv, brew, nix, devbox, apt, winget, apk, deb, and rpm packages from one source. The pipeline mirrors the `levonk-packages/packaging/` generator pattern. The Rust binary is the single source; each ecosystem package wraps the pre-built binary.

## Current State

- **Relevant files and their roles:**
  - PRD FR-12 (sections FR-12.1 through FR-12.3) — Multi-ecosystem release
  - `levonk-packages/packaging/` at `~/p/gh/levonk/levonk-packages/packaging/` — Generator pattern to mirror (alpine/debian/fedora/arch/brew/mise)
  - `Dockerfile` — Multi-stage build (rust:1.75-slim to debian:bookworm-slim)
  - `.github/workflows/ci.yml` — Existing CI pipeline
- **Repository conventions:** Use devbox for builds. CI runs on GitHub Actions.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `packaging/` directory with generators for each ecosystem:
  - `packaging/npm/` — npm/pnpm package (wraps pre-built binary)
  - `packaging/pypi/` — PyPI package (wraps pre-built binary)
  - `packaging/brew/` — Homebrew formula
  - `packaging/nix/` — Nix derivation
  - `packaging/devbox/` — Devbox devbox.json
  - `packaging/apt/` — apt package (deb)
  - `packaging/winget/` — Winget manifest
  - `packaging/apk/` — Alpine apk package
  - `packaging/deb/` — Debian deb package
  - `packaging/rpm/` — RPM package
- Create GitHub Actions workflow for multi-ecosystem release
- Build Rust binary for multiple targets (x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin, x86_64-windows)
- Each ecosystem package wraps the pre-built binary for the target platform
- Handle AUR name collision: use `apmw-bin` on AUR (per PRD Open Question 2)
- Add release automation (tag-triggered, changelog generation)

**Out of scope:**
- Documentation finalization (story 07-002)
- Feature implementation (all features are complete by this phase)

## Sub-Tasks

- [x] Create `packaging/` directory structure with generators for each ecosystem
  **Verify**: `ls packaging/` → all ecosystem dirs present
- [x] Create npm/pnpm package generator (wraps pre-built binary)
  **Verify**: `packaging/npm/generate.sh` → produces valid package.json
- [x] Create PyPI package generator
  **Verify**: `packaging/pypi/generate.sh` → produces valid setup.py/pyproject.toml
- [x] Create Homebrew formula generator
  **Verify**: `packaging/brew/generate.sh` → produces valid formula
- [x] Create Nix derivation generator
  **Verify**: `packaging/nix/generate.sh` → produces valid default.nix
- [x] Create apt/deb package generator
  **Verify**: `packaging/apt/generate.sh` → produces valid deb
- [x] Create winget manifest generator
  **Verify**: `packaging/winget/generate.sh` → produces valid manifest
- [x] Create apk package generator
  **Verify**: `packaging/apk/generate.sh` → produces valid apk
- [x] Create rpm package generator
  **Verify**: `packaging/rpm/generate.sh` → produces valid rpm
- [x] Create GitHub Actions release workflow (multi-target build + package + publish)
  **Verify**: `.github/workflows/release.yml` → valid workflow
- [x] Handle AUR name collision (use apmw-bin)
  **Verify**: `packaging/aur/` → uses apmw-bin name
- [x] Add release automation (tag-triggered)
  **Verify**: Push tag → triggers release workflow
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `packaging/README.md` — Overview of all ecosystem generators
- `packaging/metadata.sh` — Shared project metadata and helper functions
- `packaging/npm/generate.sh` — npm/pnpm package generator (package.json + install.js)
- `packaging/pypi/generate.sh` — PyPI package generator (pyproject.toml + Python module)
- `packaging/brew/generate.sh` — Homebrew formula generator (apmw.rb)
- `packaging/nix/generate.sh` — Nix derivation generator (default.nix + flake.nix)
- `packaging/devbox/generate.sh` — Devbox package generator (devbox.json)
- `packaging/apt/generate.sh` — apt source package generator (debian/)
- `packaging/winget/generate.sh` — Winget manifest generator (YAML manifests)
- `packaging/apk/generate.sh` — Alpine APK package generator (APKBUILD)
- `packaging/deb/generate.sh` — Debian .deb binary package generator
- `packaging/rpm/generate.sh` — RPM spec generator (apmw.spec)
- `packaging/aur/generate.sh` — AUR package generator (PKGBUILD, uses apmw-bin name)
- `.github/workflows/release.yml` — Tag-triggered release workflow (multi-target build + package + publish)

## Acceptance Criteria

- [x] All 9+ ecosystem package generators produce valid packages
- [x] GitHub Actions release workflow builds for multiple targets
- [x] Each ecosystem package wraps the pre-built binary
- [x] AUR package uses `apmw-bin` name to avoid collision
- [x] Release is tag-triggered
- [x] `just validate` passes

## Test Plan

- Unit: Verify each generator produces valid output
- Integration: Test release workflow on a tag push
- Lint: `just lint`

## Observability

- Release events should be logged at `info` level
- Package generation failures should be logged at `error` level

## Compliance

- PRD FR-12 (multi-ecosystem release)
- levonk-packages/packaging/ generator pattern

## Risks & Mitigations

- Risk: Cross-compilation may fail for some targets — Mitigation: Start with x86_64-linux and x86_64-darwin, add more targets incrementally
- Risk: Package registries may have different requirements — Mitigation: Test each package format before publishing

## Dependencies & Sequencing

- Depends on: 06-001 (MCP), 06-002 (hooks), 06-003 (skills/docs)
- Unblocks: None (final phase)

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- Cross-compilation fails for a required target
- A package registry rejects the package format

## Maintenance Notes

- New ecosystem generators can be added by extending the packaging/ directory
- Reviewers should check that each package wraps the pre-built binary correctly
- Release workflow should be tested end-to-end before first release

## Commit Conventions

- `feat(release): add multi-ecosystem packaging and release pipeline`
