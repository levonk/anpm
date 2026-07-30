//! Scanner plugin registry.
//!
//! Central place where scanner plugins are registered and discovered. The
//! registry is intentionally empty in story 03-002 — concrete scanner
//! implementations (npm-audit, trivy, osv-scanner, etc.) land in story
//! 03-003. This module provides the registration scaffolding those plugins
//! will plug into.
//!
//! # Adding a plugin (story 03-003)
//!
//! 1. Implement [`Scanner`] for your
//!    plugin struct.
//! 2. Add a module under `src/security/plugins/` (e.g. `npm_audit.rs`).
//! 3. Register it here in [`default_scanners`].

use crate::security::scanner::Scanner;

/// Returns the default set of scanner plugins for the current environment.
///
/// Currently empty — plugins arrive in story 03-003. Callers should still use
/// this function so that future plugins are picked up automatically.
pub fn default_scanners() -> Vec<Box<dyn Scanner>> {
  Vec::new()
}

/// Registers all default scanner plugins with an orchestrator builder.
///
/// This is a convenience for callers that construct a
/// [`ScanOrchestrator`](crate::security::ScanOrchestrator) directly. It
/// pushes every scanner returned by [`default_scanners`] into the provided
/// closure/collection.
pub fn register_defaults<F>(mut push: F)
where
  F: FnMut(Box<dyn Scanner>),
{
  for scanner in default_scanners() {
    push(scanner);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::security::scanner::ScanResult;

  #[test]
  fn test_default_scanners_is_empty_for_now() {
    assert!(default_scanners().is_empty());
  }

  #[test]
  fn test_register_defaults_pushes_nothing_yet() {
    let mut collected: Vec<String> = Vec::new();
    register_defaults(|s| collected.push(s.name().to_string()));
    assert!(collected.is_empty());
  }

  #[test]
  fn test_register_defaults_signature_compiles() {
    // Ensures the closure shape stays valid for future plugins.
    let mut count = 0usize;
    register_defaults(|_s| count += 1);
    assert_eq!(count, 0);
  }

  // A dummy scanner so the trait object path is exercised at compile time.
  struct Dummy;
  impl Scanner for Dummy {
    fn name(&self) -> &str {
      "dummy"
    }
    fn available(&self) -> bool {
      false
    }
    fn ensure(&mut self) -> crate::error::Result<()> {
      Ok(())
    }
    fn scan(&self, _dir: &std::path::Path) -> ScanResult {
      ScanResult::Safe
    }
  }

  #[test]
  fn test_dummy_scanner_can_be_boxed() {
    let s: Box<dyn Scanner> = Box::new(Dummy);
    assert_eq!(s.name(), "dummy");
    assert!(!s.available());
  }
}
