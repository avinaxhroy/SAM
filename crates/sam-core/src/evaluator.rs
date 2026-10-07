//! L2 expression evaluation (§4.4 evaluation contract).
//!
//! Values distinguish missing/null from zero and false; implicit string-to-number
//! coercion is rejected; `&&`, `||`, and ternary short-circuit; `pct` returns null
//! when denominator <= 0; division by zero and non-finite results are errors;
//! and aggregates are bounded by a step budget.
//!
//! The clock and timezone come from [`EvalContext`] so tests can freeze both (§4.4).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::diagnostic::{Diagnostic, DiagnosticError};
use crate::json_value::JSONValue;
use crate::model::{FieldDef, Record};
use crate::resolved_config::ResolvedConfig;
use crate::transaction::{civil_from_days, days_from_civil};
use crate::views::PlanState;

use crate::expression::{self, Expr, MAX_EVALUATION_STEPS, MAX_RELATION_HOPS};

/// The evaluation domain — richer than `JSONValue` because paths can land on
/// records and collections (aggregates consume those).
#[derive(Debug, Clone, PartialEq)]
pub enum L2Value {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    Record(Record),
    Records(Vec<Record>),
    Values(Vec<L2Value>),
}

impl L2Value {
    pub fn is_null(&self) -> bool {
        matches!(self, L2Value::Null)
    }

    /// Numeric view for arithmetic; bools and strings are NOT numbers.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            L2Value::Int(int) => Some(*int as f64),
            L2Value::Double(double) => Some(*double),
            _ => None,
        }
    }

    /// Integral results serialize as ints (canonical form keeps 60, not 60.0).
    pub fn from_number(value: f64) -> L2Value {
        if !value.is_finite() {
            return L2Value::Null;
        }
        if value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
            return L2Value::Int(value as i64);
        }
        L2Value::Double(value)
    }

    pub fn json(&self) -> JSONValue {
        match self {
            L2Value::Null => JSONValue::Null,
            L2Value::Bool(flag) => JSONValue::Bool(*flag),
            L2Value::Int(int) => JSONValue::from(*int),
            L2Value::Double(double) => serde_json::Number::from_f64(*double)
                .map(JSONValue::Number)
                .unwrap_or(JSONValue::Null),
            L2Value::String(text) => JSONValue::String(text.clone()),
            L2Value::Record(_) | L2Value::Records(_) | L2Value::Values(_) => JSONValue::Null,
        }
    }

    /// The scalar text of a value, or nothing when it has no scalar form
    /// (group keys and human output).
    pub fn scalar_text(&self) -> Option<String> {
        match self {
            L2Value::Null => None,
            L2Value::Bool(flag) => Some(flag.to_string()),
            L2Value::Int(int) => Some(int.to_string()),
            L2Value::Double(double) => Some(double.to_string()),
            L2Value::String(text) => Some(text.clone()),
            L2Value::Record(_) | L2Value::Records(_) | L2Value::Values(_) => None,
        }
    }
}

/// Everything an evaluation reads: the published config, the read-only
/// progress state, a clock and a timezone offset in minutes (§4.4: freeze both
/// in tests).
#[derive(Clone, Copy)]
pub struct EvalContext<'a> {
    pub config: &'a ResolvedConfig,
    pub state: Option<&'a PlanState>,
    pub now_unix: i64,
    pub tz_offset_minutes: i32,
}

impl<'a> EvalContext<'a> {
    /// The real-clock context a CLI read uses. The plan timezone is resolved
    /// through [`resolve_timezone`].
    pub fn new(config: &'a ResolvedConfig, state: Option<&'a PlanState>) -> Self {
        Self {
            config,
            state,
            now_unix: unix_now(),
            tz_offset_minutes: plan_timezone(config),
        }
    }

    /// A frozen context, for tests: same evaluation, no clock read.
    pub fn frozen(config: &'a ResolvedConfig, state: Option<&'a PlanState>, now_unix: i64) -> Self {
        Self {
            config,
            state,
            now_unix,
            tz_offset_minutes: 0,
        }
    }
}

/// The plan timezone, one location with one fallback: `rules.json#/timezone`
/// is the documented home (§3.5) and the settings row writes it; a plan written
/// while the row lived under `#/study` keeps being read, so an existing file is
/// never silently ignored.
pub fn plan_timezone(config: &ResolvedConfig) -> i32 {
    plan_timezone_of(&config.rules)
}

/// The same resolution from the rules document alone, so a caller that holds no
/// `ResolvedConfig` (a writer, a test) asks the same question.
pub fn plan_timezone_of(rules: &crate::model::RulesFile) -> i32 {
    let fallback = rules
        .study
        .as_ref()
        .and_then(|study| study.get("timezone"))
        .and_then(crate::json_value::JSONValue::as_str);
    resolve_timezone(rules.timezone.as_deref().or(fallback))
}

