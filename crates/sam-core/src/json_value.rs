//! `JSONValue` — the record's value type (§3.1: a record's `fields` is a
//! `[String: JSONValue]` document).
//!
//! Phase 1 replaces this thin alias with `strict_json.rs`, which rejects
//! duplicate object keys and invalid encoding before a value ever exists
//! (§4.1, §3.7). Until then `serde_json` is the only JSON in the crate.

pub use serde_json::Value as JSONValue;

/// The scalar text of a value, for human output only — the CLI's tables and
/// the content tree. Composite values have no scalar form; they print as
/// `(structure)` rather than pretending to be empty.
pub fn scalar_text(value: &JSONValue) -> String {
    match value {
        JSONValue::Null => "null".to_string(),
        JSONValue::Bool(flag) => flag.to_string(),
        JSONValue::Number(number) => number.to_string(),
        JSONValue::String(text) => text.clone(),
        JSONValue::Array(_) | JSONValue::Object(_) => "(structure)".to_string(),
    }
}

/// The scalar text of a value, or nothing when it has no scalar form. Alias
/// substitution needs the difference between "the string `null`" and "no
/// scalar form at all", so it cannot use [`scalar_text`].
pub fn scalar_text_opt(value: &JSONValue) -> Option<String> {
    match value {
        JSONValue::Null | JSONValue::Array(_) | JSONValue::Object(_) => None,
        JSONValue::Bool(flag) => Some(flag.to_string()),
        JSONValue::Number(number) => Some(number.to_string()),
        JSONValue::String(text) => Some(text.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_text_never_invents_a_value() {
        assert_eq!(scalar_text(&JSONValue::Null), "null");
        assert_eq!(scalar_text(&serde_json::json!(12)), "12");
        assert_eq!(scalar_text(&serde_json::json!(true)), "true");
        assert_eq!(scalar_text(&serde_json::json!("hi")), "hi");
        assert_eq!(scalar_text(&serde_json::json!([1, 2])), "(structure)");
    }

    #[test]
    fn scalar_text_opt_distinguishes_no_scalar_from_null() {
        assert_eq!(scalar_text_opt(&JSONValue::Null), None);
        assert_eq!(
            scalar_text_opt(&serde_json::json!("null")),
            Some("null".into())
        );
        assert_eq!(scalar_text_opt(&serde_json::json!(0)), Some("0".into()));
    }
}
