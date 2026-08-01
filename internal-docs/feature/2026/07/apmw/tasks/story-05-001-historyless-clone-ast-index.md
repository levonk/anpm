---
story_id: "05-001"
story_title: "Historyless clone engine + local .gitignore + AST indexing"
story_name: "historyless-clone-ast-index"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 5
parallel_id: 1
branch: "feature/current/apmw/story-05-001-historyless-clone-ast-index"
status: "done"
assignee: ""
reviewer: ""
dependencies: ["01-001", "01-004", "01-005", "02-001"]
parallel_safe: false
modules: ["src/clone/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "clone", "historyless", "ast-index", "gitignore"]
due: "2026-10-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the historyless clone engine that clones any package with `--depth 1 --single-branch --no-tags`, adds a local `.gitignore` that excludes AST index files, devbox.json, and AGENTS.md, and loads the appropriate indexed-AST search tool (CodeGraph/Graphify/GitNexus per the indexed-ast-tools decision tree) to create indexes.

## Current State

- **Relevant files and their roles:**
  - PRD FR-6 (sections FR-6.1 through FR-6.4) — Historyless clone requirements
  - indexed-ast-tools.md at `~/p/gh/levonk/skills-src/src/current/knowledge/software-architecture-essentials/indexed-ast-tools.md` — AST tool decision tree
  - `src/daemon/` — Daemon and job manager (from story 01-004) for background clone/index jobs
- **Existing code excerpts (from indexed-ast-tools.md):**
  ```
  Decision tree (lines 34-48):
  1. Project < 20 files? Skip.
  2. Need to link non-code knowledge? Use Graphify (MIT, multimodal).
  3. Work across multiple repos? Use GitNexus (PolyForm Noncommercial).
  4. Need zero-maintenance + dynamic dispatch tracing? Use CodeGraph (MIT) — default.
  5. Power-user structural queries on a single repo? Use GitNexus (17 MCP tools).
  ```
- **Repository conventions:** Module by feature/domain. Use tokio for async git operations.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/clone/mod.rs` — Historyless clone engine
- Create `src/clone/gitignore.rs` — Local .gitignore generation (exclude: AST index files, devbox.json, AGENTS.md)
- Create `src/clone/ast_index.rs` — AST indexing integration (CodeGraph/Graphify/GitNexus decision tree)
- Implement `git clone --depth 1 --single-branch --no-tags` via subprocess or git2 crate
- Implement local .gitignore generation after clone
- Implement AST tool selection per the indexed-ast-tools decision tree:
  1. Project < 20 files? Skip indexing
  2. Need multimodal? Use Graphify
  3. Multi-repo? Use GitNexus
  4. Default: Use CodeGraph (zero-maintenance, dynamic dispatch)
  5. Power-user single repo? Use GitNexus
- Implement index creation by invoking the selected AST tool
- Wire `apmw clone <repo>` command
- Run clone and index as background jobs in daemon mode
- Output clone/index status in TOON format
- Write audit log entry for each clone operation
- Add integration tests with assert_cmd for `apmw clone`

**Out of scope:**
- AST tool implementation (apmw delegates to existing CodeGraph/Graphify/GitNexus)
- MCP integration for AST tools (story 06-001)

## Sub-Tasks

- [x] Create `src/clone/mod.rs` with CloneEngine
  **Verify**: `cargo check` → exit 0
- [x] Implement historyless clone (git clone --depth 1 --single-branch --no-tags)
  **Verify**: `cargo test clone` → tests pass
- [x] Create `src/clone/gitignore.rs` with local .gitignore generation
  **Verify**: `cargo test gitignore` → tests pass
- [x] Create `src/clone/ast_index.rs` with AST tool decision tree
  **Verify**: `cargo test ast_index` → tests pass
- [x] Implement CodeGraph integration (default tool)
  **Verify**: `cargo test codegraph` → tests pass
- [x] Implement Graphify integration (multimodal)
  **Verify**: `cargo test graphify` → tests pass
- [x] Implement GitNexus integration (multi-repo)
  **Verify**: `cargo test gitnexus` → tests pass
- [x] Implement skip-indexing for projects < 20 files
  **Verify**: `cargo test skip_index` → tests pass
- [x] Wire `apmw clone <repo>` command
  **Verify**: `apmw clone <test-repo>` → creates historyless clone with .gitignore
- [x] Run clone and index as background jobs in daemon mode
  **Verify**: `apmw --daemon clone <large-repo>` → returns job ID
- [x] Output clone/index status in TOON format
  **Verify**: `apmw clone <test-repo>` → valid TOON output
- [x] Write audit log entry for each clone
  **Verify**: audit log contains entry after clone
- [x] Add integration tests with assert_cmd
  **Verify**: `just test` → all pass
- [x] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/clone/mod.rs` — Clone engine (CloneEngine, CloneResult, derive_repo_name, default_clone_dest)
- `src/clone/gitignore.rs` — .gitignore generation (write_gitignore, generate_gitignore_contents, IGNORED_PATTERNS)
- `src/clone/ast_index.rs` — AST indexing (AstTool, IndexOptions, select_ast_tool, create_index, count_files, is_tool_available)
- `src/lib.rs` — Added `pub mod clone;` and `pub use` exports
- `src/main.rs` — Wired Clone command (handle_clone, run_clone_as_job with TOON output, audit log, daemon jobs)
- `tests/integration_tests.rs` — Added 10 integration tests for `apmw clone`
- `Cargo.toml` — No changes needed (uses tokio::process::Command subprocess, not git2 crate)

## Acceptance Criteria

- [x] Clone uses --depth 1 --single-branch --no-tags
- [x] Local .gitignore excludes AST index files, devbox.json, AGENTS.md
- [x] AST tool selection follows the indexed-ast-tools decision tree
- [x] CodeGraph is the default for single-project workflows
- [x] Projects < 20 files skip indexing
- [x] Clone and index run as background jobs in daemon mode
- [x] Output is in TOON format in agent mode
- [x] Audit log entry is written for each clone
- [x] All integration tests pass
- [x] `just validate` passes

## Test Plan

- Unit: `cargo test clone` — clone engine, gitignore, ast_index tests
- Integration: `assert_cmd` tests for `apmw clone` with test repos
- Lint: `just lint`

## Observability

- Clone lifecycle events should be logged at `info` level
- AST tool selection should be logged at `info` level
- Index creation progress should be logged at `debug` level
- Clone/index failures should be logged at `error` level

## Compliance

- PRD FR-6 (historyless clone requirements)
- indexed-ast-tools.md decision tree

## Risks & Mitigations

- Risk: AST tools may not be installed — Mitigation: Check availability before invoking, suggest installation if missing
- Risk: Large repos may take a long time to index — Mitigation: Run as background job in daemon mode, report progress

## Dependencies & Sequencing

- Depends on: 01-001 (error types), 01-004 (daemon), 01-005 (audit), 02-001 (detection)
- Unblocks: 06-001

## Definition of Done

- [x] All verification commands pass
- [x] Code, tests, docs updated; CI green; story file updated
- [x] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- git clone --depth 1 --single-branch --no-tags is not supported by the installed git version
- AST tools (CodeGraph/Graphify/GitNexus) are not available and cannot be installed

## Maintenance Notes

- New AST tools can be added by extending the decision tree in ast_index.rs
- Reviewers should check that .gitignore excludes all required files
- Clone should be fast (shallow clone) and index should run as a background job

## Commit Conventions

- `feat(clone): add historyless clone engine with AST indexing and local .gitignore`