/// Resolve `rules.timezone` to an offset in minutes.
///
/// **Ceiling, deliberately named:** §9 permits no date crate before Phase 6, so
/// this understands `UTC`, `Z` and numeric offsets (`+05:30`, `-0800`, `+05`).
/// A full IANA zone name (`Asia/Kolkata`) resolves to UTC here; the plan
/// timezone is otherwise a fixed offset, and a tz database arrives with the
/// scheduler if a real need is measured. `smith: named-zone resolution, add a
/// tz source in Phase 6 if off-by-a-day reports appear`.
pub fn resolve_timezone(timezone: Option<&str>) -> i32 {
    let Some(name) = timezone else { return 0 };
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("utc")
        || trimmed.eq_ignore_ascii_case("gmt")
        || trimmed.eq_ignore_ascii_case("z")
    {
        return 0;
    }
    let body = trimmed
        .strip_prefix("UTC")
        .or_else(|| trimmed.strip_prefix("utc"))
        .or_else(|| trimmed.strip_prefix("GMT"))
        .or_else(|| trimmed.strip_prefix("gmt"))
        .unwrap_or(trimmed);
    let Some(sign) = body.chars().next().filter(|c| *c == '+' || *c == '-') else {
        return 0; // an IANA name: UTC until a tz source exists
    };
    let digits: String = body[1..].chars().filter(char::is_ascii_digit).collect();
    let minutes = match digits.len() {
        2 => digits.parse::<i32>().ok().map(|hours| hours * 60),
        4 => {
            let hours = digits[..2].parse::<i32>().ok();
            let mins = digits[2..].parse::<i32>().ok();
            hours.zip(mins).map(|(h, m)| h * 60 + m)
        }
        _ => None,
    };
    let Some(minutes) = minutes else { return 0 };
    if sign == '-' { -minutes } else { minutes }
}

/// A bounded evaluator: one context, one step budget.
pub struct Evaluation<'a> {
    pub context: EvalContext<'a>,
    steps: usize,
    budget: usize,
}

impl<'a> Evaluation<'a> {
    pub fn new(context: EvalContext<'a>, budget: usize) -> Self {
        Self {
            context,
            steps: 0,
            budget,
        }
    }

    fn step(&mut self) -> Result<(), DiagnosticError> {
        self.steps += 1;
        if self.steps > self.budget {
            return Err(DiagnosticError::new(Diagnostic::error(
                "expr.limit",
                "expression",
                format!(
                    "evaluation exceeded {} steps — the expression or its aggregate is too costly",
                    self.budget
                ),
            )));
        }
        Ok(())
    }

    fn runtime(&self, message: impl Into<String>) -> DiagnosticError {
        DiagnosticError::new(Diagnostic::error("expr.runtime", "expression", message))
    }

    /// Evaluate `expr` against one record (the `complete()` context). Returns a
    /// diagnostic on runtime errors — callers surface it and treat the value as
    /// null (§4.4).
    pub fn evaluate(
        &mut self,
        expr: &Expr,
        record: Option<&Record>,
    ) -> Result<L2Value, DiagnosticError> {
        self.step()?;
        match expr {
            Expr::Int(int) => Ok(L2Value::Int(*int)),
            Expr::Double(double) => Ok(L2Value::Double(*double)),
            Expr::String(text) => Ok(L2Value::String(text.clone())),
            Expr::Bool(flag) => Ok(L2Value::Bool(*flag)),
            Expr::Null => Ok(L2Value::Null),
            Expr::Path(segments) => self.resolve_path(segments, record),
            Expr::Call(name, args) => self.call(name, args, record),
            Expr::Unary(op, inner) => self.unary(op, inner, record),
            Expr::Binary(op, left, right) => self.binary(op, left, right, record),
            Expr::Ternary(condition, then_branch, else_branch) => {
                let value = self.evaluate(condition, record)?;
                if value.is_null() {
                    return Ok(L2Value::Null);
                }
                let L2Value::Bool(flag) = value else {
                    return Err(self.runtime("ternary condition must be a boolean"));
                };
                self.evaluate(if flag { then_branch } else { else_branch }, record)
            }
        }
    }

    // ── paths (§3.1: course.short resolves links.course then the target's
    // fields.short; many-valued relations aggregate explicitly) ──────────────

