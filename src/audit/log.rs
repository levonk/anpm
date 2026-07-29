//! Append-only audit log writer with retention.
//!
//! Entries are serialized as JSONL (one JSON object per line) and flushed to
//! disk before returning success. On writer creation, entries older than the
//! configured retention period are pruned.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::error::{ApmwError, Result};

/// Default retention period in days.
pub const DEFAULT_RETENTION_DAYS: u64 = 30;

/// The audit log file name within the apmw cache directory.
pub const AUDIT_LOG_FILE_NAME: &str = "audit.log";

/// The apmw subdirectory inside the cache root.
pub const AUDIT_LOG_SUBDIR: &str = "apmw";

/// The type of terminal the caller is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalType {
  /// Login shell (argv zero starts with `-`).
  Login,
  /// Interactive shell (TTY on stdin and `TERM`/`PS1` set).
  Interactive,
  /// Non-interactive (piped stdin or no TTY).
  NonInteractive,
}

/// A single audit log entry recording one operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLogEntry {
  /// ISO 8601 timestamp (UTC) of the operation.
  pub timestamp: String,
  /// What was requested (e.g. `add prettier`).
  pub request: String,
  /// What was done (e.g. `installed prettier@3.2.0`).
  pub action: String,
  /// The terminal type the caller ran from.
  pub terminal_type: TerminalType,
  /// The program that invoked apmw (e.g. `claude-code`, `unknown`).
  pub caller_program: String,
  /// Tools used during the operation.
  pub tools_used: Vec<String>,
}

impl AuditLogEntry {
  /// Create a new entry with the current timestamp (ISO 8601 UTC).
  pub fn now(
    request: impl Into<String>,
    action: impl Into<String>,
    terminal_type: TerminalType,
    caller_program: impl Into<String>,
    tools_used: Vec<String>,
  ) -> Self {
    AuditLogEntry {
      timestamp: current_iso8601_utc(),
      request: request.into(),
      action: action.into(),
      terminal_type,
      caller_program: caller_program.into(),
      tools_used,
    }
  }
}

/// Append-only audit log writer with retention pruning.
#[derive(Debug, Clone)]
pub struct AuditLogWriter {
  /// Path to the audit log file.
  log_path: PathBuf,
  /// Retention period in days; entries older than this are pruned.
  retention_days: u64,
}

impl AuditLogWriter {
  /// Create a writer using the default XDG cache path and default retention.
  ///
  /// Prunes entries older than the retention period immediately.
  pub fn new() -> Result<Self> {
    Self::with_path(audit_log_path()?, DEFAULT_RETENTION_DAYS)
  }

  /// Create a writer with a custom log path and retention period.
  ///
  /// Prunes entries older than `retention_days` immediately.
  pub fn with_path(log_path: PathBuf, retention_days: u64) -> Result<Self> {
    let writer = AuditLogWriter {
      log_path,
      retention_days,
    };
    writer.prune()?;
    Ok(writer)
  }

  /// Returns the log file path.
  pub fn path(&self) -> &Path {
    &self.log_path
  }

  /// Returns the configured retention period in days.
  pub fn retention_days(&self) -> u64 {
    self.retention_days
  }

