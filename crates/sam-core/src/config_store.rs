//! `ConfigStore` (§4.2): load → explicit migration → validate → publish, ported
//! from `ConfigStore.swift`. Read-only in Phase 1 — the transactional write
//! path is Phase 2 over this same validation. One authoritative store, several
//! possible writers: the loader tolerates what external editors produce (CRLF,
//! a missing final newline) and rejects what the contract forbids (§4.1).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::decode;
use crate::diagnostic::Diagnostic;
use crate::json_cursor::Cursor;
use crate::json_value::JSONValue;
use crate::model::Record;
use crate::resolved_config::ResolvedConfig;
use crate::strict_json;
use crate::validator;

/// The active schema version (D10). A future version is rejected, never
/// guessed at.
pub const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigStoreError {
    /// A missing or unreadable plan root → CLI exit 4.
    Io(String),
    /// Structural findings — every one collected, not just the first.
    Validation(Vec<Diagnostic>),
}

impl fmt::Display for ConfigStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigStoreError::Io(message) => write!(f, "{message}"),
            ConfigStoreError::Validation(diagnostics) => {
                for diagnostic in diagnostics {
                    writeln!(f, "{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigStoreError {}

/// A record with its source position. Every JSONL diagnostic carries file and
/// line (§4.7: line-level diagnostics), and the loader must know both before it
/// can report one.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedRecord {
    pub record: Record,
    pub file: String,
    pub line: u64,
}

// ── plan-root resolution (§4.1 scope 2) ─────────────────────────────────────

/// Explicit `--plan <dir>` wins; then the `SAM_PLAN` environment form (the
/// assistant's seam, `resources::PLAN_ENV`); otherwise exactly one plan must
/// exist under the platform data directory's `SAM/plans/`. None → I/O error;
/// several → ambiguous (the UI's plan picker is Phase 4).
pub fn plan_root(explicit: Option<&Path>) -> Result<(PathBuf, String), ConfigStoreError> {
    let from_environment = std::env::var_os(crate::resources::PLAN_ENV).map(PathBuf::from);
    plan_root_with(explicit, from_environment.as_deref())
}

/// The resolution without reading the process environment, so tests stay
/// hermetic (nothing here races another test's `set_var`).
pub fn plan_root_with(
    explicit: Option<&Path>,
    environment: Option<&Path>,
) -> Result<(PathBuf, String), ConfigStoreError> {
    if let Some(explicit) = explicit {
        if !explicit.is_dir() {
            return Err(ConfigStoreError::Io(format!(
                "plan root not found: {}",
                explicit.display()
            )));
        }
        return Ok((explicit.to_path_buf(), "explicit".into()));
    }

    if let Some(root) = environment {
        if !root.is_dir() {
            return Err(ConfigStoreError::Io(format!(
                "SAM_PLAN points at {}, which is not a folder",
                root.display()
            )));
        }
        return Ok((root.to_path_buf(), "environment".into()));
    }

    let base = crate::resources::paths()
        .map_err(ConfigStoreError::Io)?
        .plans_dir;
    let mut plans: Vec<PathBuf> = std::fs::read_dir(&base)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default();
    plans.sort();

    match plans.len() {
        0 => Err(ConfigStoreError::Io(
            "no plan found — pass --plan <directory>".into(),
        )),
        1 => Ok((plans.remove(0), "default".into())),
        _ => {
            let names = plans
                .iter()
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(ConfigStoreError::Io(format!(
                "several plans exist ({names}) — pass --plan <directory>"
            )))
        }
    }
}

// ── source fingerprint (§4.6) ───────────────────────────────────────────────

/// Every authoritative document's raw bytes, keyed by plan-relative path: the
/// six documents plus every `records/*.jsonl`.
pub fn source_file_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for rel in [
        "content/types.json",
        "content/views.json",
        "content/rules.json",
        "content/shell.json",
        "content/appearance.json",
        "state/state.json",
    ] {
        if let Ok(bytes) = std::fs::read(root.join(rel)) {
            out.insert(rel.to_string(), bytes);
        }
    }
    let records_dir = root.join("content/records");
    if let Ok(entries) = std::fs::read_dir(&records_dir) {
        let mut files: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        files.sort();
        for file in files {
            if let Some(name) = file.file_name().and_then(|name| name.to_str()) {
                if let Ok(bytes) = std::fs::read(&file) {
                    out.insert(format!("content/records/{name}"), bytes);
                }
            }
        }
    }
    out
}

pub fn source_file_hashes(root: &Path) -> BTreeMap<String, String> {
    source_file_bytes(root)
        .into_iter()
        .map(|(rel, bytes)| (rel, fnv_hex(&bytes)))
        .collect()
}

/// The revision fingerprint over a path→hash map — shared by `load`'s publish
/// step and by [`raw_revision`], so a revision from `paths` is the same value
/// a load would produce (§4.7: an agent pins one and the other accepts it).
pub fn combined_revision(file_hashes: &BTreeMap<String, String>) -> String {
    let joined = file_hashes
        .iter()
        .map(|(path, hash)| format!("{path}\u{0}{hash}"))
        .collect::<Vec<_>>()
        .join("\u{1}");
    fnv_hex(joined.as_bytes())
}

/// Fingerprint of raw source bytes **without validation** — `paths` must work
/// on an invalid plan, because an agent needs the root and revision to repair
/// one (§4.7).
pub fn raw_revision(root: &Path) -> String {
    combined_revision(&source_file_hashes(root))
}

/// FNV-1a 64 — Foundation-free, deterministic, not a cryptographic claim.
pub fn fnv_hex(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

// ── create (§4.1 ownership) ─────────────────────────────────────────────────

/// The bookkeeping a file manager drops into a folder just for being looked
/// at. It is not content: a folder holding only this is still the empty
/// destination the storage door promised, and a plan that picks one up on the
/// way in is not a plan that already exists.
pub fn os_junk(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".DS_Store" | ".localized" | "Thumbs.db" | "desktop.ini" | "Icon\r")
    )
}

