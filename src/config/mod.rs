//! Configuration management for apmw.
//!
//! Implements the 6-level config precedence chain from ADR-20260607001 §2:
//!   CLI args > env vars > local project config > user config (XDG) > system config > defaults
//!
//! Config files are TOML. On first run, a default config with all settings
//! commented out is created (ADR §3). Legacy configs are auto-migrated with a
//! `.bak` backup (ADR §29).

pub mod migration;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::error::{ApmwError, Result};

/// The apmw configuration file name.
pub const CONFIG_FILE_NAME: &str = "apmw.toml";

/// The system-level config directory.
pub const SYSTEM_CONFIG_DIR: &str = "/etc/apmw";

/// Environment variable overrides for config values.
///
/// These map directly to fields in [`ApmwConfig`] and take precedence over
/// file-based config (but are overridden by CLI args).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EnvOverrides {
  /// `APMW_DAEMON_ENABLED` — override daemon enabled flag.
  pub daemon_enabled: Option<bool>,
  /// `APMW_MIN_RELEASE_AGE_DAYS` — override minimum release age.
  pub min_release_age_days: Option<u32>,
  /// `APMW_AGENT_MODE` — override agent mode.
  pub agent_mode: Option<bool>,
  /// `APMW_TELEMETRY` — override telemetry enabled flag.
  pub telemetry: Option<bool>,
}

/// CLI argument overrides for config values.
///
/// CLI args have the highest precedence in the config chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CliOverrides {
  /// Override daemon enabled flag.
  pub daemon_enabled: Option<bool>,
  /// Override minimum release age.
  pub min_release_age_days: Option<u32>,
  /// Override agent mode.
  pub agent_mode: Option<bool>,
  /// Override config file path.
  pub config_file: Option<PathBuf>,
  /// Override telemetry enabled flag.
  pub telemetry: Option<bool>,
}

/// The full apmw configuration.
///
/// Future stories will add fields as features are implemented. All fields
/// default to sensible values so a freshly-initialized config (all settings
/// commented out) resolves to [`ApmwConfig::default`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApmwConfig {
  /// Whether the daemon is enabled (ADR §13).
  #[serde(default)]
  pub daemon_enabled: bool,
  /// Minimum release age in days before a version is considered (supply-chain).
  #[serde(default = "default_min_release_age_days")]
  pub min_release_age_days: u32,
  /// Whether agent mode (AXI/TOON) is enabled (ADR §36-45).
  #[serde(default)]
  pub agent_mode: bool,
  /// Whether anonymized telemetry collection is enabled (PRD FR-6).
  #[serde(default = "default_telemetry_enabled")]
  pub telemetry: bool,
  /// The telemetry collection endpoint URL (PRD FR-6).
  #[serde(default = "default_telemetry_endpoint")]
  pub telemetry_endpoint: String,
}

impl Default for ApmwConfig {
  fn default() -> Self {
    ApmwConfig {
      daemon_enabled: false,
      min_release_age_days: default_min_release_age_days(),
      agent_mode: false,
      telemetry: default_telemetry_enabled(),
      telemetry_endpoint: default_telemetry_endpoint(),
    }
  }
}

fn default_min_release_age_days() -> u32 {
  2
}

/// Default telemetry enabled state: `true` (opt-out model).
fn default_telemetry_enabled() -> bool {
  true
}

/// Default telemetry endpoint URL.
fn default_telemetry_endpoint() -> String {
  "https://telemetry.apmw.dev/v1/event".to_string()
}

/// Resolved config file paths following the XDG Base Directory Specification.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigPaths {
  /// System-level config path (`/etc/apmw/apmw.toml`).
  pub system: PathBuf,
  /// User-level config path (`$XDG_CONFIG_HOME/apmw/apmw.toml` or `~/.config/apmw/apmw.toml`).
  pub user: PathBuf,
  /// Local project config path (`./.apmw.toml`).
  pub local: PathBuf,
}

impl ConfigPaths {
  /// Resolve config paths using XDG environment variables.
  ///
  /// - `XDG_CONFIG_HOME` — user config root (defaults to `$HOME/.config`)
  /// - `HOME` — fallback for user config root
  pub fn resolve() -> Self {
    let user = user_config_path();
    let local = PathBuf::from(".apmw.toml");
    let system = PathBuf::from(SYSTEM_CONFIG_DIR).join(CONFIG_FILE_NAME);
    ConfigPaths {
      system,
      user,
      local,
    }
  }

