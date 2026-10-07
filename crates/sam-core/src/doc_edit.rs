//! Phase 4: the mutation layer behind all four surfaces (§4.8).
//!
//! Every UI menu item, palette action, source-pane commit, and CLI verb resolves
//! to these operations. Writes are surgical `serde_json::Value` edits on the
//! documents (preserving unknown keys) and line-preserving edits on JSONL (§4.1),
//! validated whole-plan through the loader before commit (§4.6 step 2, Phase 4 Gate 1).
//!
//! Reuses `apply::rebuild_jsonl` for line preservation, `apply::plan` for record
//! batches, `transaction::validate_by_overlay` for whole-plan validation,
//! and `validator::field_value_problem` for field-value validation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::apply;
use crate::config_store::{self, PositionedRecord};
use crate::decode;
use crate::diagnostic::{Diagnostic, DiagnosticError, escape_pointer_segment};
use crate::dispatch::{DispatchError, ParamValue, Params, flag, optional};
use crate::model::{FieldDef, Record};
use crate::resolved_config::ResolvedConfig;
use crate::strict_json;
use crate::transaction::{self, FileChange, OverlayError, TransactionError};
use crate::validator;

/// The authoritative documents a pane may target by path (§4.3). Records files
/// join them dynamically (they are per-type), which is why this list is not the
/// whole addressable set.
pub const DOCUMENTS: [&str; 6] = [
    "content/types.json",
    "content/views.json",
    "content/rules.json",
    "content/shell.json",
    "content/appearance.json",
    "state/state.json",
];

/// A usage problem in the request itself (wrong parameter count, bad spec) —
/// §4.9 exit 2, never a prompt.
#[derive(Debug)]
pub enum EditError {
    Usage(String),
    Invalid(Vec<Diagnostic>),
    Transaction(TransactionError),
    Io(String),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::Usage(message) | EditError::Io(message) => write!(f, "{message}"),
            EditError::Transaction(error) => write!(f, "{error}"),
            EditError::Invalid(diagnostics) => {
                for diagnostic in diagnostics {
                    writeln!(f, "{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

impl From<DiagnosticError> for EditError {
    fn from(error: DiagnosticError) -> Self {
        EditError::Invalid(vec![error.diagnostic])
    }
}

impl From<TransactionError> for EditError {
    fn from(error: TransactionError) -> Self {
        EditError::Transaction(error)
    }
}

impl From<OverlayError> for EditError {
    fn from(error: OverlayError) -> Self {
        match error {
            OverlayError::Io(message) => EditError::Io(message),
            OverlayError::Invalid(diagnostics) => EditError::Invalid(diagnostics),
        }
    }
}

impl From<EditError> for DispatchError {
    fn from(error: EditError) -> Self {
        match error {
            EditError::Usage(message) => DispatchError::Usage(message),
            EditError::Invalid(diagnostics) => DispatchError::Invalid(diagnostics),
            EditError::Transaction(error) => DispatchError::Transaction(error),
            EditError::Io(message) => DispatchError::Io(message),
        }
    }
}

/// One validated document/record mutation, ready for the transaction engine.
/// `preview` carries the dry-run payload when the caller asked for one.
#[derive(Debug, Clone)]
pub struct PlannedEdit {
    pub changes: Vec<FileChange>,
    pub summary: String,
    pub preview: Option<Value>,
}

impl PlannedEdit {
    pub fn changes_anything(&self) -> bool {
        !self.changes.is_empty()
    }

    pub(crate) fn named(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }
}

// ── params ───────────────────────────────────────────────────────────────────

fn require(params: &Params, name: &str) -> Result<String, EditError> {
    optional(params, name)
        .ok_or_else(|| EditError::Usage(format!("missing required parameter --{name}")))
}

/// A repeatable parameter's values (`--field key:kind` more than once).
fn strings(params: &Params, name: &str) -> Vec<String> {
    match params.get(name) {
        Some(ParamValue::Strings(values)) => values.clone(),
        Some(ParamValue::Str(value)) => vec![value.clone()],
        _ => Vec::new(),
    }
}

/// `<type>.<field>` — never parsed for meaning beyond locating the field.
pub fn parse_spec(spec: &str) -> Result<(String, String), EditError> {
    match spec.split_once('.') {
        Some((type_, key)) if !type_.is_empty() && !key.is_empty() => {
            Ok((type_.to_string(), key.to_string()))
        }
        _ => Err(EditError::Usage(format!(
            "expected a <type>.<field> spec, got \"{spec}\""
        ))),
    }
}

/// Parse one field value from its CLI/palette string form (§3.4). Relations
/// never come through here — they live in `links`.
pub fn parse_field_value(text: &str, field: &FieldDef, at: &str) -> Result<Value, EditError> {
    let bad = |why: String| {
        EditError::Invalid(vec![Diagnostic::error(
            "value.invalid",
            at,
            format!("field \"{}\": {why}", field.key),
        )])
    };
    match field.type_.as_str() {
        "text" | "longtext" | "url" => Ok(Value::String(text.into())),
        "number" => match text.parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(number_value(number)),
            _ => Err(bad(format!("\"{text}\" is not a finite number"))),
        },
        "duration" => match text.parse::<f64>() {
            Ok(number) if number.is_finite() && number >= 0.0 => Ok(number_value(number)),
            _ => Err(bad(format!(
                "\"{text}\" is not a non-negative minute count"
            ))),
        },
        // Format is the validator's business, not the parser's.
        "date" => Ok(Value::String(text.into())),
        "bool" => match text {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(bad(format!(
                "bool fields take true or false, got \"{text}\""
            ))),
        },
        "select" => Ok(Value::String(text.into())),
        "multiSelect" => Ok(Value::Array(
            text.split(',')
                .map(|item| Value::String(item.trim().to_string()))
                .collect(),
        )),
        "rating" => match text.parse::<i64>() {
            Ok(number) if (1..=5).contains(&number) => Ok(json!(number)),
            _ => Err(bad("rating must be an integer 1–5".into())),
        },
        "json" | "daterange" => strict_json::parse(text, at).map_err(EditError::from),
        "relation" => Err(bad(
            "relation values live in links — pass the id as the value".into(),
        )),
        "formula" | "progress" => Err(bad(format!(
            "\"{}\" is derived — never persisted in fields",
            field.key
        ))),
        other => Err(bad(format!("unknown field type \"{other}\""))),
    }
}

/// A whole number stays an integer when it is one (§4.1: `12` is not `12.0`).
pub fn number_value(number: f64) -> Value {
    if number.fract() == 0.0 && number.abs() < 9e15 {
        json!(number as i64)
    } else {
        json!(number)
    }
}

pub fn duration_text(minutes: i64) -> String {
    if minutes % 60 == 0 {
        format!("{}h", minutes / 60)
    } else if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h {}m", minutes / 60, minutes % 60)
    }
}

// ── document JSON: surgical edits, and the canonical pretty form ─────────────

/// SAM-authored documents serialize as pretty JSON: UTF-8, LF, 2-space indent,
/// sorted object keys, scalar arrays on one line, final newline (§4.1). Both
/// doors canonicalize through here, which is exactly why *"equivalent inputs
/// produce the same resolved model … and identical canonical serialization"*
/// (§1.2 #3) holds. `serde_json`'s `Map` is key-sorted by default, so sorting
/// is the codec's property rather than another pass.
pub mod doc_json {
    use super::{Map, Value};

    pub fn parse(text: &str, file: &str) -> Result<Value, crate::diagnostic::DiagnosticError> {
        crate::strict_json::parse(text, file)
    }

    pub fn pretty(value: &Value) -> String {
        let mut out = String::new();
        render(value, 0, &mut out);
        out.push('\n');
        out
    }

    fn render(value: &Value, indent: usize, out: &mut String) {
        match value {
            Value::Object(object) => {
                if object.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                let last = object.len() - 1;
                for (index, (key, child)) in object.iter().enumerate() {
                    out.push_str(&"  ".repeat(indent + 1));
                    out.push_str(&quoted(key));
                    out.push_str(": ");
                    render(child, indent + 1, out);
                    out.push_str(if index == last { "\n" } else { ",\n" });
                }
                out.push_str(&"  ".repeat(indent));
                out.push('}');
            }
            Value::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                // Scalar arrays (options, stages, columns …) stay on one line.
                if items.iter().all(is_scalar) {
                    let rendered: Vec<String> = items.iter().map(scalar).collect();
                    out.push('[');
                    out.push_str(&rendered.join(", "));
                    out.push(']');
                    return;
                }
                out.push_str("[\n");
                let last = items.len() - 1;
                for (index, child) in items.iter().enumerate() {
                    out.push_str(&"  ".repeat(indent + 1));
                    render(child, indent + 1, out);
                    out.push_str(if index == last { "\n" } else { ",\n" });
                }
                out.push_str(&"  ".repeat(indent));
                out.push(']');
            }
            scalar => out.push_str(&scalar.to_string()),
        }
    }

    fn is_scalar(value: &Value) -> bool {
        !matches!(value, Value::Object(_) | Value::Array(_))
    }

    fn scalar(value: &Value) -> String {
        value.to_string()
    }

    fn quoted(text: &str) -> String {
        Value::String(text.to_string()).to_string()
    }

    /// RFC 6901-style segments; arrays are addressed by index.
    pub fn get(value: &Value, path: &[String]) -> Option<Value> {
        let mut cursor = value;
        for segment in path {
            cursor = match cursor {
                Value::Object(object) => object.get(segment)?,
                Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(cursor.clone())
    }

    pub fn set(value: &mut Value, path: &[String], new: Value) -> Result<(), super::EditError> {
        let Some((head, rest)) = path.split_first() else {
            *value = new;
            return Ok(());
        };
        if rest.is_empty() {
            match value {
                Value::Object(object) => {
                    object.insert(head.clone(), new);
                    Ok(())
                }
                _ => Err(super::EditError::Usage(format!(
                    "cannot set {} inside a non-object",
                    path.join("/")
                ))),
            }
        } else {
            let mut child = get(value, std::slice::from_ref(head)).ok_or_else(|| {
                super::EditError::Usage(format!("path {} does not resolve", path.join("/")))
            })?;
            set(&mut child, rest, new)?;
            set(value, std::slice::from_ref(head), child)
        }
    }

    pub fn append(value: &mut Value, path: &[String], new: Value) -> Result<(), super::EditError> {
        let mut items = match get(value, path) {
            Some(Value::Array(items)) => items,
            _ => {
                return Err(super::EditError::Usage(format!(
                    "{} is not an array",
                    path.join("/")
                )));
            }
        };
        items.push(new);
        set(value, path, Value::Array(items))
    }

    /// Insert at a position — the one array edit `column.new --before` needs.
    pub fn insert(
        value: &mut Value,
        path: &[String],
        at: usize,
        new: Value,
    ) -> Result<(), super::EditError> {
        let mut items = match get(value, path) {
            Some(Value::Array(items)) => items,
            _ => {
                return Err(super::EditError::Usage(format!(
                    "{} is not an array",
                    path.join("/")
                )));
            }
        };
        items.insert(at.min(items.len()), new);
        set(value, path, Value::Array(items))
    }

    pub fn remove(value: &mut Value, path: &[String]) -> Result<(), super::EditError> {
        let Some((head, rest)) = path.split_first() else {
            return Ok(());
        };
        if rest.is_empty() {
            if let Value::Object(object) = value {
                object.remove(head);
            }
            return Ok(());
        }
        let Some(mut child) = get(value, std::slice::from_ref(head)) else {
            return Ok(());
        };
        remove(&mut child, rest)?;
        set(value, std::slice::from_ref(head), child)
    }

    /// Empty object/array so a first write into `views.json` creates the shape.
    pub fn with_schema(top: Option<&str>) -> Value {
        let mut root = Map::new();
        root.insert("schemaVersion".into(), Value::from(1));
        if let Some(top) = top {
            root.insert(top.into(), Value::Object(Map::new()));
        }
        Value::Object(root)
    }
}

// ── the shared plumbing every operation uses ─────────────────────────────────

fn doc(config: &ResolvedConfig, rel: &str) -> Result<Option<Value>, EditError> {
    let Some(bytes) = config.source_files.get(rel) else {
        return Ok(None);
    };
    let text = std::str::from_utf8(bytes)
        .map_err(|_| EditError::Usage(format!("{rel} is not valid UTF-8")))?;
    Ok(Some(doc_json::parse(text, rel)?))
}

fn require_doc(config: &ResolvedConfig, rel: &str) -> Result<Value, EditError> {
    doc(config, rel)?.ok_or_else(|| EditError::Usage(format!("{rel} is missing — create it first")))
}

/// Create the intermediate objects a dotted path needs — a write may be the
/// first key under an object that does not exist yet, and an operation that
/// required the object to exist first would make the CLI order-dependent.
/// `separator` and `file` are only how a refusal *names* the path, because the
/// documents speak differently: rules is addressed `…/a/b`, appearance `a.b`.
pub(crate) fn ensure_object_path(
    document: &mut Value,
    segments: &[String],
    separator: &str,
    file: &str,
) -> Result<(), EditError> {
    let parents = segments.len().saturating_sub(1);
    let mut current = document;
    for (index, segment) in segments.iter().take(parents).enumerate() {
        let address = segments[..=index].join(separator);
        if current.get(segment.as_str()).is_none() {
            doc_json::set(current, std::slice::from_ref(segment), json!({}))?;
        }
        let next = current
            .get_mut(segment.as_str())
            .ok_or_else(|| EditError::Usage(format!("cannot descend into \"{address}\"")))?;
        if !next.is_object() {
            return Err(EditError::Usage(format!(
                "\"{address}\" is not an object — fix {file} first"
            )));
        }
        current = next;
    }
    Ok(())
}

/// A `FileChange` replacing one document with its canonical pretty form.
pub(crate) fn doc_change(config: &ResolvedConfig, rel: &str, value: &Value) -> FileChange {
    FileChange::new(
        rel,
        config.source_files.get(rel).cloned(),
        Some(doc_json::pretty(value).into_bytes()),
    )
}

fn file_list(changes: &[FileChange]) -> Vec<String> {
    changes.iter().map(|change| change.path.clone()).collect()
}

/// The dry-run envelope every operation reports: which files, which revision.
pub(crate) fn finalize_with(
    config: &ResolvedConfig,
    resources_dir: &Path,
    changes: Vec<FileChange>,
    preview: Value,
) -> Result<PlannedEdit, EditError> {
    let hashes = config_store::source_file_hashes(&config.plan_root);
    let candidate = transaction::revision_after(&hashes, &changes);
    transaction::validate_by_overlay(&config.plan_root, &changes, resources_dir)?;
    let files = file_list(&changes);
    let mut preview = match preview {
        Value::Object(object) => object,
        other => {
            let mut object = Map::new();
            object.insert("value".into(), other);
            object
        }
    };
    preview.insert("files".into(), json!(files));
    preview.insert("candidateRevision".into(), json!(candidate));
    Ok(PlannedEdit {
        changes,
        summary: String::new(),
        preview: Some(Value::Object(preview)),
    })
}

// ── the cascade: a delete that has to leave the plan valid ───────────────────

/// A structural delete's candidate documents, held as values so the removal can
/// edit several documents at once and **ask the one validator** what it left
/// behind (`UI_PLAN.md` D13's corollary: a structural write previews its
/// consequence and never leaves invisible dependents).
///
/// A kind or a view is referenced by more than the file that declares it: a
/// relation column points at the kind, a block draws the view, a metric folds
/// it, a formula reads through the relation that named it. Removing the thing
/// and stopping is a plan that no longer loads (`relation.target-unknown`,
/// `view.unknown-type`, `shell.unknown-view`, `expr.unknown-root`) — so what the
/// removal left behind is found by `transaction::validate_by_overlay`, the
/// validator every write already passes, and each finding is fixed at the JSON
/// pointer it already carries. Nothing here re-implements a reference rule, and
/// a finding this code does not recognise is reported in the engine's own words
/// rather than swallowed.
pub(crate) struct Cascade {
    docs: BTreeMap<String, Value>,
    before: BTreeMap<String, Option<Vec<u8>>>,
    byte_changes: Vec<FileChange>,
    removed: BTreeMap<String, usize>,
}

/// Fix-and-revalidate rounds. Every round removes at least one finding or the
/// cascade stops, and a round can expose one more layer (a removed block takes
/// its own expressions with it), so this is a safety net, not a budget.
const CASCADE_ROUNDS: usize = 8;

impl Cascade {
    /// Split a delete's planned changes: JSON documents are parsed so the
    /// cascade can edit them; everything else (the JSONL record files) rides
    /// along untouched.
    pub fn start(changes: Vec<FileChange>) -> Result<Self, EditError> {
        let mut cascade = Cascade {
            docs: BTreeMap::new(),
            before: BTreeMap::new(),
            byte_changes: Vec::new(),
            removed: BTreeMap::new(),
        };
        for change in changes {
            let parseable =
                change.path.ends_with(".json") && !change.path.starts_with("content/records/");
            match (parseable, change.after) {
                (true, Some(after)) => {
                    let text = std::str::from_utf8(&after).map_err(|_| {
                        EditError::Usage(format!("{} is not valid UTF-8", change.path))
                    })?;
                    cascade
                        .docs
                        .insert(change.path.clone(), doc_json::parse(text, &change.path)?);
                    cascade.before.insert(change.path, change.before);
                }
                (_, after) => {
                    let path = change.path;
                    let before = change.before;
                    cascade.byte_changes.push(FileChange {
                        path,
                        before,
                        after,
                    });
                }
            }
        }
        Ok(cascade)
    }

    /// The candidate document, created from its schema when the plan has none —
    /// the same rule `type_ops::doc_with_schema` applies to the create path.
    pub fn doc(
        &mut self,
        config: &ResolvedConfig,
        rel: &str,
        top: Option<&str>,
    ) -> Result<&mut Value, EditError> {
        if !self.docs.contains_key(rel) {
            let value = match doc(config, rel)? {
                Some(value) => value,
                None => doc_json::with_schema(top),
            };
            self.before
                .insert(rel.to_string(), config.source_files.get(rel).cloned());
            self.docs.insert(rel.to_string(), value);
        }
        Ok(self.docs.get_mut(rel).expect("inserted above"))
    }

    /// Count what a cascade removed outside the structural pass, for the preview
    /// the UI reads before committing.
    pub fn bump(&mut self, key: &str, by: usize) {
        if by > 0 {
            *self.removed.entry(key.to_string()).or_default() += by;
        }
    }

    fn changes(&self) -> Vec<FileChange> {
        let mut changes = self.byte_changes.clone();
        for (rel, value) in &self.docs {
            let before = self.before.get(rel).cloned().unwrap_or(None);
            let after = doc_json::pretty(value).into_bytes();
            if before.as_deref() == Some(after.as_slice()) {
                continue;
            }
            changes.push(FileChange::new(rel, before, Some(after)));
        }
        changes
    }

    /// What the candidate leaves behind, in the validator's own findings.
    fn findings(
        &self,
        config: &ResolvedConfig,
        resources_dir: &Path,
    ) -> Result<Vec<Diagnostic>, EditError> {
        match transaction::validate_by_overlay(&config.plan_root, &self.changes(), resources_dir) {
            Ok(_) => Ok(Vec::new()),
            Err(OverlayError::Invalid(diagnostics)) => Ok(diagnostics),
            Err(OverlayError::Io(message)) => Err(EditError::Io(message)),
        }
    }

    /// Fix what this code knows how to fix, then let the validator speak again.
    /// Anything left is returned as the engine's own diagnostics, unchanged.
    pub fn finish(
        mut self,
        config: &ResolvedConfig,
        resources_dir: &Path,
        preview: Value,
    ) -> Result<PlannedEdit, EditError> {
        let mut findings = self.findings(config, resources_dir)?;
        for _ in 0..CASCADE_ROUNDS {
            if findings.is_empty() {
                break;
            }
            let mut fixed = 0;
            for finding in &findings {
                if self.fix(finding)? {
                    fixed += 1;
                }
            }
            if fixed == 0 {
                return Err(EditError::Invalid(findings));
            }
            findings = self.findings(config, resources_dir)?;
        }
        if !findings.is_empty() {
            return Err(EditError::Invalid(findings));
        }
        let mut preview = preview;
        if let Value::Object(object) = &mut preview {
            object.insert("removed".into(), json!(self.removed));
        }
        finalize_with(config, resources_dir, self.changes(), preview)
    }

    /// One finding → one removal, addressed by the finding's own pointer.
    /// `false` means this code does not recognise the finding.
    fn fix(&mut self, finding: &Diagnostic) -> Result<bool, EditError> {
        let Some((file, segments)) = split_pointer(&finding.path) else {
            return Ok(false);
        };
        let Some(value) = self.docs.get_mut(file) else {
            return Ok(false);
        };
        let removed = match finding.code.as_str() {
            // An expression whose path is gone: the thing that HELD the
            // expression goes with it — a formula column, a view's sort key, or
            // the block that read it.
            "expr.unknown-root" | "expr.bad-path" => remove_finding_target(value, &segments),
            // A block that drew a view which is gone.
            "view.unknown-view" => remove_finding_target(value, &segments),
            // A view whose column list named a column which is gone.
            "view.unknown-column" => remove_finding_target(value, &segments),
            // A metric folding a view which is gone.
            "metric.unknown-view" => remove_finding_target(value, &segments),
            _ => None,
        };
        if let Some(kind) = removed {
            self.bump(kind, 1);
        }
        Ok(removed.is_some())
    }
}

/// Remove the smallest object that owns a broken reference, from the shapes a
/// delete can break. Returns what was removed, for the preview's counts.
fn remove_finding_target(doc: &mut Value, segments: &[String]) -> Option<&'static str> {
    match segments.first().map(String::as_str) {
        // …/metrics/<name>/… — a metric folding a view that is gone.
        Some("metrics") => remove_member(doc, segments).then_some("metrics"),
        Some("types") => {
            // …/fields/<i>/… — the column whose expression or relation is gone.
            if segments.get(2).map(String::as_str) == Some("fields") {
                return remove_array_entry(doc, &segments[..4]).then_some("columns");
            }
            None
        }
        Some("views") => {
            if segments.get(2).map(String::as_str) == Some("columns") {
                return remove_array_entry(doc, &segments[..4]).then_some("columns");
            }
            if segments.len() == 3 && matches!(segments[2].as_str(), "filter" | "sort" | "group") {
                return remove_member(doc, segments).then_some("viewKeys");
            }
            // A finding inside a block removes that block: it is what read the
            // thing that is gone, and a half-block is not a shape the model has.
            let view = segments.get(1)?.clone();
            let path = block_path_of_pointer(segments)?;
            let pointer = block_pointer(&view, &path);
            remove_array_entry(doc, &pointer).then_some("blocks")
        }
        _ => None,
    }
}

/// The block address a pointer names: `views/<v>/blocks/1/blocks/0/expr` →
/// `1.blocks.0`. The trailing key segments (and the root `blocks` key) fall away.
fn block_path_of_pointer(segments: &[String]) -> Option<Vec<String>> {
    let rest = segments.get(2..)?;
    if rest.first().map(String::as_str) != Some("blocks") {
        return None;
    }
    let mut path: Vec<String> = Vec::new();
    for (index, segment) in rest[1..].iter().enumerate() {
        if index % 2 == 0 {
            segment.parse::<usize>().ok()?;
        } else if segment != "blocks" && segment != "else" {
            break;
        }
        path.push(segment.clone());
    }
    if path.is_empty() || path.len() % 2 == 0 {
        None
    } else {
        Some(path)
    }
}

/// The document pointer of a container's block array: the view's top level when
/// `parent` is empty, else the nested array the parent path names.
fn container_pointer(view: &str, parent: &[String]) -> Vec<String> {
    let mut pointer = vec!["views".to_string(), view.to_string(), "blocks".to_string()];
    pointer.extend_from_slice(parent);
    pointer
}

/// The document pointer of one block, from its address (`0.blocks.2`).
fn block_pointer(view: &str, path: &[String]) -> Vec<String> {
    let mut pointer = container_pointer(view, &path[..path.len().saturating_sub(1)]);
    if let Some(last) = path.last() {
        pointer.push(last.clone());
    }
    pointer
}

/// Write at a pointer that passes through arrays. `doc_json::set` writes object
/// members only, and a block's address goes through the `blocks` array, so this
/// is the one array-aware write the block ops need. The **last** segment may be
/// created (a container's first `blocks` array is exactly that); an intermediate
/// one may not, because a missing parent is a mistake rather than a growth.
/// Returns false when the path does not resolve.
fn set_at(value: &mut Value, path: &[String], new: Value) -> bool {
    let Some((head, rest)) = path.split_first() else {
        *value = new;
        return true;
    };
    match value {
        Value::Object(object) => {
            if object.get(head).is_none() {
                // Only the terminal member is created: `views/<v>/blocks/5/blocks`
                // is a container receiving its first child, while a missing
                // middle segment is a path that does not name anything.
                if rest.is_empty() {
                    object.insert(head.clone(), new);
                    return true;
                }
                return false;
            }
            let child = object.get_mut(head).expect("just checked");
            if rest.is_empty() {
                *child = new;
                true
            } else {
                set_at(child, rest, new)
            }
        }
        Value::Array(items) => {
            let Ok(index) = head.parse::<usize>() else {
                return false;
            };
            let Some(child) = items.get_mut(index) else {
                return false;
            };
            if rest.is_empty() {
                *child = new;
                true
            } else {
                set_at(child, rest, new)
            }
        }
        _ => false,
    }
}

/// A block's address as a parameter spells it: container keys between indices,
/// `0` or `0.blocks.2`. A tree editor needs this because an index alone cannot
/// address a nested block (`UI_SPEC.md` §1 R18).
pub(crate) fn parse_block_path(text: &str) -> Result<Vec<String>, EditError> {
    let segments = parse_address_segments(text)?;
    if segments.is_empty() || segments.len() % 2 == 1 {
        return Ok(segments);
    }
    Err(EditError::Usage(format!(
        "\"{text}\" ends at a container — a block's address names the block, e.g. 0.blocks.2"
    )))
}

/// A container's address: empty (the view's top level) or the path of a block
/// array inside it (`0.blocks`, `1.else`).
pub(crate) fn parse_container_path(text: &str) -> Result<Vec<String>, EditError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let segments = parse_address_segments(trimmed)?;
    if segments.len() % 2 == 0 {
        return Ok(segments);
    }
    Err(EditError::Usage(format!(
        "\"{text}\" ends at a block — a container's address names an array, e.g. 0.blocks"
    )))
}

fn parse_address_segments(text: &str) -> Result<Vec<String>, EditError> {
    let segments: Vec<String> = text
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect();
    for (index, segment) in segments.iter().enumerate() {
        if index % 2 == 0 {
            if segment.parse::<usize>().is_err() {
                return Err(EditError::Usage(format!(
                    "\"{text}\" is not a block address — positions are numbers and containers are \"blocks\" or \"else\", e.g. 0 or 0.blocks.2"
                )));
            }
        } else if segment != "blocks" && segment != "else" {
            return Err(EditError::Usage(format!(
                "\"{text}\" is not a block address — the containers are \"blocks\" and \"else\""
            )));
        }
    }
    Ok(segments)
}

/// Remove every destination that opened one of `views`. Returns how many went.
/// Nothing is written when the plan declares none, so a plan without
/// `shell.json` navigation stays that way.
fn drop_destinations(
    cascade: &mut Cascade,
    config: &ResolvedConfig,
    views: &BTreeSet<String>,
) -> Result<usize, EditError> {
    if views.is_empty() {
        return Ok(0);
    }
    let mut removed = 0usize;
    let doc_value = cascade.doc(config, "content/shell.json", None)?;
    let Some(Value::Array(entries)) = doc_json::get(doc_value, &["navigation".into()]) else {
        return Ok(0);
    };
    let mut kept = Vec::new();
    for entry in &entries {
        let opens_a_removed_view = entry
            .get("view")
            .and_then(Value::as_str)
            .is_some_and(|view| views.contains(view));
        if opens_a_removed_view {
            removed += 1;
        } else {
            kept.push(entry.clone());
        }
    }
    if removed > 0 {
        doc_json::set(doc_value, &["navigation".into()], Value::Array(kept))?;
    }
    Ok(removed)
}

/// Remove one destination: the entry that opens `view`, narrowed by `title` when
/// a view is on the rail more than once. Returns whether one went.
fn drop_destination(
    cascade: &mut Cascade,
    config: &ResolvedConfig,
    view: &str,
    title: Option<&str>,
) -> Result<bool, EditError> {
    let doc_value = cascade.doc(config, "content/shell.json", None)?;
    let Some(Value::Array(entries)) = doc_json::get(doc_value, &["navigation".into()]) else {
        return Err(EditError::Usage(format!(
            "no destination opens \"{view}\" — the plan declares no navigation"
        )));
    };
    let mut removed = false;
    let mut kept = Vec::new();
    for entry in &entries {
        let matches = !removed
            && entry.get("view").and_then(Value::as_str) == Some(view)
            && title
                .is_none_or(|wanted| entry.get("title").and_then(Value::as_str) == Some(wanted));
        if matches {
            removed = true;
        } else {
            kept.push(entry.clone());
        }
    }
    if !removed {
        return Err(EditError::Usage(format!(
            "no destination opens \"{view}\"{} — nothing to remove",
            title
                .map(|title| format!(" with the title \"{title}\""))
                .unwrap_or_default()
        )));
    }
    doc_json::set(doc_value, &["navigation".into()], Value::Array(kept))?;
    Ok(true)
}

/// Put one destination at the end of the rail.
fn add_destination(
    cascade: &mut Cascade,
    config: &ResolvedConfig,
    title: &str,
    view: &str,
) -> Result<(), EditError> {
    let doc_value = cascade.doc(config, "content/shell.json", None)?;
    if doc_json::get(doc_value, &["navigation".into()]).is_none() {
        doc_json::set(doc_value, &["navigation".into()], json!([]))?;
    }
    doc_json::append(
        doc_value,
        &["navigation".into()],
        json!({ "title": title, "view": view }),
    )
}

/// `keepView` is a boolean whose default is **true** — removing a list normally
/// keeps the view. A presence-only flag cannot express `false`, so the terminal
/// spells the destructive case `--discard-view`; the JSON door sends
/// `keepView: false` directly.
fn keep_view(params: &Params) -> bool {
    if flag(params, "discard-view") {
        return false;
    }
    match params.get("keepView") {
        Some(ParamValue::Bool(value)) => *value,
        Some(ParamValue::Str(text)) => text != "false",
        _ => true,
    }
}

/// The rail title a view carries, when it carries one — the name a recreated
/// destination keeps so the kind does not lose the word the student knows.
fn destination_title(config: &ResolvedConfig, view: &str) -> Option<String> {
    config
        .shell
        .as_ref()
        .and_then(|shell| shell.navigation.as_ref())
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.view == view)
                .map(|entry| entry.title.clone())
        })
}

