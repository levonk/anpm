//! AXI output module — the output boundary for apmw.
//!
//! Internal logic stays on [`serde_json::Value`]; conversion to the final
//! output format (TOON for agent mode, pretty-printed text for human mode)
//! happens here. Agent mode is the **default**; human mode is opt-in via
//! `--human` or when `--json` is off.
//!
//! ## AXI contract (ADR-20260607001 §36-45)
//!
//! - **Minimal schemas**: list output shows 3-4 fields by default; `--fields`
//!   selects additional fields.
//! - **Content truncation**: field values longer than a threshold (default 500
//!   chars preview, max 1500) are truncated with a hint suffix. `--full`
//!   disables truncation.
//! - **Pre-computed aggregates**: list output includes a `total` count.
//! - **Definitive empty states**: empty results emit
//!   `{"total":0,"items":[],"help":[...]}` — never silence.
//! - **Structured errors on stdout**: `{"error":"...","suggestion":"..."}` — no
//!   raw stack traces.
//! - **Contextual disclosure**: a `help[]` array with 2-4 complete next-step
//!   command strings.

pub mod human;
pub mod toon;

use serde_json::{json, Map, Value};

use tracing::debug;

/// Default preview length for truncated field values (ADR §38).
pub const DEFAULT_PREVIEW_LEN: usize = 500;

/// Maximum length for truncated field values (ADR §38).
pub const DEFAULT_MAX_LEN: usize = 1500;

/// Minimum number of help steps in contextual disclosure (ADR §45).
pub const MIN_HELP_STEPS: usize = 2;

/// Maximum number of help steps in contextual disclosure (ADR §45).
pub const MAX_HELP_STEPS: usize = 4;

/// Output mode: agent (TOON/JSON) is the default, human is opt-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
  /// Agent mode — TOON or JSON output, minimal schemas, truncation (default).
  #[default]
  Agent,
  /// Human mode — pretty-printed, colorized output (opt-in via `--human`).
  Human,
}

/// Serialization format within agent mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentFormat {
  /// TOON (Token-Oriented Object Notation) — the default agent format.
  #[default]
  Toon,
  /// JSON — opt-in via `--json`.
  Json,
}

/// Schema configuration controlling which fields are emitted in list output.
///
/// Defaults to 3-4 fields (ADR §37). Additional fields can be requested via
/// `--fields`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
  /// Default fields always included (3-4 per ADR §37).
  pub default_fields: Vec<String>,
  /// Additional fields requested via `--fields`.
  pub extra_fields: Vec<String>,
}

impl Schema {
  /// Create a new schema with the given default fields and no extras.
  pub fn new(default_fields: Vec<String>) -> Self {
    Schema {
      default_fields,
      extra_fields: Vec::new(),
    }
  }

  /// Create a schema with extra fields parsed from a comma-separated string.
  pub fn with_fields_str(default_fields: Vec<String>, fields: Option<&str>) -> Self {
    let extra_fields = fields
      .map(|s| {
        s.split(',')
          .map(|f| f.trim().to_string())
          .filter(|f| !f.is_empty())
          .collect()
      })
      .unwrap_or_default();
    Schema {
      default_fields,
      extra_fields,
    }
  }

  /// Set extra fields directly.
  pub fn with_extra_fields(mut self, fields: Vec<String>) -> Self {
    self.extra_fields = fields;
    self
  }

  /// The full ordered list of fields to emit (defaults + extras, deduped).
  pub fn resolved_fields(&self, items: &[Value]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for f in &self.default_fields {
      if seen.insert(f.clone()) {
        result.push(f.clone());
      }
    }
    for f in &self.extra_fields {
      if seen.insert(f.clone()) {
        result.push(f.clone());
      }
    }
    // If no default fields matched the data, fall back to the data's keys.
    if result.is_empty() {
      if let Some(first) = items.first() {
        if let Some(obj) = first.as_object() {
          for k in obj.keys() {
            if seen.insert(k.clone()) {
              result.push(k.clone());
              if result.len() >= 4 {
                break;
              }
            }
          }
        }
      }
    }
    result
  }

