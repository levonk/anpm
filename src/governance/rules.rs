//! Governance rule definitions — prefer, force, block, eject.
//!
//! Each governance type describes how apmw should treat a non-canonical package
//! manager within an ecosystem:
//!
//! | Type    | Behaviour                                                              |
//! |---------|------------------------------------------------------------------------|
//! | `prefer`| Soft guidance — warn the user and delegate to the canonical manager.  |
//! | `force` | Strict replacement — intercept calls and route through the canonical. |
//! | `block` | Hard block — error-exiting wrapper that prevents use entirely.        |
//! | `eject` | Remove and block — remove the non-canonical manager and block future. |
//!
//! Per-tool governance settings are read from TOML config under
//! `[governance.<tool>]` (e.g. `[governance.pip] type = "force"`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors produced while parsing or validating governance rules.
#[derive(Error, Debug)]
pub enum RuleError {
  #[error("unknown governance type '{0}' (expected prefer, force, block, or eject)")]
  UnknownType(String),

  #[error("invalid tool name '{0}': must be a non-empty alphanumeric identifier")]
  InvalidToolName(String),
}

/// The four governance types (PRD FR-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GovernanceType {
  /// Soft guidance — warn the user and delegate to the canonical manager if
  /// installed.
  Prefer,
  /// Strict replacement — intercept calls to the non-canonical manager and
  /// route through the canonical one via a wrapper.
  Force,
  /// Hard block — error-exiting wrapper that prevents use of the non-canonical
  /// manager entirely.
  Block,
  /// Remove and block — remove the non-canonical manager from the project and
  /// block future use.
  Eject,
}

impl GovernanceType {
  /// Returns the string label used in config and the spec.
  pub fn as_str(self) -> &'static str {
    match self {
      GovernanceType::Prefer => "prefer",
      GovernanceType::Force => "force",
      GovernanceType::Block => "block",
      GovernanceType::Eject => "eject",
    }
  }

  /// Parse a governance type from a string (case-insensitive).
  pub fn parse(s: &str) -> std::result::Result<Self, RuleError> {
    match s.to_ascii_lowercase().as_str() {
      "prefer" => Ok(GovernanceType::Prefer),
      "force" => Ok(GovernanceType::Force),
      "block" => Ok(GovernanceType::Block),
      "eject" => Ok(GovernanceType::Eject),
      other => Err(RuleError::UnknownType(other.to_string())),
    }
  }

  /// Returns `true` if this governance type uses a wrapper/shim.
  pub fn uses_wrapper(self) -> bool {
    matches!(self, GovernanceType::Force | GovernanceType::Block)
  }
}

impl std::fmt::Display for GovernanceType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// A single governance rule binding a tool (package manager) to a governance
/// type with an optional message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceRule {
  /// The non-canonical package manager name (e.g. `pip`, `npm`).
  pub tool: String,
  /// The governance type to apply.
  #[serde(rename = "type")]
  pub governance_type: GovernanceType,
  /// Optional human-readable explanation shown to the user.
  #[serde(default)]
  pub message: Option<String>,
}

impl GovernanceRule {
  /// Create a new governance rule.
  pub fn new(tool: impl Into<String>, governance_type: GovernanceType) -> Self {
    GovernanceRule {
      tool: tool.into(),
      governance_type,
      message: None,
    }
  }

  /// Attach a human-readable message to the rule.
  pub fn with_message(mut self, msg: impl Into<String>) -> Self {
    self.message = Some(msg.into());
    self
  }
}

/// Per-tool governance configuration parsed from TOML.
///
/// Example TOML:
///
/// ```toml
/// [governance.pip]
/// type = "force"
/// message = "Use uv instead of pip"
///
/// [governance.npm]
/// type = "block"
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceConfig {
  /// Map of tool name → per-tool governance settings.
  #[serde(default)]
  pub tools: HashMap<String, ToolGovernanceSetting>,
}

/// Settings for a single tool under `[governance.<tool>]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolGovernanceSetting {
  /// The governance type (`prefer`, `force`, `block`, `eject`).
  #[serde(rename = "type")]
  pub governance_type: GovernanceType,
  /// Optional human-readable message.
  #[serde(default)]
  pub message: Option<String>,
}

impl GovernanceConfig {
  /// Parse governance config from a TOML string.
  pub fn from_toml(toml_str: &str) -> std::result::Result<Self, RuleError> {
    let cfg: GovernanceConfig = toml::from_str(toml_str)
      .map_err(|e| RuleError::InvalidToolName(format!("TOML parse error: {e}")))?;
    Ok(cfg)
  }

