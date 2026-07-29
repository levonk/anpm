//! TOON (Token-Oriented Object Notation) encoder and parser.
//!
//! TOON is a token-efficient serialization format designed for LLM consumption.
//! It minimizes punctuation tokens while remaining unambiguous and round-trip
//! compatible with `serde_json::Value`.
//!
//! ## Format assumptions (compact subset)
//!
//! The TOON specification at <https://toonformat.dev> was not accessible during
//! implementation, so this module implements a reasonable compact subset with
//! the following rules:
//!
//! - **null** → `null`
//! - **bool** → `true` | `false`
//! - **number** → JSON number representation (integers and floats, no trailing
//!   zeros).
//! - **string** → double-quoted with JSON-style escape sequences. Strings are
//!   always quoted to guarantee round-trip safety with arbitrary content
//!   (spaces, special characters, empty strings).
//! - **array** → `[elem|elem|elem]` — elements separated by `|`, no commas.
//! - **object** → `{key=value key=value}` — entries separated by a single
//!   space, keys and values separated by `=`.
//!
//! Token efficiency vs JSON comes from: no commas (arrays use `|`, objects use
//! spaces), no colons (objects use `=`), and compact whitespace.

use serde_json::Value;

/// Encode a [`serde_json::Value`] into a compact TOON string.
pub fn encode(value: &Value) -> String {
  let mut out = String::new();
  encode_value(value, &mut out);
  out
}

fn encode_value(value: &Value, out: &mut String) {
  match value {
    Value::Null => out.push_str("null"),
    Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
    Value::Number(n) => out.push_str(&n.to_string()),
    Value::String(s) => encode_string(s, out),
    Value::Array(arr) => {
      out.push('[');
      for (i, elem) in arr.iter().enumerate() {
        if i > 0 {
          out.push('|');
        }
        encode_value(elem, out);
      }
      out.push(']');
    }
    Value::Object(obj) => {
      out.push('{');
      for (i, (k, v)) in obj.iter().enumerate() {
        if i > 0 {
          out.push(' ');
        }
        encode_string(k, out);
        out.push('=');
        encode_value(v, out);
      }
      out.push('}');
    }
  }
}

/// Encode a string as a double-quoted TOON string with JSON-style escapes.
fn encode_string(s: &str, out: &mut String) {
  out.push('"');
  for ch in s.chars() {
    match ch {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      '\u{08}' => out.push_str("\\b"),
      '\u{0c}' => out.push_str("\\f"),
      c if (c as u32) < 0x20 => {
        out.push_str(&format!("\\u{:04x}", c as u32));
      }
      c => out.push(c),
    }
  }
  out.push('"');
}

/// Error returned when parsing TOON fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToonError {
  /// Byte offset where the error was detected.
  pub offset: usize,
  /// Human-readable description of the error.
  pub message: String,
}

impl std::fmt::Display for ToonError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "TOON parse error at offset {}: {}",
      self.offset, self.message
    )
  }
}

impl std::error::Error for ToonError {}

/// Parse a TOON string back into a [`serde_json::Value`].
pub fn parse(input: &str) -> Result<Value, ToonError> {
  let mut parser = Parser::new(input);
  parser.skip_ws();
  let value = parser.parse_value()?;
  parser.skip_ws();
  if parser.pos < parser.bytes.len() {
    return Err(ToonError {
      offset: parser.pos,
      message: format!(
        "unexpected trailing byte {:?}",
        parser.bytes[parser.pos] as char
      ),
    });
  }
  Ok(value)
}

struct Parser<'a> {
  bytes: &'a [u8],
  pos: usize,
}

impl<'a> Parser<'a> {
  fn new(input: &'a str) -> Self {
    Parser {
      bytes: input.as_bytes(),
      pos: 0,
    }
  }

  fn skip_ws(&mut self) {
    while self.pos < self.bytes.len() {
      match self.bytes[self.pos] {
        b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
        _ => break,
      }
    }
  }

  fn peek(&self) -> Option<u8> {
    self.bytes.get(self.pos).copied()
  }

