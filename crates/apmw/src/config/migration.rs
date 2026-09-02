//! Config auto-migration (ADR-20260607001 §29).
//!
//! Detects legacy config files, creates a `.bak` backup, migrates the schema
//! to the current format, and validates the result.

use std::path::{Path, PathBuf};

use tracing::{info, warn};

use super::ApmwConfig;
use crate::error::Result;

/// The suffix appended to legacy config files when backing them up.
pub const BACKUP_SUFFIX: &str = ".bak";

/// The legacy config file name (pre-migration).
pub const LEGACY_CONFIG_FILE_NAME: &str = "apmw-config.toml";

/// Outcome of a migration attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationOutcome {
  /// No legacy config was found; nothing to migrate.
  NotNeeded,
  /// Migration succeeded. Contains the path of the `.bak` backup.
  Migrated { backup_path: PathBuf },
}

/// Detect whether a config file is in a legacy format.
///
/// A file is considered legacy if it contains keys that are no longer used or
/// if it uses the legacy file name (`apmw-config.toml`). This is a heuristic;
/// future schema changes should extend this check.
pub fn is_legacy_config(path: &Path) -> bool {
  // Legacy file name.
  if path.file_name().and_then(|n| n.to_str()) == Some(LEGACY_CONFIG_FILE_NAME) {
    return true;
  }
  // Legacy keys inside a current-named file.
  match std::fs::read_to_string(path) {
    Ok(contents) => {
      contents.contains("daemon_socket_path")
        || contents.contains("scan_timeout_secs")
        || contents.contains("telemetry_endpoint")
    }
    Err(_) => false,
  }
}

/// Migrate a legacy config file to the current schema.
///
/// Steps (ADR §29):
/// 1. Detect legacy config via [`is_legacy_config`].
/// 2. Create a `.bak` backup of the original file.
/// 3. Transform the content to the current schema.
/// 4. Validate the migrated config by parsing it.
/// 5. Write the migrated config to `target_path`.
///
/// Returns [`MigrationOutcome::NotNeeded`] when the source is not legacy.
pub fn migrate_config(source_path: &Path, target_path: &Path) -> Result<MigrationOutcome> {
  if !source_path.exists() {
    return Ok(MigrationOutcome::NotNeeded);
  }
  if !is_legacy_config(source_path) {
    info!(path = %source_path.display(), "Config is not legacy, skipping migration");
    return Ok(MigrationOutcome::NotNeeded);
  }

  info!(path = %source_path.display(), "Detected legacy config, starting migration");

  // 2. Create .bak backup.
  let backup_path = backup_path(source_path);
  std::fs::copy(source_path, &backup_path)?;
  info!(backup = %backup_path.display(), "Created backup of legacy config");

  // 3. Transform content.
  let contents = std::fs::read_to_string(source_path)?;
  let migrated = transform_legacy_content(&contents);

  // 4. Validate by parsing.
  let _validated: ApmwConfig = toml::from_str(&migrated).map_err(|err| {
    warn!(error = %err, "Migrated config failed validation");
    err
  })?;

  // 5. Write migrated config.
  if let Some(parent) = target_path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(target_path, &migrated)?;
  info!(path = %target_path.display(), "Wrote migrated config");

  Ok(MigrationOutcome::Migrated { backup_path })
}

/// Compute the backup path for a given source file.
pub fn backup_path(source: &Path) -> PathBuf {
  let mut s = source.as_os_str().to_os_string();
  s.push(BACKUP_SUFFIX);
  PathBuf::from(s)
}

