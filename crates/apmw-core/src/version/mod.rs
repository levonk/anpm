//! Version resolution engine.
//!
//! Resolves package versions using the priority chain from PRD FR-4.1:
//!   1. **Pinned** — version locked in a lockfile (Cargo.lock, package-lock.json,
//!      pnpm-lock.yaml, poetry.lock, yarn.lock).
//!   2. **Engine-compatible** — latest version satisfying the project's engine
//!      constraints (`engines` in package.json, `python-requires` in
//!      setup.py/pyproject.toml, `rust-version` in Cargo.toml).
//!   3. **Latest** — the newest version published to the registry.
//!   4. **Latest-minor** — the newest minor version of a specified major.
//!
//! ## Supply-chain defense (min-age-days)
//!
//! Per PRD FR-4.3 through FR-4.5, auto-resolved versions (priority 3 "latest"
//! and priority 4 "latest-minor") are refused if they were published less than
//! `min_age_days` days ago (default 2). When the newest version is too fresh,
//! the engine falls back to the most recent version that is at least
//! `min_age_days` days old. Pinned (priority 1) and engine-compatible
//! (priority 2) versions bypass the age check — the user explicitly locked or
//! constrained those.
//!
//! The threshold is overrideable with this precedence (highest first):
//!   1. CLI flag (`--min-age-days <N>` or `--allow-new` alias for `0`)
//!   2. Environment variable `APMW_MIN_AGE_DAYS`
//!   3. Config file `min_age_days`
//!   4. Default (`2`)
//!
//! Registry queries return version + publish date. If a registry source does
//! not provide publish dates, the age check is skipped for that source (the
//! version's yank/listed status is still respected).

pub mod resolver;

pub use resolver::{
  EngineConstraints, LockfileKind, MockRegistryClient, RegistryClient, RegistryVersion,
  UreqRegistryClient, VersionResolver,
};

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The strategy used to resolve a version (PRD FR-4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
  /// Priority 1 — version pinned in a lockfile.
  Pinned,
  /// Priority 2 — latest version satisfying engine constraints.
  EngineCompatible,
  /// Priority 3 — newest version from the registry.
  Latest,
  /// Priority 4 — newest minor version of a specified major.
  LatestMinor,
}

impl fmt::Display for ResolutionStrategy {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ResolutionStrategy::Pinned => write!(f, "pinned"),
      ResolutionStrategy::EngineCompatible => write!(f, "engine_compatible"),
      ResolutionStrategy::Latest => write!(f, "latest"),
      ResolutionStrategy::LatestMinor => write!(f, "latest_minor"),
    }
  }
}

/// The outcome of a version resolution request.
///
/// Records the requested spec, the resolved version, and the strategy that
/// produced it. Serialized with serde for TOON/JSON output and audit logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionResolution {
  /// The original version request (e.g. `""`, `"4"`, `"^1.2"`, `"=1.0.0"`).
  pub requested: String,
  /// The resolved concrete version string (e.g. `"1.2.3"`).
  pub resolved: String,
  /// The strategy that produced the resolution.
  pub resolution_strategy: ResolutionStrategy,
}

impl VersionResolution {
  /// Create a new resolution record.
  pub fn new(
    requested: impl Into<String>,
    resolved: impl Into<String>,
    strategy: ResolutionStrategy,
  ) -> Self {
    VersionResolution {
      requested: requested.into(),
      resolved: resolved.into(),
      resolution_strategy: strategy,
    }
  }
}

/// A semantic version with major, minor, patch numeric components and an
/// optional pre-release/build suffix.
///
/// Only the numeric `major.minor.patch` prefix participates in ordering;
/// pre-release strings are compared lexically per SemVer when present. This
/// is intentionally lightweight — it parses the common `MAJOR.MINOR.PATCH`
/// forms used by npm, cargo, PyPI, and Go modules without pulling in a full
/// SemVer crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
  /// Major version number.
  pub major: u64,
  /// Minor version number.
  pub minor: u64,
  /// Patch version number.
  pub patch: u64,
  /// Optional pre-release suffix (e.g. `alpha.1`, `rc.2`).
  pub pre: Option<String>,
  /// Optional build metadata suffix (e.g. `build.7`). Ignored for ordering.
  pub build: Option<String>,
}

