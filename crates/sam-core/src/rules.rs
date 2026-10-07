//! `content/rules.json` as typed values (§3.1, Phase 6): the study behaviour is
//! **data** — the pipeline a type follows, the scheduler that spaces its
//! reviews, and the derived metrics a screen shows. Switching the method or the
//! schedule is a JSON write, never a code change (D11).
//!
//! This module owns the shapes and their defaults. The decode stage
//! ([`crate::decode`]) fills them in, the validator checks them, and
//! [`crate::pipeline`] / [`crate::scheduler`] read them at the moment they act.

use serde::{Deserialize, Serialize};

use crate::resolved_config::ResolvedConfig;

/// The scheduler names the engine implements (§6 Phase 6: `sm2 | fsrs | fixed`).
pub const SCHEDULERS: [&str; 3] = ["fixed", "sm2", "fsrs"];

/// §3.2's default ladder: `[1, 7, 30]` calendar days from the completion anchor.
pub const DEFAULT_FIXED_INTERVALS: [i64; 3] = [1, 7, 30];

/// FSRS's default requested retention; the crate's own default is 0.9.
pub const DEFAULT_DESIRED_RETENTION: f64 = 0.9;

/// Default SM-2 ease factor (SuperMemo's published starting value).
pub const DEFAULT_SM2_EASE: f64 = 2.5;

/// `content/rules.json#/scheduler` — the plan's schedule, with one entry per
/// named scheduler so a switch does not discard the other settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerDef {
    /// One of [`SCHEDULERS`]; the validator rejects anything else.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<FixedSchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm2: Option<Sm2Schedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fsrs: Option<FsrsSchedule>,
}

impl SchedulerDef {
    /// The plan's default when `rules.json` declares no scheduler at all: the
    /// fixed ladder is the default (§3.5, D11).
    pub fn default_fixed() -> Self {
        Self {
            name: "fixed".into(),
            fixed: None,
            sm2: None,
            fsrs: None,
        }
    }

    pub fn intervals(&self) -> Vec<i64> {
        self.fixed
            .as_ref()
            .and_then(|fixed| fixed.intervals.clone())
            .filter(|intervals| !intervals.is_empty())
            .unwrap_or_else(|| DEFAULT_FIXED_INTERVALS.to_vec())
    }

    pub fn desired_retention(&self) -> f64 {
        self.fsrs
            .as_ref()
            .and_then(|fsrs| fsrs.desired_retention)
            .unwrap_or(DEFAULT_DESIRED_RETENTION)
    }

    /// The 21 FSRS weights a plan may pin; `None` = the crate's own default set
    /// (FSRS-6), which is what the recorded algorithm version refers to (§9).
    pub fn fsrs_weights(&self) -> Option<Vec<f32>> {
        self.fsrs.as_ref().and_then(|fsrs| fsrs.weights.clone())
    }
}

