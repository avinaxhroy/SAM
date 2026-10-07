//! Decode stage (§4.2 `ConfigStore`: load → explicit migration → validate →
//! publish), ported from `Decode.swift`.
//!
//! Strict JSON gives lexical discipline; the cursor gives paths; this module
//! turns trees into model values. One error stops that document; the store
//! keeps going so every file's findings are reported together.

use crate::diagnostic::DiagnosticError;
use crate::json_cursor::Cursor;
use crate::json_value::JSONValue;
use crate::model::{
    AnchorSkip, AppearanceFile, BlockDef, ComponentDef, FieldDef, Gate, PipelineDef, Proof, Record,
    RulesFile, ShellFile, ShellNavEntry, TypeDef, ViewDef,
};

pub fn field_def(cursor: &Cursor) -> Result<FieldDef, DiagnosticError> {
    Ok(FieldDef {
        key: cursor.string("key")?,
        type_: cursor.string("type")?,
        label: cursor.optional_string("label")?,
        to: cursor.optional_string("to")?,
        cardinality: cursor.optional_string("cardinality")?,
        required: cursor.optional_bool("required")?,
        options: nil_if_empty(cursor.string_array("options")?),
        expr: cursor.optional_string("expr")?,
    })
}

pub fn type_def(cursor: &Cursor) -> Result<TypeDef, DiagnosticError> {
    let fields = cursor
        .array("fields")?
        .iter()
        .map(field_def)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypeDef {
        icon: cursor.optional_string("icon")?,
        parent: cursor.optional_string("parent")?,
        color_role: cursor.optional_string("colorRole")?,
        trackable: cursor.optional_bool("trackable")?,
        pipeline: cursor.optional_string("pipeline")?,
        private: cursor.optional_bool("private")?,
        fields,
    })
}

pub fn types_file(
    cursor: &Cursor,
) -> Result<std::collections::BTreeMap<String, TypeDef>, DiagnosticError> {
    cursor.schema_version(1)?;
    let members = cursor.object("types")?.members()?;
    members
        .iter()
        .map(|(name, child)| Ok((name.clone(), type_def(child)?)))
        .collect()
}

