# Using the MCP Server

This example shows how to use apmw's MCP (Model Context Protocol) server for
AI agent integration.

## Start the MCP server

```bash
$ apmw mcp
```

The server reads line-delimited JSON-RPC 2.0 messages from stdin and writes
responses to stdout. AI agents (Claude Code, Codex, OpenCode) connect to
this server to invoke apmw operations as MCP tools.

## Protocol

The MCP server implements JSON-RPC 2.0 over stdio:

- **Initialize** — handshake; the server advertises its capabilities and
  protocol version.
- **tools/list** — returns the list of available apmw tools.
- **tools/call** — invokes a tool (e.g., `add`, `detect`, `scan`, `clone`).

## Available tools

The MCP server exposes apmw operations as tools, including:

| Tool | Description |
|------|-------------|
| `add` | Add (install) a package |
| `detect` | Detect the package manager |
| `scan` | Scan a package for vulnerabilities |
| `clone` | Historyless clone with AST indexing |
| `info` | Show package info |
| `suggest` | Suggest within-ecosystem alternatives |

## Agent session integration

apmw can install the MCP server config into AI agent session files:

- **Claude Code** — `ClaudeCodeInstaller` writes the MCP server config.
- **Codex** — `CodexInstaller` writes the MCP server config.
- **OpenCode** — `OpenCodeInstaller` writes the MCP server config.

Use `install_all_session_integrations` to install for all supported agents.

## Example JSON-RPC interaction

```jsonc
// Client → Server (initialize)
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}

// Server → Client (response)
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"apmw","version":"0.1.0"},"capabilities":{"tools":{}}}}

// Client → Server (list tools)
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}

// Client → Server (call a tool)
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"detect","arguments":{}}}
```

## TOON output in MCP

Tool results are returned in TOON (token-optimized) format by default,
minimizing token usage for AI agents. The agent can request specific fields
or full output via tool arguments.
