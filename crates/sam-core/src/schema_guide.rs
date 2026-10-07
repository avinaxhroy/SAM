//! `--schema` and `--guide` (§4.7 #8, #9), ported from `SchemaGuide.swift`:
//! two more outputs from ONE source — the same `TypeDef`s the validator, the
//! generated forms (Phase 4) and record batches use. The agent never guesses
//! field names or option values. The guide's worked examples are GENERATED from
//! the active type system and are validated through the Phase 1 loader in
//! self-checks — generated prose can drift, so it is tested.

use std::collections::BTreeMap;
use std::path::Path;

use crate::apply::MAX_BATCH_RECORDS;
use crate::expression::{
    FUNCTIONS, MAX_DEPTH, MAX_EVALUATION_STEPS, MAX_EXPRESSION_LENGTH, MAX_PATH_SEGMENTS,
    MAX_RELATION_HOPS,
};
use crate::json_value::JSONValue;
use crate::model::Record;
use crate::resolved_config::ResolvedConfig;
use crate::transaction::DEFAULT_RETENTION;

pub fn schema_json(config: &ResolvedConfig) -> serde_json::Value {
    let mut types = serde_json::Map::new();
    for (name, def) in &config.types {
        let mut field_values = Vec::new();
        for field in &def.fields {
            let mut entry = serde_json::Map::new();
            entry.insert("key".into(), field.key.clone().into());
            entry.insert("type".into(), field.type_.clone().into());
            entry.insert(
                "cardinality".into(),
                if field.is_one() { "one" } else { "many" }.into(),
            );
            entry.insert("required".into(), field.required.unwrap_or(false).into());
            if let Some(label) = &field.label {
                entry.insert("label".into(), label.clone().into());
            }
            if let Some(to) = &field.to {
                entry.insert("to".into(), to.clone().into());
            }
            if let Some(options) = &field.options {
                entry.insert("options".into(), serde_json::json!(options));
            }
            if let Some(expr) = &field.expr {
                entry.insert("expr".into(), expr.clone().into());
            }
            field_values.push(serde_json::Value::Object(entry));
        }
        let mut def_value = serde_json::Map::new();
        def_value.insert(
            "icon".into(),
            def.icon
                .clone()
                .map(Into::into)
                .unwrap_or(serde_json::Value::Null),
        );
        def_value.insert(
            "parent".into(),
            def.parent
                .clone()
                .map(Into::into)
                .unwrap_or(serde_json::Value::Null),
        );
        def_value.insert(
            "colorRole".into(),
            def.color_role
                .clone()
                .map(Into::into)
                .unwrap_or(serde_json::Value::Null),
        );
        def_value.insert("trackable".into(), def.trackable.unwrap_or(false).into());
        def_value.insert(
            "pipeline".into(),
            def.pipeline
                .clone()
                .map(Into::into)
                .unwrap_or(serde_json::Value::Null),
        );
        def_value.insert("fields".into(), serde_json::Value::Array(field_values));
        types.insert(name.clone(), serde_json::Value::Object(def_value));
    }

    let mut views = serde_json::Map::new();
    for (name, view) in &config.views {
        let mut entry = serde_json::Map::new();
        if let Some(type_) = &view.type_ {
            entry.insert("type".into(), type_.clone().into());
        }
        if let Some(layout) = &view.layout {
            entry.insert("layout".into(), layout.clone().into());
        }
        if let Some(components) = &view.components {
            entry.insert(
                "components".into(),
                serde_json::Value::Array(
                    components
                        .iter()
                        .map(|component| {
                            serde_json::to_value(component).unwrap_or(serde_json::Value::Null)
                        })
                        .collect::<Vec<_>>(),
                ),
            );
        }
        if let Some(filter) = &view.filter {
            entry.insert("filter".into(), filter.clone().into());
        }
        if let Some(sort) = &view.sort {
            entry.insert("sort".into(), sort.clone().into());
        }
        if let Some(group) = &view.group {
            entry.insert("group".into(), group.clone().into());
        }
        if let Some(limit) = view.limit {
            entry.insert("limit".into(), limit.into());
        }
        if let Some(columns) = &view.columns {
            entry.insert("columns".into(), serde_json::json!(columns));
        }
        if let Some(nulls) = &view.nulls {
            entry.insert("nulls".into(), nulls.clone().into());
        }
        views.insert(name.clone(), serde_json::Value::Object(entry));
    }

    let mut pipelines = serde_json::Map::new();
    for (name, pipeline) in config.rules.pipelines.as_ref().into_iter().flatten() {
        let mut entry = serde_json::Map::new();
        entry.insert("stages".into(), serde_json::json!(pipeline.stages));
        if let Some(complete) = &pipeline.complete_when {
            entry.insert("completeWhen".into(), complete.clone().into());
        }
        if let Some(scheduler) = &pipeline.scheduler {
            entry.insert("scheduler".into(), scheduler.clone().into());
        }
        pipelines.insert(name.clone(), serde_json::Value::Object(entry));
    }

    serde_json::json!({
        "schemaVersion": crate::config_store::SCHEMA_VERSION,
        "types": serde_json::Value::Object(types),
        "views": serde_json::Value::Object(views),
        "pipelines": serde_json::Value::Object(pipelines),
        "recordContract": {
            "shape": "{\"schemaVersion\":1,\"id\":\"…\",\"type\":\"…\",\"fields\":{…},\"links\":{…}}",
            "id": "globally unique, stable, non-empty; never encode meaning in it",
            "fields": "non-relation values only; formula and progress values are derived, never persisted",
            "links": "relation values as id arrays, including single-valued relations",
        },
        "limits": {
            "maxExpressionLength": MAX_EXPRESSION_LENGTH,
            "maxDepth": MAX_DEPTH,
            "maxPathSegments": MAX_PATH_SEGMENTS,
            "maxRelationHops": MAX_RELATION_HOPS,
            "maxEvaluationSteps": MAX_EVALUATION_STEPS,
            "maxBatchRecords": MAX_BATCH_RECORDS,
        },
    })
}

