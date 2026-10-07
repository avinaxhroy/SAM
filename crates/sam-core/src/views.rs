//! Saved-view resolution (§3.6, §4.7): combines a query definition with layout metadata.
//!
//! Filter, sort, and group keys are L2 expressions evaluated over the in-memory plan snapshot.
//! `SAM view` serves as the primary view projection, tagged with the active source revision.
//!
//! Output is deterministic: canonical record serialization, explicit null ordering,
//! and stable ID tie-breakers on every sort (§4.4).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::diagnostic::Diagnostic;
use crate::evaluator::{EvalContext, Evaluation, L2Value};
use crate::expression::{self, Expr, MAX_EVALUATION_STEPS};
use crate::json_value::JSONValue;
use crate::model::{BLOCK_KINDS, BlockDef, MAX_BLOCK_DEPTH, Record, ViewDef};
use crate::resolved_config::ResolvedConfig;
use crate::strict_json;

/// Progress state from `state/state.json` — the read projection `complete()`
/// and the views need. The write model is [`crate::pipeline::ProgressEntry`];
/// this reads it through the same codec, so a panel and a transition cannot
/// disagree about what a stage map is.
#[derive(Debug, Clone, Default)]
pub struct PlanState {
    pub progress: BTreeMap<String, crate::pipeline::ProgressEntry>,
}

pub use crate::pipeline::ProgressEntry;

impl PlanState {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Read `state/state.json`; anything missing or malformed yields an empty
    /// state, because a plan without progress is a valid plan (§4.6 keeps the
    /// last-known-good snapshot for a malformed file elsewhere).
    pub fn load(root: &Path) -> Self {
        let Ok(bytes) = std::fs::read(root.join("state/state.json")) else {
            return Self::empty();
        };
        let Ok(text) = String::from_utf8(bytes) else {
            return Self::empty();
        };
        let Ok(value) = strict_json::parse(&text, "state/state.json") else {
            return Self::empty();
        };
        let Some(progress) = value.get("progress").and_then(JSONValue::as_object) else {
            return Self::empty();
        };
        let mut out = BTreeMap::new();
        for (id, raw) in progress {
            out.insert(id.clone(), ProgressEntry::from_json(raw));
        }
        Self { progress: out }
    }

    pub fn entry(&self, id: &str) -> Option<&ProgressEntry> {
        self.progress.get(id)
    }
}

#[derive(Debug)]
pub enum ViewError {
    Unknown(String),
    Expression(crate::diagnostic::DiagnosticError),
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewError::Unknown(name) => write!(f, "no saved view named \"{name}\""),
            ViewError::Expression(error) => write!(f, "{}", error.diagnostic.message),
        }
    }
}

impl std::error::Error for ViewError {}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub key: Option<String>,
    pub records: Vec<Record>,
}

/// One saved view's own query, resolved (§3.6).
///
/// A screen's record blocks each resolve one of these, through the same filter,
/// sort, group and limit code — so a block drawn inside a screen cannot disagree
/// with the same view run from the terminal (one evaluator, one null order, one
/// stable id tie-breaker).
pub struct QueryOutcome {
    /// Absent for a **composed screen** (COMPOSER §2.1): it queries no type, so
    /// it returns no records and its projection is empty by construction.
    pub type_: Option<String>,
    pub layout: Option<String>,
    pub columns: Vec<String>,
    /// Flat, post filter/sort/limit.
    pub records: Vec<Record>,
    /// Present iff the view groups.
    pub groups: Option<Vec<Group>>,
    pub candidates: usize,
    pub warnings: Vec<Diagnostic>,
}

impl QueryOutcome {
    /// The query as data. `config` comes in because a record's **derived**
    /// values (§3.1: formula and progress are computed, never persisted) belong
    /// in the projection: a screen that shows a table has to show the computed
    /// column, and the alternative — the UI evaluating the expression itself —
    /// would be a second evaluator (§4.4: one parser, one budget, one place
    /// errors come from).
    pub fn json(&self, config: &ResolvedConfig, view_name: &str) -> serde_json::Value {
        let mut data = serde_json::Map::new();
        data.insert("view".into(), serde_json::json!(view_name));
        data.insert("type".into(), serde_json::json!(self.type_));
        data.insert("layout".into(), serde_json::json!(self.layout));
        data.insert("columns".into(), serde_json::json!(self.columns));
        data.insert("candidates".into(), serde_json::json!(self.candidates));
        data.insert("returned".into(), serde_json::json!(self.records.len()));
        data.insert(
            "records".into(),
            serde_json::Value::Array(
                self.records
                    .iter()
                    .map(|record| record_json(config, record))
                    .collect(),
            ),
        );
        data.insert(
            "warnings".into(),
            serde_json::Value::Array(
                self.warnings
                    .iter()
                    .map(|warning| serde_json::to_value(warning).unwrap_or(serde_json::Value::Null))
                    .collect(),
            ),
        );
        if let Some(groups) = &self.groups {
            data.insert("grouped".into(), serde_json::json!(true));
            data.insert(
                "groups".into(),
                serde_json::Value::Array(
                    groups
                        .iter()
                        .map(|group| {
                            serde_json::json!({
                                "key": group.key,
                                "records": group
                                    .records
                                    .iter()
                                    .map(|record| record_json(config, record))
                                    .collect::<Vec<_>>(),
                            })
                        })
                        .collect(),
                ),
            );
        }
        serde_json::Value::Object(data)
    }

    /// The same projection, as a **record block node**: the query's own fields
    /// plus the block's `kind`. A block node is byte-compatible with the
    /// pre-Phase-5 `view` projection, so the record table renders either one.
    fn as_record_node(&self, config: &ResolvedConfig, view_name: &str) -> serde_json::Value {
        let mut node = self.json(config, view_name);
        if let serde_json::Value::Object(object) = &mut node {
            // A composed screen has no layout to draw itself in; the node omits
            // `kind` rather than providing `kind: null` for the renderer to
            // disambiguate.
            if let Some(layout) = &self.layout {
                object.insert("kind".into(), serde_json::json!(layout));
            }
        }
        node
    }
}

/// One resolved screen: the view's own query, plus its blocks.
pub struct ViewOutcome {
    pub view: String,
    /// Absent for a composed screen.
    pub type_: Option<String>,
    pub layout: Option<String>,
    pub columns: Vec<String>,
    /// Flat, post filter/sort/limit.
    pub records: Vec<Record>,
    /// Present iff the view groups.
    pub groups: Option<Vec<Group>>,
    pub candidates: usize,
    pub warnings: Vec<Diagnostic>,
    /// The screen, resolved (Phase 5). Never empty: a view that declares no
    /// blocks draws itself as one record block, so the renderer has one path.
    pub blocks: Vec<serde_json::Value>,
    /// Phase 6's panel for this destination, when it declares one.
    pub panel: Option<String>,
    /// The student's own composition (COMPOSER §2.1), when the view declares
    /// `components`. `Some(vec![])` is a composed-but-empty screen — the empty
    /// state — and is distinct from `None` (a designed view).
    pub components: Option<Vec<crate::model::ComponentDef>>,
}

