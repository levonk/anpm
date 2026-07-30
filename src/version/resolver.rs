//! Version resolution logic — the `VersionResolver` and its supporting types.
//!
//! This module implements the priority chain described in the parent
//! [`mod@crate::version`] module:
//!   1. **Pinned** — read from a lockfile.
//!   2. **Engine-compatible** — latest version satisfying project engine
//!      constraints.
//!   3. **Latest** — newest version from the registry.
//!   4. **Latest-minor** — newest minor of a specified major.
//!
//! The min-age-days supply-chain defense (PRD FR-4.3–4.5) is applied only to
//! the auto-resolved strategies (3 and 4). Pinned and engine-compatible
//! resolutions bypass the age check.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::error::{ApmwError, Result};

use super::{MinAgeDaysConfig, ResolutionStrategy, Version, VersionConstraint, VersionResolution};

/// The kind of lockfile that pins a package version (PRD FR-4.1 priority 1).
///
/// Each variant maps to a parser that extracts the pinned version for a
/// single named package. Lockfiles that pin many packages (Cargo.lock,
/// package-lock.json, pnpm-lock.yaml, yarn.lock, poetry.lock) are scanned for
/// the requested package name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockfileKind {
  /// `Cargo.lock` — Rust.
  Cargo,
  /// `package-lock.json` — npm.
  Npm,
  /// `pnpm-lock.yaml` — pnpm.
  Pnpm,
  /// `yarn.lock` — Yarn (v1+).
  Yarn,
  /// `poetry.lock` — Poetry (Python).
  Poetry,
}

impl LockfileKind {
  /// Return the canonical file name for this lockfile kind.
  pub fn file_name(self) -> &'static str {
    match self {
      LockfileKind::Cargo => "Cargo.lock",
      LockfileKind::Npm => "package-lock.json",
      LockfileKind::Pnpm => "pnpm-lock.yaml",
      LockfileKind::Yarn => "yarn.lock",
      LockfileKind::Poetry => "poetry.lock",
    }
  }

  /// Detect the lockfile kind from a file name.
  pub fn from_file_name(name: &str) -> Option<Self> {
    match name {
      "Cargo.lock" => Some(LockfileKind::Cargo),
      "package-lock.json" => Some(LockfileKind::Npm),
      "pnpm-lock.yaml" => Some(LockfileKind::Pnpm),
      "yarn.lock" => Some(LockfileKind::Yarn),
      "poetry.lock" => Some(LockfileKind::Poetry),
      _ => None,
    }
  }
}

impl std::fmt::Display for LockfileKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.file_name())
  }
}

/// Engine constraints extracted from a project's configuration files
/// (PRD FR-4.1 priority 2).
///
/// These describe the version ranges the project's runtime/toolchain
/// supports. The resolver uses them to filter registry versions when no
/// lockfile pin is found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineConstraints {
  /// `engines.node` / `engines.npm` style constraints from `package.json`.
  /// Each entry is a raw constraint string (e.g. `">=18"`, `"^4"`).
  pub node_engines: Vec<String>,
  /// `python-requires` / `requires-python` from `setup.py` / `pyproject.toml`.
  pub python_requires: Option<String>,
  /// `rust-version` (MSRV) from `Cargo.toml`.
  pub rust_version: Option<String>,
}

impl EngineConstraints {
  /// Returns `true` if no constraints were extracted.
  pub fn is_empty(&self) -> bool {
    self.node_engines.is_empty() && self.python_requires.is_none() && self.rust_version.is_none()
  }

  /// Read engine constraints from a project directory.
  ///
  /// Looks for `package.json`, `pyproject.toml`, `setup.py`, and `Cargo.toml`
  /// in the given directory. Missing files are silently skipped.
  pub fn from_project_dir(dir: &Path) -> Self {
    let mut constraints = EngineConstraints::default();
    if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
      if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(engines) = value.get("engines").and_then(|e| e.as_object()) {
          for (key, val) in engines {
            if let Some(s) = val.as_str() {
              if key == "node" || key == "npm" {
                constraints.node_engines.push(s.to_string());
              }
            }
          }
        }
      }
    }
    if let Ok(content) = std::fs::read_to_string(dir.join("Cargo.toml")) {
      if let Ok(value) = content.parse::<toml::Value>() {
        if let Some(rust_version) = value
          .get("package")
          .and_then(|p| p.get("rust-version"))
          .and_then(|v| v.as_str())
        {
          constraints.rust_version = Some(rust_version.to_string());
        }
      }
    }
    if let Ok(content) = std::fs::read_to_string(dir.join("pyproject.toml")) {
      if let Ok(value) = content.parse::<toml::Value>() {
        if let Some(req) = value
          .get("project")
          .and_then(|p| p.get("requires-python"))
          .and_then(|v| v.as_str())
        {
          constraints.python_requires = Some(req.to_string());
        }
      }
    }
    if constraints.is_empty() {
      if let Ok(content) = std::fs::read_to_string(dir.join("setup.py")) {
        // Lightweight scan for python_requires="...".
        if let Some(idx) = content.find("python_requires") {
          let rest = &content[idx..];
          if let Some(start) = rest.find(['"', '\'']) {
            let quote = rest.as_bytes()[start] as char;
            let after = &rest[start + 1..];
            if let Some(end) = after.find(quote) {
              constraints.python_requires = Some(after[..end].to_string());
            }
          }
        }
      }
    }
    constraints
  }
}