pub fn view_def(cursor: &Cursor) -> Result<ViewDef, DiagnosticError> {
    Ok(ViewDef {
        type_: cursor.optional_string("type")?,
        layout: cursor.optional_string("layout")?,
        filter: cursor.optional_string("filter")?,
        sort: cursor.optional_string("sort")?,
        group: cursor.optional_string("group")?,
        limit: cursor.optional_int("limit")?,
        columns: nil_if_empty(cursor.string_array("columns")?),
        nulls: cursor.optional_string("nulls")?,
        blocks: match cursor.optional_array("blocks")? {
            None => None,
            Some(items) => Some(items.iter().map(block_def).collect::<Result<Vec<_>, _>>()?),
        },
        panel: cursor.optional_string("panel")?,
        components: match cursor.optional_array("components")? {
            None => None,
            Some(items) => Some(
                items
                    .iter()
                    .map(component_def)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        },
    })
}

/// One component of a composed screen (COMPOSER §2.1). The object is
/// forward-open like a block's; `surface` is read leniently **here** so a
/// missing or empty one is the validator's [`crate::validator`] error
/// (`component.surface-missing`) rather than a load failure — the same division
/// of labour a block's shape findings follow.
pub fn component_def(cursor: &Cursor) -> Result<ComponentDef, DiagnosticError> {
    Ok(ComponentDef {
        surface: cursor.optional_string("surface")?.unwrap_or_default(),
        // Any integer decodes. Whether it is a width this build draws is the
        // validator's finding (`component.span-invalid`/`component.span-wide`),
        // never the loader's — the same split `surface` follows.
        span: cursor.optional_int("span")?,
    })
}

/// One block (§3.6, Phase 5). Every member but `kind` is optional, and a member
/// this build does not know is **left where it is**: blocks are forward-open
/// data, and the write path edits the document in place rather than re-encoding
/// it, so an unknown key authored for a newer build survives a round trip.
pub fn block_def(cursor: &Cursor) -> Result<BlockDef, DiagnosticError> {
    Ok(BlockDef {
        kind: cursor.string("kind")?,
        view: cursor.optional_string("view")?,
        title: cursor.optional_string("title")?,
        label: cursor.optional_string("label")?,
        text: cursor.optional_string("text")?,
        expr: cursor.optional_string("expr")?,
        reduce: cursor.optional_string("reduce")?,
        when: cursor.optional_string("when")?,
        blocks: match cursor.optional_array("blocks")? {
            None => None,
            Some(items) => Some(items.iter().map(block_def).collect::<Result<Vec<_>, _>>()?),
        },
        else_blocks: match cursor.optional_array("else")? {
            None => None,
            Some(items) => Some(items.iter().map(block_def).collect::<Result<Vec<_>, _>>()?),
        },
        limit: cursor.optional_int("limit")?,
        x: cursor.optional_string("x")?,
        y: cursor.optional_string("y")?,
        days: cursor.optional_int("days")?,
    })
}

pub fn views_file(
    cursor: &Cursor,
) -> Result<std::collections::BTreeMap<String, ViewDef>, DiagnosticError> {
    cursor.schema_version(1)?;
    let members = cursor.object("views")?.members()?;
    members
        .iter()
        .map(|(name, child)| Ok((name.clone(), view_def(child)?)))
        .collect()
}

pub fn rules_file(cursor: &Cursor) -> Result<RulesFile, DiagnosticError> {
    cursor.schema_version(1)?;
    let mut rules = RulesFile {
        schema_version: Some(1),
        pipelines: None,
        scheduler: None,
        metrics: None,
        lint: None,
        study: None,
        timezone: None,
    };
    if let Some(pipelines) = cursor.optional_object("pipelines")? {
        let mut decoded = std::collections::BTreeMap::new();
        for (name, child) in pipelines.members()? {
            decoded.insert(name, pipeline_def(&child)?);
        }
        rules.pipelines = Some(decoded);
    }
    if let Some(scheduler) = cursor.optional_object("scheduler")? {
        rules.scheduler = Some(scheduler_def(&scheduler)?);
    }
    if let Some(metrics) = cursor.optional_object("metrics")? {
        let mut decoded = std::collections::BTreeMap::new();
        for (name, child) in metrics.members()? {
            decoded.insert(name, metric_def(&child)?);
        }
        rules.metrics = Some(decoded);
    }
    if let Some(study) = cursor.optional_object("study")? {
        rules.study = Some(study.value.clone());
    }
    if let Some(lint) = cursor.optional_object("lint")? {
        rules.lint = Some(lint_file(&lint)?);
    }
    rules.timezone = cursor.optional_string("timezone")?;
    Ok(rules)
}

/// `content/rules.json#/scheduler` (§3.5, D11). The shape is fixed by
/// [`crate::rules::SchedulerDef`]; the *values* are checked here for type, and
/// the validator checks them for meaning (known name, positive intervals).
pub fn scheduler_def(cursor: &Cursor) -> Result<crate::rules::SchedulerDef, DiagnosticError> {
    let name = cursor.string("name")?;
    let fixed = match cursor.optional_object("fixed")? {
        Some(fixed) => Some(crate::rules::FixedSchedule {
            intervals: fixed
                .number_array("intervals")?
                .map(|values| values.into_iter().map(|value| value as i64).collect()),
        }),
        None => None,
    };
    let sm2 = match cursor.optional_object("sm2")? {
        Some(sm2) => Some(crate::rules::Sm2Schedule {
            min_ease: sm2.value.get("minEase").and_then(|value| value.as_f64()),
            first_interval: sm2
                .value
                .get("firstInterval")
                .and_then(|value| value.as_i64()),
        }),
        None => None,
    };
    let fsrs = match cursor.optional_object("fsrs")? {
        Some(fsrs) => Some(crate::rules::FsrsSchedule {
            desired_retention: fsrs
                .value
                .get("desiredRetention")
                .and_then(|value| value.as_f64()),
            weights: fsrs
                .number_array("weights")?
                .map(|values| values.into_iter().map(|value| value as f32).collect()),
            advance_stages: fsrs.optional_bool("advanceStages")?,
        }),
        None => None,
    };
    Ok(crate::rules::SchedulerDef {
        name,
        fixed,
        sm2,
        fsrs,
    })
}

/// `content/rules.json#/lint` — §5's severity overrides and dated waivers.
/// Both are optional; an absent `lint` object means every rule keeps its
/// default severity and nothing is waived.
pub fn lint_file(cursor: &Cursor) -> Result<crate::rules::LintFile, DiagnosticError> {
    let mut lint = crate::rules::LintFile::default();
    if let Some(rules) = cursor.optional_object("rules")? {
        let mut decoded = std::collections::BTreeMap::new();
        for (id, child) in rules.members()? {
            decoded.insert(
                id,
                crate::rules::LintSetting {
                    severity: child.optional_string("severity")?,
                },
            );
        }
        lint.rules = Some(decoded);
    }
    if let Some(waivers) = cursor.optional_array("waivers")? {
        let mut decoded = Vec::new();
        for waiver in waivers {
            decoded.push(crate::rules::LintWaiver {
                id: waiver.string("id")?,
                waived_at: waiver.optional_string("waivedAt")?,
                reason: waiver.optional_string("reason")?,
            });
        }
        lint.waivers = Some(decoded);
    }
    Ok(lint)
}

/// `content/rules.json#/metrics/<name>` — a derived metric: a saved view, an
/// expression and a fold (see [`crate::rules::MetricDef`]).
pub fn metric_def(cursor: &Cursor) -> Result<crate::rules::MetricDef, DiagnosticError> {
    Ok(crate::rules::MetricDef {
        label: cursor.optional_string("label")?.unwrap_or_default(),
        view: cursor.string("view")?,
        expr: cursor.optional_string("expr")?,
        reduce: cursor.optional_string("reduce")?,
        unit: cursor.optional_string("unit")?,
    })
}

pub fn pipeline_def(cursor: &Cursor) -> Result<PipelineDef, DiagnosticError> {
    let mut gates = None;
    if let Some(gates_cursor) = cursor.optional_object("gates")? {
        let mut decoded = std::collections::BTreeMap::new();
        for (stage, gate) in gates_cursor.members()? {
            decoded.insert(
                stage,
                Gate {
                    require: gate.optional_string("require")?,
                },
            );
        }
        gates = Some(decoded);
    }

    let mut proof = None;
    if let Some(proof_cursor) = cursor.optional_object("proof")? {
        proof = Some(Proof {
            applies_to_kinds: nil_if_empty(proof_cursor.string_array("appliesToKinds")?),
            min_problems: proof_cursor.optional_int("minProblems")?,
        });
    }

    let mut anchor_skip = None;
    if let Some(skip) = cursor.optional_object("anchorSkip")? {
        anchor_skip = Some(AnchorSkip {
            allowed: skip.optional_bool("allowed")?,
            require_reason: skip.optional_bool("requireReason")?,
        });
    }

    Ok(PipelineDef {
        stages: cursor.string_array("stages")?,
        gates,
        complete_when: cursor.optional_string("completeWhen")?,
        proof,
        anchor_skip,
        progress: cursor.optional_string("progress")?,
        counter: cursor.optional_bool("counter")?,
        scheduler: cursor.optional_string("scheduler")?,
    })
}

pub fn shell_file(cursor: &Cursor) -> Result<ShellFile, DiagnosticError> {
    cursor.schema_version(1)?;
    let mut shell = ShellFile {
        schema_version: Some(1),
        navigation: None,
        commands: None,
        keybindings: None,
    };

    // Navigation/commands/keybindings stay lenient about their own absence,
    // but a present entry that is malformed is still a diagnostic.
    if let Ok(entries) = cursor.array("navigation") {
        let decoded = entries
            .iter()
            .map(|entry| {
                Ok(ShellNavEntry {
                    title: entry.string("title")?,
                    view: entry.string("view")?,
                    // Absent is the common case (§1 R3): the rail's icon is a
                    // plan's choice, not a requirement.
                    icon: entry.optional_string("icon")?,
                })
            })
            .collect::<Result<Vec<_>, DiagnosticError>>()?;
        shell.navigation = Some(decoded);
    }
    if let Ok(entries) = cursor.array("commands") {
        shell.commands = Some(entries.iter().map(|entry| entry.value.clone()).collect());
    }
    if let Ok(entries) = cursor.array("keybindings") {
        shell.keybindings = Some(entries.iter().map(|entry| entry.value.clone()).collect());
    }
    Ok(shell)
}

pub fn appearance_file(cursor: &Cursor) -> Result<AppearanceFile, DiagnosticError> {
    cursor.schema_version(1)?;
    let mut appearance = AppearanceFile {
        schema_version: Some(1),
        theme: None,
        text_scale: None,
        overrides: None,
    };
    appearance.theme = cursor.optional_string("theme")?;
    appearance.text_scale = cursor.optional_number("textScale")?;
    if let Some(overrides) = cursor.optional_object("overrides")? {
        appearance.overrides = Some(overrides.value.clone());
    }
    Ok(appearance)
}

/// One JSONL line as a record (§3.1). The cursor's file is the source file
/// plus its `#line:N` selector, so a decode diagnostic points at the line
/// without claiming a pointer into a document that has no stable identity.
pub fn record(value: &JSONValue, file: &str, line: u64) -> Result<Record, DiagnosticError> {
    let cursor = Cursor::root(format!("{file}#line:{line}"), value);
    cursor.schema_version(1)?;
    let mut record = Record {
        schema_version: Some(1),
        id: cursor.string("id")?,
        type_: cursor.string("type")?,
        fields: Default::default(),
        links: Default::default(),
    };
    if let Some(fields) = cursor.optional_object("fields")? {
        let JSONValue::Object(map) = fields.value else {
            return Err(fields.mismatch_object());
        };
        // serde_json's map is a BTreeMap-backed newtype without
        // `preserve_order`, so this copy is key-sorted — §4.1's canonical order.
        record.fields = map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
    }
    if let Some(links) = cursor.optional_object("links")? {
        let JSONValue::Object(map) = links.value else {
            return Err(links.mismatch_object());
        };
        for (key, value) in map {
            let JSONValue::Array(targets) = value else {
                return Err(Cursor::new(
                    links.file.clone(),
                    vec!["links".into(), key.clone()],
                    value,
                )
                .mismatch_array());
            };
            let mut ids = Vec::with_capacity(targets.len());
            for (index, target) in targets.iter().enumerate() {
                let JSONValue::String(id) = target else {
                    return Err(Cursor::new(
                        links.file.clone(),
                        vec!["links".into(), key.clone(), index.to_string()],
                        target,
                    )
                    .mismatch("a record id string"));
                };
                ids.push(id.clone());
            }
            record.links.insert(key.clone(), ids);
        }
    }
    Ok(record)
}

fn nil_if_empty(values: Vec<String>) -> Option<Vec<String>> {
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strict_json;

    #[test]
    fn a_type_decodes_with_its_fields_and_relations() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"types":{"topic":{"icon":"bolt","trackable":true,
               "pipeline":"flip","fields":[
                 {"key":"course","type":"relation","to":"course","cardinality":"one"},
                 {"key":"title","type":"text"}]}}}"#,
            "content/types.json",
        )
        .expect("fixture parses");
        let cursor = Cursor::root("content/types.json", &value);
        let types = types_file(&cursor).expect("the type file decodes");
        let topic = types.get("topic").expect("topic is declared");
        assert_eq!(topic.icon.as_deref(), Some("bolt"));
        assert_eq!(topic.allowed_link_keys().len(), 1);
        assert_eq!(topic.field("course").unwrap().to.as_deref(), Some("course"));
    }

    #[test]
    fn a_record_keeps_links_out_of_fields() {
        let value = strict_json::parse(
            r#"{"fields":{"title":"Read"},"id":"t.1","links":{"course":["c.1"]},
                "schemaVersion":1,"type":"topic"}"#,
            "content/records/topic.jsonl",
        )
        .expect("fixture parses");
        let decoded = record(&value, "content/records/topic.jsonl", 1).expect("the record decodes");
        assert_eq!(decoded.id, "t.1");
        assert_eq!(decoded.links["course"], vec!["c.1"]);
        assert!(!decoded.fields.contains_key("course"));
    }

    #[test]
    fn a_record_missing_schema_version_is_refused() {
        let value = strict_json::parse(r#"{"id":"t.1","type":"topic"}"#, "x.jsonl")
            .expect("fixture parses");
        let error = record(&value, "content/records/topic.jsonl", 3)
            .expect_err("schemaVersion is mandatory on every record (D10)");
        assert_eq!(error.diagnostic.code, "schema-version.missing");
    }

    #[test]
    fn a_navigation_entry_decodes_the_rail_icon_the_plan_declares() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"navigation":[
                 {"title":"Today","view":"today.screen","icon":"sun"},
                 {"title":"Notes","view":"notes.screen"}]}"#,
            "content/shell.json",
        )
        .expect("fixture parses");
        let shell =
            shell_file(&Cursor::root("content/shell.json", &value)).expect("the shell decodes");
        let entries = shell.navigation.expect("navigation is declared");
        assert_eq!(entries[0].icon.as_deref(), Some("sun"));
        assert_eq!(
            entries[1].icon, None,
            "an entry that names no icon is a complete entry (§1 R3)"
        );
    }

    #[test]
    fn a_view_with_components_decodes_without_a_type_and_serializes_correctly() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"views":{"morning.screen":{
                 "components":[{"surface":"week-chart","carried":true}]}}}"#,
            "content/views.json",
        )
        .expect("fixture parses");
        let views =
            views_file(&Cursor::root("content/views.json", &value)).expect("the view file decodes");
        let view = views.get("morning.screen").expect("the screen is declared");
        assert_eq!(view.type_, None);
        assert_eq!(view.layout, None);
        let components = view.components.as_ref().expect("components decode");
        assert_eq!(components[0].surface, "week-chart");
        // Serialized by what the engine knows: no invented `type`/`layout`.
        assert_eq!(
            serde_json::to_value(view).unwrap(),
            serde_json::json!({ "components": [{ "surface": "week-chart" }] })
        );
    }

    #[test]
    fn a_component_without_a_surface_still_decodes_for_the_validator_to_name() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"views":{"morning.screen":{"components":[{},{"surface":""}]}}}"#,
            "content/views.json",
        )
        .expect("fixture parses");
        let views =
            views_file(&Cursor::root("content/views.json", &value)).expect("the view file decodes");
        let components = views["morning.screen"]
            .components
            .as_ref()
            .expect("components decode");
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].surface, "");
        assert_eq!(components[1].surface, "");
    }

    #[test]
    fn a_component_span_decodes_as_written_and_the_default_is_never_serialized() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"views":{"morning.screen":{"components":[
                 {"surface":"week-chart","span":1,"carried":true},
                 {"surface":"today-focus"},
                 {"surface":"plan-spine","span":3}]}}}"#,
            "content/views.json",
        )
        .expect("fixture parses");
        let views =
            views_file(&Cursor::root("content/views.json", &value)).expect("the view file decodes");
        let components = views["morning.screen"]
            .components
            .as_ref()
            .expect("components decode");
        assert_eq!(components[0].span, Some(1));
        assert_eq!(
            components[1].span, None,
            "absent span is the default, not zero"
        );
        // Any integer decodes: whether 3 is a width this build draws is the
        // validator's finding (`component.span-wide`).
        assert_eq!(components[2].span, Some(3));
        // Serialized by what the file states: half the row is written, the
        // defaulted whole row is not.
        assert_eq!(
            serde_json::to_value(&views["morning.screen"]).unwrap(),
            serde_json::json!({ "components": [
                { "surface": "week-chart", "span": 1 },
                { "surface": "today-focus" },
                { "surface": "plan-spine", "span": 3 }
            ]})
        );
    }

    #[test]
    fn a_component_span_that_is_not_an_integer_is_a_diagnostic() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"views":{"morning.screen":{"components":[{"surface":"week-chart","span":"half"}]}}}"#,
            "content/views.json",
        )
        .expect("fixture parses");
        let error = views_file(&Cursor::root("content/views.json", &value))
            .expect_err("a span is a number, not a name");
        assert_eq!(error.diagnostic.code, "shape.type-mismatch");
        assert_eq!(
            error.diagnostic.path,
            "content/views.json#/views/morning.screen/components/0/span"
        );
    }

    #[test]
    fn a_non_string_icon_is_a_diagnostic() {
        let value = strict_json::parse(
            r#"{"schemaVersion":1,"navigation":[{"title":"Today","view":"today.screen","icon":7}]}"#,
            "content/shell.json",
        )
        .expect("fixture parses");
        let error = shell_file(&Cursor::root("content/shell.json", &value))
            .expect_err("an icon is a name, not a number");
        assert_eq!(error.diagnostic.code, "shape.type-mismatch");
        assert_eq!(
            error.diagnostic.path,
            "content/shell.json#/navigation/0/icon"
        );
    }
}
