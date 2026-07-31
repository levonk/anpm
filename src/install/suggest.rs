//! Alternative suggestions engine — suggests canonical alternatives.
//!
//! When a user requests an install via a non-canonical manager (e.g. `pip`),
//! apmw suggests the canonical alternative within the same ecosystem (e.g.
//! `uv`) and offers to use it. Suggestions are derived from the ecosystem
//! mapping table (PRD FR-2.2, Goal 5).
//!
//! # Core principle: WITHIN-ecosystem only
//!
//! Suggestions are **never** cross-ecosystem (PRD FR-2.1). A Python runner
//! suggests a Python runner; a Node runner suggests a Node runner. For example,
//! `pip` suggests `uv` (both Python), and `npm` suggests `pnpm` (both Node),
//! but `uvx` (Python) must NEVER suggest `pnpm dlx` (Node).
//!
//! # Offer-to-use modes
//!
//! The [`SuggestEngine::offer_to_use`] method adapts its output to the active
//! [`OutputMode`]:
//!
//! - **Human mode** ([`OutputMode::Human`]): produces a prompt string asking
//!   the user whether to use the canonical alternative.
//! - **Agent mode** ([`OutputMode::Agent`]): produces `help[]` array entries
//!   containing the suggestion as complete next-step command strings (per
//!   ADR-20260607001 §45 — no interactive prompts in agent mode).
//!
//! # Example
//!
//! ```
//! use apmw::ecosystem::PackageManager;
//! use apmw::install::SuggestEngine;
//! use apmw::output::OutputMode;
//!
//! let engine = SuggestEngine::new();
//! // pip (Python) -> uv (Python): within-ecosystem suggestion
//! let suggestion = engine.suggest(PackageManager::Pip).unwrap();
//! assert_eq!(suggestion.canonical, PackageManager::Uv);
//! assert!(!suggestion.reason.is_empty());
//!
//! // uv is already canonical — no suggestion.
//! assert!(engine.suggest(PackageManager::Uv).is_none());
//!
//! // Offer-to-use in agent mode produces help[] entries.
//! let offer = engine.offer_to_use(PackageManager::Pip, OutputMode::Agent);
//! assert!(!offer.help.is_empty());
//! ```

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::ecosystem::{EcosystemMapper, PackageManager};
use crate::output::OutputMode;

/// A canonical-alternative suggestion for a non-canonical package manager.
///
/// Produced by [`SuggestEngine::suggest`] when the source manager should be
/// remapped to its ecosystem's canonical manager. Contains the canonical
/// alternative and a human-readable reason explaining the recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
  /// The source (non-canonical) package manager the user requested.
  pub source: PackageManager,
  /// The canonical alternative within the same ecosystem.
  pub canonical: PackageManager,
  /// A human-readable reason explaining why the canonical is recommended.
  pub reason: String,
}

impl Suggestion {
  /// Returns the canonical manager's add command for the given package,
  /// suitable for inclusion in a `help[]` next-step entry.
  ///
  /// Returns `None` if the canonical manager has no `add` command in the
  /// mapping table.
  pub fn canonical_add_command(&self, package: &str) -> Option<String> {
    crate::ecosystem::manager_command(self.canonical, crate::ecosystem::ApmwCommand::Add)
      .map(|cmd| cmd.replace("<pkg>", package))
  }
}

/// The result of an offer-to-use — adapts to the active output mode.
///
/// In human mode, `prompt` contains the interactive prompt text and `help` is
/// empty. In agent mode, `help` contains the suggestion as `help[]` entries
/// and `prompt` is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferResult {
  /// The underlying suggestion, if any (None when the source is already
  /// canonical and no switch is needed).
  pub suggestion: Option<Suggestion>,
  /// The human-mode prompt text (None in agent mode or when no suggestion).
  pub prompt: Option<String>,
  /// The agent-mode `help[]` array entries (empty in human mode or when no
  /// suggestion).
  pub help: Vec<String>,
}

/// The alternative suggestions engine.
///
/// Wraps an [`EcosystemMapper`] and produces within-ecosystem canonical
/// suggestions for non-canonical package managers. The engine is cheap to
/// construct and fully deterministic.
#[derive(Debug, Clone)]
pub struct SuggestEngine {
  mapper: EcosystemMapper,
}