/// A version published to a package registry, with its publish date.
///
/// The publish date is used by the min-age-days defense. When a registry
/// does not provide publish dates, `published_at` is `None` and the age
/// check is skipped for that version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryVersion {
  /// The version string (e.g. `"1.2.3"`).
  pub version: String,
  /// When the version was published to the registry, if known.
  pub published_at: Option<DateTime<Utc>>,
}

impl RegistryVersion {
  /// Create a registry version with no publish date.
  pub fn new(version: impl Into<String>) -> Self {
    RegistryVersion {
      version: version.into(),
      published_at: None,
    }
  }

  /// Create a registry version with a publish date.
  pub fn with_date(version: impl Into<String>, published_at: DateTime<Utc>) -> Self {
    RegistryVersion {
      version: version.into(),
      published_at: Some(published_at),
    }
  }

  /// Parse the version string into a [`Version`].
  pub fn parsed(&self) -> Option<Version> {
    Version::parse(&self.version)
  }
}

/// A client that queries a package registry for published versions.
///
/// Implementations are expected to return versions sorted newest-first by the
/// resolver (the resolver sorts defensively regardless).
pub trait RegistryClient: Send + Sync {
  /// Return all published (non-yanked) versions for `package` in the
  /// `manager` ecosystem, newest-first. An empty vector means the package
  /// was not found or the registry returned no versions.
  fn list_versions(&self, manager: &str, package: &str) -> Result<Vec<RegistryVersion>>;
}

/// A mock registry client for unit tests.
///
/// Holds a simple in-memory map keyed by `manager/package` returning a cloned
/// vector of versions. Does not perform any network I/O.
#[derive(Debug, Clone, Default)]
pub struct MockRegistryClient {
  entries: std::collections::HashMap<String, Vec<RegistryVersion>>,
}

impl MockRegistryClient {
  /// Create an empty mock client.
  pub fn new() -> Self {
    MockRegistryClient::default()
  }

  /// Register versions for a `manager`/`package` pair.
  pub fn with(mut self, manager: &str, package: &str, versions: Vec<RegistryVersion>) -> Self {
    self
      .entries
      .insert(format!("{manager}/{package}"), versions);
    self
  }

  fn key(manager: &str, package: &str) -> String {
    format!("{manager}/{package}")
  }
}

impl RegistryClient for MockRegistryClient {
  fn list_versions(&self, manager: &str, package: &str) -> Result<Vec<RegistryVersion>> {
    Ok(
      self
        .entries
        .get(&Self::key(manager, package))
        .cloned()
        .unwrap_or_default(),
    )
  }
}

/// A [`RegistryClient`] backed by HTTP calls via `ureq`.
///
/// Currently supports the npm registry (`https://registry.npmjs.org`) and
/// crates.io (`https://crates.io/api/v1/crates`). Other ecosystems return an
/// empty list (the resolver falls back to other strategies). Network errors
/// are logged and returned as an empty list rather than failing the whole
/// resolution — a registry outage should not block a pinned resolution.
pub struct UreqRegistryClient {
  agent: ureq::Agent,
}

impl UreqRegistryClient {
  /// Create a new HTTP registry client with a 5-second timeout.
  pub fn new() -> Self {
    let agent = ureq::AgentBuilder::new()
      .timeout(std::time::Duration::from_secs(5))
      .build();
    UreqRegistryClient { agent }
  }

  fn query_npm(&self, package: &str) -> Vec<RegistryVersion> {
    let url = format!("https://registry.npmjs.org/{package}");
    match self.agent.get(&url).call() {
      Ok(resp) => match resp.into_json::<serde_json::Value>() {
        Ok(value) => parse_npm_versions(&value),
        Err(e) => {
          error!("npm registry JSON parse failed for {package}: {e}");
          Vec::new()
        }
      },
      Err(e) => {
        debug!("npm registry query failed for {package}: {e}");
        Vec::new()
      }
    }
  }

  fn query_crates_io(&self, package: &str) -> Vec<RegistryVersion> {
    let url = format!("https://crates.io/api/v1/crates/{package}");
    match self
      .agent
      .get(&url)
      .set("User-Agent", "apmw (https://github.com/levonk/apmw)")
      .call()
    {
      Ok(resp) => match resp.into_json::<serde_json::Value>() {
        Ok(value) => parse_crates_io_versions(&value),
        Err(e) => {
          error!("crates.io JSON parse failed for {package}: {e}");
          Vec::new()
        }
      },
      Err(e) => {
        debug!("crates.io query failed for {package}: {e}");
        Vec::new()
      }
    }
  }
}

impl Default for UreqRegistryClient {
  fn default() -> Self {
    UreqRegistryClient::new()
  }
}

impl RegistryClient for UreqRegistryClient {
  fn list_versions(&self, manager: &str, package: &str) -> Result<Vec<RegistryVersion>> {
    let versions = match manager {
      "npm" | "pnpm" | "yarn" | "bun" => self.query_npm(package),
      "cargo" => self.query_crates_io(package),
      _ => {
        debug!("no registry endpoint configured for manager {manager}");
        Vec::new()
      }
    };
    Ok(versions)
  }
}