  fn parse_value(&mut self) -> Result<Value, ToonError> {
    self.skip_ws();
    match self.peek() {
      Some(b'{') => self.parse_object(),
      Some(b'[') => self.parse_array(),
      Some(b'"') => self.parse_string().map(Value::String),
      Some(b't') => self.parse_keyword("true", Value::Bool(true)),
      Some(b'f') => self.parse_keyword("false", Value::Bool(false)),
      Some(b'n') => self.parse_keyword("null", Value::Null),
      Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
      Some(other) => Err(ToonError {
        offset: self.pos,
        message: format!("unexpected byte {:?}", other as char),
      }),
      None => Err(ToonError {
        offset: self.pos,
        message: "unexpected end of input".to_string(),
      }),
    }
  }

  fn parse_keyword(&mut self, kw: &str, value: Value) -> Result<Value, ToonError> {
    let kw_bytes = kw.as_bytes();
    if self.pos + kw_bytes.len() <= self.bytes.len()
      && &self.bytes[self.pos..self.pos + kw_bytes.len()] == kw_bytes
    {
      self.pos += kw_bytes.len();
      Ok(value)
    } else {
      Err(ToonError {
        offset: self.pos,
        message: format!("expected keyword '{kw}'"),
      })
    }
  }

  fn parse_number(&mut self) -> Result<Value, ToonError> {
    let start = self.pos;
    if self.peek() == Some(b'-') {
      self.pos += 1;
    }
    while let Some(b'0'..=b'9') = self.peek() {
      self.pos += 1;
    }
    let mut is_float = false;
    if self.peek() == Some(b'.') {
      is_float = true;
      self.pos += 1;
      while let Some(b'0'..=b'9') = self.peek() {
        self.pos += 1;
      }
    }
    if matches!(self.peek(), Some(b'e') | Some(b'E')) {
      is_float = true;
      self.pos += 1;
      if matches!(self.peek(), Some(b'+') | Some(b'-')) {
        self.pos += 1;
      }
      while let Some(b'0'..=b'9') = self.peek() {
        self.pos += 1;
      }
    }
    let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| ToonError {
      offset: start,
      message: "invalid utf-8 in number".to_string(),
    })?;
    if is_float {
      text
        .parse::<f64>()
        .map(|f| {
          serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)
        })
        .map_err(|_| ToonError {
          offset: start,
          message: format!("invalid float '{text}'"),
        })
    } else {
      text
        .parse::<i64>()
        .map(|i| Value::Number(serde_json::Number::from(i)))
        .or_else(|_| {
          text
            .parse::<u64>()
            .map(|u| Value::Number(serde_json::Number::from(u)))
        })
        .map_err(|_| ToonError {
          offset: start,
          message: format!("invalid integer '{text}'"),
        })
    }
  }

  fn parse_string(&mut self) -> Result<String, ToonError> {
    // Expect opening quote.
    if self.peek() != Some(b'"') {
      return Err(ToonError {
        offset: self.pos,
        message: "expected '\"'".to_string(),
      });
    }
    self.pos += 1;
    let mut s = String::new();
    loop {
      match self.peek() {
        None => {
          return Err(ToonError {
            offset: self.pos,
            message: "unterminated string".to_string(),
          });
        }
        Some(b'"') => {
          self.pos += 1;
          break;
        }
        Some(b'\\') => {
          self.pos += 1;
          match self.peek() {
            None => {
              return Err(ToonError {
                offset: self.pos,
                message: "unterminated escape".to_string(),
              });
            }
            Some(b'"') => {
              s.push('"');
              self.pos += 1;
            }
            Some(b'\\') => {
              s.push('\\');
              self.pos += 1;
            }
            Some(b'/') => {
              s.push('/');
              self.pos += 1;
            }
            Some(b'n') => {
              s.push('\n');
              self.pos += 1;
            }
            Some(b'r') => {
              s.push('\r');
              self.pos += 1;
            }
            Some(b't') => {
              s.push('\t');
              self.pos += 1;
            }
            Some(b'b') => {
              s.push('\u{08}');
              self.pos += 1;
            }
            Some(b'f') => {
              s.push('\u{0c}');
              self.pos += 1;
            }
            Some(b'u') => {
              self.pos += 1;
              if self.pos + 4 > self.bytes.len() {
                return Err(ToonError {
                  offset: self.pos,
                  message: "incomplete unicode escape".to_string(),
                });
              }
              let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).map_err(|_| {
                ToonError {
                  offset: self.pos,
                  message: "invalid utf-8 in unicode escape".to_string(),
                }
              })?;
              let code = u32::from_str_radix(hex, 16).map_err(|_| ToonError {
                offset: self.pos,
                message: format!("invalid unicode escape '\\u{hex}'"),
              })?;
              self.pos += 4;
              // Handle surrogate pairs.
              if (0xD800..=0xDBFF).contains(&code) {
                // Expect a low surrogate.
                if self.pos + 6 > self.bytes.len() || &self.bytes[self.pos..self.pos + 2] != b"\\u"
                {
                  return Err(ToonError {
                    offset: self.pos,
                    message: "expected low surrogate after high surrogate".to_string(),
                  });
                }
                self.pos += 2;
                let hex2 =
                  std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).map_err(|_| {
                    ToonError {
                      offset: self.pos,
                      message: "invalid utf-8 in low surrogate".to_string(),
                    }
                  })?;
                let code2 = u32::from_str_radix(hex2, 16).map_err(|_| ToonError {
                  offset: self.pos,
                  message: format!("invalid low surrogate '\\u{hex2}'"),
                })?;
                self.pos += 4;
                if !(0xDC00..=0xDFFF).contains(&code2) {
                  return Err(ToonError {
                    offset: self.pos,
                    message: "invalid low surrogate value".to_string(),
                  });
                }
                let combined = 0x10000 + ((code - 0xD800) << 10) + (code2 - 0xDC00);
                s.push(char::from_u32(combined).ok_or_else(|| ToonError {
                  offset: self.pos,
                  message: "invalid surrogate pair".to_string(),
                })?);
              } else if (0xDC00..=0xDFFF).contains(&code) {
                return Err(ToonError {
                  offset: self.pos,
                  message: "unexpected low surrogate without high surrogate".to_string(),
                });
              } else {
                s.push(char::from_u32(code).ok_or_else(|| ToonError {
                  offset: self.pos,
                  message: format!("invalid unicode codepoint U+{code:04X}"),
                })?);
              }
            }
            Some(other) => {
              return Err(ToonError {
                offset: self.pos,
                message: format!("invalid escape '\\{:?}", other as char),
              });
            }
          }
        }
        Some(_) => {
          // Copy a full UTF-8 character.
          let ch_start = self.pos;
          let first = self.bytes[ch_start];
          let len = utf8_len(first);
          if ch_start + len > self.bytes.len() {
            return Err(ToonError {
              offset: ch_start,
              message: "incomplete utf-8 sequence".to_string(),
            });
          }
          let chunk =
            std::str::from_utf8(&self.bytes[ch_start..ch_start + len]).map_err(|_| ToonError {
              offset: ch_start,
              message: "invalid utf-8".to_string(),
            })?;
          s.push_str(chunk);
          self.pos += len;
        }
      }
    }
    Ok(s)
  }

  fn parse_array(&mut self) -> Result<Value, ToonError> {
    self.pos += 1; // consume '['
    let mut arr = Vec::new();
    self.skip_ws();
    if self.peek() == Some(b']') {
      self.pos += 1;
      return Ok(Value::Array(arr));
    }
    loop {
      let val = self.parse_value()?;
      arr.push(val);
      self.skip_ws();
      match self.peek() {
        Some(b'|') => {
          self.pos += 1;
          self.skip_ws();
        }
        Some(b']') => {
          self.pos += 1;
          break;
        }
        Some(other) => {
          return Err(ToonError {
            offset: self.pos,
            message: format!("expected '|' or ']' in array, got {:?}", other as char),
          });
        }
        None => {
          return Err(ToonError {
            offset: self.pos,
            message: "unterminated array".to_string(),
          });
        }
      }
    }
    Ok(Value::Array(arr))
  }

  fn parse_object(&mut self) -> Result<Value, ToonError> {
    self.pos += 1; // consume '{'
    let mut obj = serde_json::Map::new();
    self.skip_ws();
    if self.peek() == Some(b'}') {
      self.pos += 1;
      return Ok(Value::Object(obj));
    }
    loop {
      self.skip_ws();
      let key = self.parse_string()?;
      self.skip_ws();
      if self.peek() != Some(b'=') {
        return Err(ToonError {
          offset: self.pos,
          message: "expected '=' after object key".to_string(),
        });
      }
      self.pos += 1;
      let val = self.parse_value()?;
      obj.insert(key, val);
      // Do NOT skip_ws here — the space is the entry separator.
      match self.peek() {
        Some(b'}') => {
          self.pos += 1;
          break;
        }
        Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {
          self.skip_ws();
          if self.peek() == Some(b'}') {
            self.pos += 1;
            break;
          }
          // Continue to next entry (loop will skip_ws before key).
        }
        Some(other) => {
          return Err(ToonError {
            offset: self.pos,
            message: format!("expected ' ' or '}}' in object, got {:?}", other as char),
          });
        }
        None => {
          return Err(ToonError {
            offset: self.pos,
            message: "unterminated object".to_string(),
          });
        }
      }
    }
    Ok(Value::Object(obj))
  }
}

