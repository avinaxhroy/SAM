//! Layer 2 safe expressions (§4.4): literals, dotted paths, arithmetic,
//! comparisons, boolean logic, ternary, and a fixed function catalog.
//!
//! Turing-incomplete by design: no loops, no assignment, no I/O, and no user-defined
//! functions, keeping configuration offline-validatable and statically verifiable (D6).
//!
//! Static checks run at validation time before any commit; evaluation errors
//! surface as null plus a diagnostic without persisting replacement values.

use crate::diagnostic::{Diagnostic, DiagnosticError};

/// One parsed expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Double(f64),
    String(String),
    Bool(bool),
    Null,
    Path(Vec<String>),
    Call(String, Vec<Expr>),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Every dotted path in the expression, in source order — for static
    /// validation and dependency analysis.
    pub fn paths(&self) -> Vec<Vec<String>> {
        match self {
            Expr::Int(_) | Expr::Double(_) | Expr::String(_) | Expr::Bool(_) | Expr::Null => {
                Vec::new()
            }
            Expr::Path(segments) => vec![segments.clone()],
            Expr::Call(_, args) => args.iter().flat_map(Expr::paths).collect(),
            Expr::Unary(_, inner) => inner.paths(),
            Expr::Binary(_, left, right) => {
                let mut paths = left.paths();
                paths.extend(right.paths());
                paths
            }
            Expr::Ternary(condition, then_branch, else_branch) => {
                let mut paths = condition.paths();
                paths.extend(then_branch.paths());
                paths.extend(else_branch.paths());
                paths
            }
        }
    }
}

/// The published limits (§4.4: "set explicit … limits in the schema; surface
/// limit errors"). Surfaced verbatim in `--schema`.
pub const MAX_EXPRESSION_LENGTH: usize = 512;
pub const MAX_DEPTH: usize = 32;
pub const MAX_PATH_SEGMENTS: usize = 8;
pub const MAX_RELATION_HOPS: usize = 5;
pub const MAX_EVALUATION_STEPS: usize = 10_000;

/// The fixed function list (§4.4). Adding one is a spec decision, not a
/// convenience.
pub const FUNCTIONS: [&str; 13] = [
    "sum",
    "count",
    "avg",
    "min",
    "max",
    "clamp",
    "round",
    "pct",
    "daysBetween",
    "weekOf",
    "today",
    "currentTerm",
    "complete",
];

/// Parse one expression. `at` is the diagnostic path (document role + JSON
/// pointer) so errors land path-precisely in `--configcheck`.
pub fn parse(text: &str, at: &str) -> Result<Expr, DiagnosticError> {
    if text.chars().count() > MAX_EXPRESSION_LENGTH {
        return Err(DiagnosticError::new(Diagnostic::error(
            "expr.too-long",
            at,
            format!(
                "expression exceeds {MAX_EXPRESSION_LENGTH} characters ({})",
                text.chars().count()
            ),
        )));
    }
    let mut parser = Parser {
        chars: text.chars().collect(),
        index: 0,
        at: at.to_string(),
    };
    let expr = parser.parse_expression(0)?;
    parser.expect_end()?;
    for segments in expr.paths() {
        if segments.len() > MAX_PATH_SEGMENTS {
            return Err(DiagnosticError::new(Diagnostic::error(
                "expr.path-too-long",
                at,
                format!(
                    "path \"{}\" exceeds {MAX_PATH_SEGMENTS} segments",
                    segments.join(".")
                ),
            )));
        }
    }
    Ok(expr)
}

// ── lexer/parser (recursive descent, single pass) ───────────────────────────

struct Parser {
    chars: Vec<char>,
    index: usize,
    at: String,
}

impl Parser {
    fn err(&self, code: &str, message: impl Into<String>) -> DiagnosticError {
        DiagnosticError::new(Diagnostic::error(code, &self.at, message))
    }

