//! Pipeline stage machine for trackable records (§3.5, Phase 6, §4.8).
//!
//! Models stage progression, transition gates, required evidence, flip loops,
//! and backtrack invalidation. Stage mutations commit whole-plan-validated updates
//! to `state/state.json` through the unified transaction engine.
//!
//! Spec stage conventions:
//! - `proved`: guarded by the `proof` gate threshold (§3.5).
//! - `anchored`: governed by `anchorSkip` evidence requirements (§3.5).
//!
//! Pipelines declaring `proof` or `anchorSkip` without corresponding stages emit
//! advisory load diagnostics.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::diagnostic::Diagnostic;
use crate::doc_edit::{self, EditError, PlannedEdit};
use crate::model::{PipelineDef, Proof, Record, TypeDef};
use crate::resolved_config::ResolvedConfig;

/// The stage the `proof` gate guards (§3.5).
pub const PROOF_STAGE: &str = "proved";
/// The stage `anchorSkip` governs (§3.5).
pub const ANCHOR_STAGE: &str = "anchored";
/// The progress document (§4.1).
pub const STATE_FILE: &str = "state/state.json";

fn fail(code: &str, path: impl Into<String>, message: impl Into<String>) -> EditError {
    EditError::Invalid(vec![Diagnostic::error(code, path, message)])
}

// ── the progress entry: one model for reading and writing ────────────────────

/// One record's entry in `state/state.json#/progress/<id>` (§3.5). `stages`
/// maps a stage name to the ISO-8601 UTC instant it was recorded; `data` holds
/// typed evidence; `review` is the scheduler's state ([`crate::scheduler`]).
/// Keys this build does not model are preserved in `extra`, so a newer writer's
/// field is not silently dropped by an older one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgressEntry {
    pub pipeline: Option<String>,
    pub stages: BTreeMap<String, String>,
    pub data: BTreeMap<String, Value>,
    pub review: Option<Value>,
    /// Prior pipelines' stages, preserved when the machine changes (§3.5:
    /// *"preserving prior history"*).
    pub history: Vec<Value>,
    pub extra: Map<String, Value>,
}

impl ProgressEntry {
    pub fn from_json(value: &Value) -> Self {
        let mut entry = ProgressEntry::default();
        let Some(object) = value.as_object() else {
            return entry;
        };
        entry.pipeline = object
            .get("pipeline")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(stages) = object.get("stages").and_then(Value::as_object) {
            for (stage, instant) in stages {
                entry
                    .stages
                    .insert(stage.clone(), instant.as_str().unwrap_or("").to_string());
            }
        }
        if let Some(data) = object.get("data").and_then(Value::as_object) {
            entry.data = data.clone().into_iter().collect();
        }
        entry.review = object.get("review").cloned();
        entry.history = object
            .get("history")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (key, value) in object {
            if !matches!(
                key.as_str(),
                "pipeline" | "stages" | "data" | "review" | "history"
            ) {
                entry.extra.insert(key.clone(), value.clone());
            }
        }
        entry
    }