/// Parse the npm registry response into registry versions.
fn parse_npm_versions(value: &serde_json::Value) -> Vec<RegistryVersion> {
  let mut out = Vec::new();
  if let Some(times) = value.get("time").and_then(|t| t.as_object()) {
    for (version, ts) in times {
      if version == "created" || version == "modified" {
        continue;
      }
      let published_at = ts
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
      out.push(RegistryVersion {
        version: version.clone(),
        published_at,
      });
    }
  } else if let Some(versions) = value.get("versions").and_then(|v| v.as_object()) {
    for version in versions.keys() {
      out.push(RegistryVersion::new(version));
    }
  }
  sort_newest_first(&mut out);
  out
}

/// Parse the crates.io API response into registry versions.
fn parse_crates_io_versions(value: &serde_json::Value) -> Vec<RegistryVersion> {
  let mut out = Vec::new();
  if let Some(versions) = value.get("versions").and_then(|v| v.as_array()) {
    for entry in versions {
      let version = entry
        .get("num")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
      if version.is_empty() {
        continue;
      }
      let published_at = entry
        .get("created_at")
        .and_then(|c| c.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));
      let yanked = entry
        .get("yanked")
        .and_then(|y| y.as_bool())
        .unwrap_or(false);
      if !yanked {
        out.push(RegistryVersion {
          version,
          published_at,
        });
      }
    }
  }
  sort_newest_first(&mut out);
  out
}

/// Sort registry versions newest-first by version (and publish date when
/// available as a tiebreaker).
fn sort_newest_first(versions: &mut [RegistryVersion]) {
  versions.sort_by(|a, b| {
    let av = a.parsed();
    let bv = b.parsed();
    match (av, bv) {
      (Some(av), Some(bv)) => bv.cmp(&av),
      _ => b.version.cmp(&a.version),
    }
  });
}

/// The version resolver (PRD FR-4.1).
///
/// Holds a registry client and the resolved min-age-days configuration. The
/// resolver is cheap to clone and may be reused across resolution requests.
#[derive(Debug, Clone)]
pub struct VersionResolver<C: RegistryClient = MockRegistryClient> {
  registry: C,
  min_age: MinAgeDaysConfig,
}

impl<C: RegistryClient> VersionResolver<C> {
  /// Create a new resolver with the given registry client and min-age config.
  pub fn new(registry: C, min_age: MinAgeDaysConfig) -> Self {
    VersionResolver { registry, min_age }
  }

  /// The effective min-age-days configuration.
  pub fn min_age_days(&self) -> &MinAgeDaysConfig {
    &self.min_age
  }

  /// Resolve a version for `package` in `manager` using the priority chain.
  ///
  /// `project_dir` is scanned for a lockfile and engine constraints. The
  /// `requested` spec is the user's version request:
  ///   - `""` → latest (priority 3)
  ///   - `"4"` → latest minor of major 4 (priority 4)
  ///   - `"^1.2"` / `">=1.0"` / `"=1.0.0"` → engine-compatible (priority 2)
  ///   - a lockfile pin always wins when present (priority 1)
  pub fn resolve(
    &self,
    manager: &str,
    package: &str,
    requested: &str,
    project_dir: &Path,
  ) -> Result<VersionResolution> {
    // Priority 1 — pinned in a lockfile.
    if let Some(pinned) = self.resolve_pinned(manager, package, project_dir)? {
      info!("resolved {package} via pinned lockfile: {pinned}");
      return Ok(VersionResolution::new(
        requested,
        pinned,
        ResolutionStrategy::Pinned,
      ));
    }

    // Determine whether the request is a constraint (priority 2) or a bare
    // major / empty (priorities 3 / 4).
    let requested_trimmed = requested.trim();
    let is_constraint = is_constraint_spec(requested_trimmed);

    // Priority 2 — engine-compatible.
    if is_constraint {
      if let Some(resolved) = self.resolve_engine_compatible(manager, package, requested_trimmed)? {
        info!("resolved {package} via engine-compatible: {resolved}");
        return Ok(VersionResolution::new(
          requested,
          resolved,
          ResolutionStrategy::EngineCompatible,
        ));
      }
    }

    // Priority 4 — latest-minor (a bare major like "4").
    if let Some(major) = parse_bare_major(requested_trimmed) {
      if let Some(resolved) = self.resolve_latest_minor(manager, package, major)? {
        info!("resolved {package} via latest-minor (major {major}): {resolved}");
        return Ok(VersionResolution::new(
          requested,
          resolved,
          ResolutionStrategy::LatestMinor,
        ));
      }
    }

    // Priority 3 — latest.
    if let Some(resolved) = self.resolve_latest(manager, package)? {
      info!("resolved {package} via latest: {resolved}");
      return Ok(VersionResolution::new(
        requested,
        resolved,
        ResolutionStrategy::Latest,
      ));
    }

    Err(ApmwError::VersionResolution(format!(
      "no version found for {package} (manager {manager}, requested {requested:?})"
    )))
  }