/// `content/views.json#/views/a~1b/blocks/0/expr` → the file and its segments,
/// with RFC 6901's escapes undone.
fn split_pointer(path: &str) -> Option<(&str, Vec<String>)> {
    let (file, pointer) = path.split_once('#')?;
    let segments = pointer
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect();
    Some((file, segments))
}

/// Remove the array element the path's last segment addresses.
fn remove_array_entry(doc: &mut Value, path: &[String]) -> bool {
    let Some((index, parents)) = path.split_last() else {
        return false;
    };
    let Ok(index) = index.parse::<usize>() else {
        return false;
    };
    let Some(Value::Array(mut items)) = doc_json::get(doc, parents) else {
        return false;
    };
    if index >= items.len() {
        return false;
    }
    items.remove(index);
    doc_json::set(doc, parents, Value::Array(items)).is_ok()
}

/// Remove the object member the path addresses.
fn remove_member(doc: &mut Value, path: &[String]) -> bool {
    if doc_json::get(doc, path).is_none() {
        return false;
    }
    doc_json::remove(doc, path).is_ok()
}

/// One records file rebuilt from replacements and deletions, byte-preserving
/// every untouched line (§4.1) — `apply::rebuild_jsonl` is the one that does it.
fn rebuild_records(
    config: &ResolvedConfig,
    rel: &str,
    replacements: &BTreeMap<u64, Record>,
    deletions: &BTreeSet<u64>,
) -> Result<Option<FileChange>, EditError> {
    let before = config.source_files.get(rel).cloned();
    let raw_text = before
        .as_ref()
        .map(|bytes| String::from_utf8_lossy(bytes).to_string());
    let text = apply::rebuild_jsonl(raw_text.as_deref(), replacements, deletions, &[])
        .map_err(EditError::Io)?;
    // A file whose every record went away is removed, not left as a blank line.
    let after = if text.is_empty() || text == "\n" {
        None
    } else {
        Some(text.into_bytes())
    };
    if before == after {
        return Ok(None);
    }
    Ok(Some(FileChange::new(rel, before, after)))
}

/// Auto id for the click path: `<type>.rNNN`, first free (§4.7 rule 2 — the id
/// comes from the engine, never the UI).
pub fn auto_id(config: &ResolvedConfig, type_: &str) -> String {
    let taken: BTreeSet<&str> = config.records_iter().map(|r| r.id.as_str()).collect();
    let mut n = 1;
    while taken.contains(format!("{type_}.r{n}").as_str()) {
        n += 1;
    }
    format!("{type_}.r{n}")
}

// ── records ──────────────────────────────────────────────────────────────────

pub mod record_ops {
    use super::*;

    /// One record from a type, its values and an optional id. `values` is the
    /// parameter vocabulary `record.new` already reads — a field key per entry —
    /// so the single door and the bulk door build records with **one** function
    /// (§4.8: *"the bulk door feeds the simple door"*).
    pub fn build_record(
        config: &ResolvedConfig,
        type_name: &str,
        id: Option<&str>,
        values: &Params,
    ) -> Result<Record, EditError> {
        // An omitted id is the click path's normal case: the engine picks the
        // next free one (§4.7 rule 2 — an id is never the UI's to invent).
        let id = match id.filter(|id| !id.is_empty()) {
            Some(id) => {
                if config.record(id).is_some() {
                    return Err(EditError::Usage(format!(
                        "record \"{id}\" already exists — edit it, or choose another id"
                    )));
                }
                id.to_string()
            }
            None => auto_id(config, type_name),
        };
        let Some(def) = config.types.get(type_name) else {
            return Err(EditError::Usage(format!(
                "unknown type \"{type_name}\" — SAM --schema lists the active types"
            )));
        };
        let mut record = Record {
            // Every record declares the version it was written under (§4.1,
            // D10). A record created here is new, so it carries the current one;
            // omitting it is the `schema-version.missing` the loader refuses.
            schema_version: Some(crate::config_store::SCHEMA_VERSION as u64),
            id: id.clone(),
            type_: type_name.to_string(),
            fields: BTreeMap::new(),
            links: BTreeMap::new(),
        };
        for (name, value) in values {
            let Some(field) = def.field(name) else {
                return Err(EditError::Usage(format!(
                    "type \"{type_name}\" has no field \"{name}\" — SAM --schema lists fields"
                )));
            };
            if field.is_relation() {
                let targets: Vec<String> = match value {
                    ParamValue::Strings(values) => values.clone(),
                    ParamValue::Str(text) => vec![text.clone()],
                    _ => Vec::new(),
                };
                let targets: Vec<String> =
                    targets.into_iter().filter(|id| !id.is_empty()).collect();
                if !targets.is_empty() {
                    record.links.insert(field.key.clone(), targets);
                }
            } else if field.is_formula() || field.type_ == "progress" {
                return Err(EditError::Usage(format!(
                    "field \"{name}\" is derived — it cannot be set"
                )));
            } else {
                let texts = match value {
                    ParamValue::Strings(items) => items.clone(),
                    ParamValue::Str(text) => vec![text.clone()],
                    ParamValue::Bool(boolean) => vec![boolean.to_string()],
                    ParamValue::Bytes(_) => Vec::new(),
                };
                for text in texts {
                    if text.is_empty() && field.type_ != "text" && field.type_ != "longtext" {
                        continue;
                    }
                    let parsed = parse_field_value(&text, field, &format!("fields/{name}"))?;
                    record.fields.insert(field.key.clone(), parsed);
                    break;
                }
            }
        }
        Ok(record)
    }

    /// Plan records as one batch through `apply` — the same validator, the same
    /// transaction path and the same canonical bytes as every other door.
    fn plan_records(
        config: &ResolvedConfig,
        records: &[Record],
        at: &str,
    ) -> Result<apply::PlannedBatch, EditError> {
        let mut text = String::new();
        for record in records {
            text.push_str(&ResolvedConfig::canonical_line(record).map_err(EditError::Io)?);
            text.push('\n');
        }
        apply::plan(config, text.as_bytes(), at).map_err(|error| match error {
            apply::ApplyError::Usage(message) => EditError::Usage(message),
            apply::ApplyError::Validation(diagnostics) => EditError::Invalid(diagnostics),
        })
    }

    /// `<type>.new` / `record.new` — the single door: one record, one batch.
    pub fn new_record(
        config: &ResolvedConfig,
        params: &Params,
    ) -> Result<apply::PlannedBatch, EditError> {
        let type_name = require(params, "type")?;
        let record = build_record(
            config,
            &type_name,
            optional(params, "id").as_deref(),
            &field_values(params),
        )?;
        plan_records(config, &[record], "record.new")
    }

    /// `record.paste` / `<type>.paste` (§4.8). Parses row objects, builds records
    /// using `build_record`, and commits them in a single batch.
    /// Default IDs follow `<type>.paste.<n>`, allocating the first free suffix (§4.7 rule 2).
    pub fn paste_records(
        config: &ResolvedConfig,
        params: &Params,
    ) -> Result<apply::PlannedBatch, EditError> {
        let type_name = require(params, "type")?;
        let raw = require(params, "records")?;
        let parsed: Value = strict_json::parse(&raw, "record.paste").map_err(EditError::from)?;
        let Value::Array(rows) = parsed else {
            return Err(EditError::Usage(
                "record.paste wants \"records\": an array of row objects".into(),
            ));
        };
        if rows.is_empty() {
            return Err(EditError::Usage("record.paste got no rows".into()));
        }
        let mut taken: BTreeSet<String> = config.records_iter().map(|r| r.id.clone()).collect();
        let mut records = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let Value::Object(object) = row else {
                return Err(EditError::Usage(format!(
                    "record.paste row {} is not an object",
                    index + 1
                )));
            };
            let mut values = Params::new();
            let mut explicit: Option<String> = None;
            for (key, item) in object {
                if key == "id" {
                    explicit = item.as_str().map(str::to_string);
                    continue;
                }
                values.insert(key.clone(), cell_value(item));
            }
            let id = explicit
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| paste_id(&type_name, index + 1, &mut taken));
            let record = build_record(config, &type_name, Some(&id), &values)?;
            taken.insert(record.id.clone());
            records.push(record);
        }
        plan_records(config, &records, "record.paste")
    }

    /// The field values of a parameter set: everything that is not the type or
    /// the id. `record.new`'s parameters are field keys by construction.
    fn field_values(params: &Params) -> Params {
        params
            .iter()
            .filter(|(name, _)| name.as_str() != "type" && name.as_str() != "id")
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }

    /// One JSON cell as the value vocabulary `build_record` reads: arrays stay
    /// arrays (a relation is a list of ids, §4.7), scalars keep their kind.
    fn cell_value(item: &Value) -> ParamValue {
        match item {
            Value::Array(items) => ParamValue::Strings(
                items
                    .iter()
                    .map(|item| match item.as_str() {
                        Some(text) => text.to_string(),
                        None => item.to_string(),
                    })
                    .collect(),
            ),
            Value::Bool(flag) => ParamValue::Bool(*flag),
            Value::Null => ParamValue::Str(String::new()),
            other => ParamValue::Str(match other.as_str() {
                Some(text) => text.to_string(),
                None => other.to_string(),
            }),
        }
    }

    /// `<type>.paste.<n>` — first free slot at or after `start`. Pasting twice
    /// adds rows; it never clobbers the first paste.
    fn paste_id(type_name: &str, start: usize, taken: &mut BTreeSet<String>) -> String {
        let mut n = start;
        loop {
            let candidate = format!("{type_name}.paste.{n}");
            if !taken.contains(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Decode one record from source-pane JSON — the same strict decode the
    /// loader runs per JSONL line.
    pub fn pane_record(value: &Value) -> Result<Record, EditError> {
        decode::record(value, "source-pane", 1).map_err(EditError::from)
    }

    pub fn positioned<'c>(config: &'c ResolvedConfig, id: &str) -> Option<&'c PositionedRecord> {
        config.positioned_records.iter().find(|p| p.record.id == id)
    }

    /// `record.setField`: one field (or relation link) on one record.
    ///
    /// `ids` is the relation form the UI dispatches — an id **list**, because
    /// §4.7 lets an id contain a comma and a joined string cannot survive it.
    /// The terminal's `--value a,b` still works: it is split here, and that is
    /// the only place the comma is read.
    pub fn set_field(
        config: &ResolvedConfig,
        id: &str,
        key: &str,
        value: Option<&str>,
        ids: Option<&[String]>,
    ) -> Result<apply::PlannedBatch, EditError> {
        let Some(existing) = positioned(config, id) else {
            return Err(EditError::Usage(format!("no record named \"{id}\"")));
        };
        let record = existing.record.clone();
        let Some(def) = config.types.get(&record.type_) else {
            return Err(EditError::Usage(format!(
                "record \"{id}\" has unknown type \"{}\"",
                record.type_
            )));
        };
        let Some(field) = def.field(key) else {
            return Err(EditError::Usage(format!(
                "type \"{}\" has no field \"{key}\"",
                record.type_
            )));
        };
        let mut updated = record.clone();
        if field.is_relation() {
            let targets: Vec<String> = match ids {
                Some(ids) => ids
                    .iter()
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .collect(),
                None => value
                    .unwrap_or_default()
                    .split(',')
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .collect(),
            };
            if targets.is_empty() {
                updated.links.remove(key);
            } else {
                updated.links.insert(key.to_string(), targets);
            }
        } else if field.is_formula() || field.type_ == "progress" {
            return Err(EditError::Usage(format!(
                "field \"{key}\" is derived — it cannot be set"
            )));
        } else if let Some(text) = value.filter(|text| !text.is_empty() && *text != "null") {
            let parsed =
                parse_field_value(text, field, &format!("{}#{id}/fields/{key}", existing.file))?;
            updated.fields.insert(key.to_string(), parsed);
        } else {
            updated.fields.remove(key); // null clears the field
        }
        let line = ResolvedConfig::canonical_line(&updated).map_err(EditError::Io)?;
        apply::plan(config, format!("{line}\n").as_bytes(), "record.setField").map_err(|error| {
            match error {
                apply::ApplyError::Usage(message) => EditError::Usage(message),
                apply::ApplyError::Validation(diagnostics) => EditError::Invalid(diagnostics),
            }
        })
    }

    /// `record.delete`: reject · unlink · cascade, previewed (§4.6: *"Reject a
    /// delete with inbound links unless the caller explicitly selects a
    /// previewed cascade or unlink policy"*).
    pub fn delete(
        config: &ResolvedConfig,
        resources_dir: &Path,
        id: &str,
        policy: &str,
        dry_run: bool,
    ) -> Result<PlannedEdit, EditError> {
        let Some(target) = positioned(config, id) else {
            return Err(EditError::Usage(format!("no record named \"{id}\"")));
        };
        let inbound = |rid: &str| -> Vec<&PositionedRecord> {
            config
                .positioned_records
                .iter()
                .filter(|p| {
                    p.record.id != rid
                        && p.record
                            .links
                            .values()
                            .any(|targets| targets.iter().any(|t| t == rid))
                })
                .collect()
        };

        let mut delete_ids: BTreeSet<String> = BTreeSet::from([id.to_string()]);
        let mut unlink_records: Vec<&PositionedRecord> = Vec::new();
        match policy {
            "reject" | "" => {
                let inbound_records = inbound(id);
                if !inbound_records.is_empty() {
                    let mut names: Vec<&str> = inbound_records
                        .iter()
                        .map(|p| p.record.id.as_str())
                        .collect();
                    names.sort_unstable();
                    return Err(EditError::Invalid(vec![Diagnostic::error(
                        "record.delete-inbound",
                        format!("{}#{id}", target.file),
                        format!(
                            "record \"{id}\" has inbound links from {} — pass --policy unlink or --policy cascade",
                            names.join(", ")
                        ),
                    )]));
                }
            }
            "unlink" => unlink_records = inbound(id),
            "cascade" => {
                // Transitive: deleting a record deletes records that link to it.
                let mut queue = vec![id.to_string()];
                while let Some(current) = queue.pop() {
                    for p in inbound(&current) {
                        if delete_ids.insert(p.record.id.clone()) {
                            queue.push(p.record.id.clone());
                        }
                    }
                }
                unlink_records.clear();
            }
            other => {
                return Err(EditError::Usage(format!(
                    "policy must be unlink or cascade, got \"{other}\""
                )));
            }
        }

        #[derive(Default)]
        struct Work {
            replacements: BTreeMap<u64, Record>,
            deletions: BTreeSet<u64>,
        }
        let mut work: BTreeMap<String, Work> = BTreeMap::new();
        for p in &config.positioned_records {
            if delete_ids.contains(&p.record.id) {
                work.entry(p.file.clone())
                    .or_default()
                    .deletions
                    .insert(p.line);
            }
        }
        for p in &unlink_records {
            let mut edited = p.record.clone();
            for (key, targets) in edited.links.clone() {
                if targets.iter().any(|t| t == id) {
                    let kept: Vec<String> = targets.into_iter().filter(|t| t != id).collect();
                    if kept.is_empty() {
                        edited.links.remove(&key);
                    } else {
                        edited.links.insert(key, kept);
                    }
                }
            }
            work.entry(p.file.clone())
                .or_default()
                .replacements
                .insert(p.line, edited);
        }

        let mut changes = Vec::new();
        for (rel, job) in &work {
            if let Some(change) = rebuild_records(config, rel, &job.replacements, &job.deletions)? {
                changes.push(change);
            }
        }

        let preview = json!({
            "delete": delete_ids.iter().cloned().collect::<Vec<String>>(),
            "unlink": unlink_records.iter().map(|p| p.record.id.clone()).collect::<Vec<String>>(),
        });
        if dry_run {
            return Ok(PlannedEdit {
                changes,
                summary: format!("record.delete {id} (dry-run)"),
                preview: Some(preview),
            });
        }
        let suffix = if policy == "cascade" {
            " (cascade)"
        } else {
            ""
        };
        finalize_with(config, resources_dir, changes, preview)
            .map(|edit| edit.named(format!("record.delete {id}{suffix}")))
    }

    /// `record.defer <ids…> --date YYYY-MM-DD [--field focus] [--reviews]` (F13, `UX_FLOWS.md` §3.4).
    /// Updates the record date field, and optionally reschedules `due` in `state/state.json`
    /// without modifying review history.
    pub fn defer(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let ids = strings(params, "ids");
        if ids.is_empty() {
            return Err(EditError::Usage(
                "record.defer takes at least one record id".into(),
            ));
        }
        let date = require(params, "date")?;
        if crate::scheduler::parse_date(&date).is_none() {
            return Err(EditError::Usage(format!(
                "\"{date}\" is not a date — use YYYY-MM-DD"
            )));
        }
        let field = optional(params, "field")
            .filter(|field| !field.is_empty())
            .unwrap_or_else(|| "focus".into());

        let mut work: BTreeMap<String, BTreeMap<u64, Record>> = BTreeMap::new();
        let mut moved = 0usize;
        let mut without_field: Vec<String> = Vec::new();
        for id in &ids {
            let Some(sitting) = positioned(config, id) else {
                return Err(EditError::Usage(format!("no record named \"{id}\"")));
            };
            let Some(def) = config.types.get(&sitting.record.type_) else {
                return Err(EditError::Usage(format!(
                    "record \"{id}\" has unknown type \"{}\"",
                    sitting.record.type_
                )));
            };
            let Some(field_def) = def.field(&field) else {
                without_field.push(id.clone());
                continue;
            };
            if field_def.type_ != "date" {
                return Err(EditError::Usage(format!(
                    "field \"{field}\" on {} is {} — defer writes a date",
                    sitting.record.type_, field_def.type_
                )));
            }
            let mut updated = sitting.record.clone();
            updated.fields.insert(field.clone(), json!(date));
            work.entry(sitting.file.clone())
                .or_default()
                .insert(sitting.line, updated);
            moved += 1;
        }
        let mut changes = Vec::new();
        for (rel, replacements) in &work {
            if let Some(change) = rebuild_records(config, rel, replacements, &BTreeSet::new())? {
                changes.push(change);
            }
        }

        let mut reviews = 0usize;
        if flag(params, "reviews") {
            let mut document = crate::pipeline::state_document(config)?;
            for id in &ids {
                let mut entry = crate::pipeline::entry_in(&document, id);
                let Some(Value::Object(review)) = &mut entry.review else {
                    continue;
                };
                review.insert("due".into(), json!(date));
                crate::pipeline::set_entry_in(&mut document, id, &entry)?;
                reviews += 1;
            }
            if reviews > 0 {
                changes.push(doc_change(config, "state/state.json", &document));
            }
        }

        if moved == 0 && reviews == 0 {
            return Err(EditError::Invalid(vec![Diagnostic::error(
                "record.defer-nothing",
                "content/records/<kind>.jsonl",
                format!(
                    "none of the {} named record(s) carries a \"{field}\" date and none has a scheduled review — nothing would move",
                    ids.len()
                ),
            )]));
        }
        let preview = json!({
            "ids": ids,
            "date": date,
            "field": field,
            "moved": moved,
            "withoutField": without_field,
            "reviews": reviews,
        });
        finalize_with(config, resources_dir, changes, preview).map(|edit| {
            edit.named(format!(
                "moved {} record{} to {date}",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            ))
        })
    }
}