impl ViewOutcome {
    /// The view as data. `config` comes in because a record's **derived** values
    /// (§3.1: formula and progress are computed, never persisted) belong in the
    /// projection: a screen that shows a table has to show the computed column,
    /// and the alternative — the UI evaluating the expression itself — would be a
    /// second evaluator (§4.4: one parser, one budget, one place errors come from).
    pub fn json(&self, config: &ResolvedConfig) -> serde_json::Value {
        let mut data = serde_json::Map::new();
        data.insert("view".into(), serde_json::json!(self.view));
        data.insert("type".into(), serde_json::json!(self.type_));
        data.insert("layout".into(), serde_json::json!(self.layout));
        data.insert("columns".into(), serde_json::json!(self.columns));
        data.insert("candidates".into(), serde_json::json!(self.candidates));
        data.insert("returned".into(), serde_json::json!(self.records.len()));
        data.insert(
            "records".into(),
            serde_json::Value::Array(
                self.records
                    .iter()
                    .map(|record| record_json(config, record))
                    .collect(),
            ),
        );
        data.insert(
            "warnings".into(),
            serde_json::Value::Array(
                self.warnings
                    .iter()
                    .map(|warning| serde_json::to_value(warning).unwrap_or(serde_json::Value::Null))
                    .collect(),
            ),
        );
        if let Some(groups) = &self.groups {
            data.insert("grouped".into(), serde_json::json!(true));
            data.insert(
                "groups".into(),
                serde_json::Value::Array(
                    groups
                        .iter()
                        .map(|group| {
                            serde_json::json!({
                                "key": group.key,
                                "records": group
                                    .records
                                    .iter()
                                    .map(|record| record_json(config, record))
                                    .collect::<Vec<_>>(),
                            })
                        })
                        .collect(),
                ),
            );
        }
        data.insert(
            "blocks".into(),
            serde_json::Value::Array(self.blocks.clone()),
        );
        if let Some(panel) = &self.panel {
            data.insert("panel".into(), serde_json::json!(panel));
        }
        // `components` is present **only when the view declares it**: the UI's
        // "is this screen the student's?" test is `components !== undefined`, so
        // an absent composition must not be spelled as a null one (COMPOSER
        // §3.2). An empty list is written as itself — the empty state.
        if let Some(components) = &self.components {
            data.insert(
                "components".into(),
                serde_json::Value::Array(
                    components
                        .iter()
                        // The engine's own members, exactly as the file states
                        // them: a half-row `span` is here, the defaulted whole
                        // row is not.
                        .map(|component| {
                            serde_json::to_value(component).unwrap_or(serde_json::Value::Null)
                        })
                        .collect(),
                ),
            );
        }
        serde_json::Value::Object(data)
    }
}

pub fn record_json(config: &ResolvedConfig, record: &Record) -> serde_json::Value {
    let mut value = serde_json::to_value(record).unwrap_or(serde_json::Value::Null);
    if let serde_json::Value::Object(object) = &mut value {
        object.insert("derived".into(), derived_json(config, record));
    }
    value
}

/// Every `formula` field of a record's type, evaluated (§3.4). A field whose
/// expression errors renders as `null` — never as a persisted replacement
/// (§4.4: an evaluation error is a rendering, not a value).
fn derived_json(config: &ResolvedConfig, record: &Record) -> serde_json::Value {
    let mut derived = serde_json::Map::new();
    let Some(def) = config.types.get(&record.type_) else {
        return serde_json::Value::Object(derived);
    };
    for field in &def.fields {
        if field.type_ != "formula" {
            continue;
        }
        let value = field
            .expr
            .as_deref()
            .and_then(|expr| crate::evaluator::formula_scalar(config, record, expr))
            .map(|value| value.json())
            .unwrap_or(serde_json::Value::Null);
        derived.insert(field.key.clone(), value);
    }
    serde_json::Value::Object(derived)
}

/// The frame's date facts (§1 R2/R3 — `UI_SPEC.md`): the titlebar's slot reads
/// "week n of N" and the rail's date disc reads the day pair, both from here
/// because the plan owns the timezone and the calendar — a UI clock would
/// answer a different question. The week pair comes from the evaluator's own
/// helpers, so the frame and a saved view can never disagree about the week.
pub fn today_json(config: &ResolvedConfig, context: &EvalContext) -> serde_json::Value {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    // 1970-01-01 was a Thursday, so day zero sits at index 4 of a Sunday-first
    // week — the one constant that makes the weekday a lookup, not a calendar.
    const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    let date = crate::evaluator::civil_date(context.now_unix, context.tz_offset_minutes);
    // `civil_date` only ever prints a well-formed date, so the fallback is
    // unreachable; it keeps the frame drawing rather than panicking on a clock
    // the range check rejects.
    let (year, month, day) = crate::evaluator::parse_date(&date).unwrap_or((1970, 1, 1));
    let days = crate::transaction::days_from_civil(year, month, day);
    let weekday = WEEKDAYS[(days + 4).rem_euclid(7) as usize];

    let evaluation = Evaluation::new(*context, MAX_EVALUATION_STEPS);
    let week = evaluation
        .covering_week_index(days)
        .map(|index| serde_json::json!({ "index": index, "of": week_count(config, &evaluation) }));

    serde_json::json!({
        "date": date,
        "day": day,
        "month": MONTHS[month.clamp(1, 12) as usize - 1],
        "weekday": weekday,
        "week": week.unwrap_or(serde_json::Value::Null),
    })
}

/// The denominator of "week n of N": the current term's declared `weeks` when
/// it has one, else the number of week records. When neither is present,
/// the frame states the index alone. Shared with `today.view`.
pub(crate) fn week_count(config: &ResolvedConfig, evaluation: &Evaluation) -> serde_json::Value {
    if let Some(term_id) = evaluation.current_term_id()
        && let Some(weeks) = config
            .record(&term_id)
            .and_then(|term| term.fields.get("weeks"))
            .and_then(JSONValue::as_i64)
    {
        return serde_json::json!(weeks);
    }
    match config
        .records_iter()
        .filter(|record| record.type_ == "week")
        .count()
    {
        0 => serde_json::Value::Null,
        count => serde_json::json!(count),
    }
}

/// Resolve one saved view: its own query first, then its blocks (§3.6, Phase 5).
/// `context` supplies the clock, timezone and state; `config` supplies the view,
/// the types and the records.
pub fn resolve(
    config: &ResolvedConfig,
    name: &str,
    context: &EvalContext,
) -> Result<ViewOutcome, ViewError> {
    resolve_with(config, name, context, None)
}