  /// Priority 1 — read the pinned version from a lockfile in `project_dir`.
  fn resolve_pinned(
    &self,
    manager: &str,
    package: &str,
    project_dir: &Path,
  ) -> Result<Option<String>> {
    let candidates = lockfile_candidates(manager);
    for kind in candidates {
      let path = project_dir.join(kind.file_name());
      if !path.is_file() {
        continue;
      }
      let content = std::fs::read_to_string(&path).map_err(ApmwError::from)?;
      if let Some(pinned) = parse_lockfile_version(*kind, &content, package) {
        debug!("lockfile {} pinned {package} to {pinned}", kind);
        return Ok(Some(pinned));
      }
    }
    Ok(None)
  }

  /// Priority 2 — latest version satisfying the requested constraint.
  fn resolve_engine_compatible(
    &self,
    manager: &str,
    package: &str,
    requested: &str,
  ) -> Result<Option<String>> {
    let constraint = match VersionConstraint::parse(requested) {
      Some(c) => c,
      None => return Ok(None),
    };
    let versions = self.registry.list_versions(manager, package)?;
    for rv in &versions {
      if let Some(v) = rv.parsed() {
        if constraint.satisfies(&v) {
          // Engine-compatible bypasses the age check (PRD FR-4.5).
          return Ok(Some(rv.version.clone()));
        }
      }
    }
    Ok(None)
  }

  /// Priority 3 — newest version, applying the min-age-days defense.
  fn resolve_latest(&self, manager: &str, package: &str) -> Result<Option<String>> {
    let versions = self.registry.list_versions(manager, package)?;
    Ok(apply_min_age(&versions, &self.min_age).map(|rv| rv.version.clone()))
  }

  /// Priority 4 — newest minor of `major`, applying the min-age-days defense.
  fn resolve_latest_minor(
    &self,
    manager: &str,
    package: &str,
    major: u64,
  ) -> Result<Option<String>> {
    let versions = self.registry.list_versions(manager, package)?;
    let filtered: Vec<RegistryVersion> = versions
      .into_iter()
      .filter(|rv| rv.parsed().map(|v| v.major == major).unwrap_or(false))
      .collect();
    Ok(apply_min_age(&filtered, &self.min_age).map(|rv| rv.version.clone()))
  }
}

/// Return the lockfile kinds that may pin a version for `manager`.
fn lockfile_candidates(manager: &str) -> &'static [LockfileKind] {
  match manager {
    "cargo" => &[LockfileKind::Cargo],
    "npm" => &[LockfileKind::Npm, LockfileKind::Yarn],
    "pnpm" => &[LockfileKind::Pnpm],
    "yarn" => &[LockfileKind::Yarn],
    "bun" => &[LockfileKind::Npm],
    "poetry" => &[LockfileKind::Poetry],
    "uv" | "pip" | "pdm" => &[LockfileKind::Poetry],
    _ => &[],
  }
}

/// Extract the pinned version of `package` from a lockfile body.
fn parse_lockfile_version(kind: LockfileKind, content: &str, package: &str) -> Option<String> {
  match kind {
    LockfileKind::Cargo => parse_cargo_lock(content, package),
    LockfileKind::Npm => parse_npm_lock(content, package),
    LockfileKind::Pnpm => parse_pnpm_lock(content, package),
    LockfileKind::Yarn => parse_yarn_lock(content, package),
    LockfileKind::Poetry => parse_poetry_lock(content, package),
  }
}

/// Parse `Cargo.lock` for a crate's version.
fn parse_cargo_lock(content: &str, package: &str) -> Option<String> {
  let value: toml::Value = content.parse().ok()?;
  let packages = value.get("package")?.as_array()?;
  for entry in packages {
    let name = entry.get("name").and_then(|n| n.as_str())?;
    if name == package {
      return entry
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from);
    }
  }
  None
}

/// Parse `package-lock.json` for a package's version.
fn parse_npm_lock(content: &str, package: &str) -> Option<String> {
  let value: serde_json::Value = serde_json::from_str(content).ok()?;
  // lockfileVersion 2/3 nests under packages; v1 nests under dependencies.
  if let Some(packages) = value.get("packages").and_then(|p| p.as_object()) {
    // The root package is keyed by "". A dependency is keyed by
    // "node_modules/<name>" or a nested path ending in it.
    for (key, entry) in packages {
      if key.is_empty() {
        continue;
      }
      let leaf = key.rsplit("node_modules/").next().unwrap_or(key);
      if leaf == package {
        if let Some(v) = entry.get("version").and_then(|v| v.as_str()) {
          return Some(v.to_string());
        }
      }
    }
  }
  if let Some(deps) = value.get("dependencies").and_then(|d| d.as_object()) {
    if let Some(entry) = deps.get(package) {
      if let Some(v) = entry.get("version").and_then(|v| v.as_str()) {
        return Some(v.to_string());
      }
    }
  }
  None
}

/// Parse `pnpm-lock.yaml` for a package's version.
///
/// pnpm lockfiles (v5+) list packages under `packages:` keyed by
/// `/name@version` or `name@version(peer)`. We extract the first matching
/// version for `package`.
fn parse_pnpm_lock(content: &str, package: &str) -> Option<String> {
  // pnpm lockfiles are YAML; toml::Value won't parse them. Fall back to a
  // line scan for `/package@version` or `package@version:` entries.
  scan_yaml_for_package_version(content, package)
}