// ── types and lists ──────────────────────────────────────────────────────────

pub mod type_ops {
    use super::*;

    pub fn slug(name: &str) -> String {
        let mut out = String::new();
        for character in name.chars() {
            if character.is_ascii_alphanumeric() {
                out.push(character.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
        while out.contains("--") {
            out = out.replace("--", "-");
        }
        out.trim_matches('-').to_string()
    }

    /// A type id is a safe path segment and a stable identity (§4.6 filesystem
    /// safety: *"paths come from validated document roles"*).
    pub fn safe_type_id(id: &str) -> bool {
        !id.is_empty()
            && id != "."
            && id != ".."
            && !id.contains('/')
            && !id.contains('\\')
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    }

    pub fn doc_with_schema(
        config: &ResolvedConfig,
        rel: &str,
        top: Option<&str>,
    ) -> Result<Value, EditError> {
        match doc(config, rel)? {
            Some(value) => Ok(value),
            None => Ok(doc_json::with_schema(top)),
        }
    }

    /// `type.new`: one transaction writes the TypeDef, a default ViewDef and the
    /// shell entry (§6 Phase 4: *"Add list can create a kind"* — and a kind
    /// without a view and a way to reach it is a junk drawer, §3.6).
    pub fn new_type(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let id = optional(params, "id")
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| slug(&name));
        if !safe_type_id(&id) {
            return Err(EditError::Usage(format!(
                "type id \"{id}\" must be letters, numbers, '-' and '.'"
            )));
        }
        if config.types.contains_key(&id) {
            return Err(EditError::Usage(format!("type \"{id}\" already exists")));
        }
        let mut changes = Vec::new();

        let mut fields = Vec::new();
        for spec in strings(params, "field") {
            let parts: Vec<&str> = spec.splitn(3, ':').collect();
            if parts.len() < 2 {
                return Err(EditError::Usage(format!(
                    "--field takes key:kind[:opt1|opt2…], got \"{spec}\""
                )));
            }
            let mut entry = Map::new();
            entry.insert("key".into(), json!(parts[0]));
            entry.insert("type".into(), json!(parts[1]));
            if parts.len() == 3 {
                entry.insert(
                    "options".into(),
                    json!(parts[2].split('|').collect::<Vec<&str>>()),
                );
            }
            fields.push(Value::Object(entry));
        }
        let mut entry = Map::new();
        entry.insert("fields".into(), Value::Array(fields));
        if let Some(icon) = optional(params, "icon") {
            entry.insert("icon".into(), json!(icon));
        }
        if let Some(parent) = optional(params, "parent") {
            entry.insert("parent".into(), json!(parent));
        }
        if flag(params, "trackable") {
            entry.insert("trackable".into(), json!(true));
        }
        if let Some(pipeline) = optional(params, "pipeline") {
            entry.insert("pipeline".into(), json!(pipeline));
        }
        let mut types_doc = require_doc(config, "content/types.json")?;
        doc_json::set(
            &mut types_doc,
            &["types".into(), id.clone()],
            Value::Object(entry),
        )?;
        changes.push(doc_change(config, "content/types.json", &types_doc));

        let mut views_doc = doc_with_schema(config, "content/views.json", Some("views"))?;
        doc_json::set(
            &mut views_doc,
            &["views".into(), format!("{id}.all")],
            json!({ "type": id, "layout": "table" }),
        )?;
        changes.push(doc_change(config, "content/views.json", &views_doc));

        let mut shell_doc = doc_with_schema(config, "content/shell.json", None)?;
        if doc_json::get(&shell_doc, &["navigation".into()]).is_none() {
            doc_json::set(&mut shell_doc, &["navigation".into()], json!([]))?;
        }
        doc_json::append(
            &mut shell_doc,
            &["navigation".into()],
            json!({ "title": name, "view": format!("{id}.all") }),
        )?;
        changes.push(doc_change(config, "content/shell.json", &shell_doc));

        let preview = json!({ "type": id, "view": format!("{id}.all") });
        finalize_with(config, resources_dir, changes, preview)
            .map(|edit| edit.named(format!("type.new {id}")))
    }

    /// `type.setPrivate <type> --private|--public` — the private-record rule
    /// (§6 Phase 7). Records of a private kind are personal: a shared profile
    /// leaves them out, and the JSON says so where a student can read it.
    pub fn set_private(
        config: &ResolvedConfig,
        resources_dir: &Path,
        type_id: &str,
        private: bool,
    ) -> Result<PlannedEdit, EditError> {
        if !config.types.contains_key(type_id) {
            let known: Vec<&String> = config.types.keys().collect();
            return Err(EditError::Usage(format!(
                "no type \"{type_id}\" — declared: {}",
                known
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        let mut types_doc = require_doc(config, "content/types.json")?;
        doc_json::set(
            &mut types_doc,
            &[
                "types".to_string(),
                type_id.to_string(),
                "private".to_string(),
            ],
            json!(private),
        )?;
        let changes = vec![doc_change(config, "content/types.json", &types_doc)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "type": type_id, "private": private }),
        )
        .map(|edit| edit.named(format!("type.setPrivate {type_id} {private}")))
    }

    /// `type.delete <id> [--mode records|schema]` (§6.2 U6; `UI_SPEC.md` §4 E2).
    /// `schema` mode errors if records exist; `records` mode deletes them.
    /// Related foreign keys, views, and destinations are pruned via [`Cascade`].
    pub fn delete(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        if !config.types.contains_key(&name) {
            let known: Vec<&str> = config.types.keys().map(String::as_str).collect();
            return Err(EditError::Usage(format!(
                "no kind named \"{name}\" — declared: {}",
                known.join(", ")
            )));
        }
        let mode = optional(params, "mode").unwrap_or_else(|| "records".into());
        if mode != "records" && mode != "schema" {
            return Err(EditError::Usage(format!(
                "mode must be \"records\" or \"schema\", got \"{mode}\""
            )));
        }
        let records: Vec<&PositionedRecord> = config
            .positioned_records
            .iter()
            .filter(|p| p.record.type_ == name)
            .collect();
        if mode == "schema" && !records.is_empty() {
            return Err(EditError::Invalid(vec![Diagnostic::error(
                "type.has-records",
                format!(
                    "content/types.json#/types/{}",
                    escape_pointer_segment(&name)
                ),
                format!(
                    "kind \"{name}\" still holds {} record(s) — \"schema\" will not take them with it; \"records\" deletes them with the kind",
                    records.len()
                ),
            )]));
        }

        // 1 · The records, and every link that pointed at them: a link to a
        // record that no longer exists is `record.dangling-link`.
        let gone: BTreeSet<String> = records.iter().map(|p| p.record.id.clone()).collect();
        let mut replaced: BTreeMap<String, BTreeMap<u64, Record>> = BTreeMap::new();
        let mut deleted: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
        let mut unlinked = 0usize;
        for p in &config.positioned_records {
            if gone.contains(&p.record.id) {
                deleted.entry(p.file.clone()).or_default().insert(p.line);
                continue;
            }
            let mut edited = p.record.clone();
            let before: usize = edited.links.values().map(Vec::len).sum();
            for (key, targets) in edited.links.clone() {
                let kept: Vec<String> = targets
                    .into_iter()
                    .filter(|target| !gone.contains(target))
                    .collect();
                if kept.is_empty() {
                    edited.links.remove(&key);
                } else {
                    edited.links.insert(key, kept);
                }
            }
            let after: usize = edited.links.values().map(Vec::len).sum();
            if after != before {
                unlinked += before - after;
                replaced
                    .entry(p.file.clone())
                    .or_default()
                    .insert(p.line, edited);
            }
        }
        let files: BTreeSet<String> = deleted.keys().chain(replaced.keys()).cloned().collect();
        let mut changes = Vec::new();
        for rel in &files {
            let no_replacements = BTreeMap::new();
            let no_deletions = BTreeSet::new();
            if let Some(change) = rebuild_records(
                config,
                rel,
                replaced.get(rel).unwrap_or(&no_replacements),
                deleted.get(rel).unwrap_or(&no_deletions),
            )? {
                changes.push(change);
            }
        }

        let mut cascade = Cascade::start(changes)?;

        // 2 · The kinds that pointed at it: a relation column whose target is
        // gone is `relation.target-unknown` and a parent edge to it is
        // `types.parent-unknown` — both are fields, and both go with the kind.
        let mut relations = 0usize;
        let mut parents = 0usize;
        {
            let doc_value = cascade.doc(config, "content/types.json", Some("types"))?;
            let mut types =
                doc_json::get(doc_value, &["types".into()]).unwrap_or_else(|| json!({}));
            if let Value::Object(object) = &mut types {
                for (other, entry) in object.iter_mut() {
                    if other == &name {
                        continue;
                    }
                    let Value::Object(definition) = entry else {
                        continue;
                    };
                    if definition.get("parent").and_then(Value::as_str) == Some(name.as_str()) {
                        definition.remove("parent");
                        parents += 1;
                    }
                    if let Some(Value::Array(fields)) = definition.get_mut("fields") {
                        let before = fields.len();
                        fields.retain(|field| {
                            !(field.get("type").and_then(Value::as_str) == Some("relation")
                                && field.get("to").and_then(Value::as_str) == Some(name.as_str()))
                        });
                        relations += before - fields.len();
                    }
                }
                object.remove(&name);
            }
            doc_json::set(doc_value, &["types".into()], types)?;
        }

        // 3 · Its views, and the destinations that opened them — a view of a
        // missing kind is `view.unknown-type` and a destination opening a
        // missing view is `shell.unknown-view`.
        let doomed: BTreeSet<String> = config
            .views
            .iter()
            .filter(|(_, view)| view.type_.as_deref() == Some(name.as_str()))
            .map(|(view_name, _)| view_name.clone())
            .collect();
        let mut views = 0usize;
        {
            let doc_value = cascade.doc(config, "content/views.json", Some("views"))?;
            let mut all = doc_json::get(doc_value, &["views".into()]).unwrap_or_else(|| json!({}));
            if let Value::Object(object) = &mut all {
                for view_name in &doomed {
                    if object.remove(view_name).is_some() {
                        views += 1;
                    }
                }
            }
            doc_json::set(doc_value, &["views".into()], all)?;
        }
        let sidebar = drop_destinations(&mut cascade, config, &doomed)?;

        // 4 · Everything the removal left behind is the validator's to find: a
        // block that drew one of the views, a metric that folded one, a formula
        // or a sort key that read through a removed relation.
        let preview = json!({
            "kind": name,
            "mode": mode,
            "records": records.len(),
            "unlinked": unlinked,
            "relations": relations,
            "parents": parents,
            "views": views,
            "sidebar": sidebar,
        });
        cascade.finish(config, resources_dir, preview).map(|edit| {
            edit.named(if mode == "records" {
                format!("removed the kind \"{name}\" with its records")
            } else {
                format!("removed the kind \"{name}\" (schema only)")
            })
        })
    }
}

pub mod list_ops {
    use super::*;

    /// `list.new`: a saved view of an EXISTING kind, plus its shell entry. Never
    /// duplicates the schema — the view references the type (§4.8 Principle 1).
    ///
    /// With **no type** (COMPOSER §2.2) it creates a composed screen instead:
    /// `{ "components": [] }` and the sidebar entry, so the student's new screen
    /// opens empty and ready for its first component. `--icon` writes the entry's
    /// glyph in the same transaction.
    pub fn new_list(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        // An absent OR empty `--type` is "no kind" — a composed screen.
        let type_name = optional(params, "type").filter(|type_| !type_.is_empty());
        if let Some(type_name) = &type_name {
            if !config.types.contains_key(type_name) {
                return Err(EditError::Usage(format!(
                    "unknown type \"{type_name}\" — type.new creates kinds"
                )));
            }
        }
        let id = optional(params, "id")
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| type_ops::slug(&name));
        let layout = optional(params, "layout").unwrap_or_else(|| "table".into());
        let icon = optional(params, "icon").filter(|icon| !icon.is_empty());
        let view_id = if id.contains('.') {
            id.clone()
        } else {
            format!("{id}.all")
        };
        if config.views.contains_key(&view_id) {
            return Err(EditError::Usage(format!(
                "a view named \"{view_id}\" already exists"
            )));
        }
        let mut changes = Vec::new();
        let mut views_doc = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let definition = match &type_name {
            Some(type_name) => json!({ "type": type_name, "layout": layout }),
            // A composed screen: no query, no layout — an empty composition.
            None => json!({ "components": [] }),
        };
        doc_json::set(
            &mut views_doc,
            &["views".into(), view_id.clone()],
            definition,
        )?;
        changes.push(doc_change(config, "content/views.json", &views_doc));

        let mut shell_doc = type_ops::doc_with_schema(config, "content/shell.json", None)?;
        if doc_json::get(&shell_doc, &["navigation".into()]).is_none() {
            doc_json::set(&mut shell_doc, &["navigation".into()], json!([]))?;
        }
        let mut entry = json!({ "title": name, "view": view_id });
        if let Some(icon) = &icon
            && let Value::Object(object) = &mut entry
        {
            object.insert("icon".into(), json!(icon));
        }
        doc_json::append(&mut shell_doc, &["navigation".into()], entry)?;
        changes.push(doc_change(config, "content/shell.json", &shell_doc));

        finalize_with(config, resources_dir, changes, json!({ "view": view_id }))
            .map(|edit| edit.named(format!("list.new {view_id}")))
    }

    /// `list.set <view> [--title T] [--icon I] [--clear-icon]` — what a
    /// destination is called and what it draws (U6b: the icon is data, and data
    /// with no UI door is the D11 gap this closes). The entry is found by the
    /// view it opens, narrowed by `--title` when one view is on the rail twice.
    pub fn set(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let view = require(params, "view")?;
        let title = optional(params, "title");
        let icon = optional(params, "icon");
        let clear_icon = flag(params, "clear-icon");
        if title.is_none() && icon.is_none() && !clear_icon {
            return Err(EditError::Usage(
                "list.set changes a destination's --title or --icon; nothing was named".into(),
            ));
        }
        if title
            .as_deref()
            .is_some_and(|title| title.trim().is_empty())
        {
            return Err(EditError::Usage(
                "a destination's title cannot be empty — list.delete removes it".into(),
            ));
        }
        let mut cascade = Cascade::start(Vec::new())?;
        let changed = {
            let doc_value = cascade.doc(config, "content/shell.json", None)?;
            let Some(Value::Array(entries)) = doc_json::get(doc_value, &["navigation".into()])
            else {
                return Err(EditError::Usage(format!(
                    "no destination opens \"{view}\" — the plan declares no navigation"
                )));
            };
            // The entry is found by the view it opens; `--find-title` narrows it
            // when one view is on the rail twice. The new title is a separate
            // parameter — using `--title` as the lookup would make a rename
            // impossible (it would only ever "find" its own old value).
            let find_title = optional(params, "find-title");
            let at = entries.iter().position(|entry| {
                entry.get("view").and_then(Value::as_str) == Some(view.as_str())
                    && find_title.as_deref().is_none_or(|wanted| {
                        entry.get("title").and_then(Value::as_str) == Some(wanted)
                    })
            });
            let Some(at) = at else {
                return Err(EditError::Usage(format!(
                    "no destination opens \"{view}\"{} — nothing was set",
                    find_title
                        .map(|title| format!(" with the title \"{title}\""))
                        .unwrap_or_default()
                )));
            };
            let mut entries = entries;
            let Some(Value::Object(entry)) = entries.get_mut(at) else {
                return Err(EditError::Usage("that destination is not an object".into()));
            };
            if let Some(title) = title.clone() {
                entry.insert("title".into(), json!(title));
            }
            if clear_icon {
                entry.remove("icon");
            } else if let Some(icon) = icon.clone() {
                entry.insert("icon".into(), json!(icon));
            }
            doc_json::set(doc_value, &["navigation".into()], Value::Array(entries))?;
            true
        };
        if !changed {
            return Err(EditError::Usage("nothing changed".into()));
        }
        let preview = json!({
            "view": view,
            "title": title,
            "icon": if clear_icon { Value::Null } else { json!(icon) },
            "cleared": clear_icon,
        });
        cascade
            .finish(config, resources_dir, preview)
            .map(|edit| edit.named(format!("named the destination that opens {view}")))
    }

    /// `list.delete <view> [--title T] [--keep-view false]` — remove one rail
    /// entry. The view survives as a saved, non-navigable view unless
    /// `keepView` is explicitly false, which is the destructive half and runs
    /// `view.delete`'s own cascade, shared so the two doors cannot drift
    /// (§6.2 U6; `UI_SPEC.md` §4 E2: *"the view survives as a non-navigable
    /// saved view"*).
    pub fn delete(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let title = optional(params, "title");
        let keep = keep_view(params);
        let view = config.views.get(&name).ok_or_else(|| {
            EditError::Usage(format!("no view named \"{name}\" — SAM views lists them"))
        })?;
        let kind = view.type_.clone();
        let mut cascade = Cascade::start(Vec::new())?;
        drop_destination(&mut cascade, config, &name, title.as_deref())?;
        let recreated = if keep {
            Value::Null
        } else {
            view_ops::remove_view(&mut cascade, config, &name, kind.as_deref(), title.clone())?
        };
        let preview = json!({
            "list": name,
            "title": title,
            "keepView": keep,
            "sidebar": 1,
            "recreated": recreated,
        });
        cascade.finish(config, resources_dir, preview).map(|edit| {
            edit.named(format!(
                "removed the destination \"{}\"",
                title.clone().unwrap_or_else(|| name.clone())
            ))
        })
    }
}

// ── columns: the schema editor IS the table (§4.8 Principle 3) ───────────────

pub mod column_ops {
    use super::*;

    fn field_index(
        config: &ResolvedConfig,
        type_: &str,
        key: &str,
    ) -> Result<(usize, FieldDef), EditError> {
        let Some(def) = config.types.get(type_) else {
            return Err(EditError::Usage(format!("no type named \"{type_}\"")));
        };
        match def.fields.iter().position(|field| field.key == key) {
            Some(index) => Ok((index, def.fields[index].clone())),
            None => Err(EditError::Usage(format!(
                "type \"{type_}\" has no field \"{key}\""
            ))),
        }
    }

    fn fields_path(type_: &str) -> Vec<String> {
        vec!["types".into(), type_.into(), "fields".into()]
    }

    fn fields_of(doc: &Value, type_: &str) -> Result<Vec<Value>, EditError> {
        match doc_json::get(doc, &fields_path(type_)) {
            Some(Value::Array(items)) => Ok(items),
            _ => Err(EditError::Usage(format!(
                "fields of {type_} are not an array"
            ))),
        }
    }

    fn write_fields(doc: &mut Value, type_: &str, fields: Vec<Value>) -> Result<(), EditError> {
        doc_json::set(doc, &fields_path(type_), Value::Array(fields))
    }

    /// Replace one field's object, keeping every key the caller did not name.
    fn edit_field_object(
        doc: &mut Value,
        type_: &str,
        key: &str,
        edit: impl FnOnce(&mut Map<String, Value>),
    ) -> Result<(), EditError> {
        let mut fields = fields_of(doc, type_)?;
        let found = fields.iter().position(|field| match field {
            Value::Object(object) => object.get("key") == Some(&json!(key)),
            _ => false,
        });
        if let Some(index) = found {
            if let Value::Object(object) = &mut fields[index] {
                edit(object);
            }
        }
        write_fields(doc, type_, fields)
    }

    /// `column.new` — one FieldDef; the form, the table cell, the validation and
    /// the schema read by the agent all follow with zero view code (§4.2).
    pub fn new_column(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        let Some(def) = config.types.get(&type_) else {
            return Err(EditError::Usage(format!("no type named \"{type_}\"")));
        };
        if def.field(&key).is_some() {
            return Err(EditError::Usage(format!(
                "field \"{key}\" already exists on {type_}"
            )));
        }
        let kind = optional(params, "kind").unwrap_or_else(|| "text".into());
        if !validator::FIELD_TYPES.contains(&kind.as_str()) {
            return Err(EditError::Usage(format!(
                "unknown field type \"{kind}\" — the vocabulary is closed (§3.4)"
            )));
        }
        let mut entry = Map::new();
        entry.insert("key".into(), json!(key));
        entry.insert("type".into(), json!(kind));
        if let Some(label) = optional(params, "label") {
            entry.insert("label".into(), json!(label));
        }
        if let Some(options) = optional(params, "options") {
            entry.insert(
                "options".into(),
                json!(
                    options
                        .split(',')
                        .map(|option| option.trim())
                        .collect::<Vec<&str>>()
                ),
            );
        }
        if let Some(to) = optional(params, "to") {
            entry.insert("to".into(), json!(to));
        }
        if let Some(expr) = optional(params, "expr") {
            entry.insert("expr".into(), json!(expr));
        }
        if flag(params, "required") {
            entry.insert("required".into(), json!(true));
        }
        if (kind == "select" || kind == "multiSelect") && !entry.contains_key("options") {
            return Err(EditError::Usage(format!("{kind} needs --options a,b,c")));
        }
        if kind == "relation" {
            let Some(to) = optional(params, "to").filter(|to| !to.is_empty()) else {
                return Err(EditError::Usage("relation needs --to <type>".into()));
            };
            if !config.types.contains_key(&to) {
                return Err(EditError::Usage(format!(
                    "relation target type {to} does not exist"
                )));
            }
        }
        if kind == "formula" {
            match optional(params, "expr").filter(|expr| !expr.is_empty()) {
                Some(_) => {}
                None => return Err(EditError::Usage("formula needs --expr \"…\"".into())),
            }
        }
        let mut doc_value = require_doc(config, "content/types.json")?;
        if doc_json::get(&doc_value, &fields_path(&type_)).is_none() {
            doc_json::set(&mut doc_value, &fields_path(&type_), json!([]))?;
        }
        let at = optional(params, "before").and_then(|before| {
            field_index(config, &type_, &before)
                .ok()
                .map(|(index, _)| index)
        });
        match at {
            Some(index) => doc_json::insert(
                &mut doc_value,
                &fields_path(&type_),
                index,
                Value::Object(entry),
            )?,
            None => doc_json::append(&mut doc_value, &fields_path(&type_), Value::Object(entry))?,
        }
        let changes = vec![doc_change(config, "content/types.json", &doc_value)];
        finalize_with(config, resources_dir, changes, json!({ "column": spec }))
            .map(|edit| edit.named(format!("column.new {spec}")))
    }

    /// `column.rename` — the LABEL. The key never changes: it is the identity a
    /// record's `fields` object, every view and every JSON pointer uses
    /// (§4.8 Principle 1).
    pub fn rename(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        label: Option<&str>,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        field_index(config, &type_, &key)?;
        let mut doc_value = require_doc(config, "content/types.json")?;
        edit_field_object(&mut doc_value, &type_, &key, |object| match label {
            Some(label) if !label.is_empty() => {
                object.insert("label".into(), json!(label));
            }
            _ => {
                object.remove("label");
            }
        })?;
        let changes = vec![doc_change(config, "content/types.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "column": spec, "label": label }),
        )
        .map(|edit| edit.named(format!("column.rename {spec}")))
    }

    /// `column.retype` — value migration with a preview. Unsupported or lossy
    /// conversions are diagnosed per record before anything changes, and a
    /// dry-run reports them instead of throwing, so the preview can be shown
    /// and cancelled (§6 Phase 4 step 3).
    pub fn retype(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        params: &Params,
        dry_run: bool,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        let (_, field) = field_index(config, &type_, &key)?;
        let kind = require(params, "kind")?;
        if !validator::FIELD_TYPES.contains(&kind.as_str()) {
            return Err(EditError::Usage(format!("unknown field type \"{kind}\"")));
        }
        if field.is_relation() || kind == "relation" {
            return Err(EditError::Usage(
                "relations are not value types — delete the column and add a new relation".into(),
            ));
        }

        let mut migrations: Vec<(PositionedRecord, Value)> = Vec::new();
        let mut drops = 0usize;
        let mut problems: Vec<Diagnostic> = Vec::new();
        if field.type_ != kind {
            for p in config
                .positioned_records
                .iter()
                .filter(|p| p.record.type_ == type_)
            {
                let Some(value) = p.record.fields.get(&key) else {
                    continue;
                };
                // Derived ↔ stored drops the values either way.
                if kind == "formula" || field.type_ == "progress" || field.type_ == "formula" {
                    drops += 1;
                    continue;
                }
                let converted = match (field.type_.as_str(), kind.as_str()) {
                    ("text", "longtext")
                    | ("longtext", "text")
                    | ("number", "duration")
                    | ("duration", "number")
                    | ("text", "url")
                    | ("select", "text") => Some(value.clone()),
                    ("number", "text") | ("duration", "text") => {
                        Some(json!(crate::json_value::scalar_text(value)))
                    }
                    ("text", "select") | ("longtext", "select") => match value {
                        Value::String(text) => Some(json!(text)),
                        other => Some(other.clone()),
                    },
                    _ => None,
                };
                let Some(converted) = converted else {
                    problems.push(
                        Diagnostic::error(
                            "column.retype-unsupported",
                            format!("{}#{}/fields/{}", p.file, p.record.id, key),
                            format!(
                                "cannot convert \"{}\" from {} to {kind} — record \"{}\"",
                                crate::json_value::scalar_text(value),
                                field.type_,
                                p.record.id
                            ),
                        )
                        .at_line(p.line),
                    );
                    continue;
                };
                // The post-condition the validator would catch anyway, reported
                // per record up front so the preview can name the record.
                if matches!(field.type_.as_str(), "text" | "longtext") && kind == "number" {
                    let text = crate::json_value::scalar_text(&converted);
                    if text.parse::<f64>().is_err() {
                        problems.push(
                            Diagnostic::error(
                                "column.retype-value",
                                format!("{}#{}/fields/{}", p.file, p.record.id, key),
                                format!(
                                    "value \"{text}\" is not a number — record \"{}\"",
                                    p.record.id
                                ),
                            )
                            .at_line(p.line),
                        );
                        continue;
                    }
                }
                migrations.push((p.clone(), converted));
            }
        }
        if !problems.is_empty() {
            if dry_run {
                return Ok(PlannedEdit {
                    changes: Vec::new(),
                    summary: format!("column.retype {spec} (dry-run, blocked)"),
                    preview: Some(json!({
                        "blocked": true,
                        "problems": problems.iter().map(|p| json!({
                            "record": p.path, "message": p.message,
                        })).collect::<Vec<Value>>(),
                    })),
                });
            }
            return Err(EditError::Invalid(problems));
        }
        if (kind == "select" || kind == "multiSelect")
            && optional(params, "options").is_none_or(|options| options.is_empty())
        {
            return Err(EditError::Usage(format!(
                "retyping to {kind} needs --options covering current values"
            )));
        }

        let mut changes = Vec::new();
        let mut doc_value = require_doc(config, "content/types.json")?;
        let options = optional(params, "options");
        let expr = optional(params, "expr");
        edit_field_object(&mut doc_value, &type_, &key, |object| {
            object.insert("type".into(), json!(kind));
            match &options {
                Some(options) if kind == "select" || kind == "multiSelect" => {
                    object.insert(
                        "options".into(),
                        json!(
                            options
                                .split(',')
                                .map(|option| option.trim())
                                .collect::<Vec<&str>>()
                        ),
                    );
                }
                _ => {
                    object.remove("options");
                }
            }
            match &expr {
                Some(expr) if kind == "formula" => {
                    object.insert("expr".into(), json!(expr));
                }
                _ => {
                    object.remove("expr");
                }
            }
            object.remove("to");
        })?;
        changes.push(doc_change(config, "content/types.json", &doc_value));

        if !migrations.is_empty() || drops > 0 {
            let mut work: BTreeMap<String, BTreeMap<u64, Record>> = BTreeMap::new();
            for (p, converted) in &migrations {
                let mut edited = p.record.clone();
                if matches!(field.type_.as_str(), "text" | "longtext") && kind == "number" {
                    if let Ok(number) = crate::json_value::scalar_text(converted).parse::<f64>() {
                        edited.fields.insert(key.clone(), number_value(number));
                    }
                } else {
                    edited.fields.insert(key.clone(), converted.clone());
                }
                work.entry(p.file.clone())
                    .or_default()
                    .insert(p.line, edited);
            }
            if drops > 0 && field.type_ != "formula" {
                for p in config
                    .positioned_records
                    .iter()
                    .filter(|p| p.record.type_ == type_ && p.record.fields.contains_key(&key))
                {
                    let mut edited = p.record.clone();
                    edited.fields.remove(&key);
                    work.entry(p.file.clone())
                        .or_default()
                        .insert(p.line, edited);
                }
            }
            for (rel, replacements) in &work {
                if let Some(change) = rebuild_records(config, rel, replacements, &BTreeSet::new())?
                {
                    changes.push(change);
                }
            }
        }

        let preview = json!({
            "column": spec,
            "from": field.type_,
            "to": kind,
            "converted": migrations.len(),
            "valuesDropped": drops,
        });
        if dry_run {
            return Ok(PlannedEdit {
                changes,
                summary: format!("column.retype {spec} (dry-run)"),
                preview: Some(preview),
            });
        }
        finalize_with(config, resources_dir, changes, preview.clone())
            .map(|edit| edit.named(format!("column.retype {spec} → {kind}")))
    }

    /// `column.choices` — replace a select's options. Records must still
    /// conform; a value outside the new list blocks the commit (§3.7).
    pub fn choices(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        options: &str,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        let (_, field) = field_index(config, &type_, &key)?;
        if field.type_ != "select" && field.type_ != "multiSelect" {
            return Err(EditError::Usage(format!(
                "column.choices edits select/multiSelect fields; \"{key}\" is {}",
                field.type_
            )));
        }
        let mut doc_value = require_doc(config, "content/types.json")?;
        edit_field_object(&mut doc_value, &type_, &key, |object| {
            object.insert(
                "options".into(),
                json!(
                    options
                        .split(',')
                        .map(|option| option.trim())
                        .collect::<Vec<&str>>()
                ),
            );
        })?;
        let changes = vec![doc_change(config, "content/types.json", &doc_value)];
        finalize_with(config, resources_dir, changes, json!({ "column": spec }))
            .map(|edit| edit.named(format!("column.choices {spec}")))
    }

    /// `column.duplicate` — a copy of the FieldDef under a free key.
    pub fn duplicate(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        as_key: Option<&str>,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        let (index, _) = field_index(config, &type_, &key)?;
        let target = as_key
            .filter(|key| !key.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{key}2"));
        if config
            .types
            .get(&type_)
            .and_then(|def| def.field(&target))
            .is_some()
        {
            return Err(EditError::Usage(format!(
                "field \"{target}\" already exists on {type_}"
            )));
        }
        let mut doc_value = require_doc(config, "content/types.json")?;
        let mut fields = fields_of(&doc_value, &type_)?;
        let mut entry = match fields.get(index) {
            Some(Value::Object(object)) => object.clone(),
            _ => Map::new(),
        };
        entry.insert("key".into(), json!(target));
        fields.push(Value::Object(entry));
        write_fields(&mut doc_value, &type_, fields)?;
        let changes = vec![doc_change(config, "content/types.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "column": spec, "as": target }),
        )
        .map(|edit| edit.named(format!("column.duplicate {spec} → {target}")))
    }

    /// `column.delete` — the FieldDef, **its data in every record** and every
    /// view's column reference, in one previewed transaction (D14's undo
    /// restores all three from the recorded before-images).
    pub fn delete(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        dry_run: bool,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        let (index, field) = field_index(config, &type_, &key)?;
        let mut changes = Vec::new();

        let touched: Vec<&PositionedRecord> = config
            .positioned_records
            .iter()
            .filter(|p| {
                p.record.type_ == type_
                    && (p.record.fields.contains_key(&key) || p.record.links.contains_key(&key))
            })
            .collect();
        if !touched.is_empty() {
            let mut work: BTreeMap<String, BTreeMap<u64, Record>> = BTreeMap::new();
            for p in &touched {
                let mut edited = p.record.clone();
                edited.fields.remove(&key);
                edited.links.remove(&key);
                work.entry(p.file.clone())
                    .or_default()
                    .insert(p.line, edited);
            }
            for (rel, replacements) in &work {
                if let Some(change) = rebuild_records(config, rel, replacements, &BTreeSet::new())?
                {
                    changes.push(change);
                }
            }
        }

        let mut doc_value = require_doc(config, "content/types.json")?;
        let mut fields = fields_of(&doc_value, &type_)?;
        if index < fields.len() {
            fields.remove(index);
        }
        write_fields(&mut doc_value, &type_, fields)?;
        changes.push(doc_change(config, "content/types.json", &doc_value));

        // A view of this type loses the column reference — by name, not by
        // position, because position is not an identity (§4.3).
        let affected: Vec<String> = config
            .views
            .iter()
            .filter(|(_, view)| {
                view.type_.as_deref() == Some(type_.as_str())
                    && view.columns.as_ref().is_some_and(|c| c.contains(&key))
            })
            .map(|(name, _)| name.clone())
            .collect();
        if !affected.is_empty() {
            let mut views_doc = require_doc(config, "content/views.json")?;
            for name in &affected {
                let Some(Value::Array(columns)) = doc_json::get(
                    &views_doc,
                    &["views".into(), name.clone(), "columns".into()],
                ) else {
                    continue;
                };
                let kept: Vec<Value> = columns
                    .into_iter()
                    .filter(|column| column.as_str() != Some(key.as_str()))
                    .collect();
                doc_json::set(
                    &mut views_doc,
                    &["views".into(), name.clone(), "columns".into()],
                    Value::Array(kept),
                )?;
            }
            changes.push(doc_change(config, "content/views.json", &views_doc));
        }

        let preview = json!({
            "column": spec,
            "recordsTouched": touched.len(),
            "kind": field.type_,
            "views": affected,
        });
        if dry_run {
            return Ok(PlannedEdit {
                changes,
                summary: format!("column.delete {spec} (dry-run)"),
                preview: Some(preview),
            });
        }
        finalize_with(config, resources_dir, changes, preview.clone())
            .map(|edit| edit.named(format!("column.delete {spec}")))
    }

    /// `column.reorder` — move one field before another. An empty `before` moves
    /// it **last**, which "before" alone could never express: there is no field
    /// after the last one to name.
    pub fn reorder(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        before: &str,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        let (index, _) = field_index(config, &type_, &key)?;
        let target = if before.trim().is_empty() {
            None
        } else {
            Some(field_index(config, &type_, before)?.0)
        };
        let mut doc_value = require_doc(config, "content/types.json")?;
        let mut fields = fields_of(&doc_value, &type_)?;
        let entry = fields.remove(index);
        let at = match target {
            None => fields.len(),
            Some(target) if index < target => target - 1,
            Some(target) => target,
        };
        fields.insert(at.min(fields.len()), entry);
        write_fields(&mut doc_value, &type_, fields)?;
        let changes = vec![doc_change(config, "content/types.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "column": spec, "before": before, "last": before.trim().is_empty() }),
        )
        .map(|edit| {
            edit.named(if before.trim().is_empty() {
                format!("column.reorder {spec} last")
            } else {
                format!("column.reorder {spec} before {before}")
            })
        })
    }

    /// `column.hide` / `column.show` — a **view's** column list. Hiding is a
    /// property of the view, not the field: the design's *"Show in
    /// Table/Board/Calendar"* is one view's decision, and the type keeps its
    /// column (§3.6).
    pub fn set_visibility(
        config: &ResolvedConfig,
        resources_dir: &Path,
        spec: &str,
        view: &str,
        hidden: bool,
    ) -> Result<PlannedEdit, EditError> {
        let (type_, key) = parse_spec(spec)?;
        if config
            .types
            .get(&type_)
            .and_then(|def| def.field(&key))
            .is_none()
        {
            return Err(EditError::Usage(format!(
                "type \"{type_}\" has no field \"{key}\""
            )));
        }
        let Some(view_def) = config.views.get(view) else {
            return Err(EditError::Usage(format!("no view named \"{view}\"")));
        };
        if view_def.type_.as_deref() != Some(type_.as_str()) {
            return Err(EditError::Usage(format!(
                "view \"{view}\" shows {}, not {type_}",
                view_def.type_.as_deref().unwrap_or("no kind")
            )));
        }
        // A view with no declared columns shows every non-derived field, so
        // hiding one must first materialise that default list.
        let current = view_def.columns.clone().unwrap_or_else(|| {
            config
                .types
                .get(&type_)
                .map(|def| {
                    def.fields
                        .iter()
                        .filter(|field| field.type_ != "progress")
                        .map(|field| field.key.clone())
                        .collect()
                })
                .unwrap_or_default()
        });
        let mut next: Vec<String> = current
            .into_iter()
            .filter(|column| column != &key)
            .collect();
        if !hidden {
            next.push(key);
        }
        let mut views_doc = require_doc(config, "content/views.json")?;
        doc_json::set(
            &mut views_doc,
            &["views".into(), view.into(), "columns".into()],
            json!(next),
        )?;
        let changes = vec![doc_change(config, "content/views.json", &views_doc)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "column": spec, "view": view }),
        )
        .map(|edit| {
            edit.named(format!(
                "column.{} {spec} in {view}",
                if hidden { "hide" } else { "show" }
            ))
        })
    }
}

// ── settings, and the text door ──────────────────────────────────────────────

/// The app's own settings schema (§4.2: *"One function generates record forms
/// from user-defined types **and** the settings UI from the app's own schema"*).
/// Each row names the JSON pointer it persists at — Phase 6 spans more than
/// `#/study`, so the row is the unit rather than the file.
pub mod settings_ops {
    use super::*;

    /// One generated settings row: the `FieldDef` vocabulary the form renders,
    /// plus where the value persists (§4.8 P3: every row shows its config key).
    pub struct SettingDef {
        pub key: &'static str,
        pub type_: &'static str,
        pub label: &'static str,
        /// The JSON pointer under `content/rules.json`.
        pub path: &'static [&'static str],
        /// Declared choices for a `select` row.
        pub options: &'static [&'static str],
    }

    pub const FIELDS: &[SettingDef] = &[
        SettingDef {
            key: "dailyTargetMin",
            type_: "duration",
            label: "Daily target",
            path: &["study", "dailyTargetMin"],
            options: &[],
        },
        SettingDef {
            key: "timezone",
            type_: "text",
            label: "Plan timezone",
            // §3.5 puts the plan timezone at `rules.json#/timezone`; the
            // reader also accepts the `#/study/timezone` spelling older plans
            // carry (one location, one documented fallback).
            path: &["timezone"],
            options: &[],
        },
        // Phase 6 (§3.5, D11): the scheduler is a settings row like any other,
        // so choosing `sm2` or `fsrs` is a data change through `settings.set`,
        // not a second command.
        SettingDef {
            key: "scheduler",
            type_: "select",
            label: "Review schedule",
            path: &["scheduler", "name"],
            options: &crate::rules::SCHEDULERS,
        },
        SettingDef {
            key: "fixedIntervals",
            type_: "json",
            label: "Fixed review intervals (days)",
            path: &["scheduler", "fixed", "intervals"],
            options: &[],
        },
    ];

    pub fn find(key: &str) -> Option<&'static SettingDef> {
        FIELDS.iter().find(|def| def.key == key)
    }

    pub fn field(key: &str) -> Option<FieldDef> {
        find(key).map(|def| FieldDef {
            key: def.key.into(),
            type_: def.type_.into(),
            label: Some(def.label.into()),
            options: if def.options.is_empty() {
                None
            } else {
                Some(
                    def.options
                        .iter()
                        .map(|option| (*option).to_string())
                        .collect(),
                )
            },
            ..Default::default()
        })
    }

    pub fn keys() -> Vec<&'static str> {
        FIELDS.iter().map(|def| def.key).collect()
    }

    /// The display text of every row's current value, keyed by the row's key:
    /// what the settings screen shows beside the key it is editing. A row whose
    /// pointer is absent shows as "default" rather than an invented zero.
    pub fn values(config: &ResolvedConfig) -> BTreeMap<String, String> {
        let document = doc(config, "content/rules.json")
            .ok()
            .flatten()
            .unwrap_or_else(|| doc_json::with_schema(None));
        let mut out = BTreeMap::new();
        for def in FIELDS {
            let segments: Vec<String> = def.path.iter().map(|part| (*part).to_string()).collect();
            let text = doc_json::get(&document, &segments)
                .map(|value| crate::json_value::scalar_text(&value));
            if let Some(text) = text {
                out.insert(def.key.to_string(), text);
            }
        }
        out
    }

    /// `settings.set <key> <value>` — one row of the generated screen. A value
    /// of `null` removes the key, which is how a setting returns to its default
    /// rather than being pinned to a copy of it (§4.1: *"Reset removes the
    /// override key"*).
    pub fn set(
        config: &ResolvedConfig,
        resources_dir: &Path,
        key: &str,
        value: &str,
    ) -> Result<PlannedEdit, EditError> {
        let Some(def) = find(key) else {
            return Err(EditError::Usage(format!(
                "unknown setting \"{key}\" — declared: {}",
                keys().join(", ")
            )));
        };
        let address = format!("content/rules.json#/{}", def.path.join("/"));
        let field_def = field(key).expect("a declared row has a field def");
        let parsed = parse_field_value(value, &field_def, &address)?;
        let mut doc_value = type_ops::doc_with_schema(config, "content/rules.json", None)?;
        let segments: Vec<String> = def.path.iter().map(|part| (*part).to_string()).collect();
        if value == "null" {
            if doc_json::get(&doc_value, &segments).is_some() {
                doc_json::remove(&mut doc_value, &segments)?;
            }
        } else {
            ensure_object_path(&mut doc_value, &segments, "/", "content/rules.json")?;
            doc_json::set(&mut doc_value, &segments, parsed)?;
        }
        let changes = vec![doc_change(config, "content/rules.json", &doc_value)];
        finalize_with(config, resources_dir, changes, json!({ "setting": key }))
            .map(|edit| edit.named(format!("settings.set {key}")))
    }
}

// ── derived figures: the rules place's own write (§3.1, §4.2) ────────────────

/// `content/rules.json#/metrics/<name>` — one **derived figure**: a label, a
/// saved view and the fold run over it. The figure is a member of a map, so the
/// member is the unit rather than the file, and the whole-plan validator is
/// what keeps it loadable: a figure pointing at an undeclared view or an
/// unknown fold is refused by `metric.unknown-view` / `metric.bad-reduce`
/// before the transaction commits (`finalize_with`).
pub mod metric_ops {
    use super::*;

