# Task Index — apmw (All Package Manager Wrapper)

**PRD**: `internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md`

## Relevant Files

- `src/main.rs` — Binary entry point (CLI with clap), currently has stub commands
- `src/lib.rs` — Library root, exports error module and version()
- `src/error.rs` — Error types (thiserror), 5 variants
- `Cargo.toml` — Rust manifest with dependencies (tokio, serde, clap, etc.)
- `justfile` — Task runner with build/test/lint/format/check/audit/validate targets
- `AGENTS.md` — Project conventions (daemon + AXI, quality gates, error handling)
- `Dockerfile` — Multi-stage build (rust:1.75-slim to debian:bookworm-slim)
- `.github/workflows/ci.yml` — CI pipeline
- `tests/integration_tests.rs` — Integration test stubs
- `benches/performance.rs` — Benchmark stubs

### Notes

- Use `just build` to build, `just test` to test, `just lint` to lint, `just validate` for all gates.
- All stories must pass `just validate` (7 quality gates: check, test, clippy, fmt, doc, build, audit).
- Module organization is by feature/domain, not by file type.
- Error handling: thiserror for library, anyhow for binary, no panics in library code.

## Parallel Development Sets

### Phase 01 — Foundation (CLI, Config, Error Types, Daemon Skeleton, AXI Output)

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 01-001 | Expand error types and config module | 01 | [x] Done |  | true | — | 02-001, 02-002, 03-001, 05-001, 06-002 | src/error.rs, src/config/ | feature/current/apmw/story-01-001-expand-error-types-and-config |
| 01-002 | CLI subcommands and argument parsing | 01 | [x] Done |  | true | — | 02-001, 04-001, 06-001, 06-003 | src/cli.rs, src/main.rs | feature/current/apmw/story-01-002-cli-subcommands-and-arg-parsing |
| 01-003 | AXI output module (TOON encoder, minimal schemas, truncation) | 01 | [x] Done |  | true | — | 02-001, 04-001, 06-001 | src/output/ | feature/current/apmw/story-01-003-axi-output-module |
| 01-004 | Daemon skeleton (tokio, local socket, job manager) | 01 | [x] Done |  | true | — | 02-001, 05-001, 06-001 | src/daemon/ | feature/current/apmw/story-01-004-daemon-skeleton |
| 01-005 | Audit log writer | 01 | [x] Done |  | true | — | 02-001, 03-001, 04-001, 05-001 | src/audit/ | feature/current/apmw/story-01-005-audit-log-writer |

### Phase 02 — Detection + PATH Scan + Ecosystem Mapping + CLI Override

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 02-001 | Package manager detection engine | 02 | [x] Done |  | true | 01-001, 01-002, 01-003, 01-004, 01-005 | 03-001, 04-001, 06-001 | src/detect/ | feature/current/apmw/story-02-001-detection-engine |
| 02-002 | PATH scanner (cli-tool-discovery integration) | 02 | [x] Done |  | true | 01-001 | 04-001 | src/path_scan/ | feature/current/apmw/story-02-002-path-scanner |
| 02-003 | Ecosystem mapping engine (within-ecosystem only) | 02 | [x] Done |  | true | 01-001 | 04-001, 04-002 | src/ecosystem/ | feature/current/apmw/story-02-003-ecosystem-mapping |
| 02-004 | `--manager <name>` CLI override | 02 | [~] In-Progress |  | true | 01-002, 02-001 | 04-001 | src/cli.rs | feature/current/apmw/story-02-004-manager-override |

### Phase 03 — Version Resolution (with min-age-days) + Security Scanning

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 03-001 | Version resolution engine (pinned/engine/latest/latest-minor + min-age-days supply-chain defense) | 03 | [ ] Todo |  | true | 01-001, 01-005, 02-001 | 04-001 | src/version/ | feature/current/apmw/story-03-001-version-resolution |
| 03-002 | Security scanning orchestrator (two-phase) | 03 | [ ] Todo |  | true | 01-001 | 04-001, 06-002 | src/security/ | feature/current/apmw/story-03-002-security-scanning |
| 03-003 | Initial scanner plugins (cargo audit, npm audit, pip-audit, osv-scanner, trivy/grype for containers) | 03 | [ ] Todo |  | true | 03-002 | 04-001, 04-004 | src/security/plugins/ | feature/current/apmw/story-03-003-scanner-plugins |