  /// Filter an item to only the resolved fields.
  pub fn filter_item(&self, item: &Value, items: &[Value]) -> Value {
    let fields = self.resolved_fields(items);
    if let Some(obj) = item.as_object() {
      let mut filtered = Map::new();
      for f in &fields {
        if let Some(v) = obj.get(f) {
          filtered.insert(f.clone(), v.clone());
        }
      }
      Value::Object(filtered)
    } else {
      item.clone()
    }
  }
}

impl Default for Schema {
  fn default() -> Self {
    Schema::new(vec![
      "name".into(),
      "version".into(),
      "description".into(),
      "status".into(),
    ])
  }
}

/// Truncation configuration (ADR §38).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruncationConfig {
  /// Preview length before truncation (default 500).
  pub preview_len: usize,
  /// Maximum length when not using `--full` (default 1500).
  pub max_len: usize,
  /// Whether truncation is disabled (`--full`).
  pub full: bool,
}

impl Default for TruncationConfig {
  fn default() -> Self {
    TruncationConfig {
      preview_len: DEFAULT_PREVIEW_LEN,
      max_len: DEFAULT_MAX_LEN,
      full: false,
    }
  }
}

impl TruncationConfig {
  /// Create a truncation config with `--full` enabled (no truncation).
  pub fn full() -> Self {
    TruncationConfig {
      preview_len: DEFAULT_PREVIEW_LEN,
      max_len: DEFAULT_MAX_LEN,
      full: true,
    }
  }

  /// Truncate a string value, returning (display_value, was_truncated).
  pub fn truncate_string(&self, s: &str) -> (String, bool) {
    if self.full || s.len() <= self.preview_len {
      return (s.to_string(), false);
    }
    let preview: String = s.chars().take(self.preview_len).collect();
    (preview, true)
  }

  /// Build the truncation hint suffix.
  pub fn hint(&self, total_len: usize) -> String {
    format!("… ({total_len} chars total, --full to show)")
  }

  /// Recursively truncate long string values within a [`serde_json::Value`].
  pub fn truncate_value(&self, value: &Value) -> Value {
    match value {
      Value::String(s) => {
        let (display, truncated) = self.truncate_string(s);
        if truncated {
          Value::String(format!("{}{}", display, self.hint(s.len())))
        } else {
          Value::String(display)
        }
      }
      Value::Array(arr) => Value::Array(arr.iter().map(|v| self.truncate_value(v)).collect()),
      Value::Object(obj) => {
        let mut map = Map::new();
        for (k, v) in obj {
          map.insert(k.clone(), self.truncate_value(v));
        }
        Value::Object(map)
      }
      other => other.clone(),
    }
  }
}

/// The output dispatcher — selects between agent and human mode.
#[derive(Debug, Clone)]
pub struct OutputDispatcher {
  /// Output mode (agent default, human opt-in).
  pub mode: OutputMode,
  /// Agent serialization format (TOON default, JSON opt-in).
  pub agent_format: AgentFormat,
  /// Schema for field selection.
  pub schema: Schema,
  /// Truncation configuration.
  pub truncation: TruncationConfig,
}

impl Default for OutputDispatcher {
  fn default() -> Self {
    OutputDispatcher {
      mode: OutputMode::Agent,
      agent_format: AgentFormat::Toon,
      schema: Schema::default(),
      truncation: TruncationConfig::default(),
    }
  }
}

