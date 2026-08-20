//! Governance engine — reads and applies governance rules from the
//! levonk-packages spec.
//!
//! The governance engine orchestrates spec loading, rule lookup, and
//! enforcement of the four governance types:
//!
//! - **prefer**: soft guidance — warn the user and delegate to the canonical
//!   manager if installed.
//! - **force**: strict replacement — intercept calls and route through the
//!   canonical manager via a wrapper.
//! - **block**: hard block — error-exiting wrapper that prevents use.
//! - **eject**: remove and block — remove the non-canonical manager and block
//!   future use.
//!
//! # Example
//!
//! ```
//! use apmw::governance::{GovernanceEngine, GovernanceType, MockSpecClient};
//!
//! let engine = GovernanceEngine::with_spec_text(
//!   "version: 1.0\nforce npm\nblock pip",
//! );
//! let outcome = engine.evaluate("npm", Some("pnpm"), true).unwrap();
//! assert_eq!(outcome.governance_type, GovernanceType::Force);
//! ```

pub mod rules;
pub mod spec;
pub mod wrapper;

pub use rules::{
  GovernanceConfig, GovernanceOutcome, GovernanceRule, GovernanceType, RuleError,
  ToolGovernanceSetting,
};
pub use spec::{
  default_cache_path, GovernanceSpec, MockSpecClient, ReqwestSpecClient, SpecError, SpecHttpClient,
  SpecLoader, SPEC_URL,
};
pub use wrapper::{
  build_wrapper, is_wrapper_name, tool_from_wrapper_name, wrapper_name_for, WrapperError,
  WrapperKind, WrapperSpec, WRAPPER_PREFIX,
};

use std::collections::HashMap;

use tracing::{debug, info, warn};

use crate::error::{ApmwError, Result};

/// The governance engine — orchestrates spec loading and rule enforcement.
pub struct GovernanceEngine {
  spec: GovernanceSpec,
  config: GovernanceConfig,
}

impl GovernanceEngine {
  /// Create a new engine from a parsed spec and config.
  pub fn new(spec: GovernanceSpec, config: GovernanceConfig) -> Self {
    GovernanceEngine { spec, config }
  }

  /// Create an engine with no spec (empty) and the given config.
  pub fn with_config(config: GovernanceConfig) -> Self {
    GovernanceEngine {
      spec: GovernanceSpec::empty(),
      config,
    }
  }

  /// Create an engine by parsing the given spec text (no config).
  pub fn with_spec_text(spec_text: &str) -> Self {
    let spec = spec::parse_spec(spec_text).unwrap_or_else(|_| GovernanceSpec::empty());
    GovernanceEngine {
      spec,
      config: GovernanceConfig::default(),
    }
  }

  /// Load the engine from the spec (via HTTP client) and config.
  pub fn load(
    loader: &SpecLoader,
    client: &dyn SpecHttpClient,
    config: GovernanceConfig,
  ) -> Result<Self> {
    let spec = loader.load(client)?;
    Ok(GovernanceEngine::new(spec, config))
  }

  /// Returns a reference to the parsed spec.
  pub fn spec(&self) -> &GovernanceSpec {
    &self.spec
  }

  /// Returns a reference to the governance config.
  pub fn config(&self) -> &GovernanceConfig {
    &self.config
  }

  /// Look up the governance rule for a tool.
  ///
  /// Config rules take precedence over spec rules (config is per-project,
  /// spec is global).
  pub fn rule_for(&self, tool: &str) -> Option<GovernanceRule> {
    // Config takes precedence.
    if let Some(setting) = self.config.setting_for(tool) {
      let mut rule = GovernanceRule::new(tool, setting.governance_type);
      rule.message = setting.message.clone();
      return Some(rule);
    }
    // Fall back to spec.
    self.spec.rule_for(tool).cloned()
  }

