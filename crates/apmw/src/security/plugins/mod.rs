//! Scanner plugin registry.
//!
//! Central place where scanner plugins are registered and discovered. Each
//! plugin implements the [`Scanner`] trait and lives in its own module under
//! `src/security/plugins/`. The [`default_scanners`] function returns the full
//! set of built-in scanners so the orchestrator picks them up automatically.
//!
//! # Adding a plugin
//!
//! 1. Implement [`Scanner`] for your plugin struct in a new module (e.g.
//!    `npm_audit.rs`).
//! 2. Add `pub mod npm_audit;` below.
//! 3. Register it in [`default_scanners`].

pub mod cargo_audit;
pub mod grype;
pub mod npm_audit;
pub mod osv_scanner;
pub mod pip_audit;
pub mod trivy;

pub use cargo_audit::CargoAudit;
pub use grype::Grype;
pub use npm_audit::NpmAudit;
pub use osv_scanner::OsvScanner;
pub use pip_audit::PipAudit;
pub use trivy::Trivy;

use crate::security::scanner::Scanner;

/// Returns the default set of scanner plugins for the current environment.
///
/// The returned scanners are **not** filtered by availability — the
/// orchestrator checks `available()` before invoking each scanner. This means
/// all six built-in scanners are always registered, and only the ones whose
/// binaries are on `PATH` will actually run.
pub fn default_scanners() -> Vec<Box<dyn Scanner>> {
  vec![
    Box::new(CargoAudit::new()),
    Box::new(NpmAudit::new()),
    Box::new(PipAudit::new()),
    Box::new(OsvScanner::new()),
    Box::new(Trivy::new()),
    Box::new(Grype::new()),
  ]
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

  #[test]
  fn test_default_scanners_returns_all_six() {
    let scanners = default_scanners();
    assert_eq!(scanners.len(), 6, "expected 6 default scanners");
  }

  #[test]
  fn test_default_scanners_names() {
    let scanners = default_scanners();
    let names: Vec<&str> = scanners.iter().map(|s| s.name()).collect();
    assert!(names.contains(&"cargo-audit"));
    assert!(names.contains(&"npm-audit"));
    assert!(names.contains(&"pip-audit"));
    assert!(names.contains(&"osv-scanner"));
    assert!(names.contains(&"trivy"));
    assert!(names.contains(&"grype"));
  }

  #[test]
  fn test_register_defaults_pushes_all() {
    let mut collected: Vec<String> = Vec::new();
    register_defaults(|s| collected.push(s.name().to_string()));
    assert_eq!(collected.len(), 6);
  }

  #[test]
  fn test_all_scanners_have_unique_names() {
    let scanners = default_scanners();
    let names: Vec<&str> = scanners.iter().map(|s| s.name()).collect();
    let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(names.len(), unique.len(), "scanner names must be unique");
  }
}