    fn resolve_path(
        &mut self,
        segments: &[String],
        record: Option<&Record>,
    ) -> Result<L2Value, DiagnosticError> {
        let mut current = record
            .map(|record| L2Value::Record(record.clone()))
            .unwrap_or(L2Value::Null);
        let mut hops = 0usize;
        for segment in segments {
            self.step()?;
            match current {
                L2Value::Record(record) => {
                    current = self.resolve_segment(segment, &record, &mut hops)?;
                }
                L2Value::Records(records) => {
                    // Map the next segment over the collection. Relations,
                    // parents and child types continue as a FLAT collection;
                    // scalar fields terminate into a value collection.
                    if records.is_empty() {
                        current = L2Value::Values(Vec::new());
                    } else if segment == "id" {
                        current = L2Value::Values(
                            records
                                .iter()
                                .map(|record| L2Value::String(record.id.clone()))
                                .collect(),
                        );
                    } else {
                        let first_type = records[0].type_.clone();
                        let config = self.context.config;
                        let Some(def) = config.types.get(&first_type) else {
                            return Err(self.runtime(format!("unknown type {first_type}")));
                        };
                        let relationish = def.field(segment).is_some_and(FieldDef::is_relation)
                            || def.parent.as_deref() == Some(segment.as_str());
                        if relationish || self.child_targets(segment, &first_type) {
                            let mut next: Vec<Record> = Vec::new();
                            for record in &records {
                                self.step()?;
                                match self.resolve_segment(segment, record, &mut hops)? {
                                    L2Value::Record(one) => next.push(one),
                                    L2Value::Records(many) => next.extend(many),
                                    _ => {}
                                }
                            }
                            current = L2Value::Records(next);
                        } else if def.field(segment).is_some() {
                            let mut mapped = Vec::new();
                            for record in &records {
                                self.step()?;
                                mapped.push(self.resolve_segment(segment, record, &mut hops)?);
                            }
                            current = L2Value::Values(mapped);
                        } else {
                            return Err(self.runtime(format!(
                                "\"{segment}\" is not a field of type {first_type}"
                            )));
                        }
                    }
                }
                _ => {
                    return Err(self.runtime(format!(
                        "path segment \"{segment}\" continues from a non-record value"
                    )));
                }
            }
        }
        Ok(current)
    }

    /// True when `segment` names a type whose relations target `of_type`
    /// (reverse collection), used for child-type path segments.
    fn child_targets(&self, segment: &str, of_type: &str) -> bool {
        let Some(child) = self.context.config.types.get(segment) else {
            return false;
        };
        if child.parent.as_deref() == Some(of_type) {
            return true;
        }
        child
            .fields
            .iter()
            .any(|field| field.is_relation() && field.to.as_deref() == Some(of_type))
    }

    fn resolve_segment(
        &mut self,
        segment: &str,
        record: &Record,
        hops: &mut usize,
    ) -> Result<L2Value, DiagnosticError> {
        if segment == "id" {
            return Ok(L2Value::String(record.id.clone())); // reserved (§3.1)
        }
        let config = self.context.config;
        let Some(def) = config.types.get(&record.type_) else {
            return Err(self.runtime(format!("unknown type {}", record.type_)));
        };
        if let Some(field) = def.field(segment) {
            if field.is_relation() {
                let ids = record.links.get(segment).cloned().unwrap_or_default();
                self.count_hop(hops)?;
                let mut targets = Vec::with_capacity(ids.len());
                for id in &ids {
                    let Some(target) = config.record(id) else {
                        return Err(self
                            .runtime(format!("relation \"{segment}\" → \"{id}\" has no target")));
                    };
                    targets.push(target.clone());
                }
                return Ok(if field.is_one() {
                    targets
                        .into_iter()
                        .next()
                        .map(L2Value::Record)
                        .unwrap_or(L2Value::Null)
                } else {
                    L2Value::Records(targets)
                });
            }
            if field.is_formula() {
                if let Some(text) = &field.expr {
                    // Load-time cache (§4.4): the AST parses once per plan, not
                    // once per record per access. Miss = hand-built config; the
                    // parse then either succeeds or reports like validation did.
                    let expr = match self.context.config.formula(text) {
                        Some(cached) => cached,
                        None => &expression::parse(text, "content/types.json")?,
                    };
                    return self.evaluate(expr, Some(record)); // acyclic — validated
                }
            }
            if field.type_ == "progress" {
                // §3.5: a `progress` field is derived from the pipeline, never
                // persisted. `progress: "pct"` names the field carrying the
                // percentage (fraction = pct/100); a counter pipeline counts
                // recorded stages; any other stage machine reports the fraction
                // of stages recorded. Null when the type is not trackable or the
                // named field is absent — "no value" is not zero.
                return Ok(self.progress_value(record));
            }
            return match record.fields.get(segment).unwrap_or(&JSONValue::Null) {
                JSONValue::Null => Ok(L2Value::Null),
                JSONValue::Bool(flag) => Ok(L2Value::Bool(*flag)),
                JSONValue::Number(number) => Ok(match number.as_i64() {
                    Some(int) => L2Value::Int(int),
                    None => number
                        .as_f64()
                        .map(L2Value::Double)
                        .unwrap_or(L2Value::Null),
                }),
                JSONValue::String(text) => Ok(L2Value::String(text.clone())),
                JSONValue::Array(_) | JSONValue::Object(_) => Err(self.runtime(format!(
                    "field \"{segment}\" holds a json value — not usable in an expression"
                ))),
            };
        }
        if def.parent.as_deref() == Some(segment) {
            self.count_hop(hops)?;
            return Ok(record
                .links
                .get(segment)
                .and_then(|ids| ids.first())
                .and_then(|id| config.record(id))
                .cloned()
                .map(L2Value::Record)
                .unwrap_or(L2Value::Null));
        }
        if self.child_targets(segment, &record.type_) {
            self.count_hop(hops)?;
            let children: Vec<Record> = config
                .records_iter()
                .filter(|candidate| {
                    candidate.type_ == segment
                        && candidate.links.values().any(|ids| ids.contains(&record.id))
                })
                .cloned()
                .collect();
            return Ok(L2Value::Records(children));
        }
        Err(self.runtime(format!(
            "\"{segment}\" is not a field, relation or child type of {}",
            record.type_
        )))
    }

