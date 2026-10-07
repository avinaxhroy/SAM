//! Schedulers (§3.5, D11): computes next due dates given stored review state and rating (`fixed | sm2 | fsrs`).
//!
//! Key properties:
//! - Civil dates: Due dates are calendar dates in the plan's timezone; fixed ladder offsets
//!   are measured from the completion anchor rather than the previous review (§3.5).
//! - Historical ratings: Review history is logged as timestamped ratings; scheduler state
//!   (stability/difficulty for FSRS, ease/reps for SM-2, step for fixed) is derived and persisted.
//! - Versioned algorithms: FSRS pins [`FSRS_ALGORITHM`] and parameter sets for explicit migrations (D11, §9).
//! - Calendar day arithmetic: Uses `civil_from_days`/`days_from_civil` with fixed-offset timezones.

use serde_json::{Value, json};

use crate::evaluator::{EvalContext, civil_date};
use crate::model::PipelineDef;
use crate::pipeline::{self, ProgressEntry};
use crate::resolved_config::ResolvedConfig;
use crate::rules::SchedulerDef;

/// The FSRS algorithm version this build computes with: the pinned crate's
/// default model (FSRS-6, 21 weights). Recorded in every FSRS review state.
pub const FSRS_ALGORITHM: &str = "FSRS-6";

/// The review vocabulary: FSRS's four buttons, mapped onto SM-2's quality scale
/// (`again` = fail). One vocabulary across schedulers, so a plan can switch
/// without rewriting its history.
pub const RATINGS: [&str; 4] = ["again", "hard", "good", "easy"];

/// A sanity cap on a computed interval: 100 years. An interval beyond it is a
/// data problem, not a study plan.
pub const MAX_INTERVAL_DAYS: i64 = 36_500;

/// The next scheduled review when the last one is logged: two calendar weeks.
pub const UPCOMING_DAYS: i64 = 14;

#[derive(Debug)]
pub struct SchedulingError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for SchedulingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SchedulingError {}

fn error(code: &'static str, message: impl Into<String>) -> SchedulingError {
    SchedulingError {
        code,
        message: message.into(),
    }
}

// ── which scheduler, and with what settings ──────────────────────────────────

/// The scheduler a record's pipeline uses: the pipeline's own `scheduler` when
/// declared (an `srs` pipeline names one), otherwise the plan's — and the fixed
/// ladder when `rules.json` declares none at all (§3.5: the fixed ladder is the
/// default).
pub fn scheduler_for(config: &ResolvedConfig, pipeline: &PipelineDef) -> SchedulerDef {
    let plan = config.rules.scheduler.clone().unwrap_or_default();
    match pipeline.scheduler.as_deref() {
        Some(name) if name != plan.name => SchedulerDef {
            name: name.to_string(),
            fixed: plan.fixed.clone(),
            sm2: plan.sm2.clone(),
            fsrs: plan.fsrs.clone(),
        },
        _ => plan,
    }
}

// ── the stored review state ──────────────────────────────────────────────────

/// One record's scheduler state, inside `progress/<id>/review`. The fields are
/// the union of the three schedulers, with only the acting one's populated —
/// which is what makes a switch a visible migration rather than a silent one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Review {
    pub scheduler: String,
    /// The algorithm/parameter provenance, for FSRS (e.g. `FSRS-6 · default`).
    pub algorithm: Option<String>,
    /// The next due date, `YYYY-MM-DD`, or `None` when the ladder is exhausted.
    pub due: Option<String>,
    pub interval_days: Option<i64>,
    pub reps: Option<i64>,
    pub lapses: Option<i64>,
    /// The last review instant, ISO-8601 UTC.
    pub last: Option<String>,
    /// Fixed: the completion anchor the offsets are measured from.
    pub anchor: Option<String>,
    /// Fixed: how many offsets have been logged.
    pub step: Option<i64>,
    /// SM-2.
    pub ease: Option<f64>,
    /// FSRS.
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    /// Timestamped ratings — the history the scheduler states are derived from.
    pub log: Vec<Value>,
}

