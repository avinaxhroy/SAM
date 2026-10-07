//! Path-aware decoding cursor (§4.3 address contract), ported from
//! `JSONCursor.swift`. Walks a parsed `JSONValue` tree, throwing a single
//! path-precise [`Diagnostic`] on the first structural problem.
//!
//! Semantic validation — which must report EVERY bad line — happens after
//! decode in `validator`, not here.

use crate::diagnostic::{Diagnostic, DiagnosticError, escape_pointer_segment};
use crate::json_value::JSONValue;

pub struct Cursor<'a> {
    /// File-relative path, e.g. `content/types.json`.
    pub file: String,
    /// RFC 6901 pointer segments after the file.
    pub path: Vec<String>,
    pub value: &'a JSONValue,
}

impl<'a> Cursor<'a> {
    pub fn new(file: impl Into<String>, path: Vec<String>, value: &'a JSONValue) -> Self {
        Self {
            file: file.into(),
            path,
            value,
        }
    }

    pub fn root(file: impl Into<String>, value: &'a JSONValue) -> Self {
        Self::new(file, Vec::new(), value)
    }

    pub fn address(&self) -> String {
        if self.path.is_empty() {
            self.file.clone()
        } else {
            format!("{}#{}", self.file, pointer(&self.path))
        }
    }

    fn child(&self, key: &str, value: &'a JSONValue) -> Self {
        let mut path = self.path.clone();
        path.push(key.to_string());
        Self::new(self.file.clone(), path, value)
    }

    pub fn mismatch(&self, want: &str) -> DiagnosticError {
        DiagnosticError::new(Diagnostic::error(
            "shape.type-mismatch",
            self.address(),
            format!("expected {want}, found {}", describe(self.value)),
        ))
    }

    pub fn mismatch_object(&self) -> DiagnosticError {
        DiagnosticError::new(Diagnostic::error(
            "shape.type-mismatch",
            self.address(),
            "expected an object",
        ))
    }

    pub fn mismatch_array(&self) -> DiagnosticError {
        DiagnosticError::new(Diagnostic::error(
            "shape.type-mismatch",
            self.address(),
            "expected an array",
        ))
    }

    fn missing_key(&self, key: &str) -> DiagnosticError {
        DiagnosticError::new(Diagnostic::error(
            "shape.missing-key",
            self.address(),
            format!("required key \"{key}\" is missing"),
        ))
    }