    /// The keys one figure declares, in the order the screen draws them.
    /// `label` and `view` have no default in `MetricDef`, so they are required
    /// to create a figure and cannot be cleared from one.
    const KEYS: [&str; 5] = ["label", "view", "expr", "reduce", "unit"];
    const REQUIRED: [&str; 2] = ["label", "view"];

    /// `dispatch::require` speaks `DispatchError` and this layer speaks
    /// `EditError`; a parameter the request left out is a usage problem in both.
    fn required(params: &Params, name: &str) -> Result<String, EditError> {
        match params.get(name) {
            Some(ParamValue::Str(value)) => Ok(value.clone()),
            _ => Err(EditError::Usage(format!(
                "missing required parameter \"{name}\""
            ))),
        }
    }

    /// `metric.set --name <figure> [--label …] [--view …] [--expr …]
    /// [--reduce …] [--unit …]` — one figure, whole. A key given empty is
    /// **removed** rather than written as `""`: an undeclared `unit` and an
    /// empty string are different figures, and the first is the one that means
    /// "no unit" (`view.set*` and `settings.set` take the same route). A view
    /// that does not exist and a fold outside the closed vocabulary are refused
    /// by the validator, which sees the candidate plan whole.
    pub fn set(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = required(params, "name")?;
        // The same name space as a type id: a JSON object member and a path
        // segment, so the file's map key and the CLI's argument are one string.
        if !type_ops::safe_type_id(&name) {
            return Err(EditError::Usage(format!(
                "\"{name}\" is not a figure name — letters, digits, '.' and '-'"
            )));
        }
        let provided: Vec<(&str, Option<String>)> =
            KEYS.map(|key| (key, optional(params, key))).to_vec();
        let mut doc_value = type_ops::doc_with_schema(config, "content/rules.json", None)?;
        let segments = vec!["metrics".to_string(), name.clone()];
        let existing = doc_json::get(&doc_value, &segments).is_some();
        for (key, value) in &provided {
            let Some(text) = value else { continue };
            if text.is_empty() && REQUIRED.contains(key) {
                return Err(EditError::Usage(format!(
                    "a figure keeps its {key} — \"{name}\" would not load without one"
                )));
            }
        }
        if !existing {
            for key in REQUIRED {
                let missing = !provided.iter().any(|(given, value)| {
                    *given == key && value.as_deref().is_some_and(|text| !text.is_empty())
                });
                if missing {
                    return Err(EditError::Usage(format!(
                        "a new figure needs --{key} — \"{name}\" has none yet"
                    )));
                }
            }
        }
        let mut wrote: Vec<&str> = Vec::new();
        for (key, value) in &provided {
            let Some(text) = value else { continue };
            let path = vec!["metrics".to_string(), name.clone(), (*key).to_string()];
            if text.is_empty() {
                // A key that is absent and a key cleared to nothing are the same
                // figure, so the removal is a no-op rather than a refusal.
                doc_json::remove(&mut doc_value, &path)?;
            } else {
                if *key == "expr" {
                    // The evaluator parses a figure's expression the way it
                    // parses a block's (`rules::metrics_json`), so a write that
                    // cannot parse is refused here rather than stored as a row
                    // that renders an error forever.
                    crate::expression::parse(
                        text,
                        &format!("content/rules.json#/metrics/{name}/expr"),
                    )?;
                }
                // The figure itself may be the first key under `#/metrics`, so
                // the path is created on the way in — the CLI stays order-free.
                ensure_object_path(&mut doc_value, &path, "/", "content/rules.json")?;
                doc_json::set(&mut doc_value, &path, json!(text))?;
            }
            wrote.push(key);
        }
        if wrote.is_empty() {
            return Err(EditError::Usage(format!(
                "nothing to write — pass at least one of {}",
                KEYS.map(|key| format!("--{key}")).join(", ")
            )));
        }
        let entry = doc_json::get(&doc_value, &segments).unwrap_or(Value::Null);
        let changes = vec![doc_change(config, "content/rules.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "metric": name, "created": !existing, "wrote": wrote, "entry": entry }),
        )
        .map(|edit| edit.named(format!("metric.set {name}")))
    }
}

// ── structural editing: cross-type moves and renumbering (§6 Phase 6) ────────

/// Bulk structural editing (§6 Phase 6's step 4): *"reparent a course, renumber
/// units, move records between types"*. Each operation is one whole-plan
/// validated transaction on the same protocol as every other write — a move
/// removes a line from one file and appends it to another, and a renumber
/// rewrites only the records whose number changes.
pub mod structure_ops {
    use super::*;