/// One worked example per type, generated from the active schema. Relation
/// links point at other examples, so the set is closed and validates
/// whole-plan through the loader (tested in self-checks).
pub fn example_records(config: &ResolvedConfig) -> Vec<Record> {
    fn example_id(type_name: &str) -> String {
        format!("example.{type_name}")
    }

    let mut out = Vec::with_capacity(config.types.len());
    for name in config.types.keys() {
        let def = &config.types[name];
        let mut fields: BTreeMap<String, JSONValue> = BTreeMap::new();
        let mut links: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for field in &def.fields {
            if field.is_formula() || field.type_ == "progress" {
                continue; // derived, never persisted
            }
            match field.type_.as_str() {
                "text" => {
                    fields.insert(
                        field.key.clone(),
                        JSONValue::String(format!("Example {}", field.key)),
                    );
                }
                "longtext" => {
                    fields.insert(
                        field.key.clone(),
                        JSONValue::String(format!("Example {} — long text", field.key)),
                    );
                }
                "number" => {
                    fields.insert(field.key.clone(), JSONValue::from(1));
                }
                "duration" => {
                    fields.insert(field.key.clone(), JSONValue::from(30));
                }
                "date" => {
                    fields.insert(field.key.clone(), JSONValue::String("2026-06-15".into()));
                }
                "daterange" => {
                    fields.insert(
                        field.key.clone(),
                        serde_json::json!({ "start": "2026-06-15", "end": "2026-06-22" }),
                    );
                }
                "bool" => {
                    fields.insert(field.key.clone(), JSONValue::Bool(true));
                }
                "select" => {
                    let option = field.options.as_ref().and_then(|options| options.first());
                    fields.insert(
                        field.key.clone(),
                        JSONValue::String(option.cloned().unwrap_or_default()),
                    );
                }
                "multiSelect" => {
                    let items = field
                        .options
                        .as_ref()
                        .and_then(|options| options.first())
                        .map(|option| vec![JSONValue::String(option.clone())])
                        .unwrap_or_default();
                    fields.insert(field.key.clone(), JSONValue::Array(items));
                }
                "rating" => {
                    fields.insert(field.key.clone(), JSONValue::from(3));
                }
                "url" => {
                    fields.insert(
                        field.key.clone(),
                        JSONValue::String(format!("https://example.com/{name}/{}", field.key)),
                    );
                }
                "relation" => {
                    let target = field.to.clone().unwrap_or_else(|| name.clone());
                    links.insert(field.key.clone(), vec![example_id(&target)]);
                }
                "json" => {
                    fields.insert(field.key.clone(), serde_json::json!({}));
                }
                _ => {}
            }
        }
        if let Some(parent) = &def.parent {
            links.insert(parent.clone(), vec![example_id(parent)]);
        }
        out.push(Record {
            schema_version: Some(crate::config_store::SCHEMA_VERSION as u64),
            id: example_id(name),
            type_: name.clone(),
            fields,
            links,
        });
    }
    out
}