impl OutputDispatcher {
  /// Create a dispatcher from CLI flags.
  pub fn from_flags(human: bool, json: bool, fields: Option<&str>, full: bool) -> Self {
    let mode = if human {
      OutputMode::Human
    } else {
      OutputMode::Agent
    };
    let agent_format = if json {
      AgentFormat::Json
    } else {
      AgentFormat::Toon
    };
    let schema = Schema::with_fields_str(
      vec![
        "name".into(),
        "version".into(),
        "description".into(),
        "status".into(),
      ],
      fields,
    );
    let truncation = if full {
      TruncationConfig::full()
    } else {
      TruncationConfig::default()
    };
    debug!(mode = ?mode, agent_format = ?agent_format, "OutputDispatcher created");
    OutputDispatcher {
      mode,
      agent_format,
      schema,
      truncation,
    }
  }

  /// Render a list of items.
  pub fn render_list(&self, items: &[Value], help: &[String]) -> String {
    let total = items.len();
    let filtered: Vec<Value> = items
      .iter()
      .map(|item| self.schema.filter_item(item, items))
      .map(|item| self.truncation.truncate_value(&item))
      .collect();

    let help_steps = normalize_help(help);

    if items.is_empty() {
      return self.render_empty(&help_steps);
    }

    let payload = json!({
      "total": total,
      "items": filtered,
      "help": help_steps,
    });

    self.emit(&payload)
  }

  /// Render a single item (detail view).
  pub fn render_item(&self, item: &Value, help: &[String]) -> String {
    let truncated = self.truncation.truncate_value(item);
    let help_steps = normalize_help(help);
    let payload = json!({
      "item": truncated,
      "help": help_steps,
    });
    self.emit(&payload)
  }

  /// Render a definitive empty state (ADR §40).
  pub fn render_empty(&self, help: &[String]) -> String {
    let help_steps = normalize_help(help);
    let payload = json!({
      "total": 0,
      "items": [],
      "help": help_steps,
    });
    self.emit(&payload)
  }

  /// Render a structured error (ADR §41).
  pub fn render_error(&self, error: &str, suggestion: &str, help: &[String]) -> String {
    let help_steps = normalize_help(help);
    match self.mode {
      OutputMode::Agent => {
        let payload = json!({
          "error": error,
          "suggestion": suggestion,
          "help": help_steps,
        });
        self.emit(&payload)
      }
      OutputMode::Human => {
        let style = human::HumanStyle::default();
        let mut out = human::render_error(error, suggestion, &style);
        if !help_steps.is_empty() {
          out.push_str(&human::render_help(&help_steps, &style));
        }
        out
      }
    }
  }

  /// Emit a payload in the configured format.
  fn emit(&self, value: &Value) -> String {
    match self.mode {
      OutputMode::Agent => match self.agent_format {
        AgentFormat::Toon => toon::encode(value),
        AgentFormat::Json => serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
      },
      OutputMode::Human => {
        let style = human::HumanStyle::default();
        human_render_payload(value, &self.schema, &self.truncation, &style)
      }
    }
  }
}

/// Render a payload for human mode.
fn human_render_payload(
  value: &Value,
  schema: &Schema,
  truncation: &TruncationConfig,
  style: &human::HumanStyle,
) -> String {
  if let Some(obj) = value.as_object() {
    // If it has "items", render as a list.
    if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
      return human::render_list(items, schema, truncation, style);
    }
    // If it has "error", render as error.
    if let Some(err) = obj.get("error").and_then(|v| v.as_str()) {
      let suggestion = obj.get("suggestion").and_then(|v| v.as_str()).unwrap_or("");
      return human::render_error(err, suggestion, style);
    }
  }
  // Fallback: pretty-print the JSON.
  serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