  /// Evaluate the governance policy for a tool invocation.
  ///
  /// `canonical_manager` is the canonical manager for the tool's ecosystem
  /// (from the ecosystem mapper). `canonical_installed` indicates whether the
  /// canonical manager is available on the system (relevant for `prefer`).
  pub fn evaluate(
    &self,
    tool: &str,
    canonical_manager: Option<&str>,
    canonical_installed: bool,
  ) -> Result<GovernanceOutcome> {
    let rule = self
      .rule_for(tool)
      .ok_or_else(|| ApmwError::Governance(format!("no governance rule for tool '{tool}'")))?;

    info!(
      tool = tool,
      governance_type = %rule.governance_type,
      "applying governance rule"
    );

    let outcome = match rule.governance_type {
      GovernanceType::Prefer => {
        let canonical = canonical_manager.unwrap_or("canonical");
        GovernanceOutcome::prefer(tool, canonical, canonical_installed)
      }
      GovernanceType::Force => {
        let canonical = canonical_manager.ok_or_else(|| {
          ApmwError::Governance(format!(
            "force governance for '{tool}' requires a canonical manager"
          ))
        })?;
        GovernanceOutcome::force(tool, canonical)
      }
      GovernanceType::Block => {
        warn!(tool = tool, "tool invocation blocked by governance");
        GovernanceOutcome::block(tool)
      }
      GovernanceType::Eject => {
        warn!(tool = tool, "tool ejected and blocked by governance");
        GovernanceOutcome::eject(tool)
      }
    };

    debug!(outcome = ?outcome, "governance outcome");
    Ok(outcome)
  }

  /// Build a wrapper spec for a tool if its governance type uses a wrapper.
  pub fn wrapper_for(&self, tool: &str) -> Result<Option<WrapperSpec>> {
    let rule = self
      .rule_for(tool)
      .ok_or_else(|| ApmwError::Governance(format!("no governance rule for tool '{tool}'")))?;
    match rule.governance_type {
      GovernanceType::Force => {
        let canonical = self.canonical_for(tool)?;
        Ok(Some(WrapperSpec::force(tool, &canonical)))
      }
      GovernanceType::Block => Ok(Some(WrapperSpec::block(tool))),
      _ => Ok(None),
    }
  }

  /// Determine the canonical manager for a tool (from the ecosystem mapper or
  /// spec). Returns a sensible default based on common ecosystems.
  fn canonical_for(&self, tool: &str) -> Result<String> {
    // Use the ecosystem mapper for canonical lookup.
    use crate::ecosystem::{EcosystemMapper, PackageManager};
    if let Some(pm) = PackageManager::parse_manager(tool) {
      let mapper = EcosystemMapper::new();
      let canonical = mapper.suggest_canonical(pm);
      if canonical != pm {
        return Ok(canonical.to_string());
      }
    }
    Err(ApmwError::Governance(format!(
      "no canonical manager found for '{tool}'"
    )))
  }

  /// Returns all rules from both spec and config (config overrides spec for
  /// the same tool).
  pub fn all_rules(&self) -> Vec<GovernanceRule> {
    let mut by_tool: HashMap<String, GovernanceRule> = HashMap::new();
    for rule in &self.spec.rules {
      by_tool.insert(rule.tool.clone(), rule.clone());
    }
    for rule in self.config.to_rules() {
      by_tool.insert(rule.tool.clone(), rule);
    }
    by_tool.into_values().collect()
  }

  /// Returns `true` if the engine has any governance rules.
  pub fn has_rules(&self) -> bool {
    !self.spec.rules.is_empty() || !self.config.tools.is_empty()
  }
}

impl Default for GovernanceEngine {
  fn default() -> Self {
    GovernanceEngine {
      spec: GovernanceSpec::empty(),
      config: GovernanceConfig::default(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  // ===========================================================================
  // GovernanceEngine — prefer
  // ===========================================================================

  #[test]
  fn test_governance_prefer_delegates_when_installed() {
    let engine = GovernanceEngine::with_spec_text("prefer pip");
    let outcome = engine.evaluate("pip", Some("uv"), true).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Prefer);
    assert!(!outcome.blocked);
    assert!(outcome.redirected);
  }

  #[test]
  fn test_governance_prefer_warns_when_not_installed() {
    let engine = GovernanceEngine::with_spec_text("prefer pip");
    let outcome = engine.evaluate("pip", Some("uv"), false).unwrap();
    assert!(!outcome.blocked);
    assert!(!outcome.redirected);
  }

  // ===========================================================================
  // GovernanceEngine — force
  // ===========================================================================

  #[test]
  fn test_governance_force_redirects() {
    let engine = GovernanceEngine::with_spec_text("force npm");
    let outcome = engine.evaluate("npm", Some("pnpm"), true).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Force);
    assert!(outcome.redirected);
    assert!(!outcome.blocked);
  }

  #[test]
  fn test_governance_force_without_canonical_errors() {
    let engine = GovernanceEngine::with_spec_text("force npm");
    let result = engine.evaluate("npm", None, false);
    assert!(result.is_err());
  }

  // ===========================================================================
  // GovernanceEngine — block
  // ===========================================================================

  #[test]
  fn test_governance_block_blocks() {
    let engine = GovernanceEngine::with_spec_text("block pip");
    let outcome = engine.evaluate("pip", None, false).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Block);
    assert!(outcome.blocked);
    assert!(!outcome.redirected);
  }