  /// Resolve config paths with an explicit working directory for the local config.
  pub fn resolve_with_cwd(cwd: &Path) -> Self {
    let user = user_config_path();
    let local = cwd.join(".apmw.toml");
    let system = PathBuf::from(SYSTEM_CONFIG_DIR).join(CONFIG_FILE_NAME);
    ConfigPaths {
      system,
      user,
      local,
    }
  }
}

/// Compute the user-level config path via XDG or HOME fallback.
pub fn user_config_path() -> PathBuf {
  if let Some(xdg_config_home) = std::env::var_os("XDG_CONFIG_HOME") {
    if !xdg_config_home.is_empty() {
      return PathBuf::from(xdg_config_home)
        .join("apmw")
        .join(CONFIG_FILE_NAME);
    }
  }
  if let Some(home) = dirs::config_dir() {
    return home.join("apmw").join(CONFIG_FILE_NAME);
  }
  // Last-resort fallback using HOME directly.
  if let Some(home) = std::env::var_os("HOME") {
    if !home.is_empty() {
      return PathBuf::from(home)
        .join(".config")
        .join("apmw")
        .join(CONFIG_FILE_NAME);
    }
  }
  PathBuf::from(".config").join("apmw").join(CONFIG_FILE_NAME)
}

/// Load environment-variable overrides from the process environment.
pub fn load_env_overrides() -> EnvOverrides {
  EnvOverrides {
    daemon_enabled: std::env::var("APMW_DAEMON_ENABLED")
      .ok()
      .and_then(|v| parse_bool(&v)),
    min_release_age_days: std::env::var("APMW_MIN_RELEASE_AGE_DAYS")
      .ok()
      .and_then(|v| v.parse::<u32>().ok()),
    agent_mode: std::env::var("APMW_AGENT_MODE")
      .ok()
      .and_then(|v| parse_bool(&v)),
    telemetry: std::env::var("APMW_TELEMETRY")
      .ok()
      .and_then(|v| parse_bool(&v)),
  }
}

fn parse_bool(value: &str) -> Option<bool> {
  match value.to_ascii_lowercase().as_str() {
    "1" | "true" | "yes" | "on" => Some(true),
    "0" | "false" | "no" | "off" => Some(false),
    _ => None,
  }
}

/// A partial config where all fields are optional, used for merging file-based
/// configs. Only fields present in the TOML are `Some`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct PartialApmwConfig {
  #[serde(default)]
  daemon_enabled: Option<bool>,
  #[serde(default)]
  min_release_age_days: Option<u32>,
  #[serde(default)]
  agent_mode: Option<bool>,
  #[serde(default)]
  telemetry: Option<bool>,
  #[serde(default)]
  telemetry_endpoint: Option<String>,
}

impl PartialApmwConfig {
  /// Merge `source` into `target`, with `source` taking precedence on set fields.
  fn merge_into(target: &mut PartialApmwConfig, source: &PartialApmwConfig) {
    if source.daemon_enabled.is_some() {
      target.daemon_enabled = source.daemon_enabled;
    }
    if source.min_release_age_days.is_some() {
      target.min_release_age_days = source.min_release_age_days;
    }
    if source.agent_mode.is_some() {
      target.agent_mode = source.agent_mode;
    }
    if source.telemetry.is_some() {
      target.telemetry = source.telemetry;
    }
    if source.telemetry_endpoint.is_some() {
      target.telemetry_endpoint = source.telemetry_endpoint.clone();
    }
  }

  /// Apply this partial config onto a fully-resolved [`ApmwConfig`].
  fn apply_to(&self, config: &mut ApmwConfig) {
    if let Some(v) = self.daemon_enabled {
      config.daemon_enabled = v;
    }
    if let Some(v) = self.min_release_age_days {
      config.min_release_age_days = v;
    }
    if let Some(v) = self.agent_mode {
      config.agent_mode = v;
    }
    if let Some(v) = self.telemetry {
      config.telemetry = v;
    }
    if let Some(ref v) = self.telemetry_endpoint {
      config.telemetry_endpoint = v.clone();
    }
  }
}

/// Read a TOML config file if it exists, returning `None` when missing.
fn read_config_file(path: &Path) -> Result<Option<PartialApmwConfig>> {
  match std::fs::read_to_string(path) {
    Ok(contents) => {
      let cfg: PartialApmwConfig = toml::from_str(&contents)?;
      debug!(path = %path.display(), "Loaded config file");
      Ok(Some(cfg))
    }
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
      debug!(path = %path.display(), "Config file not found, skipping");
      Ok(None)
    }
    Err(err) => {
      error!(path = %path.display(), error = %err, "Failed to read config file");
      Err(ApmwError::Io(err))
    }
  }
}

