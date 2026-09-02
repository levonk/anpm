//! Spec loader — fetch, parse, and cache the levonk-packages governance spec.
//!
//! The spec is fetched from
//! `https://raw.githubusercontent.com/levonk/levonk-packages/main/docs/SPEC.md`
//! and parsed for governance rules. A local JSON cache is stored at
//! `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/governance/spec.json` with a 7-day
//! TTL. If the cache is fresh, no network call is made. If the fetch fails and
//! a cached copy exists, the stale cache is used with a warning.
//!
//! Tests use a trait-based HTTP client ([`SpecHttpClient`]) so no real network
//! calls are made.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::error::{ApmwError, Result};

use super::rules::{GovernanceRule, GovernanceType};

/// The default URL for the levonk-packages governance spec (raw markdown).
pub const SPEC_URL: &str =
  "https://raw.githubusercontent.com/levonk/levonk-packages/main/docs/SPEC.md";

/// Cache TTL: 7 days in seconds.
pub const CACHE_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// Errors produced by the spec loader.
#[derive(Error, Debug, Clone)]
pub enum SpecError {
  #[error("HTTP fetch failed: {0}")]
  HttpFetch(String),

  #[error("spec parse error: {0}")]
  Parse(String),

  #[error("cache error: {0}")]
  Cache(String),
}

/// A trait for HTTP clients that fetch the spec text.
///
/// This allows tests to inject a mock client without making real network calls.
pub trait SpecHttpClient: Send + Sync {
  /// Fetch the spec text from the given URL.
  fn fetch_text(&self, url: &str) -> std::result::Result<String, SpecError>;
}

/// A production HTTP client backed by `reqwest` (blocking).
pub struct ReqwestSpecClient {
  timeout: std::time::Duration,
}

impl ReqwestSpecClient {
  /// Create a new reqwest-backed client with a 10-second timeout.
  pub fn new() -> Self {
    ReqwestSpecClient {
      timeout: std::time::Duration::from_secs(10),
    }
  }
}

impl Default for ReqwestSpecClient {
  fn default() -> Self {
    ReqwestSpecClient::new()
  }
}

impl SpecHttpClient for ReqwestSpecClient {
  fn fetch_text(&self, url: &str) -> std::result::Result<String, SpecError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .map_err(|e| SpecError::HttpFetch(format!("failed to create runtime: {e}")))?;
    runtime.block_on(async {
      let client = reqwest::Client::builder()
        .timeout(self.timeout)
        .build()
        .map_err(|e| SpecError::HttpFetch(format!("client build: {e}")))?;
      let resp = client
        .get(url)
        .header("User-Agent", "apmw (https://github.com/levonk/apmw)")
        .send()
        .await
        .map_err(|e| SpecError::HttpFetch(format!("request: {e}")))?;
      if !resp.status().is_success() {
        return Err(SpecError::HttpFetch(format!("HTTP {}", resp.status())));
      }
      resp
        .text()
        .await
        .map_err(|e| SpecError::HttpFetch(format!("body read: {e}")))
    })
  }
}

/// A mock HTTP client for tests.
#[derive(Debug, Clone)]
pub struct MockSpecClient {
  response: std::result::Result<String, SpecError>,
}

impl MockSpecClient {
  /// Create a mock client that returns the given text on fetch.
  pub fn with_ok(text: impl Into<String>) -> Self {
    MockSpecClient {
      response: Ok(text.into()),
    }
  }

  /// Create a mock client that returns an error on fetch.
  pub fn with_err(err: impl Into<String>) -> Self {
    MockSpecClient {
      response: Err(SpecError::HttpFetch(err.into())),
    }
  }
}

impl SpecHttpClient for MockSpecClient {
  fn fetch_text(&self, _url: &str) -> std::result::Result<String, SpecError> {
    self.response.clone()
  }
}

/// The parsed governance spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceSpec {
  /// The spec version string (e.g. "1.0").
  #[serde(default)]
  pub version: String,
  /// The governance rules extracted from the spec.
  pub rules: Vec<GovernanceRule>,
  /// The raw spec source (markdown) for debugging.
  #[serde(default, skip_serializing)]
  pub raw: String,
}