    /// The link keys of a target type's fields, for dropping what does not
    /// survive a move.
    fn target_links(config: &ResolvedConfig, type_name: &str) -> BTreeSet<String> {
        config
            .types
            .get(type_name)
            .map(|def| def.allowed_link_keys())
            .unwrap_or_default()
    }

    fn field_keys(config: &ResolvedConfig, type_name: &str) -> BTreeSet<String> {
        config
            .types
            .get(type_name)
            .map(|def| def.fields.iter().map(|field| field.key.clone()).collect())
            .unwrap_or_default()
    }

    /// `record.move <id> --to <type> [--parent <id>]`: change a record's type,
    /// dropping values the target type cannot hold (**reported**, never silent)
    /// and migrating its progress entry through the same rules a pipeline
    /// switch uses (§3.5: a different machine keeps the history).
    pub fn move_record(
        config: &ResolvedConfig,
        resources_dir: &Path,
        id: &str,
        to: &str,
        parent: Option<&str>,
    ) -> Result<PlannedEdit, EditError> {
        let Some(source) = record_ops::positioned(config, id) else {
            return Err(EditError::Usage(format!("no record named \"{id}\"")));
        };
        let Some(target_def) = config.types.get(to) else {
            return Err(EditError::Usage(format!(
                "no type named \"{to}\" — declared: {}",
                config.types.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        };
        if source.record.type_ == to {
            return Err(EditError::Usage(format!(
                "record \"{id}\" is already a {to}"
            )));
        }
        let fields_allowed = field_keys(config, to);
        let links_allowed = target_links(config, to);
        let mut moved = source.record.clone();
        moved.type_ = to.to_string();
        let mut dropped_fields: Vec<String> = Vec::new();
        let mut dropped_links: Vec<String> = Vec::new();
        moved.fields.retain(|key, _| {
            let keep = fields_allowed.contains(key);
            if !keep {
                dropped_fields.push(key.clone());
            }
            keep
        });
        moved.links.retain(|key, _| {
            let keep = links_allowed.contains(key);
            if !keep {
                dropped_links.push(key.clone());
            }
            keep
        });
        if let Some(parent_id) = parent {
            let Some(edge) = target_def.parent.as_deref() else {
                return Err(EditError::Usage(format!(
                    "type \"{to}\" declares no parent — pass no --parent, or a type with one"
                )));
            };
            if config.record(parent_id).is_none() {
                return Err(EditError::Usage(format!(
                    "no record named \"{parent_id}\" for the parent edge"
                )));
            }
            moved
                .links
                .insert(edge.to_string(), vec![parent_id.to_string()]);
        }

        // Source file: the line goes away. Target file: the record is appended.
        let source_change = rebuild_records(
            config,
            &source.file,
            &BTreeMap::new(),
            &BTreeSet::from([source.line]),
        )?;
        let target_file = format!("content/records/{to}.jsonl");
        let target_raw = config
            .source_files
            .get(&target_file)
            .map(|bytes| String::from_utf8_lossy(bytes).to_string());
        let target_text = apply::rebuild_jsonl(
            target_raw.as_deref(),
            &BTreeMap::new(),
            &BTreeSet::new(),
            std::slice::from_ref(&moved),
        )
        .map_err(EditError::Io)?;
        let target_before = config.source_files.get(&target_file).cloned();
        let target_after: Option<Vec<u8>> = if target_text.is_empty() {
            None
        } else {
            Some(target_text.into_bytes())
        };

        // Progress: the entry follows the record. A different pipeline keeps the
        // old machine in `history` and starts a fresh active state.
        let mut changes: Vec<FileChange> = Vec::new();
        if let Some(change) = source_change {
            changes.push(change);
        }
        if target_before != target_after {
            changes.push(FileChange::new(&target_file, target_before, target_after));
        }
        let mut pipeline_note = serde_json::Value::Null;
        let from_pipeline = config
            .types
            .get(&source.record.type_)
            .and_then(|def| def.pipeline.clone());
        let to_pipeline = target_def.pipeline.clone();
        if from_pipeline != to_pipeline {
            let mut document = crate::pipeline::state_document(config)?;
            let mut entry = crate::pipeline::entry_in(&document, id);
            if !entry.stages.is_empty() || entry.pipeline.is_some() {
                let mut history = entry.history.clone();
                history.push(json!({
                    "pipeline": entry.pipeline.clone(),
                    "stages": entry.stages,
                    "data": entry.data,
                    "movedAt": crate::transaction::iso_now(),
                    "to": to,
                }));
                entry.pipeline = to_pipeline.clone();
                entry.stages = BTreeMap::new();
                entry.review = None;
                entry.history = history;
                crate::pipeline::set_entry_in(&mut document, id, &entry)?;
                changes.push(doc_change(config, crate::pipeline::STATE_FILE, &document));
            }
            pipeline_note = json!({ "from": from_pipeline, "to": to_pipeline, "history": true });
        }

        let preview = json!({
            "id": id,
            "from": source.record.type_,
            "to": to,
            "droppedFields": dropped_fields,
            "droppedLinks": dropped_links,
            "parent": parent,
            "pipeline": pipeline_note,
            "files": ["content/records", crate::pipeline::STATE_FILE],
        });
        finalize_with(config, resources_dir, changes, preview)
            .map(|edit| edit.named(format!("record.move {id} → {to}")))
    }

    /// `records.renumber <type> [--field index] [--parent <id>] [--start 1]`:
    /// assign consecutive numbers in the records' current order. The order is
    /// the records' existing numeric order when the field is populated, and file
    /// order for records the field has never numbered — so running it twice is a
    /// no-op and the first run numbers a hand-authored list deterministically.
    pub fn renumber(
        config: &ResolvedConfig,
        resources_dir: &Path,
        type_name: &str,
        field_key: &str,
        parent: Option<&str>,
        start: i64,
    ) -> Result<PlannedEdit, EditError> {
        let Some(def) = config.types.get(type_name) else {
            return Err(EditError::Usage(format!("no type named \"{type_name}\"")));
        };
        let Some(field) = def.field(field_key) else {
            return Err(EditError::Usage(format!(
                "type \"{type_name}\" has no field \"{field_key}\""
            )));
        };
        if field.type_ != "number" {
            return Err(EditError::Usage(format!(
                "renumber writes a number field; \"{field_key}\" is {}",
                field.type_
            )));
        }
        if let Some(parent_id) = parent
            && config.record(parent_id).is_none()
        {
            return Err(EditError::Usage(format!(
                "no record named \"{parent_id}\" to scope the renumber to"
            )));
        }
        let mut candidates: Vec<&PositionedRecord> = config
            .positioned_records
            .iter()
            .filter(|positioned| positioned.record.type_ == type_name)
            .filter(|positioned| match parent {
                None => true,
                Some(parent_id) => def.parent.as_deref().is_some_and(|edge| {
                    positioned
                        .record
                        .links
                        .get(edge)
                        .is_some_and(|targets| targets.iter().any(|t| t == parent_id))
                }),
            })
            .collect();
        // Deterministic order: a populated number first (ascending), then the
        // records that have none, each group in file order.
        candidates.sort_by_key(|positioned| {
            let number = positioned
                .record
                .fields
                .get(field_key)
                .and_then(|value| value.as_f64());
            (
                number.is_none(),
                number.map(|number| number as i64).unwrap_or(0),
                positioned.line,
            )
        });

        let mut mapping: Vec<serde_json::Value> = Vec::new();
        let mut by_file: BTreeMap<String, BTreeMap<u64, Record>> = BTreeMap::new();
        for (index, positioned) in candidates.iter().enumerate() {
            let number = start + index as i64;
            let current = positioned
                .record
                .fields
                .get(field_key)
                .and_then(Value::as_i64);
            if current == Some(number) {
                continue;
            }
            let mut updated = positioned.record.clone();
            updated.fields.insert(field_key.to_string(), json!(number));
            by_file
                .entry(positioned.file.clone())
                .or_default()
                .insert(positioned.line, updated.clone());
            mapping.push(json!({ "id": positioned.record.id, "from": current, "to": number }));
        }
        let mut changes = Vec::new();
        for (file, replacements) in &by_file {
            if let Some(change) = rebuild_records(config, file, replacements, &BTreeSet::new())? {
                changes.push(change);
            }
        }
        let preview = json!({
            "type": type_name,
            "field": field_key,
            "parent": parent,
            "start": start,
            "count": candidates.len(),
            "changed": mapping,
        });
        finalize_with(config, resources_dir, changes, preview)
            .map(|edit| edit.named(format!("records.renumber {type_name}.{field_key}")))
    }
}

// ── views and screens: the query is the view, the blocks are its shape ───────

/// Phase 5's mutation layer: a saved view's query keys and its block tree,
/// written one key at a time so every write has a JSON identity (§4.8 P3) and
/// the same validator sees every candidate (`finalize_with`).
pub mod view_ops {
    use super::*;

    /// The query keys `view.set*` may write. `layout` and `columns` are shaped;
    /// the rest are L2 text.
    const QUERY_KEYS: [&str; 7] = [
        "layout", "filter", "sort", "group", "limit", "columns", "panel",
    ];

    fn require_view<'c>(
        config: &'c ResolvedConfig,
        name: &str,
    ) -> Result<&'c crate::model::ViewDef, EditError> {
        config.views.get(name).ok_or_else(|| {
            EditError::Usage(format!("no view named \"{name}\" — SAM views lists them"))
        })
    }

    fn require_block(
        config: &ResolvedConfig,
        name: &str,
        index: usize,
    ) -> Result<crate::model::BlockDef, EditError> {
        let view = require_view(config, name)?;
        let blocks = view.blocks.as_deref().unwrap_or_default();
        blocks.get(index).cloned().ok_or_else(|| {
            EditError::Usage(format!(
                "view \"{name}\" declares {} block(s); {index} is out of range",
                blocks.len()
            ))
        })
    }

    /// `--value key=value` repeats, as a block's own key map. A value is parsed
    /// as JSON when it parses (`2`, `true`, `["a"]`, `{"k":1}`) and kept as a
    /// string otherwise — the same rule `view.block.set` applies to one key.
    fn initial_keys(params: &Params) -> Vec<(String, Value)> {
        let entries = match params.get("value") {
            Some(ParamValue::Strings(values)) => values.clone(),
            Some(ParamValue::Str(text)) => vec![text.clone()],
            _ => Vec::new(),
        };
        entries
            .iter()
            .filter_map(|entry| {
                let (key, text) = entry.split_once('=')?;
                let key = key.trim();
                if key.is_empty() {
                    return None;
                }
                let value = serde_json::from_str::<Value>(text)
                    .ok()
                    .filter(|value| !value.is_null())
                    .unwrap_or_else(|| Value::String(text.to_string()));
                Some((key.to_string(), value))
            })
            .collect()
    }

    /// What one `--value` means for a key: a value, or "remove the key".
    enum Written {
        Set(Value),
        Remove,
    }

    /// A key cleared is a key **removed**, never written as empty: an undeclared
    /// `filter` and an empty string are different documents, and the first is
    /// the one that means "no filter" (settings.set takes the same route).
    fn parse_value(key: &str, value: Option<&str>) -> Result<Written, EditError> {
        let Some(text) = value.filter(|text| !text.is_empty() && *text != "null") else {
            return Ok(Written::Remove);
        };
        match key {
            "limit" => text
                .parse::<i64>()
                .map(|number| Written::Set(json!(number)))
                .map_err(|_| {
                    EditError::Usage(format!("limit must be a whole number, not \"{text}\""))
                }),
            "columns" => Ok(Written::Set(json!(
                text.split(',')
                    .map(str::trim)
                    .filter(|column| !column.is_empty())
                    .collect::<Vec<_>>()
            ))),
            _ => Ok(Written::Set(json!(text))),
        }
    }

    /// Write one key of one block. Blocks live in an **array**, and
    /// `doc_json::set` writes object members only, so the array is read, edited
    /// and written back — the same shape `doc_json::insert` uses.
    fn block_key(
        doc: &mut Value,
        name: &str,
        index: usize,
        key: &str,
        written: Written,
    ) -> Result<(), EditError> {
        let path = vec!["views".to_string(), name.to_string(), "blocks".to_string()];
        let Some(Value::Array(mut blocks)) = doc_json::get(doc, &path) else {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares no blocks"
            )));
        };
        let Some(block) = blocks.get_mut(index) else {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares {} block(s); {index} is out of range",
                blocks.len()
            )));
        };
        let Value::Object(object) = block else {
            return Err(EditError::Usage(format!(
                "view \"{name}\" block {index} is not an object"
            )));
        };
        match written {
            Written::Set(value) => {
                object.insert(key.to_string(), value);
            }
            Written::Remove => {
                object.remove(key);
            }
        }
        doc_json::set(doc, &path, Value::Array(blocks))
    }

    /// `view.setLayout` / `view.setFilter` / `view.setSort` / `view.setGroup` /
    /// `view.setLimit` / `view.setColumns` — one key of one saved view.
    ///
    /// The candidate is validated whole-plan before it lands, so an unknown
    /// layout, an unparsable expression, a negative limit or a column the type
    /// does not declare is reported by the one validator rather than by a second
    /// copy of its rules here.
    pub fn set_query(
        config: &ResolvedConfig,
        resources_dir: &Path,
        name: &str,
        key: &str,
        value: Option<&str>,
    ) -> Result<PlannedEdit, EditError> {
        if !QUERY_KEYS.contains(&key) {
            return Err(EditError::Usage(format!(
                "\"{key}\" is not a view query key — one of {}",
                QUERY_KEYS.join(", ")
            )));
        }
        require_view(config, name)?;
        let written = parse_value(key, value)?;
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let path = vec!["views".to_string(), name.to_string(), key.to_string()];
        match written {
            Written::Set(written) => doc_json::set(&mut doc_value, &path, written)?,
            Written::Remove => doc_json::remove(&mut doc_value, &path)?,
        }
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "view": name, "key": key, "value": value }),
        )
        .map(|edit| edit.named(format!("view.set {name} · {key}")))
    }

    /// `view.setComponents <name> [--surfaces a,b,c] [--spans 1,2,1] [--clear]` (COMPOSER §2.2).
    /// Updates the screen's component list and spans in a single transaction.
    /// `--clear` removes the `components` key entirely to restore panel defaults.
    pub fn set_components(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let clear = flag(params, "clear");
        let surfaces = optional(params, "surfaces");
        let spans = optional(params, "spans");
        require_view(config, &name)?;
        if clear && surfaces.is_some() {
            return Err(EditError::Usage(
                "--clear and --surfaces are two different writes; name one".into(),
            ));
        }
        if spans.is_some() && surfaces.is_none() {
            return Err(EditError::Usage(
                "--spans gives each surface its width, so it needs --surfaces".into(),
            ));
        }
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let path = vec!["views".to_string(), name.clone(), "components".to_string()];
        let written: Vec<Value> = if clear {
            doc_json::remove(&mut doc_value, &path)?;
            Vec::new()
        } else {
            // The surfaces are the list; the widths are read against it before
            // anything touches the document, so a refused list leaves the plan
            // exactly as it was.
            let wanted: Vec<&str> = surfaces
                .as_deref()
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|surface| !surface.is_empty())
                .collect();
            let widths: Vec<u8> = match spans.as_deref() {
                None => Vec::new(),
                Some(spans) => {
                    let list: Vec<u8> = spans
                        .split(',')
                        .map(str::trim)
                        // The same reading `--surfaces` gets: an empty token is
                        // no value, so `--spans ""` beside `--surfaces ""` is
                        // the empty screen, not a malformed list.
                        .filter(|span| !span.is_empty())
                        .map(|span| {
                            span.parse::<u8>()
                                .ok()
                                .filter(|width| (1..=2).contains(width))
                                .ok_or_else(|| {
                                    EditError::Usage(format!(
                                        "--spans takes 1 or 2 for each surface, got \"{span}\""
                                    ))
                                })
                        })
                        .collect::<Result<Vec<u8>, EditError>>()?;
                    if list.len() != wanted.len() {
                        return Err(EditError::Usage(format!(
                            "--spans names {} width{} for {} surface{}; give one per surface",
                            list.len(),
                            if list.len() == 1 { "" } else { "s" },
                            wanted.len(),
                            if wanted.len() == 1 { "" } else { "s" },
                        )));
                    }
                    list
                }
            };
            // A surface already on the screen keeps **its own object**: the write
            // is an in-place edit, so a member this build does not know survives
            // a reorder exactly as a block member does. Only a surface that was
            // not there starts from `{ "surface": … }`.
            let mut carried: BTreeMap<String, Value> = doc_json::get(&doc_value, &path)
                .and_then(|components| components.as_array().cloned())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|component| {
                    let surface = component
                        .get("surface")
                        .and_then(Value::as_str)?
                        .to_string();
                    Some((surface, component))
                })
                .collect();
            let list: Vec<Value> = wanted
                .iter()
                .enumerate()
                .map(|(index, surface)| {
                    let mut component = carried
                        .remove(*surface)
                        .unwrap_or_else(|| json!({ "surface": surface }));
                    // `span` is this command's own member: 1 is written, 2 — the
                    // default — is removed. With no `--spans`, the request says
                    // nothing about a component's width, so nothing about it
                    // changes.
                    if let (Some(width), Some(object)) =
                        (widths.get(index), component.as_object_mut())
                    {
                        if *width == 1 {
                            object.insert("span".into(), json!(1));
                        } else {
                            object.remove("span");
                        }
                    }
                    component
                })
                .collect();
            doc_json::set(&mut doc_value, &path, Value::Array(list.clone()))?;
            list
        };
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        // The widths the screen now holds, one per written surface and in the
        // order it draws them: the read-back the composer's frames size
        // themselves by, taken from the document and not from the request.
        let written_spans: Vec<i64> = written
            .iter()
            .map(|component| component.get("span").and_then(Value::as_i64).unwrap_or(2))
            .collect();
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({
                "view": name,
                "surfaces": written,
                "spans": written_spans,
                "cleared": clear,
            }),
        )
        .map(|edit| edit.named(format!("view.setComponents {name}")))
    }

    /// `view.setLayout … --block N` — a record block's kind **is** its layout,
    /// so the design's *Table / Board / Calendar* control writes `blocks/N/kind`
    /// rather than pretending a screen has one layout.
    pub fn set_block_layout(
        config: &ResolvedConfig,
        resources_dir: &Path,
        name: &str,
        index: usize,
        layout: &str,
    ) -> Result<PlannedEdit, EditError> {
        let block = require_block(config, name, index)?;
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        block_key(
            &mut doc_value,
            name,
            index,
            "kind",
            Written::Set(json!(layout)),
        )?;
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "view": name, "block": index, "kind": layout, "was": block.kind }),
        )
        .map(|edit| edit.named(format!("view.setLayout {name} block {index} {layout}")))
    }

    /// `view.block.add <name> --kind <kind> [--index <i>] [--parent <path>] [--value k=v...]` (§3.7).
    /// Adds a block at top level or inside a container block (`--parent`).
    /// Initial properties are required at creation time to prevent committing invalid block configurations.
    pub fn block_add(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let kind = require(params, "kind")?;
        let source = optional(params, "source");
        let parent = parse_container_path(&optional(params, "parent").unwrap_or_default())?;
        let index = optional(params, "index")
            .map(|text| {
                text.parse::<usize>().map_err(|_| {
                    EditError::Usage(format!("index must be a whole number, not \"{text}\""))
                })
            })
            .transpose()?;
        require_view(config, &name)?;
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let pointer = container_pointer(&name, &parent);
        let mut blocks = match doc_json::get(&doc_value, &pointer) {
            Some(Value::Array(items)) => items,
            _ if parent.is_empty() => Vec::new(),
            // A container written by `view.block.add --kind columns` carries no
            // `blocks` key until it has a child: the container exists, so the
            // array is created here rather than refusing the first child.
            _ if doc_json::get(&doc_value, &pointer[..pointer.len() - 1]).is_some() => Vec::new(),
            _ => {
                return Err(EditError::Usage(format!(
                    "view \"{name}\" declares no block at {}",
                    parent.join("/")
                )));
            }
        };
        let mut block = Map::new();
        block.insert("kind".into(), Value::String(kind.clone()));
        if let Some(source) = &source {
            block.insert("view".into(), Value::String(source.clone()));
        }
        for entry in initial_keys(params) {
            block.insert(entry.0, entry.1);
        }
        let at = index.unwrap_or(blocks.len()).min(blocks.len());
        blocks.insert(at, Value::Object(block));
        if !set_at(&mut doc_value, &pointer, Value::Array(blocks)) {
            return Err(EditError::Usage(format!(
                "view \"{name}\" has no block array at {}",
                parent.join("/")
            )));
        }
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "view": name, "index": at, "kind": kind, "parent": parent.join(".") }),
        )
        .map(|edit| edit.named(format!("view.block.add {name} {kind}")))
    }

    /// The block a request addresses: `--path` when given (a nested address,
    /// `0.blocks.2`), `--index` at the top level otherwise — the Phase-5
    /// spelling every script and the CLI already use.
    fn addressed_block(
        name: &str,
        doc: &Value,
        params: &Params,
    ) -> Result<(Vec<String>, Map<String, Value>), EditError> {
        let pointer = match optional(params, "path").filter(|path| !path.is_empty()) {
            Some(path) => {
                let parsed = parse_block_path(&path)?;
                block_pointer(name, &parsed)
            }
            None => {
                let index = require(params, "index")?
                    .parse::<usize>()
                    .map_err(|_| EditError::Usage("index must be a whole number".into()))?;
                container_pointer(name, &[])
                    .into_iter()
                    .chain(std::iter::once(index.to_string()))
                    .collect()
            }
        };
        match doc_json::get(doc, &pointer) {
            Some(Value::Object(object)) => Ok((pointer, object)),
            Some(_) => Err(EditError::Usage(format!(
                "view \"{name}\" block {} is not an object",
                pointer.last().cloned().unwrap_or_default()
            ))),
            None => Err(EditError::Usage(format!(
                "view \"{name}\" declares no block at {}",
                pointer[2..].join("/")
            ))),
        }
    }

    /// `view.block.set` — one JSON key of one block. `--value` is parsed as JSON
    /// when it parses (`12`, `true`, `["a"]`, `{"k":1}`) and kept as a string
    /// otherwise, which is the whole vocabulary a block's keys need.
    pub fn block_set(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let key = require(params, "key")?;
        let value = optional(params, "value");
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let (pointer, mut block) = addressed_block(&name, &doc_value, params)?;
        let written = match value.as_deref() {
            None => Written::Remove,
            Some(text) if text.is_empty() || text == "null" => Written::Remove,
            Some(text) => match serde_json::from_str::<Value>(text) {
                Ok(parsed) if !parsed.is_null() => Written::Set(parsed),
                Ok(_) => Written::Remove,
                Err(_) => Written::Set(Value::String(text.to_string())),
            },
        };
        match written {
            Written::Set(value) => {
                block.insert(key.clone(), value);
            }
            Written::Remove => {
                block.remove(&key);
            }
        }
        let address = pointer[2..].join("/");
        if !set_at(&mut doc_value, &pointer, Value::Object(block)) {
            return Err(EditError::Usage(format!(
                "view \"{name}\" has no block at {address}"
            )));
        }
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "view": name, "path": address, "key": key, "value": value }),
        )
        .map(|edit| edit.named(format!("view.block.set {name} {address} {key}")))
    }

    /// `view.block.remove` — drop one block, addressed by `--path` or `--index`.
    /// Removing the **last** block at the view's top level removes the `blocks`
    /// key, which is what "this view is a single list again" means (§3.6): the
    /// view keeps working, it just stops being a screen.
    pub fn block_remove(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let (pointer, block) = addressed_block(&name, &doc_value, params)?;
        let kind = block.get("kind").cloned().unwrap_or(Value::Null);
        let (last, parents) = pointer.split_last().expect("a block pointer names a block");
        let at = last.parse::<usize>().unwrap_or(0);
        let Some(Value::Array(mut items)) = doc_json::get(&doc_value, parents) else {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares no blocks"
            )));
        };
        if at >= items.len() {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares {} block(s); {at} is out of range",
                items.len()
            )));
        }
        items.remove(at);
        let address = pointer[2..].join("/");
        if parents.len() == 3 && items.is_empty() {
            doc_json::remove(&mut doc_value, parents)?;
        } else if !set_at(&mut doc_value, parents, Value::Array(items)) {
            return Err(EditError::Usage(format!(
                "view \"{name}\" has no block array at {address}"
            )));
        }
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({ "view": name, "path": address, "kind": kind }),
        )
        .map(|edit| edit.named(format!("view.block.remove {name} {address}")))
    }

    /// `view.block.move` — move one block to a position in a container (the same
    /// one or another), which is what a block **tree** needs and an index alone
    /// cannot say (§6.2 U6). `--parent` is the destination container (empty = the
    /// view's top level) and `--index` is the position the block lands at,
    /// counted **after** it leaves its old place.
    pub fn block_move(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let path = parse_block_path(&require(params, "path")?)?;
        let parent = parse_container_path(&optional(params, "parent").unwrap_or_default())?;
        let index = optional(params, "index")
            .map(|text| {
                text.parse::<usize>().map_err(|_| {
                    EditError::Usage(format!("index must be a whole number, not \"{text}\""))
                })
            })
            .transpose()?;
        require_view(config, &name)?;
        let mut doc_value = type_ops::doc_with_schema(config, "content/views.json", Some("views"))?;
        let from = block_pointer(&name, &path);
        let Some(Value::Object(block)) = doc_json::get(&doc_value, &from) else {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares no block at {}",
                path.join(".")
            )));
        };
        let to = container_pointer(&name, &parent);
        // A block cannot land inside itself: the destination array sits below the
        // block that is moving.
        if to.len() >= from.len() && to[..from.len()] == from[..] {
            return Err(EditError::Usage(format!(
                "a block cannot be moved inside itself ({} into {})",
                path.join("."),
                parent.join(".")
            )));
        }
        let (last, from_parents) = from.split_last().expect("a block pointer names a block");
        let at = last.parse::<usize>().unwrap_or(0);
        let Some(Value::Array(mut items)) = doc_json::get(&doc_value, from_parents) else {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares no blocks"
            )));
        };
        if at >= items.len() {
            return Err(EditError::Usage(format!(
                "view \"{name}\" declares {} block(s); {at} is out of range",
                items.len()
            )));
        }
        items.remove(at);
        if from_parents.len() == 3 && items.is_empty() {
            doc_json::remove(&mut doc_value, from_parents)?;
        } else if !set_at(&mut doc_value, from_parents, Value::Array(items)) {
            return Err(EditError::Usage(format!(
                "view \"{name}\" has no block array at {}",
                path.join(".")
            )));
        }
        let mut target = match doc_json::get(&doc_value, &to) {
            Some(Value::Array(items)) => items,
            _ if parent.is_empty() => Vec::new(),
            // Same rule as `block_add`: a container that exists may receive its
            // first child.
            _ if doc_json::get(&doc_value, &to[..to.len() - 1]).is_some() => Vec::new(),
            _ => {
                return Err(EditError::Usage(format!(
                    "view \"{name}\" declares no block at {}",
                    parent.join("/")
                )));
            }
        };
        let landing = index.unwrap_or(target.len()).min(target.len());
        target.insert(landing, Value::Object(block));
        if !set_at(&mut doc_value, &to, Value::Array(target)) {
            return Err(EditError::Usage(format!(
                "view \"{name}\" has no block array at {}",
                parent.join("/")
            )));
        }
        let changes = vec![doc_change(config, "content/views.json", &doc_value)];
        finalize_with(
            config,
            resources_dir,
            changes,
            json!({
                "view": name,
                "path": path.join("."),
                "parent": parent.join("."),
                "index": landing,
            }),
        )
        .map(|edit| edit.named(format!("view.block.move {name} {}", path.join("."))))
    }

    /// `view.delete <name>` — remove a saved view, the destinations that opened
    /// it and every block or metric that drew it (§6.2 U6; `UI_SPEC.md` §4 E2).
    pub fn delete_view(
        config: &ResolvedConfig,
        resources_dir: &Path,
        params: &Params,
    ) -> Result<PlannedEdit, EditError> {
        let name = require(params, "name")?;
        let view = require_view(config, &name)?;
        let kind = view.type_.clone();
        let title = destination_title(config, &name);
        let mut cascade = Cascade::start(Vec::new())?;
        let recreated = remove_view(&mut cascade, config, &name, kind.as_deref(), title)?;
        let preview = json!({
            "view": name,
            "kind": kind,
            "recreated": recreated,
        });
        cascade
            .finish(config, resources_dir, preview)
            .map(|edit| edit.named(format!("removed the view \"{name}\"")))
    }

    /// Shared helper for view deletion (`view.delete`, `list.delete`).
    /// Deletes view definition and associated destinations. If deleting the last view
    /// for a schema kind, recreates `<kind>.all` default table to maintain navigation.
    /// For composed screens with no associated kind, `kind` is `None` (COMPOSER §2.1).
    pub(crate) fn remove_view(
        cascade: &mut Cascade,
        config: &ResolvedConfig,
        name: &str,
        kind: Option<&str>,
        title: Option<String>,
    ) -> Result<Value, EditError> {
        {
            let doc_value = cascade.doc(config, "content/views.json", Some("views"))?;
            doc_json::remove(doc_value, &["views".into(), name.to_string()])?;
        }
        let doomed = BTreeSet::from([name.to_string()]);
        drop_destinations(cascade, config, &doomed)?;

        // A type-less view has no kind to recreate.
        let Some(kind) = kind else {
            return Ok(Value::Null);
        };
        let kind_keeps_a_view = config
            .views
            .iter()
            .any(|(other, view)| other != name && view.type_.as_deref() == Some(kind));
        if kind_keeps_a_view {
            return Ok(Value::Null);
        }
        let default = format!("{kind}.all");
        {
            let doc_value = cascade.doc(config, "content/views.json", Some("views"))?;
            doc_json::set(
                doc_value,
                &["views".into(), default.clone()],
                json!({ "type": kind, "layout": "table" }),
            )?;
        }
        // A destination is added only when the view had one: the rail speaks the
        // student's titles, and an entry named after an id is not a title.
        if let Some(title) = title {
            add_destination(cascade, config, &title, &default)?;
        }
        Ok(json!(default))
    }
}

