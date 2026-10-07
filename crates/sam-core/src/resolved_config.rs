//! `ResolvedConfig` (§4.2): the plain value type compiled once by
//! `ConfigStore`'s load → migrate → validate → publish pipeline. Readers take
//! THIS, never the merge pipeline.
//!
//! **Phase 3.** Formula values resolve through the L2 evaluator
//! ([`crate::evaluator`]) and views through [`crate::views`]; this module only
//! derives them for display in the content tree. Formula values are derived,
//! never persisted in `fields` (§3.1).

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config_store::PositionedRecord;
use crate::diagnostic::Diagnostic;
use crate::json_value::{JSONValue, scalar_text};
use crate::model::{AppearanceFile, Record, RulesFile, ShellFile, TypeDef, ViewDef};
use crate::resources::TokenRegister;

pub struct ResolvedConfig {
    pub plan_root: PathBuf,
    pub layer: String,
    pub types: BTreeMap<String, TypeDef>,
    pub views: BTreeMap<String, ViewDef>,
    pub rules: RulesFile,
    pub shell: Option<ShellFile>,
    pub appearance: Option<AppearanceFile>,
    /// Every record with its source position — the single record store. The
    /// reader's iteration order (§4.1 file order) and the writer's positions
    /// are the same collection; there is no second copy.
    pub positioned_records: Vec<PositionedRecord>,
    /// Layer-2 expression ASTs, parsed once at load. Keyed by the expression
    /// text as it appears in `content/types.json`, shared by every record of
    /// every type that declares it (§4.4).
    pub formulas: BTreeMap<String, crate::expression::Expr>,
    /// byte-preservation of untouched lines, clobber verification).
    pub source_files: BTreeMap<String, Vec<u8>>,
    /// The bundled register with `appearance.json` overrides applied (§4.1).
    pub tokens: TokenRegister,
    /// Fingerprint of every authoritative source byte (§4.6).
    pub revision: String,
    /// Advisory findings from load (§3.7: taste and authoring warnings never
    /// block; structural errors were returned before publish).
    pub load_warnings: Vec<Diagnostic>,
}

impl ResolvedConfig {
    /// The records in file order (§4.1). This is the read-side view of
    /// [`Self::positioned_records`].
    pub fn records_iter(&self) -> impl Iterator<Item = &Record> {
        self.positioned_records
            .iter()
            .map(|positioned| &positioned.record)
    }

    pub fn record_count(&self) -> usize {
        self.positioned_records.len()
    }

    pub fn records_by_type(&self) -> BTreeMap<&str, Vec<&Record>> {
        let mut grouped: BTreeMap<&str, Vec<&Record>> = BTreeMap::new();
        for record in self.records_iter() {
            grouped
                .entry(record.type_.as_str())
                .or_default()
                .push(record);
        }
        grouped
    }

    pub fn record(&self, id: &str) -> Option<&Record> {
        self.positioned_records
            .iter()
            .map(|positioned| &positioned.record)
            .find(|record| record.id == id)
    }

    /// The parsed AST for a `formula` field's expression, from the load-time
    /// cache. Hand-built configs (tests) may not have populated it; the
    /// caller parses on a miss.
    pub fn formula(&self, text: &str) -> Option<&crate::expression::Expr> {
        self.formulas.get(text)
    }

    // ── canonical serialization (§4.1) ──────────────────────────────────────

    /// One complete record per line, sorted object keys. `serde_json`'s map is
    /// a `BTreeMap` and its serializer does not escape `/`, so this is §4.1's
    /// canonical form: UTF-8, LF, sorted keys.
    pub fn canonical_line(record: &Record) -> Result<String, String> {
        let value = serde_json::to_value(record).map_err(|error| error.to_string())?;
        serde_json::to_string(&value).map_err(|error| error.to_string())
    }

    pub fn round_trips(record: &Record) -> bool {
        let Ok(line) = Self::canonical_line(record) else {
            return false;
        };
        match serde_json::from_str::<Record>(&line) {
            Ok(back) => back == *record,
            Err(_) => false,
        }
    }

    // ── content tree (`--rendercheck`: content net, not pixel net) ──────────

