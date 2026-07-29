# AGENTS.md — apmw

> Binding contract for AI agents working in this repository. Read this before
> editing anything in this repo.

## Project Snapshot

| Field | Value |
|-------|-------|
| Project name | apmw (All Package Manager Wrapper) |
| Language | Rust (edition 2021, MSRV 1.70) |
| Type | CLI tool + daemon (tokio async runtime) |
| Repository | https://github.com/levonk/apmw |
| License | MIT |

## Project Overview

apmw abstracts every package installer into one intelligent surface. It
detects the correct package manager for the current project, adds tools and
dependencies with install-on-use semantics (distinguishing runtime vs.
development/build-time deps via `--dev`), maps within-ecosystem alternatives
to canonical runners (pip to uv within Python, npm/yarn/bun to pnpm within
Node — never across ecosystems), scans PATH before adding, resolves versions
intelligently with a default 2-day minimum-release-age supply-chain defense,
reads governance rules (prefer/force/block/eject) from the levonk-packages
governance spec, collects anonymized tool-usage telemetry, supports container
packages (docker/podman images), creates historyless clones with AST
indexing, runs security scanning before add, and keeps an audit log.

The tool runs as a Rust daemon (tokio "full") with a thin CLI over a local
socket. Background agents handle clone, scan, and index operations. The CLI
follows ADR-20260607001 (daemon mode, AXI agent mode, TOON output).

## Development Workflow

### AI Agent Loop

1. **Setup**: Ensure clean state and latest code
2. **Plan**: Understand requirements and create implementation plan
3. **Implement**: Write code following project patterns
4. **Test**: Run tests and verify functionality
5. **Review**: Check code quality and documentation
6. **Commit**: Commit changes with clear messages

### Commands

```bash
# Developer interface (auto-detects devbox)
just build       # Build the project (release mode)
just test        # Run all tests
just lint        # Run clippy with -D warnings
just format      # Format code with rustfmt
just check       # Run cargo check
just audit       # Run cargo audit for vulnerabilities
just outdated    # Check for outdated dependencies
just validate    # Run all quality gates (fmt + clippy + test + doc)
just clean       # Remove build artifacts
just doctor      # Check development environment
just bootstrap   # Set up development environment
```

### Implementation Commands (inside devbox)

```bash
just build_impl       # cargo build --release
just test_impl        # cargo test
just lint_impl        # cargo clippy --all-targets --all-features -- -D warnings
just format_impl      # cargo fmt --all
just check_impl       # cargo check --all-targets --all-features
just audit_impl       # cargo audit
just outdated_impl    # cargo outdated
just validate_impl    # fmt check + clippy + test + doc
just clean_impl       # cargo clean
```

## Project Structure

```
.
├── Cargo.toml              # Rust manifest with dependencies
├── justfile                # Task runner (ADR 20260131001 compliant)
├── devbox.json             # Nix dev environment (Rust toolchain)
├── .envrc                  # direnv integration
├── rustfmt.toml            # Rust formatting config (2-space, 100 char)
├── clippy.toml             # Clippy lint thresholds
├── .rust-analyzer.toml     # IDE configuration
├── Dockerfile              # Multi-stage build (rust:1.75-slim → debian:bookworm-slim)
├── .github/workflows/ci.yml  # CI pipeline (fmt, clippy, test, build, audit)
├── src/
│   ├── main.rs             # Binary entry point (CLI with clap)
│   ├── lib.rs              # Library root
│   └── error.rs            # Error types (thiserror)
├── tests/
│   └── integration_tests.rs  # Black-box integration tests
├── benches/
│   └── performance.rs      # Criterion benchmarks
└── .windsurf/rules/        # PRD/task generation rules (keep)
```

## Architecture

### Service Model

apmw follows ADR-20260607001 §13 — daemon + CLI:

- Long-running Rust daemon (tokio "full") for background jobs
- Thin CLI over a local socket
- `--daemon` / `--no-daemon` flags for explicit control
- Auto-spawn daemon on first async operation
- `--list-jobs` to show background job status
- `--cancel-job <id>` to cancel specific jobs
- Platform fallback to synchronous with clear error message

### Agent Mode (AXI)

Default agent mode per ADR-20260607001 §36-45:

- TOON (Token-Optimized Output Notation) output
- Minimal schemas, content truncation
- Pre-computed aggregates
- Definitive empty states
- Structured errors on stdout
- No interactive prompts
- Content-first no-args behavior

### Key Dependencies

| Dependency | Purpose |
|------------|---------|
| tokio | Async runtime (full features) |
| serde + serde_json | Serialization |
| thiserror | Structured error types |
| anyhow | Error context |
| clap | CLI argument parsing |
| secrecy | Secret handling |
| zeroize | Secure memory clearing |
| tracing | Structured logging |
| indicatif | Progress indicators |

### Testing Dependencies

| Dependency | Purpose |
|------------|---------|
| assert_cmd | CLI integration testing |
| predicates | Assertions for assert_cmd |
| serial_test | Serial test execution |
| criterion | Benchmarking |
| proptest | Property-based testing |
| tempfile | Temporary file handling |

## Development Guidelines

### Code Style

- Follow rustfmt configuration (2-space indent, 100 char width)
- Run `just format` before committing
- Run `just lint` to check clippy (treats warnings as errors)
- Organize modules by feature/domain, not by file type
- Use `pub use` in lib.rs for a clean public API

### Error Handling

- Use `thiserror` for structured error types in library code
- Use `anyhow` for context in binary/application code
- Never use `panic!` in library code except for unrecoverable logic errors
- Implement `From` traits via `#[from]` for error conversion

### Testing

- Write unit tests inline in source files using `#[cfg(test)]`
- Write integration tests in `tests/` directory
- Write doc tests in doc comments
- Write benchmarks in `benches/` directory
- Run `just test` to execute all tests

### Security

- Never commit secrets or credentials
- Use `secrecy` crate for secret handling
- Use `zeroize` for secure memory clearing
- Run `cargo audit` regularly (CI runs it on every push)
- Validate all external inputs

### Commits

- No AI attribution boilerplate in commit messages
- No "Generated by" or "Co-Authored-By" trailers
- Write clear subject and body describing the change

## Environment Setup

```bash
# Clone the repository
git clone https://github.com/levonk/apmw.git
cd apmw

# Enter devbox environment (provides rustc, cargo, clippy, just)
devbox shell

# Bootstrap (builds the project)
just bootstrap

# Verify environment
just doctor
```

## Quality Gates

A change is ready to merge when all of these pass:

1. `cargo check` passes without warnings
2. `cargo test` passes all tests
3. `cargo clippy` passes without warnings
4. `cargo fmt --check` passes
5. `cargo doc` generates docs without warnings
6. `cargo build --release` succeeds
7. `cargo audit` finds no vulnerabilities

Run `just validate` to check all gates at once.