/// The same resolution with an optional derived index: a view whose filter
/// translates to SQL narrows its candidate set through the index and then runs
/// the **same** L2 filter over the candidates, so the result is identical to the
/// unindexed path by construction (§6 Phase 6's index parity, §4.4's single
/// evaluator).
pub fn resolve_with(
    config: &ResolvedConfig,
    name: &str,
    context: &EvalContext,
    index: Option<&crate::index::Index>,
) -> Result<ViewOutcome, ViewError> {
    let Some(view) = config.views.get(name) else {
        return Err(ViewError::Unknown(name.to_string()));
    };
    let mut own = query(config, name, view, context, index)?;
    let blocks = match &view.blocks {
        // No blocks: the view draws itself as one record block in its layout,
        // which is exactly what a pre-Phase-5 view did.
        None => vec![own.as_record_node(config, name)],
        Some(blocks) => {
            let screen = Screen {
                config,
                context,
                index,
                owner: name.to_string(),
            };
            screen.blocks(blocks, name, view, None, 0)
        }
    };
    Ok(ViewOutcome {
        view: name.to_string(),
        type_: own.type_.clone(),
        layout: own.layout.clone(),
        columns: own.columns.clone(),
        records: std::mem::take(&mut own.records),
        groups: own.groups.clone(),
        candidates: own.candidates,
        warnings: own.warnings,
        blocks,
        panel: view.panel.clone(),
        components: view.components.clone(),
    })
}

/// Resolve one view's **query** without its blocks (§3.6) — what a derived
/// metric folds over ([`crate::rules::metrics_json`]) and what a parity check
/// compares. Same function a block runs, so a metric and a screen cannot
/// disagree about a view's records.
pub fn query_records(
    config: &ResolvedConfig,
    name: &str,
    context: &EvalContext,
    index: Option<&crate::index::Index>,
) -> Result<QueryOutcome, ViewError> {
    let Some(view) = config.views.get(name) else {
        return Err(ViewError::Unknown(name.to_string()));
    };
    query(config, name, view, context, index)
}

/// Resolve one view's query — filter, sort, limit, group, columns (§3.6). The
/// whole of the pre-Phase-5 `resolve`, unchanged, so a block and a terminal
/// `SAM view` run the same code over the same records.
fn query(
    config: &ResolvedConfig,
    name: &str,
    view: &crate::model::ViewDef,
    context: &EvalContext,
    index: Option<&crate::index::Index>,
) -> Result<QueryOutcome, ViewError> {
    let view_path = format!("content/views.json#/views/{name}");
    // A composed screen names no type, so it queries nothing: no candidate
    // records, no index narrowing, and an empty projection (COMPOSER §2.1).
    let mut candidates: Vec<&Record> = view
        .type_
        .as_deref()
        .map(|type_name| {
            config
                .records_iter()
                .filter(|record| record.type_ == type_name)
                .collect()
        })
        .unwrap_or_default();
    let candidate_count = candidates.len();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    // ---- the index narrows, it never decides. The translated predicate is
    // implied by the L2 filter, so the candidate set is a superset of the true
    // matches and the filter below still runs over it (§4.6: "Query plans and
    // evaluator/index result parity are test gates").
    if let (Some(index), Some(filter_text), Some(type_name)) =
        (index, view.filter.as_deref(), view.type_.as_deref())
        && let Ok(expr) = expression::parse(filter_text, &format!("{view_path}/filter"))
        && let Some(filter) = crate::index::translate_filter(config, type_name, &expr)
        && let Ok(ids) = index.candidates(type_name, &filter)
    {
        let wanted: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
        candidates.retain(|record| wanted.contains(record.id.as_str()));
    }

    // ---- filter: only records that evaluate to true pass (null/false and
    // runtime errors exclude, errors also warn — §4.4).
    let included: Vec<&Record> = if let Some(filter_text) = &view.filter {
        let expr = expression::parse(filter_text, &format!("{view_path}/filter"))
            .map_err(ViewError::Expression)?;
        let mut included = Vec::new();
        for record in &candidates {
            match evaluate(context, &expr, Some(*record)) {
                Ok(L2Value::Bool(true)) => included.push(*record),
                Ok(_) => {}
                Err(error) => warnings.push(record_warning(
                    &error,
                    &format!("record \"{}\": {}", record.id, error.diagnostic.message),
                )),
            }
        }
        included
    } else {
        candidates
    };

    // ---- sort: comma-separated keys, optional '-' prefix for descending.
    struct SortKey {
        expr: Expr,
        descending: bool,
    }
    let mut sort_keys: Vec<SortKey> = Vec::new();
    if let Some(sort_text) = &view.sort {
        for part in sort_text.split(',') {
            let text = part.trim();
            if text.is_empty() {
                continue;
            }
            let descending = text.starts_with('-');
            let body = if descending { &text[1..] } else { text };
            let expr = expression::parse(body, &format!("{view_path}/sort"))
                .map_err(ViewError::Expression)?;
            sort_keys.push(SortKey { expr, descending });
        }
    }

    let mut keyed: Vec<(&Record, Vec<L2Value>)> = included
        .iter()
        .map(|record| (*record, Vec::new()))
        .collect();
    for key in &sort_keys {
        for (record, keys) in keyed.iter_mut() {
            match evaluate(context, &key.expr, Some(&*record)) {
                Ok(value) => keys.push(value),
                Err(error) => {
                    warnings.push(record_warning(
                        &error,
                        &format!(
                            "record \"{}\" sort key: {}",
                            record.id, error.diagnostic.message
                        ),
                    ));
                    keys.push(L2Value::Null);
                }
            }
        }
    }
    let nulls_first = view.nulls.as_deref() == Some("first");
    keyed.sort_by(|a, b| {
        for (index, key) in sort_keys.iter().enumerate() {
            if let Some(order) = compare_keys(&a.1[index], &b.1[index], nulls_first) {
                return if key.descending {
                    order.reverse()
                } else {
                    order
                };
            }
        }
        a.0.id.cmp(&b.0.id) // stable id tie-breaker (§4.4)
    });
    // Limit before materialising: only the returned window is cloned, not
    // every candidate that matched.
    if let Some(limit) = view.limit {
        if limit >= 0 {
            keyed.truncate(limit as usize);
        }
    }
    let ordered: Vec<Record> = keyed
        .into_iter()
        .map(|(record, _)| record.clone())
        .collect();

    // ---- group: an expression; groups ordered by key with the same null
    // order, records keep the sorted order inside groups.
    let mut groups: Option<Vec<Group>> = None;
    if let Some(group_text) = &view.group {
        let expr = expression::parse(group_text, &format!("{view_path}/group"))
            .map_err(ViewError::Expression)?;
        let mut buckets: Vec<(L2Value, Vec<Record>)> = Vec::new();
        for record in &ordered {
            let value = match evaluate(context, &expr, Some(record)) {
                Ok(value) => value,
                Err(error) => {
                    warnings.push(record_warning(
                        &error,
                        &format!(
                            "record \"{}\" group key: {}",
                            record.id, error.diagnostic.message
                        ),
                    ));
                    L2Value::Null
                }
            };
            match buckets.iter().position(|(key, _)| same_key(key, &value)) {
                Some(index) => buckets[index].1.push(record.clone()),
                None => buckets.push((value, vec![record.clone()])),
            }
        }
        buckets.sort_by(|a, b| compare_keys(&a.0, &b.0, nulls_first).unwrap_or(Ordering::Equal));
        groups = Some(
            buckets
                .into_iter()
                .map(|(key, records)| Group {
                    key: key.scalar_text(),
                    records,
                })
                .collect(),
        );
    }

    // ---- columns: declared, or every field except derived progress
    let columns = view.columns.clone().unwrap_or_else(|| {
        view.type_
            .as_deref()
            .and_then(|type_name| config.types.get(type_name))
            .map(|def| {
                def.fields
                    .iter()
                    .filter(|field| field.type_ != "progress")
                    .map(|field| field.key.clone())
                    .collect()
            })
            .unwrap_or_default()
    });

    Ok(QueryOutcome {
        type_: view.type_.clone(),
        layout: view.layout.clone(),
        columns,
        records: ordered,
        groups,
        candidates: candidate_count,
        warnings,
    })
}