impl Review {
    pub fn from_json(value: &Value) -> Option<Review> {
        let object = value.as_object()?;
        let mut review = Review {
            scheduler: object
                .get("scheduler")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            algorithm: object
                .get("algorithm")
                .and_then(Value::as_str)
                .map(str::to_string),
            due: object
                .get("due")
                .and_then(Value::as_str)
                .map(str::to_string),
            interval_days: object.get("intervalDays").and_then(Value::as_i64),
            reps: object.get("reps").and_then(Value::as_i64),
            lapses: object.get("lapses").and_then(Value::as_i64),
            last: object
                .get("last")
                .and_then(Value::as_str)
                .map(str::to_string),
            anchor: object
                .get("anchor")
                .and_then(Value::as_str)
                .map(str::to_string),
            step: object.get("step").and_then(Value::as_i64),
            ease: object.get("ease").and_then(Value::as_f64),
            stability: object.get("stability").and_then(Value::as_f64),
            difficulty: object.get("difficulty").and_then(Value::as_f64),
            log: object
                .get("log")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        };
        if review.scheduler.is_empty() {
            review.scheduler = "fixed".into();
        }
        Some(review)
    }

    pub fn to_json(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("scheduler".into(), json!(self.scheduler));
        if let Some(algorithm) = &self.algorithm {
            object.insert("algorithm".into(), json!(algorithm));
        }
        if let Some(due) = &self.due {
            object.insert("due".into(), json!(due));
        }
        if let Some(interval) = self.interval_days {
            object.insert("intervalDays".into(), json!(interval));
        }
        if let Some(reps) = self.reps {
            object.insert("reps".into(), json!(reps));
        }
        if let Some(lapses) = self.lapses {
            object.insert("lapses".into(), json!(lapses));
        }
        if let Some(last) = &self.last {
            object.insert("last".into(), json!(last));
        }
        if let Some(anchor) = &self.anchor {
            object.insert("anchor".into(), json!(anchor));
        }
        if let Some(step) = self.step {
            object.insert("step".into(), json!(step));
        }
        if let Some(ease) = self.ease {
            object.insert("ease".into(), json!(ease));
        }
        if let Some(stability) = self.stability {
            object.insert("stability".into(), json!(stability));
        }
        if let Some(difficulty) = self.difficulty {
            object.insert("difficulty".into(), json!(difficulty));
        }
        if !self.log.is_empty() {
            object.insert("log".into(), Value::Array(self.log.clone()));
        }
        Value::Object(object)
    }

    pub fn is_due(&self, today: &str) -> bool {
        self.due.as_deref().is_some_and(|due| due <= today)
    }

    /// Days a due date is past `today` (0 when due today or later).
    pub fn overdue_days(&self, today: &str) -> i64 {
        match (self.due.as_deref(), parse_date(today)) {
            (Some(due), Some((y, m, d))) => match parse_date(due) {
                Some(date) => (crate::transaction::days_from_civil(y, m, d)
                    - crate::transaction::days_from_civil(date.0, date.1, date.2))
                .max(0),
                None => 0,
            },
            _ => 0,
        }
    }
}