    pub fn to_json(&self) -> Value {
        let mut object = self.extra.clone();
        if let Some(pipeline) = &self.pipeline {
            object.insert("pipeline".into(), json!(pipeline));
        }
        object.insert(
            "stages".into(),
            Value::Object(
                self.stages
                    .iter()
                    .map(|(stage, instant)| (stage.clone(), json!(instant)))
                    .collect(),
            ),
        );
        if !self.data.is_empty() {
            object.insert(
                "data".into(),
                Value::Object(
                    self.data
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(review) = &self.review {
            object.insert("review".into(), review.clone());
        }
        if !self.history.is_empty() {
            object.insert("history".into(), Value::Array(self.history.clone()));
        }
        Value::Object(object)
    }

    pub fn recorded(&self, stage: &str) -> bool {
        self.stages.contains_key(stage)
    }
}

/// The progress document from the loaded snapshot, ready to edit: the parsed
/// source bytes, or a fresh skeleton when the plan has never recorded progress.
pub fn state_document(config: &ResolvedConfig) -> Result<Value, EditError> {
    match config.source_files.get(STATE_FILE) {
        Some(bytes) => {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                fail(
                    "state.encoding",
                    STATE_FILE,
                    "state/state.json is not valid UTF-8",
                )
            })?;
            doc_edit::doc_json::parse(text, STATE_FILE).map_err(EditError::from)
        }
        None => Ok(json!({ "schemaVersion": 1, "progress": {} })),
    }
}

/// The entry for one record inside a state document.
pub fn entry_in(document: &Value, id: &str) -> ProgressEntry {
    document
        .get("progress")
        .and_then(Value::as_object)
        .and_then(|progress| progress.get(id))
        .map(ProgressEntry::from_json)
        .unwrap_or_default()
}

/// Write an entry back into a state document, creating the containers.
pub fn set_entry_in(
    document: &mut Value,
    id: &str,
    entry: &ProgressEntry,
) -> Result<(), EditError> {
    if document.get("progress").is_none() {
        doc_edit::doc_json::set(document, &["progress".into()], json!({}))?;
    }
    let Some(progress) = document.get_mut("progress").and_then(Value::as_object_mut) else {
        return Err(fail(
            "state.shape",
            format!("{STATE_FILE}#/progress"),
            "progress must be an object",
        ));
    };
    progress.insert(id.to_string(), entry.to_json());
    Ok(())
}

// ── the machine ──────────────────────────────────────────────────────────────

/// The declared pipeline for a type: `(name, definition)`. Every refusal names
/// what is missing, because "not trackable" and "no such pipeline" are the two
/// authoring mistakes this path meets most.
pub fn pipeline_for<'a>(
    config: &'a ResolvedConfig,
    type_name: &str,
) -> Result<(&'a str, &'a PipelineDef), EditError> {
    let Some(def) = config.types.get(type_name) else {
        return Err(fail(
            "pipeline.unknown-type",
            format!("content/types.json#/types/{type_name}"),
            format!("no type named \"{type_name}\""),
        ));
    };
    let Some(name) = def.pipeline.as_deref() else {
        return Err(fail(
            "pipeline.none",
            format!("content/types.json#/types/{type_name}/pipeline"),
            format!(
                "type \"{type_name}\" is not trackable — declare a pipeline on it before recording stages"
            ),
        ));
    };
    let Some(pipeline) = config
        .rules
        .pipelines
        .as_ref()
        .and_then(|pipelines| pipelines.get(name))
    else {
        return Err(fail(
            "pipeline.undeclared",
            format!("content/rules.json#/pipelines/{name}"),
            format!("type \"{type_name}\" names pipeline \"{name}\", which is not declared"),
        ));
    };
    Ok((name, pipeline))
}

/// The stage a complete record has reached (§3.5): `completeWhen` when
/// declared, else the last declared stage.
pub fn complete_stage(pipeline: &PipelineDef) -> Option<&str> {
    pipeline
        .complete_when
        .as_deref()
        .or_else(|| pipeline.stages.last().map(String::as_str))
}

pub fn complete(pipeline: &PipelineDef, entry: Option<&ProgressEntry>) -> bool {
    let Some(entry) = entry else {
        return false;
    };
    complete_stage(pipeline).is_some_and(|stage| entry.recorded(stage))
}

/// The next unrecorded stage, in declared order.
pub fn next_stage(pipeline: &PipelineDef, entry: Option<&ProgressEntry>) -> Option<String> {
    pipeline.stages.iter().find_map(|stage| {
        let recorded = entry.is_some_and(|entry| entry.recorded(stage));
        (!recorded).then(|| stage.clone())
    })
}

/// Whether a pipeline's `proof` gate applies to this record: the declared kinds
/// when `appliesToKinds` is present, otherwise every record of the type (§3.5:
/// *"The default rules apply this proof gate to watch tasks"*).
pub fn proof_applies(proof: &Proof, record: &Record) -> bool {
    match proof.applies_to_kinds.as_deref() {
        Some(kinds) => record
            .fields
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|kind| kinds.iter().any(|candidate| candidate == kind)),
        None => true,
    }
}

