# apmw — All Package Manager Wrapper

apmw abstracts every package installer into one intelligent surface. It
detects the correct package manager for the current project, installs tools
and dependencies with install-on-use semantics (distinguishing runtime vs.
development/build-time deps via `--dev`), maps within-ecosystem alternatives
to canonical runners (pip to uv within Python, npm/yarn/bun to pnpm within
Node — never across ecosystems), scans PATH before adding, resolves versions
intelligently with a default 2-day minimum-release-age supply-chain defense,
reads governance rules (prefer/force/block/eject) from the levonk-packages
governance spec, collects anonymized tool-usage telemetry, supports container
packages (docker/podman images), creates historyless clones with AST
indexing, runs security scanning before install, and keeps an audit log.

The tool runs as a Rust daemon (tokio "full") with a thin CLI over a local
socket. Background agents handle clone, scan, and index operations. The CLI
follows ADR-20260607001 (daemon mode, AXI agent mode, TOON output).

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage](#usage)
- [Configuration](#configuration)
- [Examples](#examples)
- [Architecture](#architecture)
- [Shell Completions](#shell-completions)
- [Man Pages](#man-pages)
- [AI Agent Integration](#ai-agent-integration)
- [Build and Test](#build-and-test)
- [Project Structure](#project-structure)
- [License](#license)

## Installation

apmw ships as a multi-ecosystem release. Choose the package manager that
matches your environment.

### npm / pnpm

```bash
npm install -g apmw        # or: pnpm add -g apmw
```

### PyPI / uv

```bash
pip install apmw           # or: uv tool install apmw
```

### Homebrew (macOS / Linuxbrew)

```bash
brew install levonk/tap/apmw
```

### Nix / Devbox

```bash
# Nix flake
nix profile install github:levonk/apmw

# Devbox (inside a devbox project)
devbox add apmw
```

### apt (Debian / Ubuntu)

```bash
curl -fsSL https://apt.apmw.dev/gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/apmw.gpg
echo "deb [signed-by=/usr/share/keyrings/apmw.gpg] https://apt.apmw.dev stable main" | sudo tee /etc/apt/sources.list.d/apmw.list
sudo apt update && sudo apt install apmw
```

### winget (Windows)

```powershell
winget install levonk.apmw
```

### Pre-built binaries

Download the latest release for your platform from the
[releases page](https://github.com/levonk/apmw/releases) and add the binary
to your `PATH`.

### Build from source

```bash
git clone https://github.com/levonk/apmw.git
cd apmw

# Enter the devbox environment (provides Rust toolchain + just)
devbox shell

# Build (release mode)
just build

# Or build directly with cargo
cargo build --release
# Binary: target/release/apmw
```

### Post-install setup

After installing, generate shell completions and initialize the config file:

```bash
apmw --install                  # generates completions for bash, zsh, fish
apmw --install --shell zsh      # generate only for zsh
```

To install PATH shims that intercept package manager calls (hard intercept):

```bash
apmw --install --intercept
```

To remove everything later:

```bash
apmw --uninstall                # remove completions + config
apmw --uninstall --intercept    # remove PATH shims
```

## Quick Start

```bash
# Detect the package manager for the current project
apmw detect

# Install a package (auto-detects the manager)
apmw install express

# Install a development/build-time dependency
apmw install jest --dev

# Force a specific manager
apmw install lodash --manager pnpm

# Scan a package for security issues before installing
apmw scan left-pad

# Clone a repo historylessly with AST indexing
apmw clone https://github.com/owner/repo

# Show status
apmw status
```

## Usage

```
apmw [OPTIONS] [COMMAND]
```

### Commands

| Command | Description |
|---------|-------------|
| `install <package>` | Install a package or tool. Use `--dev` for build-time deps. |
| `detect` | Detect the package manager for the current project. |
| `status` | Show apmw version and status. |
| `clone <repo>` | Create a historyless clone of a repository with AST indexing. |
| `scan [package]` | Scan a package (or the current project) for security issues. |
| `suggest <package>` | Suggest within-ecosystem alternatives for a package. |
| `info <package>` | Show detailed info about a package. |
| `audit-log` | Show the audit log of past operations. |
| `config` | View or initialize configuration (`--init`, `--show`). |
| `governance refresh` | Force-refresh the cached governance spec from levonk-packages. |
| `intercept <tool> [args...]` | Intercept a package manager call (invoked by PATH shims). |
| `mcp` | Start the MCP (Model Context Protocol) server over stdio. |

### Global Flags

These flags are available on all subcommands.

#### I/O and Output

| Flag | Description |
|------|-------------|
| `--json` | Emit output as JSON (machine-readable). |
| `--color <WHEN>` | Color output: `auto` (default), `always`, `never`. |
| `--human` | Human-readable output (escape hatch for AXI/TOON mode). |
| `--no-pager` | Disable pager output. |

#### Logging

| Flag | Description |
|------|-------------|
| `-v` / `-vv` | Increase verbosity (repeatable). |
| `-q` / `--quiet` | Suppress non-error output. |
| `--debug` | Enable debug-level diagnostics. |

#### Execution Control

| Flag | Description |
|------|-------------|
| `--dry-run` | Show what would happen without making changes. |
| `--force` | Force the operation, bypassing confirmations. |
| `--interactive` / `--tui` | Enable interactive/TUI mode. |

#### Security Scanning

| Flag | Description |
|------|-------------|
| `--no-scan` | Skip security scanning. |
| `--scan-only` | Only run the security scan, then exit. |
| `--on-risk <ACTION>` | Action on risk: `prompt` (default), `abort`, `proceed`, `quarantine`. |
| `--update-security-db` | Update the security vulnerability database before scanning. |

#### AXI Agent Mode

| Flag | Description |
|------|-------------|
| `--fields <LIST>` | Comma-separated fields to include in AXI minimal output. |
| `--full` | Show full content (escape hatch for AXI truncation). |

#### Daemon Control

| Flag | Description |
|------|-------------|
| `--daemon` | Run in daemon mode (long-running background process). |
| `--no-daemon` | Disable daemon mode (force synchronous operation). |
| `--list-jobs` | List background jobs. |
| `--cancel-job <ID>` | Cancel a background job by ID. |

#### Manager Override

| Flag | Description |
|------|-------------|
| `--manager <NAME>` / `--use <NAME>` | Override auto-detection and force a specific package manager. |

Valid managers: `pnpm`, `npm`, `yarn`, `bun`, `uv`, `pip`, `poetry`, `pipenv`,
`pdm`, `conda`, `cargo`, `go`, `gem`, `brew`, `nix`, `devbox`, `apt`, `dnf`,
`pacman`, `winget`, `snap`, `flatpak`, `helm`, `docker`, `podman`, `maven`,
`gradle`, `sbt`, `dotnet`.

#### Telemetry

| Flag | Description |
|------|-------------|
| `--no-telemetry` | Disable telemetry collection for this invocation. |
| `--telemetry-preview` | Print the telemetry payload without sending it. |

#### Install / Setup

| Flag | Description |
|------|-------------|
| `--install` | Generate shell completions and initialize the config file. |
| `--uninstall` | Remove generated completions and config (best-effort). |
| `--intercept` | Install/remove PATH shims (with `--install`/`--uninstall`). |
| `--shell <SHELL>` | Shell to generate completions for: `bash`, `zsh`, `fish`. |
| `--man` | Print the man page (groff/troff) to stdout and exit. |
| `--usage` | Print a brief usage summary and exit. |
| `--help` | Show full help. |
| `--version` | Show version. |

### Brief Usage

For a quick reference, run:

```bash
apmw --usage
```

### Full Help

```bash
apmw --help            # top-level help
apmw install --help    # help for a specific subcommand
```

## Configuration

apmw follows a 6-level config precedence chain (highest to lowest):

1. **CLI args** — highest precedence
2. **Environment variables** — `APMW_*`
3. **Local project config** — `./.apmw.toml`
4. **User config (XDG)** — `$XDG_CONFIG_HOME/apmw/apmw.toml` (or `~/.config/apmw/apmw.toml`)
5. **System config** — `/etc/apmw/apmw.toml`
6. **Defaults** — built-in sensible defaults

### Config File Format

Config files are TOML. On first run, a default config with all settings
commented out is created. Initialize it with:

```bash
apmw config --init
```

Example `apmw.toml`:

```toml
# Whether the daemon is enabled (background jobs for clone/scan/index).
daemon_enabled = false

# Minimum release age (in days) before a version is considered for install.
# Supply-chain defense: avoids brand-new, untested releases. Default: 2.
min_release_age_days = 2

# AXI agent mode (token-optimized TOON output for AI agents).
agent_mode = false

# Telemetry (anonymized tool-usage). Opt-out model: default true.
telemetry = true

# Telemetry endpoint URL.
telemetry_endpoint = "https://telemetry.apmw.dev/v1/event"
```

View the resolved configuration:

```bash
apmw config --show
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `APMW_DAEMON_ENABLED` | Override daemon enabled flag (`1`/`true`/`on` or `0`/`false`/`off`). |
| `APMW_MIN_RELEASE_AGE_DAYS` | Override minimum release age (integer). |
| `APMW_AGENT_MODE` | Override agent mode (boolean). |
| `APMW_TELEMETRY` | Override telemetry enabled flag (boolean). |

## Examples

See [`docs/examples/`](docs/examples/) for detailed usage scenarios.

### Detect the package manager

```bash
$ cd my-project
$ apmw detect
```

### Install a package

```bash
# Auto-detect manager and install
apmw install express

# Install as a development dependency
apmw install jest --dev

# Force a specific manager
apmw install lodash --manager pnpm

# Dry run (show what would happen)
apmw install react --dry-run
```

### Scan for vulnerabilities

```bash
# Scan a specific package
apmw scan left-pad

# Scan the current project
apmw scan

# Update the vulnerability DB first, then scan
apmw scan --update-security-db

# Only run the scan (don't install)
apmw install pkg --scan-only
```

### Clone a repository

```bash
# Historyless clone with AST indexing (synchronous)
apmw clone https://github.com/owner/repo

# Run as a background job via the daemon
apmw --daemon clone https://github.com/owner/repo

# List background jobs
apmw --list-jobs

# Cancel a job
apmw --cancel-job <job-id>
```

### Suggest alternatives

```bash
apmw suggest npm
```

### View audit log

```bash
apmw audit-log
```

### Start the MCP server

```bash
apmw mcp
```

AI agents (Claude Code, Codex, OpenCode) connect to this server over stdio
to invoke apmw operations as MCP tools.

### Install agent hooks

```bash
apmw --install --intercept
```

## Architecture

apmw runs as a Rust daemon (tokio "full") with a thin CLI over a local
socket. For the full architecture overview, see
[`docs/architecture.md`](docs/architecture.md).

Key design points:

- **Daemon + CLI** — A long-running daemon handles background jobs (clone,
  scan, index). The CLI is a thin client over a local socket. Use
  `--daemon` / `--no-daemon` for explicit control; the daemon auto-spawns on
  first async operation.
- **AXI Agent Mode** — Default output is TOON (Token-Optimized Output
  Notation) for AI agents: minimal schemas, content truncation, pre-computed
  aggregates, definitive empty states, structured errors on stdout, no
  interactive prompts. Use `--human` for human-readable output.
- **Install-on-use** — Tools are installed on first use with install-on-use
  semantics, distinguishing runtime vs. development/build-time deps via
  `--dev`.
- **Ecosystem mapping** — Within-ecosystem alternatives map to canonical
  runners (pip to uv within Python, npm/yarn/bun to pnpm within Node — never
  across ecosystems).
- **Security scanning** — Two-phase: scan all packages, then install all.
  Configurable via `--no-scan`, `--scan-only`, `--on-risk`.
- **Governance** — Reads prefer/force/block/eject rules from the
  levonk-packages governance spec.
- **Historyless clone + AST indexing** — Clones are historyless, branchless,
  and tagless, with AST indexing for fast code search.
- **MCP server** — Exposes apmw operations as MCP tools over stdio for AI
  agents.
- **Agent hooks/skills** — PATH shims intercept package manager calls; skill
  files are generated for AI agent sessions.

## Shell Completions

apmw can generate shell completion scripts for bash, zsh, and fish.

### Auto-install completions

```bash
apmw --install --shell bash   # or zsh, fish
apmw --install                # all three shells
```

### Static completion files

Pre-generated completion scripts are in the [`completions/`](completions/)
directory. To regenerate them:

```bash
just completions
# or
cargo run --example gen-assets -- completions
```

### Manual installation

**Bash:**

```bash
cp completions/apmw.bash /etc/bash_completion.d/apmw
# or for user-level:
mkdir -p ~/.local/share/bash-completion/completions
cp completions/apmw.bash ~/.local/share/bash-completion/completions/apmw
```

**Zsh:**

```bash
cp completions/apmw.zsh ~/.zfunc/_apmw
# Ensure ~/.zshrc contains: fpath+=~/.zfunc; autoload -Uz compinit && compinit
```

**Fish:**

```bash
cp completions/apmw.fish ~/.config/fish/completions/apmw.fish
```

## Man Pages

A man page is generated via `clap_mangen` and stored in [`man/man1/apmw.1`](man/man1/apmw.1).

### View the man page

```bash
# Once installed to a manpath:
man apmw

# Or view directly from the repo:
man -l man/man1/apmw.1     # GNU man
MANPATH=$(pwd)/man man apmw

# Or print to stdout via the --man flag:
apmw --man | man -l -     # pipe to man
apmw --man > apmw.1        # save to a file
```

### Regenerate man pages

```bash
just man
# or
cargo run --example gen-assets -- man
```

## AI Agent Integration

apmw is designed for AI agent use (Claude Code, Codex, OpenCode).

### MCP Server

Start the MCP server over stdio:

```bash
apmw mcp
```

AI agents connect to this server to invoke apmw operations (add, detect,
scan, clone, etc.) as MCP tools via JSON-RPC 2.0.

### Agent Hooks (PATH shims)

Install PATH shims that intercept package manager calls:

```bash
apmw --install --intercept
```

The shims evaluate governance rules, run security scanning, and delegate to
the canonical binary (or the canonical alternative if governance forces it).

### AXI / TOON Output

By default, apmw emits TOON (Token-Optimized Output Notation) — minimal,
token-efficient output ideal for AI agents. Use `--human` for human-readable
output, or `--json` for JSON. Use `--fields` to select specific fields and
`--full` to disable truncation.

For AI agent conventions, build commands, and architecture details, see
[AGENTS.md](AGENTS.md).

## Build and Test

```bash
just build       # Build (release mode)
just test        # Run all tests
just lint        # Run clippy (warnings as errors)
just format      # Format code
just check       # Run cargo check
just validate    # Run all quality gates (fmt + clippy + test + doc)
just clean       # Remove build artifacts
just man         # Generate man pages
just completions # Generate shell completions
just assets      # Generate both man pages and completions
```

Run `just --list` to see all available commands.

## Project Structure

```
.
├── Cargo.toml              # Rust manifest with dependencies
├── justfile                # Task runner (ADR 20260131001 compliant)
├── devbox.json             # Nix dev environment (Rust toolchain)
├── .envrc                  # direnv integration
├── rustfmt.toml            # Rust formatting config (2-space, 100 char)
├── clippy.toml             # Clippy lint thresholds
├── Dockerfile              # Multi-stage build (rust:1.75-slim → debian:bookworm-slim)
├── src/
│   ├── main.rs             # Binary entry point (CLI with clap)
│   ├── lib.rs              # Library root
│   ├── cli.rs              # CLI argument definitions
│   ├── error.rs            # Error types (thiserror)
│   ├── agent/              # MCP server, hooks, skills, session integrations
│   ├── audit/              # Audit log
│   ├── clone/              # Historyless clone + AST indexing
│   ├── config/             # Configuration management
│   ├── containers/         # Container package support
│   ├── daemon/             # Daemon + job manager
│   ├── detect/             # Package manager detection
│   ├── ecosystem/          # Ecosystem mapping (pip→uv, npm→pnpm)
│   ├── governance/         # Governance rules (prefer/force/block/eject)
│   ├── install/            # Install-on-use engine
│   ├── output/             # TOON/AXI output dispatcher
│   ├── path_scan/          # PATH scanning
│   ├── security/           # Security scanning
│   ├── telemetry/          # Anonymized telemetry
│   └── version/            # Version resolution
├── examples/
│   └── gen-assets.rs       # Man page + completion generator
├── tests/
│   └── integration_tests.rs  # Black-box integration tests
├── benches/
│   └── performance.rs      # Criterion benchmarks
├── man/
│   └── man1/apmw.1         # Generated man page
├── completions/
│   ├── apmw.bash           # Bash completion
│   ├── apmw.zsh            # Zsh completion
│   └── apmw.fish           # Fish completion
├── docs/
│   ├── architecture.md     # Architecture overview
│   └── examples/           # Example usage scenarios
└── .github/workflows/ci.yml  # CI pipeline (fmt, clippy, test, build, audit)
```

## License

MIT