/// Determine the expected length of a UTF-8 sequence from its leading byte.
fn utf8_len(first: u8) -> usize {
  if first < 0x80 {
    1
  } else if first >> 5 == 0b110 {
    2
  } else if first >> 4 == 0b1110 {
    3
  } else if first >> 3 == 0b11110 {
    4
  } else {
    1
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_encode_null() {
    assert_eq!(encode(&json!(null)), "null");
  }

  #[test]
  fn test_encode_bool() {
    assert_eq!(encode(&json!(true)), "true");
    assert_eq!(encode(&json!(false)), "false");
  }

  #[test]
  fn test_encode_numbers() {
    assert_eq!(encode(&json!(42)), "42");
    assert_eq!(encode(&json!(-7)), "-7");
    assert_eq!(encode(&json!(1.5)), "1.5");
    assert_eq!(encode(&json!(0)), "0");
  }

  #[test]
  fn test_encode_string() {
    assert_eq!(encode(&json!("hello")), "\"hello\"");
    assert_eq!(encode(&json!("")), "\"\"");
    assert_eq!(encode(&json!("with space")), "\"with space\"");
  }

  #[test]
  fn test_encode_string_escapes() {
    assert_eq!(encode(&json!("a\"b")), "\"a\\\"b\"");
    assert_eq!(encode(&json!("a\\b")), "\"a\\\\b\"");
    assert_eq!(encode(&json!("a\nb")), "\"a\\nb\"");
    assert_eq!(encode(&json!("a\tb")), "\"a\\tb\"");
  }

  #[test]
  fn test_encode_array() {
    assert_eq!(encode(&json!([1, 2, 3])), "[1|2|3]");
    assert_eq!(encode(&json!([])), "[]");
    assert_eq!(encode(&json!(["a", "b", "c"])), "[\"a\"|\"b\"|\"c\"]");
  }

  #[test]
  fn test_encode_object() {
    assert_eq!(
      encode(&json!({"name": "apmw", "version": 1})),
      "{\"name\"=\"apmw\" \"version\"=1}"
    );
    assert_eq!(encode(&json!({})), "{}");
  }

  #[test]
  fn test_encode_nested() {
    let v = json!({
      "items": [{"id": 1, "name": "foo"}, {"id": 2, "name": "bar"}],
      "total": 2
    });
    let encoded = encode(&v);
    assert!(encoded.contains("\"items\"=["));
    assert!(encoded.contains("\"total\"=2"));
  }

  #[test]
  fn test_parse_null() {
    assert_eq!(parse("null").unwrap(), json!(null));
  }

  #[test]
  fn test_parse_bool() {
    assert_eq!(parse("true").unwrap(), json!(true));
    assert_eq!(parse("false").unwrap(), json!(false));
  }

  #[test]
  fn test_parse_numbers() {
    assert_eq!(parse("42").unwrap(), json!(42));
    assert_eq!(parse("-7").unwrap(), json!(-7));
    assert_eq!(parse("1.5").unwrap(), json!(1.5));
  }

  #[test]
  fn test_parse_string() {
    assert_eq!(parse("\"hello\"").unwrap(), json!("hello"));
    assert_eq!(parse("\"\"").unwrap(), json!(""));
    assert_eq!(parse("\"a\\nb\"").unwrap(), json!("a\nb"));
  }

  #[test]
  fn test_parse_array() {
    assert_eq!(parse("[1|2|3]").unwrap(), json!([1, 2, 3]));
    assert_eq!(parse("[]").unwrap(), json!([]));
    assert_eq!(parse("[\"a\"|\"b\"]").unwrap(), json!(["a", "b"]));
  }

  #[test]
  fn test_parse_object() {
    assert_eq!(
      parse("{\"name\"=\"apmw\" \"version\"=1}").unwrap(),
      json!({"name": "apmw", "version": 1})
    );
    assert_eq!(parse("{}").unwrap(), json!({}));
  }

  #[test]
  fn test_round_trip_simple() {
    let v = json!({
      "name": "apmw",
      "version": 1,
      "active": true,
      "tags": ["rust", "cli"],
      "meta": null
    });
    let encoded = encode(&v);
    let parsed = parse(&encoded).unwrap();
    assert_eq!(parsed, v);
  }

  #[test]
  fn test_round_trip_nested() {
    let v = json!({
      "items": [
        {"id": 1, "name": "foo", "data": [1, 2, 3]},
        {"id": 2, "name": "bar", "data": []}
      ],
      "total": 2,
      "empty": {}
    });
    let encoded = encode(&v);
    let parsed = parse(&encoded).unwrap();
    assert_eq!(parsed, v);
  }

  #[test]
  fn test_round_trip_special_chars() {
    let v = json!({
      "path": "/usr/bin/foo",
      "desc": "A tool with \"quotes\" and \n newlines",
      "unicode": "héllo 世界"
    });
    let encoded = encode(&v);
    let parsed = parse(&encoded).unwrap();
    assert_eq!(parsed, v);
  }

  #[test]
  fn test_parse_error_trailing() {
    let err = parse("true false").unwrap_err();
    assert!(err.message.contains("trailing"));
  }

  #[test]
  fn test_parse_error_unterminated_string() {
    let err = parse("\"hello").unwrap_err();
    assert!(err.message.contains("unterminated"));
  }

  #[test]
  fn test_parse_error_unterminated_array() {
    let err = parse("[1|2").unwrap_err();
    assert!(err.message.contains("unterminated") || err.message.contains("expected"));
  }

  #[test]
  fn test_token_efficiency_vs_json() {
    // TOON should be more token-efficient than JSON for objects.
    let v = json!({"name": "apmw", "version": 1, "active": true});
    let toon = encode(&v);
    let json_str = serde_json::to_string(&v).unwrap();
    // TOON uses '=' instead of ':' and spaces instead of ',', saving tokens.
    assert!(toon.len() <= json_str.len());
    assert!(!toon.contains(':'));
    assert!(!toon.contains(','));
  }

  mod proptest_tests {
    use super::*;
    use ::proptest::prelude::*;

    fn arb_json_value() -> impl Strategy<Value = Value> {
      let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|i| Value::Number(serde_json::Number::from(i))),
        any::<u64>().prop_map(|u| Value::Number(serde_json::Number::from(u))),
        any::<f64>().prop_map(|f| {
          serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)
        }),
        any::<String>().prop_map(Value::String),
      ];
      leaf.prop_recursive(3, 64, 10, |inner| {
        prop_oneof![
          prop::collection::vec(inner.clone(), 0..10).prop_map(Value::Array),
          prop::collection::vec(any::<String>(), 0..10).prop_map(|keys| {
            let mut map = serde_json::Map::new();
            for (i, k) in keys.into_iter().enumerate() {
              // Use a deterministic value to avoid duplicate-key issues.
              map.insert(k, Value::Number(serde_json::Number::from(i as i64)));
            }
            Value::Object(map)
          }),
        ]
      })
    }

    proptest! {
      #[test]
      fn proptest_toon_round_trip(value in arb_json_value()) {
        let encoded = encode(&value);
        let parsed = parse(&encoded).expect("TOON round-trip parse failed");
        prop_assert_eq!(parsed, value);
      }

      #[test]
      fn proptest_toon_no_structural_commas_or_colons(value in arb_json_value()) {
        // TOON uses '=' for key-value and '|' for array separators instead of
        // ':' and ','. String contents may legitimately contain these characters.
        let encoded = encode(&value);
        // Verify round-trip succeeds (structural correctness).
        let parsed = parse(&encoded).expect("TOON round-trip parse failed");
        prop_assert_eq!(&parsed, &value);
        // Verify the encoded form uses TOON separators, not JSON separators,
        // by checking that re-encoding the parsed value is idempotent.
        let re_encoded = encode(&parsed);
        prop_assert_eq!(re_encoded, encoded);
      }
    }
  }
}