    fn cur(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn skip_ws(&mut self) {
        while matches!(self.cur(), Some(' ' | '\t' | '\n' | '\r')) {
            self.advance();
        }
    }

    fn expect_end(&mut self) -> Result<(), DiagnosticError> {
        self.skip_ws();
        if let Some(c) = self.cur() {
            return Err(self.err("expr.syntax", format!("unexpected trailing content '{c}'")));
        }
        Ok(())
    }

    fn parse_expression(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        if depth > MAX_DEPTH {
            return Err(self.err(
                "expr.too-deep",
                format!("expression nests deeper than {MAX_DEPTH} levels"),
            ));
        }
        let condition = self.parse_or(depth + 1)?;
        self.skip_ws();
        if self.cur() == Some('?') {
            self.advance();
            let then_branch = self.parse_expression(depth + 1)?;
            self.skip_ws();
            if self.cur() != Some(':') {
                return Err(self.err("expr.syntax", "expected ':' in ternary"));
            }
            self.advance();
            let else_branch = self.parse_expression(depth + 1)?;
            return Ok(Expr::Ternary(
                Box::new(condition),
                Box::new(then_branch),
                Box::new(else_branch),
            ));
        }
        Ok(condition)
    }

    fn parse_or(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        let mut lhs = self.parse_and(depth)?;
        loop {
            self.skip_ws();
            if self.cur() == Some('|') && self.peek() == Some('|') {
                self.advance();
                self.advance();
                let rhs = self.parse_and(depth)?;
                lhs = Expr::Binary("||".into(), Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    fn parse_and(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        let mut lhs = self.parse_equality(depth)?;
        loop {
            self.skip_ws();
            if self.cur() == Some('&') && self.peek() == Some('&') {
                self.advance();
                self.advance();
                let rhs = self.parse_equality(depth)?;
                lhs = Expr::Binary("&&".into(), Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    fn parse_equality(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        let mut lhs = self.parse_comparison(depth)?;
        loop {
            self.skip_ws();
            if self.cur() == Some('=') && self.peek() == Some('=') {
                self.advance();
                self.advance();
                let rhs = self.parse_comparison(depth)?;
                lhs = Expr::Binary("==".into(), Box::new(lhs), Box::new(rhs));
            } else if self.cur() == Some('!') && self.peek() == Some('=') {
                self.advance();
                self.advance();
                let rhs = self.parse_comparison(depth)?;
                lhs = Expr::Binary("!=".into(), Box::new(lhs), Box::new(rhs));
            } else {
                return Ok(lhs);
            }
        }
    }

    fn parse_comparison(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        let mut lhs = self.parse_additive(depth)?;
        loop {
            self.skip_ws();
            if self.cur() == Some('<') {
                self.advance();
                let op = if self.cur() == Some('=') {
                    self.advance();
                    "<="
                } else {
                    "<"
                };
                lhs = Expr::Binary(
                    op.into(),
                    Box::new(lhs),
                    Box::new(self.parse_additive(depth)?),
                );
            } else if self.cur() == Some('>') {
                self.advance();
                let op = if self.cur() == Some('=') {
                    self.advance();
                    ">="
                } else {
                    ">"
                };
                lhs = Expr::Binary(
                    op.into(),
                    Box::new(lhs),
                    Box::new(self.parse_additive(depth)?),
                );
            } else {
                return Ok(lhs);
            }
        }
    }

    fn parse_additive(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        let mut lhs = self.parse_multiplicative(depth)?;
        loop {
            self.skip_ws();
            let op = match self.cur() {
                Some('+') => "+",
                Some('-') => "-",
                _ => return Ok(lhs),
            };
            self.advance();
            lhs = Expr::Binary(
                op.into(),
                Box::new(lhs),
                Box::new(self.parse_multiplicative(depth)?),
            );
        }
    }

    fn parse_multiplicative(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        let mut lhs = self.parse_unary(depth)?;
        loop {
            self.skip_ws();
            let op = match self.cur() {
                Some('*') => "*",
                Some('/') => "/",
                Some('%') => "%",
                _ => return Ok(lhs),
            };
            self.advance();
            lhs = Expr::Binary(op.into(), Box::new(lhs), Box::new(self.parse_unary(depth)?));
        }
    }

    fn parse_unary(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        self.skip_ws();
        if self.cur() == Some('!') {
            self.advance();
            return Ok(Expr::Unary(
                "!".into(),
                Box::new(self.parse_unary(depth + 1)?),
            ));
        }
        if self.cur() == Some('-') {
            self.advance();
            return Ok(Expr::Unary(
                "-".into(),
                Box::new(self.parse_unary(depth + 1)?),
            ));
        }
        self.parse_primary(depth)
    }

    fn parse_primary(&mut self, depth: usize) -> Result<Expr, DiagnosticError> {
        self.skip_ws();
        let Some(c) = self.cur() else {
            return Err(self.err("expr.syntax", "expression ends where a value was expected"));
        };
        match c {
            '(' => {
                self.advance();
                let inner = self.parse_expression(depth + 1)?;
                self.skip_ws();
                if self.cur() != Some(')') {
                    return Err(self.err("expr.syntax", "expected ')'"));
                }
                self.advance();
                Ok(inner)
            }
            '\'' | '"' => Ok(Expr::String(self.parse_string()?)),
            '0'..='9' => self.parse_number(),
            _ => {
                if !c.is_alphabetic() && c != '_' {
                    return Err(self.err("expr.syntax", format!("unexpected character '{c}'")));
                }
                let ident = self.parse_ident();
                if self.cur() == Some('(') {
                    return self.parse_call(&ident);
                }
                match ident.as_str() {
                    "true" => return Ok(Expr::Bool(true)),
                    "false" => return Ok(Expr::Bool(false)),
                    "null" => return Ok(Expr::Null),
                    _ => {}
                }
                // Dotted path: `week.term.id`. No whitespace around dots.
                let mut segments = vec![ident];
                while self.cur() == Some('.')
                    && self.peek().is_some_and(|p| p.is_alphabetic() || p == '_')
                {
                    self.advance();
                    segments.push(self.parse_ident());
                }
                Ok(Expr::Path(segments))
            }
        }
    }

    fn parse_ident(&mut self) -> String {
        let mut text = String::new();
        while let Some(c) = self.cur() {
            if c.is_alphanumeric() || c == '_' {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }
        text
    }

    fn parse_call(&mut self, name: &str) -> Result<Expr, DiagnosticError> {
        if !FUNCTIONS.contains(&name) {
            let mut sorted = FUNCTIONS;
            sorted.sort_unstable();
            return Err(self.err(
                "expr.unknown-function",
                format!(
                    "unknown function \"{name}\" — the list is fixed: {}",
                    sorted.join(", ")
                ),
            ));
        }
        self.advance(); // (
        let mut args = Vec::new();
        self.skip_ws();
        if self.cur() == Some(')') {
            self.advance();
            return Ok(Expr::Call(name.into(), args));
        }
        loop {
            args.push(self.parse_expression(1)?);
            self.skip_ws();
            match self.cur() {
                Some(',') => {
                    self.advance();
                }
                Some(')') => {
                    self.advance();
                    return Ok(Expr::Call(name.into(), args));
                }
                _ => {
                    return Err(self.err(
                        "expr.syntax",
                        format!("expected ',' or ')' in {name}(…) arguments"),
                    ));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, DiagnosticError> {
        let quote = self.cur().expect("caller checked the quote");
        self.advance();
        let mut out = String::new();
        while let Some(c) = self.cur() {
            if c == quote {
                self.advance();
                return Ok(out);
            }
            if c == '\n' {
                return Err(self.err("expr.syntax", "unterminated string"));
            }
            if c == '\\' {
                self.advance();
                let Some(esc) = self.cur() else {
                    return Err(self.err("expr.syntax", "unterminated escape"));
                };
                match esc {
                    '\\' => out.push('\\'),
                    '\'' => out.push('\''),
                    '"' => out.push('"'),
                    other => {
                        return Err(self.err("expr.syntax", format!("invalid escape '\\{other}'")));
                    }
                }
                self.advance();
                continue;
            }
            out.push(c);
            self.advance();
        }
        Err(self.err("expr.syntax", "unterminated string"))
    }

    fn take_digits(&mut self, out: &mut String) {
        while let Some(c) = self.cur() {
            if !c.is_ascii_digit() {
                break;
            }
            out.push(c);
            self.advance();
        }
    }

    fn take_sign(&mut self, out: &mut String) {
        if let Some(c @ ('+' | '-')) = self.cur() {
            out.push(c);
            self.advance();
        }
    }

    fn parse_number(&mut self) -> Result<Expr, DiagnosticError> {
        let mut text = String::new();
        let mut is_double = false;
        self.take_digits(&mut text);
        if self.cur() == Some('.') && self.peek().is_some_and(|p| p.is_ascii_digit()) {
            is_double = true;
            text.push('.');
            self.advance();
            self.take_digits(&mut text);
        }
        if matches!(self.cur(), Some('e' | 'E')) {
            is_double = true;
            text.push('e');
            self.advance();
            self.take_sign(&mut text);
            if !self.cur().is_some_and(|c| c.is_ascii_digit()) {
                return Err(self.err("expr.syntax", "digit expected in exponent"));
            }
            self.take_digits(&mut text);
        }
        if !is_double {
            if let Ok(int) = text.parse::<i64>() {
                return Ok(Expr::Int(int));
            }
        }
        match text.parse::<f64>() {
            Ok(double) if double.is_finite() => Ok(Expr::Double(double)),
            _ => Err(self.err("expr.syntax", format!("malformed number '{text}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_and_associativity_parse_as_written() {
        // 2 + 3 * 4 parses as 2 + (3 * 4).
        assert_eq!(
            parse("2 + 3 * 4", "t").expect("parses"),
            Expr::Binary(
                "+".into(),
                Box::new(Expr::Int(2)),
                Box::new(Expr::Binary(
                    "*".into(),
                    Box::new(Expr::Int(3)),
                    Box::new(Expr::Int(4))
                ))
            )
        );
    }

    #[test]
    fn every_published_limit_is_enforced_at_parse_time() {
        let long = format!("{}1", "1 + ".repeat(128));
        assert_eq!(long.chars().count(), 513);
        assert_eq!(
            parse(&long, "t").expect_err("too long").diagnostic.code,
            "expr.too-long"
        );

        let deep = format!("{}1{}", "(".repeat(33), ")".repeat(33));
        assert_eq!(
            parse(&deep, "t").expect_err("too deep").diagnostic.code,
            "expr.too-deep"
        );

        assert_eq!(
            parse(&format!("{}x", "x.".repeat(9)), "t")
                .expect_err("too many segments")
                .diagnostic
                .code,
            "expr.path-too-long"
        );
    }

    #[test]
    fn the_function_list_is_fixed() {
        assert!(parse("nope(1)", "t").is_err());
        assert!(parse("pct(1, 2)", "t").is_ok());
    }
}