  /// Convert the config into a vector of [`GovernanceRule`]s.
  pub fn to_rules(&self) -> Vec<GovernanceRule> {
    self
      .tools
      .iter()
      .map(|(tool, setting)| {
        let mut rule = GovernanceRule::new(tool.clone(), setting.governance_type);
        rule.message = setting.message.clone();
        rule
      })
      .collect()
  }

  /// Look up the governance setting for a tool.
  pub fn setting_for(&self, tool: &str) -> Option<&ToolGovernanceSetting> {
    self.tools.get(tool)
  }
}

/// The outcome of applying a governance rule to a tool invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceOutcome {
  /// The tool that was governed.
  pub tool: String,
  /// The governance type that was applied.
  pub governance_type: GovernanceType,
  /// The canonical manager to delegate to (for `prefer` and `force`).
  pub canonical_manager: Option<String>,
  /// A human-readable message describing the outcome.
  pub message: String,
  /// Whether the invocation should be blocked (error-exit).
  pub blocked: bool,
  /// Whether the invocation was redirected to the canonical manager.
  pub redirected: bool,
}

impl GovernanceOutcome {
  /// Build a `prefer` outcome — warn and delegate.
  pub fn prefer(tool: &str, canonical: &str, installed: bool) -> Self {
    let message = if installed {
      format!("prefer: '{tool}' is non-canonical; delegating to '{canonical}'")
    } else {
      format!("prefer: '{tool}' is non-canonical; consider installing '{canonical}'")
    };
    GovernanceOutcome {
      tool: tool.to_string(),
      governance_type: GovernanceType::Prefer,
      canonical_manager: Some(canonical.to_string()),
      message,
      blocked: false,
      redirected: installed,
    }
  }

  /// Build a `force` outcome — intercept and route through canonical.
  pub fn force(tool: &str, canonical: &str) -> Self {
    GovernanceOutcome {
      tool: tool.to_string(),
      governance_type: GovernanceType::Force,
      canonical_manager: Some(canonical.to_string()),
      message: format!("force: '{tool}' intercepted; routing through '{canonical}'"),
      blocked: false,
      redirected: true,
    }
  }

  /// Build a `block` outcome — error-exit.
  pub fn block(tool: &str) -> Self {
    GovernanceOutcome {
      tool: tool.to_string(),
      governance_type: GovernanceType::Block,
      canonical_manager: None,
      message: format!("block: '{tool}' is blocked by governance policy"),
      blocked: true,
      redirected: false,
    }
  }