### Phase 04 — Add Engine + Dev Deps + Governance + Containers + Telemetry + Suggestions

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 04-001 | Add engine (runtime + --dev deps, install-on-use, devbox+rtk routing, --manager override) | 04 | [ ] Todo |  | false | 01-002, 01-003, 01-005, 02-001, 02-002, 02-003, 02-004, 03-001, 03-002, 03-003 | 05-001, 06-001, 06-003 | src/install/ | feature/current/apmw/story-04-001-add-engine |
| 04-002 | Alternative suggestions engine (within-ecosystem only) | 04 | [ ] Todo |  | true | 02-003 | 06-001 | src/install/suggest.rs | feature/current/apmw/story-04-002-alternative-suggestions |
| 04-003 | Governance engine (reads prefer/force/block/eject from levonk-packages spec) | 04 | [ ] Todo |  | true | 01-001, 02-003 | 06-002 | src/governance/ | feature/current/apmw/story-04-003-governance-engine |
| 04-004 | Container package support (docker/podman image pull/scan/list, compose detection) | 04 | [ ] Todo |  | true | 02-001, 03-003 | 06-001 | src/containers/ | feature/current/apmw/story-04-004-container-packages |
| 04-005 | Anonymized telemetry collector (categorical usage, opt-out, non-blocking) | 04 | [ ] Todo |  | true | 01-001 | 07-002 | src/telemetry/ | feature/current/apmw/story-04-005-telemetry-collector |

### Phase 05 — Historyless Clone + AST Indexing

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 05-001 | Historyless clone engine + local .gitignore + AST indexing | 05 | [ ] Todo |  | false | 01-001, 01-004, 01-005, 02-001 | 06-001 | src/clone/ | feature/current/apmw/story-05-001-historyless-clone-ast-index |

### Phase 06 — AI Agent Integration (MCP, Hooks, Skills, Docs Notification)

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 06-001 | MCP server (stdio transport, add/detect/scan tools) | 06 | [ ] Todo |  | true | 01-002, 01-003, 01-004, 02-001, 04-001, 04-002, 04-004, 05-001 | 07-001 | src/agent/mcp.rs | feature/current/apmw/story-06-001-mcp-server |
| 06-002 | AI agent coding hooks (hard intercept + soft convention, governance-aware) | 06 | [ ] Todo |  | true | 01-001, 03-002, 04-003 | 07-001 | src/agent/hooks.rs | feature/current/apmw/story-06-002-agent-hooks |
| 06-003 | Installable Agent Skills + session integrations + docs notification | 06 | [ ] Todo |  | true | 01-002, 01-003, 04-001 | 07-001 | src/agent/skills.rs, src/agent/docs_notify.rs | feature/current/apmw/story-06-003-agent-skills-docs-notify |

### Phase 07 — Multi-Ecosystem Release + CI/CD + Documentation

| Story ID | Title | Phase | Status | Assignee | Parallel-safe | Dependencies | Dependants | Modules | Branch |
|---|---|---:|---|---|---|---|---|---|---|
| 07-001 | Multi-ecosystem packaging and release pipeline | 07 | [ ] Todo |  | false | 06-001, 06-002, 06-003 | — | packaging/, .github/workflows/ | feature/current/apmw/story-07-001-multi-ecosystem-release |
| 07-002 | Documentation finalization (man pages, shell completion, README) | 07 | [ ] Todo |  | true | 06-001, 06-002, 06-003 | — | docs/, README.md | feature/current/apmw/story-07-002-documentation-finalization |