    fn count_hop(&self, hops: &mut usize) -> Result<(), DiagnosticError> {
        *hops += 1;
        if *hops > MAX_RELATION_HOPS {
            return Err(DiagnosticError::new(Diagnostic::error(
                "expr.limit",
                "expression",
                format!("path traverses more than {MAX_RELATION_HOPS} relations"),
            )));
        }
        Ok(())
    }

    // ── operators ────────────────────────────────────────────────────────────

    fn unary(
        &mut self,
        op: &str,
        expr: &Expr,
        record: Option<&Record>,
    ) -> Result<L2Value, DiagnosticError> {
        let value = self.evaluate(expr, record)?;
        if value.is_null() {
            return Ok(L2Value::Null);
        }
        match op {
            "!" => match value {
                L2Value::Bool(flag) => Ok(L2Value::Bool(!flag)),
                _ => Err(self.runtime("'!' needs a boolean")),
            },
            "-" => match value.as_number() {
                Some(number) => Ok(L2Value::from_number(-number)),
                None => Err(self.runtime("unary '-' needs a number")),
            },
            _ => Err(self.runtime(format!("unknown operator {op}"))),
        }
    }

    fn binary(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        record: Option<&Record>,
    ) -> Result<L2Value, DiagnosticError> {
        // Short-circuit first (§4.4): the right side is not evaluated when the
        // left already decides.
        if op == "&&" {
            let lhs = self.evaluate(left, record)?;
            if lhs == L2Value::Bool(false) {
                return Ok(L2Value::Bool(false));
            }
            let rhs = self.evaluate(right, record)?;
            return kleene("&&", &lhs, &rhs);
        }
        if op == "||" {
            let lhs = self.evaluate(left, record)?;
            if lhs == L2Value::Bool(true) {
                return Ok(L2Value::Bool(true));
            }
            let rhs = self.evaluate(right, record)?;
            return kleene("||", &lhs, &rhs);
        }
        let lhs = self.evaluate(left, record)?;
        let rhs = self.evaluate(right, record)?;
        match op {
            "==" => Ok(L2Value::Bool(equal(&lhs, &rhs))),
            "!=" => Ok(L2Value::Bool(!equal(&lhs, &rhs))),
            "<" | "<=" | ">" | ">=" => {
                if lhs.is_null() || rhs.is_null() {
                    return Ok(L2Value::Null);
                }
                if let (Some(a), Some(b)) = (lhs.as_number(), rhs.as_number()) {
                    return Ok(L2Value::Bool(compare_f64(op, a, b)));
                }
                // Strings compare lexicographically (dates are ISO strings).
                match (&lhs, &rhs) {
                    (L2Value::String(a), L2Value::String(b)) => {
                        Ok(L2Value::Bool(compare_str(op, a, b)))
                    }
                    _ => Err(self.runtime(format!(
                        "cannot compare {} with {} — no implicit coercion",
                        describe(&lhs),
                        describe(&rhs)
                    ))),
                }
            }
            "+" | "-" | "*" | "/" | "%" => {
                if lhs.is_null() || rhs.is_null() {
                    return Ok(L2Value::Null);
                }
                let (Some(a), Some(b)) = (lhs.as_number(), rhs.as_number()) else {
                    return Err(self.runtime(format!(
                        "'{op}' needs numbers, found {} and {} — no implicit coercion",
                        describe(&lhs),
                        describe(&rhs)
                    )));
                };
                match op {
                    "+" => Ok(L2Value::from_number(a + b)),
                    "-" => Ok(L2Value::from_number(a - b)),
                    "*" => Ok(L2Value::from_number(a * b)),
                    "/" => {
                        if b == 0.0 {
                            return Err(DiagnosticError::new(Diagnostic::error(
                                "expr.divide-by-zero",
                                "expression",
                                "division by zero",
                            )));
                        }
                        Ok(L2Value::from_number(a / b))
                    }
                    "%" => {
                        if b == 0.0 {
                            return Err(DiagnosticError::new(Diagnostic::error(
                                "expr.divide-by-zero",
                                "expression",
                                "modulo by zero",
                            )));
                        }
                        Ok(L2Value::from_number(a % b))
                    }
                    _ => unreachable!("matched above"),
                }
            }
            _ => Err(self.runtime(format!("unknown operator {op}"))),
        }
    }