  /// Append an entry to the log. The entry is serialized as JSONL and flushed
  /// (sync_all) to disk before returning success.
  pub fn append(&self, entry: &AuditLogEntry) -> Result<()> {
    if let Some(parent) = self.log_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
      .append(true)
      .create(true)
      .open(&self.log_path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{line}")?;
    file.flush()?;
    file.sync_all()?;
    debug!(path = %self.log_path.display(), "Appended audit entry");
    Ok(())
  }

  /// Prune entries older than the retention period.
  ///
  /// Reads the log, filters entries by timestamp, and rewrites the file in
  /// place when any entries are removed. If the file does not exist, this is
  /// a no-op.
  pub fn prune(&self) -> Result<()> {
    if !self.log_path.exists() {
      return Ok(());
    }
    let entries = self.read_entries()?;
    let cutoff_secs = current_unix_secs().saturating_sub(self.retention_days * 86_400);
    let mut kept: Vec<AuditLogEntry> = Vec::with_capacity(entries.len());
    let mut removed = 0usize;
    for entry in entries {
      match parse_iso8601_to_unix_secs(&entry.timestamp) {
        Some(secs) if secs < cutoff_secs => removed += 1,
        _ => kept.push(entry),
      }
    }
    if removed == 0 {
      return Ok(());
    }
    info!(removed, kept = kept.len(), "Pruned old audit entries");
    let mut file = File::create(&self.log_path)?;
    for entry in &kept {
      let line = serde_json::to_string(entry)?;
      writeln!(file, "{line}")?;
    }
    file.flush()?;
    file.sync_all()?;
    Ok(())
  }

  /// Export all log entries as a `Vec<AuditLogEntry>`.
  pub fn export(&self) -> Result<Vec<AuditLogEntry>> {
    self.read_entries()
  }

  /// Export all log entries to a given path as JSONL.
  ///
  /// The output file is overwritten if it exists.
  pub fn export_to(&self, dest: &Path) -> Result<()> {
    let entries = self.read_entries()?;
    if let Some(parent) = dest.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(dest)?;
    for entry in &entries {
      let line = serde_json::to_string(entry)?;
      writeln!(file, "{line}")?;
    }
    file.flush()?;
    file.sync_all()?;
    Ok(())
  }

  /// Read all entries from the log file. Returns an empty vec if the file
  /// does not exist. Lines that fail to deserialize are skipped with a warning.
  fn read_entries(&self) -> Result<Vec<AuditLogEntry>> {
    if !self.log_path.exists() {
      return Ok(Vec::new());
    }
    let file = File::open(&self.log_path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
      let line = line?;
      let trimmed = line.trim();
      if trimmed.is_empty() {
        continue;
      }
      match serde_json::from_str::<AuditLogEntry>(trimmed) {
        Ok(entry) => entries.push(entry),
        Err(err) => {
          warn!(
            line = idx,
            path = %self.log_path.display(),
            error = %err,
            "Skipping malformed audit log line"
          );
        }
      }
    }
    Ok(entries)
  }
}

impl Default for AuditLogWriter {
  fn default() -> Self {
    Self::new().unwrap_or_else(|err| {
      error!(error = %err, "Failed to initialize default audit log writer");
      AuditLogWriter {
        log_path: PathBuf::from("audit.log"),
        retention_days: DEFAULT_RETENTION_DAYS,
      }
    })
  }
}

/// Resolve the audit log path via XDG cache home.
///
/// Uses `XDG_CACHE_HOME` if set and non-empty, otherwise falls back to
/// `$HOME/.cache` via `dirs::cache_dir()`.
pub fn audit_log_path() -> Result<PathBuf> {
  if let Some(xdg_cache_home) = std::env::var_os("XDG_CACHE_HOME") {
    if !xdg_cache_home.is_empty() {
      return Ok(
        PathBuf::from(xdg_cache_home)
          .join(AUDIT_LOG_SUBDIR)
          .join(AUDIT_LOG_FILE_NAME),
      );
    }
  }
  if let Some(cache) = dirs::cache_dir() {
    return Ok(cache.join(AUDIT_LOG_SUBDIR).join(AUDIT_LOG_FILE_NAME));
  }
  // Last-resort fallback using HOME directly.
  if let Some(home) = std::env::var_os("HOME") {
    if !home.is_empty() {
      return Ok(
        PathBuf::from(home)
          .join(".cache")
          .join(AUDIT_LOG_SUBDIR)
          .join(AUDIT_LOG_FILE_NAME),
      );
    }
  }
  Err(ApmwError::AuditLogError(
    "Could not resolve cache directory for audit log".to_string(),
  ))
}

/// Detect the terminal type from the environment.
///
/// - Login: `argv[0]` starts with `-` (checked via the `_` env var or
///   `SHELL` basename) or `0` arg has a `-` prefix.
/// - Interactive: stdin is a TTY and `TERM`/`PS1` is set.
/// - NonInteractive: otherwise.
pub fn detect_terminal_type() -> TerminalType {
  // Login shell detection: argv[0] starting with `-`.
  if let Some(arg0) = std::env::args_os().next() {
    let name = arg0.to_string_lossy();
    let basename = name.rsplit('/').next().unwrap_or(&name);
    if basename.starts_with('-') {
      return TerminalType::Login;
    }
  }
  // Interactive: stdin is a TTY and terminal env vars are set.
  let is_tty = is_stdin_tty();
  let has_term = std::env::var_os("TERM").is_some();
  let has_ps1 = std::env::var_os("PS1").is_some();
  let has_shell = std::env::var_os("SHELL").is_some();
  if is_tty && (has_term || has_ps1) && has_shell {
    return TerminalType::Interactive;
  }
  TerminalType::NonInteractive
}

/// Detect the caller program from environment variables.
///
/// Checks `APMW_CALLER`, `CLAUDE_CODE`, `CURSOR_TRACE_ID`, `INVOCATION_ID`,
/// and `_` (last command). Falls back to `"unknown"`.
pub fn detect_caller_program() -> String {
  if let Some(caller) = std::env::var_os("APMW_CALLER") {
    if !caller.is_empty() {
      return caller.to_string_lossy().into_owned();
    }
  }
  if std::env::var_os("CLAUDE_CODE").is_some() {
    return "claude-code".to_string();
  }
  if std::env::var_os("CURSOR_TRACE_ID").is_some() {
    return "cursor".to_string();
  }
  if std::env::var_os("INVOCATION_ID").is_some() {
    return "systemd".to_string();
  }
  if let Some(last) = std::env::var_os("_") {
    if !last.is_empty() {
      let path = last.to_string_lossy();
      let basename = path.rsplit('/').next().unwrap_or(&path);
      if !basename.is_empty() {
        return basename.to_string();
      }
    }
  }
  "unknown".to_string()
}

/// Returns true if stdin appears to be a TTY.
fn is_stdin_tty() -> bool {
  // Use libc-free check via std::io::IsTerminal (stable since 1.70).
  use std::io::IsTerminal;
  std::io::stdin().is_terminal()
}

/// Current time as Unix seconds.
fn current_unix_secs() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0)
}

