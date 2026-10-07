//! Profiles: `*.samprofile` export and import (§6 Phase 7, §8 item 2).
//!
//! A profile bundles schema, views, rules, shell layout, appearance, and records.
//! Import validates via staging before activating the new plan (§4.1, §4.6).
//!
//! Private-record handling:
//! - Default export excludes types with `"private": true` along with referring links
//!   and progress state.
//! - `--personal` includes private records and `state/state.json`.
//!
//! Profiles contain no executable content and are subject to whole-plan validation on import.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::json_value::JSONValue;
use crate::model::Record;
use crate::resolved_config::ResolvedConfig;

/// The document kind, checked on import. A file that does not say this is not
/// a profile, whatever its extension.
pub const PROFILE_KIND: &str = "samprofile";

/// The extension `bundle.fileAssociations` binds (§6 Phase 7).
pub const PROFILE_EXTENSION: &str = "samprofile";

/// The documents a profile carries, in one fixed order. `state/state.json` is
/// not here: progress is personal and ships only under `--personal`.
pub const PROFILE_DOCUMENTS: [&str; 5] = [
    "content/types.json",
    "content/views.json",
    "content/rules.json",
    "content/shell.json",
    "content/appearance.json",
];

/// The progress document, present only in a personal export.
pub const STATE_DOCUMENT: &str = "state/state.json";

/// Who wrote the profile, and from what revision — provenance, never a
/// precondition for import.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

/// What the export decided to carry — readable in the file, so a student can
/// see why a note is missing before they send it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileIncludes {
    /// True when private records and progress state are included.
    pub personal: bool,
    /// Whether `state/state.json` is present.
    pub state: bool,
    #[serde(rename = "recordCount", default)]
    pub record_count: usize,
    /// The private kinds left out, by type id.
    #[serde(
        rename = "excludedTypes",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub excluded_types: Vec<String>,
    #[serde(rename = "excludedRecords", default, skip_serializing_if = "is_zero")]
    pub excluded_records: usize,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

/// The profile document itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileDoc {
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: Option<u64>,
    pub kind: String,
    pub name: String,
    #[serde(
        rename = "exportedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exported_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ProfileSource>,
    #[serde(default)]
    pub includes: ProfileIncludes,
    /// Document path → its parsed JSON, canonical on write.
    pub documents: BTreeMap<String, JSONValue>,
    /// Type id → its records in file order.
    #[serde(default)]
    pub records: BTreeMap<String, Vec<JSONValue>>,
    /// `state/state.json`, when the export was personal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<JSONValue>,
}

impl ProfileDoc {
    /// The plan name this profile materializes as when the caller names none:
    /// the profile's own name, slugged.
    pub fn default_plan_name(&self) -> String {
        let mut out = String::new();
        for character in self.name.chars() {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                out.push(character.to_ascii_lowercase());
            } else if character.is_whitespace() && !out.ends_with('-') {
                out.push('-');
            }
        }
        let trimmed = out.trim_matches('-').to_string();
        if trimmed.is_empty() {
            "imported".to_string()
        } else {
            trimmed
        }
    }
}

// ── export ───────────────────────────────────────────────────────────────────