    // ── functions (the fixed list — §4.4) ────────────────────────────────────

    fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        record: Option<&Record>,
    ) -> Result<L2Value, DiagnosticError> {
        self.step()?;
        match name {
            "today" => Ok(L2Value::String(civil_date(
                self.context.now_unix,
                self.context.tz_offset_minutes,
            ))),
            "currentTerm" => Ok(self
                .current_term_id()
                .map(L2Value::String)
                .unwrap_or(L2Value::Null)),
            "complete" => {
                let Some(record) = record else {
                    return Err(self.runtime("complete() needs a record context"));
                };
                Ok(L2Value::Bool(self.is_complete(record)))
            }
            "pct" => {
                if args.len() != 2 {
                    return Err(self.runtime("pct(a, b) takes two numbers"));
                }
                let a = self.evaluate(&args[0], record)?;
                let b = self.evaluate(&args[1], record)?;
                if a.is_null() || b.is_null() {
                    return Ok(L2Value::Null);
                }
                let (Some(a), Some(b)) = (a.as_number(), b.as_number()) else {
                    return Err(self.runtime("pct needs numbers"));
                };
                if b <= 0.0 {
                    return Ok(L2Value::Null); // §4.4: null when b <= 0
                }
                Ok(L2Value::from_number(a / b * 100.0))
            }
            "clamp" => {
                if args.len() != 3 {
                    return Err(self.runtime("clamp(x, lo, hi) takes three numbers"));
                }
                let parts = self.evaluate_all(args, record)?;
                if parts.iter().any(L2Value::is_null) {
                    return Ok(L2Value::Null);
                }
                let mut numbers = Vec::with_capacity(parts.len());
                for part in &parts {
                    match part.as_number() {
                        Some(number) => numbers.push(number),
                        None => return Err(self.runtime("clamp needs numbers")),
                    }
                }
                Ok(L2Value::from_number(
                    numbers[0].max(numbers[1]).min(numbers[2]),
                ))
            }
            "round" => {
                if args.len() != 1 {
                    return Err(self.runtime("round(x) takes one number"));
                }
                let value = self.evaluate(&args[0], record)?;
                if value.is_null() {
                    return Ok(L2Value::Null);
                }
                match value.as_number() {
                    Some(number) => Ok(L2Value::from_number(number.round())),
                    None => Err(self.runtime("round needs a number")),
                }
            }
            "daysBetween" => {
                if args.len() != 2 {
                    return Err(self.runtime("daysBetween(a, b) takes two dates"));
                }
                let a = self.evaluate(&args[0], record)?;
                let b = self.evaluate(&args[1], record)?;
                if a.is_null() || b.is_null() {
                    return Ok(L2Value::Null);
                }
                let (L2Value::String(a), L2Value::String(b)) = (&a, &b) else {
                    return Err(self.runtime("daysBetween needs YYYY-MM-DD strings"));
                };
                let (Some(a), Some(b)) = (parse_date(a), parse_date(b)) else {
                    return Err(self.runtime("daysBetween got a malformed date"));
                };
                let difference = days_from_civil(a.0, a.1, a.2) - days_from_civil(b.0, b.1, b.2);
                Ok(L2Value::from_number(difference as f64))
            }
            "weekOf" => {
                if args.len() != 1 {
                    return Err(self.runtime("weekOf(date) takes one date"));
                }
                let value = self.evaluate(&args[0], record)?;
                if value.is_null() {
                    return Ok(L2Value::Null);
                }
                let L2Value::String(text) = &value else {
                    return Err(self.runtime("weekOf needs a YYYY-MM-DD string"));
                };
                let Some(date) = parse_date(text) else {
                    return Err(self.runtime("weekOf got a malformed date"));
                };
                Ok(self
                    .covering_week_index(days_from_civil(date.0, date.1, date.2))
                    .map(L2Value::Int)
                    .unwrap_or(L2Value::Null))
            }
            "sum" | "count" | "avg" | "min" | "max" => {
                if args.len() != 1 {
                    return Err(self.runtime(format!("{name}(path) takes one path")));
                }
                let value = self.evaluate(&args[0], record)?;
                let numbers: Vec<f64> = match value {
                    L2Value::Records(records) => {
                        if name != "count" {
                            return Err(self.runtime(format!(
                                "{name} needs a field path — records have no numeric value"
                            )));
                        }
                        return Ok(L2Value::Int(records.len() as i64));
                    }
                    L2Value::Values(values) => {
                        let mut numbers = Vec::with_capacity(values.len());
                        for value in &values {
                            self.step()?;
                            if value.is_null() {
                                continue; // aggregates skip nulls (SQL-like)
                            }
                            match value.as_number() {
                                Some(number) => numbers.push(number),
                                None => {
                                    return Err(self.runtime(format!(
                                        "{name} found {} — numbers only",
                                        describe(value)
                                    )));
                                }
                            }
                        }
                        numbers
                    }
                    other => {
                        if other.as_number().is_some() && name == "count" {
                            return Ok(L2Value::Int(1));
                        }
                        return Err(self.runtime(format!("{name} needs a collection path")));
                    }
                };
                Ok(match name {
                    "count" => L2Value::Int(numbers.len() as i64), // 0 when empty
                    "sum" => sum_or_null(&numbers),
                    "avg" => {
                        if numbers.is_empty() {
                            L2Value::Null
                        } else {
                            L2Value::from_number(numbers.iter().sum::<f64>() / numbers.len() as f64)
                        }
                    }
                    "min" => numbers
                        .iter()
                        .copied()
                        .reduce(f64::min)
                        .map(L2Value::from_number)
                        .unwrap_or(L2Value::Null),
                    "max" => numbers
                        .iter()
                        .copied()
                        .reduce(f64::max)
                        .map(L2Value::from_number)
                        .unwrap_or(L2Value::Null),
                    _ => unreachable!("matched above"),
                })
            }
            _ => Err(self.runtime(format!("unknown function {name}"))),
        }
    }

    fn evaluate_all(
        &mut self,
        args: &[Expr],
        record: Option<&Record>,
    ) -> Result<Vec<L2Value>, DiagnosticError> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.evaluate(arg, record)?);
        }
        Ok(values)
    }

    // ── calendar helpers (clock + timezone come from the context) ────────────

    /// The covering `week` record's index, else null (§3.2 seed contract).
    /// Visible to `views::today_json` and `today::today_view_json`: the frame's
    /// "week n of N" must be the same answer `weekOf()` gives a saved view, so
    /// they call this, not a copy.
    pub(crate) fn covering_week_index(&self, day: i64) -> Option<i64> {
        self.covering_week(day).map(|(_, index)| index)
    }

    /// The covering `week` record's **id** and index, else null. The id is what
    /// a write needs — a session logged on this day is attributed to this week
    /// (F2), and the id is the plan's own key for it, never a guess from a label.
    pub(crate) fn covering_week(&self, day: i64) -> Option<(String, i64)> {
        let mut best: Option<(i64, String)> = None;
        for week in self
            .context
            .config
            .records_iter()
            .filter(|record| record.type_ == "week")
        {
            let (Some(start), Some(end), Some(index)) = (
                week.fields.get("start").and_then(JSONValue::as_str),
                week.fields.get("end").and_then(JSONValue::as_str),
                week.fields.get("index").and_then(JSONValue::as_i64),
            ) else {
                continue;
            };
            let (Some(start), Some(end)) = (parse_date(start), parse_date(end)) else {
                continue;
            };
            let start = days_from_civil(start.0, start.1, start.2);
            let end = days_from_civil(end.0, end.1, end.2);
            if day >= start && day <= end {
                let better = best.as_ref().is_none_or(|(best_index, best_id)| {
                    index < *best_index || (index == *best_index && week.id < *best_id)
                });
                if better {
                    best = Some((index, week.id.clone()));
                }
            }
        }
        best.map(|(index, id)| (id, index))
    }

    /// Id of the term with the greatest start ≤ today; ties by the greater id.
    /// Visible to `views::today_json`: the frame's denominator comes from the
    /// same term `currentTerm()` names, so the two cannot disagree.
    pub(crate) fn current_term_id(&self) -> Option<String> {
        let today = civil_date(self.context.now_unix, self.context.tz_offset_minutes);
        let mut best: Option<(String, String)> = None;
        for term in self
            .context
            .config
            .records_iter()
            .filter(|record| record.type_ == "term")
        {
            let Some(start) = term.fields.get("start").and_then(JSONValue::as_str) else {
                continue;
            };
            if start > today.as_str() {
                continue;
            }
            let candidate = (start.to_string(), term.id.clone());
            if best.as_ref().is_none_or(|current| &candidate > current) {
                best = Some(candidate);
            }
        }
        best.map(|(_, id)| id)
    }

    fn is_complete(&self, record: &Record) -> bool {
        let config = self.context.config;
        let Some(def) = config.types.get(&record.type_) else {
            return false;
        };
        let Some(pipeline_name) = def.pipeline.as_deref() else {
            return false;
        };
        let Some(pipeline) = config
            .rules
            .pipelines
            .as_ref()
            .and_then(|pipelines| pipelines.get(pipeline_name))
        else {
            return false;
        };
        let target = pipeline
            .complete_when
            .clone()
            .or_else(|| pipeline.stages.last().cloned());
        let Some(target) = target else {
            return false;
        };
        self.context
            .state
            .and_then(|state| state.progress.get(&record.id))
            .is_some_and(|entry| entry.recorded(&target))
    }

    /// §3.5's derived `progress` value: `pct`-carrying pipelines divide the
    /// named field by 100; a counter pipeline counts recorded stages against the
    /// declared stages; anything else reports the fraction recorded. A fraction
    /// in `0..=1`, or `null` when there is nothing to derive.
    fn progress_value(&mut self, record: &Record) -> L2Value {
        let config = self.context.config;
        let Some(def) = config.types.get(&record.type_) else {
            return L2Value::Null;
        };
        let Some(pipeline_name) = def.pipeline.as_deref() else {
            return L2Value::Null;
        };
        let Some(pipeline) = config
            .rules
            .pipelines
            .as_ref()
            .and_then(|pipelines| pipelines.get(pipeline_name))
        else {
            return L2Value::Null;
        };
        if let Some(key) = pipeline.progress.as_deref() {
            // The named field is resolved the way every field is — a `formula`
            // is evaluated, a stored number is read — because `progress: "pct"`
            // names a computed column in every plan that ships one. A field that
            // is itself `progress` cannot be a source: that is the one cycle
            // this derivation could have.
            if def
                .field(key)
                .is_some_and(|field| field.type_ == "progress")
            {
                return L2Value::Null;
            }
            let expr = Expr::Path(vec![key.to_string()]);
            let value = match self.evaluate(&expr, Some(record)) {
                Ok(value) => value,
                Err(_) => return L2Value::Null,
            };
            return value
                .as_number()
                .map(|pct| L2Value::from_number((pct / 100.0).clamp(0.0, 1.0)))
                .unwrap_or(L2Value::Null);
        }
        if pipeline.stages.is_empty() {
            return L2Value::Null;
        }
        let recorded = self
            .context
            .state
            .and_then(|state| state.progress.get(&record.id))
            .map(|entry| {
                pipeline
                    .stages
                    .iter()
                    .filter(|stage| entry.recorded(stage))
                    .count()
            })
            .unwrap_or(0);
        L2Value::from_number(recorded as f64 / pipeline.stages.len() as f64)
    }
}