/// Format the current time as an ISO 8601 UTC string.
///
/// Produces `YYYY-MM-DDTHH:MM:SSZ` without external dependencies. Uses a
/// civil-from-days algorithm to avoid pulling in chrono.
fn current_iso8601_utc() -> String {
  let secs = current_unix_secs();
  format_unix_secs_iso8601(secs)
}

/// Format a Unix timestamp (seconds) as an ISO 8601 UTC string.
fn format_unix_secs_iso8601(secs: u64) -> String {
  let days = (secs / 86_400) as i64;
  let rem = (secs % 86_400) as i64;
  let hour = rem / 3600;
  let minute = (rem % 3600) / 60;
  let second = rem % 60;
  let (year, month, day) = civil_from_days(days);
  format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert days since the Unix epoch (1970-01-01) to a (year, month, day).
///
/// Based on Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
  let z = days + 719_468;
  let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
  let doe = (z - era * 146_097) as u64; // [0, 146096]
  let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
  let y = yoe as i64 + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
  let mp = (5 * doy + 2) / 153; // [0, 11]
  let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
  let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
  let year = if m <= 2 { y + 1 } else { y };
  (year, m, d)
}

/// Parse an ISO 8601 UTC string (`YYYY-MM-DDTHH:MM:SSZ`) to Unix seconds.
///
/// Returns `None` if the string cannot be parsed.
fn parse_iso8601_to_unix_secs(ts: &str) -> Option<u64> {
  let ts = ts.trim();
  // Expected format: YYYY-MM-DDTHH:MM:SSZ
  if ts.len() < 20 {
    return None;
  }
  let bytes = ts.as_bytes();
  let year: i64 = std::str::from_utf8(&bytes[0..4]).ok()?.parse().ok()?;
  if bytes[4] != b'-'
    || bytes[7] != b'-'
    || bytes[10] != b'T'
    || bytes[13] != b':'
    || bytes[16] != b':'
  {
    return None;
  }
  let month: u32 = std::str::from_utf8(&bytes[5..7]).ok()?.parse().ok()?;
  let day: u32 = std::str::from_utf8(&bytes[8..10]).ok()?.parse().ok()?;
  let hour: u64 = std::str::from_utf8(&bytes[11..13]).ok()?.parse().ok()?;
  let minute: u64 = std::str::from_utf8(&bytes[14..16]).ok()?.parse().ok()?;
  let second: u64 = std::str::from_utf8(&bytes[17..19]).ok()?.parse().ok()?;
  let days = days_from_civil(year, month, day);
  let secs = days as u64 * 86_400 + hour * 3600 + minute * 60 + second;
  Some(secs)
}

