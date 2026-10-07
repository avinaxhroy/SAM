//! Batch record ingestion (`SAM apply`) (§4.7, §4.9).
//!
//! Upserts records idempotently by ID through the [`crate::transaction`] engine.
//! Follows §4.1 serialization semantics: preserves untouched lines byte-for-byte,
//! formats updated records canonically, preserves input order for additions,
//! and treats identical re-applications as byte-level no-ops.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::config_store::{ConfigStoreError, PositionedRecord, fnv_hex};
use crate::decode;
use crate::diagnostic::Diagnostic;
use crate::json_value::JSONValue;
use crate::model::Record;
use crate::resolved_config::ResolvedConfig;
use crate::strict_json;
use crate::transaction::{self, CommitResult, FileChange, TransactionError};
use crate::validator;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ApplyCounts {
    pub added: usize,
    pub updated: usize,
    pub moved: usize,
    pub unchanged: usize,
}

impl ApplyCounts {
    pub fn total(&self) -> usize {
        self.added + self.updated + self.moved + self.unchanged
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordDiffEntry {
    pub id: String,
    /// `added` | `updated` | `moved`
    pub action: String,
    /// Canonical before line.
    pub from: Option<String>,
    /// Canonical after line.
    pub to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlannedBatch {
    pub base_revision: String,
    pub candidate_revision: String,
    pub changes: Vec<FileChange>,
    pub diff: Vec<RecordDiffEntry>,
    pub counts: ApplyCounts,
    pub warnings: Vec<Diagnostic>,
}

impl PlannedBatch {
    pub fn changes_anything(&self) -> bool {
        !self.changes.is_empty()
    }
}

#[derive(Debug)]
pub enum ApplyError {
    /// Every bad line at once; a malformed line rejects the whole batch (§4.7 #5).
    Validation(Vec<Diagnostic>),
    Usage(String),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::Usage(message) => write!(f, "{message}"),
            ApplyError::Validation(diagnostics) => {
                for diagnostic in diagnostics {
                    writeln!(f, "{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ApplyError {}

/// The published batch limit (§4.9).
pub const MAX_BATCH_RECORDS: usize = 10_000;

const RECORDS_DIR: &str = "content/records";

/// Parse a batch and plan it against a loaded plan. Pure — no writes, which is
/// exactly what `--dry-run` runs. Every line-level diagnostic is thrown at
/// once; a malformed line rejects the whole batch.
pub fn plan(
    config: &ResolvedConfig,
    batch: &[u8],
    batch_name: &str,
) -> Result<PlannedBatch, ApplyError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // ---- parse (report EVERY bad line)
    let Ok(text) = std::str::from_utf8(batch) else {
        return Err(ApplyError::Validation(vec![Diagnostic::error(
            "encoding.invalid",
            batch_name,
            "batch is not valid UTF-8",
        )]));
    };
    let text = text.replace("\r\n", "\n");
    let mut parsed: Vec<(u64, Record)> = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let line_no = index as u64 + 1;
        match strict_json::parse_line(line, batch_name, line_no)
            .and_then(|value| decode::record(&value, batch_name, line_no))
        {
            Ok(record) => parsed.push((line_no, record)),
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }

    // ---- within-batch duplicates: identical dedups, conflicting fails (§4.9)
    let mut first_seen: BTreeMap<String, u64> = BTreeMap::new();
    let mut records: Vec<(u64, Record)> = Vec::new();
    for (line, record) in parsed {
        match first_seen.get(&record.id).copied() {
            Some(first_line) => {
                let first = records
                    .iter()
                    .find(|(_, existing)| existing.id == record.id)
                    .map(|(_, existing)| existing.clone());
                if first.is_some_and(|existing| records_equivalent(&existing, &record)) {
                    continue;
                }
                diagnostics.push(Diagnostic::error(
                    "batch.conflicting-duplicate",
                    batch_name,
                    format!(
                        "id \"{}\" appears twice with different content (first at line {first_line})",
                        record.id
                    ),
                ).at_line(line));
            }
            None => {
                first_seen.insert(record.id.clone(), line);
                records.push((line, record));
            }
        }
    }
    if records.len() > MAX_BATCH_RECORDS {
        diagnostics.push(Diagnostic::error(
            "batch.too-large",
            batch_name,
            format!(
                "batch holds {} records; the published limit is {MAX_BATCH_RECORDS} — split the batch",
                records.len()
            ),
        ));
    }
    if !diagnostics.is_empty() {
        return Err(ApplyError::Validation(diagnostics));
    }

    // ---- filename safety before any path is built (§4.6)
    for (line, record) in &records {
        let kind = record.type_.as_str();
        if kind.is_empty()
            || kind.contains('/')
            || kind.contains('\\')
            || kind == "."
            || kind == ".."
        {
            diagnostics.push(
                Diagnostic::error(
                    "batch.type-unsafe",
                    batch_name,
                    format!("type \"{kind}\" is not a safe records-file name"),
                )
                .at_line(*line),
            );
        }
    }
    if !diagnostics.is_empty() {
        return Err(ApplyError::Validation(diagnostics));
    }

    // ---- merge plan: replace in place · move across files · append
    let mut by_id: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, positioned) in config.positioned_records.iter().enumerate() {
        by_id.entry(positioned.record.id.as_str()).or_insert(index);
    }

    /// `(positioned index, the plan's line, replacement record)`
    struct Replacement {
        positioned: usize,
        /// The line the record currently occupies **in the plan**, never the
        /// batch line: a batch's own line numbers say nothing about where the
        /// record it names lives, and `rebuild_jsonl` replaces by line.
        line: u64,
        record: Record,
    }
    let mut replaces: BTreeMap<String, Replacement> = BTreeMap::new();
    let mut removals: BTreeSet<String> = BTreeSet::new();
    let mut appends: Vec<(u64, Record)> = Vec::new();
    let mut diff: Vec<RecordDiffEntry> = Vec::new();
    let mut counts = ApplyCounts::default();

    for (line, record) in records {
        match by_id.get(record.id.as_str()).copied() {
            Some(index) => {
                let existing = &config.positioned_records[index];
                if records_equivalent(&existing.record, &record) {
                    counts.unchanged += 1;
                    continue;
                }
                if existing.record.type_ == record.type_ {
                    let from = ResolvedConfig::canonical_line(&existing.record).ok();
                    let to = ResolvedConfig::canonical_line(&record).ok();
                    diff.push(RecordDiffEntry {
                        id: record.id.clone(),
                        action: "updated".into(),
                        from,
                        to,
                    });
                    replaces.insert(
                        record.id.clone(),
                        Replacement {
                            positioned: index,
                            line: existing.line,
                            record,
                        },
                    );
                    counts.updated += 1;
                } else {
                    // Cross-type move (§4.6): the old line goes, a new file
                    // (or a new line) takes it.
                    let from = ResolvedConfig::canonical_line(&existing.record).ok();
                    let to = ResolvedConfig::canonical_line(&record).ok();
                    diff.push(RecordDiffEntry {
                        id: record.id.clone(),
                        action: "moved".into(),
                        from,
                        to,
                    });
                    removals.insert(record.id.clone());
                    appends.push((line, record));
                    counts.moved += 1;
                }
            }
            None => {
                let to = ResolvedConfig::canonical_line(&record).ok();
                diff.push(RecordDiffEntry {
                    id: record.id.clone(),
                    action: "added".into(),
                    from: None,
                    to,
                });
                appends.push((line, record));
                counts.added += 1;
            }
        }
    }

    // ---- rebuild affected files (§4.1 serialization rules)
    #[derive(Default)]
    struct Work {
        replacements: BTreeMap<u64, Record>,
        deletions: BTreeSet<u64>,
        appends: Vec<Record>,
    }
    let mut file_work: BTreeMap<String, Work> = BTreeMap::new();
    for replacement in replaces.values() {
        let file = config.positioned_records[replacement.positioned]
            .file
            .clone();
        file_work
            .entry(file)
            .or_default()
            .replacements
            .insert(replacement.line, replacement.record.clone());
    }
    for id in &removals {
        let positioned = &config.positioned_records[by_id[id.as_str()]];
        file_work
            .entry(positioned.file.clone())
            .or_default()
            .deletions
            .insert(positioned.line);
    }
    for (_, record) in &appends {
        file_work
            .entry(format!("{RECORDS_DIR}/{}.jsonl", record.type_))
            .or_default()
            .appends
            .push(record.clone());
    }

    let mut changes: Vec<FileChange> = Vec::new();
    for (rel, work) in &file_work {
        let raw = config.source_files.get(rel);
        let raw_text = raw.and_then(|bytes| std::str::from_utf8(bytes).ok());
        let new_text = rebuild_jsonl(raw_text, &work.replacements, &work.deletions, &work.appends)
            .map_err(ApplyError::Usage)?;
        changes.push(FileChange::new(
            rel.clone(),
            raw.cloned(),
            Some(new_text.into_bytes()),
        ));
    }

    // ---- validate the ENTIRE resulting plan (§4.6 step 2). Batch-sourced
    // records keep their batch position so diagnostics point at the input the
    // caller is about to correct (§4.7 #5).
    let mut merged: Vec<PositionedRecord> = Vec::new();
    for (index, positioned) in config.positioned_records.iter().enumerate() {
        if let Some(replacement) = replaces.get(&positioned.record.id) {
            debug_assert_eq!(replacement.positioned, index);
            merged.push(PositionedRecord {
                record: replacement.record.clone(),
                file: batch_name.to_string(),
                line: replacement.line,
            });
        } else if !removals.contains(&positioned.record.id) {
            merged.push(positioned.clone());
        }
    }
    for (line, record) in &appends {
        merged.push(PositionedRecord {
            record: record.clone(),
            file: batch_name.to_string(),
            line: *line,
        });
    }
    diagnostics.extend(validator::validate(
        &config.types,
        &config.views,
        &config.rules,
        config.shell.as_ref(),
        &merged,
    ));
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(ApplyError::Validation(diagnostics));
    }

    // ---- candidate revision: what the plan fingerprints after commit
    let base_hashes: BTreeMap<String, String> = config
        .source_files
        .iter()
        .map(|(path, bytes)| (path.clone(), fnv_hex(bytes)))
        .collect();
    let candidate_revision = transaction::revision_after(&base_hashes, &changes);

    Ok(PlannedBatch {
        base_revision: config.revision.clone(),
        candidate_revision,
        changes,
        diff,
        counts,
        warnings: diagnostics
            .into_iter()
            .filter(|diagnostic| !diagnostic.is_error())
            .collect(),
    })
}

/// Commit a planned batch. The caller holds the lock and has already recovered.
/// An all-no-op batch never reaches the engine — the repeated batch changes no
/// bytes (§4.7 #3).
pub fn commit(
    root: &std::path::Path,
    planned: &PlannedBatch,
) -> Result<CommitResult, TransactionError> {
    if !planned.changes_anything() {
        return Ok(CommitResult {
            txid: String::new(),
            base_revision: planned.base_revision.clone(),
            new_revision: planned.base_revision.clone(),
            files: Vec::new(),
        });
    }
    let counts = &planned.counts;
    transaction::commit(
        root,
        &planned.changes,
        &planned.base_revision,
        &planned.candidate_revision,
        &format!(
            "apply: +{} added · {} updated · {} moved · {} unchanged",
            counts.added, counts.updated, counts.moved, counts.unchanged
        ),
    )
}

/// Rebuild one records file: replacements and removals by line index (no
/// shift), appends after the last record, the boundary repaired when the source
/// lacks a final newline, untouched lines byte-for-byte. `raw_text == None`
/// builds a new canonical file from the appends alone.
pub fn rebuild_jsonl(
    raw_text: Option<&str>,
    replacements: &BTreeMap<u64, Record>,
    deletions: &BTreeSet<u64>,
    appends: &[Record],
) -> Result<String, String> {
    let canonical = |record: &Record| ResolvedConfig::canonical_line(record);
    let Some(raw_text) = raw_text else {
        let mut out: Vec<String> = Vec::new();
        for record in appends {
            out.push(canonical(record)?);
        }
        return Ok(if out.is_empty() {
            String::new()
        } else {
            format!("{}\n", out.join("\n"))
        });
    };

    // A zero-byte file has no lines; anything else splits on LF, keeping the
    // trailing empty a final newline produces.
    let parts: Vec<&str> = if raw_text.is_empty() {
        Vec::new()
    } else {
        raw_text.split('\n').collect()
    };
    let had_trailing_empty = parts.last().is_some_and(|line| line.is_empty());

    let mut keep: Vec<String> = Vec::new();
    for (index, line) in parts.iter().enumerate() {
        let line_no = index as u64 + 1;
        if let Some(record) = replacements.get(&line_no) {
            keep.push(canonical(record)?);
        } else if !deletions.contains(&line_no) {
            keep.push((*line).to_string());
        }
    }
    if !appends.is_empty() {
        if had_trailing_empty {
            keep.pop();
        }
        for record in appends {
            keep.push(canonical(record)?);
        }
        keep.push(String::new());
    }
    // No appends: the loop already preserved the trailing empty when the source
    // had one — appending another would turn the final newline into a blank
    // interior line on update-only rewrites.
    Ok(keep.join("\n"))
}

/// Semantic record equality: an absent `schemaVersion` normalizes to 1, so a
/// record is the same whether or not it spelled the version out (§1.2 #3:
/// hand-edited whitespace and equivalent inputs are not a difference).
pub fn records_equivalent(a: &Record, b: &Record) -> bool {
    a.schema_version.unwrap_or(1) == b.schema_version.unwrap_or(1)
        && a.id == b.id
        && a.type_ == b.type_
        && a.fields == b.fields
        && a.links == b.links
}

/// The canonical line for a batch record, for callers building a diff view.
pub fn canonical_or_null(record: &Record) -> JSONValue {
    match ResolvedConfig::canonical_line(record) {
        Ok(line) => JSONValue::String(line),
        Err(_) => JSONValue::Null,
    }
}

/// The `ConfigStoreError` shape callers get when an apply-time load fails.
pub type LoadError = ConfigStoreError;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn record(id: &str, title: &str, est: i64) -> Record {
        Record {
            schema_version: Some(1),
            id: id.into(),
            type_: "topic".into(),
            fields: BTreeMap::from([
                ("est".to_string(), JSONValue::from(est)),
                ("kind".to_string(), JSONValue::String("watch".into())),
                ("title".to_string(), JSONValue::String(title.into())),
            ]),
            links: BTreeMap::new(),
        }
    }

    #[test]
    fn untouched_lines_are_preserved_byte_for_byte_and_appends_repair_the_boundary() {
        // Verbatim from the serialization rules: replaces normalize, untouched
        // lines survive, a missing final newline is repaired on append.
        let raw = "{\"a\":  1}\n{\"b\": 2}";
        let mut replacements = BTreeMap::new();
        replacements.insert(2, record("t.2", "New", 20));
        let appends = vec![record("t.3", "Third", 30)];
        let out = rebuild_jsonl(Some(raw), &replacements, &BTreeSet::new(), &appends)
            .expect("a rebuild succeeds");
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(
            lines[0], "{\"a\":  1}",
            "line 1 is untouched, spacing and all"
        );
        assert!(lines[1].contains("\"t.2\""), "line 2 was replaced");
        assert!(
            lines[1].starts_with("{\"fields\":"),
            "a replacement is canonical"
        );
        assert!(
            lines[2].contains("\"t.3\""),
            "appends follow in input order"
        );
        assert_eq!(lines[3], "", "the boundary was repaired");
    }

    #[test]
    fn an_update_only_rewrite_does_not_double_the_final_newline() {
        let raw = "{\"a\":  1}\n{\"b\": 2}\n";
        let mut replacements = BTreeMap::new();
        replacements.insert(2, record("t.2", "New", 20));
        let out = rebuild_jsonl(Some(raw), &replacements, &BTreeSet::new(), &[])
            .expect("a rebuild succeeds");
        assert_eq!(out.matches('\n').count(), 2, "two lines in, two lines out");
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn deletions_remove_the_line_and_nothing_else() {
        let raw = "{\"a\":  1}\n{\"b\": 2}\n{\"c\": 3}\n";
        let out = rebuild_jsonl(Some(raw), &BTreeMap::new(), &BTreeSet::from([2]), &[])
            .expect("a rebuild succeeds");
        assert_eq!(out, "{\"a\":  1}\n{\"c\": 3}\n");
    }

    #[test]
    fn a_new_file_is_canonical_from_the_first_byte() {
        let out = rebuild_jsonl(
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &[record("t.1", "A", 10)],
        )
        .expect("a rebuild succeeds");
        assert_eq!(out.matches('\n').count(), 1);
        assert!(out.starts_with("{\"fields\":{\"est\":10,"));
    }

    #[test]
    fn an_absent_schema_version_is_the_same_record_as_one() {
        let mut a = record("t.1", "A", 10);
        let mut b = record("t.1", "A", 10);
        a.schema_version = None;
        b.schema_version = Some(1);
        assert!(records_equivalent(&a, &b));
        b.fields.insert("est".into(), JSONValue::from(11));
        assert!(
            !records_equivalent(&a, &b),
            "different content is a difference"
        );
    }

    #[test]
    fn a_conflicting_duplicate_is_rejected_and_an_identical_one_dedups() {
        // The seed preset, because this batch is made of `topic` records and the
        // corpus fixture holds only courses and problem sets.
        let resources =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources");
        let plan_root = resources.join("presets/seed");
        let config = crate::config_store::load_with(&plan_root, "explicit", &resources)
            .expect("the seed loads");
        let line = r#"{"fields":{"est":10,"kind":"watch","title":"A"},"id":"t.new","links":{},"schemaVersion":1,"type":"topic"}"#;

        let identical = format!("{line}\n{line}\n");
        let planned =
            plan(&config, identical.as_bytes(), "batch").expect("identical duplicates dedup");
        assert_eq!(
            planned.counts.added, 1,
            "the second copy is not a second record"
        );

        let conflicting = format!(
            "{line}\n{}\n",
            r#"{"fields":{"est":11,"kind":"watch","title":"A"},"id":"t.new","links":{},"schemaVersion":1,"type":"topic"}"#
        );
        let error = plan(&config, conflicting.as_bytes(), "batch").expect_err("conflicting fails");
        let ApplyError::Validation(diagnostics) = error else {
            panic!("a conflicting duplicate is a validation failure");
        };
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "batch.conflicting-duplicate")
        );
    }

    /// Regression, found by hand on a real plan: a batch's own line numbers say
    /// nothing about where the record it names lives. Keying the rebuild by the
    /// batch line made a one-line `record.setField` overwrite **line 1** of the
    /// records file — duplicating the target and silently deleting whatever
    /// record was first. The update must land on the record's own line and leave
    /// every other line byte-for-byte.
    #[test]
    fn an_update_rewrites_the_records_own_line_and_nothing_else() {
        let resources =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources");
        let dir =
            std::env::temp_dir().join(format!("sam-apply-{}", crate::transaction::make_txid()));
        crate::transaction::copy_tree(&resources.join("presets/seed"), &dir)
            .expect("the seed copies");
        let config =
            crate::config_store::load_with(&dir, "explicit", &resources).expect("the copy loads");

        let target = config
            .positioned_records
            .iter()
            .find(|positioned| positioned.record.id == "t.demo.03")
            .expect("the seed has t.demo.03");
        assert!(
            target.line > 1,
            "this regression needs a record that is not the file's first line"
        );
        let file = target.file.clone();
        let before_text = String::from_utf8(
            config
                .source_files
                .get(&file)
                .expect("the records file is loaded")
                .clone(),
        )
        .expect("the records file is UTF-8");
        let before_lines: Vec<&str> = before_text.lines().collect();

        let mut updated = target.record.clone();
        updated
            .fields
            .insert("title".into(), JSONValue::String("Moved title".into()));
        let batch = format!(
            "{}\n",
            ResolvedConfig::canonical_line(&updated).expect("canonical")
        );
        let planned = plan(&config, batch.as_bytes(), "test-batch").expect("the batch plans");

        let change = planned
            .changes
            .iter()
            .find(|change| change.path == file)
            .expect("the records file is rewritten");
        let after_text = String::from_utf8(change.after.clone().expect("an after-image")).unwrap();
        let after_lines: Vec<&str> = after_text.lines().collect();

        assert_eq!(
            before_lines.len(),
            after_lines.len(),
            "an update neither adds nor drops a line"
        );
        assert!(
            after_lines[target.line as usize - 1].contains("Moved title"),
            "the update lands on the record's own line"
        );
        for (index, line) in before_lines.iter().enumerate() {
            if index as u64 + 1 == target.line {
                continue;
            }
            assert_eq!(&after_lines[index], line, "every other line is untouched");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