/// Whether a folder holds nothing at all — the one existing shape a new plan
/// may take over. `plan.new` refuses a destination that already holds
/// something (a plan, or anything else), and a folder that holds nothing is a
/// destination rather than a duplicate: the storage door can point a blank
/// plan at the empty folder a student chose (§3.2). A file manager's own
/// droppings do not count as content (`os_junk`): Finder writes `.DS_Store`
/// into a folder the moment the dialog shows it.
pub fn empty_directory(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    match std::fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .all(|entry| os_junk(&entry.file_name())),
        Err(_) => false,
    }
}

/// Where a new plan goes: an explicit target directory, or a name under the
/// plans directory. One function, because the terminal and the registry must
/// refuse the same names with the same rule (§4.9's one mechanism; a name is a
/// directory name, never a path).
pub fn plan_destination(
    plans_dir: &Path,
    name: Option<&str>,
    target: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(target) = target {
        return Ok(target.to_path_buf());
    }
    let name = name.unwrap_or("imported").trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(format!(
            "\"{name}\" is not a plan name — a name is one directory, not a path"
        ));
    }
    Ok(plans_dir.join(name))
}

/// A free plan directory for an import that named none: the profile's own name,
/// else `name-2`, `name-3` … A double-click on the same `*.samprofile` twice is
/// not a dead end, and an import never clobbers a plan. An *explicit*
/// `--name`/`--into` still refuses an existing directory: explicit intent
/// deserves the error rather than a suffix.
pub fn free_plan_destination(plans_dir: &Path, name: &str) -> PathBuf {
    let base = name.trim();
    let base = if base.is_empty() { "imported" } else { base };
    let first = plans_dir.join(base);
    if !first.exists() {
        return first;
    }
    for suffix in 2..1000 {
        let candidate = plans_dir.join(format!("{base}-{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    plans_dir.join(format!("{base}-{}", crate::transaction::make_txid()))
}

/// Would `preset` be materialized at `target`? The two checks every creation
/// makes first — the preset exists, the destination is free — answered without
/// writing, so `--dry-run` and the app's *Make a new plan here* probe ask the
/// engine instead of re-deriving the rule. A folder holding only a file
/// manager's droppings is free (`empty_directory`).
pub fn plan_target_check(
    target: &Path,
    preset: &str,
    resources_dir: &Path,
) -> Result<PathBuf, ConfigStoreError> {
    let preset_dir = bundled_dir(resources_dir, &format!("presets/{preset}"))
        .ok_or_else(|| ConfigStoreError::Io(format!("preset not found: {preset}")))?;
    if target.exists() && !empty_directory(target) {
        return Err(ConfigStoreError::Io(format!(
            "plan already exists: {}",
            target.display()
        )));
    }
    Ok(preset_dir)
}

/// Materialize a new plan from a bundled preset: a complete writable copy,
/// staged and validated before it appears at `target`. Never a live merge —
/// preset upgrades are explicit migrations, so nothing deleted can resurrect.
pub fn create_plan(
    target: &Path,
    preset: &str,
    resources_dir: &Path,
) -> Result<PathBuf, ConfigStoreError> {
    let preset_dir = plan_target_check(target, preset, resources_dir)?;
    create_plan_with(target, resources_dir, |staging| {
        crate::transaction::copy_tree(&preset_dir, staging)
            .map_err(|error| format!("cannot stage the preset: {error}"))
    })
}

/// The one staged path every plan creation shares (§4.1 ownership, §4.6): a
/// caller fills a staging directory, the **real loader** proves it, and only
/// then does it appear at `target`. A preset copy and a profile import differ
/// only in what `fill` writes — the validation, the atomic rename and the
/// refusal to leave a half-plan behind are the same code.
pub fn create_plan_with<F>(
    target: &Path,
    resources_dir: &Path,
    fill: F,
) -> Result<PathBuf, ConfigStoreError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    if target.exists() && !empty_directory(target) {
        return Err(ConfigStoreError::Io(format!(
            "plan already exists: {}",
            target.display()
        )));
    }
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| {
        ConfigStoreError::Io(format!("cannot create {}: {error}", parent.display()))
    })?;

    let staging = parent.join(format!(".sam-plan-new-{}", crate::transaction::make_txid()));
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&staging);
    };
    if let Err(error) = fill(&staging) {
        cleanup();
        return Err(ConfigStoreError::Io(error));
    }
    if let Err(error) = crate::transaction::ensure_layout(&staging) {
        cleanup();
        return Err(ConfigStoreError::Io(error.to_string()));
    }
    // Prove the copy validates before it is visible (one atomic appearance).
    if let Err(error) = load_with(&staging, "explicit", resources_dir) {
        cleanup();
        return Err(error);
    }
    // An empty destination is replaced, never shadowed: `rename` onto an
    // existing directory works on Unix and fails on Windows, so the empty
    // folder is removed first either way (one mechanism, every platform). Its
    // file-manager droppings go first — `empty_directory` refused to count
    // them, so `remove_dir` must not trip over them either.
    if target.is_dir() {
        if let Ok(entries) = std::fs::read_dir(target) {
            for entry in entries.filter_map(Result::ok) {
                if os_junk(&entry.file_name()) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        let _ = std::fs::remove_dir(target);
    }
    if let Err(error) = std::fs::rename(&staging, target) {
        cleanup();
        return Err(ConfigStoreError::Io(format!(
            "cannot move the new plan into place: {error}"
        )));
    }
    Ok(target.to_path_buf())
}

/// A bundled plan directory — plain files, never compiled (§9 rule 3). The
/// resource directory the platform resolver already answered IS the bundle's
/// resource root, so this is a lookup, not a second search path.
pub fn bundled_dir(resources_dir: &Path, subpath: &str) -> Option<PathBuf> {
    let candidate = resources_dir.join(subpath);
    candidate.is_dir().then_some(candidate)
}

// ── load ────────────────────────────────────────────────────────────────────

pub fn load(root: &Path, layer: &str) -> Result<ResolvedConfig, ConfigStoreError> {
    let resources = crate::resources::paths()
        .map_err(ConfigStoreError::Io)?
        .resources_dir;
    load_with(root, layer, &resources)
}

/// [`load`] against an explicit resource directory. The token register is a
/// bundled resource (§1.4.A), so a caller that already resolved the bundle —
/// the packaged shell, or a test — hands the answer in rather than letting the
/// engine re-derive it from the executable's path.
pub fn load_with(
    root: &Path,
    layer: &str,
    resources_dir: &Path,
) -> Result<ResolvedConfig, ConfigStoreError> {
    load_impl(root, layer, resources_dir, true)
}

/// A read-path load: identical to [`load_with`] except the raw file bytes are
/// not retained. Reads that only project data (views, today, search) never
/// need `source_files`; only the source pane's file read does, and it stays on
/// [`load_with`]. The revision is untouched — `file_hashes` is filled either
/// way — so a readonly load and a full load report the same revision.
pub fn load_readonly(
    root: &Path,
    layer: &str,
    resources_dir: &Path,
) -> Result<ResolvedConfig, ConfigStoreError> {
    load_impl(root, layer, resources_dir, false)
}

fn load_impl(
    root: &Path,
    layer: &str,
    resources_dir: &Path,
    retain: bool,
) -> Result<ResolvedConfig, ConfigStoreError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut file_hashes: BTreeMap<String, String> = BTreeMap::new();
    // Raw bytes of every authoritative file, and every record with its
    // position: the writer's view of the same plan (§4.6 step 2 verifies
    // against these bytes, §4.7 reports line-level diagnostics).
    let mut source_files: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    // A folder with no `content/` at all is not a broken plan — it is not a
    // plan. `list_plans` asks the same question of a directory it offers, and
    // the storage doors read this one sentence as "a destination": reporting
    // `types.json` missing plus "at least one type" for a folder a student
    // merely picked is a page of findings about a thing that does not exist.
    // A plan that HAS `content/` but lost a document stays a plan with a fault.
    if !root.join("content").is_dir() {
        return Err(ConfigStoreError::Validation(vec![Diagnostic::error(
            "plan.absent",
            ".",
            "this folder is not a plan — it holds no content folder",
        )]));
    }

    check_layout(root, &mut diagnostics);

    let types_tree = read_document(
        root,
        "content/types.json",
        true,
        retain,
        &mut file_hashes,
        &mut source_files,
        &mut diagnostics,
    );
    let views_tree = read_document(
        root,
        "content/views.json",
        false,
        retain,
        &mut file_hashes,
        &mut source_files,
        &mut diagnostics,
    );
    let rules_tree = read_document(
        root,
        "content/rules.json",
        false,
        retain,
        &mut file_hashes,
        &mut source_files,
        &mut diagnostics,
    );
    let shell_tree = read_document(
        root,
        "content/shell.json",
        false,
        retain,
        &mut file_hashes,
        &mut source_files,
        &mut diagnostics,
    );
    let appearance_tree = read_document(
        root,
        "content/appearance.json",
        false,
        retain,
        &mut file_hashes,
        &mut source_files,
        &mut diagnostics,
    );

    // ---- records/*.jsonl — one record per line (§4.1)
    let mut record_lines: Vec<(String, u64, String)> = Vec::new();
    let records_dir = root.join("content/records");
    if records_dir.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&records_dir)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();

        for file in files {
            let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let rel = format!("content/records/{name}");
            let bytes = match std::fs::read(&file) {
                Ok(bytes) => bytes,
                Err(error) => {
                    diagnostics.push(Diagnostic::error(
                        "file.unreadable",
                        rel,
                        format!("cannot read file: {error}"),
                    ));
                    continue;
                }
            };
            file_hashes.insert(rel.clone(), fnv_hex(&bytes));
            if retain {
                source_files.insert(rel.clone(), bytes.clone());
            }
            let Ok(text) = String::from_utf8(bytes) else {
                diagnostics.push(Diagnostic::error(
                    "encoding.invalid",
                    rel,
                    "file is not valid UTF-8",
                ));
                continue;
            };
            // §4.1: CRLF and a missing final newline are tolerated; a blank
            // interior line is not. A Unix file pays no copy: only a file that
            // actually carries CR gets the normalized owned string.
            let normalized: Cow<'_, str> = if text.contains('\r') {
                Cow::Owned(text.replace("\r\n", "\n"))
            } else {
                Cow::Borrowed(text.as_str())
            };
            let mut lines = normalized.split('\n').peekable();
            let mut index = 0usize;
            while let Some(raw) = lines.next() {
                let last = lines.peek().is_none();
                let line = raw.trim();
                if line.is_empty() {
                    if index == 0 || last {
                        index += 1;
                        continue;
                    }
                    diagnostics.push(
                        Diagnostic::error("jsonl.blank-line", rel.clone(), "blank interior line")
                            .at_line(index as u64 + 1),
                    );
                    index += 1;
                    continue;
                }
                record_lines.push((rel.clone(), index as u64 + 1, line.to_string()));
                index += 1;
            }
        }
    }

    // ---- decode (one error per document; records per line)
    let mut types = BTreeMap::new();
    if let Some(tree) = &types_tree {
        match decode::types_file(&Cursor::root("content/types.json", tree)) {
            Ok(decoded) => types = decoded,
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }
    let mut views = BTreeMap::new();
    if let Some(tree) = &views_tree {
        match decode::views_file(&Cursor::root("content/views.json", tree)) {
            Ok(decoded) => views = decoded,
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }
    let mut rules = crate::model::RulesFile {
        schema_version: Some(SCHEMA_VERSION as u64),
        pipelines: None,
        scheduler: None,
        metrics: None,
        lint: None,
        study: None,
        timezone: None,
    };
    if let Some(tree) = &rules_tree {
        match decode::rules_file(&Cursor::root("content/rules.json", tree)) {
            Ok(decoded) => rules = decoded,
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }
    let mut shell = None;
    if let Some(tree) = &shell_tree {
        match decode::shell_file(&Cursor::root("content/shell.json", tree)) {
            Ok(decoded) => shell = Some(decoded),
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }
    let mut appearance = None;
    if let Some(tree) = &appearance_tree {
        match decode::appearance_file(&Cursor::root("content/appearance.json", tree)) {
            Ok(decoded) => appearance = Some(decoded),
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }

    let mut positioned: Vec<PositionedRecord> = Vec::new();
    for (file, line, text) in &record_lines {
        match strict_json::parse_line(text, file, *line)
            .and_then(|value| decode::record(&value, file, *line))
        {
            Ok(record) => positioned.push(PositionedRecord {
                record,
                file: file.clone(),
                line: *line,
            }),
            Err(error) => diagnostics.push(error.diagnostic),
        }
    }

    // ---- state/state.json (§4.6): versioned progress, lightly checked
    let state_path = root.join("state/state.json");
    if state_path.is_file() {
        if let Ok(bytes) = std::fs::read(&state_path) {
            if let Ok(text) = String::from_utf8(bytes.clone()) {
                match strict_json::parse(&text, "state/state.json").and_then(|tree| {
                    Cursor::root("state/state.json", &tree).schema_version(SCHEMA_VERSION)
                }) {
                    Ok(_) => {
                        file_hashes.insert("state/state.json".into(), fnv_hex(&bytes));
                        if retain {
                            source_files.insert("state/state.json".into(), bytes);
                        }
                    }
                    Err(error) => diagnostics.push(error.diagnostic),
                }
            }
        }
    }

    // ---- semantic validation (collects every finding)
    diagnostics.extend(validator::validate(
        &types,
        &views,
        &rules,
        shell.as_ref(),
        &positioned,
    ));

    // ---- tokens: bundled register with appearance overrides merged (§4.1)
    let mut tokens =
        crate::resources::TokenRegister::bundled(resources_dir).map_err(ConfigStoreError::Io)?;
    if let Some(overrides) = appearance
        .as_ref()
        .and_then(|appearance| appearance.overrides.as_ref())
        .filter(|overrides| !overrides.is_null())
    {
        match tokens.merged(overrides) {
            Ok(merged) => tokens = merged,
            Err(message) => diagnostics.push(Diagnostic::error(
                "appearance.override-invalid",
                "content/appearance.json#/overrides",
                message,
            )),
        }
    }

    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(ConfigStoreError::Validation(diagnostics));
    }

    // ---- formula cache: parse every type's expression once, not per record
    // (§4.4). Validation has passed, so every entry parses.
    let mut formulas: BTreeMap<String, crate::expression::Expr> = BTreeMap::new();
    for def in types.values() {
        for field in &def.fields {
            if field.is_formula()
                && let Some(text) = &field.expr
            {
                if let Ok(parsed) = crate::expression::parse(text, "content/types.json") {
                    formulas.insert(text.clone(), parsed);
                }
            }
        }
    }

    // ---- publish: revision = fingerprint of every authoritative byte
    let revision = combined_revision(&file_hashes);
    let load_warnings: Vec<Diagnostic> = diagnostics
        .into_iter()
        .filter(|diagnostic| !diagnostic.is_error())
        .collect();

    let lookup = crate::resolved_config::PlanLookup::build(&positioned);
    Ok(ResolvedConfig {
        plan_root: root.to_path_buf(),
        layer: layer.to_string(),
        types,
        views,
        rules,
        shell,
        appearance,
        positioned_records: positioned,
        lookup,
        formulas,
        source_files,
        tokens,
        revision,
        load_warnings,
    })
}

/// Read one document, hashing and retaining its bytes on the way. A document
/// that exists but does not parse still contributes its hash — the revision
/// describes the source, not the load's success.
fn read_document(
    root: &Path,
    rel: &str,
    required: bool,
    retain: bool,
    file_hashes: &mut BTreeMap<String, String>,
    source_files: &mut BTreeMap<String, Vec<u8>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<JSONValue> {
    let path = root.join(rel);
    if !path.is_file() {
        if required {
            diagnostics.push(Diagnostic::error(
                "file.missing",
                rel,
                "required document is missing",
            ));
        }
        return None;
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "file.unreadable",
                rel,
                format!("cannot read file: {error}"),
            ));
            return None;
        }
    };
    file_hashes.insert(rel.to_string(), fnv_hex(&bytes));
    if retain {
        source_files.insert(rel.to_string(), bytes.clone());
    }
    let Ok(text) = String::from_utf8(bytes) else {
        diagnostics.push(Diagnostic::error(
            "encoding.invalid",
            rel,
            "file is not valid UTF-8",
        ));
        return None;
    };
    match strict_json::parse(&text, rel) {
        Ok(tree) => Some(tree),
        Err(error) => {
            diagnostics.push(error.diagnostic);
            None
        }
    }
}

// ── filesystem safety (§4.6) ────────────────────────────────────────────────

/// Document roles only, no surprises: an unexpected entry is a diagnostic, not
/// something the loader quietly walks past.
pub fn check_layout(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        diagnostics.push(Diagnostic::error(
            "file.unreadable",
            ".",
            "cannot list plan root",
        ));
        return;
    };
    let known_root = ["content", "state"];
    let known_content = [
        "types.json",
        "views.json",
        "rules.json",
        "shell.json",
        "appearance.json",
        "records",
    ];

    for entry in entries.filter_map(|entry| entry.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue; // .DS_Store, .git, .sam (Phase 2's bookkeeping), …
        }
        // A preset is also documentation: `README.md` travels with a bundled
        // preset (its label, its sources, what it is for) and is copied into a
        // plan created from it. It is not a document, so the loader never reads
        // it — but it is expected, not surprising, and the safety rule is about
        // unexpected *data*, not prose.
        let documentation = name.ends_with(".md");
        if !known_root.contains(&name.as_str()) && !documentation {
            diagnostics.push(Diagnostic::error(
                "file.unexpected",
                name.clone(),
                "unexpected entry in plan root (expected content/, state/, or a README.md)",
            ));
        }
        if name != "content" {
            continue;
        }
        let Ok(children) = std::fs::read_dir(entry.path()) else {
            continue;
        };
        for child in children.filter_map(|child| child.ok()) {
            let child_name = child.file_name().to_string_lossy().to_string();
            if child_name.starts_with('.') {
                continue;
            }
            if !known_content.contains(&child_name.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "file.unexpected",
                    format!("content/{child_name}"),
                    "unexpected file in content/",
                ));
            }
            if child_name != "records" {
                continue;
            }
            let Ok(records) = std::fs::read_dir(child.path()) else {
                continue;
            };
            for record in records.filter_map(|record| record.ok()) {
                let record_name = record.file_name().to_string_lossy().to_string();
                if record_name.starts_with('.') {
                    continue;
                }
                if record.path().extension().is_none_or(|ext| ext != "jsonl") {
                    diagnostics.push(Diagnostic::error(
                        "file.unexpected",
                        format!("content/records/{record_name}"),
                        "records/ holds only .jsonl files",
                    ));
                }
            }
        }
    }

    // Symlink escapes: no symlink may stand in for a document role.
    let real_root = root.canonicalize().ok();
    for rel in ["content", "content/records", "state"] {
        let path = root.join(rel);
        if !path.exists() {
            continue;
        }
        let Ok(resolved) = path.canonicalize() else {
            continue;
        };
        let inside = real_root
            .as_ref()
            .is_some_and(|real| resolved.starts_with(real));
        if resolved != path && !inside {
            diagnostics.push(Diagnostic::error(
                "file.symlink-escape",
                rel,
                "symlink escapes the plan root",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_import_name_never_lands_on_an_existing_plan() {
        let dir = std::env::temp_dir().join(crate::transaction::make_txid());
        std::fs::create_dir_all(dir.join("imported")).unwrap();
        std::fs::create_dir_all(dir.join("imported-2")).unwrap();
        assert_eq!(
            free_plan_destination(&dir, "imported"),
            dir.join("imported-3")
        );
        assert_eq!(free_plan_destination(&dir, "fresh"), dir.join("fresh"));
        assert_eq!(free_plan_destination(&dir, ""), dir.join("imported-3"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// The token register is a bundled input (§1.4.A); in the workspace that
    /// is the design system's machine-readable twin.
    fn resources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
    }

    fn load_fixture(name: &str) -> Result<ResolvedConfig, ConfigStoreError> {
        load_with(&fixture(name), "explicit", &resources())
    }

    #[test]
    fn the_problemset_fixture_loads_and_publishes_a_revision() {
        let config = load_fixture("problemset").expect("the fixture validates");
        assert_eq!(config.types.len(), 2);
        assert_eq!(config.record_count(), 5);
        assert_eq!(config.views.len(), 1);
        assert!(!config.revision.is_empty());
        assert_eq!(config.revision, raw_revision(&fixture("problemset")));
    }

    #[test]
    fn crlf_and_a_missing_final_newline_load() {
        let config = load_fixture("crlf-ok").expect("tolerances hold");
        assert_eq!(config.record_count(), 2);
        assert_eq!(
            config.record("t.2").unwrap().fields["title"],
            JSONValue::String("no final newline".into())
        );
    }

    #[test]
    fn every_invalid_fixture_fails_with_its_code() {
        let expected = [
            (
                "invalid/invalid-value",
                "record.invalid-value",
                "content/records/topic.jsonl",
            ),
            (
                "invalid/unknown-schema-version",
                "schema-version.unsupported",
                "content/types.json",
            ),
            (
                "invalid/type-parent-cycle",
                "types.parent-cycle",
                "content/types.json",
            ),
            (
                "invalid/formula-cycle",
                "formula.cycle",
                "content/types.json",
            ),
            (
                "invalid/dangling-relation",
                "record.dangling-link",
                "content/records/topic.jsonl",
            ),
            (
                "invalid/duplicate-ids",
                "record.duplicate-id",
                "content/records/topic.jsonl",
            ),
            ("invalid/empty-input", "json.eof", "content/types.json"),
            (
                "invalid/invalid-utf8",
                "encoding.invalid",
                "content/types.json",
            ),
            (
                "invalid/malformed-jsonl",
                "json.invalid",
                "content/records/topic.jsonl",
            ),
            (
                "invalid/duplicate-keys",
                "json.duplicate-key",
                "content/types.json",
            ),
        ];
        for (name, code, file) in expected {
            let Err(ConfigStoreError::Validation(diagnostics)) = load_fixture(name) else {
                panic!("{name} should fail validation, not load or fail as I/O");
            };
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.path.starts_with(file)),
                "{name}: expected {code} at {file}, got {:?}",
                diagnostics
                    .iter()
                    .map(|diagnostic| (&diagnostic.code, &diagnostic.path))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn a_revision_describes_the_source_not_the_load() {
        // §4.7: `paths` must answer on an INVALID plan, so the raw fingerprint
        // is over bytes and never consults the validator.
        let invalid = fixture("invalid/formula-cycle");
        assert!(!raw_revision(&invalid).is_empty());
        assert_ne!(raw_revision(&invalid), raw_revision(&fixture("problemset")));
    }

    #[test]
    fn a_folder_with_no_plan_is_one_sentence_not_a_page_of_findings() {
        let dir =
            std::env::temp_dir().join(format!("sam-empty-{}", crate::transaction::make_txid()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let Err(ConfigStoreError::Validation(diagnostics)) =
            load_with(&dir, "explicit", &resources())
        else {
            panic!("a folder with no content/ is not a plan");
        };
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, "plan.absent");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plan_that_lost_types_json_is_still_a_plan_with_a_fault() {
        // The absence rule is about a folder with no plan at all: a `content/`
        // directory makes it a plan, and a missing document is a finding about
        // that plan — not "this is not a plan".
        let dir =
            std::env::temp_dir().join(format!("sam-broken-{}", crate::transaction::make_txid()));
        std::fs::create_dir_all(dir.join("content")).expect("content dir");
        let Err(ConfigStoreError::Validation(diagnostics)) =
            load_with(&dir, "explicit", &resources())
        else {
            panic!("a plan without types.json does not load");
        };
        assert!(diagnostics.iter().any(|d| d.code == "file.missing"));
        assert!(!diagnostics.iter().any(|d| d.code == "plan.absent"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_managers_droppings_do_not_make_a_folder_occupied() {
        let dir =
            std::env::temp_dir().join(format!("sam-junk-{}", crate::transaction::make_txid()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join(".DS_Store"), b"finder").expect("ds store");
        assert!(empty_directory(&dir), "Finder junk is not content");
        std::fs::write(dir.join("notes.txt"), b"real").expect("a real entry");
        assert!(!empty_directory(&dir), "a real entry makes it occupied");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_finder_visited_is_still_an_empty_destination() {
        // The storage door's promise, end to end: a "blank" folder the user
        // just looked at in Finder carries `.DS_Store`, and `plan.new` takes
        // it over rather than refusing it as occupied.
        let dir =
            std::env::temp_dir().join(format!("sam-junk-plan-{}", crate::transaction::make_txid()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join(".DS_Store"), b"finder").expect("ds store");
        let created = create_plan(&dir, "blank", &resources()).expect("the folder takes the plan");
        assert_eq!(created, dir);
        assert!(dir.join("content/types.json").is_file());
        assert!(
            !dir.join(".DS_Store").exists(),
            "the old folder is replaced"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_plan_flag_beats_the_environment_and_the_environment_beats_the_default() {
        let flag = std::env::temp_dir().join(format!("sam-plan-flag-{}", crate::transaction::make_txid()));
        let environment =
            std::env::temp_dir().join(format!("sam-plan-env-{}", crate::transaction::make_txid()));
        std::fs::create_dir_all(&flag).expect("temp dir");
        std::fs::create_dir_all(&environment).expect("temp dir");

        let (root, origin) = plan_root_with(Some(&flag), Some(&environment)).expect("the flag wins");
        assert_eq!(root, flag);
        assert_eq!(origin, "explicit");

        let (root, origin) = plan_root_with(None, Some(&environment)).expect("the env answers");
        assert_eq!(root, environment);
        assert_eq!(origin, "environment");

        // Neither given: the data dir's scan decides (here: this machine's
        // plans, whatever they are — the branch exists, which is what matters).
        let fallback = plan_root_with(None, None);
        assert!(!matches!(&fallback, Ok((_, origin)) if origin == "environment"));

        let _ = std::fs::remove_dir_all(&flag);
        let _ = std::fs::remove_dir_all(&environment);
    }

    #[test]
    fn a_stale_sam_plan_path_is_refused_by_name() {
        let missing = std::env::temp_dir().join("sam-plan-that-does-not-exist-xyz");
        let Err(ConfigStoreError::Io(message)) = plan_root_with(None, Some(&missing)) else {
            panic!("a missing SAM_PLAN folder is an I/O failure");
        };
        assert!(message.contains("SAM_PLAN"), "{message}");
    }
}