// ── the screen: blocks (§3.6, D8) ───────────────────────────────────────────

/// The most instances one `repeat` block draws. D8 permits a located boundary
/// where a measurement asks for one, and a screen built from an unbounded record
/// set is exactly that case: the cut is reported on the node, never silent.
const MAX_BLOCK_INSTANCES: usize = 60;

/// One screen being resolved. Walks the block tree with the enclosing view as
/// the default source and, inside a `repeat`, the instance's record as the
/// expression context.
struct Screen<'a> {
    config: &'a ResolvedConfig,
    context: &'a EvalContext<'a>,
    /// The optional derived index a sub-view's candidate set may narrow through.
    index: Option<&'a crate::index::Index>,
    /// The view the blocks live in — the tail of every diagnostic path and the
    /// default source for a block that names no view.
    owner: String,
}

impl Screen<'_> {
    fn path(&self, index: usize, key: &str) -> String {
        format!(
            "content/views.json#/views/{}/blocks/{index}/{}",
            self.owner, key
        )
    }

    fn blocks(
        &self,
        blocks: &[BlockDef],
        owner_name: &str,
        owner: &ViewDef,
        record: Option<&Record>,
        depth: usize,
    ) -> Vec<serde_json::Value> {
        blocks
            .iter()
            .enumerate()
            .map(|(index, block)| self.node(block, index, owner_name, owner, record, depth))
            .collect()
    }

    /// One block. The two forward-compatibility cases — a kind this build does
    /// not know and a tree past [`MAX_BLOCK_DEPTH`] — come back as a stated
    /// placeholder, which is the whole of "never a blank screen" (D8).
    fn node(
        &self,
        block: &BlockDef,
        index: usize,
        owner_name: &str,
        owner: &ViewDef,
        record: Option<&Record>,
        depth: usize,
    ) -> serde_json::Value {
        if !BLOCK_KINDS.contains(&block.kind.as_str()) {
            return placeholder(
                &block.kind,
                "unknown-kind",
                format!(
                    "unknown block: \"{}\" · known: {}",
                    block.kind,
                    BLOCK_KINDS.join(" · ")
                ),
            );
        }
        if depth >= MAX_BLOCK_DEPTH {
            return placeholder(
                &block.kind,
                "depth",
                format!(
                    "blocks nest deeper than {MAX_BLOCK_DEPTH} — not drawn · known: {}",
                    BLOCK_KINDS.join(" · ")
                ),
            );
        }
        if block.is_record() {
            return self.record_node(block, owner_name, owner);
        }
        match block.kind.as_str() {
            "stat" => self.stat_node(block, index, owner_name, owner),
            "chart" => self.chart_node(block, index, owner_name, owner),
            "callout" => serde_json::json!({
                "kind": "callout",
                "title": block.title,
                "text": block.text,
                "warnings": [],
            }),
            "columns" => serde_json::json!({
                "kind": "columns",
                "title": block.title,
                "blocks": self.blocks(
                    block.blocks.as_deref().unwrap_or_default(),
                    owner_name,
                    owner,
                    record,
                    depth + 1,
                ),
                "warnings": [],
            }),
            "conditional" => {
                let (matched, warnings) = match &block.when {
                    Some(text) => match expression::parse(text, &self.path(index, "when")) {
                        Ok(expr) => match self.value_of(&expr, record) {
                            Ok(value) => (matches!(value, L2Value::Bool(true)), Vec::new()),
                            Err(error) => (false, vec![self.warning(index, "when", &error)]),
                        },
                        Err(error) => (false, vec![error.diagnostic]),
                    },
                    None => (false, Vec::new()),
                };
                let branch = if matched {
                    block.blocks.as_deref().unwrap_or_default()
                } else {
                    block.else_blocks.as_deref().unwrap_or_default()
                };
                serde_json::json!({
                    "kind": "conditional",
                    "title": block.title,
                    "when": block.when,
                    "matched": matched,
                    "blocks": self.blocks(branch, owner_name, owner, record, depth + 1),
                    "warnings": warnings,
                })
            }
            "repeat" => self.repeat_node(block, index, owner_name, owner, depth),
            // `BLOCK_KINDS` is closed and every member is handled above; a kind
            // added to the vocabulary without a renderer lands here, which is
            // why the placeholder is the default rather than a panic.
            other => placeholder(other, "unknown-kind", format!("no renderer for {other}")),
        }
    }

    fn record_node(
        &self,
        block: &BlockDef,
        owner_name: &str,
        owner: &ViewDef,
    ) -> serde_json::Value {
        match self.source(block, owner_name, owner) {
            Ok((name, outcome)) => {
                let mut node = outcome.as_record_node(self.config, &name);
                if let serde_json::Value::Object(object) = &mut node {
                    // The **block** decides how the records are drawn, not the
                    // view it reads: a `board` block over a `list` view is a
                    // board, and `view.setLayout --block N` is the control that
                    // changes it. `view` stays the view whose query and columns
                    // are in play, so a column write lands on the right one.
                    object.insert("kind".into(), serde_json::json!(block.kind));
                    object.insert("layout".into(), serde_json::json!(block.kind));
                    if let Some(title) = &block.title {
                        object.insert("title".into(), serde_json::json!(title));
                    }
                }
                node
            }
            Err(node) => node,
        }
    }

    fn stat_node(
        &self,
        block: &BlockDef,
        index: usize,
        owner_name: &str,
        owner: &ViewDef,
    ) -> serde_json::Value {
        let Some((name, outcome)) = self.source_quiet(block, owner_name, owner) else {
            return placeholder(
                &block.kind,
                "unknown-view",
                format!(
                    "block names undeclared view \"{}\"",
                    block.view.as_deref().unwrap_or("")
                ),
            );
        };
        let reduce = block.reduce_kind();
        let mut warnings = outcome.warnings.clone();
        let (value, of) = match &block.expr {
            None => (
                L2Value::Int(outcome.records.len() as i64),
                outcome.records.len(),
            ),
            Some(text) => {
                let path = self.path(index, "expr");
                match expression::parse(text, &path) {
                    Ok(expr) => {
                        let mut values = Vec::with_capacity(outcome.records.len());
                        for record in &outcome.records {
                            match self.value_of(&expr, Some(record)) {
                                Ok(value) => values.push(value),
                                Err(error) => warnings.push(self.warning(index, "expr", &error)),
                            }
                        }
                        match fold(reduce, &values) {
                            Ok(value) => (value, values.len()),
                            Err(error) => {
                                warnings.push(self.warning(index, "reduce", &error));
                                (L2Value::Null, values.len())
                            }
                        }
                    }
                    Err(error) => {
                        warnings.push(error.diagnostic);
                        (L2Value::Null, 0)
                    }
                }
            }
        };
        serde_json::json!({
            "kind": "stat",
            "title": block.title,
            "label": block.label,
            "view": name,
            "expr": block.expr,
            "reduce": reduce,
            "value": value.json(),
            "of": of,
            "warnings": warnings
                .iter()
                .map(|warning| serde_json::to_value(warning).unwrap_or(serde_json::Value::Null))
                .collect::<Vec<_>>(),
        })
    }

    fn chart_node(
        &self,
        block: &BlockDef,
        index: usize,
        owner_name: &str,
        owner: &ViewDef,
    ) -> serde_json::Value {
        let Some((name, outcome)) = self.source_quiet(block, owner_name, owner) else {
            return placeholder(
                &block.kind,
                "unknown-view",
                format!(
                    "block names undeclared view \"{}\"",
                    block.view.as_deref().unwrap_or("")
                ),
            );
        };
        let mut warnings = outcome.warnings.clone();
        let reduce = block.reduce_kind();
        let mut buckets: Vec<(String, Vec<L2Value>)> = Vec::new();
        let x = block
            .x
            .as_deref()
            .map(|text| expression::parse(text, &self.path(index, "x")));
        let y = block
            .y
            .as_deref()
            .map(|text| expression::parse(text, &self.path(index, "y")));
        if let (Some(Ok(x)), Some(Ok(y))) = (&x, &y) {
            for record in &outcome.records {
                let key = match self.value_of(x, Some(record)) {
                    Ok(value) => value.scalar_text().unwrap_or_else(|| "(none)".into()),
                    Err(error) => {
                        warnings.push(self.warning(index, "x", &error));
                        continue;
                    }
                };
                let value = match self.value_of(y, Some(record)) {
                    Ok(value) => value,
                    Err(error) => {
                        warnings.push(self.warning(index, "y", &error));
                        continue;
                    }
                };
                match buckets.iter_mut().find(|(existing, _)| existing == &key) {
                    Some((_, values)) => values.push(value),
                    None => buckets.push((key, vec![value])),
                }
            }
        }
        if let Some(Err(error)) = &x {
            warnings.push(error.diagnostic.clone());
        }
        if let Some(Err(error)) = &y {
            warnings.push(error.diagnostic.clone());
        }
        // `days` fills the last N calendar days, empty ones as zero, so a strip
        // shows the days nothing was logged — the design's own rule: a day with
        // no work is a fact to report, not a gap to hide.
        if let Some(days) = block.days.filter(|days| *days > 0) {
            let today =
                crate::evaluator::civil_date(self.context.now_unix, self.context.tz_offset_minutes);
            if let Some(start) = crate::evaluator::parse_date(&today)
                .map(|(year, month, day)| crate::transaction::days_from_civil(year, month, day))
            {
                // A descending range is empty in Rust, so the walk is expressed
                // forward from today and sorted afterwards.
                for offset in 0..days {
                    let (year, month, day) = crate::transaction::civil_from_days(start - offset);
                    let key = format!("{year:04}-{month:02}-{day:02}");
                    if !buckets.iter().any(|(existing, _)| existing == &key) {
                        buckets.push((key, Vec::new()));
                    }
                }
            }
        }
        buckets.sort_by(|a, b| a.0.cmp(&b.0));
        let mut rows = Vec::with_capacity(buckets.len());
        for (key, values) in &buckets {
            let value = if values.is_empty() {
                L2Value::Int(0)
            } else {
                match fold(reduce, values) {
                    Ok(value) => value,
                    Err(error) => {
                        warnings.push(self.warning(index, "reduce", &error));
                        L2Value::Null
                    }
                }
            };
            rows.push(serde_json::json!({ "key": key, "value": value.json() }));
        }
        serde_json::json!({
            "kind": "chart",
            "title": block.title,
            "label": block.label,
            "view": name,
            "x": block.x,
            "y": block.y,
            "reduce": reduce,
            "days": block.days,
            "buckets": rows,
            "warnings": warnings
                .iter()
                .map(|warning| serde_json::to_value(warning).unwrap_or(serde_json::Value::Null))
                .collect::<Vec<_>>(),
        })
    }

    fn repeat_node(
        &self,
        block: &BlockDef,
        index: usize,
        owner_name: &str,
        owner: &ViewDef,
        depth: usize,
    ) -> serde_json::Value {
        let Some((name, outcome)) = self.source_quiet(block, owner_name, owner) else {
            return placeholder(
                &block.kind,
                "unknown-view",
                format!(
                    "block names undeclared view \"{}\"",
                    block.view.as_deref().unwrap_or("")
                ),
            );
        };
        let mut warnings = outcome.warnings.clone();
        let wanted = block
            .limit
            .map(|limit| limit as usize)
            .unwrap_or(usize::MAX);
        let taken = wanted.min(MAX_BLOCK_INSTANCES);
        if outcome.records.len() > taken {
            warnings.push(Diagnostic::warning(
                "view.block-instances",
                self.path(index, "limit"),
                format!(
                    "drew the first {taken} of {} — a repeat block stops at {MAX_BLOCK_INSTANCES}",
                    outcome.records.len()
                ),
            ));
        }
        let instances: Vec<serde_json::Value> = outcome
            .records
            .iter()
            .take(taken)
            .map(|record| {
                serde_json::json!({
                    "record": record_json(self.config, record),
                    "blocks": self.blocks(
                        block.blocks.as_deref().unwrap_or_default(),
                        owner_name,
                        owner,
                        Some(record),
                        depth + 1,
                    ),
                })
            })
            .collect();
        serde_json::json!({
            "kind": "repeat",
            "title": block.title,
            "view": name,
            "instances": instances,
            "warnings": warnings
                .iter()
                .map(|warning| serde_json::to_value(warning).unwrap_or(serde_json::Value::Null))
                .collect::<Vec<_>>(),
        })
    }

    /// The query a block draws or folds, with its name. A block that names no
    /// view reads the view it lives in (§3.6: the source is a saved view, and a
    /// screen is composed of them).
    fn source(
        &self,
        block: &BlockDef,
        owner_name: &str,
        owner: &ViewDef,
    ) -> Result<(String, QueryOutcome), serde_json::Value> {
        self.source_quiet(block, owner_name, owner).ok_or_else(|| {
            placeholder(
                &block.kind,
                "unknown-view",
                format!(
                    "block names undeclared view \"{}\"",
                    block.view.as_deref().unwrap_or("")
                ),
            )
        })
    }

    fn source_quiet(
        &self,
        block: &BlockDef,
        owner_name: &str,
        owner: &ViewDef,
    ) -> Option<(String, QueryOutcome)> {
        let (name, view) = match &block.view {
            None => (owner_name.to_string(), owner),
            Some(name) => {
                let view = self.config.views.get(name)?;
                (name.clone(), view)
            }
        };
        query(self.config, &name, view, self.context, self.index)
            .ok()
            .map(|outcome| (name, outcome))
    }

    fn value_of(
        &self,
        expr: &Expr,
        record: Option<&Record>,
    ) -> Result<L2Value, crate::diagnostic::DiagnosticError> {
        let mut evaluation = Evaluation::new(*self.context, MAX_EVALUATION_STEPS);
        evaluation.evaluate(expr, record)
    }

    fn warning(
        &self,
        index: usize,
        key: &str,
        error: &crate::diagnostic::DiagnosticError,
    ) -> Diagnostic {
        Diagnostic::warning(
            error.diagnostic.code.clone(),
            self.path(index, key),
            error.diagnostic.message.clone(),
        )
    }
}