pub fn guide_json(config: &ResolvedConfig, resources_dir: &Path) -> serde_json::Value {
    let examples = example_records(config);
    let example_lines: Vec<String> = examples
        .iter()
        .filter_map(|record| ResolvedConfig::canonical_line(record).ok())
        .collect();

    let mut functions = FUNCTIONS;
    functions.sort_unstable();
    let functions: Vec<String> = functions
        .iter()
        .map(|name| match *name {
            "sum" | "count" | "avg" | "min" | "max" => format!(
                "{name}(relation.path[.field]) — aggregate over a collection; nulls skipped; count is 0 when empty, the others null"
            ),
            "clamp" => "clamp(x, lo, hi) — clamp a number into [lo, hi]".into(),
            "round" => "round(x) — half away from zero".into(),
            "pct" => "pct(a, b) — 0–100; null when b <= 0".into(),
            "daysBetween" => "daysBetween(a, b) — whole days from b to a, YYYY-MM-DD inputs".into(),
            "weekOf" => "weekOf(date) — index of the covering week record, else null".into(),
            "today" => "today() — civil date in the plan timezone".into(),
            "currentTerm" => "currentTerm() — id of the term with the greatest start <= today".into(),
            "complete" => "complete() — true when this record's pipeline reached completeWhen".into(),
            other => other.into(),
        })
        .collect();

    let preset_ids: Vec<String> = crate::config_store::bundled_dir(resources_dir, "presets")
        .map(|dir| {
            std::fs::read_dir(&dir)
                .map(|entries| {
                    let mut ids: Vec<String> = entries
                        .filter_map(|entry| entry.ok())
                        .filter(|entry| entry.path().is_dir())
                        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                        .collect();
                    ids.sort();
                    ids
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let theme_ids: Vec<String> = crate::theme::list(resources_dir)
        .map(|themes| themes.into_iter().map(|theme| theme.id).collect())
        .unwrap_or_default();

    serde_json::json!({
        "authoring": [
            "Records live in content/records/<type>.jsonl, one complete record per line, final newline. SAM writes UTF-8, LF, keys sorted; hand edits keep your bytes until a line changes.",
            "Upsert by id: a repeated identical batch changes no bytes. Ids are stable and opaque — never encode week or course in them.",
            "Relation values live in links as id arrays (single-valued relations too); non-relation values live in fields. formula and progress values are derived — never persist them.",
            "Every record and document carries schemaVersion: 1. Unknown future versions are rejected.",
            format!("Batch limit: {MAX_BATCH_RECORDS} records per apply."),
            format!("Retained transactions: the last {DEFAULT_RETENTION} before-images are kept per plan; set .sam/settings.json#backupRetention to change it."),
        ],
        "expressions": {
            "grammar": "literals, dotted paths, + - * / %, < <= > >= == !=, && || !, ternary, parentheses. No loops, no assignment, no I/O, no user-defined functions.",
            "semantics": [
                "null is missing — distinct from 0 and false; arithmetic with null yields null; == treats null as null (null == null is true).",
                "No implicit string-to-number coercion — it is an error.",
                "&& || and ?: short-circuit; division or modulo by zero is an error; runtime errors evaluate to null with a visible diagnostic.",
                "Paths walk fields, relations, the parent, child types by name, and the reserved id. Example: week.term.id, course.topic.est.",
            ],
            "functions": functions,
            "limits": {
                "maxExpressionLength": MAX_EXPRESSION_LENGTH,
                "maxDepth": MAX_DEPTH,
                "maxPathSegments": MAX_PATH_SEGMENTS,
                "maxRelationHops": MAX_RELATION_HOPS,
                "maxEvaluationSteps": MAX_EVALUATION_STEPS,
            },
        },
        "study": {
            "scheduler": {
                "name": config
                    .rules
                    .scheduler
                    .as_ref()
                    .map(|scheduler| scheduler.name.clone())
                    .unwrap_or_else(|| "fixed".into()),
                "declared": crate::rules::SCHEDULERS,
                "fixedIntervals": config
                    .rules
                    .scheduler
                    .as_ref()
                    .map(|scheduler| scheduler.intervals())
                    .unwrap_or_else(|| crate::rules::DEFAULT_FIXED_INTERVALS.to_vec()),
                "fsrs": "the pinned crate's FSRS-6 model; pin weights at rules.json#/scheduler/fsrs/weights (21 numbers)",
                "ratings": crate::scheduler::RATINGS,
                "dates": "every due date is a civil date in the plan timezone (rules.json#/timezone, a fixed offset); day arithmetic is calendar arithmetic",
            },
            "pipelines": config
                .rules
                .pipelines
                .as_ref()
                .map(|pipelines| {
                    pipelines
                        .iter()
                        .map(|(name, pipeline)| {
                            (
                                name.clone(),
                                serde_json::json!({
                                    "stages": pipeline.stages,
                                    "completeWhen": crate::pipeline::complete_stage(pipeline),
                                    "scheduler": pipeline.scheduler,
                                    "gates": pipeline.gates,
                                    "proof": pipeline.proof,
                                    "anchorSkip": pipeline.anchor_skip,
                                }),
                            )
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default(),
            "metrics": config
                .rules
                .metrics
                .as_ref()
                .map(|metrics| {
                    metrics
                        .iter()
                        .map(|(name, metric)| {
                            let mut shape = serde_json::to_value(metric).unwrap_or_default();
                            if let Some(object) = shape.as_object_mut() {
                                object.insert(
                                    "path".into(),
                                    serde_json::Value::String(format!(
                                        "content/rules.json#/metrics/{name}"
                                    )),
                                );
                            }
                            (name.clone(), shape)
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default(),
            "transitions": [
                "record.advanceStage — record (or backtrack) one stage; `proved` needs --problems >= proof.minProblems, `anchored` needs a logged review or an explicit --reason when anchorSkip is declared.",
                "record.logReview — log a timestamped rating (again | hard | good | easy); the scheduler computes the next due date. On a pipeline that declares anchorSkip, the first review is the anchor's evidence.",
                "type.setPipeline — switch a type's machine with --map old=new per stage or --fresh; prior stages move to `history` and are never re-interpreted.",
                "metric.set — write one derived figure (the Progress panel's numbers) at rules.json#/metrics/<name> with --label, --view, --expr, --reduce and --unit; an empty value drops that key, and the fold lands only if the view exists and the expression compiles.",
                "reviews.due — the queue as data: due, upcoming, waiting (each with `next` and the evidence it `asks` for).",
                "state/state.json#/progress/<id> holds stages, evidence and the review state; formula and progress values are derived from it, never persisted.",
            ],
        },
        "index": [
            "SAM index.rebuild — rebuild the derived SQLite cache (FTS5 search + expression indexes); it never touches source.",
            "SAM index.status — present / current / healthy, with the revision it was built from.",
            "SAM search <words> — full-text search; `--engine auto|index|oracle` forces which one answers, and the two agree.",
            "SAM paths --json reports the index path; JSONL records remain canonical (§4.6).",
        ],
        // Phase 7: how a plan is shared, what a preset is, and where the look
        // lives. Generated from the resources directory so the ids an agent
        // reads are the ids this build actually ships.
        "sharing": {
            "profiles": [
                "SAM profile.export --out <file.samprofile> --json — the schema, views, rules, shell, appearance and public records as one document.",
                "SAM profile.export --out <file.samprofile> --personal --json — also carries private kinds' records and state/state.json.",
                "SAM profile.import --file <file.samprofile> [--name <plan> | --into <directory>] --json — materializes a new plan; the document is validated by the loader before it is visible.",
                "A private kind declares \"private\": true in types.json (or `SAM type.setPrivate <type> --private`). Its records never leave in a default export, and the links that pointed at them are dropped.",
                "A profile carries no executable content: documents and records only, validated by the same whole-plan validator a hand edit meets.",
            ],
            "presets": {
                "available": preset_ids,
                "new": "SAM plan.new --preset <name> --name <plan> --json — a preset is a complete writable copy, never a live layer.",
            },
            "themes": {
                "available": theme_ids,
                "dir": crate::theme::themes_dir(resources_dir).display().to_string(),
                "set": "SAM theme.set --id <theme> --json — pins content/appearance.json#/theme.",
                "resolve": "SAM appearance.resolve --mode light|dark --json — the resolved CSS custom properties the app applies.",
                "overrides": "SAM appearance.setOverride --key <token.path> --value <value> --json · SAM appearance.clearOverride --key <token.path> --json.",
                "textScale": "SAM appearance.setTextScale --value 1.15 --json — scales every fontSize token.",
                "doctor": "SAM design.check --json — §5's lints over resolved colours; findings carry their rule id and JSON path.",
            },
        },
        "examples": example_lines,
        "agentLoop": [
            "1. SAM --schema --json — learn the active type system",
            "2. SAM view <view> --json — read records and the source revision",
            "3. write batch.jsonl OUTSIDE the plan",
            "4. SAM apply batch.jsonl --dry-run --json — validate and preview the diff",
            "5. review the preview",
            "6. SAM apply batch.jsonl --if-revision <hash> --json — commit at the reviewed revision",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config(types: BTreeMap<String, crate::model::TypeDef>) -> ResolvedConfig {
        ResolvedConfig {
            plan_root: std::path::PathBuf::new(),
            layer: "explicit".into(),
            types,
            views: BTreeMap::new(),
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
            formulas: BTreeMap::new(),
            source_files: BTreeMap::new(),
            tokens: crate::resources::TokenRegister {
                version: "test".into(),
                tokens: BTreeMap::new(),
            },
            revision: "test".into(),
            load_warnings: Vec::new(),
        }
    }

    #[test]
    fn examples_are_generated_per_type_and_close_over_their_links() {
        // A minimal schema with a relation and a parent edge.
        let types: BTreeMap<String, crate::model::TypeDef> = serde_json::from_str(
            r#"{
              "a":{"fields":[{"key":"name","type":"text"},{"key":"b","type":"relation","to":"b"}]},
              "b":{"parent":"a","fields":[{"key":"n","type":"number"}]}
            }"#,
        )
        .expect("the schema decodes");
        let config = empty_config(types);
        let examples = example_records(&config);
        assert_eq!(examples.len(), 2);
        let ids: Vec<&str> = examples.iter().map(|record| record.id.as_str()).collect();
        assert_eq!(ids, vec!["example.a", "example.b"]);
        assert_eq!(examples[0].links["b"], vec!["example.b".to_string()]);
        assert_eq!(
            examples[0].fields["name"],
            JSONValue::String("Example name".into())
        );
        assert_eq!(examples[1].links["a"], vec!["example.a".to_string()]);
    }
}