/// Load and merge configuration following the 6-level precedence chain.
///
/// Precedence (highest to lowest):
/// 1. CLI args (`cli`)
/// 2. Env vars (`env`)
/// 3. Local project config (`./.apmw.toml`)
/// 4. User config (XDG)
/// 5. System config (`/etc/apmw/apmw.toml`)
/// 6. Defaults ([`ApmwConfig::default`])
pub fn load(cli: &CliOverrides, env: &EnvOverrides, paths: &ConfigPaths) -> Result<ApmwConfig> {
  // Accumulate file-based overrides from lowest to highest file precedence.
  let mut merged: PartialApmwConfig = PartialApmwConfig::default();

  // 5. System config (lowest file precedence).
  if let Some(system_cfg) = read_config_file(&paths.system)? {
    PartialApmwConfig::merge_into(&mut merged, &system_cfg);
  }

  // 4. User config (XDG).
  if let Some(user_cfg) = read_config_file(&paths.user)? {
    PartialApmwConfig::merge_into(&mut merged, &user_cfg);
  }

  // 3. Local project config.
  if let Some(local_cfg) = read_config_file(&paths.local)? {
    PartialApmwConfig::merge_into(&mut merged, &local_cfg);
  }

  // Start from defaults and apply file-based overrides.
  let mut config = ApmwConfig::default();
  merged.apply_to(&mut config);

  // 2. Env vars.
  apply_env(&mut config, env);

  // 1. CLI args (highest precedence).
  apply_cli(&mut config, cli);

  debug!(?config, "Resolved final config");
  Ok(config)
}

fn apply_env(config: &mut ApmwConfig, env: &EnvOverrides) {
  if let Some(v) = env.daemon_enabled {
    config.daemon_enabled = v;
  }
  if let Some(v) = env.min_release_age_days {
    config.min_release_age_days = v;
  }
  if let Some(v) = env.agent_mode {
    config.agent_mode = v;
  }
  if let Some(v) = env.telemetry {
    config.telemetry = v;
  }
}

fn apply_cli(config: &mut ApmwConfig, cli: &CliOverrides) {
  if let Some(v) = cli.daemon_enabled {
    config.daemon_enabled = v;
  }
  if let Some(v) = cli.min_release_age_days {
    config.min_release_age_days = v;
  }
  if let Some(v) = cli.agent_mode {
    config.agent_mode = v;
  }
  if let Some(v) = cli.telemetry {
    config.telemetry = v;
  }
}

/// The default config file content written on first run.
///
/// All settings are commented out so the user opts in explicitly. Uncommenting
/// a line overrides the built-in default.
pub const DEFAULT_CONFIG_CONTENT: &str = "\
# apmw configuration file
#
# All settings are commented out. Uncomment and edit to override the defaults.
# See ADR-20260607001 for the full config precedence chain:
#   CLI args > env vars > local project config > user config (XDG) > system config > defaults

# Whether the daemon is enabled (ADR §13).
# daemon_enabled = false

# Minimum release age in days before a version is considered (supply-chain defense).
# min_release_age_days = 2

# Whether agent mode (AXI/TOON output) is enabled (ADR §36-45).
# agent_mode = false

# Whether anonymized telemetry collection is enabled (PRD FR-6).
# Set to false to disable telemetry. See --telemetry-preview for the payload.
# telemetry = true

# The telemetry collection endpoint URL (PRD FR-6).
# telemetry_endpoint = \"https://telemetry.apmw.dev/v1/event\"
";