/// Normalize the help array to satisfy the 2-4 step requirement (ADR §45).
///
/// If fewer than [`MIN_HELP_STEPS`] are provided, default steps are appended.
/// If more than [`MAX_HELP_STEPS`] are provided, the list is truncated.
fn normalize_help(help: &[String]) -> Vec<String> {
  let mut steps: Vec<String> = help.iter().filter(|s| !s.is_empty()).cloned().collect();
  let defaults = [
    "apmw --help".to_string(),
    "apmw detect".to_string(),
    "apmw status".to_string(),
    "apmw install <package>".to_string(),
  ];
  let mut di = 0;
  while steps.len() < MIN_HELP_STEPS && di < defaults.len() {
    if !steps.contains(&defaults[di]) {
      steps.push(defaults[di].clone());
    }
    di += 1;
  }
  if steps.len() > MAX_HELP_STEPS {
    steps.truncate(MAX_HELP_STEPS);
  }
  steps
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  // --- Schema filtering tests ---

  #[test]
  fn test_schema_default_fields() {
    let schema = Schema::default();
    assert_eq!(schema.default_fields.len(), 4);
    assert!(schema.default_fields.contains(&"name".to_string()));
  }

  #[test]
  fn test_schema_filter_item() {
    let item = json!({
      "name": "foo",
      "version": "1.0",
      "description": "a tool",
      "status": "active",
      "extra": "hidden"
    });
    let items = vec![item.clone()];
    let schema = Schema::default();
    let filtered = schema.filter_item(&item, &items);
    let obj = filtered.as_object().unwrap();
    assert!(obj.contains_key("name"));
    assert!(obj.contains_key("version"));
    assert!(!obj.contains_key("extra"));
  }

  #[test]
  fn test_schema_with_extra_fields() {
    let schema = Schema::new(vec!["name".into()]).with_extra_fields(vec!["extra".into()]);
    let item = json!({"name": "foo", "extra": "data", "hidden": "no"});
    let items = vec![item.clone()];
    let filtered = schema.filter_item(&item, &items);
    let obj = filtered.as_object().unwrap();
    assert!(obj.contains_key("name"));
    assert!(obj.contains_key("extra"));
    assert!(!obj.contains_key("hidden"));
  }

  #[test]
  fn test_schema_fields_from_str() {
    let schema =
      Schema::with_fields_str(vec!["name".into()], Some("version, description , status"));
    assert_eq!(schema.default_fields, vec!["name"]);
    assert_eq!(
      schema.extra_fields,
      vec!["version", "description", "status"]
    );
  }

  #[test]
  fn test_schema_fields_from_str_empty() {
    let schema = Schema::with_fields_str(vec!["name".into()], Some(""));
    assert!(schema.extra_fields.is_empty());
  }

  #[test]
  fn test_schema_fields_from_str_none() {
    let schema = Schema::with_fields_str(vec!["name".into()], None);
    assert!(schema.extra_fields.is_empty());
  }

  #[test]
  fn test_schema_fallback_to_data_keys() {
    let schema = Schema::new(vec![]);
    let item = json!({"a": 1, "b": 2, "c": 3, "d": 4, "e": 5});
    let items = vec![item.clone()];
    let fields = schema.resolved_fields(&items);
    assert!(!fields.is_empty());
    assert!(fields.len() <= 4);
  }

  // --- Truncation tests ---

  #[test]
  fn test_truncation_short_string() {
    let config = TruncationConfig::default();
    let (display, truncated) = config.truncate_string("short");
    assert_eq!(display, "short");
    assert!(!truncated);
  }

  #[test]
  fn test_truncation_long_string() {
    let config = TruncationConfig::default();
    let long = "x".repeat(600);
    let (display, truncated) = config.truncate_string(&long);
    assert!(truncated);
    assert_eq!(display.len(), 500);
  }

  #[test]
  fn test_truncation_full_disables() {
    let config = TruncationConfig::full();
    let long = "x".repeat(600);
    let (display, truncated) = config.truncate_string(&long);
    assert!(!truncated);
    assert_eq!(display, long);
  }

  #[test]
  fn test_truncation_hint() {
    let config = TruncationConfig::default();
    let hint = config.hint(1234);
    assert!(hint.contains("1234 chars total"));
    assert!(hint.contains("--full to show"));
  }

  #[test]
  fn test_truncation_value_object() {
    let config = TruncationConfig::default();
    let long = "x".repeat(600);
    let val = json!({"desc": long, "name": "foo"});
    let truncated = config.truncate_value(&val);
    let desc = truncated.get("desc").unwrap().as_str().unwrap();
    assert!(desc.contains("…"));
    assert!(desc.contains("600 chars total"));
  }

  #[test]
  fn test_truncation_value_array() {
    let config = TruncationConfig::default();
    let long = "x".repeat(600);
    let val = json!([long, "short"]);
    let truncated = config.truncate_value(&val);
    let arr = truncated.as_array().unwrap();
    let first = arr[0].as_str().unwrap();
    assert!(first.contains("…"));
  }

  #[test]
  fn test_truncation_value_full_no_change() {
    let config = TruncationConfig::full();
    let long = "x".repeat(600);
    let val = json!({"desc": long});
    let truncated = config.truncate_value(&val);
    assert_eq!(truncated, val);
  }

  // --- Aggregates tests ---

  #[test]
  fn test_aggregates_total_in_list() {
    let dispatcher = OutputDispatcher::default();
    let items = vec![
      json!({"name": "foo", "version": "1.0", "description": "a", "status": "ok"}),
      json!({"name": "bar", "version": "2.0", "description": "b", "status": "ok"}),
    ];
    let out = dispatcher.render_list(&items, &[]);
    assert!(out.contains("\"total\"=2"));
    assert!(out.contains("\"items\""));
  }

  #[test]
  fn test_aggregates_total_zero_empty() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_list(&[], &[]);
    assert!(out.contains("\"total\"=0"));
    assert!(out.contains("\"items\"=[]"));
  }

  // --- Empty states tests ---

  #[test]
  fn test_empty_state_agent_mode() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_empty(&[]);
    assert!(out.contains("\"total\"=0"));
    assert!(out.contains("\"items\"=[]"));
    assert!(out.contains("\"help\""));
  }

  #[test]
  fn test_empty_state_not_silent() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_list(&[], &[]);
    assert!(!out.is_empty());
  }

  #[test]
  fn test_empty_state_with_help() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_empty(&["apmw detect".to_string()]);
    assert!(out.contains("apmw detect"));
  }

  // --- Structured errors tests ---

  #[test]
  fn test_structured_error_agent_mode() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_error("package not found", "run 'apmw detect'", &[]);
    assert!(out.contains("\"error\""));
    assert!(out.contains("package not found"));
    assert!(out.contains("\"suggestion\""));
    assert!(out.contains("run 'apmw detect'"));
  }

  #[test]
  fn test_structured_error_no_stack_trace() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_error("something failed", "try again", &[]);
    assert!(!out.contains("stack"));
    assert!(!out.contains("panic"));
    assert!(!out.contains("backtrace"));
  }

  #[test]
  fn test_structured_error_includes_help() {
    let dispatcher = OutputDispatcher::default();
    let out = dispatcher.render_error("err", "fix", &["apmw --help".to_string()]);
    assert!(out.contains("\"help\""));
    assert!(out.contains("apmw --help"));
  }

  // --- Contextual disclosure tests ---

  #[test]
  fn test_contextual_disclosure_min_steps() {
    let steps = normalize_help(&[]);
    assert!(steps.len() >= MIN_HELP_STEPS);
  }

  #[test]
  fn test_contextual_disclosure_max_steps() {
    let steps = normalize_help(&[
      "a".to_string(),
      "b".to_string(),
      "c".to_string(),
      "d".to_string(),
      "e".to_string(),
    ]);
    assert!(steps.len() <= MAX_HELP_STEPS);
  }

  #[test]
  fn test_contextual_disclosure_in_list() {
    let dispatcher = OutputDispatcher::default();
    let items = vec![json!({"name": "foo", "version": "1", "description": "d", "status": "s"})];
    let out = dispatcher.render_list(&items, &["apmw install foo".to_string()]);
    assert!(out.contains("\"help\""));
    assert!(out.contains("apmw install foo"));
  }

  #[test]
  fn test_contextual_disclosure_complete_commands() {
    let steps = normalize_help(&["apmw install foo".to_string(), "apmw scan foo".to_string()]);
    // Each step should be a non-empty command string.
    for step in &steps {
      assert!(!step.is_empty());
      assert!(!step.starts_with(' '));
    }
  }

  // --- Output mode tests ---

  #[test]
  fn test_default_mode_is_agent() {
    let dispatcher = OutputDispatcher::default();
    assert_eq!(dispatcher.mode, OutputMode::Agent);
  }

  #[test]
  fn test_from_flags_human() {
    let dispatcher = OutputDispatcher::from_flags(true, false, None, false);
    assert_eq!(dispatcher.mode, OutputMode::Human);
  }

  #[test]
  fn test_from_flags_json() {
    let dispatcher = OutputDispatcher::from_flags(false, true, None, false);
    assert_eq!(dispatcher.agent_format, AgentFormat::Json);
  }

  #[test]
  fn test_from_flags_toon_default() {
    let dispatcher = OutputDispatcher::from_flags(false, false, None, false);
    assert_eq!(dispatcher.agent_format, AgentFormat::Toon);
  }

  #[test]
  fn test_from_flags_full() {
    let dispatcher = OutputDispatcher::from_flags(false, false, None, true);
    assert!(dispatcher.truncation.full);
  }

  #[test]
  fn test_from_flags_fields() {
    let dispatcher = OutputDispatcher::from_flags(false, false, Some("a,b,c"), false);
    assert_eq!(dispatcher.schema.extra_fields, vec!["a", "b", "c"]);
  }

  // --- Agent format emission tests ---

  #[test]
  fn test_emit_toon() {
    let dispatcher = OutputDispatcher::default();
    let items = vec![json!({"name": "foo", "version": "1", "description": "d", "status": "s"})];
    let out = dispatcher.render_list(&items, &[]);
    // TOON format: no colons, no commas.
    assert!(!out.contains(':'));
    assert!(!out.contains(','));
    assert!(out.contains('='));
  }

  #[test]
  fn test_emit_json() {
    let dispatcher = OutputDispatcher {
      agent_format: AgentFormat::Json,
      ..OutputDispatcher::default()
    };
    let items = vec![json!({"name": "foo", "version": "1", "description": "d", "status": "s"})];
    let out = dispatcher.render_list(&items, &[]);
    // JSON format: has colons and commas.
    assert!(out.contains(':'));
    assert!(out.contains(','));
  }

  // --- Human output tests ---

  #[test]
  fn test_human_output_list() {
    let dispatcher = OutputDispatcher {
      mode: OutputMode::Human,
      ..OutputDispatcher::default()
    };
    let items = vec![json!({"name": "foo", "version": "1", "description": "d", "status": "s"})];
    let out = dispatcher.render_list(&items, &[]);
    assert!(out.contains("foo"));
  }

  #[test]
  fn test_human_output_error() {
    let dispatcher = OutputDispatcher {
      mode: OutputMode::Human,
      ..OutputDispatcher::default()
    };
    let out = dispatcher.render_error("not found", "try detect", &[]);
    assert!(out.contains("not found"));
    assert!(out.contains("try detect"));
  }

  #[test]
  fn test_human_output_empty() {
    let dispatcher = OutputDispatcher {
      mode: OutputMode::Human,
      ..OutputDispatcher::default()
    };
    let out = dispatcher.render_list(&[], &[]);
    assert!(!out.is_empty());
  }

  // --- Integration: render_item ---

  #[test]
  fn test_render_item_agent() {
    let dispatcher = OutputDispatcher::default();
    let item = json!({"name": "foo", "version": "1.0", "description": "a tool", "status": "ok"});
    let out = dispatcher.render_item(&item, &[]);
    assert!(out.contains("\"item\""));
    assert!(out.contains("foo"));
  }
}