/// Parse `yarn.lock` for a package's version.
///
/// yarn.lock v1 entries look like:
/// ```text
/// "package@^1.2.3", "package@npm:^1.2.3":
///   version "1.2.3"
/// ```
fn parse_yarn_lock(content: &str, package: &str) -> Option<String> {
  let mut lines = content.lines().peekable();
  while let Some(line) = lines.next() {
    let trimmed = line.trim_end();
    if trimmed.is_empty() || trimmed.starts_with('#') {
      continue;
    }
    // A key line ends with `:` and may list multiple spec strings.
    if trimmed.ends_with(':') && line_starts_with_package_spec(trimmed, package) {
      // The next non-empty, indented line that starts with `version` is the
      // resolved version.
      for body in lines.by_ref() {
        let body = body.trim();
        if body.is_empty() {
          continue;
        }
        if let Some(rest) = body.strip_prefix("version ") {
          return Some(rest.trim_matches('"').to_string());
        }
        if let Some(rest) = body.strip_prefix("version \"") {
          return Some(rest.trim_end_matches('"').to_string());
        }
        // If we hit another key line, stop.
        if !body.starts_with(char::is_numeric)
          && !body.starts_with('"')
          && !body.starts_with('\'')
          && !body.contains(' ')
        {
          break;
        }
      }
    }
  }
  None
}

/// Parse `poetry.lock` for a package's version.
fn parse_poetry_lock(content: &str, package: &str) -> Option<String> {
  let value: toml::Value = content.parse().ok()?;
  let packages = value.get("package")?.as_array()?;
  for entry in packages {
    let name = entry.get("name").and_then(|n| n.as_str())?;
    if name == package {
      return entry
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from);
    }
  }
  None
}

/// Lightweight scan for `package@version` in a YAML lockfile body.
fn scan_yaml_for_package_version(content: &str, package: &str) -> Option<String> {
  let needle = format!("{package}@");
  for line in content.lines() {
    let trimmed = line.trim();
    if let Some(idx) = trimmed.find(&needle) {
      let after = &trimmed[idx + needle.len()..];
      // The version runs until `:`, `(`, `,`, or whitespace.
      let end = after
        .find(|c: char| c == ':' || c == '(' || c == ',' || c.is_whitespace())
        .unwrap_or(after.len());
      let version = &after[..end];
      if !version.is_empty() {
        return Some(version.to_string());
      }
    }
  }
  None
}

/// Returns true if a yarn.lock key line references `package`.
fn line_starts_with_package_spec(key_line: &str, package: &str) -> bool {
  // Strip the trailing `:`.
  let body = key_line.trim_end_matches(':').trim();
  // Each spec is quoted; split on `,` and check each.
  for spec in body.split(',') {
    let spec = spec.trim().trim_matches('"').trim_matches('\'');
    // Strip any `npm:` or `patch:` prefix.
    let spec = spec.rsplit("npm:").next().unwrap_or(spec);
    // The package name is everything up to the last `@` (names can contain
    // scopes like `@scope/pkg`).
    if let Some(name) = spec.rsplit_once('@').map(|(n, _)| n) {
      if name == package {
        return true;
      }
    }
  }
  false
}

/// Apply the min-age-days defense to a newest-first sorted list.
///
/// Returns the first version that is at least `min_age_days` old, or the
/// newest version when the threshold is `0` or no publish dates are present.
fn apply_min_age(versions: &[RegistryVersion], cfg: &MinAgeDaysConfig) -> Option<RegistryVersion> {
  if versions.is_empty() {
    return None;
  }
  if cfg.min_age_days == 0 {
    return Some(versions[0].clone());
  }
  let now = Utc::now();
  let threshold = chrono::Duration::days(cfg.min_age_days as i64);
  // First, try to find a version with a publish date that is old enough.
  for rv in versions {
    if let Some(published) = rv.published_at {
      if now - published >= threshold {
        return Some(rv.clone());
      }
    }
  }
  // If no version has a publish date, skip the age check and return the
  // newest (PRD: "If a registry source does not provide publish dates, the
  // age check is skipped for that source").
  let any_dates = versions.iter().any(|rv| rv.published_at.is_some());
  if !any_dates {
    return Some(versions[0].clone());
  }
  // All versions are too fresh — return None so the caller can fall back to
  // another strategy or error.
  None
}

/// Returns true if `spec` looks like a constraint (starts with an operator
/// or contains a `.` after a digit).
fn is_constraint_spec(spec: &str) -> bool {
  if spec.is_empty() {
    return false;
  }
  let first = spec.chars().next().unwrap();
  if matches!(first, '^' | '~' | '>' | '<' | '=') {
    return true;
  }
  // "1.2.3" (exact) is a constraint; "4" (bare major) is not.
  spec.contains('.') || spec.contains('x') || spec.contains('*')
}