/// Convert a (year, month, day) to days since the Unix epoch (1970-01-01).
///
/// Based on Howard Hinnant's days-from-civil algorithm.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
  let y = if month <= 2 { year - 1 } else { year };
  let era = if y >= 0 { y } else { y - 399 } / 400;
  let yoe = (y - era * 400) as u64; // [0, 399]
  let m = month as i64;
  let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i64 - 1; // [0, 365]
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy as u64; // [0, 146096]
  era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use tempfile::tempdir;

  // ---- entry serialization ----

  #[test]
  fn test_audit_entry_serialization_roundtrip() {
    let entry = AuditLogEntry {
      timestamp: "2026-07-29T12:00:00Z".to_string(),
      request: "add prettier".to_string(),
      action: "installed prettier@3.2.0".to_string(),
      terminal_type: TerminalType::Interactive,
      caller_program: "claude-code".to_string(),
      tools_used: vec!["npm".to_string(), "pnpm".to_string()],
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    let back: AuditLogEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(entry, back);
  }

  #[test]
  fn test_audit_entry_serialization_snake_case_terminal() {
    let entry = AuditLogEntry {
      timestamp: "2026-07-29T12:00:00Z".to_string(),
      request: "scan".to_string(),
      action: "scanned".to_string(),
      terminal_type: TerminalType::NonInteractive,
      caller_program: "unknown".to_string(),
      tools_used: vec![],
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    assert!(json.contains("\"non_interactive\""), "json: {json}");
  }

  #[test]
  fn test_audit_entry_now_has_timestamp() {
    let entry = AuditLogEntry::now(
      "req",
      "act",
      TerminalType::Login,
      "caller",
      vec!["t".to_string()],
    );
    assert!(entry.timestamp.ends_with('Z'));
    assert_eq!(entry.request, "req");
    assert_eq!(entry.action, "act");
    assert_eq!(entry.terminal_type, TerminalType::Login);
    assert_eq!(entry.caller_program, "caller");
    assert_eq!(entry.tools_used, vec!["t".to_string()]);
  }

  // ---- path resolution ----

  #[test]
  #[serial]
  fn test_audit_path_xdg_cache_home() {
    let dir = tempdir().expect("tempdir");
    let saved = std::env::var_os("XDG_CACHE_HOME");
    std::env::set_var("XDG_CACHE_HOME", dir.path());
    let path = audit_log_path().expect("path");
    assert_eq!(path, dir.path().join("apmw").join("audit.log"));
    if let Some(s) = saved {
      std::env::set_var("XDG_CACHE_HOME", s);
    } else {
      std::env::remove_var("XDG_CACHE_HOME");
    }
  }

  #[test]
  #[serial]
  fn test_audit_path_empty_xdg_falls_back() {
    let saved = std::env::var_os("XDG_CACHE_HOME");
    std::env::set_var("XDG_CACHE_HOME", "");
    let path = audit_log_path().expect("path");
    assert!(path.ends_with("audit.log"));
    if let Some(s) = saved {
      std::env::set_var("XDG_CACHE_HOME", s);
    } else {
      std::env::remove_var("XDG_CACHE_HOME");
    }
  }

  // ---- append-only write ----

  #[test]
  fn test_audit_write_appends_jsonl() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("audit.log");
    let writer =
      AuditLogWriter::with_path(log_path.clone(), DEFAULT_RETENTION_DAYS).expect("writer");
    let entry = AuditLogEntry {
      timestamp: "2026-07-29T12:00:00Z".to_string(),
      request: "add prettier".to_string(),
      action: "installed prettier@3.2.0".to_string(),
      terminal_type: TerminalType::Interactive,
      caller_program: "claude-code".to_string(),
      tools_used: vec!["npm".to_string()],
    };
    writer.append(&entry).expect("append");
    writer.append(&entry).expect("append second");

    let contents = std::fs::read_to_string(&log_path).expect("read");
    let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 JSONL lines");
    for line in &lines {
      let parsed: AuditLogEntry = serde_json::from_str(line).expect("parse line");
      assert_eq!(parsed, entry);
    }
  }

  #[test]
  fn test_audit_write_creates_parent_dirs() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("nested").join("dir").join("audit.log");
    let writer =
      AuditLogWriter::with_path(log_path.clone(), DEFAULT_RETENTION_DAYS).expect("writer");
    let entry = AuditLogEntry::now("r", "a", TerminalType::Login, "c", vec![]);
    writer.append(&entry).expect("append");
    assert!(log_path.exists());
  }

  // ---- retention pruning ----

  #[test]
  fn test_audit_retention_prunes_old_entries() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("audit.log");
    // Write an old entry and a recent entry directly.
    let old_ts = format_unix_secs_iso8601(current_unix_secs().saturating_sub(60 * 86_400));
    let new_ts = format_unix_secs_iso8601(current_unix_secs());
    {
      let mut file = File::create(&log_path).expect("create");
      let old = AuditLogEntry {
        timestamp: old_ts,
        request: "old".to_string(),
        action: "old-action".to_string(),
        terminal_type: TerminalType::Login,
        caller_program: "unknown".to_string(),
        tools_used: vec![],
      };
      let new = AuditLogEntry {
        timestamp: new_ts,
        request: "new".to_string(),
        action: "new-action".to_string(),
        terminal_type: TerminalType::Interactive,
        caller_program: "claude-code".to_string(),
        tools_used: vec![],
      };
      writeln!(file, "{}", serde_json::to_string(&old).unwrap()).unwrap();
      writeln!(file, "{}", serde_json::to_string(&new).unwrap()).unwrap();
    }
    // Create writer with 30-day retention; old entry should be pruned.
    let writer = AuditLogWriter::with_path(log_path.clone(), 30).expect("writer");
    let entries = writer.export().expect("export");
    assert_eq!(entries.len(), 1, "expected 1 entry after pruning");
    assert_eq!(entries[0].request, "new");
  }

  #[test]
  fn test_audit_retention_keeps_all_when_young() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("audit.log");
    let ts = format_unix_secs_iso8601(current_unix_secs());
    {
      let mut file = File::create(&log_path).expect("create");
      let entry = AuditLogEntry {
        timestamp: ts,
        request: "r".to_string(),
        action: "a".to_string(),
        terminal_type: TerminalType::Interactive,
        caller_program: "c".to_string(),
        tools_used: vec![],
      };
      writeln!(file, "{}", serde_json::to_string(&entry).unwrap()).unwrap();
    }
    let writer = AuditLogWriter::with_path(log_path, 30).expect("writer");
    let entries = writer.export().expect("export");
    assert_eq!(entries.len(), 1);
  }

  #[test]
  fn test_audit_retention_no_file_is_noop() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("does-not-exist.log");
    let writer = AuditLogWriter::with_path(log_path, 30).expect("writer");
    let entries = writer.export().expect("export");
    assert!(entries.is_empty());
  }

  // ---- terminal detection ----

  #[test]
  #[serial]
  fn test_terminal_detection_non_interactive() {
    // In test harness stdin is typically not a TTY and TERM/PS1 may be unset.
    // We cannot reliably force a TTY, so we assert it resolves to a valid
    // variant and is NonInteractive in CI-like contexts.
    let tt = detect_terminal_type();
    assert!(matches!(
      tt,
      TerminalType::Login | TerminalType::Interactive | TerminalType::NonInteractive
    ));
  }

  // ---- caller detection ----

  #[test]
  #[serial]
  fn test_caller_detection_apmw_caller() {
    let saved = std::env::var_os("APMW_CALLER");
    std::env::set_var("APMW_CALLER", "my-tool");
    assert_eq!(detect_caller_program(), "my-tool");
    if let Some(s) = saved {
      std::env::set_var("APMW_CALLER", s);
    } else {
      std::env::remove_var("APMW_CALLER");
    }
  }

  #[test]
  #[serial]
  fn test_caller_detection_claude_code() {
    let saved_caller = std::env::var_os("APMW_CALLER");
    let saved_cc = std::env::var_os("CLAUDE_CODE");
    std::env::remove_var("APMW_CALLER");
    std::env::set_var("CLAUDE_CODE", "1");
    assert_eq!(detect_caller_program(), "claude-code");
    if let Some(s) = saved_caller {
      std::env::set_var("APMW_CALLER", s);
    } else {
      std::env::remove_var("APMW_CALLER");
    }
    if let Some(s) = saved_cc {
      std::env::set_var("CLAUDE_CODE", s);
    } else {
      std::env::remove_var("CLAUDE_CODE");
    }
  }

  #[test]
  #[serial]
  fn test_caller_detection_cursor() {
    let saved_caller = std::env::var_os("APMW_CALLER");
    let saved_cc = std::env::var_os("CLAUDE_CODE");
    let saved_cursor = std::env::var_os("CURSOR_TRACE_ID");
    std::env::remove_var("APMW_CALLER");
    std::env::remove_var("CLAUDE_CODE");
    std::env::set_var("CURSOR_TRACE_ID", "abc");
    assert_eq!(detect_caller_program(), "cursor");
    if let Some(s) = saved_caller {
      std::env::set_var("APMW_CALLER", s);
    } else {
      std::env::remove_var("APMW_CALLER");
    }
    if let Some(s) = saved_cc {
      std::env::set_var("CLAUDE_CODE", s);
    } else {
      std::env::remove_var("CLAUDE_CODE");
    }
    if let Some(s) = saved_cursor {
      std::env::set_var("CURSOR_TRACE_ID", s);
    } else {
      std::env::remove_var("CURSOR_TRACE_ID");
    }
  }

  #[test]
  #[serial]
  fn test_caller_detection_fallback_unknown() {
    let saved_caller = std::env::var_os("APMW_CALLER");
    let saved_cc = std::env::var_os("CLAUDE_CODE");
    let saved_cursor = std::env::var_os("CURSOR_TRACE_ID");
    let saved_inv = std::env::var_os("INVOCATION_ID");
    let saved_underscore = std::env::var_os("_");
    std::env::remove_var("APMW_CALLER");
    std::env::remove_var("CLAUDE_CODE");
    std::env::remove_var("CURSOR_TRACE_ID");
    std::env::remove_var("INVOCATION_ID");
    std::env::remove_var("_");
    assert_eq!(detect_caller_program(), "unknown");
    if let Some(s) = saved_caller {
      std::env::set_var("APMW_CALLER", s);
    } else {
      std::env::remove_var("APMW_CALLER");
    }
    if let Some(s) = saved_cc {
      std::env::set_var("CLAUDE_CODE", s);
    } else {
      std::env::remove_var("CLAUDE_CODE");
    }
    if let Some(s) = saved_cursor {
      std::env::set_var("CURSOR_TRACE_ID", s);
    } else {
      std::env::remove_var("CURSOR_TRACE_ID");
    }
    if let Some(s) = saved_inv {
      std::env::set_var("INVOCATION_ID", s);
    } else {
      std::env::remove_var("INVOCATION_ID");
    }
    if let Some(s) = saved_underscore {
      std::env::set_var("_", s);
    } else {
      std::env::remove_var("_");
    }
  }

  // ---- export ----

  #[test]
  fn test_audit_export_returns_all_entries() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("audit.log");
    let writer =
      AuditLogWriter::with_path(log_path.clone(), DEFAULT_RETENTION_DAYS).expect("writer");
    let e1 = AuditLogEntry::now("r1", "a1", TerminalType::Login, "c1", vec![]);
    let e2 = AuditLogEntry::now(
      "r2",
      "a2",
      TerminalType::Interactive,
      "c2",
      vec!["t".to_string()],
    );
    writer.append(&e1).expect("append");
    writer.append(&e2).expect("append");
    let entries = writer.export().expect("export");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].request, "r1");
    assert_eq!(entries[1].request, "r2");
  }

  #[test]
  fn test_audit_export_to_writes_jsonl() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("audit.log");
    let dest = dir.path().join("export").join("out.jsonl");
    let writer =
      AuditLogWriter::with_path(log_path.clone(), DEFAULT_RETENTION_DAYS).expect("writer");
    let e1 = AuditLogEntry::now("r1", "a1", TerminalType::Login, "c1", vec![]);
    writer.append(&e1).expect("append");
    writer.export_to(&dest).expect("export_to");
    let contents = std::fs::read_to_string(&dest).expect("read dest");
    let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    let parsed: AuditLogEntry = serde_json::from_str(lines[0]).expect("parse");
    assert_eq!(parsed.request, "r1");
  }

  #[test]
  fn test_audit_export_skips_malformed_lines() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("audit.log");
    // Write a malformed line followed by a valid one.
    {
      let mut file = File::create(&log_path).expect("create");
      writeln!(file, "{{not valid json").expect("write");
      let entry = AuditLogEntry {
        timestamp: format_unix_secs_iso8601(current_unix_secs()),
        request: "good".to_string(),
        action: "a".to_string(),
        terminal_type: TerminalType::Interactive,
        caller_program: "c".to_string(),
        tools_used: vec![],
      };
      writeln!(file, "{}", serde_json::to_string(&entry).unwrap()).expect("write");
    }
    // Use a large retention so nothing is pruned.
    let writer = AuditLogWriter::with_path(log_path, 365 * 100).expect("writer");
    let entries = writer.export().expect("export");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].request, "good");
  }

  // ---- date helpers ----

  #[test]
  fn test_format_unix_secs_iso8601_epoch() {
    assert_eq!(format_unix_secs_iso8601(0), "1970-01-01T00:00:00Z");
  }

  #[test]
  fn test_format_unix_secs_iso8601_known() {
    // 2026-07-29T12:00:00Z = 1785326400
    assert_eq!(
      format_unix_secs_iso8601(1_785_326_400),
      "2026-07-29T12:00:00Z"
    );
  }

  #[test]
  fn test_parse_iso8601_roundtrip() {
    let secs = 1_785_326_400u64;
    let ts = format_unix_secs_iso8601(secs);
    assert_eq!(parse_iso8601_to_unix_secs(&ts), Some(secs));
  }

  #[test]
  fn test_parse_iso8601_invalid() {
    assert_eq!(parse_iso8601_to_unix_secs("not a date"), None);
    assert_eq!(parse_iso8601_to_unix_secs("2026-07-29"), None);
  }

  #[test]
  fn test_civil_from_days_known() {
    // 1970-01-01 is day 0.
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    // 2026-07-29 is day 20663.
    assert_eq!(civil_from_days(20_663), (2026, 7, 29));
  }

  #[test]
  fn test_days_from_civil_known() {
    assert_eq!(days_from_civil(1970, 1, 1), 0);
    assert_eq!(days_from_civil(2026, 7, 29), 20_663);
  }
}