impl Version {
  /// Parse a version string into a [`Version`].
  ///
  /// Accepts `MAJOR`, `MAJOR.MINOR`, `MAJOR.MINOR.PATCH`, and the standard
  /// SemVer pre-release (`-pre`) and build (`+build`) suffixes. A leading `=`
  /// or `v` is stripped. Returns `None` if the major component is not numeric.
  pub fn parse(s: &str) -> Option<Self> {
    let trimmed = s.trim().trim_start_matches('=').trim_start_matches('v');
    if trimmed.is_empty() {
      return None;
    }
    let (core, rest) = match trimmed.split_once('+') {
      Some((c, b)) => (c, Some(b.to_string())),
      None => (trimmed, None),
    };
    let (numeric, pre) = match core.split_once('-') {
      Some((n, p)) => (n, Some(p.to_string())),
      None => (core, None),
    };
    let mut parts = numeric.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts
      .next()
      .map(|p| p.parse::<u64>().unwrap_or(0))
      .unwrap_or(0);
    let patch = parts
      .next()
      .map(|p| p.parse::<u64>().unwrap_or(0))
      .unwrap_or(0);
    Some(Version {
      major,
      minor,
      patch,
      pre,
      build: rest,
    })
  }

  /// Return the major version number.
  pub fn major(&self) -> u64 {
    self.major
  }

  /// Return the minor version number.
  pub fn minor(&self) -> u64 {
    self.minor
  }
}

impl PartialOrd for Version {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for Version {
  fn cmp(&self, other: &Self) -> Ordering {
    self
      .major
      .cmp(&other.major)
      .then(self.minor.cmp(&other.minor))
      .then(self.patch.cmp(&other.patch))
      .then_with(|| {
        // Per SemVer: a version with a pre-release is LOWER than the same
        // version without one. Two pre-releases compare lexically.
        match (&self.pre, &other.pre) {
          (None, None) => Ordering::Equal,
          (None, Some(_)) => Ordering::Greater,
          (Some(_), None) => Ordering::Less,
          (Some(a), Some(b)) => compare_pre_release(a, b),
        }
      })
  }
}

impl fmt::Display for Version {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
    if let Some(pre) = &self.pre {
      write!(f, "-{pre}")?;
    }
    if let Some(build) = &self.build {
      write!(f, "+{build}")?;
    }
    Ok(())
  }
}

/// Compare two pre-release strings per SemVer dot-separated identifier rules.
fn compare_pre_release(a: &str, b: &str) -> Ordering {
  let a_ids: Vec<&str> = a.split('.').collect();
  let b_ids: Vec<&str> = b.split('.').collect();
  let max = a_ids.len().max(b_ids.len());
  for i in 0..max {
    match (a_ids.get(i), b_ids.get(i)) {
      (Some(a_id), Some(b_id)) => {
        let ord = match (a_id.parse::<u64>(), b_id.parse::<u64>()) {
          (Ok(an), Ok(bn)) => an.cmp(&bn),
          (Ok(_), Err(_)) => Ordering::Less,
          (Err(_), Ok(_)) => Ordering::Greater,
          (Err(_), Err(_)) => a_id.cmp(b_id),
        };
        if ord != Ordering::Equal {
          return ord;
        }
      }
      (Some(_), None) => return Ordering::Greater,
      (None, Some(_)) => return Ordering::Less,
      (None, None) => return Ordering::Equal,
    }
  }
  Ordering::Equal
}

/// A version constraint expression (e.g. `^1.2.3`, `>=1.0`, `1.x`, `1`).
///
/// Supports the caret (`^`), tilde (`~`), comparison (`>=`, `>`, `<=`, `<`,
/// `=`), and wildcard (`x`/`*`) operators used by npm and cargo. A bare major
/// (`4`) is treated as `>=4.0.0 <5.0.0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConstraint {
  /// The constraint operator.
  pub op: ConstraintOp,
  /// The version the operator applies to.
  pub version: Version,
}