impl SuggestEngine {
  /// Create a new suggestions engine with the default ecosystem mapper.
  pub fn new() -> Self {
    SuggestEngine {
      mapper: EcosystemMapper::new(),
    }
  }

  /// Create a new suggestions engine with a custom ecosystem mapper (for
  /// testing).
  pub fn with_mapper(mapper: EcosystemMapper) -> Self {
    SuggestEngine { mapper }
  }

  /// Suggest the canonical alternative for the given source manager.
  ///
  /// Returns `Some(Suggestion)` when the source manager should be remapped to
  /// a different canonical manager. Only the five within-ecosystem remapping
  /// pairs produce a suggestion: `pip -> uv`, `npm -> pnpm`, `yarn -> pnpm`,
  /// `yarn2 -> pnpm`, and `bun -> pnpm` (PRD FR-2.1, FR-2.2).
  ///
  /// Returns `None` when the source is already canonical or "remains as-is"
  /// (poetry, pipenv, pdm, conda, cargo, go, etc.). These managers are not
  /// forced to a different runner even though their ecosystem has a canonical.
  ///
  /// The suggestion is always within-ecosystem (PRD FR-2.1): the canonical
  /// manager belongs to the same ecosystem as the source.
  pub fn suggest(&self, source: PackageManager) -> Option<Suggestion> {
    // Only the five explicit remapping pairs produce a suggestion. Other
    // non-canonical managers (poetry, pipenv, pdm, conda) "remain as-is" per
    // PRD FR-2.1 and are not forced to the ecosystem canonical. This mirrors
    // the `should_remap` whitelist in `runner.rs`.
    let is_remappable = matches!(
      source,
      PackageManager::Pip
        | PackageManager::Npm
        | PackageManager::Yarn
        | PackageManager::Yarn2
        | PackageManager::Bun
    );
    if !is_remappable {
      info!(source = %source, "no suggestion — source is canonical or remains as-is");
      return None;
    }
    let canonical = self.mapper.suggest_canonical(source);
    let reason = reason_for(source, canonical);
    info!(
      source = %source,
      canonical = %canonical,
      "suggested canonical alternative"
    );
    Some(Suggestion {
      source,
      canonical,
      reason,
    })
  }

  /// Produce an offer-to-use for the given source manager, adapted to the
  /// active output mode.
  ///
  /// - **Human mode** ([`OutputMode::Human`]): returns an [`OfferResult`] with
  ///   a `prompt` string asking the user to use the canonical alternative. No
  ///   interactive I/O is performed — the caller renders the prompt and reads
  ///   the response.
  /// - **Agent mode** ([`OutputMode::Agent`]): returns an [`OfferResult`] with
  ///   `help[]` entries containing the suggestion as complete next-step
  ///   command strings (per ADR-20260607001 §45). No interactive prompts.
  ///
  /// Returns an [`OfferResult`] with `suggestion: None` when the source is
  /// already canonical (no switch needed).
  pub fn offer_to_use(&self, source: PackageManager, mode: OutputMode) -> OfferResult {
    let suggestion = self.suggest(source);
    match (&suggestion, mode) {
      (None, _) => OfferResult {
        suggestion: None,
        prompt: None,
        help: Vec::new(),
      },
      (Some(s), OutputMode::Human) => {
        let prompt = format!(
          "{} is the canonical {} package manager. {} Use `{}` instead of `{}`? \
           (apmw will use {} automatically unless you decline)",
          capitalize(s.canonical.as_str()),
          s.canonical.ecosystem(),
          s.reason,
          s.canonical,
          s.source,
          s.canonical,
        );
        OfferResult {
          suggestion,
          prompt: Some(prompt),
          help: Vec::new(),
        }
      }
      (Some(s), OutputMode::Agent) => {
        let help = vec![
          format!(
            "apmw add --manager {} <pkg>  # suggested canonical {} alternative for {}",
            s.canonical,
            s.canonical.ecosystem(),
            s.source,
          ),
          format!("# reason: {}", s.reason,),
        ];
        OfferResult {
          suggestion,
          prompt: None,
          help,
        }
      }
    }
  }