pub mod source_ops {
    use super::*;

    /// Is `file` an authoritative document the pane may commit? Path safety by
    /// role, never by caller-supplied string (§4.6).
    pub fn is_authoritative(file: &str) -> bool {
        if DOCUMENTS.contains(&file) {
            return true;
        }
        file.strip_prefix("content/records/")
            .and_then(|rest| rest.strip_suffix(".jsonl"))
            .is_some_and(type_ops::safe_type_id)
    }

    /// `source.apply` — the text door's commit: replace one document (or one
    /// records file) with caller text, canonicalized and validated whole-plan.
    /// This is what the ⌘⇧J pane and a hand edit share.
    pub fn apply(
        config: &ResolvedConfig,
        resources_dir: &Path,
        file: &str,
        text: &str,
        dry_run: bool,
    ) -> Result<PlannedEdit, EditError> {
        if !is_authoritative(file) {
            return Err(EditError::Usage(format!(
                "{file} is not an authoritative document (content/*.json, state/state.json, content/records/<type>.jsonl)"
            )));
        }
        let after: Vec<u8> = if file.ends_with(".jsonl") {
            let normalized = text.replace("\r\n", "\n");
            let mut lines = Vec::new();
            for (index, raw) in normalized.split('\n').enumerate() {
                if raw.trim().is_empty() {
                    continue;
                }
                let value = strict_json::parse_line(raw, file, index as u64 + 1)?;
                let record = decode::record(&value, file, index as u64 + 1)?;
                lines.push(ResolvedConfig::canonical_line(&record).map_err(EditError::Io)?);
            }
            if lines.is_empty() {
                Vec::new()
            } else {
                format!("{}\n", lines.join("\n")).into_bytes()
            }
        } else {
            doc_json::pretty(&doc_json::parse(text, file)?).into_bytes()
        };
        let before = config.source_files.get(file).cloned();
        if before.as_deref() == Some(after.as_slice()) {
            return Ok(PlannedEdit {
                changes: Vec::new(),
                summary: format!("source.apply {file} (no-op)"),
                preview: None,
            });
        }
        let changes = vec![FileChange::new(file, before, Some(after))];
        if dry_run {
            return Ok(PlannedEdit {
                changes,
                summary: format!("source.apply {file} (dry-run)"),
                preview: None,
            });
        }
        finalize_with(config, resources_dir, changes, json!({ "file": file }))
            .map(|edit| edit.named(format!("source.apply {file}")))
    }
}

// ── undo/redo inverses (§4.8 Principle 6) ────────────────────────────────────

/// The engine-side inverse every UI undo and redo uses: **byte-exact
/// before-images restored through the SAME validated transaction path**. A later
/// external commit makes the bytes mismatch and the engine refuses, so stale
/// undo can never erase a newer write.
pub mod undo_engine {
    use super::*;

    pub fn invert(changes: &[FileChange]) -> Vec<FileChange> {
        changes
            .iter()
            .map(|change| {
                FileChange::new(
                    change.path.clone(),
                    change.after.clone(),
                    change.before.clone(),
                )
            })
            .collect()
    }

    /// Validate + commit an inverse. `from_revision` is the revision the undo
    /// entry expects; the engine's own byte check makes it safe.
    pub fn apply(
        root: &Path,
        resources_dir: &Path,
        changes: &[FileChange],
        from_revision: &str,
        summary: &str,
    ) -> Result<transaction::CommitResult, EditError> {
        let hashes = config_store::source_file_hashes(root);
        let new_revision = transaction::revision_after(&hashes, changes);
        transaction::validate_by_overlay(root, changes, resources_dir)?;
        transaction::commit(root, changes, from_revision, &new_revision, summary)
            .map_err(EditError::Transaction)
    }
}

/// The source pane's pre-commit syntax check: **every** finding, line-precise,
/// through the same strict parser the loader uses (§4.8 P5).
pub mod pane_precheck {
    use super::*;

    pub fn diagnostics(file: &str, text: &str) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        if file.ends_with(".jsonl") {
            let normalized = text.replace("\r\n", "\n");
            for (index, raw) in normalized.split('\n').enumerate() {
                if raw.trim().is_empty() {
                    continue;
                }
                if let Err(error) = strict_json::parse_line(raw, file, index as u64 + 1) {
                    out.push(error.diagnostic);
                }
            }
        } else if let Err(error) = strict_json::parse(text, file) {
            out.push(error.diagnostic);
        }
        out
    }
}