/// The stated fallback (D8): one shape for every reason a block cannot be drawn,
/// so a screen never renders blank and the student can see what to fix.
fn placeholder(unknown: &str, reason: &str, message: String) -> serde_json::Value {
    serde_json::json!({
        "kind": "placeholder",
        "unknown": unknown,
        "reason": reason,
        "message": message,
        "known": BLOCK_KINDS,
        "warnings": [],
    })
}

/// The fold a `stat`/`chart` applies to its per-record values — shared with
/// derived metrics ([`crate::rules::metrics_json`]), so one reduction rule
/// serves every surface (§4.4).
pub(crate) fn fold_public(
    name: &str,
    values: &[L2Value],
) -> Result<L2Value, crate::diagnostic::DiagnosticError> {
    fold(name, values)
}

/// The fold a `stat`/`chart` applies to its per-record values.
///
/// `sum`, `avg`, `min` and `max` follow §4.4's aggregate semantics exactly —
/// nulls are skipped, and a non-number is a runtime diagnostic rather than a
/// silent zero. `count` counts the instances whose value is neither null nor
/// false, which is the question a count on a screen asks ("how many are open");
/// it is deliberately not `sum` over booleans, and it is why an expression-less
/// count block needs no expression at all.
fn fold(name: &str, values: &[L2Value]) -> Result<L2Value, crate::diagnostic::DiagnosticError> {
    if name == "count" {
        return Ok(L2Value::Int(
            values
                .iter()
                .filter(|value| !value.is_null() && !matches!(value, L2Value::Bool(false)))
                .count() as i64,
        ));
    }
    let mut numbers = Vec::with_capacity(values.len());
    for value in values {
        if value.is_null() {
            continue;
        }
        match value.as_number() {
            Some(number) => numbers.push(number),
            None => {
                return Err(crate::diagnostic::DiagnosticError::new(Diagnostic::error(
                    "expr.runtime",
                    "expression",
                    format!("{name} found a value that is not a number"),
                )));
            }
        }
    }
    Ok(match name {
        "sum" => crate::evaluator::sum_or_null(&numbers),
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
        _ => L2Value::Null,
    })
}

