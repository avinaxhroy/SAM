//! Validation diagnostic representations (§3.7, §4.3, §4.9).
//!
//! Captures precise error locations with RFC 6901 JSON pointers, file-relative paths,
//! line numbers, and machine-readable error codes.

use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub path: String,
    pub line: Option<u64>,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
            line: None,
            message: message.into(),
            severity: Severity::Error,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, path, message)
        }
    }

    pub fn at_line(mut self, line: u64) -> Self {
        self.line = Some(line);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{}: {}", self.path, line, self.message)?,
            None => write!(f, "{}: {}", self.path, self.message)?,
        }
        if self.severity == Severity::Warning {
            write!(f, " (warning)")?;
        }
        write!(f, " [{}]", self.code)
    }
}

/// One finding as an error — what a cursor or parser throws. The store
/// collects these into a [`ValidationError`] so a load reports every file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticError {
    pub diagnostic: Diagnostic,
}

impl DiagnosticError {
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self { diagnostic }
    }
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.diagnostic)
    }
}

impl std::error::Error for DiagnosticError {}

/// All validation findings of one load (§3.7: every bad line reported, and a
/// managed commit is all-or-nothing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationError {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// RFC 6901 pointer segment escaping (§4.3): `~` → `~0`, `/` → `~1`.
pub fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// A JSON Pointer (`/a/b`) over already-escaped segments.
pub fn pointer(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| format!("/{}", escape_pointer_segment(segment)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_escapes_the_two_reserved_characters() {
        assert_eq!(
            pointer(&["a~b".into(), "c/d".into()]),
            "/a~0b/c~1d",
            "§4.3: ~ and / are escaped in pointer segments"
        );
    }

    #[test]
    fn a_diagnostic_renders_its_path_line_and_code() {
        let diagnostic =
            Diagnostic::error("json.invalid", "content/records/topic.jsonl", "bad").at_line(2);
        assert_eq!(
            diagnostic.to_string(),
            "content/records/topic.jsonl:2: bad [json.invalid]"
        );
        assert!(diagnostic.is_error());
        assert!(!Diagnostic::warning("w", "p", "m").is_error());
    }
}