impl GovernanceSpec {
  /// Create an empty spec.
  pub fn empty() -> Self {
    GovernanceSpec {
      version: String::new(),
      rules: Vec::new(),
      raw: String::new(),
    }
  }

  /// Returns `true` if the spec defines the four required governance types.
  pub fn has_all_types(&self) -> bool {
    let mut has_prefer = false;
    let mut has_force = false;
    let mut has_block = false;
    let mut has_eject = false;
    for rule in &self.rules {
      match rule.governance_type {
        GovernanceType::Prefer => has_prefer = true,
        GovernanceType::Force => has_force = true,
        GovernanceType::Block => has_block = true,
        GovernanceType::Eject => has_eject = true,
      }
    }
    has_prefer && has_force && has_block && has_eject
  }

  /// Look up the rule for a given tool.
  pub fn rule_for(&self, tool: &str) -> Option<&GovernanceRule> {
    self.rules.iter().find(|r| r.tool == tool)
  }
}

/// The on-disk cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
  spec: GovernanceSpec,
  fetched_at: chrono::DateTime<chrono::Utc>,
}

/// The spec loader: fetches, parses, and caches the governance spec.
pub struct SpecLoader {
  cache_path: PathBuf,
  ttl: chrono::Duration,
}

impl SpecLoader {
  /// Create a new loader with the default cache path and 7-day TTL.
  pub fn new() -> Self {
    SpecLoader {
      cache_path: default_cache_path(),
      ttl: chrono::Duration::seconds(CACHE_TTL_SECS),
    }
  }

  /// Create a loader with a custom cache path (for testing).
  pub fn with_cache_path(cache_path: PathBuf) -> Self {
    SpecLoader {
      cache_path,
      ttl: chrono::Duration::seconds(CACHE_TTL_SECS),
    }
  }

  /// Create a loader with a custom cache path and TTL (for testing).
  pub fn with_cache_path_and_ttl(cache_path: PathBuf, ttl: chrono::Duration) -> Self {
    SpecLoader { cache_path, ttl }
  }

  /// Load the spec, using the cache if fresh, otherwise fetching via the
  /// provided HTTP client.
  pub fn load(&self, client: &dyn SpecHttpClient) -> Result<GovernanceSpec> {
    if let Some(spec) = self.load_fresh_cache() {
      debug!("governance spec cache hit");
      return Ok(spec);
    }
    debug!("governance spec cache miss or stale; fetching");
    self.fetch_and_cache(client)
  }

  /// Force-refresh the spec, ignoring the cache freshness.
  pub fn refresh(&self, client: &dyn SpecHttpClient) -> Result<GovernanceSpec> {
    info!("force-refreshing governance spec");
    self.fetch_and_cache(client)
  }

  /// Load the cache if it exists and is fresh (within TTL).
  fn load_fresh_cache(&self) -> Option<GovernanceSpec> {
    let entry = self.read_cache().ok()??;
    let now = chrono::Utc::now();
    if now - entry.fetched_at > self.ttl {
      debug!("governance spec cache is stale");
      return None;
    }
    Some(entry.spec)
  }

  /// Fetch the spec from the network, parse it, and write to cache.
  fn fetch_and_cache(&self, client: &dyn SpecHttpClient) -> Result<GovernanceSpec> {
    match client.fetch_text(SPEC_URL) {
      Ok(text) => {
        let spec = parse_spec(&text).map_err(|e| {
          error!(error = %e, "governance spec parse failed");
          ApmwError::Governance(e.to_string())
        })?;
        if let Err(e) = self.write_cache(&spec) {
          warn!(error = %e, "failed to write governance spec cache");
        }
        Ok(spec)
      }
      Err(e) => {
        error!(error = %e, "governance spec fetch failed");
        // Fallback to stale cache if available.
        if let Some(entry) = self.read_cache().ok().flatten() {
          warn!("using stale governance spec cache after fetch failure");
          return Ok(entry.spec);
        }
        Err(ApmwError::Governance(format!("spec fetch failed: {e}")))
      }
    }
  }

