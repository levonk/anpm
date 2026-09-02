//! Human-readable output formatting.
//!
//! Produces pretty-printed, colorized output for interactive use. This is the
//! opt-in alternative to the default agent (TOON/JSON) mode. Colors are emitted
//! via [`anstyle`] so the output is testable without a real terminal.

use std::fmt::Write;

use anstyle::{AnsiColor, Style};
use serde_json::Value;

use super::{Schema, TruncationConfig};

/// Style configuration for human output.
#[derive(Debug, Clone)]
pub struct HumanStyle {
  /// Style for headings.
  pub heading: Style,
  /// Style for keys / labels.
  pub key: Style,
  /// Style for string values.
  pub string: Style,
  /// Style for numeric values.
  pub number: Style,
  /// Style for booleans.
  pub boolean: Style,
  /// Style for null values.
  pub null: Style,
  /// Style for truncation hints.
  pub hint: Style,
  /// Style for error messages.
  pub error: Style,
  /// Style for suggestions.
  pub suggestion: Style,
}

impl Default for HumanStyle {
  fn default() -> Self {
    HumanStyle {
      heading: Style::new().bold().fg_color(Some(AnsiColor::Cyan.into())),
      key: Style::new().bold().fg_color(Some(AnsiColor::Blue.into())),
      string: Style::new().fg_color(Some(AnsiColor::Green.into())),
      number: Style::new().fg_color(Some(AnsiColor::Yellow.into())),
      boolean: Style::new().fg_color(Some(AnsiColor::Magenta.into())),
      null: Style::new().fg_color(Some(AnsiColor::BrightBlack.into())),
      hint: Style::new().fg_color(Some(AnsiColor::BrightBlack.into())),
      error: Style::new().bold().fg_color(Some(AnsiColor::Red.into())),
      suggestion: Style::new().fg_color(Some(AnsiColor::Cyan.into())),
    }
  }
}

/// Wrap `text` with the SGR codes for `style` and a reset at the end.
fn paint(style: Style, text: &str) -> String {
  format!("{}{}{}", style.render(), text, style.render_reset())
}

/// Render a list result as a human-readable table.
///
/// Applies schema filtering and truncation before formatting.
pub fn render_list(
  items: &[Value],
  schema: &Schema,
  truncation: &TruncationConfig,
  style: &HumanStyle,
) -> String {
  let mut out = String::new();
  let total = items.len();

  let _ = writeln!(out, "{}", paint(style.heading, "Items"));

  if items.is_empty() {
    let _ = writeln!(
      out,
      "  {}",
      paint(
        style.hint,
        "No items found. Use --help for available commands."
      )
    );
    return out;
  }

  let fields = schema.resolved_fields(items);
  for item in items {
    let _ = writeln!(out, "  {}", paint(style.heading, "─"));
    for field in &fields {
      let val = item.get(field).unwrap_or(&Value::Null);
      let rendered = render_value(val, truncation, style);
      let _ = writeln!(
        out,
        "  {} {}",
        paint(style.key, &format!("{field}:")),
        rendered
      );
    }
  }

  let _ = writeln!(out, "  {}", paint(style.hint, &format!("Total: {total}")));
  out
}

/// Render a single value with truncation and colorization.
fn render_value(value: &Value, truncation: &TruncationConfig, style: &HumanStyle) -> String {
  match value {
    Value::Null => paint(style.null, "null"),
    Value::Bool(b) => paint(style.boolean, &b.to_string()),
    Value::Number(n) => paint(style.number, &n.to_string()),
    Value::String(s) => {
      let (display, truncated) = truncation.truncate_string(s);
      let base = paint(style.string, &format!("\"{display}\""));
      if truncated {
        format!("{} {}", base, paint(style.hint, &truncation.hint(s.len())))
      } else {
        base
      }
    }
    Value::Array(arr) => {
      let inner: Vec<String> = arr
        .iter()
        .map(|v| render_value(v, truncation, style))
        .collect();
      format!("[{}]", inner.join(", "))
    }
    Value::Object(obj) => {
      let inner: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
          format!(
            "{}: {}",
            paint(style.key, k),
            render_value(v, truncation, style)
          )
        })
        .collect();
      format!("{{{}}}", inner.join(", "))
    }
  }
}