/// `YYYY-MM-DD` → `(year, month, day)`, range-checked.
pub fn parse_date(text: &str) -> Option<(i64, u32, u32)> {
    let mut parts = text.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    if !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// A civil date plus `days`, both ways through the calendar — never seconds.
pub fn add_days(date: &str, days: i64) -> Option<String> {
    let (year, month, day) = parse_date(date)?;
    let days = crate::transaction::days_from_civil(year, month, day) + days;
    let (year, month, day) = crate::transaction::civil_from_days(days);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// Whole calendar days from `from` to `to` — the day count FSRS consumes.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    let (fy, fm, fd) = parse_date(from)?;
    let (ty, tm, td) = parse_date(to)?;
    Some(
        crate::transaction::days_from_civil(ty, tm, td)
            - crate::transaction::days_from_civil(fy, fm, fd),
    )
}

/// The civil date of an ISO instant in the plan's fixed offset.
pub fn date_of(instant: &str, tz_offset_minutes: i32) -> Option<String> {
    let unix = crate::transaction::parse_iso(instant)?;
    Some(civil_date(unix, tz_offset_minutes))
}

// ── the three schedulers ─────────────────────────────────────────────────────

/// Log one rating and compute the next state.
///
/// `entry` is the record's progress entry — the fixed ladder reads its anchor
/// from the stage that completed the pipeline. `fresh_anchor` says this call
/// recorded that stage: the anchoring review **is** the completion anchor, so it
/// consumes no offset and the ladder's first review is due at `anchor + 1`
/// (§3.5: offsets are measured from the completion anchor).
pub fn log(
    config: &ResolvedConfig,
    pipeline: &PipelineDef,
    entry: &ProgressEntry,
    rating: &str,
    at_instant: &str,
    tz_offset_minutes: i32,
    fresh_anchor: bool,
) -> Result<Review, SchedulingError> {
    if !RATINGS.contains(&rating) {
        return Err(error(
            "scheduler.unknown-rating",
            format!(
                "\"{rating}\" is not a review rating — declared: {}",
                RATINGS.join(", ")
            ),
        ));
    }
    let scheduler = scheduler_for(config, pipeline);
    let today = date_of(at_instant, tz_offset_minutes).ok_or_else(|| {
        error(
            "scheduler.bad-instant",
            format!("\"{at_instant}\" is not an ISO-8601 instant"),
        )
    })?;
    let stored = entry.review.as_ref().and_then(Review::from_json);
    let log_tail = stored
        .as_ref()
        .map(|review| review.log.clone())
        .unwrap_or_default();
    let previous = stored.filter(|review| review.scheduler == scheduler.name);
    let mut log = log_tail;
    log.push(json!({
        "at": at_instant,
        "rating": rating,
        "due": previous.as_ref().and_then(|review| review.due.clone()),
    }));

    match scheduler.name.as_str() {
        "fixed" => {
            let intervals = scheduler.intervals();
            let anchor = previous
                .as_ref()
                .and_then(|review| review.anchor.clone())
                .or_else(|| anchor_from_stages(entry, pipeline, tz_offset_minutes))
                .unwrap_or_else(|| today.clone());
            let step = if fresh_anchor {
                0
            } else {
                previous
                    .as_ref()
                    .and_then(|review| review.step)
                    .unwrap_or(0)
                    + 1
            };
            let due = intervals
                .get(step as usize)
                .and_then(|offset| add_days(&anchor, *offset));
            Ok(Review {
                scheduler: "fixed".into(),
                due,
                interval_days: intervals.get(step as usize).copied(),
                reps: Some(step),
                lapses: previous.as_ref().and_then(|review| review.lapses),
                last: Some(at_instant.to_string()),
                anchor: Some(anchor),
                step: Some(step),
                log,
                ..Default::default()
            })
        }
        "sm2" => {
            let sm2 = scheduler.sm2.clone().unwrap_or(crate::rules::Sm2Schedule {
                min_ease: None,
                first_interval: None,
            });
            let quality = match rating {
                "again" => 2,
                "hard" => 3,
                "good" => 4,
                _ => 5,
            };
            let ease = previous
                .as_ref()
                .and_then(|review| review.ease)
                .unwrap_or(crate::rules::DEFAULT_SM2_EASE);
            let interval_prev = previous
                .as_ref()
                .and_then(|review| review.interval_days)
                .unwrap_or(0);
            let reps_prev = previous
                .as_ref()
                .and_then(|review| review.reps)
                .unwrap_or(0);
            let lapses_prev = previous
                .as_ref()
                .and_then(|review| review.lapses)
                .unwrap_or(0);
            // SM-2: a fail (q < 3) resets the repetition count and repeats
            // soon — the civil-date translation of "repeat today" is due today.
            let (interval, reps, lapses) = if quality < 3 {
                (0, 0, lapses_prev + 1)
            } else {
                let reps = reps_prev + 1;
                let interval = match reps {
                    1 => sm2.first_interval(),
                    2 => 6,
                    _ => ((interval_prev.max(1) as f64) * ease).round() as i64,
                };
                (interval.clamp(1, MAX_INTERVAL_DAYS), reps, lapses_prev)
            };
            let ease_after = (ease
                + (0.1 - (5 - quality) as f64 * (0.08 + (5 - quality) as f64 * 0.02)))
                .max(sm2.min_ease());
            Ok(Review {
                scheduler: "sm2".into(),
                due: add_days(&today, interval),
                interval_days: Some(interval),
                reps: Some(reps),
                lapses: Some(lapses),
                last: Some(at_instant.to_string()),
                ease: Some(round6(ease_after)),
                log,
                ..Default::default()
            })
        }
        "fsrs" => {
            let fsrs = fsrs_with(scheduler.fsrs_weights())?;
            let desired = scheduler.desired_retention() as f32;
            let elapsed = previous
                .as_ref()
                .and_then(|review| review.last.as_deref())
                .and_then(|last| date_of(last, tz_offset_minutes))
                .and_then(|last| days_between(&last, &today))
                .map(|elapsed| elapsed.max(0) as u32)
                .unwrap_or(0);
            let current =
                previous
                    .as_ref()
                    .and_then(|review| match (review.stability, review.difficulty) {
                        (Some(stability), Some(difficulty)) => Some(fsrs::MemoryState {
                            stability: stability as f32,
                            difficulty: difficulty as f32,
                        }),
                        _ => None,
                    });
            let next = fsrs
                .next_states(current, desired, elapsed)
                .map_err(|problem| {
                    error(
                        "scheduler.fsrs",
                        format!("FSRS refused the history: {problem}"),
                    )
                })?;
            let (chosen, rating_index) = match rating {
                "again" => (next.again, 1),
                "hard" => (next.hard, 2),
                "good" => (next.good, 3),
                _ => (next.easy, 4),
            };
            let days = (chosen.interval.round() as i64).clamp(0, MAX_INTERVAL_DAYS);
            Ok(Review {
                scheduler: "fsrs".into(),
                algorithm: Some(format!(
                    "{FSRS_ALGORITHM} · {}",
                    if scheduler.fsrs_weights().is_some() {
                        "pinned parameters"
                    } else {
                        "default parameters"
                    }
                )),
                due: add_days(&today, days),
                interval_days: Some(days),
                reps: Some(
                    previous
                        .as_ref()
                        .and_then(|review| review.reps)
                        .unwrap_or(0)
                        + 1,
                ),
                lapses: Some(
                    previous
                        .as_ref()
                        .and_then(|review| review.lapses)
                        .unwrap_or(0)
                        + i64::from(rating_index == 1),
                ),
                last: Some(at_instant.to_string()),
                stability: Some(round6(chosen.memory.stability as f64)),
                difficulty: Some(round6(chosen.memory.difficulty as f64)),
                log,
                ..Default::default()
            })
        }
        other => Err(error(
            "scheduler.unknown",
            format!(
                "scheduler \"{other}\" is not one of {} — it is a setting, not a code path",
                crate::rules::SCHEDULERS.join(", ")
            ),
        )),
    }
}

/// The fixed ladder's anchor: the recorded instant of the stage that anchored
/// the record — `anchored`, else `completeWhen`, else the last recorded stage.
fn anchor_from_stages(
    entry: &ProgressEntry,
    pipeline: &PipelineDef,
    tz_offset_minutes: i32,
) -> Option<String> {
    let stage = ["anchored"]
        .into_iter()
        .find(|stage| entry.recorded(stage))
        .map(str::to_string)
        .or_else(|| {
            pipeline::complete_stage(pipeline)
                .filter(|stage| entry.recorded(stage))
                .map(str::to_string)
        })
        .or_else(|| {
            pipeline
                .stages
                .iter()
                .rev()
                .find(|stage| entry.recorded(stage))
                .cloned()
        })?;
    entry
        .stages
        .get(&stage)
        .and_then(|instant| date_of(instant, tz_offset_minutes))
}

/// The pinned crate, with a plan's weights when it declares them.
fn fsrs_with(weights: Option<Vec<f32>>) -> Result<fsrs::FSRS, SchedulingError> {
    match weights {
        None => Ok(fsrs::FSRS::default()),
        Some(weights) => fsrs::FSRS::new(&weights).map_err(|problem| {
            error(
                "scheduler.bad-weights",
                format!("FSRS refused the pinned weights: {problem}"),
            )
        }),
    }
}

/// Six decimals: enough to keep `f32` values stable in JSON text, few enough
/// that two runs serialize identically.
fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

// ── logging a review: one transaction ────────────────────────────────────────

/// Log one review rating against a record, in one whole-plan-validated
/// transaction.
///
/// Two things happen together when the pipeline is the flip loop (§3.5): if the
/// anchor stage is not recorded yet, the review **is** its evidence — the stage
/// is recorded through the same gate-checked transition the terminal uses —
/// and the scheduler then computes the next due date. A review that cannot
/// anchor says what is missing rather than anchoring silently.
pub fn log_review(
    config: &ResolvedConfig,
    resources_dir: &std::path::Path,
    id: &str,
    rating: &str,
    at: Option<&str>,
) -> Result<crate::doc_edit::PlannedEdit, crate::doc_edit::EditError> {
    let record = config
        .record(id)
        .ok_or_else(|| pipeline_fail("record.unknown", "", format!("no record named \"{id}\"")))?;
    let (name, pipeline) = pipeline::pipeline_for(config, &record.type_)?;
    if !RATINGS.contains(&rating) {
        return Err(pipeline_fail(
            "scheduler.unknown-rating",
            format!("content/rules.json#/scheduler"),
            format!(
                "\"{rating}\" is not a review rating — declared: {}",
                RATINGS.join(", ")
            ),
        ));
    }
    let mut document = pipeline::state_document(config)?;
    let instant = at
        .map(str::to_string)
        .unwrap_or_else(crate::transaction::iso_now);
    let tz = tz_offset(config);

    let mut anchored = false;
    let mut invalidated: Vec<String> = Vec::new();
    let entry = pipeline::entry_in(&document, id);
    if pipeline.anchor_skip.is_some() && !entry.recorded(pipeline::ANCHOR_STAGE) {
        // The anchor's prerequisites are the gate's business, and the gate
        // speaks through the transition: run it and let it refuse.
        let outcome = pipeline::advance_entry(
            config,
            record,
            &mut document,
            Some(pipeline::ANCHOR_STAGE),
            &pipeline::Evidence {
                review: Some(rating.to_string()),
                ..Default::default()
            },
            &instant,
        )?;
        anchored = true;
        invalidated = outcome.invalidated;
    }

    let mut entry = pipeline::entry_in(&document, id);
    // An `srs` machine moves its stages with its reviews (§3.5's separate
    // stores: the rating drives the scheduler, the stage machine moves with it).
    let mut stage_move: Option<String> = None;
    if let Some(target) = stage_after_review(pipeline, &entry, rating) {
        let target_index = pipeline
            .stages
            .iter()
            .position(|candidate| candidate == &target);
        let current = pipeline
            .stages
            .iter()
            .enumerate()
            .filter(|(_, stage)| entry.recorded(stage))
            .map(|(index, _)| index)
            .max();
        if let Some(target_index) = target_index {
            match current {
                Some(current) if target_index < current => {
                    for (index, stage) in pipeline.stages.iter().enumerate() {
                        if index > target_index && entry.stages.remove(stage).is_some() {
                            invalidated.push(stage.clone());
                        }
                    }
                }
                _ => {
                    entry.stages.insert(target.clone(), instant.clone());
                }
            }
            stage_move = Some(target);
        }
    }

    let review =
        log(config, pipeline, &entry, rating, &instant, tz, anchored).map_err(|error| {
            pipeline_fail(
                error.code,
                format!("content/rules.json#/scheduler"),
                error.message,
            )
        })?;
    entry.pipeline = Some(name.to_string());
    entry.review = Some(review.to_json());
    pipeline::set_entry_in(&mut document, id, &entry)?;

    let preview = json!({
        "id": id,
        "type": record.type_,
        "pipeline": name,
        "scheduler": review.scheduler,
        "algorithm": review.algorithm,
        "rating": rating,
        "anchored": anchored,
        "stage": stage_move,
        "stages": entry.stages.keys().collect::<Vec<_>>(),
        "recorded": entry.stages,
        "invalidated": invalidated,
        "review": review.to_json(),
        "due": review.due,
        "intervalDays": review.interval_days,
        "complete": pipeline::complete(pipeline, Some(&entry)),
        "file": pipeline::STATE_FILE,
    });
    let change = crate::doc_edit::doc_change(config, pipeline::STATE_FILE, &document);
    crate::doc_edit::finalize_with(config, resources_dir, vec![change], preview)
        .map(|edit| edit.named(format!("record.logReview {id} · {rating}")))
}

fn pipeline_fail(
    code: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) -> crate::doc_edit::EditError {
    crate::doc_edit::EditError::Invalid(vec![crate::diagnostic::Diagnostic::error(
        code, path, message,
    )])
}

// ── the queue the Reviews panel draws ────────────────────────────────────────

/// The label rule Appendix C.1 fixed for a record: `title` → `label` → `name`
/// → the first non-empty text field → id.
///
/// The middle rung is the same one `ui/src/types.ts`'s `recordLabel` keeps, and
/// it exists for the same reason: a kind whose own name for a row is something
/// else (`chapter` on a problem set, `term` on a vocabulary word) still reads as
/// words in a review queue. A row titled `p.jee.chem.atomic` is the engine
/// showing its primary key where a student expects a name (D2).
pub fn record_label(record: &crate::model::Record) -> String {
    for key in ["title", "label", "name"] {
        if let Some(text) = record.fields.get(key).and_then(Value::as_str)
            && !text.is_empty()
        {
            return text.to_string();
        }
    }
    for value in record.fields.values() {
        if let Some(text) = value.as_str()
            && !text.is_empty()
        {
            return text.to_string();
        }
    }
    record.id.clone()
}

/// The companion a row shows beside its label: the first relation target's
/// label — a topic's course, a unit's course, a session's course. Shared with
/// `today.view`, so a Today row and a review row name the same course.
pub(crate) fn context_of(config: &ResolvedConfig, record: &crate::model::Record) -> Option<Value> {
    let def = config.types.get(&record.type_)?;
    let mut keys: Vec<&str> = Vec::new();
    if let Some(parent) = def.parent.as_deref() {
        keys.push(parent);
    }
    keys.push("course");
    for key in keys {
        if let Some(target) = record.links.get(key).and_then(|ids| ids.first())
            && let Some(target) = config.record(target)
        {
            return Some(json!({ "key": key, "id": target.id, "label": record_label(target) }));
        }
    }
    None
}

/// The whole queue as data: what is due, what is coming, and what is waiting on
/// a stage. The Reviews panel renders this; it dispatches transitions back.
pub fn due_json(config: &ResolvedConfig, context: &EvalContext) -> Value {
    let today = civil_date(context.now_unix, context.tz_offset_minutes);
    let state = context.state;
    let mut due: Vec<Value> = Vec::new();
    let mut upcoming: Vec<Value> = Vec::new();
    let mut waiting: Vec<Value> = Vec::new();
    let mut complete_count = 0usize;
    let mut trackable = 0usize;
    let mut scheduled = 0usize;

    for record in config.records_iter() {
        let Some(def) = config.types.get(&record.type_) else {
            continue;
        };
        if def.trackable != Some(true) {
            continue;
        }
        trackable += 1;
        let Ok((name, pipeline_def)) = pipeline::pipeline_for(config, &record.type_) else {
            continue;
        };
        let entry = state.and_then(|state| state.progress.get(&record.id));
        let review = entry
            .and_then(|entry| entry.review.as_ref())
            .and_then(Review::from_json);
        let complete = pipeline::complete(pipeline_def, entry);
        if complete {
            complete_count += 1;
        }
        // The row every list shares: identity, label, context, and the next
        // transition with the evidence it asks for.
        let row = |review: Option<&Review>| {
            json!({
                "id": record.id,
                "type": record.type_,
                "title": record_label(record),
                "context": context_of(config, record),
                "pipeline": name,
                "stages": entry.map(|entry| entry.stages.keys().collect::<Vec<_>>()).unwrap_or_default(),
                "next": pipeline::next_stage(pipeline_def, entry),
                "asks": pipeline::asks(config, record, pipeline_def, entry)
                    .iter()
                    .map(pipeline::Ask::json)
                    .collect::<Vec<_>>(),
                "complete": complete,
                "review": review.map(Review::to_json),
                "due": review.and_then(|review| review.due.clone()),
                "overdueDays": review.map(|review| review.overdue_days(&today)).unwrap_or(0),
            })
        };
        match &review {
            Some(review) if review.due.is_some() => {
                scheduled += 1;
                let due_date = review.due.clone().unwrap_or_default();
                if due_date <= today {
                    due.push(row(Some(review)));
                } else if let Some(limit) = add_days(&today, UPCOMING_DAYS)
                    && due_date <= limit
                {
                    upcoming.push(row(Some(review)));
                }
            }
            _ if !complete => waiting.push(row(None)),
            _ => {}
        }
    }
    let by_due = |left: &Value, right: &Value| {
        left.get("due")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(right.get("due").and_then(Value::as_str).unwrap_or(""))
    };
    due.sort_by(|a, b| by_due(a, b).then_with(|| a["id"].as_str().cmp(&b["id"].as_str())));
    upcoming.sort_by(by_due);
    waiting.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    waiting.truncate(50);

    json!({
        "today": today,
        "timezoneMinutes": context.tz_offset_minutes,
        "timezone": config.rules.timezone.clone(),
        "scheduler": config
            .rules
            .scheduler
            .as_ref()
            .map(|scheduler| scheduler.name.clone())
            .unwrap_or_else(|| "fixed".into()),
        // Every declared machine, and which one each trackable type follows —
        // the data the panel's study-method control offers, so the picker never
        // hardcodes a list of methods.
        "pipelines": pipeline::pipelines_json(config)["pipelines"],
        "pipelineOf": config
            .types
            .iter()
            .filter_map(|(name, def)| {
                def.pipeline
                    .as_ref()
                    .map(|pipeline| (name.clone(), pipeline.clone()))
            })
            .collect::<std::collections::BTreeMap<_, _>>(),
        "counts": {
            "trackable": trackable,
            "complete": complete_count,
            "scheduled": scheduled,
            "due": due.len(),
            "upcoming": upcoming.len(),
            "waiting": waiting.len(),
        },
        "due": due,
        "upcoming": upcoming,
        "waiting": waiting,
    })
}

/// Turn a rating into the next stage for a pipeline whose `scheduler` drives
/// its stages (an `srs` machine): a failure steps back one stage, a success
/// steps forward one — never past the last, so `mature` is reached by review,
/// not by a checkbox (§3.5: *"`mature` is not an automatic claim that study is
/// finished"*).
pub fn stage_after_review(
    pipeline: &PipelineDef,
    entry: &ProgressEntry,
    rating: &str,
) -> Option<String> {
    let scheduler = pipeline.scheduler.as_deref()?;
    if scheduler.is_empty() {
        return None;
    }
    let recorded: Vec<&String> = pipeline
        .stages
        .iter()
        .filter(|stage| entry.recorded(stage))
        .collect();
    let current = pipeline
        .stages
        .iter()
        .rposition(|stage| recorded.iter().any(|recorded| *recorded == stage));
    match rating {
        "again" => {
            let index = current?;
            if index == 0 {
                None
            } else {
                Some(pipeline.stages[index - 1].clone())
            }
        }
        _ => {
            let index = current.map(|index| index + 1).unwrap_or(0);
            pipeline.stages.get(index).cloned()
        }
    }
}

/// The plan timezone's offset in minutes, resolved once, for callers that hold
/// no [`EvalContext`] — the same resolution an evaluation uses, so a due date
/// and a `today()` can never disagree about what day it is.
pub fn tz_offset(config: &ResolvedConfig) -> i32 {
    crate::evaluator::plan_timezone(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S6 (§6 Phase 6): the pinned crate's model, replayed against its own
    /// documented expectation before anything integrates it. The doc example for
    /// `FSRS::default().next_states(None, 0.9, 0)` is the known history:
    /// stability 0.212 / 1.2931 / 2.3065 / 8.2956 for again / hard / good / easy.
    #[test]
    fn the_pinned_fsrs_model_replays_its_documented_expectation() {
        let fsrs = fsrs::FSRS::default();
        let next = fsrs.next_states(None, 0.9, 0).expect("computes");
        assert!((next.again.memory.stability - 0.212).abs() < 1e-4);
        assert!((next.hard.memory.stability - 1.2931).abs() < 1e-4);
        assert!((next.good.memory.stability - 2.3065).abs() < 1e-4);
        assert!((next.easy.memory.stability - 8.2956).abs() < 1e-4);
        assert_eq!(FSRS_ALGORITHM, "FSRS-6");
        // The parameter set is the crate's own 21-weight FSRS-6 default.
        assert_eq!(fsrs::DEFAULT_PARAMETERS.len(), 21);
    }

    /// The interval arithmetic this module adds on top: a `f32` interval
    /// becomes a whole number of civil days, never a fraction.
    #[test]
    fn intervals_become_whole_civil_days() {
        let fsrs = fsrs::FSRS::default();
        let next = fsrs.next_states(None, 0.9, 0).expect("computes");
        assert_eq!((next.good.interval.round() as i64), 2);
        assert_eq!((next.again.interval.round() as i64), 0);
    }

    #[test]
    fn calendar_arithmetic_crosses_months_and_dst_dates() {
        assert_eq!(add_days("2026-01-31", 1).as_deref(), Some("2026-02-01"));
        // A DST transition day in a named zone is still one calendar day: the
        // engine adds days through the calendar, never seconds.
        assert_eq!(add_days("2026-03-08", 1).as_deref(), Some("2026-03-09"));
        assert_eq!(days_between("2026-03-07", "2026-03-09"), Some(2));
        assert_eq!(
            date_of("2026-03-08T23:30:00-05:00", -300).as_deref(),
            Some("2026-03-08")
        );
        assert_eq!(
            date_of("2026-03-08T07:30:00Z", -300).as_deref(),
            Some("2026-03-08")
        );
        // The same instant in a +05:30 plan is already the 9th.
        assert_eq!(
            date_of("2026-03-08T23:30:00Z", 330).as_deref(),
            Some("2026-03-09")
        );
    }
}
