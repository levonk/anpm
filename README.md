# apmw — All Package Manager Wrapper

apmw abstracts every package installer into one intelligent surface. It
detects the correct package manager for the current project, installs tools
and dependencies with install-on-use semantics, maps ecosystems (pip to uv,
npm/yarn/bun to pnpm), scans PATH before installing, resolves versions
intelligently, creates historyless clones with AST indexing, runs security
scanning before install, and keeps an audit log.

## Quick Start

```bash
# Clone
git clone https://github.com/levonk/apmw.git
cd apmw

# Enter devbox environment (provides Rust toolchain + just)
devbox shell

# Bootstrap
just bootstrap

# Verify environment
just doctor
```

## Build and Test

```bash
just build       # Build (release mode)
just test        # Run all tests
just lint        # Run clippy (warnings as errors)
just format      # Format code
just validate    # Run all quality gates
just clean       # Remove build artifacts
```

Run `just --list` to see all available commands.

## Project Structure

```
.
├── Cargo.toml              # Rust manifest
├── justfile                # Task runner
├── devbox.json             # Nix dev environment
├── .envrc                  # direnv integration
├── rustfmt.toml            # Formatting config
├── clippy.toml             # Lint thresholds
├── Dockerfile              # Multi-stage container build
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library root
│   └── error.rs            # Error types
├── tests/
│   └── integration_tests.rs
├── benches/
│   └── performance.rs
└── .github/workflows/ci.yml
```

## Architecture

apmw runs as a Rust daemon (tokio) with a thin CLI over a local socket.
Background agents handle clone, scan, and index operations. The CLI follows
ADR-20260607001 with daemon mode support and AXI agent mode (token-efficient
output for AI agents).

Key features:

- Detect the correct package manager for the current project
- Install tools and dependencies with install-on-use semantics
- Map ecosystems: pip to uv, npm/yarn/bun to pnpm, uvx to pnpm dlx
- Scan PATH to check if a tool is already installed
- Resolve versions intelligently (pinned, latest compatible, latest, latest minor)
- Create historyless, branchless, tagless clones with AST indexing
- Run security scanning before install (two-phase: scan all, then install all)
- Keep an audit log in `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/`
- Expose AI agent hooks, MCP, and Skills
- Ship as multi-ecosystem release (npm/pnpm, PyPI/uv, brew, nix, devbox, apt, winget, apk, deb, rpm)

## AI Agent Documentation

For AI agent conventions, build commands, and architecture details, see
[AGENTS.md](AGENTS.md).

## License

MIT
