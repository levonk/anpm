# Architecture Overview

This document describes the architecture of apmw — the All Package Manager
Wrapper. It covers the daemon + CLI model, AXI agent mode, TOON output,
install-on-use flow, security scanning, governance, the MCP server, agent
hooks/skills, and historyless clone + AST indexing.

## Table of Contents

- [High-Level Model](#high-level-model)
- [Daemon + CLI](#daemon--cli)
- [AXI Agent Mode and TOON Output](#axi-agent-mode-and-toon-output)
- [Install-on-Use Flow](#install-on-use-flow)
- [Ecosystem Mapping](#ecosystem-mapping)
- [Security Scanning](#security-scanning)
- [Governance](#governance)
- [MCP Server](#mcp-server)
- [Agent Hooks and Skills](#agent-hooks-and-skills)
- [Historyless Clone + AST Indexing](#historyless-clone--ast-indexing)
- [Configuration](#configuration)
- [Audit Log](#audit-log)
- [Telemetry](#telemetry)
- [Module Map](#module-map)

## High-Level Model

apmw is a Rust CLI tool + daemon built on the tokio async runtime (edition
2021, MSRV 1.70). It abstracts every package installer into one intelligent
surface:

```
┌─────────────────────────────────────────────────────────────┐
│                         User / AI Agent                      │
│                    (CLI, MCP, PATH shims)                    │
└───────────────┬───────────────────────────┬─────────────────┘
                │                           │
                ▼                           ▼
        ┌───────────────┐           ┌───────────────┐
        │   apmw CLI    │           │   MCP Server  │
        │  (thin client)│           │   (stdio)     │
        └───────┬───────┘           └───────────────┘
                │ local socket
                ▼
        ┌───────────────────────────────────────────┐
        │              apmw Daemon                  │
        │         (tokio "full" runtime)            │
        │  ┌─────────────┐  ┌────────────────────┐  │
        │  │ JobManager  │  │  Background Jobs   │  │
        │  │             │  │ (clone/scan/index) │  │
        │  └─────────────┘  └────────────────────┘  │
        └───────────────────────────────────────────┘
                │
                ▼
        ┌───────────────────────────────────────────┐
        │          Package Managers                 │
        │  pnpm · uv · cargo · go · brew · nix ...  │
        └───────────────────────────────────────────┘
```

## Daemon + CLI

apmw follows ADR-20260607001 §13 — a daemon + CLI architecture:

- **Long-running daemon** (tokio "full") handles background jobs: clone,
  scan, and AST indexing operations that may take seconds to minutes.
- **Thin CLI** communicates with the daemon over a local Unix socket.
- **`--daemon` / `--no-daemon`** flags give explicit control:
  - `--daemon` starts or connects to the daemon and submits jobs to it.
  - `--no-daemon` forces synchronous, in-process operation (no background
    jobs).
- **Auto-spawn**: when a command needs the daemon and it is not running,
  apmw auto-spawns it transparently, then submits the job.
- **`--list-jobs`** shows the status of background jobs (job ID, status,
  kind, description).
- **`--cancel-job <id>`** cancels a specific background job.
- **Platform fallback**: on platforms where the daemon is unavailable, apmw
  falls back to synchronous operation with a clear error message.

The daemon exposes a simple IPC protocol (`Request` / `Response` enums) over
the local socket. The `SocketClient` and `DaemonManager` types in
`src/daemon/` implement this protocol.

## AXI Agent Mode and TOON Output

apmw defaults to AXI (Agent eXecution Interface) agent mode per
ADR-20260607001 §36-45. In agent mode, output is TOON (Token-Optimized
Output Notation):

- **Minimal schemas** — only the requested fields are emitted. Use
  `--fields name,version` to select specific fields.
- **Content truncation** — long content is truncated to save tokens. Use
  `--full` to disable truncation.
- **Pre-computed aggregates** — counts and summaries are pre-computed so the
  agent does not need to recompute them.
- **Definitive empty states** — empty results are explicitly represented (no
  ambiguity between "no results" and "error").
- **Structured errors on stdout** — errors are structured JSON on stdout,
  not free-text on stderr, so agents can parse them.
- **No interactive prompts** — agent mode never blocks on user input.
- **Content-first no-args behavior** — running a command with no args
  produces useful content (e.g., `apmw detect` with no args detects the
  current project).

Escape hatches:

- `--human` — human-readable output (for terminal use).
- `--json` — JSON output (machine-readable, full schemas).
- `--fields <LIST>` — select specific fields.
- `--full` — disable truncation.

The `OutputDispatcher` in `src/output/` implements the rendering logic,
selecting between TOON, JSON, and human modes based on the flags.

## Install-on-Use Flow

apmw installs tools and dependencies with install-on-use semantics:

1. **Detect** the package manager for the current project (or use
   `--manager` to override).
2. **Scan PATH** to check if the tool is already installed (avoid redundant
   installs).
3. **Resolve the version** intelligently:
   - Pinned versions (from lockfiles or manifests).
   - Latest compatible (within semver constraints).
   - Latest (no constraints).
   - Latest minor (patch within a minor).
   - **2-day minimum-release-age** supply-chain defense: brand-new releases
     (younger than `min_release_age_days`, default 2) are skipped by default.
4. **Security scan** the package (two-phase: scan all, then install all).
5. **Install** the package via the canonical manager.
6. **Audit log** the operation.

The `--dev` flag marks a dependency as development/build-time (not runtime).
This distinction is preserved in the audit log and telemetry.

The `AddEngine` in `src/install/` orchestrates this flow. The
`OnUseEngine` handles install-on-use semantics, and the `PathScanner` in
`src/path_scan/` checks if a tool is already on PATH.

## Ecosystem Mapping

apmw maps within-ecosystem alternatives to canonical runners — **never
across ecosystems**:

| Ecosystem | Alternative | Canonical |
|-----------|-------------|-----------|
| Python | `pip` | `uv` |
| Node | `npm`, `yarn`, `bun` | `pnpm` |
| Node | `npx` | `pnpm dlx` |
| Python | `pipx` | `uv tool` / `uvx` |

The `EcosystemMapper` in `src/ecosystem/` implements this mapping. When a
user runs `pip install foo`, apmw (via intercept shims or direct invocation)
maps it to `uv install foo` within the Python ecosystem — but never maps
`pip` to `pnpm` (cross-ecosystem).

## Security Scanning

apmw runs security scanning before install (ADR-20260607001 §7). The scan is
**two-phase**:

1. **Scan all** — scan every package in the batch for vulnerabilities.
2. **Install all** — only after all scans pass (or the user approves), install.

Flags:

- `--no-scan` — skip security scanning entirely.
- `--scan-only` — only run the scan, then exit (don't install).
- `--on-risk <ACTION>` — what to do when risk is detected:
  - `prompt` (default) — ask the user.
  - `abort` — abort the operation.
  - `proceed` — proceed despite the risk.
  - `quarantine` — quarantine the package.
- `--update-security-db` — update the vulnerability database before scanning.

The `ScanOrchestrator` and `Scanner` in `src/security/` implement the
scanning logic. Scan results include severity, findings, and an aggregated
verdict.

## Governance

apmw reads governance rules from the levonk-packages governance spec. Rules
are of four types:

- **prefer** — prefer the canonical alternative (e.g., prefer `uv` over
  `pip`).
- **force** — force the canonical alternative (block the original).
- **block** — block the package entirely.
- **eject** — eject the package from the project.

The `GovernanceEngine` in `src/governance/` evaluates rules. The `SpecLoader`
fetches and caches the governance spec; `governance refresh` force-refreshes
it. Rules are applied during intercept (PATH shims) and during install.

## MCP Server

apmw includes an MCP (Model Context Protocol) server over stdio
(`src/agent/`). AI agents (Claude Code, Codex, OpenCode) connect to this
server to invoke apmw operations as MCP tools.

- **Transport**: stdio (line-delimited JSON-RPC 2.0).
- **Protocol version**: advertised in the initialize handshake.
- **Tools**: `add`, `detect`, `scan`, `clone`, `info`, `suggest`, etc.
- **Capabilities**: tool listing and invocation.

Start the server:

```bash
apmw mcp
```

The `McpServer`, `ToolRegistry`, `StdioTransport`, and related types in
`src/agent/` implement the server. Session integrations
(`ClaudeCodeInstaller`, `CodexInstaller`, `OpenCodeInstaller`) install the
MCP server config into agent session files.

## Agent Hooks and Skills

apmw supports two agent integration mechanisms:

### PATH Shims (Hard Intercept)

Install PATH shims that intercept package manager calls:

```bash
apmw --install --intercept
```

When a user runs `pip install foo`, the shim intercepts the call, invokes
`apmw intercept pip install foo`, which:

1. Evaluates governance rules (prefer/force/block/eject).
2. Runs security scanning if needed.
3. Outputs the effective tool and args (NUL-separated) for the shim to exec.

The `HookManager` in `src/agent/` manages shim installation and intercept
logic. Shims are written to a dedicated directory that the user adds to the
front of their PATH.

### Skills

apmw generates skill files for AI agent sessions (e.g., a SKILL.md file that
describes apmw's capabilities). The `SkillGenerator` in `src/agent/`
produces these. Session integrations install them into the appropriate
agent-specific locations.

## Historyless Clone + AST Indexing

`apmw clone <repo>` creates a historyless, branchless, tagless clone with
AST indexing:

- **Historyless** — no `.git` history (uses `--depth 1` and similar flags).
- **Branchless / tagless** — only the default branch HEAD is fetched.
- **AST indexing** — after cloning, an AST tool (selected automatically:
  `tree-sitter`, `ctags`, or similar) indexes the source for fast code
  search.
- **`.gitignore`** — a local `.gitignore` is written to exclude build
  artifacts.

In daemon mode, the clone runs as a background job (the CLI returns
immediately with a job ID). In synchronous mode (`--no-daemon`), it runs
in-process.

The `CloneEngine` in `src/clone/` implements the clone logic. The
`select_ast_tool` function picks the best available AST indexer, and
`create_index` runs the indexing.

## Configuration

apmw follows a 6-level config precedence chain (ADR-20260607001 §2):

1. CLI args (highest)
2. Environment variables (`APMW_*`)
3. Local project config (`./.apmw.toml`)
4. User config (XDG: `$XDG_CONFIG_HOME/apmw/apmw.toml`)
5. System config (`/etc/apmw/apmw.toml`)
6. Built-in defaults (lowest)

Config files are TOML. On first run, a default config with all settings
commented out is created. Legacy configs are auto-migrated with a `.bak`
backup (ADR §29).

The `config` module in `src/config/` implements loading, merging, and
migration. See the [README](../README.md#configuration) for the config file
format and environment variables.

## Audit Log

apmw keeps an audit log of all operations in
`${XDG_CACHE_HOME:-$HOME/.cache}/apmw/`. Each entry records:

- Timestamp
- Request (e.g., `detect`, `install`, `clone`)
- Action (what happened)
- Terminal type
- Caller program
- Tools used

View it with `apmw audit-log`. The `AuditLogWriter` and `AuditLogEntry`
types in `src/audit/` implement the log.

## Telemetry

apmw collects anonymized tool-usage telemetry (opt-out model, default
enabled). The telemetry records:

- Command invoked
- Outcome (success/failure)
- Error category
- Terminal type
- Timing

Flags:

- `--no-telemetry` — disable telemetry for this invocation.
- `--telemetry-preview` — print the payload that would be sent without
  sending it.

Environment variable: `APMW_TELEMETRY=false` to disable globally.

The `TelemetryCollector` and `TelemetrySender` in `src/telemetry/`
implement collection and sending.

## Module Map

| Module | Responsibility |
|--------|---------------|
| `src/cli.rs` | CLI argument definitions (clap) |
| `src/main.rs` | Binary entry point, command dispatch |
| `src/lib.rs` | Library root, public API exports |
| `src/error.rs` | Error types (thiserror) |
| `src/agent/` | MCP server, hooks, skills, session integrations |
| `src/audit/` | Audit log |
| `src/clone/` | Historyless clone + AST indexing |
| `src/config/` | Configuration management (6-level precedence) |
| `src/containers/` | Container package support (docker/podman) |
| `src/daemon/` | Daemon + job manager + IPC |
| `src/detect/` | Package manager detection |
| `src/ecosystem/` | Ecosystem mapping (pip→uv, npm→pnpm) |
| `src/governance/` | Governance rules (prefer/force/block/eject) |
| `src/install/` | Install-on-use engine, add/suggest |
| `src/output/` | TOON/AXI output dispatcher |
| `src/path_scan/` | PATH scanning |
| `src/security/` | Security scanning (two-phase) |
| `src/telemetry/` | Anonymized telemetry |
| `src/version/` | Version resolution (min-age, semver) |
| `examples/gen-assets.rs` | Man page + completion generator |