/// The operator part of a [`VersionConstraint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintOp {
  /// `^` — compatible-with (same major, or same minor if major is 0).
  Caret,
  /// `~` — approximately (same major and minor).
  Tilde,
  /// `>=`
  Gte,
  /// `>`
  Gt,
  /// `<=`
  Lte,
  /// `<`
  Lt,
  /// `=` — exact match.
  Eq,
  /// Bare major or `1.x` / `1.*` — any minor/patch within the major.
  WildcardMajor,
  /// `1.2.x` / `1.2.*` — any patch within the major.minor.
  WildcardMinor,
}

impl VersionConstraint {
  /// Parse a single constraint expression.
  ///
  /// Returns `None` for unparseable input. Compound ranges (`1.2.3 || 2.0`)
  /// are not supported here — callers split on `||` first.
  pub fn parse(s: &str) -> Option<Self> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
      return None;
    }
    let (op, rest) = if let Some(r) = trimmed.strip_prefix("^") {
      (ConstraintOp::Caret, r)
    } else if let Some(r) = trimmed.strip_prefix("~") {
      (ConstraintOp::Tilde, r)
    } else if let Some(r) = trimmed.strip_prefix(">=") {
      (ConstraintOp::Gte, r)
    } else if let Some(r) = trimmed.strip_prefix("<=") {
      (ConstraintOp::Lte, r)
    } else if let Some(r) = trimmed.strip_prefix(">") {
      (ConstraintOp::Gt, r)
    } else if let Some(r) = trimmed.strip_prefix("<") {
      (ConstraintOp::Lt, r)
    } else if let Some(r) = trimmed.strip_prefix("=") {
      (ConstraintOp::Eq, r)
    } else {
      (ConstraintOp::Eq, trimmed)
    };
    let rest = rest.trim();
    // Detect wildcard major/minor: "1.x", "1.*", "1.2.x", "1.2.*".
    if rest.contains('x') || rest.contains('*') {
      let parts: Vec<&str> = rest.split('.').collect();
      let major = parts.first()?.parse::<u64>().ok()?;
      if parts.len() <= 2 {
        // "1.x", "1.*", "1"
        return Some(VersionConstraint {
          op: ConstraintOp::WildcardMajor,
          version: Version {
            major,
            minor: 0,
            patch: 0,
            pre: None,
            build: None,
          },
        });
      }
      let minor = parts.get(1)?.parse::<u64>().ok()?;
      return Some(VersionConstraint {
        op: ConstraintOp::WildcardMinor,
        version: Version {
          major,
          minor,
          patch: 0,
          pre: None,
          build: None,
        },
      });
    }
    let version = Version::parse(rest)?;
    // A bare major like "4" (no minor/patch in source) is a wildcard major.
    let has_minor = rest.split('.').nth(1).is_some();
    if !has_minor && op == ConstraintOp::Eq {
      return Some(VersionConstraint {
        op: ConstraintOp::WildcardMajor,
        version,
      });
    }
    Some(VersionConstraint { op, version })
  }

  /// Test whether `candidate` satisfies this constraint.
  pub fn satisfies(&self, candidate: &Version) -> bool {
    match self.op {
      ConstraintOp::Eq => candidate == &self.version,
      ConstraintOp::Gt => candidate > &self.version,
      ConstraintOp::Gte => candidate >= &self.version,
      ConstraintOp::Lt => candidate < &self.version,
      ConstraintOp::Lte => candidate <= &self.version,
      ConstraintOp::Caret => {
        // ^MAJOR.MINOR.PATCH: >= version and < next incompatible.
        if candidate < &self.version {
          return false;
        }
        if self.version.major > 0 {
          candidate.major == self.version.major
        } else if self.version.minor > 0 {
          candidate.major == 0 && candidate.minor == self.version.minor
        } else {
          candidate.major == 0 && candidate.minor == 0 && candidate.patch == self.version.patch
        }
      }
      ConstraintOp::Tilde => {
        if candidate < &self.version {
          return false;
        }
        candidate.major == self.version.major && candidate.minor == self.version.minor
      }
      ConstraintOp::WildcardMajor => candidate.major == self.version.major,
      ConstraintOp::WildcardMinor => {
        candidate.major == self.version.major && candidate.minor == self.version.minor
      }
    }
  }
}