/// Evaluate one formula expression against one record, no plan state. `None`
/// when the expression is malformed or evaluation errors.
pub fn formula_scalar(config: &ResolvedConfig, record: &Record, expr: &str) -> Option<L2Value> {
    let owned;
    let parsed = match config.formula(expr) {
        Some(cached) => cached,
        None => {
            owned = expression::parse(expr, "formula").ok()?;
            &owned
        }
    };
    let context = EvalContext::new(config, None);
    let mut evaluation = Evaluation::new(context, MAX_EVALUATION_STEPS);
    evaluation.evaluate(parsed, Some(record)).ok()
}

/// Sum of a number list, `null` when empty — shared with Phase 5's block folds,
/// so a `stat` over a view and a `sum(path)` in an expression cannot disagree.
pub(crate) fn sum_or_null(numbers: &[f64]) -> L2Value {
    if numbers.is_empty() {
        L2Value::Null
    } else {
        L2Value::from_number(numbers.iter().sum())
    }
}

/// Kleene logic (§4.4): null is unknown — false dominates `&&`, true dominates
/// `||`, otherwise null propagates.
fn kleene(op: &str, lhs: &L2Value, rhs: &L2Value) -> Result<L2Value, DiagnosticError> {
    let short = if op == "&&" {
        L2Value::Bool(false)
    } else {
        L2Value::Bool(true)
    };
    if *lhs == short || *rhs == short {
        return Ok(short);
    }
    if lhs.is_null() || rhs.is_null() {
        return Ok(L2Value::Null);
    }
    match (lhs, rhs) {
        (L2Value::Bool(a), L2Value::Bool(b)) => {
            Ok(L2Value::Bool(if op == "&&" { *a && *b } else { *a || *b }))
        }
        _ => Err(DiagnosticError::new(Diagnostic::error(
            "expr.runtime",
            "expression",
            format!(
                "'{op}' needs booleans, found {}",
                describe(if lhs.is_null() { rhs } else { lhs })
            ),
        ))),
    }
}