/// Build a profile from a loaded plan.
///
/// `personal` false: private kinds' records are left out, every link that
/// pointed at one is dropped, and no progress state ships. `personal` true:
/// everything ships, exactly as the plan holds it (canonicalized).
pub fn export(
    config: &ResolvedConfig,
    personal: bool,
    name: Option<&str>,
    exported_at: Option<String>,
) -> Result<ProfileDoc, String> {
    let mut documents = BTreeMap::new();
    for path in PROFILE_DOCUMENTS {
        if let Some(bytes) = config.source_files.get(path) {
            let text =
                std::str::from_utf8(bytes).map_err(|_| format!("{path} is not valid UTF-8"))?;
            let value = crate::strict_json::parse(text, path)
                .map_err(|error| format!("{path}: {}", error.diagnostic.message))?;
            documents.insert(path.to_string(), value);
        }
    }
    if !documents.contains_key("content/types.json") {
        return Err("this plan has no content/types.json to export".into());
    }

    let mut excluded_types: Vec<String> = Vec::new();
    let mut excluded_ids: BTreeSet<String> = BTreeSet::new();
    for (type_id, def) in &config.types {
        if def.private == Some(true) && !personal {
            excluded_types.push(type_id.clone());
            for record in config
                .records_iter()
                .filter(|record| record.type_ == *type_id)
            {
                excluded_ids.insert(record.id.clone());
            }
        }
    }

    let mut records: BTreeMap<String, Vec<JSONValue>> = BTreeMap::new();
    let mut record_count = 0usize;
    for (type_id, _) in &config.types {
        if excluded_types.contains(type_id) {
            continue;
        }
        let mut rows = Vec::new();
        for record in config
            .records_iter()
            .filter(|record| record.type_ == *type_id)
        {
            let mut value = serde_json::to_value(record).map_err(|error| error.to_string())?;
            if !excluded_ids.is_empty() {
                prune_links(&mut value, &excluded_ids);
            }
            rows.push(value);
            record_count += 1;
        }
        if !rows.is_empty() {
            records.insert(type_id.clone(), rows);
        }
    }

    let state = if personal {
        match config.source_files.get(STATE_DOCUMENT) {
            Some(bytes) => {
                let text = std::str::from_utf8(bytes)
                    .map_err(|_| format!("{STATE_DOCUMENT} is not valid UTF-8"))?;
                Some(
                    crate::strict_json::parse(text, STATE_DOCUMENT).map_err(|error| {
                        format!("{STATE_DOCUMENT}: {}", error.diagnostic.message)
                    })?,
                )
            }
            None => None,
        }
    } else {
        None
    };

    Ok(ProfileDoc {
        schema_version: Some(1),
        kind: PROFILE_KIND.to_string(),
        name: name
            .map(str::to_string)
            .or_else(|| {
                config
                    .plan_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "SAM profile".to_string()),
        exported_at,
        source: Some(ProfileSource {
            app: Some("SAM".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            revision: Some(config.revision.clone()),
        }),
        includes: ProfileIncludes {
            personal,
            state: state.is_some(),
            record_count,
            excluded_types: excluded_types.clone(),
            excluded_records: excluded_ids.len(),
        },
        documents,
        records,
        state,
    })
}

/// Drop link ids that point at records the export left out: a profile with a
/// dangling relation would not load, and silently rewriting the relation is
/// worse than dropping a link to a note the recipient cannot see.
fn prune_links(value: &mut JSONValue, excluded: &BTreeSet<String>) {
    let Some(links) = value.get_mut("links").and_then(JSONValue::as_object_mut) else {
        return;
    };
    let keys: Vec<String> = links.keys().cloned().collect();
    for key in keys {
        let Some(ids) = links.get_mut(&key).and_then(JSONValue::as_array_mut) else {
            continue;
        };
        ids.retain(|id| match id.as_str() {
            Some(id) => !excluded.contains(id),
            None => true,
        });
        if ids.is_empty() {
            links.remove(&key);
        }
    }
}

/// The profile's canonical bytes: pretty JSON, sorted keys, final newline —
/// the same serialization every other SAM-authored document uses (§4.1).
pub fn to_bytes(document: &ProfileDoc) -> Result<Vec<u8>, String> {
    let value = serde_json::to_value(document).map_err(|error| error.to_string())?;
    Ok(crate::doc_edit::doc_json::pretty(&value).into_bytes())
}

/// Parse and check a profile document: the kind must match and the version
/// must be one this build understands.
pub fn from_bytes(bytes: &[u8]) -> Result<ProfileDoc, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the profile is not UTF-8".to_string())?;
    let value = crate::strict_json::parse(text, "profile")
        .map_err(|error| format!("profile: {}", error.diagnostic.message))?;
    let document: ProfileDoc =
        serde_json::from_value(value).map_err(|error| format!("profile: {error}"))?;
    if document.kind != PROFILE_KIND {
        return Err(format!(
            "this file says kind \"{}\" — a SAM profile says \"{PROFILE_KIND}\"",
            document.kind
        ));
    }
    match document.schema_version {
        Some(1) => {}
        Some(other) => {
            return Err(format!(
                "profile schemaVersion {other} is newer than this build (1)"
            ));
        }
        None => return Err("profile declares no schemaVersion".into()),
    }
    if document.documents.is_empty() {
        return Err("profile carries no documents".into());
    }
    for path in document.documents.keys() {
        if !PROFILE_DOCUMENTS.contains(&path.as_str()) {
            return Err(format!(
                "profile carries \"{path}\", which is not one of {}",
                PROFILE_DOCUMENTS.join(", ")
            ));
        }
    }
    if !document.documents.contains_key("content/types.json") {
        return Err("profile carries no content/types.json".into());
    }
    Ok(document)
}

// ── import ───────────────────────────────────────────────────────────────────

/// Materialize a profile into a new plan at `target`, staged and validated
/// before it is visible. A record of a kind the profile does not declare, a
/// malformed record or a dangling relation fails the import with the loader's
/// own line-precise diagnostic — and leaves nothing behind.
pub fn import(
    document: &ProfileDoc,
    target: &Path,
    resources_dir: &Path,
) -> Result<PathBuf, crate::config_store::ConfigStoreError> {
    crate::config_store::create_plan_with(target, resources_dir, |staging| {
        fill_tree(document, staging)
    })
}

/// Write a profile's documents and records into a fresh plan tree.
pub fn fill_tree(document: &ProfileDoc, root: &Path) -> Result<(), String> {
    let types_value = document
        .documents
        .get("content/types.json")
        .ok_or_else(|| "profile carries no content/types.json".to_string())?;
    let declared: BTreeSet<String> = types_value
        .get("types")
        .and_then(JSONValue::as_object)
        .map(|types| types.keys().cloned().collect())
        .ok_or_else(|| "content/types.json declares no types".to_string())?;

    for (type_id, rows) in &document.records {
        if !crate::doc_edit::type_ops::safe_type_id(type_id) {
            return Err(format!(
                "type \"{type_id}\" is not a safe records-file name"
            ));
        }
        if !declared.contains(type_id) {
            return Err(format!(
                "profile carries {type_id} records but declares no \"{type_id}\" type"
            ));
        }
        if rows.len() > crate::apply::MAX_BATCH_RECORDS {
            return Err(format!(
                "{type_id} carries {} records; the batch limit is {}",
                rows.len(),
                crate::apply::MAX_BATCH_RECORDS
            ));
        }
    }

    for (path, value) in &document.documents {
        let text = crate::doc_edit::doc_json::pretty(value);
        write_file(&root.join(path), text.as_bytes())?;
    }

    let records_dir = root.join("content/records");
    std::fs::create_dir_all(&records_dir)
        .map_err(|error| format!("cannot create {}: {error}", records_dir.display()))?;
    for (type_id, rows) in &document.records {
        let file = format!("content/records/{type_id}.jsonl");
        let mut text = String::new();
        for (index, value) in rows.iter().enumerate() {
            let line = index as u64 + 1;
            let record: Record = crate::decode::record(value, &file, line)
                .map_err(|error| format!("{file}#line:{line}: {}", error.diagnostic.message))?;
            if record.type_ != *type_id {
                return Err(format!(
                    "{file}#line:{line}: record {} says type \"{}\", not \"{type_id}\"",
                    record.id, record.type_
                ));
            }
            text.push_str(
                &ResolvedConfig::canonical_line(&record)
                    .map_err(|error| format!("{file}#line:{line}: {error}"))?,
            );
            text.push('\n');
        }
        write_file(
            &records_dir.join(format!("{type_id}.jsonl")),
            text.as_bytes(),
        )?;
    }

    if let Some(state) = &document.state {
        let text = crate::doc_edit::doc_json::pretty(state);
        write_file(&root.join(STATE_DOCUMENT), text.as_bytes())?;
    }
    Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
    }

    fn seed() -> ResolvedConfig {
        crate::config_store::load_with(&resources().join("presets/seed"), "explicit", &resources())
            .expect("the seed loads")
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sam-profile-{tag}-{}",
            crate::transaction::make_txid()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_default_export_drops_private_kinds_and_their_links() {
        let config = seed();
        let doc = export(&config, false, Some("Shared term"), None).expect("exports");
        assert_eq!(doc.kind, PROFILE_KIND);
        assert_eq!(doc.includes.personal, false);
        assert!(doc.state.is_none(), "no progress state without --personal");
        assert!(
            doc.includes.excluded_types.contains(&"note".to_string())
                && doc.includes.excluded_types.contains(&"session".to_string()),
            "note and session are private in the seed: {:?}",
            doc.includes.excluded_types
        );
        assert!(!doc.records.contains_key("note"));
        assert!(!doc.records.contains_key("session"));
        // Every remaining record's links point at records the profile carries.
        let carried: BTreeSet<String> = doc
            .records
            .values()
            .flatten()
            .filter_map(|record| record.get("id").and_then(JSONValue::as_str))
            .map(str::to_string)
            .collect();
        for rows in doc.records.values() {
            for record in rows {
                for ids in record
                    .get("links")
                    .and_then(JSONValue::as_object)
                    .into_iter()
                    .flat_map(|links| links.values())
                {
                    for id in ids.as_array().into_iter().flatten() {
                        let id = id.as_str().unwrap_or_default();
                        assert!(
                            carried.contains(id),
                            "a profile link points at {id}, which the export left out"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_personal_export_carries_state_and_private_records() {
        let mut config = seed();
        let doc =
            export(&config, true, None, Some("2026-09-26T00:00:00Z".into())).expect("exports");
        assert!(doc.includes.personal);
        assert!(doc.records.contains_key("note"));
        assert!(doc.includes.excluded_records == 0);
        // The seed carries no state file, so a personal export may still have
        // none; adding one proves it travels.
        config.source_files.insert(
            STATE_DOCUMENT.to_string(),
            br#"{"schemaVersion":1,"progress":{}}"#.to_vec(),
        );
        let doc = export(&config, true, None, None).expect("exports");
        assert!(doc.state.is_some());
    }

    #[test]
    fn export_import_round_trips_with_and_without_personal_state() {
        for personal in [false, true] {
            let config = seed();
            let doc = export(&config, personal, Some("Round trip"), None).expect("exports");
            let bytes = to_bytes(&doc).expect("serializes");
            let parsed = from_bytes(&bytes).expect("parses");
            assert_eq!(
                parsed, doc,
                "the document round-trips through its own bytes"
            );

            let target = scratch(&format!("import-{personal}"));
            let created = import(&parsed, &target, &resources()).expect("imports and validates");
            let loaded = crate::config_store::load_with(&created, "explicit", &resources())
                .expect("the imported plan loads");

            assert_eq!(loaded.types.len(), config.types.len());
            assert_eq!(
                loaded.views.keys().collect::<Vec<_>>(),
                config.views.keys().collect::<Vec<_>>()
            );
            let expected: Vec<&String> = config
                .records_iter()
                .filter(|record| personal || config.types[&record.type_].private != Some(true))
                .map(|record| &record.id)
                .collect();
            let got: Vec<&String> = loaded.records_iter().map(|record| &record.id).collect();
            assert_eq!(got, expected, "personal={personal}");
            if personal {
                assert!(
                    loaded.source_files.contains_key(STATE_DOCUMENT)
                        || !config.source_files.contains_key(STATE_DOCUMENT)
                );
            } else {
                assert!(!loaded.source_files.contains_key(STATE_DOCUMENT));
            }
            let _ = std::fs::remove_dir_all(&target);
        }
    }

    #[test]
    fn a_foreign_or_broken_profile_is_refused_with_a_reason() {
        let mut doc = export(&seed(), false, None, None).expect("exports");
        doc.kind = "something-else".into();
        let bytes = to_bytes(&doc).unwrap();
        let error = from_bytes(&bytes).expect_err("a foreign kind is refused");
        assert!(error.contains("samprofile"), "{error}");

        let mut doc = export(&seed(), false, None, None).expect("exports");
        doc.schema_version = Some(99);
        let error = from_bytes(&to_bytes(&doc).unwrap()).expect_err("a future version is refused");
        assert!(error.contains("99"), "{error}");

        let mut doc = export(&seed(), false, None, None).expect("exports");
        doc.documents
            .insert("content/secrets.json".into(), serde_json::json!({}));
        let error =
            from_bytes(&to_bytes(&doc).unwrap()).expect_err("an unknown document is refused");
        assert!(error.contains("secrets"), "{error}");
    }

    #[test]
    fn an_imported_record_fails_with_its_own_line_when_it_is_broken() {
        let mut doc = export(&seed(), false, None, None).expect("exports");
        let topic = doc
            .records
            .get_mut("topic")
            .and_then(|rows| rows.first_mut())
            .expect("the seed has topics");
        topic["fields"]["est"] = serde_json::json!("not a duration");
        let target = scratch("broken");
        let error = import(&doc, &target, &resources()).expect_err("the loader refuses it");
        let message = error.to_string();
        assert!(
            message.contains("topic.jsonl") && message.contains("est"),
            "the diagnostic names the file and the field: {message}"
        );
        assert!(!target.exists(), "a refused import leaves nothing behind");
    }

    #[test]
    fn a_profile_never_carries_executable_content() {
        // The shape is closed: documents are plan documents, records are
        // records. A profile with a script key is a load error, not a plugin.
        let mut doc = export(&seed(), false, None, None).expect("exports");
        let value = serde_json::to_value(&doc).unwrap();
        let mut object = value.as_object().cloned().unwrap();
        object.insert("script".into(), serde_json::json!("rm -rf /"));
        doc = serde_json::from_value(JSONValue::Object(object))
            .expect("unknown keys do not fail serde");
        // Unknown top-level keys survive the parse (serde ignores them) and are
        // dropped on re-serialization: they are never executed, and never
        // carried forward.
        let bytes = to_bytes(&doc).unwrap();
        assert!(!String::from_utf8(bytes).unwrap().contains("rm -rf"));
    }

    #[test]
    fn a_hostile_type_id_cannot_escape_the_staging_root() {
        // Both sides of the old check were attacker-controlled: the records
        // key and the types.json that "declares" it come from the same file.
        // `../../pwned` wrote content/records/../../pwned.jsonl — an arbitrary
        // file write — before any loader validation ran. `safe_type_id` is the
        // same rule every other write path already enforces.
        let doc = export(&seed(), false, None, None).expect("exports");
        let mut hostile = doc.clone();
        let types = hostile
            .documents
            .get("content/types.json")
            .cloned()
            .expect("exports carry types");
        let victim = types
            .get("types")
            .and_then(JSONValue::as_object)
            .and_then(|types| types.keys().next().cloned())
            .expect("the seed declares a type");
        hostile
            .documents
            .get_mut("content/types.json")
            .expect("exports carry types")
            .as_object_mut()
            .expect("types.json is an object")
            .insert("../../pwned".into(), serde_json::json!({"label": "Pwned"}));
        let rows = hostile.records.remove(&victim).unwrap_or_default();
        hostile.records.insert("../../pwned".into(), rows);

        let outside = scratch("traversal").join("pwned.jsonl");
        let target = scratch("traversal-target");
        let error = import(&hostile, &target, &resources()).expect_err("the type id is refused");
        assert!(
            error.to_string().contains("not a safe records-file name"),
            "the refusal names the rule: {error}"
        );
        assert!(
            !outside.exists(),
            "nothing was written outside the staging root"
        );
        assert!(!target.exists(), "a refused import leaves nothing behind");
    }
}