    pub fn content_tree(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let plan = self
            .plan_root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        lines.push(format!(
            "SAM rendercheck · plan {plan} · revision {}",
            self.revision
        ));

        lines.push(format!("types {}", self.types.len()));
        for (name, def) in &self.types {
            lines.push(describe_type(name, def));
        }

        let grouped = self.records_by_type();
        lines.push(format!("records {}", self.record_count()));
        for (name, records) in &grouped {
            let def = self.types.get(*name);
            let mut header = format!("  {name}({})", records.len());
            if def.is_some_and(|def| def.trackable == Some(true)) {
                header.push_str(" trackable");
            }
            if let Some(pipeline) = def.and_then(|def| def.pipeline.as_deref()) {
                header.push_str(&format!(" pipeline={pipeline}"));
            }
            lines.push(header);
            for record in records {
                lines.push(format!("    {}", describe_record(record, def)));
                if !record.links.is_empty() {
                    let links = record
                        .links
                        .iter()
                        .map(|(key, ids)| format!("{key}→{}", ids.join(",")))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    lines.push(format!("      links: {links}"));
                }
                for field in def
                    .map(|def| def.fields.iter().filter(|field| field.is_formula()))
                    .into_iter()
                    .flatten()
                {
                    if let Some(expr) = &field.expr {
                        let value = crate::evaluator::formula_scalar(self, record, expr)
                            .map(|value| scalar_text(&value.json()))
                            .unwrap_or_else(|| "null".into());
                        lines.push(format!("      formula: {}={value}", field.key));
                    }
                }
            }
        }

        lines.push(format!("views {}", self.views.len()));
        for (name, view) in &self.views {
            lines.push(describe_view(name, view));
        }

        match self.rules.pipelines.as_ref().filter(|p| !p.is_empty()) {
            None => lines.push("rules (none declared)".into()),
            Some(pipelines) => {
                lines.push("rules".into());
                let summary = pipelines
                    .iter()
                    .map(|(name, pipeline)| format!("{name}[{}]", pipeline.stages.join("→")))
                    .collect::<Vec<_>>()
                    .join(" · ");
                lines.push(format!("  pipelines: {summary}"));
                if let Some(scheduler) = &self.rules.scheduler {
                    if let Ok(text) = serde_json::to_string(scheduler) {
                        lines.push(format!("  scheduler: {text}"));
                    }
                }
            }
        }

        match &self.shell {
            Some(shell) => {
                let nav = shell
                    .navigation
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|entry| format!("{}→{}", entry.title, entry.view))
                    .collect::<Vec<_>>()
                    .join(" · ");
                lines.push(format!("shell nav: {nav}"));
                lines.push(format!(
                    "shell commands={} keybindings={}",
                    shell.commands.as_deref().unwrap_or_default().len(),
                    shell.keybindings.as_deref().unwrap_or_default().len()
                ));
            }
            None => lines.push("shell (absent)".into()),
        }

        match &self.appearance {
            Some(appearance) => {
                let overrides = match &appearance.overrides {
                    Some(JSONValue::Object(map)) => map.len(),
                    _ => 0,
                };
                lines.push(format!(
                    "appearance theme={} overrides={overrides}",
                    appearance.theme.as_deref().unwrap_or("-")
                ));
            }
            None => lines.push("appearance (absent)".into()),
        }

        lines
    }
}

fn describe_type(name: &str, def: &TypeDef) -> String {
    let mut head = format!("  {name} {}", def.icon.as_deref().unwrap_or("-"));
    if let Some(parent) = &def.parent {
        head.push_str(&format!(" parent={parent}"));
    }
    if let Some(role) = &def.color_role {
        head.push_str(&format!(" colorRole={role}"));
    }
    if def.trackable == Some(true) {
        head.push_str(" trackable");
    }
    if let Some(pipeline) = &def.pipeline {
        head.push_str(&format!(" pipeline={pipeline}"));
    }
    let fields = def
        .fields
        .iter()
        .map(describe_field)
        .collect::<Vec<_>>()
        .join(" · ");
    format!("{head} · {fields}")
}

fn describe_field(field: &crate::model::FieldDef) -> String {
    let mut text = format!("{}:{}", field.key, field.type_);
    if field.is_relation() {
        text.push_str(&format!(
            "→{}[{}]",
            field.to.as_deref().unwrap_or("?"),
            if field.is_one() { "one" } else { "many" }
        ));
    }
    if let Some(options) = &field.options {
        text.push_str(&format!("[{}]", options.join("|")));
    }
    if let Some(expr) = &field.expr {
        text.push_str(&format!(" expr=\"{expr}\""));
    }
    text
}

fn describe_record(record: &Record, def: Option<&TypeDef>) -> String {
    let skip: std::collections::BTreeSet<&str> = def
        .map(|def| {
            def.fields
                .iter()
                .filter(|field| field.is_formula() || field.is_relation())
                .map(|field| field.key.as_str())
                .collect()
        })
        .unwrap_or_default();
    let parts = record
        .fields
        .iter()
        .filter(|(key, _)| !skip.contains(key.as_str()))
        .map(|(key, value)| format!("{key}={}", scalar_text(value)))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} · {parts}", record.id)
}

fn describe_view(name: &str, view: &ViewDef) -> String {
    // A composed screen declares neither a type nor a layout (COMPOSER §2.1):
    // it is described by its own composition instead.
    let mut text = format!(
        "  {name} · {} · {}",
        view.type_.as_deref().unwrap_or("—"),
        view.layout.as_deref().unwrap_or("—")
    );
    if let Some(components) = &view.components {
        text.push_str(&format!(" components={}", components.len()));
    }
    if let Some(sort) = &view.sort {
        text.push_str(&format!(" sort=\"{sort}\""));
    }
    if let Some(group) = &view.group {
        text.push_str(&format!(" group={group}"));
    }
    if let Some(limit) = view.limit {
        text.push_str(&format!(" limit={limit}"));
    }
    if let Some(columns) = &view.columns {
        text.push_str(&format!(" columns=[{}]", columns.join(",")));
    }
    if let Some(filter) = &view.filter {
        text.push_str(&format!("\n    filter: \"{filter}\""));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_lines_are_key_sorted_and_round_trip() {
        let record = Record {
            schema_version: Some(1),
            id: "t.1".into(),
            type_: "topic".into(),
            fields: BTreeMap::from([
                ("title".to_string(), JSONValue::String("Read".into())),
                ("est".to_string(), JSONValue::from(30)),
            ]),
            links: BTreeMap::from([("course".to_string(), vec!["c.1".to_string()])]),
        };
        let line = ResolvedConfig::canonical_line(&record).expect("serializes");
        assert_eq!(
            line,
            r#"{"fields":{"est":30,"title":"Read"},"id":"t.1","links":{"course":["c.1"]},"schemaVersion":1,"type":"topic"}"#,
            "§4.1: sorted keys, arrays in links, one record per line"
        );
        assert!(ResolvedConfig::round_trips(&record));
    }
}