fn equal(lhs: &L2Value, rhs: &L2Value) -> bool {
    match (lhs, rhs) {
        (L2Value::Null, L2Value::Null) => true,
        (L2Value::Null, _) | (_, L2Value::Null) => false,
        (L2Value::Bool(a), L2Value::Bool(b)) => a == b,
        (L2Value::String(a), L2Value::String(b)) => a == b,
        (L2Value::Record(a), L2Value::Record(b)) => a.id == b.id,
        (L2Value::Records(a), L2Value::Records(b)) => a
            .iter()
            .map(|record| &record.id)
            .eq(b.iter().map(|record| &record.id)),
        _ => match (lhs.as_number(), rhs.as_number()) {
            (Some(a), Some(b)) => a == b,
            _ => false, // different kinds are simply not equal
        },
    }
}

fn compare_f64(op: &str, a: f64, b: f64) -> bool {
    match op {
        "<" => a < b,
        "<=" => a <= b,
        ">" => a > b,
        _ => a >= b,
    }
}

fn compare_str(op: &str, a: &str, b: &str) -> bool {
    match op {
        "<" => a < b,
        "<=" => a <= b,
        ">" => a > b,
        _ => a >= b,
    }
}

fn describe(value: &L2Value) -> &'static str {
    match value {
        L2Value::Null => "null",
        L2Value::Bool(_) => "a boolean",
        L2Value::Int(_) | L2Value::Double(_) => "a number",
        L2Value::String(_) => "a string",
        L2Value::Record(_) => "a record",
        L2Value::Records(_) => "a record collection",
        L2Value::Values(_) => "a value collection",
    }
}