// ── ask-rules: what a transition asks for ────────────────────────────────────

/// One piece of evidence a transition asks for — the "ask rules" of §3.5 as
/// data, so the Reviews panel renders the same question the engine validates and
/// the terminal has the same flag.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub key: String,
    pub label: String,
    /// `integer` | `text`.
    pub kind: String,
    pub required: bool,
    pub min: Option<i64>,
    /// The stage this evidence is for.
    pub stage: String,
    /// Why it is asked, in the student's words.
    pub note: String,
}

impl Ask {
    pub fn json(&self) -> Value {
        json!({
            "key": self.key,
            "label": self.label,
            "kind": self.kind,
            "required": self.required,
            "min": self.min,
            "stage": self.stage,
            "note": self.note,
        })
    }
}

/// What the **next** transition asks for. Empty when the pipeline is complete —
/// the next action is a review, not a stage advance.
pub fn asks(
    config: &ResolvedConfig,
    record: &Record,
    pipeline: &PipelineDef,
    entry: Option<&ProgressEntry>,
) -> Vec<Ask> {
    let Some(next) = next_stage(pipeline, entry) else {
        return Vec::new();
    };
    let mut asks = Vec::new();
    if next == PROOF_STAGE
        && let Some(proof) = &pipeline.proof
        && proof_applies(proof, record)
    {
        let min = proof.min_problems.unwrap_or(2);
        asks.push(Ask {
            key: "problems".into(),
            label: "Problems solved".into(),
            kind: "integer".into(),
            required: true,
            min: Some(min),
            stage: next.clone(),
            note: format!("proved needs at least {min} solved problems — a prompt is not evidence"),
        });
    }
    if next == ANCHOR_STAGE && pipeline.anchor_skip.is_some() {
        asks.push(Ask {
            key: "reason".into(),
            label: "Anchor-skip reason".into(),
            kind: "text".into(),
            required: false,
            min: None,
            stage: next.clone(),
            note: "log a review to anchor it, or give a reason to skip the anchor".into(),
        });
    }
    asks.push(Ask {
        key: "signal".into(),
        label: "Signal for later".into(),
        kind: "text".into(),
        required: false,
        min: None,
        stage: next,
        note: format!(
            "stored separately from the stage, under {}#/progress/<id>/data/signal",
            config.plan_root.join(STATE_FILE).display()
        ),
    });
    asks
}

// ── advance: one transaction, forward or back ────────────────────────────────

/// The evidence a transition carries. Absent fields are absent, never empty
/// strings (§4.9: *"An empty field means 'not supplied'"*).
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    /// Solved problems — the proof gate's typed evidence.
    pub problems: Option<i64>,
    /// The anchor-skip reason.
    pub reason: Option<String>,
    /// The lecture signal, stored apart from the stage.
    pub signal: Option<String>,
    /// Set when the transition comes from a logged review; the review is then
    /// the anchor's evidence kind.
    pub review: Option<String>,
}

/// Record (or un-record) one stage, in one whole-plan-validated transaction.
///
/// Forward: gates must be satisfied, `proved` needs its problems, `anchored`
/// needs either a logged review or an explicit skip reason. Backward: every
/// later stage is invalidated, and a review scheduled from an invalidated
/// stage is cleared (§3.5: *"Backtracking invalidates dependent stages and
/// future review dates, with undo"*).
pub fn advance(
    config: &ResolvedConfig,
    resources_dir: &Path,
    id: &str,
    target: Option<&str>,
    evidence: &Evidence,
    at: Option<&str>,
) -> Result<PlannedEdit, EditError> {
    let record = config
        .record(id)
        .ok_or_else(|| fail("record.unknown", "", format!("no record named \"{id}\"")))?;
    let mut document = state_document(config)?;
    let instant = at
        .map(str::to_string)
        .unwrap_or_else(crate::transaction::iso_now);
    let outcome = advance_entry(config, record, &mut document, target, evidence, &instant)?;
    let (name, pipeline) = pipeline_for(config, &record.type_)?;
    let preview = outcome.preview(config, record, pipeline, name);
    let change = doc_edit::doc_change(config, STATE_FILE, &document);
    doc_edit::finalize_with(config, resources_dir, vec![change], preview)
        .map(|edit| edit.named(format!("record.advanceStage {id} → {}", outcome.stage)))
}