/// Transform legacy config content into the current schema.
///
/// Drops deprecated keys (`daemon_socket_path`, `scan_timeout_secs`,
/// `telemetry_endpoint`) and keeps recognized keys. Unrecognized keys are
/// left in place (TOML deserialization with serde defaults will ignore them
/// when parsing into [`ApmwConfig`]).
pub fn transform_legacy_content(contents: &str) -> String {
  let deprecated_keys = [
    "daemon_socket_path",
    "scan_timeout_secs",
    "telemetry_endpoint",
  ];
  let mut kept_lines = Vec::new();
  for line in contents.lines() {
    let trimmed = line.trim_start();
    let should_drop = deprecated_keys.iter().any(|key| {
      trimmed.starts_with(&format!("{key} =")) || trimmed.starts_with(&format!("{key}="))
    });
    if !should_drop {
      kept_lines.push(line);
    }
  }
  kept_lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

  #[test]
  fn test_is_legacy_config_by_filename() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(LEGACY_CONFIG_FILE_NAME);
    std::fs::write(&path, "daemon_enabled = true\n").unwrap();
    assert!(is_legacy_config(&path));
  }

  #[test]
  fn test_is_legacy_config_by_deprecated_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("apmw.toml");
    std::fs::write(&path, "daemon_socket_path = \"/tmp/apmw.sock\"\n").unwrap();
    assert!(is_legacy_config(&path));
  }

  #[test]
  fn test_is_legacy_config_current_format() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("apmw.toml");
    std::fs::write(&path, "daemon_enabled = true\nmin_release_age_days = 2\n").unwrap();
    assert!(!is_legacy_config(&path));
  }

  #[test]
  fn test_is_legacy_config_missing_file() {
    let path = PathBuf::from("/nonexistent/apmw.toml");
    assert!(!is_legacy_config(&path));
  }

  #[test]
  fn test_backup_path() {
    let path = PathBuf::from("/home/user/apmw.toml");
    let backup = backup_path(&path);
    assert_eq!(backup, PathBuf::from("/home/user/apmw.toml.bak"));
  }

  #[test]
  fn test_transform_drops_deprecated_keys() {
    let input = "daemon_enabled = true\ndaemon_socket_path = \"/tmp/apmw.sock\"\n\
                 min_release_age_days = 2\nscan_timeout_secs = 30\ntelemetry_endpoint = \"x\"\n";
    let output = transform_legacy_content(input);
    assert!(output.contains("daemon_enabled = true"));
    assert!(output.contains("min_release_age_days = 2"));
    assert!(!output.contains("daemon_socket_path"));
    assert!(!output.contains("scan_timeout_secs"));
    assert!(!output.contains("telemetry_endpoint"));
  }

  #[test]
  fn test_transform_preserves_comments() {
    let input = "# a comment\ndaemon_enabled = true\n";
    let output = transform_legacy_content(input);
    assert!(output.contains("# a comment"));
  }

  #[test]
  fn test_migrate_config_not_needed_for_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("apmw-config.toml");
    let target = tmp.path().join("apmw.toml");
    let outcome = migrate_config(&source, &target).unwrap();
    assert_eq!(outcome, MigrationOutcome::NotNeeded);
  }

  #[test]
  fn test_migrate_config_not_needed_for_current() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("apmw.toml");
    std::fs::write(&source, "daemon_enabled = true\n").unwrap();
    let target = tmp.path().join("apmw_new.toml");
    let outcome = migrate_config(&source, &target).unwrap();
    assert_eq!(outcome, MigrationOutcome::NotNeeded);
  }

  #[test]
  fn test_migrate_config_legacy_filename() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join(LEGACY_CONFIG_FILE_NAME);
    std::fs::write(&source, "daemon_enabled = true\nmin_release_age_days = 3\n").unwrap();
    let target = tmp.path().join("apmw.toml");
    let outcome = migrate_config(&source, &target).unwrap();
    match outcome {
      MigrationOutcome::Migrated { backup_path } => {
        assert!(backup_path.exists());
        assert_eq!(backup_path, source.with_extension("toml.bak"));
        assert!(target.exists());
        let migrated = std::fs::read_to_string(&target).unwrap();
        assert!(migrated.contains("daemon_enabled = true"));
      }
      _ => panic!("expected Migrated outcome"),
    }
  }

  #[test]
  fn test_migrate_config_drops_deprecated_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join(LEGACY_CONFIG_FILE_NAME);
    let mut f = std::fs::File::create(&source).unwrap();
    writeln!(f, "daemon_enabled = true").unwrap();
    writeln!(f, "daemon_socket_path = \"/tmp/apmw.sock\"").unwrap();
    writeln!(f, "min_release_age_days = 5").unwrap();
    writeln!(f, "telemetry_endpoint = \"https://example.com\"").unwrap();
    let target = tmp.path().join("apmw.toml");
    let outcome = migrate_config(&source, &target).unwrap();
    assert!(matches!(outcome, MigrationOutcome::Migrated { .. }));
    let migrated = std::fs::read_to_string(&target).unwrap();
    assert!(migrated.contains("daemon_enabled = true"));
    assert!(migrated.contains("min_release_age_days = 5"));
    assert!(!migrated.contains("daemon_socket_path"));
    assert!(!migrated.contains("telemetry_endpoint"));
    // backup retains original content
    let backup = std::fs::read_to_string(backup_path(&source)).unwrap();
    assert!(backup.contains("daemon_socket_path"));
  }

  #[test]
  fn test_migrate_config_creates_target_parent_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join(LEGACY_CONFIG_FILE_NAME);
    std::fs::write(&source, "daemon_enabled = true\n").unwrap();
    let target = tmp.path().join("nested").join("dir").join("apmw.toml");
    let outcome = migrate_config(&source, &target).unwrap();
    assert!(matches!(outcome, MigrationOutcome::Migrated { .. }));
    assert!(target.exists());
  }

  #[test]
  fn test_migrate_config_invalid_migrated_content_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join(LEGACY_CONFIG_FILE_NAME);
    // Legacy file with a key that produces invalid TOML after transform is
    // unlikely; instead test that a legacy file containing invalid TOML for
    // our schema still errors during validation.
    std::fs::write(&source, "min_release_age_days = \"not-a-number\"\n").unwrap();
    let target = tmp.path().join("apmw.toml");
    let result = migrate_config(&source, &target);
    assert!(result.is_err());
    // backup should still have been created before validation
    assert!(backup_path(&source).exists());
  }

  #[test]
  fn test_migration_outcome_debug() {
    let outcome = MigrationOutcome::NotNeeded;
    let debug = format!("{:?}", outcome);
    assert!(debug.contains("NotNeeded"));
  }
}