    fn members_map(&self) -> Result<&'a serde_json::Map<String, JSONValue>, DiagnosticError> {
        match self.value {
            JSONValue::Object(map) => Ok(map),
            _ => Err(self.mismatch("an object")),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.value, JSONValue::Null)
    }

    /// A required member. Present-and-non-null, or `shape.missing-key`.
    fn required(&self, key: &str) -> Result<&'a JSONValue, DiagnosticError> {
        match self.members_map()?.get(key) {
            Some(value) if !matches!(value, JSONValue::Null) => Ok(value),
            _ => Err(self.missing_key(key)),
        }
    }

    /// An optional member. Absent or null → `None`; present but the wrong
    /// shape is still a diagnostic, never silently ignored (§3.7: defaults
    /// apply to absent values, never to malformed ones).
    fn optional(&self, key: &str) -> Result<Option<&'a JSONValue>, DiagnosticError> {
        match self.members_map()?.get(key) {
            None => Ok(None),
            Some(JSONValue::Null) => Ok(None),
            Some(value) => Ok(Some(value)),
        }
    }

    pub fn object(&self, key: &str) -> Result<Cursor<'a>, DiagnosticError> {
        let value = self.required(key)?;
        match value {
            JSONValue::Object(_) => Ok(self.child(key, value)),
            _ => Err(self.child(key, value).mismatch("an object")),
        }
    }

    pub fn optional_object(&self, key: &str) -> Result<Option<Cursor<'a>>, DiagnosticError> {
        match self.optional(key)? {
            None => Ok(None),
            Some(value) => match value {
                JSONValue::Object(_) => Ok(Some(self.child(key, value))),
                _ => Err(self.child(key, value).mismatch("an object")),
            },
        }
    }

    pub fn string(&self, key: &str) -> Result<String, DiagnosticError> {
        let value = self.required(key)?;
        match value {
            JSONValue::String(text) => Ok(text.clone()),
            _ => Err(self.child(key, value).mismatch("a string")),
        }
    }

    pub fn optional_string(&self, key: &str) -> Result<Option<String>, DiagnosticError> {
        match self.optional(key)? {
            None => Ok(None),
            Some(value) => match value {
                JSONValue::String(text) => Ok(Some(text.clone())),
                _ => Err(self.child(key, value).mismatch("a string")),
            },
        }
    }

    pub fn optional_int(&self, key: &str) -> Result<Option<i64>, DiagnosticError> {
        match self.optional(key)? {
            None => Ok(None),
            Some(value) => match value.as_i64() {
                Some(number) => Ok(Some(number)),
                None => Err(self.child(key, value).mismatch("an integer")),
            },
        }
    }

    pub fn optional_bool(&self, key: &str) -> Result<Option<bool>, DiagnosticError> {
        match self.optional(key)? {
            None => Ok(None),
            Some(value) => match value {
                JSONValue::Bool(flag) => Ok(Some(*flag)),
                _ => Err(self.child(key, value).mismatch("a boolean")),
            },
        }
    }

    /// A finite number, strictly typed (§4.4 rejects implicit coercion; an
    /// integer is a number). Absent = `None`, so a caller can tell "not
    /// declared" from "declared zero".
    pub fn optional_number(&self, key: &str) -> Result<Option<f64>, DiagnosticError> {
        match self.optional(key)? {
            None => Ok(None),
            Some(value) => match value.as_f64() {
                Some(number) => Ok(Some(number)),
                None => Err(self.child(key, value).mismatch("a number")),
            },
        }
    }

    pub fn array(&self, key: &str) -> Result<Vec<Cursor<'a>>, DiagnosticError> {
        let value = self.required(key)?;
        let JSONValue::Array(items) = value else {
            return Err(self.child(key, value).mismatch("an array"));
        };
        Ok(items
            .iter()
            .enumerate()
            .map(|(index, item)| self.child_at(key, index, item))
            .collect())
    }

    /// Like [`Self::array`], but an absent or null member is `None` — the
    /// difference between "no blocks declared" and "blocks: null" does not
    /// exist for the reader, so both mean the same thing here.
    pub fn optional_array(&self, key: &str) -> Result<Option<Vec<Cursor<'a>>>, DiagnosticError> {
        let Some(value) = self.optional(key)? else {
            return Ok(None);
        };
        let JSONValue::Array(items) = value else {
            return Err(self.child(key, value).mismatch("an array"));
        };
        Ok(Some(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| self.child_at(key, index, item))
                .collect(),
        ))
    }

    /// Like [`Self::array`], but an absent or null member is an empty list.
    pub fn string_array(&self, key: &str) -> Result<Vec<String>, DiagnosticError> {
        let Some(value) = self.optional(key)? else {
            return Ok(Vec::new());
        };
        let JSONValue::Array(items) = value else {
            return Err(self.child(key, value).mismatch("an array"));
        };
        items
            .iter()
            .enumerate()
            .map(|(index, item)| match item {
                JSONValue::String(text) => Ok(text.clone()),
                _ => Err(self.child_at(key, index, item).mismatch("a string")),
            })
            .collect()
    }

    /// A number array, strictly typed: a string inside it is a mismatch, not a
    /// silent zero (§4.4 rejects implicit coercion). Absent = `None`, so a
    /// caller can tell "not declared" from "declared empty".
    pub fn number_array(&self, key: &str) -> Result<Option<Vec<f64>>, DiagnosticError> {
        let Some(value) = self.optional(key)? else {
            return Ok(None);
        };
        let JSONValue::Array(items) = value else {
            return Err(self.child(key, value).mismatch("an array of numbers"));
        };
        let mut out = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            match item.as_f64() {
                Some(number) if number.is_finite() => out.push(number),
                _ => return Err(self.child_at(key, index, item).mismatch("a number")),
            }
        }
        Ok(Some(out))
    }

    fn child_at(&self, key: &str, index: usize, value: &'a JSONValue) -> Cursor<'a> {
        let mut path = self.path.clone();
        path.push(key.to_string());
        path.push(index.to_string());
        Cursor::new(self.file.clone(), path, value)
    }

    /// Raw member lookup: `(key, cursor)` pairs, key-sorted so diagnostics and
    /// iteration are deterministic.
    pub fn members(&self) -> Result<Vec<(String, Cursor<'a>)>, DiagnosticError> {
        let map = self.members_map()?;
        Ok(map
            .iter()
            .map(|(key, value)| (key.clone(), self.child(key, value)))
            .collect())
    }

    /// Raw array elements.
    pub fn elements(&self) -> Result<Vec<Cursor<'a>>, DiagnosticError> {
        match self.value {
            JSONValue::Array(items) => Ok(items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let mut path = self.path.clone();
                    path.push(index.to_string());
                    Cursor::new(self.file.clone(), path, item)
                })
                .collect()),
            _ => Err(self.mismatch("an array")),
        }
    }

    /// `schemaVersion` (§3.1, D10): every document and record carries it;
    /// unknown future versions are rejected. `v1` is current.
    pub fn schema_version(&self, current: i64) -> Result<i64, DiagnosticError> {
        let value = match self.members_map()?.get("schemaVersion") {
            Some(value) if !matches!(value, JSONValue::Null) => value,
            _ => {
                return Err(DiagnosticError::new(Diagnostic::error(
                    "schema-version.missing",
                    self.address(),
                    "document must carry schemaVersion",
                )));
            }
        };
        let Some(number) = value.as_i64() else {
            return Err(self
                .child("schemaVersion", value)
                .mismatch("an integer schemaVersion"));
        };
        if number != current {
            return Err(DiagnosticError::new(Diagnostic::error(
                "schema-version.unsupported",
                self.address(),
                format!("schemaVersion {number} is not supported (current: {current})"),
            )));
        }
        Ok(number)
    }
}