impl Default for SchedulerDef {
    fn default() -> Self {
        Self::default_fixed()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixedSchedule {
    /// Explicit day offsets from the completion anchor — not chained gaps
    /// (§3.5). Absent = [`DEFAULT_FIXED_INTERVALS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intervals: Option<Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sm2Schedule {
    /// SM-2's ease floor; defaults to the published 1.3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_ease: Option<f64>,
    /// Start the first interval at this many days instead of SM-2's 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_interval: Option<i64>,
}

impl Sm2Schedule {
    pub fn min_ease(&self) -> f64 {
        self.min_ease.unwrap_or(1.3)
    }

    pub fn first_interval(&self) -> i64 {
        self.first_interval.unwrap_or(1).max(0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FsrsSchedule {
    /// The requested retention the intervals are computed for (default 0.9).
    #[serde(
        rename = "desiredRetention",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub desired_retention: Option<f64>,
    /// Pinned FSRS weights (21 numbers). Absent = the crate's default set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weights: Option<Vec<f32>>,
    /// Whether a hard/good review also moves the `srs` stage machine forward.
    /// Absent = true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance_stages: Option<bool>,
}

impl FsrsSchedule {
    pub fn advance_stages(&self) -> bool {
        self.advance_stages.unwrap_or(true)
    }
}

/// One derived metric: a saved view, an optional per-record expression, and the
/// fold that reduces it — the same vocabulary a `stat` block uses, so the
/// engine computes metrics with the evaluator a view already runs through
/// (§4.4: one evaluator, one set of null rules).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricDef {
    pub label: String,
    /// The saved view whose records the fold runs over; its filter/sort/limit
    /// are the metric's filter/sort/limit.
    pub view: String,
    /// Evaluated once per record; absent = count the records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    /// `sum | avg | min | max | count`; absent = `sum` when `expr` is present,
    /// `count` otherwise (the same rule a `stat` block applies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce: Option<String>,
    /// A unit shown beside the value ("min", "topics").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl MetricDef {
    pub fn reduce_kind(&self) -> &str {
        self.reduce
            .as_deref()
            .unwrap_or(if self.expr.is_none() { "count" } else { "sum" })
    }
}

impl Default for FixedSchedule {
    fn default() -> Self {
        Self { intervals: None }
    }
}

// ── derived metrics (§3.1's "derived metrics", §6 Phase 6) ───────────────────

/// Resolve every declared metric: a saved view's query, an optional per-record
/// expression, and the fold — computed by the **same** evaluator and the same
/// fold a `stat` block runs (§4.4: one evaluator, one set of null rules). A
/// metric whose expression errors reports `null` plus the diagnostic; it never
/// invents a zero.
pub fn metrics_json(
    config: &ResolvedConfig,
    context: &crate::evaluator::EvalContext,
) -> serde_json::Value {
    let mut out: Vec<serde_json::Value> = Vec::new();
    for (id, metric) in config.rules.metrics.as_ref().into_iter().flatten() {
        let mut row = serde_json::Map::new();
        row.insert("id".into(), serde_json::json!(id));
        row.insert("label".into(), serde_json::json!(metric.label));
        row.insert("view".into(), serde_json::json!(metric.view));
        row.insert("reduce".into(), serde_json::json!(metric.reduce_kind()));
        row.insert("unit".into(), serde_json::json!(metric.unit));
        row.insert("expr".into(), serde_json::json!(metric.expr));
        match crate::views::query_records(config, &metric.view, context, None) {
            Ok(outcome) => {
                let (value, of, error) = match &metric.expr {
                    None => (
                        crate::evaluator::L2Value::Int(outcome.records.len() as i64),
                        outcome.records.len(),
                        None,
                    ),
                    Some(text) => {
                        let mut values = Vec::with_capacity(outcome.records.len());
                        let mut error: Option<String> = None;
                        let address = format!("content/rules.json#/metrics/{id}/expr");
                        match crate::expression::parse(text, &address) {
                            Ok(expr) => {
                                let mut evaluation = crate::evaluator::Evaluation::new(
                                    *context,
                                    crate::expression::MAX_EVALUATION_STEPS,
                                );
                                for record in &outcome.records {
                                    match evaluation.evaluate(&expr, Some(record)) {
                                        Ok(value) => values.push(value),
                                        Err(problem) => {
                                            error.get_or_insert_with(|| {
                                                problem.diagnostic.message.clone()
                                            });
                                        }
                                    }
                                }
                            }
                            Err(problem) => {
                                error = Some(problem.diagnostic.message);
                            }
                        }
                        let folded = match crate::views::fold_public(metric.reduce_kind(), &values)
                        {
                            Ok(value) => value,
                            Err(problem) => {
                                error.get_or_insert_with(|| problem.diagnostic.message.clone());
                                crate::evaluator::L2Value::Null
                            }
                        };
                        (folded, values.len(), error)
                    }
                };
                row.insert("value".into(), value.json());
                row.insert("of".into(), serde_json::json!(of));
                if let Some(error) = error {
                    row.insert("error".into(), serde_json::json!(error));
                }
            }
            Err(error) => {
                row.insert("value".into(), serde_json::Value::Null);
                row.insert("of".into(), serde_json::json!(0));
                row.insert("error".into(), serde_json::json!(error.to_string()));
            }
        }
        out.push(serde_json::Value::Object(row));
    }
    serde_json::json!({
        "metrics": out,
        "timezone": config.rules.timezone,
    })
}

// ── §5's design lints and their waivers (Phase 7) ────────────────────────────

/// One rule in §5's vocabulary: what it is called, its default severity, and
/// whether the engine can **observe** it from plan data.
///
/// `checked: false` is a stated fact, not a silent pass: `design.check` reports
/// the rule as unobservable rather than claiming it is satisfied. Three of the
/// rules are CSS-level properties of the app's own layer (`solid-row-rule`,
/// `outlined-chip`, `fake-os-chrome`) and one is a render-time count, so the
/// engine has nothing to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LintRuleDef {
    pub id: &'static str,
    /// `"advisory"` (default, dismissible) or `"error"` (the release gate's
    /// accessibility floor). A taste rule never blocks a save (§5).
    pub severity: &'static str,
    /// Whether [`crate::theme::design_check`] can decide this rule.
    pub checked: bool,
    pub note: &'static str,
}

/// §5's table, in file order. The ids are the vocabulary; a plan may set a
/// severity or waive one, never invent a new rule.
pub const LINT_RULES: [LintRuleDef; 13] = [
    LintRuleDef {
        id: "design.no.streak-guilt",
        severity: "advisory",
        checked: true,
        note: "a label or title counts a streak or says you are behind (§11)",
    },
    LintRuleDef {
        id: "design.no.donut-ring",
        severity: "advisory",
        checked: true,
        note: "a chart or stat is named as a ring or donut (§11)",
    },
    LintRuleDef {
        id: "design.no.outlined-chip",
        severity: "advisory",
        checked: false,
        note: "a chip is a wash, never an outline — the app's own layer, not plan data",
    },
    LintRuleDef {
        id: "design.no.solid-row-rule",
        severity: "advisory",
        checked: false,
        note: "dashed only, inside a card — the app's own layer, not plan data",
    },
    LintRuleDef {
        id: "design.no.chromatic-page-bg",
        severity: "advisory",
        checked: true,
        note: "the resolved page backdrop carries a strong chroma (§11)",
    },
    LintRuleDef {
        id: "design.no.fake-os-chrome",
        severity: "advisory",
        checked: false,
        note: "the window controls are the OS's — CSS, not plan data",
    },
    LintRuleDef {
        id: "design.max.floating-surfaces",
        severity: "advisory",
        checked: false,
        note: "at most one rail and one console at a time — a render-time count",
    },
    LintRuleDef {
        id: "design.contrast.on-wash",
        severity: "error",
        checked: true,
        note: "normal text on a resolved colour pair is ≥ 4.5:1",
    },
    LintRuleDef {
        id: "design.type.scale",
        severity: "advisory",
        checked: true,
        note: "the resolved font-size rungs are monotonic",
    },
    LintRuleDef {
        id: "design.motion.duration",
        severity: "advisory",
        checked: true,
        note: "durations and easings are the declared ones",
    },
    LintRuleDef {
        id: "content.relation.target",
        severity: "error",
        checked: true,
        note: "no dangling relation ids — enforced at load",
    },
    LintRuleDef {
        id: "content.type.views",
        severity: "advisory",
        checked: true,
        note: "a type with no view rots into a junk drawer (§3.6)",
    },
    LintRuleDef {
        id: "content.field.no-label",
        severity: "advisory",
        checked: true,
        note: "every field names itself in plain language — a student reads the label, not the key",
    },
];

/// The rule definition for an id, if the id is in the vocabulary.
pub fn lint_rule(id: &str) -> Option<&'static LintRuleDef> {
    LINT_RULES.iter().find(|rule| rule.id == id)
}

/// `content/rules.json#/lint` — severity overrides and dated waivers.
///
/// §5 is explicit that these are two different operations: a severity change
/// says how loudly the rule reports; a waiver records a decision with a reason.
/// Neither ever blocks a save.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LintFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<std::collections::BTreeMap<String, LintSetting>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waivers: Option<Vec<LintWaiver>>,
}

impl LintFile {
    /// The effective severity: the plan's override when it names one, else the
    /// rule's default. Unknown ids are left to the validator to report.
    pub fn severity(&self, rule: &LintRuleDef) -> String {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get(rule.id))
            .and_then(|setting| setting.severity.clone())
            .unwrap_or_else(|| rule.severity.to_string())
    }

    /// The waiver recorded for a rule, if any.
    pub fn waiver(&self, id: &str) -> Option<&LintWaiver> {
        self.waivers
            .as_ref()
            .and_then(|waivers| waivers.iter().find(|waiver| waiver.id == id))
    }
}

/// One severity override: `{ "severity": "error" }`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LintSetting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

/// One dated waiver: `{ "id": …, "waivedAt": …, "reason": … }`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LintWaiver {
    pub id: String,
    #[serde(rename = "waivedAt", default, skip_serializing_if = "Option::is_none")]
    pub waived_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The severities a plan may set on a rule.
pub const LINT_SEVERITIES: [&str; 2] = ["advisory", "error"];
