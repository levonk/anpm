//! Audit logging for apmw.
//!
//! Records every install/detect/clone/scan operation to an append-only JSONL
//! log at `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/audit.log`. Entries record the
//! timestamp, what was requested, what was done, terminal type, caller program,
//! and tools used. A configurable retention period auto-prunes old entries on
//! writer creation (ADR-20260607001 §34).

pub mod log;

pub use log::{
  audit_log_path, detect_caller_program, detect_terminal_type, AuditLogEntry, AuditLogWriter,
  TerminalType,
};