fn evaluate(
    context: &EvalContext,
    expr: &Expr,
    record: Option<&Record>,
) -> Result<L2Value, crate::diagnostic::DiagnosticError> {
    let mut evaluation = Evaluation::new(*context, MAX_EVALUATION_STEPS);
    evaluation.evaluate(expr, record)
}

fn record_warning(error: &crate::diagnostic::DiagnosticError, message: &str) -> Diagnostic {
    Diagnostic::warning(
        error.diagnostic.code.clone(),
        error.diagnostic.path.clone(),
        message,
    )
}

/// Key comparison: declared null placement, then type rank (numbers, strings,
/// booleans), then value. `None` = incomparable/equal → the id tie-breaker.
fn compare_keys(a: &L2Value, b: &L2Value, nulls_first: bool) -> Option<Ordering> {
    match (a, b) {
        (L2Value::Null, L2Value::Null) => None,
        (L2Value::Null, _) => Some(if nulls_first {
            Ordering::Less
        } else {
            Ordering::Greater
        }),
        (_, L2Value::Null) => Some(if nulls_first {
            Ordering::Greater
        } else {
            Ordering::Less
        }),
        (L2Value::Int(x), L2Value::Int(y)) => (x != y).then(|| x.cmp(y)),
        (L2Value::Int(x), L2Value::Double(y)) => cmp_f64(*x as f64, *y),
        (L2Value::Double(x), L2Value::Int(y)) => cmp_f64(*x, *y as f64),
        (L2Value::Double(x), L2Value::Double(y)) => cmp_f64(*x, *y),
        (L2Value::String(x), L2Value::String(y)) => (x != y).then(|| x.cmp(y)),
        (L2Value::Bool(x), L2Value::Bool(y)) => (x != y).then(|| x.cmp(y)),
        _ => {
            // Mixed kinds: deterministic type rank, never an error mid-sort.
            let (ra, rb) = (rank(a), rank(b));
            (ra != rb).then(|| ra.cmp(&rb))
        }
    }
}

fn cmp_f64(a: f64, b: f64) -> Option<Ordering> {
    (a != b).then(|| a.partial_cmp(&b).unwrap_or(Ordering::Equal))
}