/// `record.reveal` — the address contract of §4.3, rendered for humans and for
/// clipboard use: file-relative path plus an id selector (JSONL) or a JSON
/// Pointer (documents).
pub fn record_address(positioned: &PositionedRecord) -> String {
    format!(
        "{}#{}",
        positioned.file,
        escape_pointer_segment(&positioned.record.id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_registry;

    /// The seed preset, where it lives — the read-only posture the other phase
    /// tests use. Planning writes nothing; `validate_by_overlay` works on its own
    /// throwaway copy, so no test needs a writable plan root here.
    fn seed() -> (ResolvedConfig, std::path::PathBuf) {
        let resources = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Sources/SAMCore/Resources");
        let config =
            config_store::load_with(&resources.join("presets/seed"), "explicit", &resources)
                .expect("the seed loads");
        (config, resources)
    }

    #[test]
    fn the_pretty_form_sorts_keys_and_keeps_scalar_arrays_on_one_line() {
        let value: Value =
            serde_json::from_str(r#"{"b":1,"a":["x","y"],"c":{"z":[{"k":1}]}}"#).unwrap();
        let text = doc_json::pretty(&value);
        assert_eq!(
            text,
            "{\n  \"a\": [\"x\", \"y\"],\n  \"b\": 1,\n  \"c\": {\n    \"z\": [\n      {\n        \"k\": 1\n      }\n    ]\n  }\n}\n"
        );
    }

    #[test]
    fn surgical_path_edits_leave_unknown_keys_alone() {
        let mut value: Value =
            serde_json::from_str(r#"{"types":{"topic":{"fields":[],"unknown":true}}}"#).unwrap();
        let path = vec![
            "types".to_string(),
            "topic".to_string(),
            "fields".to_string(),
        ];
        doc_json::append(
            &mut value,
            &path,
            json!({ "key": "est", "type": "duration" }),
        )
        .unwrap();
        doc_json::set(
            &mut value,
            &["types".into(), "topic".into(), "icon".into()],
            json!("bolt"),
        )
        .unwrap();
        assert_eq!(value["types"]["topic"]["unknown"], json!(true));
        assert_eq!(value["types"]["topic"]["fields"][0]["key"], json!("est"));
        doc_json::remove(&mut value, &path).unwrap();
        let _ = value;
    }

    /// A writable copy of the seed, for the one case a preview cannot show:
    /// what the file looks like after an edit whose *input* is another edit's
    /// bytes. (`finalize_with` validates on its own throwaway copy, so nothing
    /// else here needs a writable root.)
    fn temp_seed(resources: &std::path::Path, tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("sam-view-ops-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy_dir(&resources.join("presets/seed"), &root);
        root
    }

    fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    #[test]
    fn a_view_key_write_lands_on_its_key_and_a_cleared_key_is_removed() {
        let (config, resources) = seed();
        let edit = view_ops::set_query(
            &config,
            &resources,
            "today.topics",
            "filter",
            Some("complete()"),
        )
        .expect("a filter write");
        let after = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.topics"]["filter"],
            json!("complete()")
        );
        // The view is edited, never rewritten: every other key survives.
        assert_eq!(value["views"]["today.topics"]["group"], json!("kind"));
        assert_eq!(value["views"]["today.topics"]["limit"], json!(12));

        // Clearing removes the key rather than writing an empty string — an
        // undeclared filter and `"filter": ""` are different documents.
        let cleared = view_ops::set_query(&config, &resources, "today.topics", "filter", None)
            .expect("a clear");
        let after = String::from_utf8(cleared.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert!(
            value["views"]["today.topics"].get("filter").is_none(),
            "a cleared key is gone, not empty"
        );
        assert_eq!(
            value["views"]["today.topics"]["group"],
            json!("kind"),
            "and nothing else went with it"
        );
    }

    fn metric_params(entries: &[(&str, &str)]) -> Params {
        let mut params = Params::new();
        for (key, value) in entries {
            params.insert((*key).into(), ParamValue::Str((*value).into()));
        }
        params
    }

    /// The document one planned edit would write, as the validator saw it.
    fn written_document_of(edit: &PlannedEdit) -> Value {
        let text = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    /// One derived figure is one whole-plan validated write of its map member
    /// (§3.1): a created entry carries what the request named, a later write
    /// touches only the keys it was given, and a cleared key is *removed*.
    #[test]
    fn a_figure_write_creates_updates_and_clears_one_map_member() {
        let (config, resources) = seed();
        let created = metric_ops::set(
            &config,
            &resources,
            &metric_params(&[
                ("name", "test.figure"),
                ("label", "Test figure"),
                ("view", "today.topics"),
                ("expr", "est"),
                ("reduce", "sum"),
                ("unit", "min"),
            ]),
        )
        .expect("a figure write");
        let value = written_document_of(&created);
        assert_eq!(
            value["metrics"]["test.figure"],
            json!({
                "label": "Test figure",
                "view": "today.topics",
                "expr": "est",
                "reduce": "sum",
                "unit": "min",
            })
        );
        assert_eq!(
            value["metrics"]["minutes.logged"]["unit"],
            json!("min"),
            "the figures already there are not rewritten"
        );
        assert_eq!(created.preview.as_ref().unwrap()["created"], json!(true));

        // Only the keys the request names move: the label changes, the fold
        // stays, and the preview says this was an edit rather than a creation.
        let updated = metric_ops::set(
            &config,
            &resources,
            &metric_params(&[
                ("name", "minutes.logged"),
                ("label", "Minutes in the chair"),
            ]),
        )
        .expect("a label write");
        let value = written_document_of(&updated);
        assert_eq!(
            value["metrics"]["minutes.logged"]["label"],
            json!("Minutes in the chair")
        );
        assert_eq!(value["metrics"]["minutes.logged"]["expr"], json!("min"));
        assert_eq!(updated.preview.as_ref().unwrap()["created"], json!(false));

        // Clearing removes the key rather than writing an empty string — an
        // undeclared unit and `"unit": ""` are different figures.
        let cleared = metric_ops::set(
            &config,
            &resources,
            &metric_params(&[("name", "minutes.logged"), ("unit", ""), ("reduce", "")]),
        )
        .expect("a clear");
        let value = written_document_of(&cleared);
        assert!(value["metrics"]["minutes.logged"].get("unit").is_none());
        assert!(value["metrics"]["minutes.logged"].get("reduce").is_none());
        assert_eq!(value["metrics"]["minutes.logged"]["expr"], json!("min"));
    }

    /// The validator owns the closed vocabularies and sees the candidate plan
    /// whole (`finalize_with`), so a figure naming a view the plan does not
    /// declare, a fold the engine does not know, or an expression that does not
    /// parse never reaches the plan file.
    #[test]
    fn a_figure_pointing_where_the_plan_has_nothing_is_refused() {
        let (config, resources) = seed();
        let dead_view = metric_ops::set(
            &config,
            &resources,
            &metric_params(&[
                ("name", "ghost.figure"),
                ("label", "Ghost"),
                ("view", "nope.such"),
            ]),
        )
        .expect_err("an undeclared view");
        assert!(format!("{dead_view:?}").contains("metric.unknown-view"));

        let bad_reduce = metric_ops::set(
            &config,
            &resources,
            &metric_params(&[
                ("name", "ghost.figure"),
                ("label", "Ghost"),
                ("view", "today.topics"),
                ("reduce", "median"),
            ]),
        )
        .expect_err("a fold outside the vocabulary");
        assert!(format!("{bad_reduce:?}").contains("metric.bad-reduce"));

        let bad_expr = metric_ops::set(
            &config,
            &resources,
            &metric_params(&[
                ("name", "ghost.figure"),
                ("label", "Ghost"),
                ("view", "today.topics"),
                ("expr", "est +"),
            ]),
        )
        .expect_err("an expression that does not parse");
        assert!(format!("{bad_expr:?}").contains("expr.syntax"));
    }

    /// A figure has to stay loadable, so the two keys `MetricDef` insists on
    /// cannot be cleared, and a *new* figure has to name both.
    #[test]
    fn a_figure_keeps_its_label_and_view() {
        let (config, resources) = seed();
        for params in [
            metric_params(&[("name", "minutes.logged"), ("label", "")]),
            metric_params(&[("name", "minutes.logged"), ("view", "")]),
            metric_params(&[("name", "test.figure"), ("label", "Test"), ("unit", "min")]),
            metric_params(&[("name", "minutes.logged")]),
        ] {
            let refused = metric_ops::set(&config, &resources, &params).expect_err("a refusal");
            assert!(matches!(&refused, EditError::Usage(_)), "{refused:?}");
        }
    }

    fn components_params(
        name: &str,
        surfaces: Option<&str>,
        spans: Option<&str>,
        clear: bool,
    ) -> Params {
        let mut params: Params = Params::new();
        params.insert("name".into(), ParamValue::Str(name.into()));
        if let Some(surfaces) = surfaces {
            params.insert("surfaces".into(), ParamValue::Str(surfaces.into()));
        }
        if let Some(spans) = spans {
            params.insert("spans".into(), ParamValue::Str(spans.into()));
        }
        if clear {
            params.insert("clear".into(), ParamValue::Bool(true));
        }
        params
    }

    /// The composer's one write (COMPOSER §2.2): add, reorder and clear all go
    /// through `view.setComponents`, and a surface already on the screen keeps
    /// its own object so a member this build does not know survives.
    #[test]
    fn a_component_write_adds_reorders_and_clears_while_keeping_unknown_members() {
        let (config, resources) = seed();
        let added = view_ops::set_components(
            &config,
            &resources,
            &components_params("today.screen", Some("week-chart, today-focus"), None, false),
        )
        .expect("a composition write");
        let after = String::from_utf8(added.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.screen"]["components"],
            json!([{ "surface": "week-chart" }, { "surface": "today-focus" }])
        );
        // The view is edited, never rewritten: its query keys survive.
        assert_eq!(value["views"]["today.screen"]["layout"], json!("list"));

        // A reorder preserves each component's own object — the surface that was
        // there keeps an unknown member a newer build put on it.
        let root = temp_seed(&resources, "components");
        let views_path = root.join("content/views.json");
        let mut document: Value =
            serde_json::from_str(&std::fs::read_to_string(&views_path).unwrap()).unwrap();
        document["views"]["today.screen"]["components"] =
            json!([{ "surface": "week-chart", "carried": true }]);
        std::fs::write(&views_path, serde_json::to_string(&document).unwrap()).unwrap();
        let carried =
            config_store::load_with(&root, "explicit", &resources).expect("the copy loads");
        let reordered = view_ops::set_components(
            &carried,
            &resources,
            &components_params("today.screen", Some("today-focus,week-chart"), None, false),
        )
        .expect("a reorder");
        let after = String::from_utf8(reordered.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.screen"]["components"],
            json!([
                { "surface": "today-focus" },
                { "surface": "week-chart", "carried": true }
            ]),
            "the reorder carried the unknown member with its surface"
        );

        // `--clear` removes the member: the screen falls back to its panel or its
        // blocks.
        let cleared = view_ops::set_components(
            &carried,
            &resources,
            &components_params("today.screen", None, None, true),
        )
        .expect("a clear");
        let after = String::from_utf8(cleared.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert!(value["views"]["today.screen"].get("components").is_none());
        // `--clear` and `--surfaces` are two different writes.
        assert!(
            view_ops::set_components(
                &carried,
                &resources,
                &components_params("today.screen", Some("week-chart"), None, true),
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A width write (§2.1): `--spans` states one number per surface, `1` is
    /// written and the default `2` is **removed**, so the file says only what it
    /// has to. The payload answers with the widths the screen now holds — the
    /// read-back the composer sizes its frames by.
    #[test]
    fn a_span_of_one_is_written_and_the_default_two_is_removed() {
        let (config, resources) = seed();
        let edit = view_ops::set_components(
            &config,
            &resources,
            &components_params(
                "today.screen",
                Some("week-chart, today-focus"),
                Some("1,2"),
                false,
            ),
        )
        .expect("a width write");
        let after = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.screen"]["components"],
            json!([{ "surface": "week-chart", "span": 1 }, { "surface": "today-focus" }]),
            "half the row is written; the default whole row is not"
        );
        let preview = edit.preview.expect("the write reports itself");
        assert_eq!(preview["spans"], json!([1, 2]));
        assert_eq!(
            preview["surfaces"],
            json!([{ "surface": "week-chart", "span": 1 }, { "surface": "today-focus" }]),
            "the read-back is the members the plan now holds"
        );
    }

    /// A width write is still an in-place edit: a carried component keeps its
    /// unknown members, and a write that says nothing about widths changes
    /// nothing about them.
    #[test]
    fn a_carried_component_keeps_its_span_across_a_reorder_and_a_width_write() {
        let (_config, resources) = seed();
        let root = temp_seed(&resources, "spans");
        let views_path = root.join("content/views.json");
        let mut document: Value =
            serde_json::from_str(&std::fs::read_to_string(&views_path).unwrap()).unwrap();
        document["views"]["today.screen"]["components"] = json!([
            { "surface": "week-chart", "span": 1, "carried": true },
            { "surface": "today-focus" }
        ]);
        std::fs::write(&views_path, serde_json::to_string(&document).unwrap()).unwrap();
        let carried =
            config_store::load_with(&root, "explicit", &resources).expect("the copy loads");

        // A reorder with no `--spans` says nothing about widths.
        let reordered = view_ops::set_components(
            &carried,
            &resources,
            &components_params("today.screen", Some("today-focus,week-chart"), None, false),
        )
        .expect("a reorder");
        let after = String::from_utf8(reordered.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.screen"]["components"],
            json!([
                { "surface": "today-focus" },
                { "surface": "week-chart", "span": 1, "carried": true }
            ]),
            "the carried object kept its own span and its unknown member"
        );
        assert_eq!(reordered.preview.unwrap()["spans"], json!([2, 1]));

        // A width write flips the same screen back to the whole row: `span` is
        // this command's own member, so `2` removes it and `carried` stays.
        let widened = view_ops::set_components(
            &carried,
            &resources,
            &components_params(
                "today.screen",
                Some("today-focus,week-chart"),
                Some("1,2"),
                false,
            ),
        )
        .expect("a width write");
        let after = String::from_utf8(widened.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.screen"]["components"],
            json!([
                { "surface": "today-focus", "span": 1 },
                { "surface": "week-chart", "carried": true }
            ])
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `--spans` is one width per surface or a usage error: a list of another
    /// length, a value that is not 1 or 2, and a width with no surface list are
    /// each refused before the document is touched.
    #[test]
    fn a_span_list_that_is_not_one_width_per_surface_is_a_usage_error() {
        let (config, resources) = seed();
        let attempt = |surfaces: Option<&str>, spans: Option<&str>| {
            view_ops::set_components(
                &config,
                &resources,
                &components_params("today.screen", surfaces, spans, false),
            )
        };
        let refused = |surfaces: Option<&str>, spans: Option<&str>, says: &str| {
            let error = attempt(surfaces, spans).expect_err(says);
            match error {
                EditError::Usage(why) => {
                    assert!(why.contains(says), "says \"{why}\", wanted {says}")
                }
                other => panic!("expected a usage error, got {other:?}"),
            }
        };
        refused(
            Some("week-chart,today-focus"),
            Some("1"),
            "give one per surface",
        );
        refused(Some("week-chart"), Some("1,2,1"), "give one per surface");
        // 3 is wider than this build draws and 0 cannot be drawn at all: both
        // are the *file's* business (component.span-wide/-invalid), and neither
        // is a width a caller may ask for.
        refused(Some("week-chart,today-focus"), Some("1,3"), "got \"3\"");
        refused(Some("week-chart,today-focus"), Some("0,1"), "got \"0\"");
        refused(
            Some("week-chart,today-focus"),
            Some("half,1"),
            "got \"half\"",
        );
        refused(None, Some("1"), "it needs --surfaces");
        // The empty screen stays writable: an empty list has no width to state.
        assert!(
            attempt(Some(""), Some("")).is_ok(),
            "`--spans \"\"` beside `--surfaces \"\"` is the empty composition"
        );
    }

    /// An absent `--surfaces` is the empty state, not a missing key: the file
    /// states `components: []`.
    #[test]
    fn an_empty_surface_list_is_a_written_empty_composition() {
        let (config, resources) = seed();
        let edit = view_ops::set_components(
            &config,
            &resources,
            &components_params("today.screen", None, None, false),
        )
        .expect("an empty composition");
        let after = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(value["views"]["today.screen"]["components"], json!([]));
    }

    #[test]
    fn a_list_with_no_type_is_an_empty_composed_screen_with_its_icon() {
        let (config, resources) = seed();
        let mut params: Params = Params::new();
        params.insert("name".into(), ParamValue::Str("Focus".into()));
        params.insert("icon".into(), ParamValue::Str("target".into()));
        let edit = list_ops::new_list(&config, &resources, &params).expect("a screen");
        let views = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&views).unwrap();
        assert_eq!(
            value["views"]["focus.all"],
            json!({ "components": [] }),
            "the new screen is empty and composed"
        );
        let shell = String::from_utf8(edit.changes[1].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&shell).unwrap();
        let entry = value["navigation"].as_array().unwrap().last().unwrap();
        assert_eq!(
            entry,
            &json!({ "title": "Focus", "view": "focus.all", "icon": "target" })
        );
        assert_eq!(
            edit.preview.as_ref().unwrap()["view"],
            json!("focus.all"),
            "the result is still the view id"
        );

        // A declared but unknown kind still refuses, with the same words.
        let mut unknown: Params = Params::new();
        unknown.insert("name".into(), ParamValue::Str("Nowhere".into()));
        unknown.insert("type".into(), ParamValue::Str("nowhere".into()));
        let error = list_ops::new_list(&config, &resources, &unknown)
            .expect_err("an unknown type is refused");
        assert!(
            error.to_string().contains("unknown type \"nowhere\""),
            "the refusal is unchanged: {error}"
        );
    }

    #[test]
    fn a_block_write_reaches_into_the_array_and_removing_the_last_unscreens_the_view() {
        let (config, resources) = seed();
        let mut params: Params = Params::new();
        params.insert("name".into(), ParamValue::Str("today.screen".into()));
        params.insert("index".into(), ParamValue::Str("1".into()));
        params.insert("key".into(), ParamValue::Str("label".into()));
        params.insert("value".into(), ParamValue::Str("Planned minutes".into()));
        let edit = view_ops::block_set(&config, &resources, &params).expect("a block key write");
        let after = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            value["views"]["today.screen"]["blocks"][1]["label"],
            json!("Planned minutes")
        );
        assert_eq!(
            value["views"]["today.screen"]["blocks"][1]["expr"],
            json!("est"),
            "the keys beside it are untouched"
        );

        // Removing the last block removes `blocks` itself: the view goes back to
        // drawing one record block in its own layout (§3.6). Every op reads the
        // plan's bytes, so this half needs those bytes: a temp copy, written
        // through the change the previous op produced, exactly as the dispatcher
        // would.
        let root = temp_seed(&resources, "last-block");
        let mut current =
            config_store::load_with(&root, "explicit", &resources).expect("the copy loads");
        assert_eq!(
            current.views["today.screen"].blocks.as_ref().unwrap().len(),
            3
        );
        let mut last: Option<Vec<u8>> = None;
        while current.views["today.screen"]
            .blocks
            .as_deref()
            .is_some_and(|blocks| !blocks.is_empty())
        {
            let mut remove: Params = Params::new();
            remove.insert("name".into(), ParamValue::Str("today.screen".into()));
            remove.insert("index".into(), ParamValue::Str("0".into()));
            let edit = view_ops::block_remove(&current, &resources, &remove).expect("a removal");
            let after = edit.changes[0].after.clone().unwrap();
            std::fs::write(root.join("content/views.json"), &after).unwrap();
            last = Some(after);
            current =
                config_store::load_with(&root, "explicit", &resources).expect("the copy loads");
        }
        let value: Value = serde_json::from_slice(&last.unwrap()).unwrap();
        assert!(
            value["views"]["today.screen"].get("blocks").is_none(),
            "the blocks key is gone with its last block"
        );
        assert_eq!(
            value["views"]["today.screen"]["layout"],
            json!("list"),
            "the view keeps its own layout, which is now what it draws"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_new_block_carries_its_initial_keys_and_the_validator_refuses_a_half_built_one() {
        let (config, resources) = seed();
        let mut bare: Params = Params::new();
        bare.insert("name".into(), ParamValue::Str("today.screen".into()));
        bare.insert("kind".into(), ParamValue::Str("callout".into()));
        assert!(
            view_ops::block_add(&config, &resources, &bare).is_err(),
            "a callout with no text is not a block the loader accepts"
        );

        let mut with_text: Params = Params::new();
        with_text.insert("name".into(), ParamValue::Str("today.screen".into()));
        with_text.insert("kind".into(), ParamValue::Str("callout".into()));
        with_text.insert(
            "value".into(),
            ParamValue::Strings(vec!["text=Read the week before you plan it".into()]),
        );
        let edit = view_ops::block_add(&config, &resources, &with_text).expect("a callout");
        let after = String::from_utf8(edit.changes[0].after.clone().unwrap()).unwrap();
        let value: Value = serde_json::from_str(&after).unwrap();
        let blocks = value["views"]["today.screen"]["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[3]["kind"], json!("callout"));
        assert_eq!(
            blocks[3]["text"],
            json!("Read the week before you plan it"),
            "the initial key lands in the same transaction as the block"
        );
    }

    /// A loaded preset, read-only, for the delete cascades.
    fn preset(name: &str) -> (ResolvedConfig, std::path::PathBuf) {
        let resources = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../Sources/SAMCore/Resources");
        let config = config_store::load_with(
            &resources.join(format!("presets/{name}")),
            "explicit",
            &resources,
        )
        .expect("the preset loads");
        (config, resources)
    }

    fn params(pairs: &[(&str, &str)]) -> Params {
        let mut params = Params::new();
        for (name, value) in pairs {
            params.insert((*name).into(), ParamValue::Str((*value).into()));
        }
        params
    }

    /// One changed document as JSON, so a test reads the candidate the way the
    /// validator saw it.
    fn document<'a>(changes: &'a [FileChange], path: &str) -> &'a Value {
        let change = changes
            .iter()
            .find(|change| change.path == path)
            .unwrap_or_else(|| panic!("{path} is not among the changes"));
        let after = change.after.as_ref().expect("a change has an after-image");
        // Leaked on purpose: a test's life is the process's life.
        Box::leak(Box::new(
            serde_json::from_slice::<Value>(after).expect("the change is a document"),
        ))
    }

    #[test]
    fn deleting_a_kind_schema_mode_refuses_while_records_exist() {
        let (config, resources) = preset("jee");
        let edit = type_ops::delete(
            &config,
            &resources,
            &params(&[("name", "course"), ("mode", "schema")]),
        );
        match edit {
            Err(EditError::Invalid(diagnostics)) => {
                assert_eq!(diagnostics[0].code, "type.has-records");
                assert!(
                    diagnostics[0].message.contains("record(s)"),
                    "the refusal states the count: {}",
                    diagnostics[0].message
                );
            }
            other => panic!("schema mode must refuse a kind with records, got {other:?}"),
        }

        // A kind with no records may go in schema mode, and unknown names are a
        // usage error rather than a validation one.
        let (config, resources) = preset("seed");
        let empty = type_ops::delete(
            &config,
            &resources,
            &params(&[("name", "milestone"), ("mode", "schema")]),
        );
        assert!(
            matches!(empty, Err(EditError::Invalid(_)) | Err(EditError::Usage(_))) || empty.is_ok()
        );
        assert!(matches!(
            type_ops::delete(
                &config,
                &resources,
                &params(&[("name", "nothing-named-this")])
            ),
            Err(EditError::Usage(_))
        ));
        assert!(matches!(
            type_ops::delete(
                &config,
                &resources,
                &params(&[("name", "milestone"), ("mode", "maybe")])
            ),
            Err(EditError::Usage(_))
        ));
    }

    #[test]
    fn deleting_a_kind_takes_its_records_relations_views_and_the_expressions_that_read_them() {
        let (config, resources) = preset("jee");
        let course_records = config
            .records_iter()
            .filter(|record| record.type_ == "course")
            .count();
        assert!(course_records > 0, "the fixture needs records to delete");

        let edit = type_ops::delete(
            &config,
            &resources,
            &params(&[("name", "course"), ("mode", "records")]),
        )
        .expect("a kind delete");

        let preview = edit.preview.as_ref().unwrap();
        assert_eq!(preview["kind"], json!("course"), "{preview}");
        assert_eq!(preview["records"], json!(course_records));
        assert!(
            preview["relations"].as_u64().unwrap_or(0) >= 1,
            "every relation column that pointed at the kind goes with it: {preview}"
        );
        assert!(
            preview["removed"]["viewKeys"].as_u64().unwrap_or(0) >= 1,
            "the sort and group keys that read through the relation are cleared: {preview}"
        );

        let types = document(&edit.changes, "content/types.json");
        assert!(types["types"].get("course").is_none(), "the kind is gone");
        let topic = types["types"]["topic"]["fields"].as_array().unwrap();
        assert!(
            topic.iter().all(|field| field["key"] != json!("course")),
            "the relation column that pointed at the kind went with it"
        );

        let views = document(&edit.changes, "content/views.json");
        assert!(
            views["views"].get("subjects.courses").is_none(),
            "the kind's own view is gone"
        );
        for name in ["plan.screen", "plan.units", "subjects.screen"] {
            assert_ne!(
                views["views"][name].get("group"),
                Some(&json!("course.name")),
                "{name} grouped by the kind that is gone — the cascade clears the key"
            );
        }
        assert!(
            views["views"]["plan.screen"].get("sort").is_some(),
            "a key that still resolves stays"
        );

        // Every record file the kind owned is either removed or empty of it: the
        // delete is a record pass, not a document pass.
        for change in &edit.changes {
            if change.path.starts_with("content/records/course") {
                assert!(
                    change.after.is_none(),
                    "a kind with no records left keeps no file"
                );
            }
        }

        // The changed files are the ones the cascade touched, and the plan it
        // leaves behind is one the validator accepted (finalize_with ran).
        let paths: Vec<&String> = edit.changes.iter().map(|change| &change.path).collect();
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with("content/types.json"))
        );
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with("content/views.json"))
        );
    }

    #[test]
    fn deleting_a_view_takes_its_destination_and_keeps_the_kind_reachable() {
        let (config, resources) = preset("seed");
        // `programs.all` is the only view of `program`, and no other view draws
        // it: the delete recreates a default so the kind stays reachable.
        let edit = view_ops::delete_view(&config, &resources, &params(&[("name", "programs.all")]))
            .expect("a view delete");
        let views = document(&edit.changes, "content/views.json");
        assert!(views["views"].get("programs.all").is_none());
        assert_eq!(
            views["views"]["program.all"]["type"],
            json!("program"),
            "the kind keeps a view, named by the engine's own convention"
        );
        assert_eq!(
            edit.preview.as_ref().unwrap()["recreated"],
            json!("program.all")
        );

        // `library.screen` is on the rail: its destination goes with it.
        let edit =
            view_ops::delete_view(&config, &resources, &params(&[("name", "library.screen")]))
                .expect("a destination view delete");
        let shell = document(&edit.changes, "content/shell.json");
        assert!(
            shell["navigation"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["view"] != json!("library.screen")),
            "the rail entry that opened it went with it"
        );
        assert_eq!(edit.preview.as_ref().unwrap()["recreated"], Value::Null);
    }

    #[test]
    fn removing_a_destination_keeps_the_view_and_keep_view_false_deletes_it() {
        let (config, resources) = preset("seed");
        let edit = list_ops::delete(&config, &resources, &params(&[("name", "library.screen")]))
            .expect("a destination removal");
        let shell = document(&edit.changes, "content/shell.json");
        assert!(
            shell["navigation"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["view"] != json!("library.screen"))
        );
        assert!(
            edit.changes
                .iter()
                .all(|change| !change.path.ends_with("content/views.json")),
            "the view survives as a saved, non-navigable view"
        );

        let mut delete_view: Params = params(&[("name", "library.screen")]);
        delete_view.insert("keepView".into(), ParamValue::Bool(false));
        let edit = list_ops::delete(&config, &resources, &delete_view).expect("a list and view");
        assert!(
            edit.changes
                .iter()
                .any(|change| change.path.ends_with("content/views.json"))
        );

        assert!(
            matches!(
                list_ops::delete(
                    &config,
                    &resources,
                    &params(&[("name", "programs.all"), ("title", "Nope")])
                ),
                Err(EditError::Usage(_))
            ),
            "a title that matches nothing is a usage error, not a silent no-op"
        );
    }

    #[test]
    fn a_block_moves_by_path_and_nests_inside_a_container() {
        let (config, resources) = preset("seed");
        let mut add: Params = params(&[("name", "courses.screen"), ("kind", "columns")]);
        add.insert(
            "value".into(),
            ParamValue::Strings(vec![
                "title=Two columns".into(),
                "blocks=[{\"kind\":\"callout\",\"text\":\"left\"}]".into(),
            ]),
        );
        let edit = view_ops::block_add(&config, &resources, &add).expect("a container block");
        let views = document(&edit.changes, "content/views.json");
        let blocks = views["views"]["courses.screen"]["blocks"]
            .as_array()
            .unwrap();
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[3]["kind"], json!("columns"));

        // Grow it for real: the next reads are the plan's own bytes.
        let root = temp_seed(&resources, "block-move");
        std::fs::write(
            root.join("content/views.json"),
            edit.changes[0].after.clone().unwrap(),
        )
        .unwrap();
        let current = config_store::load_with(&root, "explicit", &resources).expect("the copy");

        // Into the container, at its end: `0` is the stat, the container is 3.
        let mut nest: Params = params(&[
            ("name", "courses.screen"),
            ("kind", "callout"),
            ("parent", "3.blocks"),
        ]);
        nest.insert(
            "value".into(),
            ParamValue::Strings(vec!["text=right".into()]),
        );
        let edit = view_ops::block_add(&current, &resources, &nest).expect("a nested block");
        std::fs::write(
            root.join("content/views.json"),
            edit.changes[0].after.clone().unwrap(),
        )
        .unwrap();
        let current = config_store::load_with(&root, "explicit", &resources).expect("the copy");
        let nested = current.views["courses.screen"].blocks.as_ref().unwrap()[3]
            .blocks
            .as_ref()
            .expect("the container holds the block");
        assert_eq!(nested.len(), 2);
        assert_eq!(nested[1].kind, "callout");

        // Move the container to the front; the address and the landing spot are
        // both paths.
        let edit = view_ops::block_move(
            &current,
            &resources,
            &params(&[("name", "courses.screen"), ("path", "3"), ("index", "0")]),
        )
        .expect("a move");
        std::fs::write(
            root.join("content/views.json"),
            edit.changes[0].after.clone().unwrap(),
        )
        .unwrap();
        let current = config_store::load_with(&root, "explicit", &resources).expect("the copy");
        let blocks = current.views["courses.screen"].blocks.as_ref().unwrap();
        assert_eq!(blocks[0].kind, "columns");

        // A block cannot land inside itself.
        assert!(matches!(
            view_ops::block_move(
                &current,
                &resources,
                &params(&[
                    ("name", "courses.screen"),
                    ("path", "0"),
                    ("parent", "0.blocks")
                ]),
            ),
            Err(EditError::Usage(_))
        ));

        // And it can leave again, by its own nested path.
        let edit = view_ops::block_move(
            &current,
            &resources,
            &params(&[
                ("name", "courses.screen"),
                ("path", "0.blocks.0"),
                ("index", "1"),
            ]),
        )
        .expect("a move out");
        let views = document(&edit.changes, "content/views.json");
        assert_eq!(
            views["views"]["courses.screen"]["blocks"][1]["kind"],
            json!("callout")
        );

        // Removal addresses a nested block the same way.
        let remove: Params = params(&[("name", "courses.screen"), ("path", "0.blocks.1")]);
        let edit = view_ops::block_remove(&current, &resources, &remove).expect("a nested removal");
        let views = document(&edit.changes, "content/views.json");
        assert_eq!(
            views["views"]["courses.screen"]["blocks"][0]["blocks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_spec_never_parses_week_or_course_from_an_id() {
        assert_eq!(
            parse_spec("topic.est").unwrap(),
            ("topic".into(), "est".into())
        );
        assert!(parse_spec("topic").is_err());
        assert!(parse_spec(".est").is_err());
        assert!(
            parse_spec("problemset.pct").is_ok(),
            "a dotted key is still just a spec"
        );
    }

    #[test]
    fn field_values_parse_to_their_declared_types() {
        let text = FieldDef {
            key: "title".into(),
            type_: "text".into(),
            ..Default::default()
        };
        assert_eq!(
            parse_field_value("Bayes", &text, "p").unwrap(),
            json!("Bayes")
        );
        let est = FieldDef {
            key: "est".into(),
            type_: "duration".into(),
            ..Default::default()
        };
        assert_eq!(parse_field_value("30", &est, "p").unwrap(), json!(30));
        assert_eq!(number_value(30.0), json!(30));
        assert_eq!(number_value(2.5), json!(2.5));
        assert!(parse_field_value("-5", &est, "p").is_err());
        let formula = FieldDef {
            key: "pct".into(),
            type_: "formula".into(),
            expr: Some("pct(1,2)".into()),
            ..Default::default()
        };
        assert!(
            parse_field_value("60", &formula, "p").is_err(),
            "a derived field is never persisted in fields"
        );
        let stars = FieldDef {
            key: "r".into(),
            type_: "rating".into(),
            ..Default::default()
        };
        assert!(parse_field_value("6", &stars, "p").is_err());
        let bool_field = FieldDef {
            key: "b".into(),
            type_: "bool".into(),
            ..Default::default()
        };
        assert!(parse_field_value("yes", &bool_field, "p").is_err());
    }

    #[test]
    fn auto_ids_are_the_first_free_slot_and_never_reuse() {
        let def = crate::model::TypeDef {
            fields: vec![],
            ..Default::default()
        };
        let mut config = ResolvedConfig {
            plan_root: std::env::temp_dir(),
            layer: "explicit".into(),
            types: BTreeMap::from([("topic".to_string(), def)]),
            views: BTreeMap::new(),
            rules: crate::model::RulesFile::default(),
            shell: None,
            appearance: None,
            positioned_records: vec![],
            formulas: BTreeMap::new(),
            source_files: BTreeMap::new(),
            tokens: crate::resources::TokenRegister {
                version: "t".into(),
                tokens: BTreeMap::new(),
            },
            revision: "r".into(),
            load_warnings: vec![],
        };
        assert_eq!(auto_id(&config, "topic"), "topic.r1");
        config.positioned_records.push(PositionedRecord {
            record: Record {
                schema_version: None,
                id: "topic.r1".into(),
                type_: "topic".into(),
                fields: BTreeMap::new(),
                links: BTreeMap::new(),
            },
            file: "content/records/topic.jsonl".into(),
            line: 1,
        });
        assert_eq!(auto_id(&config, "topic"), "topic.r2");
    }

    #[test]
    fn a_new_record_carries_its_schema_version_and_survives_the_loader() {
        let (config, resources) = seed();
        let params = Params::from([
            ("type".to_string(), ParamValue::Str("topic".to_string())),
            (
                "title".to_string(),
                ParamValue::Str("A created topic".to_string()),
            ),
            ("est".to_string(), ParamValue::Str("30".to_string())),
        ]);
        let planned = record_ops::new_record(&config, &params).expect("a record plans");
        assert!(planned.changes_anything(), "a new record changes the plan");
        let addition = planned
            .changes
            .iter()
            .find(|change| change.path.starts_with("content/records/"))
            .expect("it lands in a records file");
        let text = String::from_utf8(addition.after.clone().expect("an append")).unwrap();
        assert!(
            text.contains("\"schemaVersion\":1"),
            "a created record must declare its schema version, or the loader refuses it: {text}"
        );
        // And the whole plan still validates with it (§4.6 step 2).
        crate::transaction::validate_by_overlay(&config.plan_root, &planned.changes, &resources)
            .expect("the plan validates with the new record in it");
    }

    #[test]
    fn slug_and_type_id_safety() {
        assert_eq!(type_ops::slug("Problem sets!"), "problem-sets");
        assert_eq!(type_ops::slug("  JEE  Prep  "), "jee-prep");
        assert!(type_ops::safe_type_id("problemset"));
        assert!(type_ops::safe_type_id("p.dp"));
        assert!(!type_ops::safe_type_id("../etc"));
        assert!(!type_ops::safe_type_id("a/b"));
        assert!(!type_ops::safe_type_id(""));
    }

    #[test]
    fn only_the_declared_documents_and_records_files_are_writable_by_the_text_door() {
        assert!(source_ops::is_authoritative("content/types.json"));
        assert!(source_ops::is_authoritative("state/state.json"));
        assert!(source_ops::is_authoritative("content/records/topic.jsonl"));
        assert!(!source_ops::is_authoritative("content/../../etc/passwd"));
        assert!(!source_ops::is_authoritative(
            "content/records/../topic.jsonl"
        ));
        assert!(!source_ops::is_authoritative("Package.swift"));
    }

    #[test]
    fn pane_precheck_reports_every_bad_line_with_its_number() {
        let found = pane_precheck::diagnostics(
            "content/records/topic.jsonl",
            "{\"id\":\"a\"}\nnot json\n{\"id\":\"c\"}\n",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, Some(2));
        assert!(pane_precheck::diagnostics("content/types.json", "{").len() == 1);
        assert!(pane_precheck::diagnostics("content/types.json", "{}").is_empty());
    }

    #[test]
    fn the_inverse_of_a_change_set_restores_the_before_bytes() {
        let changes = vec![FileChange::new(
            "content/types.json",
            Some(b"before".to_vec()),
            Some(b"after".to_vec()),
        )];
        let inverse = undo_engine::invert(&changes);
        assert_eq!(inverse[0].before.as_deref(), Some(b"after".as_slice()));
        assert_eq!(inverse[0].after.as_deref(), Some(b"before".as_slice()));
    }

    #[test]
    fn settings_declare_their_keys_and_are_the_same_generator_as_records() {
        assert_eq!(
            settings_ops::keys(),
            vec!["dailyTargetMin", "timezone", "scheduler", "fixedIntervals"]
        );
        let field = settings_ops::field("dailyTargetMin").expect("declared");
        assert_eq!(field.type_, "duration");
        assert_eq!(field.label.as_deref(), Some("Daily target"));
        assert!(settings_ops::field("nope").is_none());
        // The scheduler row is a `select` over the declared schedulers, and its
        // value persists at the documented key (§3.5), not under `#/study`.
        let scheduler = settings_ops::field("scheduler").expect("declared");
        assert_eq!(scheduler.type_, "select");
        assert_eq!(
            scheduler.options.as_deref().map(<[String]>::to_vec),
            Some(
                crate::rules::SCHEDULERS
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(
            settings_ops::find("scheduler").expect("declared").path,
            &["scheduler", "name"]
        );
        assert_eq!(
            settings_ops::find("timezone").expect("declared").path,
            &["timezone"]
        );
    }

    #[test]
    fn the_seed_plan_still_validates_and_the_registry_resolves() {
        let (config, resources) = seed();
        // A rename keeps the key: the label changes, the identity does not.
        let edit = column_ops::rename(&config, &resources, "topic.title", Some("Name"))
            .expect("rename plans");
        let after = edit
            .changes
            .iter()
            .find(|change| change.path == "content/types.json")
            .and_then(|change| change.after.clone())
            .expect("types.json is rewritten");
        let text = String::from_utf8(after).unwrap();
        assert!(text.contains("\"key\": \"title\""), "the key never changes");
        assert!(text.contains("\"label\": \"Name\""));
        let _ = command_registry::resolve("apply").expect("the registry still resolves");
    }
}

// ── appearance: theme, overrides and text scale (§6 Phase 7, §4.1) ───────────

/// The writes `content/appearance.json` takes. §4.1's ownership rule is the
/// shape of this module: the file *"alone stores overrides against its pinned
/// bundled theme"*, so a theme choice, one token override and the text-size
/// preference are three writes to one document — never a new palette anywhere.
pub mod appearance_ops {
    use super::*;

    /// `content/appearance.json#/theme` — the pinned theme id.
    pub const THEME_PATH: [&str; 1] = ["theme"];
    /// `content/appearance.json#/textScale` — the text-size multiple.
    pub const SCALE_PATH: [&str; 1] = ["textScale"];
    /// `content/appearance.json#/overrides` — token-path overrides.
    pub const OVERRIDES_PATH: [&str; 1] = ["overrides"];

    fn appearance_doc(config: &ResolvedConfig) -> Result<Value, EditError> {
        match doc(config, "content/appearance.json")? {
            Some(value) => Ok(value),
            None => Ok(
                json!({ "schemaVersion": 1, "theme": crate::theme::DEFAULT_THEME, "overrides": {} }),
            ),
        }
    }

    fn commit(
        config: &ResolvedConfig,
        resources_dir: &Path,
        document: &Value,
        preview: Value,
        summary: &str,
    ) -> Result<PlannedEdit, EditError> {
        let changes = vec![doc_change(config, "content/appearance.json", document)];
        finalize_with(config, resources_dir, changes, preview)
            .map(|edit| edit.named(summary.to_string()))
    }

    /// `theme.set <id>` — pin one of the shipped (or student-authored) themes.
    pub fn set_theme(
        config: &ResolvedConfig,
        resources_dir: &Path,
        id: &str,
    ) -> Result<PlannedEdit, EditError> {
        if crate::theme::load(resources_dir, id)
            .map_err(EditError::Usage)?
            .is_none()
        {
            let known: Vec<String> = crate::theme::list(resources_dir)
                .map_err(EditError::Usage)?
                .into_iter()
                .map(|theme| theme.id)
                .collect();
            return Err(EditError::Usage(format!(
                "no theme \"{id}\" in {} — declared: {}",
                crate::theme::themes_dir(resources_dir).display(),
                known.join(", ")
            )));
        }
        let mut document = appearance_doc(config)?;
        doc_json::set(&mut document, &THEME_PATH.map(str::to_string), json!(id))?;
        commit(
            config,
            resources_dir,
            &document,
            json!({ "theme": id }),
            &format!("theme.set {id}"),
        )
    }

    /// `appearance.setTextScale <n>` — the §6 Phase 7 text-size preference.
    pub fn set_text_scale(
        config: &ResolvedConfig,
        resources_dir: &Path,
        value: f64,
    ) -> Result<PlannedEdit, EditError> {
        let (low, high) = crate::theme::TEXT_SCALE_RANGE;
        if !(low..=high).contains(&value) {
            return Err(EditError::Usage(format!(
                "textScale must be between {low} and {high}; got {value}"
            )));
        }
        let rounded = (value * 100.0).round() / 100.0;
        let mut document = appearance_doc(config)?;
        ensure_object_path(
            &mut document,
            &SCALE_PATH.map(str::to_string),
            ".",
            "content/appearance.json",
        )?;
        doc_json::set(
            &mut document,
            &SCALE_PATH.map(str::to_string),
            json!(rounded),
        )?;
        commit(
            config,
            resources_dir,
            &document,
            json!({ "textScale": rounded }),
            &format!("appearance.setTextScale {rounded}"),
        )
    }

    /// `appearance.setOverride <token> <value>` — one token override.
    ///
    /// The token must exist in the bundled register and the value must survive
    /// the same merge the loader runs (§4.1: unknown keys and `null` are
    /// refused), so a typo fails here with the key that is wrong rather than at
    /// the next load.
    pub fn set_override(
        config: &ResolvedConfig,
        resources_dir: &Path,
        key: &str,
        value: &str,
    ) -> Result<PlannedEdit, EditError> {
        let register =
            crate::resources::TokenRegister::bundled(resources_dir).map_err(EditError::Usage)?;
        if register.get(key).is_none() {
            return Err(EditError::Usage(format!(
                "\"{key}\" is not a token — `SAM --tokens --json` lists the {} the register carries",
                register.tokens.len()
            )));
        }
        let parsed = parse_override_value(value);
        let mut document = appearance_doc(config)?;
        ensure_object_path(
            &mut document,
            &OVERRIDES_PATH.map(str::to_string),
            ".",
            "content/appearance.json",
        )?;
        let mut overrides = doc_json::get(&document, &OVERRIDES_PATH.map(str::to_string))
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}));
        let segments: Vec<String> = key.split('.').map(str::to_string).collect();
        ensure_object_path(&mut overrides, &segments, ".", "content/appearance.json")?;
        doc_json::set(&mut overrides, &segments, parsed.clone())?;
        // The loader's own merge is the acceptance test: a value it refuses is
        // refused here, before a byte is staged.
        register.merged(&overrides).map_err(EditError::Usage)?;
        doc_json::set(
            &mut document,
            &OVERRIDES_PATH.map(str::to_string),
            overrides,
        )?;
        commit(
            config,
            resources_dir,
            &document,
            json!({ "override": { "key": key, "value": parsed } }),
            &format!("appearance.setOverride {key}"),
        )
    }

    /// `appearance.clearOverride <token>` — the theme's value returns (§4.1:
    /// *"Reset removes the override key"*).
    pub fn clear_override(
        config: &ResolvedConfig,
        resources_dir: &Path,
        key: &str,
    ) -> Result<PlannedEdit, EditError> {
        let mut document = appearance_doc(config)?;
        let mut overrides = doc_json::get(&document, &OVERRIDES_PATH.map(str::to_string))
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}));
        let segments: Vec<String> = key.split('.').map(str::to_string).collect();
        if doc_json::get(&overrides, &segments).is_none() {
            return Err(EditError::Usage(format!(
                "\"{key}\" is not overridden — nothing to reset"
            )));
        }
        doc_json::remove(&mut overrides, &segments)?;
        doc_json::set(
            &mut document,
            &OVERRIDES_PATH.map(str::to_string),
            overrides,
        )?;
        commit(
            config,
            resources_dir,
            &document,
            json!({ "cleared": key }),
            &format!("appearance.clearOverride {key}"),
        )
    }

    /// A scalar written as itself; anything else stays the string the caller
    /// typed. `[1, 2]` and `true` are JSON because a font stack and a boolean
    /// are real token values; `oklch(…)` never parses as JSON and stays text.
    fn parse_override_value(text: &str) -> Value {
        match serde_json::from_str::<Value>(text) {
            Ok(
                value @ (Value::Array(_) | Value::Object(_) | Value::Bool(_) | Value::Number(_)),
            ) => value,
            _ => json!(text),
        }
    }
}