/// Initialize a config file on first run.
///
/// Creates the parent directory (if needed) and writes a default config with
/// all settings commented out. Returns `Ok(true)` if the file was created,
/// `Ok(false)` if it already existed.
pub fn initialize_config_file(path: &Path) -> Result<bool> {
  if path.exists() {
    debug!(path = %path.display(), "Config file already exists, skipping init");
    return Ok(false);
  }
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(path, DEFAULT_CONFIG_CONTENT)?;
  debug!(path = %path.display(), "Created default config file");
  Ok(true)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serial_test::serial;
  use std::io::Write;

  #[test]
  fn test_default_config_values() {
    let cfg = ApmwConfig::default();
    assert!(!cfg.daemon_enabled);
    assert_eq!(cfg.min_release_age_days, 2);
    assert!(!cfg.agent_mode);
  }

  #[test]
  fn test_config_paths_resolve() {
    let paths = ConfigPaths::resolve();
    assert!(paths.system.to_string_lossy().contains("apmw"));
    assert!(paths.user.to_string_lossy().contains("apmw"));
    assert!(paths.local.to_string_lossy().contains(".apmw.toml"));
  }

  #[test]
  fn test_config_paths_resolve_with_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::resolve_with_cwd(tmp.path());
    assert_eq!(paths.local, tmp.path().join(".apmw.toml"));
  }

  #[test]
  #[serial]
  fn test_user_config_path_uses_xdg_config_home() {
    let tmp = tempfile::tempdir().unwrap();
    let xdg = tmp.path().join("xdg-config");
    std::env::set_var("XDG_CONFIG_HOME", &xdg);
    let path = user_config_path();
    assert_eq!(path, xdg.join("apmw").join(CONFIG_FILE_NAME));
    std::env::remove_var("XDG_CONFIG_HOME");
  }

  #[test]
  #[serial]
  fn test_user_config_path_falls_back_to_dirs() {
    std::env::remove_var("XDG_CONFIG_HOME");
    let path = user_config_path();
    // Should resolve to something containing apmw/apmw.toml
    assert!(path.to_string_lossy().contains("apmw"));
  }

  #[test]
  #[serial]
  fn test_load_env_overrides_parses_bools() {
    std::env::set_var("APMW_DAEMON_ENABLED", "true");
    std::env::set_var("APMW_MIN_RELEASE_AGE_DAYS", "7");
    std::env::set_var("APMW_AGENT_MODE", "1");
    std::env::set_var("APMW_TELEMETRY", "false");
    let env = load_env_overrides();
    assert_eq!(env.daemon_enabled, Some(true));
    assert_eq!(env.min_release_age_days, Some(7));
    assert_eq!(env.agent_mode, Some(true));
    assert_eq!(env.telemetry, Some(false));
    std::env::remove_var("APMW_DAEMON_ENABLED");
    std::env::remove_var("APMW_MIN_RELEASE_AGE_DAYS");
    std::env::remove_var("APMW_AGENT_MODE");
    std::env::remove_var("APMW_TELEMETRY");
  }

  #[test]
  #[serial]
  fn test_load_env_overrides_invalid_bool() {
    std::env::set_var("APMW_DAEMON_ENABLED", "maybe");
    let env = load_env_overrides();
    assert_eq!(env.daemon_enabled, None);
    std::env::remove_var("APMW_DAEMON_ENABLED");
  }

  #[test]
  fn test_parse_bool_variants() {
    assert_eq!(parse_bool("true"), Some(true));
    assert_eq!(parse_bool("FALSE"), Some(false));
    assert_eq!(parse_bool("yes"), Some(true));
    assert_eq!(parse_bool("no"), Some(false));
    assert_eq!(parse_bool("on"), Some(true));
    assert_eq!(parse_bool("off"), Some(false));
    assert_eq!(parse_bool("1"), Some(true));
    assert_eq!(parse_bool("0"), Some(false));
    assert_eq!(parse_bool("maybe"), None);
  }

  #[test]
  fn test_read_config_file_missing_returns_none() {
    let path = PathBuf::from("/nonexistent/path/apmw.toml");
    let result = read_config_file(&path).unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_read_config_file_valid() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("apmw.toml");
    std::fs::write(&path, "daemon_enabled = true\nmin_release_age_days = 5\n").unwrap();
    let cfg = read_config_file(&path).unwrap().unwrap();
    assert_eq!(cfg.daemon_enabled, Some(true));
    assert_eq!(cfg.min_release_age_days, Some(5));
  }

  #[test]
  fn test_read_config_file_invalid_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("apmw.toml");
    std::fs::write(&path, "not valid toml = ").unwrap();
    let result = read_config_file(&path);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ApmwError::Toml(_)));
  }

  #[test]
  fn test_load_defaults_when_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths {
      system: tmp.path().join("system.toml"),
      user: tmp.path().join("user.toml"),
      local: tmp.path().join("local.toml"),
    };
    let cli = CliOverrides::default();
    let env = EnvOverrides::default();
    let cfg = load(&cli, &env, &paths).unwrap();
    assert_eq!(cfg, ApmwConfig::default());
  }

  #[test]
  fn test_load_system_config_overrides_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let system_path = tmp.path().join("system.toml");
    std::fs::write(&system_path, "daemon_enabled = true\n").unwrap();
    let paths = ConfigPaths {
      system: system_path,
      user: tmp.path().join("user.toml"),
      local: tmp.path().join("local.toml"),
    };
    let cli = CliOverrides::default();
    let env = EnvOverrides::default();
    let cfg = load(&cli, &env, &paths).unwrap();
    assert!(cfg.daemon_enabled);
  }

  #[test]
  fn test_load_user_overrides_system() {
    let tmp = tempfile::tempdir().unwrap();
    let system_path = tmp.path().join("system.toml");
    std::fs::write(
      &system_path,
      "daemon_enabled = true\nmin_release_age_days = 10\n",
    )
    .unwrap();
    let user_path = tmp.path().join("user.toml");
    std::fs::write(&user_path, "min_release_age_days = 3\n").unwrap();
    let paths = ConfigPaths {
      system: system_path,
      user: user_path,
      local: tmp.path().join("local.toml"),
    };
    let cli = CliOverrides::default();
    let env = EnvOverrides::default();
    let cfg = load(&cli, &env, &paths).unwrap();
    // user overrides system for min_release_age_days
    assert_eq!(cfg.min_release_age_days, 3);
    // system value preserved since user didn't set it
    assert!(cfg.daemon_enabled);
  }

  #[test]
  fn test_load_local_overrides_user() {
    let tmp = tempfile::tempdir().unwrap();
    let user_path = tmp.path().join("user.toml");
    std::fs::write(&user_path, "daemon_enabled = true\nagent_mode = true\n").unwrap();
    let local_path = tmp.path().join("local.toml");
    std::fs::write(&local_path, "agent_mode = false\n").unwrap();
    let paths = ConfigPaths {
      system: tmp.path().join("system.toml"),
      user: user_path,
      local: local_path,
    };
    let cli = CliOverrides::default();
    let env = EnvOverrides::default();
    let cfg = load(&cli, &env, &paths).unwrap();
    assert!(cfg.daemon_enabled); // from user
    assert!(!cfg.agent_mode); // local overrides user
  }

  #[test]
  fn test_load_env_overrides_files() {
    let tmp = tempfile::tempdir().unwrap();
    let local_path = tmp.path().join("local.toml");
    std::fs::write(&local_path, "daemon_enabled = true\n").unwrap();
    let paths = ConfigPaths {
      system: tmp.path().join("system.toml"),
      user: tmp.path().join("user.toml"),
      local: local_path,
    };
    let cli = CliOverrides::default();
    let env = EnvOverrides {
      daemon_enabled: Some(false),
      min_release_age_days: None,
      agent_mode: None,
      telemetry: None,
    };
    let cfg = load(&cli, &env, &paths).unwrap();
    assert!(!cfg.daemon_enabled); // env overrides local
  }

  #[test]
  fn test_load_cli_overrides_env() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths {
      system: tmp.path().join("system.toml"),
      user: tmp.path().join("user.toml"),
      local: tmp.path().join("local.toml"),
    };
    let cli = CliOverrides {
      daemon_enabled: Some(true),
      min_release_age_days: Some(14),
      agent_mode: Some(true),
      config_file: None,
      telemetry: None,
    };
    let env = EnvOverrides {
      daemon_enabled: Some(false),
      min_release_age_days: Some(1),
      agent_mode: Some(false),
      telemetry: None,
    };
    let cfg = load(&cli, &env, &paths).unwrap();
    assert!(cfg.daemon_enabled);
    assert_eq!(cfg.min_release_age_days, 14);
    assert!(cfg.agent_mode);
  }

  #[test]
  fn test_initialize_config_file_creates_new() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("subdir").join("apmw.toml");
    let created = initialize_config_file(&path).unwrap();
    assert!(created);
    assert!(path.exists());
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("# apmw configuration file"));
    assert!(contents.contains("# daemon_enabled = false"));
    assert!(contents.contains("# min_release_age_days = 2"));
    assert!(contents.contains("# agent_mode = false"));
  }

  #[test]
  fn test_initialize_config_file_skips_existing() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("apmw.toml");
    std::fs::write(&path, "daemon_enabled = true\n").unwrap();
    let created = initialize_config_file(&path).unwrap();
    assert!(!created);
    // content unchanged
    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "daemon_enabled = true\n");
  }

  #[test]
  fn test_default_config_content_is_commented() {
    // No uncommented setting keys should appear.
    assert!(!DEFAULT_CONFIG_CONTENT.contains("\ndaemon_enabled ="));
    assert!(!DEFAULT_CONFIG_CONTENT.contains("\nmin_release_age_days ="));
    assert!(!DEFAULT_CONFIG_CONTENT.contains("\nagent_mode ="));
    assert!(!DEFAULT_CONFIG_CONTENT.contains("\ntelemetry ="));
    assert!(!DEFAULT_CONFIG_CONTENT.contains("\ntelemetry_endpoint ="));
  }

  #[test]
  fn test_config_serialize_deserialize_roundtrip() {
    let cfg = ApmwConfig {
      daemon_enabled: true,
      min_release_age_days: 5,
      agent_mode: true,
      telemetry: true,
      telemetry_endpoint: "https://example.com/v1/event".to_string(),
    };
    let toml_str = toml::to_string(&cfg).unwrap();
    let parsed: ApmwConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(cfg, parsed);
  }

  #[test]
  fn test_config_partial_toml_uses_defaults() {
    let toml_str = "daemon_enabled = true\n";
    let cfg: ApmwConfig = toml::from_str(toml_str).unwrap();
    assert!(cfg.daemon_enabled);
    assert_eq!(cfg.min_release_age_days, 2); // default
    assert!(!cfg.agent_mode); // default
  }

  #[test]
  fn test_merge_into_preserves_unset_fields() {
    let mut target = PartialApmwConfig {
      daemon_enabled: Some(true),
      min_release_age_days: Some(10),
      agent_mode: Some(true),
      telemetry: Some(true),
      telemetry_endpoint: Some("https://a.com".to_string()),
    };
    let source = PartialApmwConfig {
      daemon_enabled: Some(false),
      min_release_age_days: None, // unset, should not override
      agent_mode: Some(false),
      telemetry: None,
      telemetry_endpoint: None,
    };
    PartialApmwConfig::merge_into(&mut target, &source);
    assert_eq!(target.daemon_enabled, Some(false));
    assert_eq!(target.min_release_age_days, Some(10)); // preserved
    assert_eq!(target.agent_mode, Some(false));
  }

  #[test]
  fn test_apply_cli_overrides() {
    let mut config = ApmwConfig::default();
    let cli = CliOverrides {
      daemon_enabled: Some(true),
      min_release_age_days: Some(30),
      agent_mode: Some(true),
      config_file: None,
      telemetry: None,
    };
    apply_cli(&mut config, &cli);
    assert!(config.daemon_enabled);
    assert_eq!(config.min_release_age_days, 30);
    assert!(config.agent_mode);
  }

  #[test]
  fn test_apply_env_overrides() {
    let mut config = ApmwConfig::default();
    let env = EnvOverrides {
      daemon_enabled: Some(true),
      min_release_age_days: Some(9),
      agent_mode: None,
      telemetry: None,
    };
    apply_env(&mut config, &env);
    assert!(config.daemon_enabled);
    assert_eq!(config.min_release_age_days, 9);
    assert!(!config.agent_mode);
  }

  #[test]
  fn test_full_precedence_chain() {
    let tmp = tempfile::tempdir().unwrap();
    // System: daemon_enabled=true, min_release_age_days=1
    let system_path = tmp.path().join("system.toml");
    let mut f = std::fs::File::create(&system_path).unwrap();
    writeln!(f, "daemon_enabled = true").unwrap();
    writeln!(f, "min_release_age_days = 1").unwrap();
    // User: min_release_age_days=5
    let user_path = tmp.path().join("user.toml");
    std::fs::write(&user_path, "min_release_age_days = 5\n").unwrap();
    // Local: agent_mode=true
    let local_path = tmp.path().join("local.toml");
    std::fs::write(&local_path, "agent_mode = true\n").unwrap();
    let paths = ConfigPaths {
      system: system_path,
      user: user_path,
      local: local_path,
    };
    // Env: min_release_age_days=7
    let env = EnvOverrides {
      daemon_enabled: None,
      min_release_age_days: Some(7),
      agent_mode: None,
      telemetry: None,
    };
    // CLI: daemon_enabled=false (highest)
    let cli = CliOverrides {
      daemon_enabled: Some(false),
      min_release_age_days: None,
      agent_mode: None,
      config_file: None,
      telemetry: None,
    };
    let cfg = load(&cli, &env, &paths).unwrap();
    assert!(!cfg.daemon_enabled); // CLI wins
    assert_eq!(cfg.min_release_age_days, 7); // env wins over files
    assert!(cfg.agent_mode); // local wins
  }
}