  /// Returns `true` if the source manager would produce a suggestion (i.e. it
  /// is one of the five remappable non-canonical managers: pip, npm, yarn,
  /// yarn2, bun).
  pub fn has_suggestion(&self, source: PackageManager) -> bool {
    matches!(
      source,
      PackageManager::Pip
        | PackageManager::Npm
        | PackageManager::Yarn
        | PackageManager::Yarn2
        | PackageManager::Bun
    )
  }
}

impl Default for SuggestEngine {
  fn default() -> Self {
    Self::new()
  }
}

/// Returns a human-readable reason explaining why the canonical manager is
/// recommended over the source manager.
///
/// Reasons are per-source-manager and describe the concrete benefit of
/// switching within the same ecosystem.
fn reason_for(source: PackageManager, canonical: PackageManager) -> String {
  match (source, canonical) {
    // Python ecosystem: pip -> uv
    (PackageManager::Pip, PackageManager::Uv) => "uv is 10-100x faster than pip and is the \
      canonical Python installer."
      .to_string(),
    // Node ecosystem: npm/yarn/yarn2/bun -> pnpm
    (PackageManager::Npm, PackageManager::Pnpm) => {
      "pnpm is disk-efficient (content-addressed store) and strict; it is the canonical Node \
       installer."
        .to_string()
    }
    (PackageManager::Yarn, PackageManager::Pnpm) => {
      "pnpm is disk-efficient (content-addressed store) and strict; it is the canonical Node \
       installer."
        .to_string()
    }
    (PackageManager::Yarn2, PackageManager::Pnpm) => {
      "pnpm is disk-efficient (content-addressed store) and strict; it is the canonical Node \
       installer."
        .to_string()
    }
    (PackageManager::Bun, PackageManager::Pnpm) => {
      "pnpm is disk-efficient (content-addressed store), strict, and mature; it is the canonical \
       Node installer."
        .to_string()
    }
    // Fallback: generic reason for any future within-ecosystem suggestion.
    (source, canonical) => format!(
      "{canonical} is the canonical {eco} package manager and is recommended over {source}.",
      eco = canonical.ecosystem(),
    ),
  }
}