/// Render a structured error for human consumption.
pub fn render_error(error: &str, suggestion: &str, style: &HumanStyle) -> String {
  let mut out = String::new();
  let _ = writeln!(out, "{} {}", paint(style.error, "Error:"), error);
  if !suggestion.is_empty() {
    let _ = writeln!(
      out,
      "{} {}",
      paint(style.suggestion, "Suggestion:"),
      suggestion
    );
  }
  out
}

/// Render the contextual disclosure (help[]) for human consumption.
pub fn render_help(steps: &[String], style: &HumanStyle) -> String {
  let mut out = String::new();
  if steps.is_empty() {
    return out;
  }
  let _ = writeln!(out, "{}", paint(style.heading, "Next steps:"));
  for step in steps {
    let _ = writeln!(out, "  {} {}", paint(style.hint, "•"), step);
  }
  out
}

/// Render an empty state for human consumption.
pub fn render_empty(context: &str, style: &HumanStyle) -> String {
  let mut out = String::new();
  let _ = writeln!(
    out,
    "{} {}",
    paint(style.hint, "No results found."),
    context
  );
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn plain_style() -> HumanStyle {
    HumanStyle {
      heading: Style::new(),
      key: Style::new(),
      string: Style::new(),
      number: Style::new(),
      boolean: Style::new(),
      null: Style::new(),
      hint: Style::new(),
      error: Style::new(),
      suggestion: Style::new(),
    }
  }

  #[test]
  fn test_render_list_non_empty() {
    let items = vec![
      json!({"name": "foo", "version": "1.0", "desc": "a tool"}),
      json!({"name": "bar", "version": "2.0", "desc": "another"}),
    ];
    let schema = Schema::new(vec!["name".into(), "version".into()]);
    let trunc = TruncationConfig::default();
    let style = plain_style();
    let out = render_list(&items, &schema, &trunc, &style);
    assert!(out.contains("Items"));
    assert!(out.contains("foo"));
    assert!(out.contains("bar"));
    assert!(out.contains("Total: 2"));
  }

  #[test]
  fn test_render_list_empty() {
    let items: Vec<Value> = vec![];
    let schema = Schema::default();
    let trunc = TruncationConfig::default();
    let style = plain_style();
    let out = render_list(&items, &schema, &trunc, &style);
    assert!(out.contains("No items found"));
  }

  #[test]
  fn test_render_error() {
    let style = plain_style();
    let out = render_error("package not found", "try 'apmw detect'", &style);
    assert!(out.contains("Error:"));
    assert!(out.contains("package not found"));
    assert!(out.contains("Suggestion:"));
    assert!(out.contains("try 'apmw detect'"));
  }

  #[test]
  fn test_render_help() {
    let style = plain_style();
    let steps = vec!["apmw detect".to_string(), "apmw install foo".to_string()];
    let out = render_help(&steps, &style);
    assert!(out.contains("Next steps:"));
    assert!(out.contains("apmw detect"));
    assert!(out.contains("apmw install foo"));
  }

  #[test]
  fn test_render_empty_state() {
    let style = plain_style();
    let out = render_empty("no packages detected in this project", &style);
    assert!(out.contains("No results found."));
    assert!(out.contains("no packages detected"));
  }

  #[test]
  fn test_render_value_truncation() {
    let long = "x".repeat(600);
    let val = json!(long);
    let trunc = TruncationConfig::default();
    let style = plain_style();
    let out = render_value(&val, &trunc, &style);
    assert!(out.contains("…"));
    assert!(out.contains("600 chars total"));
  }

  #[test]
  fn test_render_value_no_truncation() {
    let val = json!("short");
    let trunc = TruncationConfig::default();
    let style = plain_style();
    let out = render_value(&val, &trunc, &style);
    assert!(out.contains("short"));
    assert!(!out.contains("…"));
  }

  #[test]
  fn test_human_style_default() {
    let style = HumanStyle::default();
    // Ensure default styles have colors set.
    assert!(style.heading.get_fg_color().is_some());
    assert!(style.error.get_fg_color().is_some());
  }
}