/// What one stage transition did — the write model [`advance`] and
/// [`crate::scheduler`]'s review path share, so the anchor a review records and
/// the anchor the terminal records are the same transition.
#[derive(Debug, Clone)]
pub struct AdvanceOutcome {
    pub entry: ProgressEntry,
    pub pipeline: String,
    pub stage: String,
    pub invalidated: Vec<String>,
    pub review_cleared: bool,
    pub backtracked: bool,
    pub complete: bool,
}

impl AdvanceOutcome {
    /// The dry-run/preview envelope every write reports.
    pub fn preview(
        &self,
        config: &ResolvedConfig,
        record: &Record,
        pipeline: &PipelineDef,
        name: &str,
    ) -> Value {
        json!({
            "id": record.id,
            "type": record.type_,
            "pipeline": name,
            "stage": self.stage,
            "stages": self.entry.stages.keys().collect::<Vec<_>>(),
            "recorded": self.entry.stages,
            "evidence": self.entry.data,
            "invalidated": self.invalidated,
            "reviewCleared": self.review_cleared,
            "backtracked": self.backtracked,
            "complete": self.complete,
            "completeWhen": complete_stage(pipeline),
            "next": next_stage(pipeline, Some(&self.entry)),
            "asks": asks(config, record, pipeline, Some(&self.entry))
                .iter()
                .map(Ask::json)
                .collect::<Vec<_>>(),
            "file": STATE_FILE,
        })
    }
}