/// Strict `YYYY-MM-DD`, ranges checked (§3.5: a civil date, not an instant).
/// Shared with Phase 5's chart buckets, which fill calendar days.
pub(crate) fn parse_date(text: &str) -> Option<(i64, u32, u32)> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let (year, month, day) = (
        parts[0].parse::<i64>().ok()?,
        parts[1].parse::<u32>().ok()?,
        parts[2].parse::<u32>().ok()?,
    );
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// The civil date of an instant in a fixed offset, `YYYY-MM-DD`.
pub fn civil_date(unix_seconds: i64, offset_minutes: i32) -> String {
    let shifted = unix_seconds + i64::from(offset_minutes) * 60;
    let (year, month, day) = civil_from_days(shifted.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::parse;

    #[test]
    fn numbers_keep_their_canonical_integral_form() {
        assert_eq!(L2Value::from_number(60.0), L2Value::Int(60));
        assert_eq!(L2Value::from_number(2.5), L2Value::Double(2.5));
        assert_eq!(L2Value::from_number(f64::INFINITY), L2Value::Null);
    }

    #[test]
    fn a_fixed_offset_resolves_and_an_iana_name_falls_back_to_utc() {
        assert_eq!(resolve_timezone(Some("UTC")), 0);
        assert_eq!(resolve_timezone(Some("+05:30")), 330);
        assert_eq!(resolve_timezone(Some("-0800")), -480);
        assert_eq!(resolve_timezone(Some("UTC-03:00")), -180);
        assert_eq!(resolve_timezone(Some("Asia/Kolkata")), 0);
        assert_eq!(resolve_timezone(None), 0);
    }

    /// The plan timezone's documented home is `rules.json#/timezone`; the
    /// spelling the settings row used before it moved is still read, so a file
    /// written by an earlier build is never silently ignored.
    #[test]
    fn the_plan_timezone_reads_the_documented_key_and_the_moved_one() {
        let mut rules = crate::model::RulesFile::default();
        rules.timezone = Some("-05:00".into());
        assert_eq!(plan_timezone_of(&rules), -300);
        rules.timezone = None;
        rules.study = Some(serde_json::json!({ "timezone": "UTC+05:30" }));
        assert_eq!(plan_timezone_of(&rules), 330);
        rules.study = Some(serde_json::json!({}));
        assert_eq!(plan_timezone_of(&rules), 0);
    }

    #[test]
    fn a_civil_date_uses_the_offset() {
        // 2026-01-19T23:30:00Z is already 2026-01-20 in +05:30.
        assert_eq!(civil_date(1_768_865_400, 0), "2026-01-19");
        assert_eq!(civil_date(1_768_865_400, 330), "2026-01-20");
    }

    #[test]
    fn date_parsing_checks_the_range() {
        assert_eq!(parse_date("2026-01-20"), Some((2026, 1, 20)));
        assert!(parse_date("2026-13-20").is_none());
        assert!(parse_date("2026-01").is_none());
    }

    #[test]
    fn a_lone_arithmetic_expression_needs_no_config() {
        // The evaluator's smallest smoke test: parse and evaluate with a real
        // (empty) context so the step counter runs.
        let parsed = parse("1 + 2 * 3", "t").expect("parses");
        let mut evaluation = Evaluation::new(
            EvalContext {
                config: &EMPTY,
                state: None,
                now_unix: 1_768_924_800,
                tz_offset_minutes: 0,
            },
            MAX_EVALUATION_STEPS,
        );
        assert_eq!(
            evaluation.evaluate(&parsed, None).expect("evaluates"),
            L2Value::Int(7)
        );
    }

    static EMPTY: std::sync::LazyLock<ResolvedConfig> =
        std::sync::LazyLock::new(|| ResolvedConfig {
            plan_root: std::path::PathBuf::new(),
            layer: "explicit".into(),
            types: Default::default(),
            views: Default::default(),
            rules: crate::model::RulesFile {
                schema_version: Some(1),
                pipelines: None,
                scheduler: None,
                metrics: None,
                lint: None,
                study: None,
                timezone: None,
            },
            shell: None,
            appearance: None,
            positioned_records: Vec::new(),
            formulas: std::collections::BTreeMap::new(),
            source_files: Default::default(),
            tokens: crate::resources::TokenRegister {
                version: "test".into(),
                tokens: Default::default(),
            },
            revision: "test".into(),
            load_warnings: Vec::new(),
        });
}
