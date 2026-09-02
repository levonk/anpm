# Task Index — extract-apmw-core

**PRD**: `internal-docs/feature/todo/extract-apmw-core/feat-202609021309-extract-apmw-core.md`
**Source handoff**: `.agents/handoffs/todo/202609021309-extract-apmw-core-add-features-publish.md`

## Phase 08 — apmw-core extraction, new features, and publish

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 08-001 | Extract detect+ecosystem+version into apmw-core crate (Cargo workspace) | 08 | [x] Done |  | false | — | 08-002, 08-003, 08-004, 08-005 | crates/apmw-core/, crates/apmw/, Cargo.toml | feature/current/extract-apmw-core/story-08-001-extract-apmw-core |
| 08-002 | Add YAML custom project types | 08 | [ ] Todo |  | true | 08-001 | 08-005 | crates/apmw-core/src/custom_types/ | feature/current/extract-apmw-core/story-08-002-yaml-custom-types |
| 08-003 | Add workspace/monorepo detection | 08 | [ ] Todo |  | true | 08-001 | 08-005 | crates/apmw-core/src/workspace/ | feature/current/extract-apmw-core/story-08-003-workspace-detection |
| 08-004 | Add version info extraction | 08 | [ ] Todo |  | true | 08-001 | 08-005 | crates/apmw-core/src/version_info/ | feature/current/extract-apmw-core/story-08-004-version-info-extraction |
| 08-005 | Publish apmw-core to crates.io | 08 | [ ] Todo |  | false | 08-001, 08-002, 08-003, 08-004 | — | crates/apmw-core/Cargo.toml, crates/apmw/Cargo.toml | feature/current/extract-apmw-core/story-08-005-publish-apmw-core |

## Notes

- 08-001 must complete first (all other stories depend on the workspace structure).
- 08-002, 08-003, 08-004 are parallel-safe after 08-001 (different modules, no shared files).
- 08-005 (publish) requires all feature stories done first.
- All stories must pass `just validate` (7 quality gates).
- Use `devbox run --` for all system tool invocations. Never use npm/npx.