/// Capitalize the first letter of a string (for prompt formatting).
fn capitalize(s: &str) -> String {
  let mut chars = s.chars();
  match chars.next() {
    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    None => String::new(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ecosystem::Ecosystem;

  // ===========================================================================
  // SuggestEngine::suggest — within-ecosystem suggestion pairs
  // ===========================================================================

  #[test]
  fn test_suggest_pip_to_uv() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Pip).unwrap();
    assert_eq!(s.source, PackageManager::Pip);
    assert_eq!(s.canonical, PackageManager::Uv);
    assert_eq!(s.canonical.ecosystem(), Ecosystem::Python);
    assert!(!s.reason.is_empty());
    assert!(s.reason.contains("uv"));
  }

  #[test]
  fn test_suggest_npm_to_pnpm() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Npm).unwrap();
    assert_eq!(s.source, PackageManager::Npm);
    assert_eq!(s.canonical, PackageManager::Pnpm);
    assert_eq!(s.canonical.ecosystem(), Ecosystem::Node);
    assert!(!s.reason.is_empty());
    assert!(s.reason.contains("pnpm"));
  }

  #[test]
  fn test_suggest_yarn_to_pnpm() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Yarn).unwrap();
    assert_eq!(s.source, PackageManager::Yarn);
    assert_eq!(s.canonical, PackageManager::Pnpm);
    assert_eq!(s.canonical.ecosystem(), Ecosystem::Node);
    assert!(!s.reason.is_empty());
  }

  #[test]
  fn test_suggest_bun_to_pnpm() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Bun).unwrap();
    assert_eq!(s.source, PackageManager::Bun);
    assert_eq!(s.canonical, PackageManager::Pnpm);
    assert_eq!(s.canonical.ecosystem(), Ecosystem::Node);
    assert!(!s.reason.is_empty());
  }

  #[test]
  fn test_suggest_yarn2_to_pnpm() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Yarn2).unwrap();
    assert_eq!(s.source, PackageManager::Yarn2);
    assert_eq!(s.canonical, PackageManager::Pnpm);
    assert_eq!(s.canonical.ecosystem(), Ecosystem::Node);
    assert!(!s.reason.is_empty());
  }

  // ===========================================================================
  // No suggestion for canonical / remains-as-is managers
  // ===========================================================================

  #[test]
  fn test_suggest_uv_is_none() {
    let engine = SuggestEngine::new();
    assert!(engine.suggest(PackageManager::Uv).is_none());
  }

  #[test]
  fn test_suggest_pnpm_is_none() {
    let engine = SuggestEngine::new();
    assert!(engine.suggest(PackageManager::Pnpm).is_none());
  }

  #[test]
  fn test_suggest_cargo_is_none() {
    let engine = SuggestEngine::new();
    assert!(engine.suggest(PackageManager::Cargo).is_none());
  }

  #[test]
  fn test_suggest_go_is_none() {
    let engine = SuggestEngine::new();
    assert!(engine.suggest(PackageManager::Go).is_none());
  }

  #[test]
  fn test_suggest_poetry_is_none() {
    // poetry "remains as-is" — no forced switch.
    let engine = SuggestEngine::new();
    assert!(engine.suggest(PackageManager::Poetry).is_none());
  }

  // ===========================================================================
  // No cross-ecosystem suggestions (PRD FR-2.1)
  // ===========================================================================

  #[test]
  fn test_no_cross_ecosystem_suggestions() {
    let engine = SuggestEngine::new();
    for source in crate::ecosystem::all_managers() {
      if let Some(s) = engine.suggest(source) {
        assert_eq!(
          s.canonical.ecosystem(),
          source.ecosystem(),
          "suggest({source}) returned {canonical} from a different ecosystem",
          canonical = s.canonical,
        );
      }
    }
  }

  /// Explicitly asserts that pip (Python) does NOT suggest pnpm (Node) — the
  /// classic cross-ecosystem mistake. Also asserts uvx/pnpm-dlx style
  /// cross-ecosystem suggestions do not exist.
  #[test]
  fn test_pip_does_not_suggest_pnpm() {
    let engine = SuggestEngine::new();
    if let Some(s) = engine.suggest(PackageManager::Pip) {
      assert_ne!(
        s.canonical,
        PackageManager::Pnpm,
        "pip suggested pnpm — cross-ecosystem!"
      );
      assert_eq!(s.canonical, PackageManager::Uv);
    }
  }

  // ===========================================================================
  // offer_to_use — human mode (prompt)
  // ===========================================================================

  #[test]
  fn test_offer_human_mode_produces_prompt() {
    let engine = SuggestEngine::new();
    let offer = engine.offer_to_use(PackageManager::Pip, OutputMode::Human);
    assert!(offer.suggestion.is_some());
    let prompt = offer.prompt.expect("human mode should produce a prompt");
    assert!(prompt.contains("uv"));
    assert!(prompt.contains("pip"));
    assert!(prompt.contains("Use"));
    // Human mode does not populate help[].
    assert!(offer.help.is_empty());
  }

  #[test]
  fn test_offer_human_mode_npm() {
    let engine = SuggestEngine::new();
    let offer = engine.offer_to_use(PackageManager::Npm, OutputMode::Human);
    let prompt = offer.prompt.expect("human mode should produce a prompt");
    assert!(prompt.contains("pnpm"));
    assert!(prompt.contains("npm"));
  }

  #[test]
  fn test_offer_human_mode_no_suggestion() {
    let engine = SuggestEngine::new();
    let offer = engine.offer_to_use(PackageManager::Uv, OutputMode::Human);
    assert!(offer.suggestion.is_none());
    assert!(offer.prompt.is_none());
    assert!(offer.help.is_empty());
  }

  // ===========================================================================
  // offer_to_use — agent mode (help[])
  // ===========================================================================

  #[test]
  fn test_offer_agent_mode_produces_help() {
    let engine = SuggestEngine::new();
    let offer = engine.offer_to_use(PackageManager::Pip, OutputMode::Agent);
    assert!(offer.suggestion.is_some());
    // Agent mode does not use interactive prompts.
    assert!(offer.prompt.is_none());
    // Agent mode populates help[] with next-step command strings.
    assert!(!offer.help.is_empty());
    let help_text = offer.help.join("\n");
    assert!(help_text.contains("uv"));
    assert!(help_text.contains("pip"));
    // The help entry should be a complete command string (ADR §45).
    assert!(help_text.contains("apmw add --manager uv"));
  }

  #[test]
  fn test_offer_agent_mode_npm() {
    let engine = SuggestEngine::new();
    let offer = engine.offer_to_use(PackageManager::Npm, OutputMode::Agent);
    assert!(offer.prompt.is_none());
    assert!(!offer.help.is_empty());
    let help_text = offer.help.join("\n");
    assert!(help_text.contains("pnpm"));
    assert!(help_text.contains("npm"));
  }

  #[test]
  fn test_offer_agent_mode_no_suggestion() {
    let engine = SuggestEngine::new();
    let offer = engine.offer_to_use(PackageManager::Pnpm, OutputMode::Agent);
    assert!(offer.suggestion.is_none());
    assert!(offer.prompt.is_none());
    assert!(offer.help.is_empty());
  }

  // ===========================================================================
  // offer_to_use — all non-canonical managers produce suggestions
  // ===========================================================================

  #[test]
  fn test_offer_all_non_canonical_managers() {
    let engine = SuggestEngine::new();
    for source in crate::ecosystem::all_managers() {
      let offer = engine.offer_to_use(source, OutputMode::Agent);
      if engine.has_suggestion(source) {
        assert!(
          offer.suggestion.is_some(),
          "non-canonical {source} should produce a suggestion"
        );
        assert!(
          !offer.help.is_empty(),
          "non-canonical {source} should produce help[] entries in agent mode"
        );
      } else {
        assert!(
          offer.suggestion.is_none(),
          "canonical {source} should not produce a suggestion"
        );
        assert!(offer.help.is_empty());
      }
    }
  }

  // ===========================================================================
  // Suggestion::canonical_add_command
  // ===========================================================================

  #[test]
  fn test_suggestion_canonical_add_command_pip() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Pip).unwrap();
    let cmd = s.canonical_add_command("requests").unwrap();
    assert_eq!(cmd, "uv pip install requests");
  }

  #[test]
  fn test_suggestion_canonical_add_command_npm() {
    let engine = SuggestEngine::new();
    let s = engine.suggest(PackageManager::Npm).unwrap();
    let cmd = s.canonical_add_command("express").unwrap();
    assert_eq!(cmd, "pnpm add express");
  }

  // ===========================================================================
  // has_suggestion
  // ===========================================================================

  #[test]
  fn test_has_suggestion_true_for_non_canonical() {
    let engine = SuggestEngine::new();
    assert!(engine.has_suggestion(PackageManager::Pip));
    assert!(engine.has_suggestion(PackageManager::Npm));
    assert!(engine.has_suggestion(PackageManager::Yarn));
    assert!(engine.has_suggestion(PackageManager::Bun));
    assert!(engine.has_suggestion(PackageManager::Yarn2));
  }

  #[test]
  fn test_has_suggestion_false_for_canonical() {
    let engine = SuggestEngine::new();
    assert!(!engine.has_suggestion(PackageManager::Uv));
    assert!(!engine.has_suggestion(PackageManager::Pnpm));
    assert!(!engine.has_suggestion(PackageManager::Cargo));
    assert!(!engine.has_suggestion(PackageManager::Go));
    assert!(!engine.has_suggestion(PackageManager::Poetry));
  }

  // ===========================================================================
  // reason_for — reasons are non-empty and mention the canonical
  // ===========================================================================

  #[test]
  fn test_reason_mentions_canonical() {
    let engine = SuggestEngine::new();
    for source in [
      PackageManager::Pip,
      PackageManager::Npm,
      PackageManager::Yarn,
      PackageManager::Bun,
      PackageManager::Yarn2,
    ] {
      let s = engine.suggest(source).unwrap();
      assert!(
        s.reason.contains(s.canonical.as_str()),
        "reason for {source} -> {} should mention the canonical manager",
        s.canonical,
      );
    }
  }

  // ===========================================================================
  // capitalize helper
  // ===========================================================================

  #[test]
  fn test_capitalize() {
    assert_eq!(capitalize("uv"), "Uv");
    assert_eq!(capitalize("pnpm"), "Pnpm");
    assert_eq!(capitalize(""), "");
  }

  // ===========================================================================
  // Default impl
  // ===========================================================================

  #[test]
  fn test_default_equals_new() {
    let a = SuggestEngine::default();
    let b = SuggestEngine::new();
    // Both should produce the same suggestion for pip.
    assert_eq!(
      a.suggest(PackageManager::Pip),
      b.suggest(PackageManager::Pip)
    );
  }
}
