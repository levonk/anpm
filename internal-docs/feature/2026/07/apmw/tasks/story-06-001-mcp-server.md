---
story_id: "06-001"
story_title: "MCP server (stdio transport, install/detect/scan tools)"
story_name: "mcp-server"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 6
parallel_id: 1
branch: "feature/current/apmw/story-06-001-mcp-server"
status: "todo"
assignee: ""
reviewer: ""
dependencies: ["01-002", "01-003", "01-004", "02-001", "04-001", "04-002", "05-001"]
parallel_safe: true
modules: ["src/agent/mcp.rs"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "mcp", "agent", "ai"]
due: "2026-10-30"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the MCP (Model Context Protocol) server that AI agents connect to for install/detect/scan/clone operations. The server uses stdio transport for local agent integration (Claude Code, Codex, OpenCode). It exposes tools for install, detect, scan, clone, status, and list-jobs.

## Current State

- **Relevant files and their roles:**
  - PRD FR-10.1 — MCP server requirement
  - PRD FR-9.10 — Session integrations (Claude Code, Codex, OpenCode)
  - All Phase 02-05 modules provide the operations to expose
- **Repository conventions:** Module by feature/domain. Use tokio for async.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/agent/mcp.rs` — MCP server (stdio transport)
- Implement MCP protocol handshake and tool registration
- Expose MCP tools: `apmw_install`, `apmw_detect`, `apmw_scan`, `apmw_clone`, `apmw_status`, `apmw_list_jobs`
- Each tool delegates to the corresponding apmw module (install, detect, security, clone, audit, daemon)
- Return results in MCP-compatible format (JSON)
- Handle concurrent tool calls safely
- Add unit tests for MCP protocol handling
- Add integration tests for tool execution

**Out of scope:**
- AI agent coding hooks (story 06-002)
- Installable Agent Skills (story 06-003)
- SSE/WebSocket transport (future)

## Sub-Tasks

- [ ] Create `src/agent/mcp.rs` with McpServer (stdio transport)
  **Verify**: `cargo check` → exit 0
- [ ] Implement MCP protocol handshake and tool registration
  **Verify**: `cargo test mcp_handshake` → tests pass
- [ ] Implement apmw_install tool (delegate to install engine)
  **Verify**: `cargo test mcp_install` → tests pass
- [ ] Implement apmw_detect tool (delegate to detection engine)
  **Verify**: `cargo test mcp_detect` → tests pass
- [ ] Implement apmw_scan tool (delegate to security orchestrator)
  **Verify**: `cargo test mcp_scan` → tests pass
- [ ] Implement apmw_clone tool (delegate to clone engine)
  **Verify**: `cargo test mcp_clone` → tests pass
- [ ] Implement apmw_status and apmw_list_jobs tools
  **Verify**: `cargo test mcp_status` → tests pass
- [ ] Handle concurrent tool calls safely
  **Verify**: `cargo test mcp_concurrent` → tests pass
- [ ] Add integration tests for tool execution
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/agent/mcp.rs` — MCP server
- `src/agent/mod.rs` — Agent module root
- `src/lib.rs` — Add `pub mod agent;`
- `Cargo.toml` — May need MCP-related crates

## Acceptance Criteria

- [ ] MCP server uses stdio transport
- [ ] Protocol handshake works correctly
- [ ] All 6 tools are registered and executable
- [ ] Tools delegate to the correct apmw modules
- [ ] Results are returned in MCP-compatible format
- [ ] Concurrent tool calls are handled safely
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test mcp` — MCP protocol and tool tests
- Integration: Tool execution tests
- Lint: `just lint`

## Observability

- MCP tool calls should be logged at `info` level
- MCP protocol errors should be logged at `error` level

## Compliance

- PRD FR-10.1 (MCP server)
- MCP (Model Context Protocol) specification

## Risks & Mitigations

- Risk: MCP protocol may change — Mitigation: Abstract the protocol behind a trait, update as needed
- Risk: stdio transport may conflict with CLI output — Mitigation: MCP server runs as a separate process, not mixed with CLI

## Dependencies & Sequencing

- Depends on: 01-002 (CLI), 01-003 (AXI output), 01-004 (daemon), 02-001 (detection), 04-001 (install), 04-002 (suggest), 05-001 (clone)
- Unblocks: 07-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- MCP protocol specification is not accessible or is ambiguous
- stdio transport cannot be implemented with tokio

## Maintenance Notes

- New MCP tools can be added by registering them in the tool registry
- Reviewers should check that tool results are correctly formatted
- SSE/WebSocket transport may be added in a future update

## Commit Conventions

- `feat(mcp): add MCP server with stdio transport and install/detect/scan/clone tools`
