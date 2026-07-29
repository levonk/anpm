---
story_id: "01-004"
story_title: "Daemon skeleton (tokio, local socket, job manager)"
story_name: "daemon-skeleton"
prd_name: "apmw"
prd_file: "internal-docs/feature/2026/07/apmw/feat-202607290558-apmw.md"
phase: 1
parallel_id: 4
branch: "feature/current/apmw/story-01-004-daemon-skeleton"
status: "todo"
assignee: ""
reviewer: ""
dependencies: []
parallel_safe: true
modules: ["src/daemon/"]
priority: "MUST"
risk_level: "medium"
tags: ["feat", "foundation", "daemon", "tokio", "socket"]
due: "2026-08-15"
created_at: "2026-07-29"
updated_at: "2026-07-29"
---

## Summary

Create the daemon skeleton using tokio "full" as the async runtime. Implement a long-running daemon process that listens on a local socket, a job manager for background operations (clone, scan, index), auto-spawn on first async operation, platform fallback to synchronous mode, and the --daemon/--no-daemon/--list-jobs/--cancel-job interface per ADR-20260607001 section 13.

## Current State

- **Relevant files and their roles:**
  - `src/main.rs` (lines 14-28) — Has daemon flags defined (--daemon, --no-daemon, --list-jobs, --cancel-job) but they print placeholder messages
  - `Cargo.toml` (line 20) — `tokio = { version = "1.35", features = ["full"] }`
- **Existing code excerpts:**
  ```rust
  // src/main.rs:47-55
  if cli.list_jobs {
      println!("No background jobs running.");
      return Ok(());
  }
  if let Some(job_id) = cli.cancel_job {
      println!("Cancelling job: {job_id}");
      return Ok(());
  }
  ```
- **Repository conventions:** Use tokio "full" as async runtime. Use #[tokio::test] for async tests. Never block in async contexts. See AGENTS.md lines 97-108 and async-patterns.md.
- **Build/test/lint commands:**
  | Purpose   | Command                  | Expected Result |
  |-----------|--------------------------|-----------------|
  | Build     | `just build`             | exit 0          |
  | Tests     | `just test`              | all pass        |
  | Lint      | `just lint`              | exit 0          |

## Scope

**In scope:**
- Create `src/daemon/mod.rs` — Daemon manager (start, stop, status)
- Create `src/daemon/socket.rs` — Local socket IPC (Unix domain socket on Linux/macOS, named pipe on Windows)
- Create `src/daemon/jobs.rs` — Background job manager (job ID generation, status tracking, cancellation)
- Implement auto-spawn: daemon starts on first async operation if not already running
- Implement --daemon flag: pre-launch daemon in background and wait for jobs
- Implement --no-daemon flag: force synchronous in-process operation
- Implement --list-jobs: return job status (with job ID immediately per ADR section 13)
- Implement --cancel-job <id>: cancel specific background jobs
- Implement platform fallback: if daemon not supported, fall back to synchronous with clear error
- Implement SIGINT handling (exit code 130) and SIGHUP (config reload per ADR section 31)
- Add unit tests for job manager (create, track, cancel jobs)
- Add integration tests for daemon spawn and socket communication

**Out of scope:**
- Actual background job implementations (clone, scan, index — later stories)
- CLI argument parsing (story 01-002)
- Config management (story 01-001)

## Sub-Tasks

- [ ] Create `src/daemon/mod.rs` with DaemonManager (start, stop, status, auto-spawn)
  **Verify**: `cargo check` → exit 0
- [ ] Create `src/daemon/socket.rs` with local socket IPC (Unix domain socket, named pipe on Windows)
  **Verify**: `cargo test socket` → tests pass
- [ ] Create `src/daemon/jobs.rs` with JobManager (job ID, status tracking, cancellation)
  **Verify**: `cargo test jobs` → tests pass
- [ ] Implement auto-spawn logic (detect first async op, spawn daemon if not running)
  **Verify**: `cargo test auto_spawn` → tests pass
- [ ] Implement --no-daemon synchronous fallback with clear error message
  **Verify**: `cargo test sync_fallback` → tests pass
- [ ] Implement SIGINT (exit 130) and SIGHUP (config reload) signal handlers
  **Verify**: `cargo test signals` → tests pass
- [ ] Add integration tests for daemon spawn and socket communication
  **Verify**: `just test` → all pass
- [ ] Run `just validate`
  **Verify**: `just validate` → all gates pass

## Relevant Files

- `src/daemon/mod.rs` — Daemon manager
- `src/daemon/socket.rs` — Local socket IPC
- `src/daemon/jobs.rs` — Background job manager
- `src/lib.rs` — Add `pub mod daemon;`
- `src/main.rs` — Wire daemon flags to daemon module

## Acceptance Criteria

- [ ] Daemon starts and listens on a local socket
- [ ] Auto-spawn works on first async operation
- [ ] --no-daemon forces synchronous operation
- [ ] --list-jobs returns job status with job ID
- [ ] --cancel-job cancels a specific job
- [ ] Platform fallback provides clear error message
- [ ] SIGINT exits with code 130, SIGHUP reloads config
- [ ] All unit and integration tests pass
- [ ] `just validate` passes

## Test Plan

- Unit: `cargo test daemon` — daemon manager, socket, jobs
- Integration: `assert_cmd` tests for `apmw --list-jobs`, `apmw --no-daemon`
- Lint: `just lint`

## Observability

- Daemon lifecycle events (start, stop, spawn) should be logged at `info` level
- Job lifecycle events (create, start, complete, fail, cancel) should be logged at `info` level
- Socket errors should be logged at `error` level

## Compliance

- ADR-20260607001 section 13 (daemon process support)
- ADR-20260607001 section 8 (signals and exit codes)
- ADR-20260607001 section 31 (signal-based config reload)
- rust-development-practices async-patterns.md

## Risks & Mitigations

- Risk: Windows named pipe API differs significantly from Unix sockets — Mitigation: Use conditional compilation with #[cfg(target_os = "windows")]
- Risk: Auto-spawn may race with concurrent CLI invocations — Mitigation: Use file-based PID lock

## Dependencies & Sequencing

- Depends on: None (foundation story)
- Unblocks: 02-001, 05-001, 06-001

## Definition of Done

- [ ] All verification commands pass
- [ ] Code, tests, docs updated; CI green; story file updated
- [ ] No files outside in-scope list are modified

## STOP Conditions

Stop and report if:
- tokio "full" does not support a required feature for MSRV 1.70
- Local socket IPC cannot be implemented cross-platform
- Signal handling is not supported on the target platform

## Maintenance Notes

- Future stories will register background job types (clone, scan, index) with the job manager
- Reviewers should check that the daemon handles concurrent jobs safely
- Socket security: ensure the socket is not world-writable

## Commit Conventions

- `feat(daemon): add tokio daemon skeleton with socket IPC and job manager`