/// Apply one transition to an already-parsed state document. Pure except for
/// the document it is handed — the caller validates and commits through
/// [`doc_edit::finalize_with`] exactly once, which is what keeps "one
/// transition, one transaction" true even when a review also records its anchor.
pub fn advance_entry(
    config: &ResolvedConfig,
    record: &Record,
    document: &mut Value,
    target: Option<&str>,
    evidence: &Evidence,
    instant: &str,
) -> Result<AdvanceOutcome, EditError> {
    let id = &record.id;
    let (name, pipeline) = pipeline_for(config, &record.type_)?;
    let mut entry = entry_in(document, id);
    if let Some(recorded) = &entry.pipeline
        && recorded != name
        && !entry.stages.is_empty()
    {
        return Err(fail(
            "pipeline.mismatch",
            format!("{STATE_FILE}#/progress/{id}/pipeline"),
            format!(
                "record \"{id}\" has progress under pipeline \"{recorded}\" but its type declares \"{name}\" — migrate it with type.setPipeline, or clear the entry"
            ),
        ));
    }
    let position = |stage: &str| {
        pipeline
            .stages
            .iter()
            .position(|candidate| candidate == stage)
    };
    let target = match target {
        Some(stage) if position(stage).is_some() => stage.to_string(),
        Some(stage) => {
            return Err(fail(
                "pipeline.unknown-stage",
                format!("content/rules.json#/pipelines/{name}/stages"),
                format!(
                    "\"{stage}\" is not a declared stage of {name} — declared: {}",
                    pipeline.stages.join(" → ")
                ),
            ));
        }
        None => next_stage(pipeline, Some(&entry)).ok_or_else(|| {
            fail(
                "pipeline.stage-complete",
                format!("{STATE_FILE}#/progress/{id}"),
                format!("every stage of {name} is recorded — log a review instead of advancing"),
            )
        })?,
    };
    let target_index = position(&target).expect("the target is a declared stage");
    let highest = pipeline
        .stages
        .iter()
        .enumerate()
        .filter(|(_, stage)| entry.recorded(stage))
        .map(|(index, _)| index)
        .max();
    let backtracking = highest.is_some_and(|highest| target_index < highest);

    let mut invalidated: Vec<String> = Vec::new();
    if backtracking {
        for (index, stage) in pipeline.stages.iter().enumerate() {
            if index > target_index && entry.stages.remove(stage).is_some() {
                invalidated.push(stage.clone());
            }
        }
    } else {
        // ---- gates
        if let Some(require) = pipeline
            .gates
            .as_ref()
            .and_then(|gates| gates.get(&target))
            .and_then(|gate| gate.require.as_deref())
            && !entry.recorded(require)
        {
            return Err(fail(
                "pipeline.gate",
                format!("content/rules.json#/pipelines/{name}/gates/{target}"),
                format!("stage \"{target}\" requires \"{require}\" — record it first"),
            ));
        }
        // ---- proof (§3.5: a prompt alone is not evidence)
        if target == PROOF_STAGE
            && let Some(proof) = &pipeline.proof
            && proof_applies(proof, record)
        {
            let min = proof.min_problems.unwrap_or(2);
            match evidence.problems {
                Some(problems) if problems >= min => {
                    entry.data.insert("problemsSolved".into(), json!(problems));
                }
                Some(problems) => {
                    return Err(fail(
                        "pipeline.proof-short",
                        format!("{STATE_FILE}#/progress/{id}/data/problemsSolved"),
                        format!(
                            "proved needs at least {min} solved problems; got {problems} — keep going and record the real count"
                        ),
                    ));
                }
                None => {
                    return Err(fail(
                        "pipeline.proof-required",
                        format!("{STATE_FILE}#/progress/{id}/data/problemsSolved"),
                        format!(
                            "proved needs problems solved as evidence — pass --problems N (>= {min}); a prompt alone is not evidence"
                        ),
                    ));
                }
            }
        }
        // ---- anchor (§3.5: evidence or an explicit skip reason)
        if target == ANCHOR_STAGE
            && let Some(skip) = &pipeline.anchor_skip
        {
            match (&evidence.reason, &evidence.review) {
                (Some(reason), _) if !reason.is_empty() => {
                    if skip.allowed == Some(false) {
                        return Err(fail(
                            "pipeline.anchor-skip-forbidden",
                            format!("content/rules.json#/pipelines/{name}/anchorSkip/allowed"),
                            "this pipeline does not allow skipping the anchor — log a review instead",
                        ));
                    }
                    // Evidence kinds are alternatives, not layers: a skip
                    // replaces an earlier review's evidence rather than
                    // leaving both to be read as agreement.
                    entry.data.remove("anchorEvidence");
                    entry.data.insert("anchorSkip".into(), json!(reason));
                }
                (_, Some(rating)) => {
                    entry.data.remove("anchorSkip");
                    entry.data.insert(
                        "anchorEvidence".into(),
                        json!({ "kind": "review", "rating": rating }),
                    );
                }
                _ => {
                    return Err(fail(
                        "pipeline.anchor-required",
                        format!("{STATE_FILE}#/progress/{id}"),
                        format!(
                            "anchoring \"{target}\" needs a logged review (record.logReview) or an explicit --reason to skip it"
                        ),
                    ));
                }
            }
        }
        if let Some(signal) = &evidence.signal
            && !signal.is_empty()
        {
            // §3.5: the lecture signal is stored separately from the stage.
            entry.data.insert("signal".into(), json!(signal));
        }
        entry.stages.insert(target.clone(), instant.to_string());
    }

    let complete = complete_stage(pipeline).is_some_and(|stage| entry.recorded(stage));
    let review_cleared = !complete && entry.review.take().is_some();
    entry.pipeline = Some(name.to_string());
    set_entry_in(document, id, &entry)?;
    Ok(AdvanceOutcome {
        entry,
        pipeline: name.to_string(),
        stage: target,
        invalidated,
        review_cleared,
        backtracked: backtracking,
        complete,
    })
}

// ── state as data (the Reviews panel's per-record read) ──────────────────────