/// Parse a bare major like `"4"` into `Some(4)`. Returns `None` for empty or
/// non-numeric specs.
fn parse_bare_major(spec: &str) -> Option<u64> {
  if spec.is_empty() {
    return None;
  }
  // A bare major is a pure number with no operators, dots, or wildcards.
  if spec.chars().any(|c| !c.is_ascii_digit()) {
    return None;
  }
  spec.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;

  fn mock_with_versions(
    manager: &str,
    package: &str,
    versions: Vec<RegistryVersion>,
  ) -> VersionResolver<MockRegistryClient> {
    let registry = MockRegistryClient::new().with(manager, package, versions);
    VersionResolver::new(registry, MinAgeDaysConfig { min_age_days: 0 })
  }

  fn days_ago(n: i64) -> DateTime<Utc> {
    Utc::now() - chrono::Duration::days(n)
  }

  #[test]
  fn test_lockfile_kind_from_file_name() {
    assert_eq!(
      LockfileKind::from_file_name("Cargo.lock"),
      Some(LockfileKind::Cargo)
    );
    assert_eq!(
      LockfileKind::from_file_name("package-lock.json"),
      Some(LockfileKind::Npm)
    );
    assert_eq!(
      LockfileKind::from_file_name("pnpm-lock.yaml"),
      Some(LockfileKind::Pnpm)
    );
    assert_eq!(
      LockfileKind::from_file_name("yarn.lock"),
      Some(LockfileKind::Yarn)
    );
    assert_eq!(
      LockfileKind::from_file_name("poetry.lock"),
      Some(LockfileKind::Poetry)
    );
    assert_eq!(LockfileKind::from_file_name("unknown"), None);
  }

  #[test]
  fn test_lockfile_kind_display() {
    assert_eq!(LockfileKind::Cargo.to_string(), "Cargo.lock");
    assert_eq!(LockfileKind::Npm.to_string(), "package-lock.json");
  }

  #[test]
  fn test_engine_constraints_default_empty() {
    let c = EngineConstraints::default();
    assert!(c.is_empty());
  }

  #[test]
  fn test_engine_constraints_from_project_dir_missing() {
    let dir = tempfile::tempdir().unwrap();
    let c = EngineConstraints::from_project_dir(dir.path());
    assert!(c.is_empty());
  }

  #[test]
  fn test_engine_constraints_from_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
      dir.path().join("Cargo.toml"),
      "[package]\nrust-version = \"1.70\"\n",
    )
    .unwrap();
    let c = EngineConstraints::from_project_dir(dir.path());
    assert_eq!(c.rust_version.as_deref(), Some("1.70"));
  }

  #[test]
  fn test_engine_constraints_from_package_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
      dir.path().join("package.json"),
      serde_json::json!({"engines": {"node": ">=18", "npm": ">=9"}}).to_string(),
    )
    .unwrap();
    let c = EngineConstraints::from_project_dir(dir.path());
    assert_eq!(c.node_engines, vec![">=18".to_string(), ">=9".to_string()]);
  }

  #[test]
  fn test_engine_constraints_from_pyproject_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
      dir.path().join("pyproject.toml"),
      "[project]\nrequires-python = \">=3.10\"\n",
    )
    .unwrap();
    let c = EngineConstraints::from_project_dir(dir.path());
    assert_eq!(c.python_requires.as_deref(), Some(">=3.10"));
  }

  #[test]
  fn test_registry_version_new_and_with_date() {
    let rv = RegistryVersion::new("1.0.0");
    assert_eq!(rv.version, "1.0.0");
    assert!(rv.published_at.is_none());
    let dt = Utc.timestamp_opt(1700000000, 0).unwrap();
    let rv2 = RegistryVersion::with_date("1.0.0", dt);
    assert_eq!(rv2.published_at, Some(dt));
  }

  #[test]
  fn test_mock_registry_client_returns_registered() {
    let client = MockRegistryClient::new().with(
      "cargo",
      "serde",
      vec![RegistryVersion::new("1.0.0"), RegistryVersion::new("1.1.0")],
    );
    let versions = client.list_versions("cargo", "serde").unwrap();
    assert_eq!(versions.len(), 2);
  }

  #[test]
  fn test_mock_registry_client_missing_returns_empty() {
    let client = MockRegistryClient::new();
    let versions = client.list_versions("cargo", "serde").unwrap();
    assert!(versions.is_empty());
  }

  #[test]
  fn test_resolve_pinned_from_cargo_lock() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
      dir.path().join("Cargo.lock"),
      "[[package]]\nname = \"serde\"\nversion = \"1.0.193\"\n",
    )
    .unwrap();
    let resolver = mock_with_versions("cargo", "serde", vec![]);
    let resolution = resolver.resolve("cargo", "serde", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "1.0.193");
    assert_eq!(resolution.resolution_strategy, ResolutionStrategy::Pinned);
  }

  #[test]
  fn test_resolve_pinned_from_npm_lock_v3() {
    let dir = tempfile::tempdir().unwrap();
    let lock = serde_json::json!({
      "lockfileVersion": 3,
      "packages": {
        "": {},
        "node_modules/express": { "version": "4.18.2" }
      }
    })
    .to_string();
    std::fs::write(dir.path().join("package-lock.json"), lock).unwrap();
    let resolver = mock_with_versions("npm", "express", vec![]);
    let resolution = resolver.resolve("npm", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "4.18.2");
    assert_eq!(resolution.resolution_strategy, ResolutionStrategy::Pinned);
  }

  #[test]
  fn test_resolve_pinned_from_npm_lock_v1_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let lock = serde_json::json!({
      "lockfileVersion": 1,
      "dependencies": { "express": { "version": "4.17.1" } }
    })
    .to_string();
    std::fs::write(dir.path().join("package-lock.json"), lock).unwrap();
    let resolver = mock_with_versions("npm", "express", vec![]);
    let resolution = resolver.resolve("npm", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "4.17.1");
  }

  #[test]
  fn test_resolve_pinned_from_poetry_lock() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
      dir.path().join("poetry.lock"),
      "[[package]]\nname = \"requests\"\nversion = \"2.31.0\"\n",
    )
    .unwrap();
    let resolver = mock_with_versions("poetry", "requests", vec![]);
    let resolution = resolver
      .resolve("poetry", "requests", "", dir.path())
      .unwrap();
    assert_eq!(resolution.resolved, "2.31.0");
  }

  #[test]
  fn test_resolve_pinned_from_yarn_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock = "\"express@^4.18.0\":\n  version \"4.18.2\"\n";
    std::fs::write(dir.path().join("yarn.lock"), lock).unwrap();
    let resolver = mock_with_versions("yarn", "express", vec![]);
    let resolution = resolver.resolve("yarn", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "4.18.2");
  }

  #[test]
  fn test_resolve_engine_compatible_caret() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![
      RegistryVersion::new("1.9.0"),
      RegistryVersion::new("1.2.3"),
      RegistryVersion::new("2.0.0"),
    ];
    let resolver = mock_with_versions("npm", "express", versions);
    let resolution = resolver
      .resolve("npm", "express", "^1.2.3", dir.path())
      .unwrap();
    assert_eq!(resolution.resolved, "1.9.0");
    assert_eq!(
      resolution.resolution_strategy,
      ResolutionStrategy::EngineCompatible
    );
  }

  #[test]
  fn test_resolve_engine_compatible_gte() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![
      RegistryVersion::new("2.0.0"),
      RegistryVersion::new("1.5.0"),
      RegistryVersion::new("1.0.0"),
    ];
    let resolver = mock_with_versions("cargo", "serde", versions);
    let resolution = resolver
      .resolve("cargo", "serde", ">=1.0.0", dir.path())
      .unwrap();
    assert_eq!(resolution.resolved, "2.0.0");
    assert_eq!(
      resolution.resolution_strategy,
      ResolutionStrategy::EngineCompatible
    );
  }

  #[test]
  fn test_resolve_latest_strategy() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![
      RegistryVersion::with_date("1.5.0", days_ago(10)),
      RegistryVersion::with_date("1.4.0", days_ago(30)),
    ];
    let resolver = mock_with_versions("npm", "express", versions);
    let resolution = resolver.resolve("npm", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "1.5.0");
    assert_eq!(resolution.resolution_strategy, ResolutionStrategy::Latest);
  }

  #[test]
  fn test_resolve_latest_minor_strategy() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![
      RegistryVersion::with_date("5.0.0", days_ago(1)),
      RegistryVersion::with_date("4.5.0", days_ago(10)),
      RegistryVersion::with_date("4.4.0", days_ago(30)),
      RegistryVersion::with_date("3.0.0", days_ago(100)),
    ];
    let resolver = mock_with_versions("npm", "express", versions);
    let resolution = resolver.resolve("npm", "express", "4", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "4.5.0");
    assert_eq!(
      resolution.resolution_strategy,
      ResolutionStrategy::LatestMinor
    );
  }

  #[test]
  fn test_min_age_days_refuses_too_new() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![
      RegistryVersion::with_date("1.5.0", days_ago(1)), // too new
      RegistryVersion::with_date("1.4.0", days_ago(30)), // old enough
    ];
    let registry = MockRegistryClient::new().with("npm", "express", versions);
    let resolver = VersionResolver::new(registry, MinAgeDaysConfig { min_age_days: 2 });
    let resolution = resolver.resolve("npm", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "1.4.0");
    assert_eq!(resolution.resolution_strategy, ResolutionStrategy::Latest);
  }

  #[test]
  fn test_min_age_days_zero_allows_new() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![RegistryVersion::with_date("1.5.0", days_ago(0))];
    let registry = MockRegistryClient::new().with("npm", "express", versions);
    let resolver = VersionResolver::new(registry, MinAgeDaysConfig { min_age_days: 0 });
    let resolution = resolver.resolve("npm", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "1.5.0");
  }

  #[test]
  fn test_min_age_days_skipped_when_no_dates() {
    let dir = tempfile::tempdir().unwrap();
    let versions = vec![RegistryVersion::new("1.5.0"), RegistryVersion::new("1.4.0")];
    let registry = MockRegistryClient::new().with("npm", "express", versions);
    let resolver = VersionResolver::new(registry, MinAgeDaysConfig { min_age_days: 30 });
    let resolution = resolver.resolve("npm", "express", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "1.5.0");
  }

  #[test]
  fn test_min_age_days_does_not_apply_to_engine_compatible() {
    let dir = tempfile::tempdir().unwrap();
    // The newest matching version is 1 day old, but engine-compatible bypasses
    // the age check.
    let versions = vec![RegistryVersion::with_date("1.5.0", days_ago(1))];
    let registry = MockRegistryClient::new().with("npm", "express", versions);
    let resolver = VersionResolver::new(registry, MinAgeDaysConfig { min_age_days: 30 });
    let resolution = resolver
      .resolve("npm", "express", "^1.0.0", dir.path())
      .unwrap();
    assert_eq!(resolution.resolved, "1.5.0");
    assert_eq!(
      resolution.resolution_strategy,
      ResolutionStrategy::EngineCompatible
    );
  }

  #[test]
  fn test_min_age_days_does_not_apply_to_pinned() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
      dir.path().join("Cargo.lock"),
      "[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let resolver = {
      let registry = MockRegistryClient::new();
      VersionResolver::new(registry, MinAgeDaysConfig { min_age_days: 999 })
    };
    let resolution = resolver.resolve("cargo", "serde", "", dir.path()).unwrap();
    assert_eq!(resolution.resolved, "1.0.0");
    assert_eq!(resolution.resolution_strategy, ResolutionStrategy::Pinned);
  }

  #[test]
  fn test_resolve_no_versions_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let resolver = mock_with_versions("npm", "ghost", vec![]);
    let result = resolver.resolve("npm", "ghost", "", dir.path());
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      ApmwError::VersionResolution(_)
    ));
  }

  #[test]
  fn test_parse_cargo_lock_multiple_packages() {
    let content = "[[package]]\nname = \"serde\"\nversion = \"1.0.193\"\n[[package]]\nname = \"tokio\"\nversion = \"1.35.1\"\n";
    assert_eq!(
      parse_cargo_lock(content, "serde"),
      Some("1.0.193".to_string())
    );
    assert_eq!(
      parse_cargo_lock(content, "tokio"),
      Some("1.35.1".to_string())
    );
    assert_eq!(parse_cargo_lock(content, "ghost"), None);
  }

  #[test]
  fn test_parse_yarn_lock_scoped_package() {
    let content = "\"@babel/core@^7.0.0\":\n  version \"7.23.5\"\n";
    assert_eq!(
      parse_yarn_lock(content, "@babel/core"),
      Some("7.23.5".to_string())
    );
  }

  #[test]
  fn test_parse_pnpm_lock_scan() {
    let content = "packages:\n  /express@4.18.2:\n    resolution: {integrity: sha256-...}\n";
    assert_eq!(
      scan_yaml_for_package_version(content, "express"),
      Some("4.18.2".to_string())
    );
  }

  #[test]
  fn test_is_constraint_spec() {
    assert!(is_constraint_spec("^1.2.3"));
    assert!(is_constraint_spec("~1.2.3"));
    assert!(is_constraint_spec(">=1.0.0"));
    assert!(is_constraint_spec("=1.0.0"));
    assert!(is_constraint_spec("1.2.3"));
    assert!(!is_constraint_spec(""));
    assert!(!is_constraint_spec("4"));
  }

  #[test]
  fn test_parse_bare_major() {
    assert_eq!(parse_bare_major("4"), Some(4));
    assert_eq!(parse_bare_major(""), None);
    assert_eq!(parse_bare_major("1.2"), None);
    assert_eq!(parse_bare_major("^1"), None);
  }

  #[test]
  fn test_apply_min_age_returns_newest_when_threshold_zero() {
    let versions = vec![RegistryVersion::with_date("1.5.0", days_ago(0))];
    let cfg = MinAgeDaysConfig { min_age_days: 0 };
    let result = apply_min_age(&versions, &cfg).unwrap();
    assert_eq!(result.version, "1.5.0");
  }

  #[test]
  fn test_apply_min_age_returns_none_when_all_too_new() {
    let versions = vec![
      RegistryVersion::with_date("1.5.0", days_ago(1)),
      RegistryVersion::with_date("1.4.0", days_ago(1)),
    ];
    let cfg = MinAgeDaysConfig { min_age_days: 5 };
    assert!(apply_min_age(&versions, &cfg).is_none());
  }

  #[test]
  fn test_apply_min_age_falls_back_to_oldest_satisfying() {
    let versions = vec![
      RegistryVersion::with_date("1.5.0", days_ago(1)),
      RegistryVersion::with_date("1.4.0", days_ago(1)),
      RegistryVersion::with_date("1.3.0", days_ago(40)),
    ];
    let cfg = MinAgeDaysConfig { min_age_days: 5 };
    let result = apply_min_age(&versions, &cfg).unwrap();
    assert_eq!(result.version, "1.3.0");
  }

  #[test]
  fn test_sort_newest_first() {
    let mut versions = vec![
      RegistryVersion::new("1.0.0"),
      RegistryVersion::new("2.0.0"),
      RegistryVersion::new("1.5.0"),
    ];
    sort_newest_first(&mut versions);
    assert_eq!(versions[0].version, "2.0.0");
    assert_eq!(versions[1].version, "1.5.0");
    assert_eq!(versions[2].version, "1.0.0");
  }

  #[test]
  fn test_lockfile_candidates_for_managers() {
    assert_eq!(lockfile_candidates("cargo"), &[LockfileKind::Cargo]);
    assert_eq!(
      lockfile_candidates("npm"),
      &[LockfileKind::Npm, LockfileKind::Yarn]
    );
    assert_eq!(lockfile_candidates("pnpm"), &[LockfileKind::Pnpm]);
    assert_eq!(lockfile_candidates("yarn"), &[LockfileKind::Yarn]);
    assert_eq!(lockfile_candidates("poetry"), &[LockfileKind::Poetry]);
    assert!(lockfile_candidates("unknown").is_empty());
  }
}