/// Configuration for the min-age-days supply-chain defense (PRD FR-4.3–4.5).
///
/// Resolves the effective threshold using the precedence:
///   CLI flag > env var (`APMW_MIN_AGE_DAYS`) > config > default (2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinAgeDaysConfig {
  /// The effective minimum age in days. `0` disables the check.
  pub min_age_days: u32,
}

/// The default min-age-days threshold (PRD FR-4.3).
pub const DEFAULT_MIN_AGE_DAYS: u32 = 2;
/// The environment variable name for overriding min-age-days (PRD FR-4.4).
pub const ENV_MIN_AGE_DAYS: &str = "APMW_MIN_AGE_DAYS";

impl Default for MinAgeDaysConfig {
  fn default() -> Self {
    MinAgeDaysConfig {
      min_age_days: DEFAULT_MIN_AGE_DAYS,
    }
  }
}

impl MinAgeDaysConfig {
  /// Resolve the effective min-age-days from the precedence chain.
  ///
  /// - `cli_override`: `Some(n)` from `--min-age-days <N>` or `--allow-new`
  ///   (which passes `Some(0)`). Highest precedence.
  /// - `config_value`: the `min_age_days` from the TOML config file.
  /// - The env var `APMW_MIN_AGE_DAYS` is read directly and sits between CLI
  ///   and config.
  pub fn resolve(cli_override: Option<u32>, config_value: Option<u32>) -> Self {
    if let Some(cli) = cli_override {
      return MinAgeDaysConfig { min_age_days: cli };
    }
    if let Ok(raw) = std::env::var(ENV_MIN_AGE_DAYS) {
      if let Ok(parsed) = raw.parse::<u32>() {
        return MinAgeDaysConfig {
          min_age_days: parsed,
        };
      }
    }
    if let Some(cfg) = config_value {
      return MinAgeDaysConfig { min_age_days: cfg };
    }
    MinAgeDaysConfig::default()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_version_resolution_serde() {
    let r = VersionResolution::new("", "1.2.3", ResolutionStrategy::Latest);
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"resolution_strategy\":\"latest\""));
    let back: VersionResolution = serde_json::from_str(&json).unwrap();
    assert_eq!(r, back);
  }

  #[test]
  fn test_version_resolution_strategy_display() {
    assert_eq!(ResolutionStrategy::Pinned.to_string(), "pinned");
    assert_eq!(
      ResolutionStrategy::EngineCompatible.to_string(),
      "engine_compatible"
    );
    assert_eq!(ResolutionStrategy::Latest.to_string(), "latest");
    assert_eq!(ResolutionStrategy::LatestMinor.to_string(), "latest_minor");
  }

  #[test]
  fn test_version_parse_basic() {
    let v = Version::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.to_string(), "1.2.3");
  }

  #[test]
  fn test_version_parse_partial() {
    let v = Version::parse("4").unwrap();
    assert_eq!(v.major, 4);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
  }

  #[test]
  fn test_version_parse_pre_release() {
    let v = Version::parse("1.0.0-alpha.1").unwrap();
    assert_eq!(v.pre, Some("alpha.1".to_string()));
    assert_eq!(v.to_string(), "1.0.0-alpha.1");
  }

  #[test]
  fn test_version_parse_build() {
    let v = Version::parse("1.0.0+build.7").unwrap();
    assert_eq!(v.build, Some("build.7".to_string()));
  }

  #[test]
  fn test_version_parse_v_prefix() {
    let v = Version::parse("v1.2.3").unwrap();
    assert_eq!(v.major, 1);
  }

  #[test]
  fn test_version_parse_equals_prefix() {
    let v = Version::parse("=1.2.3").unwrap();
    assert_eq!(v.major, 1);
  }

  #[test]
  fn test_version_parse_invalid() {
    assert!(Version::parse("").is_none());
    assert!(Version::parse("abc").is_none());
  }

  #[test]
  fn test_version_ordering() {
    assert!(Version::parse("1.2.3").unwrap() < Version::parse("1.2.4").unwrap());
    assert!(Version::parse("1.2.3").unwrap() < Version::parse("1.3.0").unwrap());
    assert!(Version::parse("1.2.3").unwrap() < Version::parse("2.0.0").unwrap());
  }

  #[test]
  fn test_version_pre_release_ordering() {
    // 1.0.0-alpha < 1.0.0
    assert!(Version::parse("1.0.0-alpha").unwrap() < Version::parse("1.0.0").unwrap());
    // 1.0.0-alpha.1 < 1.0.0-alpha.2
    assert!(Version::parse("1.0.0-alpha.1").unwrap() < Version::parse("1.0.0-alpha.2").unwrap());
    // 1.0.0-rc.1 < 1.0.0-rc.2
    assert!(Version::parse("1.0.0-rc.1").unwrap() < Version::parse("1.0.0-rc.2").unwrap());
  }

  #[test]
  fn test_constraint_caret() {
    let c = VersionConstraint::parse("^1.2.3").unwrap();
    assert!(c.satisfies(&Version::parse("1.2.3").unwrap()));
    assert!(c.satisfies(&Version::parse("1.9.0").unwrap()));
    assert!(!c.satisfies(&Version::parse("2.0.0").unwrap()));
    assert!(!c.satisfies(&Version::parse("1.2.2").unwrap()));
  }

  #[test]
  fn test_constraint_tilde() {
    let c = VersionConstraint::parse("~1.2.3").unwrap();
    assert!(c.satisfies(&Version::parse("1.2.3").unwrap()));
    assert!(c.satisfies(&Version::parse("1.2.9").unwrap()));
    assert!(!c.satisfies(&Version::parse("1.3.0").unwrap()));
  }

  #[test]
  fn test_constraint_gte() {
    let c = VersionConstraint::parse(">=1.0.0").unwrap();
    assert!(c.satisfies(&Version::parse("1.0.0").unwrap()));
    assert!(c.satisfies(&Version::parse("2.0.0").unwrap()));
    assert!(!c.satisfies(&Version::parse("0.9.0").unwrap()));
  }

  #[test]
  fn test_constraint_wildcard_major() {
    let c = VersionConstraint::parse("4").unwrap();
    assert_eq!(c.op, ConstraintOp::WildcardMajor);
    assert!(c.satisfies(&Version::parse("4.0.0").unwrap()));
    assert!(c.satisfies(&Version::parse("4.99.99").unwrap()));
    assert!(!c.satisfies(&Version::parse("5.0.0").unwrap()));
  }

  #[test]
  fn test_constraint_wildcard_x() {
    let c = VersionConstraint::parse("1.2.x").unwrap();
    assert_eq!(c.op, ConstraintOp::WildcardMinor);
    assert!(c.satisfies(&Version::parse("1.2.0").unwrap()));
    assert!(c.satisfies(&Version::parse("1.2.99").unwrap()));
    assert!(!c.satisfies(&Version::parse("1.3.0").unwrap()));
  }

  #[test]
  fn test_min_age_days_default() {
    std::env::remove_var(ENV_MIN_AGE_DAYS);
    let cfg = MinAgeDaysConfig::resolve(None, None);
    assert_eq!(cfg.min_age_days, DEFAULT_MIN_AGE_DAYS);
  }

  #[test]
  fn test_min_age_days_cli_override() {
    std::env::set_var(ENV_MIN_AGE_DAYS, "5");
    let cfg = MinAgeDaysConfig::resolve(Some(0), Some(3));
    assert_eq!(cfg.min_age_days, 0);
    std::env::remove_var(ENV_MIN_AGE_DAYS);
  }

  #[test]
  fn test_min_age_days_env_override() {
    std::env::set_var(ENV_MIN_AGE_DAYS, "7");
    let cfg = MinAgeDaysConfig::resolve(None, Some(3));
    assert_eq!(cfg.min_age_days, 7);
    std::env::remove_var(ENV_MIN_AGE_DAYS);
  }

  #[test]
  fn test_min_age_days_config_override() {
    std::env::remove_var(ENV_MIN_AGE_DAYS);
    let cfg = MinAgeDaysConfig::resolve(None, Some(10));
    assert_eq!(cfg.min_age_days, 10);
  }
}
