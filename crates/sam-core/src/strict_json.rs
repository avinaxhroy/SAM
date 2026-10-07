//! Strict JSON parser (§4.1).
//!
//! Rejects duplicate object keys, invalid encodings (including BOM),
//! nesting beyond 64 levels (`json.too-deep`), and malformed JSON before
//! values reach the engine (§9).

use serde::Deserialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};

use crate::diagnostic::{Diagnostic, DiagnosticError};
use crate::json_value::JSONValue;

/// Maximum parser nesting depth (`json.too-deep`).
const DEPTH_LIMIT: usize = 64;

/// Marks our own error so it can be told apart from `serde_json`'s.
const DUPLICATE_PREFIX: &str = "duplicate object key ";

/// A `serde_json::Value` that refuses duplicate object keys.
struct StrictValue(JSONValue);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct StrictVisitor;

        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = JSONValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<JSONValue, E> {
                Ok(JSONValue::Bool(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<JSONValue, E> {
                Ok(JSONValue::from(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<JSONValue, E> {
                Ok(JSONValue::from(value))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<JSONValue, E> {
                Ok(serde_json::Number::from_f64(value)
                    .map(JSONValue::Number)
                    .unwrap_or(JSONValue::Null))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<JSONValue, E> {
                Ok(JSONValue::String(value.to_string()))
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<JSONValue, E> {
                Ok(JSONValue::String(value))
            }

            fn visit_unit<E: de::Error>(self) -> Result<JSONValue, E> {
                Ok(JSONValue::Null)
            }

            fn visit_none<E: de::Error>(self) -> Result<JSONValue, E> {
                Ok(JSONValue::Null)
            }

            fn visit_seq<A>(self, mut access: A) -> Result<JSONValue, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = access.next_element::<StrictValue>()? {
                    items.push(item.0);
                }
                Ok(JSONValue::Array(items))
            }

            fn visit_map<A>(self, mut access: A) -> Result<JSONValue, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut members = serde_json::Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    let value = access.next_value::<StrictValue>()?;
                    if members.contains_key(&key) {
                        // First occurrence wins; the finding is the error
                        // (§4.1). Report the key so the caller can name it.
                        return Err(de::Error::custom(format!(
                            "{DUPLICATE_PREFIX}\"{key}\" (first occurrence kept)"
                        )));
                    }
                    members.insert(key, value.0);
                }
                Ok(JSONValue::Object(members))
            }
        }

        deserializer.deserialize_any(StrictVisitor).map(StrictValue)
    }
}

/// Parse a complete JSON document. Errors on the first lexical problem (syntax
/// or duplicate key) with its line where one is available.
pub fn parse(text: &str, file: &str) -> Result<JSONValue, DiagnosticError> {
    let value = parse_tree(text, file, None)?;
    check_depth(&value, file)?;
    Ok(value)
}

/// Parse one JSONL line (§4.1). The line number is the caller's, so a lexical
/// diagnostic inside a line still points at the line, not at column one of the
/// line in isolation.
pub fn parse_line(text: &str, file: &str, line_no: u64) -> Result<JSONValue, DiagnosticError> {
    let value = parse_tree(text, file, Some(line_no))?;
    check_depth(&value, file).map_err(|error| DiagnosticError {
        diagnostic: Diagnostic {
            line: Some(line_no),
            ..error.diagnostic
        },
    })?;
    Ok(value)
}

fn parse_tree(text: &str, file: &str, line_no: Option<u64>) -> Result<JSONValue, DiagnosticError> {
    if text.starts_with('\u{feff}') {
        return Err(DiagnosticError::new(
            Diagnostic::error("encoding.bom", file, "byte-order mark is not valid JSON")
                .at_line(line_no.unwrap_or(1)),
        ));
    }

    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value = StrictValue::deserialize(&mut deserializer)
        .map_err(|error| map_error(&error, file, line_no))?
        .0;
    deserializer
        .end()
        .map_err(|error| map_error(&error, file, line_no))?;
    Ok(value)
}

/// `serde_json`'s error → the engine's diagnostic vocabulary.
fn map_error(error: &serde_json::Error, file: &str, line_no: Option<u64>) -> DiagnosticError {
    let message = error.to_string();
    let code = if message.contains(DUPLICATE_PREFIX) {
        "json.duplicate-key"
    } else if message.contains("recursion limit exceeded") {
        "json.too-deep"
    } else if error.is_eof() {
        // Empty input where a value was expected reports `json.eof`; truncated
        // structures (e.g. unclosed object or array) report `json.invalid`.
        if message.contains("EOF while parsing a value") {
            "json.eof"
        } else {
            "json.invalid"
        }
    } else {
        "json.invalid"
    };
    let line = line_no.or_else(|| (error.line() > 0).then(|| error.line() as u64));
    DiagnosticError::new(Diagnostic {
        code: code.into(),
        path: file.into(),
        line,
        message,
        severity: crate::diagnostic::Severity::Error,
    })
}

/// §4.1's depth cap, checked iteratively so a deep document cannot overflow the
/// stack while being measured.
fn check_depth(value: &JSONValue, file: &str) -> Result<(), DiagnosticError> {
    let mut frontier: Vec<(&JSONValue, usize)> = vec![(value, 1)];
    while let Some((node, depth)) = frontier.pop() {
        if depth > DEPTH_LIMIT {
            return Err(DiagnosticError::new(Diagnostic::error(
                "json.too-deep",
                file,
                format!("nesting exceeds {DEPTH_LIMIT} levels"),
            )));
        }
        match node {
            JSONValue::Array(items) => {
                frontier.extend(items.iter().map(|item| (item, depth + 1)));
            }
            JSONValue::Object(members) => {
                frontier.extend(members.values().map(|member| (member, depth + 1)));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_keys_are_refused_and_the_first_occurrence_is_named() {
        let error = parse(r#"{"a":1,"a":2}"#, "content/types.json")
            .expect_err("a duplicate object key is not valid JSON (§4.1)");
        assert_eq!(error.diagnostic.code, "json.duplicate-key");
        assert_eq!(error.diagnostic.path, "content/types.json");
        assert!(error.diagnostic.message.contains("\"a\""));
    }

    #[test]
    fn nested_duplicate_keys_are_refused_too() {
        let error = parse(r#"{"outer":{"a":1,"a":2}}"#, "x.json")
            .expect_err("duplicates are rejected at every depth");
        assert_eq!(error.diagnostic.code, "json.duplicate-key");
    }

    #[test]
    fn a_bom_is_not_json() {
        let error = parse("\u{feff}{}", "x.json").expect_err("a BOM is refused");
        assert_eq!(error.diagnostic.code, "encoding.bom");
        assert_eq!(error.diagnostic.line, Some(1));
    }

    #[test]
    fn empty_input_is_eof_not_an_empty_document() {
        let error = parse("", "content/types.json").expect_err("empty input is refused");
        assert_eq!(error.diagnostic.code, "json.eof");
    }

    #[test]
    fn eof_where_a_value_was_expected_is_eof_but_a_truncated_structure_is_not() {
        // Nothing to parse versus an unterminated object: two different repairs.
        let eof = parse("", "x.json").expect_err("nothing is not a document");
        assert_eq!(eof.diagnostic.code, "json.eof");

        let truncated = parse("{\"broken\"", "x.json")
            .expect_err("an unterminated object is malformed, not empty");
        assert_eq!(truncated.diagnostic.code, "json.invalid");
        assert_eq!(truncated.diagnostic.line, Some(1));

        let missing_value = parse("{\"a\":", "x.json").expect_err("a missing value is an error");
        assert_eq!(missing_value.diagnostic.code, "json.eof");
    }

    #[test]
    fn malformed_input_carries_a_line() {
        let error = parse("{\n \"a\": \n}", "x.json").expect_err("malformed JSON is refused");
        assert_eq!(error.diagnostic.code, "json.invalid");
        assert!(error.diagnostic.line.is_some());
    }

    #[test]
    fn a_line_number_is_the_callers_not_the_parsers() {
        let error = parse_line("this is not json", "content/records/topic.jsonl", 2)
            .expect_err("a malformed JSONL line is refused");
        assert_eq!(error.diagnostic.code, "json.invalid");
        assert_eq!(error.diagnostic.line, Some(2));
    }

    #[test]
    fn trailing_content_is_refused() {
        let error = parse("{} {}", "x.json").expect_err("trailing content is not tolerated");
        assert_eq!(error.diagnostic.code, "json.invalid");
    }

    #[test]
    fn depth_is_capped_at_the_swift_limit() {
        let deep = format!("{}{}", "[".repeat(65), "]".repeat(65));
        let error = parse(&deep, "x.json").expect_err("65 levels exceeds the cap");
        assert_eq!(error.diagnostic.code, "json.too-deep");

        let ok = format!("{}{}", "[".repeat(64), "]".repeat(64));
        assert!(parse(&ok, "x.json").is_ok(), "64 levels is inside the cap");
    }

    #[test]
    fn integers_stay_integers() {
        let value = parse(r#"{"n":20,"f":1.5}"#, "x.json").expect("valid JSON");
        assert_eq!(value["n"], JSONValue::from(20));
        assert!(value["n"].is_i64() || value["n"].is_u64());
        assert!(value["f"].is_f64());
    }
}