fn describe(value: &JSONValue) -> &'static str {
    match value {
        JSONValue::Null => "null",
        JSONValue::Bool(_) => "a boolean",
        JSONValue::Number(_) => "a number",
        JSONValue::String(_) => "a string",
        JSONValue::Array(_) => "an array",
        JSONValue::Object(_) => "an object",
    }
}

/// RFC 6901 over already-escaped segments (§4.3), re-exported for callers that
/// build a pointer from scratch.
pub fn pointer(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| format!("/{}", escape_pointer_segment(segment)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> JSONValue {
        serde_json::json!({
            "schemaVersion": 1,
            "name": "topic",
            "fields": [{"key": "title", "type": "text"}],
            "extra": null
        })
    }

    #[test]
    fn a_missing_required_key_reports_its_address() {
        let tree = tree();
        let cursor = Cursor::root("content/types.json", &tree);
        let error = cursor.string("absent").expect_err("absent is not a string");
        assert_eq!(error.diagnostic.code, "shape.missing-key");
        assert_eq!(error.diagnostic.path, "content/types.json");
    }

    #[test]
    fn a_wrong_shape_reports_the_child_pointer() {
        let tree = tree();
        let cursor = Cursor::root("content/types.json", &tree);
        let error = cursor
            .string("fields")
            .expect_err("an array is not a string");
        assert_eq!(error.diagnostic.code, "shape.type-mismatch");
        assert_eq!(error.diagnostic.path, "content/types.json#/fields");
    }

    #[test]
    fn a_null_optional_is_absent_but_a_wrong_shape_is_not() {
        let tree = tree();
        let cursor = Cursor::root("content/types.json", &tree);
        assert!(
            cursor
                .optional_string("extra")
                .expect("null is absent")
                .is_none()
        );
        assert!(cursor.optional_string("fields").is_err());
    }

    #[test]
    fn schema_version_is_required_and_bounded() {
        let tree = tree();
        assert_eq!(
            Cursor::root("x.json", &tree)
                .schema_version(1)
                .expect("v1 is current"),
            1
        );
        let future = serde_json::json!({"schemaVersion": 2});
        let error = Cursor::root("x.json", &future)
            .schema_version(1)
            .expect_err("a future version is rejected");
        assert_eq!(error.diagnostic.code, "schema-version.unsupported");

        let absent = serde_json::json!({});
        let error = Cursor::root("x.json", &absent)
            .schema_version(1)
            .expect_err("schemaVersion is mandatory");
        assert_eq!(error.diagnostic.code, "schema-version.missing");
    }
}