  /// Build an `eject` outcome — remove and block.
  pub fn eject(tool: &str) -> Self {
    GovernanceOutcome {
      tool: tool.to_string(),
      governance_type: GovernanceType::Eject,
      canonical_manager: None,
      message: format!("eject: '{tool}' removed and blocked by governance policy"),
      blocked: true,
      redirected: false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ===========================================================================
  // GovernanceType parsing and display
  // ===========================================================================

  #[test]
  fn test_governance_type_parse_prefer() {
    assert_eq!(
      GovernanceType::parse("prefer").unwrap(),
      GovernanceType::Prefer
    );
  }

  #[test]
  fn test_governance_type_parse_force() {
    assert_eq!(
      GovernanceType::parse("force").unwrap(),
      GovernanceType::Force
    );
  }

  #[test]
  fn test_governance_type_parse_block() {
    assert_eq!(
      GovernanceType::parse("block").unwrap(),
      GovernanceType::Block
    );
  }

  #[test]
  fn test_governance_type_parse_eject() {
    assert_eq!(
      GovernanceType::parse("eject").unwrap(),
      GovernanceType::Eject
    );
  }

  #[test]
  fn test_governance_type_parse_case_insensitive() {
    assert_eq!(
      GovernanceType::parse("PREFER").unwrap(),
      GovernanceType::Prefer
    );
    assert_eq!(
      GovernanceType::parse("Force").unwrap(),
      GovernanceType::Force
    );
  }

  #[test]
  fn test_governance_type_parse_unknown() {
    let err = GovernanceType::parse("bogus").unwrap_err();
    assert!(err.to_string().contains("unknown governance type"));
  }

  #[test]
  fn test_governance_type_display() {
    assert_eq!(GovernanceType::Prefer.to_string(), "prefer");
    assert_eq!(GovernanceType::Force.to_string(), "force");
    assert_eq!(GovernanceType::Block.to_string(), "block");
    assert_eq!(GovernanceType::Eject.to_string(), "eject");
  }

  #[test]
  fn test_governance_type_uses_wrapper() {
    assert!(!GovernanceType::Prefer.uses_wrapper());
    assert!(GovernanceType::Force.uses_wrapper());
    assert!(GovernanceType::Block.uses_wrapper());
    assert!(!GovernanceType::Eject.uses_wrapper());
  }

  // ===========================================================================
  // GovernanceRule
  // ===========================================================================

  #[test]
  fn test_governance_rule_new() {
    let rule = GovernanceRule::new("pip", GovernanceType::Force);
    assert_eq!(rule.tool, "pip");
    assert_eq!(rule.governance_type, GovernanceType::Force);
    assert!(rule.message.is_none());
  }

  #[test]
  fn test_governance_rule_with_message() {
    let rule = GovernanceRule::new("npm", GovernanceType::Block).with_message("Use pnpm instead");
    assert_eq!(rule.message.as_deref(), Some("Use pnpm instead"));
  }

  // ===========================================================================
  // GovernanceConfig TOML parsing
  // ===========================================================================

  #[test]
  fn test_governance_config_parse_force() {
    let toml = r#"
[tools.pip]
type = "force"
"#;
    let cfg = GovernanceConfig::from_toml(toml).unwrap();
    let setting = cfg.setting_for("pip").unwrap();
    assert_eq!(setting.governance_type, GovernanceType::Force);
  }

  #[test]
  fn test_governance_config_parse_block_with_message() {
    let toml = r#"
[tools.npm]
type = "block"
message = "Use pnpm"
"#;
    let cfg = GovernanceConfig::from_toml(toml).unwrap();
    let setting = cfg.setting_for("npm").unwrap();
    assert_eq!(setting.governance_type, GovernanceType::Block);
    assert_eq!(setting.message.as_deref(), Some("Use pnpm"));
  }

  #[test]
  fn test_governance_config_parse_prefer() {
    let toml = r#"
[tools.yarn]
type = "prefer"
"#;
    let cfg = GovernanceConfig::from_toml(toml).unwrap();
    let setting = cfg.setting_for("yarn").unwrap();
    assert_eq!(setting.governance_type, GovernanceType::Prefer);
  }

  #[test]
  fn test_governance_config_parse_eject() {
    let toml = r#"
[tools.bun]
type = "eject"
"#;
    let cfg = GovernanceConfig::from_toml(toml).unwrap();
    let setting = cfg.setting_for("bun").unwrap();
    assert_eq!(setting.governance_type, GovernanceType::Eject);
  }

  #[test]
  fn test_governance_config_to_rules() {
    let toml = r#"
[tools.pip]
type = "force"

[tools.npm]
type = "block"
"#;
    let cfg = GovernanceConfig::from_toml(toml).unwrap();
    let rules = cfg.to_rules();
    assert_eq!(rules.len(), 2);
  }

  #[test]
  fn test_governance_config_empty() {
    let cfg = GovernanceConfig::from_toml("").unwrap();
    assert!(cfg.tools.is_empty());
    assert!(cfg.to_rules().is_empty());
  }

  // ===========================================================================
  // GovernanceOutcome
  // ===========================================================================

  #[test]
  fn test_governance_outcome_prefer_delegates_when_installed() {
    let outcome = GovernanceOutcome::prefer("pip", "uv", true);
    assert_eq!(outcome.governance_type, GovernanceType::Prefer);
    assert!(!outcome.blocked);
    assert!(outcome.redirected);
    assert_eq!(outcome.canonical_manager.as_deref(), Some("uv"));
  }

  #[test]
  fn test_governance_outcome_prefer_warns_when_not_installed() {
    let outcome = GovernanceOutcome::prefer("pip", "uv", false);
    assert!(!outcome.blocked);
    assert!(!outcome.redirected);
  }

  #[test]
  fn test_governance_outcome_force_redirects() {
    let outcome = GovernanceOutcome::force("npm", "pnpm");
    assert_eq!(outcome.governance_type, GovernanceType::Force);
    assert!(!outcome.blocked);
    assert!(outcome.redirected);
    assert_eq!(outcome.canonical_manager.as_deref(), Some("pnpm"));
  }

  #[test]
  fn test_governance_outcome_block_blocks() {
    let outcome = GovernanceOutcome::block("pip");
    assert_eq!(outcome.governance_type, GovernanceType::Block);
    assert!(outcome.blocked);
    assert!(!outcome.redirected);
  }

  #[test]
  fn test_governance_outcome_eject_blocks() {
    let outcome = GovernanceOutcome::eject("bun");
    assert_eq!(outcome.governance_type, GovernanceType::Eject);
    assert!(outcome.blocked);
    assert!(!outcome.redirected);
  }
}