  // ===========================================================================
  // GovernanceEngine — eject
  // ===========================================================================

  #[test]
  fn test_governance_eject_blocks() {
    let engine = GovernanceEngine::with_spec_text("eject bun");
    let outcome = engine.evaluate("bun", None, false).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Eject);
    assert!(outcome.blocked);
    assert!(!outcome.redirected);
  }

  // ===========================================================================
  // Config overrides spec
  // ===========================================================================

  #[test]
  fn test_governance_config_overrides_spec() {
    let spec = spec::parse_spec("force npm").unwrap();
    let toml = r#"
[tools.npm]
type = "block"
"#;
    let config = GovernanceConfig::from_toml(toml).unwrap();
    let engine = GovernanceEngine::new(spec, config);
    let outcome = engine.evaluate("npm", Some("pnpm"), true).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Block);
  }

  #[test]
  fn test_governance_config_only() {
    let toml = r#"
[tools.pip]
type = "force"
"#;
    let config = GovernanceConfig::from_toml(toml).unwrap();
    let engine = GovernanceEngine::with_config(config);
    let outcome = engine.evaluate("pip", Some("uv"), true).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Force);
  }

  // ===========================================================================
  // No rule
  // ===========================================================================

  #[test]
  fn test_governance_no_rule_errors() {
    let engine = GovernanceEngine::with_spec_text("force npm");
    let result = engine.evaluate("cargo", None, false);
    assert!(result.is_err());
  }

  // ===========================================================================
  // Wrapper integration
  // ===========================================================================

  #[test]
  fn test_governance_wrapper_for_force() {
    let engine = GovernanceEngine::with_spec_text("force npm");
    let wrapper = engine.wrapper_for("npm").unwrap().unwrap();
    assert_eq!(wrapper.kind, WrapperKind::Force);
    assert_eq!(wrapper.canonical_manager.as_deref(), Some("pnpm"));
    assert_eq!(wrapper.wrapper_name, "devbox-rtk-npm");
  }

  #[test]
  fn test_governance_wrapper_for_block() {
    let engine = GovernanceEngine::with_spec_text("block pip");
    let wrapper = engine.wrapper_for("pip").unwrap().unwrap();
    assert_eq!(wrapper.kind, WrapperKind::Block);
    assert_eq!(wrapper.wrapper_name, "devbox-rtk-pip");
  }

  #[test]
  fn test_governance_wrapper_for_prefer_returns_none() {
    let engine = GovernanceEngine::with_spec_text("prefer pip");
    let wrapper = engine.wrapper_for("pip").unwrap();
    assert!(wrapper.is_none());
  }

  // ===========================================================================
  // all_rules / has_rules
  // ===========================================================================

  #[test]
  fn test_governance_all_rules_merges() {
    let spec = spec::parse_spec("force npm\nblock pip").unwrap();
    let toml = r#"
[tools.yarn]
type = "prefer"
"#;
    let config = GovernanceConfig::from_toml(toml).unwrap();
    let engine = GovernanceEngine::new(spec, config);
    let rules = engine.all_rules();
    assert!(rules.len() >= 3);
  }

  #[test]
  fn test_governance_has_rules_true() {
    let engine = GovernanceEngine::with_spec_text("force npm");
    assert!(engine.has_rules());
  }

  #[test]
  fn test_governance_has_rules_false() {
    let engine = GovernanceEngine::default();
    assert!(!engine.has_rules());
  }

  // ===========================================================================
  // Integration: SpecLoader + GovernanceEngine
  // ===========================================================================

  #[test]
  fn test_governance_engine_load_from_spec() {
    let tmp = TempDir::new().unwrap();
    let loader = SpecLoader::with_cache_path(tmp.path().join("spec.json"));
    let client = MockSpecClient::with_ok("version: 1.0\nforce npm\nblock pip");
    let engine = GovernanceEngine::load(&loader, &client, GovernanceConfig::default()).unwrap();
    let outcome = engine.evaluate("npm", Some("pnpm"), true).unwrap();
    assert_eq!(outcome.governance_type, GovernanceType::Force);
  }

  // ===========================================================================
  // devbox-rtk-* naming convention
  // ===========================================================================

  #[test]
  fn test_devbox_rtk_naming() {
    assert_eq!(wrapper_name_for("npm"), "devbox-rtk-npm");
    assert!(is_wrapper_name("devbox-rtk-npm"));
    assert!(!is_wrapper_name("npm"));
    assert_eq!(tool_from_wrapper_name("devbox-rtk-pip"), Some("pip"));
  }
}
