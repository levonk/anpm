---
story_id: "01-002"
story_title: "CLI subcommands and argument parsing"
story_name: "cli-subcommands-and-arg-parsing"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 1
parallel_id: 2
branch: "feature/current/apmw/story-01-002-cli-subcommands-and-arg-parsing"
status: "todo"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["src/cli.rs", "src/main.rs"]
priority: "MUST"
risk_level: "low"
tags: ["feat", "foundation", "cli", "clap"]
due: "2026-08-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Expand the CLI subcommands beyond the current stubs (Install, Detect, Status) to include all apmw commands: install, detect, status, clone, scan, list-jobs, cancel-job, suggest, info, audit-log, config. Implement all ADR-20260607001 standard arguments (--help, --version, --usage, --json, --color, --verbose, --quiet, --debug, --dry-run, --force, --human, --interactive, --tui, --no-pager, --install, --uninstall). Wire the daemon flags (--daemon, --no-daemon, --list-jobs, --cancel-job) to the daemon module interface.

## Current State

- **Relevant files and their roles:**
  - `src/main.rs` (lines 1-73) — Current CLI with clap derive. Has `Cli` struct with `daemon`, `no_daemon`, `list_jobs`, `cancel_job` flags. Has `Commands` enum with `Install`, `Detect`, `Status` stubs.
  - `Cargo.toml` (line 25) — `clap = { version = "4.4", features = ["derive", "env"] }`
- **Existing code excerpts:**
  ```rust
  // src/main.rs:8-32
  #[derive(Parser)]
  #[command(name = "apmw")]
  #[command(version)]
  #[command(about = "All Package Manager Wrapper")]
  struct Cli {
      #[arg(long)]
      daemon: bool,
      #[arg(long)]
      no_daemon: bool,
      #[arg(long)]
      list_jobs: bool,
      #[arg(long, value_name = "ID")]
      cancel_job: Option<String>,
      #[command(subcommand)]
      command: Option<Commands>,
  }
  // src/main.rs:34-42
  #[derive(Subcommand)]
  enum Commands {
      Install { package: String },
      Detect,
      Status,
  }
  ```
- **Repository conventions:** Use clap derive. Organize by feature/domain. See AGENTS.md lines 148-155.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/cli.rs` module with expanded `Cli` struct and `Commands` enum
- Add subcommands: `install`, `detect`, `status`, `clone`, `scan`, `suggest`, `info`, `audit-log`, `config`
- Add standard arguments per ADR-20260607001: `--help`, `--version`, `--usage`, `--json`, `--color` (auto|always|never), `--verbose`/`-v`, `--quiet`/`-q`, `--debug`, `--dry-run`, `--force`, `--human`, `--interactive`/`--tui`, `--no-pager`, `--install`, `--uninstall`
- Add `--no-scan`, `--scan-only`, `--on-risk`, `--update-security-db` flags for security scanning
- Add `--fields` flag for AXI minimal schemas (ADR section 37)
- Add `--full` flag for content truncation escape hatch (ADR section 38)
- Wire daemon flags to daemon module interface (trait or function call)
- Implement `--install` flag (shell completion generation, config initialization, environment setup)
- Implement `--uninstall` flag (cleanup)
- Add unit tests for CLI parsing

**Out of scope:**
- Actual command implementation logic (delegated to feature stories)
- Daemon implementation (story 01-004)
- AXI output formatting (story 01-003)

## Sub-Tasks

- [ ] Create `src/cli.rs` with expanded `Cli` struct and `Commands` enum
  **Verify**: `cargo check` → exit 0
- [ ] Add all standard arguments from ADR-20260607001 sections 1-12
  **Verify**: `apmw --help` → shows all flags
- [ ] Add security scanning flags (--no-scan, --scan-only, --on-risk, --update-security-db)
  **Verify**: `apmw install --help` → shows security flags
- [ ] Add AXI flags (--fields, --full, --human)
  **Verify**: `apmw --help` → shows AXI flags
- [ ] Implement --install flag (shell completion for bash/zsh/fish, config init)
  **Verify**: `apmw --install` → creates config, generates completions
- [ ] Implement --uninstall flag (cleanup completions and config)
  **Verify**: `apmw --uninstall` → removes generated files
- [ ] Update `src/main.rs` to use the new `src/cli.rs` module
  **Verify**: `cargo check` → exit 0
- [ ] Add unit tests for CLI parsing (all subcommands, all flags)
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/cli.rs` — New CLI module
- `src/main.rs` — Update to use cli module
- `src/lib.rs` — Add `pub mod cli;`
- `Cargo.toml` — May need additional clap features

## Acceptance Criteria

- [ ] All ADR-20260607001 standard arguments are implemented
- [ ] All apmw subcommands are defined (install, detect, status, clone, scan, suggest, info, audit-log, config)
- [ ] Security scanning flags are implemented
- [ ] AXI flags (--fields, --full, --human) are implemented
- [ ] --install generates shell completions and initializes config
- [ ] --uninstall cleans up
- [ ] All unit tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test cli` — CLI parsing tests
- Integration: `assert_cmd` tests for `apmw --help`, `apmw --version`, `apmw --usage`
- Lint: `just lint`

## Observability

- CLI parsing errors should be logged at `error` level
- Flag resolution should be logged at `debug` level

## Compliance

- ADR-20260607001 sections 1-12 (standard arguments, config, install, I/O, logging, signals, TUI, dry-run, confirmation, progress)
- ADR-20260607001 section 36-45 (AXI agent mode flags)
- rust-development-practices cli-tool-standards.md

## Risks & Mitigations

- Risk: Clap derive macro expansion may hit complexity limits — Mitigation: Use subcommands and flattened args to keep the struct shallow

## Dependencies & Sequencing

- Depends on: None (foundation story)
- Unblocks: 02-001, 04-001, 06-001, 06-003

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- The code at `src/main.rs` doesn't match the excerpts (clap derive with 3 stub commands)
- Clap 4.4 does not support a required feature

## Maintenance Notes

- Future stories will wire command implementations to the subcommand arms
- Reviewers should check that all ADR-mandated flags are present
- Shell completion generation should be tested in bash, zsh, and fish

## Commit Conventions

- `feat(cli): expand subcommands and standard arguments per ADR-20260607001`