fn rank(value: &L2Value) -> u8 {
    match value {
        L2Value::Int(_) | L2Value::Double(_) => 0,
        L2Value::String(_) => 1,
        L2Value::Bool(_) => 2,
        _ => 3,
    }
}

fn same_key(a: &L2Value, b: &L2Value) -> bool {
    match (a, b) {
        (L2Value::Null, L2Value::Null) => true,
        (L2Value::String(x), L2Value::String(y)) => x == y,
        (L2Value::Int(x), L2Value::Int(y)) => x == y,
        (L2Value::String(x), L2Value::Int(y)) => *x == y.to_string(),
        (L2Value::Int(x), L2Value::String(y)) => x.to_string() == *y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-01-20T16:00Z — the seed's term is mid-flight and its one session
    /// (2026-01-13) is inside a 14-day window (§4.4: tests freeze the clock).
    const NOW: i64 = 1_768_924_800;

    fn seed() -> ResolvedConfig {
        let resources = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Sources/SAMCore/Resources");
        crate::config_store::load_with(&resources.join("presets/seed"), "explicit", &resources)
            .expect("the seed loads")
    }

    fn with_screen(config: &mut ResolvedConfig, name: &str, type_: &str, blocks: Vec<BlockDef>) {
        config.views.insert(
            name.to_string(),
            ViewDef {
                type_: Some(type_.to_string()),
                layout: Some("list".to_string()),
                blocks: Some(blocks),
                ..Default::default()
            },
        );
    }

    fn block(kind: &str) -> BlockDef {
        BlockDef {
            kind: kind.to_string(),
            ..Default::default()
        }
    }

    fn resolve_screen(config: &ResolvedConfig, name: &str) -> ViewOutcome {
        resolve(
            config,
            name,
            &EvalContext::frozen(config, Some(&PlanState::empty()), NOW),
        )
        .expect("the view resolves")
    }

    #[test]
    fn a_view_without_blocks_draws_one_record_block() {
        let config = seed();
        let outcome = resolve_screen(&config, "today.topics");
        assert_eq!(outcome.blocks.len(), 1);
        assert_eq!(outcome.blocks[0]["kind"], serde_json::json!("list"));
        assert_eq!(
            outcome.blocks[0]["view"],
            serde_json::json!("today.topics"),
            "the block names the view it draws, so column writes land on the right one"
        );
        assert_eq!(
            outcome.blocks[0]["records"].as_array().unwrap().len(),
            outcome.records.len()
        );
    }

    #[test]
    fn a_screen_resolves_its_blocks_in_order_with_the_engine_doing_the_arithmetic() {
        let config = seed();
        let outcome = resolve_screen(&config, "today.screen");
        let kinds: Vec<&str> = outcome
            .blocks
            .iter()
            .map(|block| block["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, vec!["stat", "stat", "list"]);

        let open = &outcome.blocks[0];
        assert_eq!(open["reduce"], serde_json::json!("count"));
        assert_eq!(
            open["value"].as_i64().unwrap(),
            outcome.records.len() as i64,
            "the count is the view's own returned records"
        );
        let planned = &outcome.blocks[1];
        assert_eq!(planned["reduce"], serde_json::json!("sum"));
        let minutes: i64 = outcome
            .records
            .iter()
            .filter_map(|record| record.fields.get("est"))
            .filter_map(|value| value.as_i64())
            .sum();
        assert_eq!(planned["value"].as_i64().unwrap(), minutes);
        assert_eq!(outcome.blocks[2]["kind"], serde_json::json!("list"));
    }

    #[test]
    fn a_composed_screen_projects_no_records_and_carries_its_components() {
        let mut config = seed();
        config.views.insert(
            "morning.screen".to_string(),
            ViewDef {
                components: Some(vec![
                    crate::model::ComponentDef {
                        surface: "week-chart".into(),
                        span: Some(1),
                    },
                    crate::model::ComponentDef {
                        surface: "today-focus".into(),
                        span: None,
                    },
                ]),
                ..Default::default()
            },
        );
        let outcome = resolve_screen(&config, "morning.screen");
        assert!(outcome.records.is_empty(), "no type, no records");
        assert_eq!(outcome.candidates, 0);
        assert_eq!(outcome.columns, Vec::<String>::new());
        let json = outcome.json(&config);
        assert_eq!(
            json["components"],
            serde_json::json!([
                { "surface": "week-chart", "span": 1 },
                { "surface": "today-focus" }
            ]),
            "the composition is the projection, widths and all"
        );
        assert!(
            json["type"].is_null(),
            "the absent type is stated, not invented"
        );

        // A designed view answers the other way: the key is absent,
        // satisfying the UI's `components !== undefined` check.
        let typed = resolve_screen(&config, "today.topics").json(&config);
        assert!(
            typed.get("components").is_none(),
            "a designed view declares no composition"
        );
        assert_eq!(typed["type"], serde_json::json!("topic"));
    }

    #[test]
    fn an_unknown_kind_is_a_stated_placeholder_and_never_a_blank_screen() {
        let mut config = seed();
        with_screen(
            &mut config,
            "test.screen",
            "topic",
            vec![block("list"), block("sparkline")],
        );
        let outcome = resolve_screen(&config, "test.screen");
        assert_eq!(outcome.blocks.len(), 2);
        let fallback = &outcome.blocks[1];
        assert_eq!(fallback["kind"], serde_json::json!("placeholder"));
        assert_eq!(fallback["reason"], serde_json::json!("unknown-kind"));
        assert_eq!(fallback["unknown"], serde_json::json!("sparkline"));
        let message = fallback["message"].as_str().unwrap();
        assert!(message.starts_with("unknown block: \"sparkline\" · known: "));
        assert!(message.contains("callout"));
        assert!(
            fallback["known"].as_array().unwrap().len() == BLOCK_KINDS.len(),
            "the placeholder says what this build does know"
        );
    }

    #[test]
    fn a_tree_deeper_than_the_boundary_stops_at_the_placeholder() {
        let mut config = seed();
        // Seven nested `columns` — one past MAX_BLOCK_DEPTH.
        let mut nested = block("columns");
        for _ in 0..(MAX_BLOCK_DEPTH + 1) {
            nested = BlockDef {
                blocks: Some(vec![nested]),
                ..block("columns")
            };
        }
        with_screen(&mut config, "test.deep", "topic", vec![nested]);
        let outcome = resolve_screen(&config, "test.deep");
        let mut node = &outcome.blocks[0];
        let mut depth = 0;
        loop {
            match node["kind"].as_str() {
                Some("columns") => {
                    node = &node["blocks"][0];
                    depth += 1;
                }
                Some("placeholder") => {
                    assert_eq!(node["reason"], serde_json::json!("depth"));
                    assert!(
                        node["message"].as_str().unwrap().contains("not drawn"),
                        "a stopped subtree says why it stopped"
                    );
                    break;
                }
                other => panic!("unexpected node kind {other:?} at depth {depth}"),
            }
        }
        assert_eq!(
            depth, MAX_BLOCK_DEPTH,
            "the boundary is the declared constant, not an accident of the walk"
        );
    }

    #[test]
    fn a_repeat_gives_every_instance_its_own_record_context() {
        let mut config = seed();
        let mut conditional = block("conditional");
        conditional.when = Some("credits > 3".into());
        let mut nested = block("columns");
        nested.blocks = Some(vec![conditional]);
        let mut repeat = block("repeat");
        repeat.view = Some("courses.grid".into());
        repeat.blocks = Some(vec![nested]);
        with_screen(&mut config, "test.repeat", "course", vec![repeat]);

        let outcome = resolve_screen(&config, "test.repeat");
        let instances = outcome.blocks[0]["instances"].as_array().unwrap();
        assert_eq!(instances.len(), 2, "one instance per course");
        assert_eq!(
            instances[0]["record"]["id"],
            serde_json::json!("c.demo.a"),
            "the sort is the source view's (`name`), so the order is its order"
        );
        assert_eq!(
            instances[0]["blocks"][0]["blocks"][0]["matched"],
            serde_json::json!(true),
            "4 credits passes `credits > 3`"
        );
        assert_eq!(
            instances[1]["blocks"][0]["blocks"][0]["matched"],
            serde_json::json!(false),
            "3 credits does not — and the same expression ran against the second record"
        );
    }

    #[test]
    fn a_chart_fills_the_days_nothing_was_logged() {
        let config = seed();
        let outcome = resolve_screen(&config, "progress.screen");
        let chart = outcome
            .blocks
            .iter()
            .find(|block| block["kind"] == serde_json::json!("chart"))
            .expect("the progress screen has a chart");
        let buckets = chart["buckets"].as_array().unwrap();
        assert_eq!(buckets.len(), 14, "`days` fills the window, gaps included");
        let logged = buckets
            .iter()
            .find(|bucket| bucket["key"] == serde_json::json!("2026-01-13"))
            .expect("the session's day");
        assert_eq!(
            logged["value"].as_i64().unwrap(),
            40,
            "sum(min) over that day's sessions"
        );
        let empty = buckets
            .iter()
            .find(|bucket| bucket["key"] == serde_json::json!("2026-01-20"))
            .expect("today has its own bucket");
        assert_eq!(
            empty["value"].as_i64().unwrap(),
            0,
            "a day with nothing logged reads as zero, not as a missing day"
        );
    }

    #[test]
    fn a_block_that_names_no_view_folds_the_view_it_lives_in() {
        let mut config = seed();
        let mut stat = block("stat");
        stat.expr = Some("credits".into());
        stat.label = Some("Credits".into());
        with_screen(&mut config, "test.stats", "course", vec![stat]);
        let outcome = resolve_screen(&config, "test.stats");
        assert_eq!(outcome.blocks[0]["view"], serde_json::json!("test.stats"));
        assert_eq!(
            outcome.blocks[0]["value"].as_i64().unwrap(),
            7,
            "4 + 3 credits, folded from this view's own records"
        );
    }

    #[test]
    fn fold_count_counts_instances_while_the_numeric_folds_skip_nulls() {
        let values = vec![
            L2Value::Bool(true),
            L2Value::Bool(false),
            L2Value::Null,
            L2Value::Int(3),
        ];
        assert_eq!(fold("count", &values).unwrap(), L2Value::Int(2));
        assert_eq!(
            fold("sum", &[L2Value::Int(3), L2Value::Null, L2Value::Int(4)]).unwrap(),
            L2Value::Int(7)
        );
        assert_eq!(fold("sum", &[]).unwrap(), L2Value::Null);
        assert_eq!(
            fold("avg", &[L2Value::Int(2), L2Value::Int(4)]).unwrap(),
            L2Value::Int(3)
        );
        assert!(
            fold("max", &[L2Value::String("x".into())]).is_err(),
            "a string is a diagnostic, not a zero"
        );
    }

    #[test]
    fn nulls_place_by_declaration_and_equal_values_fall_through() {
        assert_eq!(
            compare_keys(&L2Value::Null, &L2Value::Int(1), false),
            Some(Ordering::Greater)
        );
        assert_eq!(
            compare_keys(&L2Value::Null, &L2Value::Int(1), true),
            Some(Ordering::Less)
        );
        assert_eq!(compare_keys(&L2Value::Null, &L2Value::Null, false), None);
        assert_eq!(
            compare_keys(&L2Value::Int(1), &L2Value::Double(2.5), false),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn an_empty_state_loads_as_empty() {
        let state = PlanState::load(Path::new("/nonexistent-sam-plan"));
        assert!(state.progress.is_empty());
    }

    fn today_at(now: i64) -> serde_json::Value {
        let config = seed();
        today_json(
            &config,
            &EvalContext::frozen(&config, Some(&PlanState::empty()), now),
        )
    }

    #[test]
    fn a_covered_day_pairs_the_covering_weeks_index_with_the_terms_length() {
        // 2026-01-20 sits inside the seed's week 1 (2026-01-12 … 2026-01-25) and
        // after its term start, so both halves of "week 1 of 14" are facts the
        // plan already holds.
        let today = today_at(NOW);
        assert_eq!(today["date"], serde_json::json!("2026-01-20"));
        assert_eq!(today["day"], serde_json::json!(20));
        assert_eq!(today["month"], serde_json::json!("Jan"));
        assert_eq!(today["weekday"], serde_json::json!("Tue"));
        assert_eq!(today["week"]["index"], serde_json::json!(1));
        assert_eq!(
            today["week"]["of"],
            serde_json::json!(14),
            "the term's declared `weeks`, not the two week records it has"
        );
    }

    #[test]
    fn an_uncovered_day_states_the_date_and_claims_no_week() {
        // 60 days later the seed's weeks have ended: the frame still draws the
        // date, and it invents no week to fill the slot.
        let today = today_at(NOW + 60 * 86_400);
        assert_eq!(today["date"], serde_json::json!("2026-03-21"));
        assert_eq!(today["weekday"], serde_json::json!("Sat"));
        assert_eq!(today["week"], serde_json::Value::Null);
    }

    #[test]
    fn without_a_term_the_denominator_is_the_week_record_count() {
        let mut config = seed();
        config
            .positioned_records
            .retain(|positioned| positioned.record.type_ != "term");
        let today = today_json(
            &config,
            &EvalContext::frozen(&config, Some(&PlanState::empty()), NOW),
        );
        assert_eq!(today["week"]["index"], serde_json::json!(1));
        assert_eq!(
            today["week"]["of"],
            serde_json::json!(2),
            "no term to declare a length, so the week records are the count"
        );
    }
}