/// One record's pipeline as JSON: declared stages, recorded instants, the next
/// transition and what it asks for. The panel renders this; it never
/// re-derives a stage machine in TypeScript.
pub fn state_json(config: &ResolvedConfig, id: &str) -> Result<Value, EditError> {
    let record = config
        .record(id)
        .ok_or_else(|| fail("record.unknown", "", format!("no record named \"{id}\"")))?;
    let (name, pipeline) = pipeline_for(config, &record.type_)?;
    let document = state_document(config)?;
    let entry = entry_in(&document, id);
    let recorded: BTreeMap<String, Value> = pipeline
        .stages
        .iter()
        .filter(|stage| entry.recorded(stage))
        .map(|stage| (stage.clone(), json!(entry.stages[stage])))
        .collect();
    Ok(json!({
        "id": id,
        "type": record.type_,
        "pipeline": name,
        "stages": pipeline.stages,
        "recorded": recorded,
        "next": next_stage(pipeline, Some(&entry)),
        "asks": asks(config, record, pipeline, Some(&entry))
            .iter()
            .map(Ask::json)
            .collect::<Vec<_>>(),
        "complete": complete(pipeline, Some(&entry)),
        "completeWhen": complete_stage(pipeline),
        "evidence": entry.data,
        "review": entry.review,
        "address": format!("{STATE_FILE}#/progress/{id}"),
    }))
}

// ── changing the machine: previewed history mapping (§3.5) ───────────────────