  /// Read the cache file from disk.
  fn read_cache(&self) -> std::result::Result<Option<CacheEntry>, SpecError> {
    match std::fs::read_to_string(&self.cache_path) {
      Ok(contents) => {
        let entry: CacheEntry = serde_json::from_str(&contents)
          .map_err(|e| SpecError::Cache(format!("cache parse: {e}")))?;
        Ok(Some(entry))
      }
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
      Err(err) => Err(SpecError::Cache(format!("cache read: {err}"))),
    }
  }

  /// Write the cache file to disk.
  fn write_cache(&self, spec: &GovernanceSpec) -> std::result::Result<(), SpecError> {
    if let Some(parent) = self.cache_path.parent() {
      std::fs::create_dir_all(parent).map_err(|e| SpecError::Cache(format!("cache mkdir: {e}")))?;
    }
    let entry = CacheEntry {
      spec: spec.clone(),
      fetched_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string_pretty(&entry)
      .map_err(|e| SpecError::Cache(format!("cache serialize: {e}")))?;
    std::fs::write(&self.cache_path, json)
      .map_err(|e| SpecError::Cache(format!("cache write: {e}")))?;
    Ok(())
  }

  /// Returns the cache file path.
  pub fn cache_path(&self) -> &Path {
    &self.cache_path
  }
}

impl Default for SpecLoader {
  fn default() -> Self {
    SpecLoader::new()
  }
}

/// Compute the default cache path:
/// `${XDG_CACHE_HOME:-$HOME/.cache}/apmw/governance/spec.json`
pub fn default_cache_path() -> PathBuf {
  if let Some(xdg_cache) = std::env::var_os("XDG_CACHE_HOME") {
    if !xdg_cache.is_empty() {
      return PathBuf::from(xdg_cache)
        .join("apmw")
        .join("governance")
        .join("spec.json");
    }
  }
  if let Some(cache_dir) = dirs::cache_dir() {
    return cache_dir.join("apmw").join("governance").join("spec.json");
  }
  if let Some(home) = std::env::var_os("HOME") {
    if !home.is_empty() {
      return PathBuf::from(home)
        .join(".cache")
        .join("apmw")
        .join("governance")
        .join("spec.json");
    }
  }
  PathBuf::from(".cache")
    .join("apmw")
    .join("governance")
    .join("spec.json")
}

/// Parse the governance spec from markdown text.
///
/// The spec markdown contains governance directives in fenced code blocks or
/// tables. This parser looks for lines matching the pattern:
///
/// ```text
/// <type> <tool> [message...]
/// ```
///
/// where `<type>` is one of `prefer`, `force`, `block`, `eject`. It also
/// extracts a `version:` header if present.
pub fn parse_spec(text: &str) -> std::result::Result<GovernanceSpec, SpecError> {
  let mut version = String::new();
  let mut rules = Vec::new();

  for line in text.lines() {
    let trimmed = line.trim();

    // Extract version from "version: X.Y" or "# Spec vX.Y"
    if version.is_empty() {
      if let Some(v) = trimmed.strip_prefix("version:") {
        version = v.trim().to_string();
      } else if let Some(v) = trimmed.strip_prefix("# Spec v") {
        version = v.trim().to_string();
      }
    }

    // Parse governance directive lines: "prefer pip", "force npm", etc.
    if let Some(rule) = parse_directive_line(trimmed) {
      rules.push(rule);
    }
  }

  if rules.is_empty() {
    // No directives found — return empty spec (not an error; spec may be
    // in a different format, and config-based rules can still apply).
    debug!("no governance directives found in spec text");
  }

  Ok(GovernanceSpec {
    version,
    rules,
    raw: text.to_string(),
  })
}

/// Parse a single directive line like `prefer pip` or `force npm Use pnpm`.
fn parse_directive_line(line: &str) -> Option<GovernanceRule> {
  let lower = line.to_ascii_lowercase();
  for ty in [
    GovernanceType::Prefer,
    GovernanceType::Force,
    GovernanceType::Block,
    GovernanceType::Eject,
  ] {
    let prefix = format!("{} ", ty.as_str());
    if lower.starts_with(&prefix) {
      let rest = &line[prefix.len()..];
      let mut parts = rest.splitn(2, char::is_whitespace);
      let tool = parts.next()?.trim();
      if tool.is_empty() {
        return None;
      }
      let message = parts
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
      let mut rule = GovernanceRule::new(tool, ty);
      rule.message = message;
      return Some(rule);
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  // ===========================================================================
  // parse_spec
  // ===========================================================================

  #[test]
  fn test_parse_spec_extracts_version() {
    let text = "version: 1.0\nprefer pip\nforce npm";
    let spec = parse_spec(text).unwrap();
    assert_eq!(spec.version, "1.0");
    assert_eq!(spec.rules.len(), 2);
  }

  #[test]
  fn test_parse_spec_extracts_version_from_header() {
    let text = "# Spec v2.1\nprefer pip";
    let spec = parse_spec(text).unwrap();
    assert_eq!(spec.version, "2.1");
  }

  #[test]
  fn test_parse_spec_all_four_types() {
    let text = "version: 1.0\nprefer pip\nforce npm\nblock yarn\neject bun";
    let spec = parse_spec(text).unwrap();
    assert!(spec.has_all_types());
  }

  #[test]
  fn test_parse_spec_with_messages() {
    let text = "force npm Use pnpm instead";
    let spec = parse_spec(text).unwrap();
    assert_eq!(spec.rules.len(), 1);
    let rule = &spec.rules[0];
    assert_eq!(rule.tool, "npm");
    assert_eq!(rule.governance_type, GovernanceType::Force);
    assert_eq!(rule.message.as_deref(), Some("Use pnpm instead"));
  }

  #[test]
  fn test_parse_spec_empty() {
    let spec = parse_spec("").unwrap();
    assert!(spec.rules.is_empty());
    assert!(spec.version.is_empty());
  }

  #[test]
  fn test_parse_spec_no_directives() {
    let spec = parse_spec("just some text\nno rules here").unwrap();
    assert!(spec.rules.is_empty());
  }

  #[test]
  fn test_parse_directive_line_prefer() {
    let rule = parse_directive_line("prefer pip").unwrap();
    assert_eq!(rule.tool, "pip");
    assert_eq!(rule.governance_type, GovernanceType::Prefer);
  }

  #[test]
  fn test_parse_directive_line_case_insensitive() {
    let rule = parse_directive_line("FORCE npm").unwrap();
    assert_eq!(rule.tool, "npm");
    assert_eq!(rule.governance_type, GovernanceType::Force);
  }

  #[test]
  fn test_parse_directive_line_non_directive() {
    assert!(parse_directive_line("hello world").is_none());
  }

  // ===========================================================================
  // GovernanceSpec
  // ===========================================================================

  #[test]
  fn test_spec_rule_for() {
    let spec = GovernanceSpec {
      version: "1.0".to_string(),
      rules: vec![
        GovernanceRule::new("pip", GovernanceType::Force),
        GovernanceRule::new("npm", GovernanceType::Block),
      ],
      raw: String::new(),
    };
    assert_eq!(
      spec.rule_for("pip").unwrap().governance_type,
      GovernanceType::Force
    );
    assert!(spec.rule_for("cargo").is_none());
  }

  #[test]
  fn test_spec_has_all_types_false() {
    let spec = GovernanceSpec {
      version: "1.0".to_string(),
      rules: vec![GovernanceRule::new("pip", GovernanceType::Force)],
      raw: String::new(),
    };
    assert!(!spec.has_all_types());
  }

  // ===========================================================================
  // MockSpecClient
  // ===========================================================================

  #[test]
  fn test_mock_spec_client_ok() {
    let client = MockSpecClient::with_ok("version: 1.0\nprefer pip");
    let text = client.fetch_text(SPEC_URL).unwrap();
    assert!(text.contains("prefer pip"));
  }

  #[test]
  fn test_mock_spec_client_err() {
    let client = MockSpecClient::with_err("network down");
    let err = client.fetch_text(SPEC_URL).unwrap_err();
    assert!(err.to_string().contains("network down"));
  }

  // ===========================================================================
  // SpecLoader — caching with mock client
  // ===========================================================================

  #[test]
  fn test_spec_loader_fetch_and_cache() {
    let tmp = TempDir::new().unwrap();
    let cache_path = tmp.path().join("spec.json");
    let loader = SpecLoader::with_cache_path(cache_path.clone());
    let client =
      MockSpecClient::with_ok("version: 1.0\nprefer pip\nforce npm\nblock yarn\neject bun");

    let spec = loader.load(&client).unwrap();
    assert!(spec.has_all_types());
    assert!(cache_path.exists(), "cache file should be written");
  }

  #[test]
  fn test_spec_loader_cache_hit() {
    let tmp = TempDir::new().unwrap();
    let cache_path = tmp.path().join("spec.json");
    let loader = SpecLoader::with_cache_path(cache_path.clone());

    // First fetch populates the cache.
    let client1 =
      MockSpecClient::with_ok("version: 1.0\nprefer pip\nforce npm\nblock yarn\neject bun");
    let spec1 = loader.load(&client1).unwrap();
    assert!(spec1.has_all_types());

    // Second load should use cache even with a failing client.
    let client2 = MockSpecClient::with_err("should not be called");
    let spec2 = loader.load(&client2).unwrap();
    assert!(spec2.has_all_types());
  }

  #[test]
  fn test_spec_loader_cache_ttl_stale() {
    let tmp = TempDir::new().unwrap();
    let cache_path = tmp.path().join("spec.json");
    // TTL of 0 seconds → always stale.
    let loader =
      SpecLoader::with_cache_path_and_ttl(cache_path.clone(), chrono::Duration::seconds(0));

    let client =
      MockSpecClient::with_ok("version: 1.0\nprefer pip\nforce npm\nblock yarn\neject bun");
    let spec = loader.load(&client).unwrap();
    assert!(spec.has_all_types());
  }

  #[test]
  fn test_spec_loader_refresh_forces_fetch() {
    let tmp = TempDir::new().unwrap();
    let cache_path = tmp.path().join("spec.json");
    let loader = SpecLoader::with_cache_path(cache_path.clone());

    // Populate cache.
    let client1 = MockSpecClient::with_ok("version: 1.0\nprefer pip");
    let _ = loader.load(&client1).unwrap();

    // Refresh with new content.
    let client2 = MockSpecClient::with_ok("version: 2.0\nforce npm");
    let spec = loader.refresh(&client2).unwrap();
    assert_eq!(spec.version, "2.0");
  }

  #[test]
  fn test_spec_loader_fetch_failure_falls_back_to_stale_cache() {
    let tmp = TempDir::new().unwrap();
    let cache_path = tmp.path().join("spec.json");
    // TTL of 0 so cache is always stale → forces fetch attempt.
    let loader =
      SpecLoader::with_cache_path_and_ttl(cache_path.clone(), chrono::Duration::seconds(0));

    // Populate cache with a successful fetch.
    let client_ok = MockSpecClient::with_ok("version: 1.0\nprefer pip");
    let _ = loader.load(&client_ok).unwrap();

    // Now fetch fails — should fall back to stale cache.
    let client_err = MockSpecClient::with_err("network down");
    let spec = loader.load(&client_err).unwrap();
    assert_eq!(spec.version, "1.0");
  }

  #[test]
  fn test_spec_loader_fetch_failure_no_cache_errors() {
    let tmp = TempDir::new().unwrap();
    let cache_path = tmp.path().join("spec.json");
    let loader = SpecLoader::with_cache_path(cache_path);

    let client = MockSpecClient::with_err("network down");
    let result = loader.load(&client);
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("spec fetch failed"));
  }

  // ===========================================================================
  // default_cache_path
  // ===========================================================================

  #[test]
  fn test_default_cache_path_ends_with_spec_json() {
    let path = default_cache_path();
    assert!(path.ends_with("spec.json"));
    assert!(path.to_string_lossy().contains("apmw"));
    assert!(path.to_string_lossy().contains("governance"));
  }
}