/// Switch a type from one pipeline to another.
///
/// The rule §3.5 states and this implements: *"Changing pipeline requires a
/// previewed stage mapping or a fresh active state while preserving prior
/// history; it never silently interprets old stage names."* Every record's
/// previous machine is appended to `history`, so nothing is rewritten out of
/// existence — and `learned` never quietly becomes `done`.
///
/// `mapping` maps old stage name → new stage name; `fresh` starts a new active
/// state with no stages recorded. One of the two is required.
pub fn set_pipeline(
    config: &ResolvedConfig,
    resources_dir: &Path,
    type_name: &str,
    new_pipeline: &str,
    mapping: &BTreeMap<String, String>,
    fresh: bool,
) -> Result<PlannedEdit, EditError> {
    let Some(def) = config.types.get(type_name) else {
        return Err(fail(
            "pipeline.unknown-type",
            format!("content/types.json#/types/{type_name}"),
            format!("no type named \"{type_name}\""),
        ));
    };
    let Some(target) = config
        .rules
        .pipelines
        .as_ref()
        .and_then(|pipelines| pipelines.get(new_pipeline))
    else {
        return Err(fail(
            "pipeline.undeclared",
            format!("content/rules.json#/pipelines/{new_pipeline}"),
            format!("pipeline \"{new_pipeline}\" is not declared"),
        ));
    };
    if !fresh && mapping.is_empty() {
        return Err(fail(
            "pipeline.mapping-required",
            format!("content/types.json#/types/{type_name}/pipeline"),
            format!(
                "changing {type_name} to \"{new_pipeline}\" needs --map old=new (previewed stage mapping) or --fresh (start a new active state) — old stage names are never interpreted silently"
            ),
        ));
    }
    let old_name = def.pipeline.clone();
    let old_stages: Vec<String> = old_name
        .as_deref()
        .and_then(|name| {
            config
                .rules
                .pipelines
                .as_ref()
                .and_then(|pipelines| pipelines.get(name))
        })
        .map(|pipeline| pipeline.stages.clone())
        .unwrap_or_default();
    for (from, to) in mapping {
        if !old_stages.contains(from) {
            return Err(fail(
                "pipeline.map-unknown-stage",
                format!("content/types.json#/types/{type_name}/pipeline"),
                format!(
                    "\"{from}\" is not a stage of the current pipeline ({})",
                    old_stages.join(" → ")
                ),
            ));
        }
        if !target.stages.contains(to) {
            return Err(fail(
                "pipeline.map-unknown-target",
                format!("content/rules.json#/pipelines/{new_pipeline}/stages"),
                format!(
                    "\"{to}\" is not a declared stage of {new_pipeline} ({})",
                    target.stages.join(" → ")
                ),
            ));
        }
    }

    let mut document = state_document(config)?;
    let instant = crate::transaction::iso_now();
    let mut moved: Vec<Value> = Vec::new();
    let mut unmapped: Vec<String> = Vec::new();
    for record in config.records_iter() {
        if record.type_ != type_name {
            continue;
        }
        let existing = entry_in(&document, &record.id);
        if existing.stages.is_empty() && existing.pipeline.is_none() {
            continue;
        }
        let mut entry = existing.clone();
        let from_stages: Vec<String> = existing.stages.keys().cloned().collect();
        let mut history = entry.history.clone();
        history.push(json!({
            "pipeline": entry.pipeline.clone().or_else(|| old_name.clone()),
            "stages": existing.stages,
            "data": existing.data,
            "mappedAt": instant,
        }));
        let mut stages: BTreeMap<String, String> = BTreeMap::new();
        if !fresh {
            for stage in &from_stages {
                match mapping.get(stage) {
                    Some(to) => {
                        let instant = existing.stages.get(stage).cloned().unwrap_or_default();
                        stages.insert(to.clone(), instant);
                    }
                    None => unmapped.push(format!("{}:{stage}", record.id)),
                }
            }
        }
        let record_id = record.id.clone();
        entry.pipeline = Some(new_pipeline.to_string());
        entry.stages = stages;
        entry.history = history;
        entry.review = None; // the old machine's schedule is invalidated
        moved.push(json!({
            "id": record_id,
            "from": from_stages,
            "to": entry.stages.keys().collect::<Vec<_>>(),
        }));
        set_entry_in(&mut document, &record.id, &entry)?;
    }

    // types.json: the type names the new machine (§3.5: the method is data).
    let mut types_doc = config
        .source_files
        .get("content/types.json")
        .map(|bytes| {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                fail(
                    "types.encoding",
                    "content/types.json",
                    "types.json is not valid UTF-8",
                )
            })?;
            doc_edit::doc_json::parse(text, "content/types.json").map_err(EditError::from)
        })
        .transpose()?
        .ok_or_else(|| {
            fail(
                "types.missing",
                "content/types.json",
                "the plan has no types.json to edit",
            )
        })?;
    doc_edit::doc_json::set(
        &mut types_doc,
        &["types".into(), type_name.into(), "pipeline".into()],
        json!(new_pipeline),
    )?;

    let preview = json!({
        "type": type_name,
        "from": old_name,
        "to": new_pipeline,
        "fresh": fresh,
        "mapping": mapping,
        "moved": moved,
        "unmapped": unmapped,
        "stages": target.stages,
        "files": ["content/types.json", STATE_FILE],
    });
    let changes = vec![
        doc_edit::doc_change(config, "content/types.json", &types_doc),
        doc_edit::doc_change(config, STATE_FILE, &document),
    ];
    doc_edit::finalize_with(config, resources_dir, changes, preview)
        .map(|edit| edit.named(format!("type.setPipeline {type_name} → {new_pipeline}")))
}

/// The stage machine of a type, as data, for `types`-level reads (the Reviews
/// panel's "study method" control).
pub fn pipeline_json(config: &ResolvedConfig, type_name: &str) -> Option<Value> {
    let def: &TypeDef = config.types.get(type_name)?;
    let name = def.pipeline.as_deref()?;
    let pipeline = config
        .rules
        .pipelines
        .as_ref()
        .and_then(|pipelines| pipelines.get(name))?;
    Some(json!({
        "type": type_name,
        "name": name,
        "stages": pipeline.stages,
        "completeWhen": complete_stage(pipeline),
        "proof": pipeline.proof,
        "anchorSkip": pipeline.anchor_skip,
        "scheduler": pipeline.scheduler,
    }))
}

/// Every pipeline the plan declares, as data for a picker.
pub fn pipelines_json(config: &ResolvedConfig) -> Value {
    json!({
        "pipelines": config
            .rules
            .pipelines
            .as_ref()
            .map(|pipelines| pipelines
                .iter()
                .map(|(name, def)| (name.clone(), json!({
                    "stages": def.stages,
                    "completeWhen": complete_stage(def),
                    "scheduler": def.scheduler,
                })))
                .collect::<BTreeMap<_, _>>())
            .unwrap_or_default(),
    })
}
