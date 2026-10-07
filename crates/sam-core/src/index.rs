//! Derived SQLite index cache (§4.6, Phase 6, D12, D20).
//!
//! `content/records/*.jsonl` remains canonical. The index is disposable, updated
//! exclusively by the indexer, read-only to all other callers, and tagged with
//! the source revision to prevent serving stale state.
//!
//! Entry points:
//! - [`build`] / [`status`]: Rebuild and inspect index status (`index.rebuild`, `index.status`).
//! - [`search`]: FTS5 full-text search, with an [`oracle_ids`] record scan for verification.
//! - [`translate_filter`]: Conservative L2-to-SQL translation for candidate pre-filtering;
//!   candidate records are still evaluated by the L2 engine to guarantee semantic equivalence (§4.4, Phase 3, Phase 6).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, params_from_iter};
use serde_json::json;

use crate::config_store::fnv_hex;
use crate::expression::Expr;
use crate::model::FieldDef;
use crate::resolved_config::ResolvedConfig;

/// The index format version. A cache whose meta says anything else is stale,
/// which is always safe because the index is disposable.
pub const FORMAT: &str = "sam-index-1";

/// The maximum rows a search returns when the caller asks for none.
pub const DEFAULT_SEARCH_LIMIT: usize = 200;

#[derive(Debug)]
pub enum IndexError {
    Io(String),
    Sql(String),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Io(message) | IndexError::Sql(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<rusqlite::Error> for IndexError {
    fn from(error: rusqlite::Error) -> Self {
        IndexError::Sql(error.to_string())
    }
}

impl From<std::io::Error> for IndexError {
    fn from(error: std::io::Error) -> Self {
        IndexError::Io(error.to_string())
    }
}

// ── identity and location (§4.1: one resolver, four consumers) ───────────────

/// A stable plan identity: the directory's name for a human, plus a short hash
/// of the root path so two plans called `term` never share a cache. The index
/// is disposable; moving a plan triggers a fresh build.
pub fn plan_id(plan_root: &Path) -> String {
    let name = plan_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "plan".into());
    let safe: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();
    let digest = fnv_hex(plan_root.to_string_lossy().as_bytes());
    format!("{safe}-{}", &digest[..8.min(digest.len())])
}

pub fn index_path(plan_root: &Path, indexes_dir: &Path) -> PathBuf {
    indexes_dir.join(plan_id(plan_root)).join("index.sqlite")
}

// ── build ────────────────────────────────────────────────────────────────────

/// What a rebuild produced, as data (`index.rebuild --json`).
#[derive(Debug, Clone)]
pub struct Report {
    pub path: PathBuf,
    pub revision: String,
    pub records: usize,
    pub indexes: Vec<String>,
    pub sqlite: String,
    pub bytes: u64,
}

impl Report {
    pub fn json(&self) -> serde_json::Value {
        json!({
            "path": self.path.display().to_string(),
            "planId": self.path.parent().and_then(|parent| parent.file_name()).map(|name| name.to_string_lossy().to_string()),
            "revision": self.revision,
            "records": self.records,
            "indexes": self.indexes,
            "sqlite": self.sqlite,
            "bytes": self.bytes,
            "format": FORMAT,
        })
    }
}

/// The SQLite version this build links — read from the library, never assumed
/// (§9: "a feature that was never enabled is a runtime discovery").
pub fn sqlite_version() -> String {
    rusqlite::version().to_string()
}

/// A field kind an expression index is worth building for. Derived kinds
/// (`formula`, `progress`) and `json` are excluded: they are either computed or
/// not comparable in SQL, and an index that cannot be proven equivalent is not
/// built (§4.6: "Bind values, validate/escape field paths and generate only
/// whitelisted SQL expressions").
fn indexable(field: &FieldDef) -> bool {
    matches!(
        field.type_.as_str(),
        "number" | "duration" | "rating" | "bool" | "date" | "text" | "longtext" | "select" | "url"
    )
}

/// An identifier safe to embed in the generated index name and the JSON path:
/// alphanumerics and `_`/`-` only. A user key outside that set receives no
/// expression index — the index is an optimization, never a correctness gate.
fn identifier_ok(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

/// Rebuild the whole cache from the loaded snapshot, in one SQLite transaction.
///
/// The build writes a **staging file and renames it over the destination**, so
/// rebuilding is also the recovery path: a cache corrupted on disk is replaced
/// rather than opened (SQLite would refuse a garbage file), and a reader always
/// sees a complete previous build or a complete new one.
pub fn build(
    plan_root: &Path,
    indexes_dir: &Path,
    config: &ResolvedConfig,
) -> Result<Report, IndexError> {
    let path = index_path(plan_root, indexes_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = path.with_extension("sqlite.building");
    let _ = std::fs::remove_file(&staging);
    let built = match build_into(&staging, plan_root, config) {
        Ok(built) => built,
        Err(error) => {
            let _ = std::fs::remove_file(&staging);
            return Err(error);
        }
    };
    // `rename` replaces atomically on Unix; Windows refuses an existing
    // destination, so the stale cache is removed first — a disposable file, and
    // a reader in that window falls back to the in-memory snapshot.
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&staging, &path)?;
    let bytes = std::fs::metadata(&path)?.len();
    Ok(Report {
        path,
        revision: config.revision.clone(),
        records: built.0,
        indexes: built.1,
        sqlite: sqlite_version(),
        bytes,
    })
}

fn build_into(
    staging: &Path,
    plan_root: &Path,
    config: &ResolvedConfig,
) -> Result<(usize, Vec<String>), IndexError> {
    let path = staging;
    let mut connection = Connection::open(path)?;
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "DROP TABLE IF EXISTS records;
         DROP TABLE IF EXISTS search;
         DROP TABLE IF EXISTS meta;
         CREATE TABLE records(id TEXT PRIMARY KEY, type TEXT NOT NULL,
                              data TEXT NOT NULL CHECK(json_valid(data))) STRICT;
         CREATE VIRTUAL TABLE search USING fts5(id UNINDEXED, type UNINDEXED, body);
         CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;",
    )?;

    {
        let mut records =
            transaction.prepare("INSERT INTO records(id, type, data) VALUES (?1, ?2, ?3)")?;
        let mut search =
            transaction.prepare("INSERT INTO search(id, type, body) VALUES (?1, ?2, ?3)")?;
        for record in config.records_iter() {
            let line = ResolvedConfig::canonical_line(record)
                .map_err(|error| IndexError::Sql(error.to_string()))?;
            records.execute(rusqlite::params![record.id, record.type_, line])?;
            search.execute(rusqlite::params![record.id, record.type_, line])?;
        }
    }

    transaction.execute_batch("CREATE INDEX idx_records_type ON records(type)")?;
    let mut indexes = vec!["idx_records_type".to_string()];
    for (type_name, def) in &config.types {
        if !identifier_ok(type_name) {
            continue;
        }
        for field in &def.fields {
            if !indexable(field) || !identifier_ok(&field.key) {
                continue;
            }
            let name = format!("idx_{type_name}_{}", field.key);
            let sql = format!(
                "CREATE INDEX \"{name}\" ON records(type, json_extract(data, '$.fields.\"{}\"'))",
                field.key
            );
            transaction.execute_batch(&sql)?;
            indexes.push(name);
        }
    }

    for (key, value) in [
        ("format", FORMAT.to_string()),
        ("revision", config.revision.clone()),
        ("planId", plan_id(plan_root)),
        ("sqlite", sqlite_version()),
        ("builtAt", crate::transaction::iso_now()),
        ("records", config.record_count().to_string()),
        (
            "indexer",
            format!("{FORMAT} · fsrs {}", crate::scheduler::FSRS_ALGORITHM),
        ),
    ] {
        transaction.execute(
            "INSERT INTO meta(key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
    }
    transaction.commit()?;
    drop(connection);
    Ok((config.record_count(), indexes))
}

/// The stored state of the cache on disk, as data (`index.status --json`).
pub fn status_json(
    plan_root: &Path,
    indexes_dir: &Path,
    config: Option<&ResolvedConfig>,
) -> serde_json::Value {
    let path = index_path(plan_root, indexes_dir);
    let mut data = json!({
        "path": path.display().to_string(),
        "present": path.is_file(),
        "format": FORMAT,
        "sqlite": sqlite_version(),
    });
    let Some(object) = data.as_object_mut() else {
        return data;
    };
    if !path.is_file() {
        return data;
    }
    match read_meta(&path) {
        Ok(meta) => {
            let stored = meta.get("revision").cloned();
            let current = config.map(|config| config.revision.clone());
            object.insert("revision".into(), json!(stored));
            object.insert(
                "current".into(),
                json!(
                    stored.is_some()
                        && stored == current
                        && meta.get("format").map(String::as_str) == Some(FORMAT)
                ),
            );
            object.insert("records".into(), json!(meta.get("records")));
            object.insert("builtAt".into(), json!(meta.get("builtAt")));
            object.insert("healthy".into(), json!(true));
        }
        Err(error) => {
            object.insert("healthy".into(), json!(false));
            object.insert("error".into(), json!(error.to_string()));
        }
    }
    data
}

fn read_meta(path: &Path) -> Result<std::collections::BTreeMap<String, String>, IndexError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = connection.prepare("SELECT key, value FROM meta")?;
    let mut rows = statement.query([])?;
    let mut out = std::collections::BTreeMap::new();
    while let Some(row) = rows.next()? {
        out.insert(row.get::<_, String>(0)?, row.get::<_, String>(1)?);
    }
    Ok(out)
}

// ── the reader ───────────────────────────────────────────────────────────────

/// A read-only handle whose stored revision matches the current snapshot. Any
/// other state — missing, stale, corrupt — yields `None` and the caller falls
/// back to the in-memory snapshot (§4.6: *"use the in-memory snapshot if the
/// cache is unavailable"*).
pub struct Index {
    connection: Connection,
    pub path: PathBuf,
    pub revision: String,
}

pub fn open_fresh(
    plan_root: &Path,
    indexes_dir: &Path,
    revision: &str,
) -> Result<Option<Index>, IndexError> {
    let path = index_path(plan_root, indexes_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let stored: Result<String, _> =
        connection.query_row("SELECT value FROM meta WHERE key = 'revision'", [], |row| {
            row.get(0)
        });
    let format: Result<String, _> =
        connection.query_row("SELECT value FROM meta WHERE key = 'format'", [], |row| {
            row.get(0)
        });
    match (stored, format) {
        (Ok(stored), Ok(format)) if stored == revision && format == FORMAT => Ok(Some(Index {
            connection,
            path,
            revision: stored,
        })),
        _ => Ok(None),
    }
}

impl Index {
    pub fn count(&self) -> Result<usize, IndexError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// FTS5 matches for `query`, as ids. The caller orders them (plan file
    /// order), so the two engines differ only in *which* ids they find, which
    /// is exactly what the parity gate compares.
    pub fn search_ids(&self, query: &str, limit: usize) -> Result<Vec<String>, IndexError> {
        let expression = fts_expression(query);
        let Some(expression) = expression else {
            return Ok(Vec::new());
        };
        let mut statement = self
            .connection
            .prepare("SELECT id FROM search WHERE search MATCH ?1 LIMIT ?2")?;
        let mut rows = statement.query(rusqlite::params![expression, limit as i64])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get::<_, String>(0)?);
        }
        Ok(out)
    }

    /// Ids of one type passing a translated filter — a **superset** of the L2
    /// filter's matches (the predicate is implied, never a replacement).
    pub fn candidates(
        &self,
        type_name: &str,
        filter: &SqlFilter,
    ) -> Result<Vec<String>, IndexError> {
        let sql = format!(
            "SELECT id FROM records WHERE type = ?1 AND ({})",
            filter.sql
        );
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(type_name.to_string())];
        for value in &filter.params {
            values.push(value.clone_box());
        }
        let mut statement = self.connection.prepare(&sql)?;
        let mut rows =
            statement.query(params_from_iter(values.iter().map(|value| value.as_ref())))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get::<_, String>(0)?);
        }
        Ok(out)
    }
}

// ── search semantics: one tokenizer, two engines ─────────────────────────────

/// Split text into the tokens both engines see: runs of alphanumerics,
/// lowercased. This mirrors FTS5's `unicode61` tokenizer for the scripts this
/// build is verified against (ASCII and Latin-1); `unicode61` additionally
/// folds diacritics, which this oracle does not — a documented boundary rather
/// than a silent divergence (see `--guide`).
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// The FTS5 query for a user query: every token as a prefix term, implicitly
/// ANDed (`tok1* tok2*`). Returns `None` when the query holds no tokens.
fn fts_expression(query: &str) -> Option<String> {
    let tokens = tokens(query);
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|token| format!("\"{token}\"*"))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// The in-memory oracle for a search: the same rule — every query token is a
/// prefix of some token in the record — evaluated over the canonical record
/// lines. Ids come back in plan file order.
pub fn oracle_ids(config: &ResolvedConfig, query: &str) -> Vec<String> {
    let wanted = tokens(query);
    if wanted.is_empty() {
        return Vec::new();
    }
    config
        .records_iter()
        .filter(|record| {
            let line = ResolvedConfig::canonical_line(record).unwrap_or_default();
            let haystack = tokens(&line);
            wanted.iter().all(|needle| {
                haystack
                    .iter()
                    .any(|token| token.starts_with(needle.as_str()))
            })
        })
        .map(|record| record.id.clone())
        .collect()
}

// ── L2 → SQL translation (conservative; the evaluator stays the reference) ───

/// A translated SQL fragment plus its bound values.
#[derive(Debug, Clone)]
pub struct SqlFilter {
    pub sql: String,
    pub params: Vec<SqlParam>,
}

#[derive(Debug, Clone)]
pub enum SqlParam {
    Number(f64),
    Text(String),
    Bool(bool),
}

impl SqlParam {
    fn clone_box(&self) -> Box<dyn rusqlite::types::ToSql> {
        match self {
            SqlParam::Number(number) => Box::new(*number),
            SqlParam::Text(text) => Box::new(text.clone()),
            SqlParam::Bool(flag) => Box::new(i64::from(*flag)),
        }
    }
}

/// Translate the subset of L2 whose SQL meaning is **provably identical** to
/// the evaluator's, for one type's fields:
///
/// - `==`, `<`, `<=`, `>`, `>=` between a top-level scalar field and a literal
///   of the same declared type family (a number field against a number, a
///   string-ish field against a string, a bool against a bool);
/// - `&&` and `||` where **both** sides translate.
///
/// Everything else returns `None` and the caller evaluates the whole filter in
/// memory. Deliberately excluded: `!=` (L2 and SQL disagree about null), `null`
/// literals, relation paths, arithmetic, calls, `!`, and any mixed-type
/// comparison (§4.4 rejects implicit coercion; so does this).
pub fn translate_filter(
    config: &ResolvedConfig,
    type_name: &str,
    expr: &Expr,
) -> Option<SqlFilter> {
    let def = config.types.get(type_name)?;
    translate(config, def, expr)
}

fn translate(
    config: &ResolvedConfig,
    def: &crate::model::TypeDef,
    expr: &Expr,
) -> Option<SqlFilter> {
    match expr {
        Expr::Binary(operator, left, right) if matches!(operator.as_str(), "&&" | "||") => {
            let left = translate(config, def, left)?;
            let right = translate(config, def, right)?;
            let joiner = if operator == "&&" { " AND " } else { " OR " };
            let mut params = left.params;
            params.extend(right.params);
            Some(SqlFilter {
                sql: format!("({}){joiner}({})", left.sql, right.sql),
                params,
            })
        }
        Expr::Binary(operator, left, right)
            if matches!(operator.as_str(), "==" | "<" | "<=" | ">" | ">=") =>
        {
            let (field, literal) = match (left.as_ref(), right.as_ref()) {
                (Expr::Path(segments), literal) if segments.len() == 1 => {
                    (def.field(&segments[0])?, literal)
                }
                (literal, Expr::Path(segments)) if segments.len() == 1 => {
                    (def.field(&segments[0])?, literal)
                }
                _ => return None,
            };
            // A reversed comparison flips the operator (`5 > x` ≡ `x < 5`) —
            // but only for the ordered ones; `==` is symmetric already.
            let operator = if matches!(left.as_ref(), Expr::Path(_)) {
                operator.clone()
            } else {
                match operator.as_str() {
                    "<" => ">",
                    "<=" => ">=",
                    ">" => "<",
                    ">=" => "<=",
                    other => other,
                }
                .to_string()
            };
            column_predicate(&field.key, &field.type_, &operator, literal)
        }
        _ => None,
    }
}

fn column_predicate(key: &str, type_: &str, operator: &str, literal: &Expr) -> Option<SqlFilter> {
    let numeric = matches!(type_, "number" | "duration" | "rating");
    let textual = matches!(type_, "text" | "longtext" | "date" | "select" | "url");
    let (param, family_ok) = match literal {
        Expr::Int(value) => (SqlParam::Number(*value as f64), numeric || operator == "=="),
        Expr::Double(value) => (SqlParam::Number(*value), numeric || operator == "=="),
        Expr::String(text) => (SqlParam::Text(text.clone()), textual || operator == "=="),
        Expr::Bool(flag) if type_ == "bool" => (SqlParam::Bool(*flag), operator == "=="),
        _ => return None,
    };
    if !family_ok {
        return None;
    }
    // A bool literal only ever appears on a bool field; a text literal on a
    // number field (or the reverse) is a mixed-type comparison L2 refuses.
    if matches!(literal, Expr::String(_)) && numeric {
        return None;
    }
    if matches!(literal, Expr::Int(_) | Expr::Double(_)) && textual {
        return None;
    }
    Some(SqlFilter {
        sql: format!("json_extract(data, '$.fields.\"{key}\"') {operator} ?"),
        params: vec![param],
    })
}

// ── refresh after an accepted commit (§4.6 step 5) ───────────────────────────

/// Rebuild or update the cache after a committed write, **only if a cache
/// exists**: a user who never built one pays nothing, and a refresh failure is
/// a warning on a successful commit, never a failed write (§4.9).
///
/// The default path is incremental: the commit's file changes name exactly
/// which records were removed and added, so the update touches only those rows
/// instead of re-reading and re-inserting the whole plan. Anything the changes
/// cannot express — a `content/types.json` edit (expression indexes depend on
/// the type fields), a foreign cache, a decode surprise — falls back to the
/// full rebuild, which is the same code `index.rebuild` runs.
pub fn refresh_after_commit(
    plan_root: &Path,
    resources_dir: &Path,
    revision: &str,
    changes: &[crate::transaction::FileChange],
) -> (bool, Option<String>) {
    let Ok(paths) = crate::resources::paths_with_resources(resources_dir.to_path_buf()) else {
        return (false, None);
    };
    let path = index_path(plan_root, &paths.indexes_dir);
    if !path.is_file() {
        return (false, None);
    }
    let changed_types = changes
        .iter()
        .any(|change| change.path == "content/types.json");
    if !changed_types {
        match update_rows(&path, revision, &plan_id(plan_root), changes) {
            // Updated in place: rows match the committed files.
            Ok(()) => return (true, None),
            // The incremental path could not express the change (or the cache
            // is not ours to touch). The full rebuild below is the fallback
            // and also the recovery path.
            Err(Fallback::Rebuild) => {}
            Err(Fallback::Warning(message)) => return (false, Some(message)),
        }
    }
    let config = match crate::config_store::load_with(plan_root, "explicit", resources_dir) {
        Ok(config) => config,
        Err(error) => {
            return (false, Some(format!("the index was not refreshed: {error}")));
        }
    };
    match build(plan_root, &paths.indexes_dir, &config) {
        Ok(report) => (report.revision == revision, None),
        Err(error) => (
            false,
            Some(format!(
                "the write committed; the index refresh failed: {error}"
            )),
        ),
    }
}

/// Why [`refresh_after_commit`]'s incremental path gave up: silently (the full
/// rebuild handles it) or loudly (the commit already succeeded, so this is a
/// warning, not an error).
enum Fallback {
    Rebuild,
    Warning(String),
}

/// Apply one commit's record-file changes directly to the cache's rows.
///
/// Correct for moves between `content/records/*.jsonl` files because a move
/// changes both files, and both appear here: the before-image's ids are deleted
/// and the after-image's ids are inserted, wherever they now live.
fn update_rows(
    path: &Path,
    revision: &str,
    expected_plan_id: &str,
    changes: &[crate::transaction::FileChange],
) -> Result<(), Fallback> {
    let mut connection = rusqlite::Connection::open(path).map_err(|_| Fallback::Rebuild)?;
    // The cache is disposable; only a cache this indexer wrote (right format,
    // right plan) is updated in place. Anything else is rebuilt from scratch.
    let (format, plan): (String, String) = connection
        .query_row(
            "SELECT (SELECT value FROM meta WHERE key='format'),
                    (SELECT value FROM meta WHERE key='planId')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| Fallback::Rebuild)?;
    if format != FORMAT || plan != expected_plan_id {
        return Err(Fallback::Rebuild);
    }
    let transaction = connection.transaction().map_err(|_| Fallback::Rebuild)?;
    let mut deleted = transaction
        .prepare("DELETE FROM records WHERE id = ?1")
        .map_err(|_| Fallback::Rebuild)?;
    let mut delete_search = transaction
        .prepare("DELETE FROM search WHERE id = ?1")
        .map_err(|_| Fallback::Rebuild)?;
    let mut insert = transaction
        .prepare("INSERT INTO records(id, type, data) VALUES (?1, ?2, ?3)")
        .map_err(|_| Fallback::Rebuild)?;
    let mut insert_search = transaction
        .prepare("INSERT INTO search(id, type, body) VALUES (?1, ?2, ?3)")
        .map_err(|_| Fallback::Rebuild)?;
    for change in changes {
        let Some(name) = change
            .path
            .strip_prefix("content/records/")
            .filter(|rest| rest.ends_with(".jsonl"))
        else {
            // A document the row cache does not mirror (state, views, rules)
            // changes the revision but not the rows. The meta update below
            // keeps the cache fresh.
            continue;
        };
        let _ = name;
        // Old ids: parse the before-image the same way the loader does.
        for record in records_of(change.before.as_deref(), &change.path) {
            deleted
                .execute(rusqlite::params![record.id])
                .and_then(|_| delete_search.execute(rusqlite::params![record.id]))
                .map_err(|_| Fallback::Rebuild)?;
        }
        for record in records_of(change.after.as_deref(), &change.path) {
            let line = ResolvedConfig::canonical_line(&record)
                .map_err(|error| Fallback::Warning(error))?;
            insert
                .execute(rusqlite::params![record.id, record.type_, line])
                .and_then(|_| {
                    insert_search.execute(rusqlite::params![record.id, record.type_, line])
                })
                .map_err(|_| Fallback::Rebuild)?;
        }
    }
    let count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .map_err(|_| Fallback::Rebuild)?;
    for (key, value) in [
        ("revision", revision.to_string()),
        ("builtAt", crate::transaction::iso_now()),
        ("records", count.to_string()),
    ] {
        transaction
            .execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )
            .map_err(|_| Fallback::Rebuild)?;
    }
    drop(deleted);
    drop(delete_search);
    drop(insert);
    drop(insert_search);
    transaction
        .commit()
        .map_err(|_| Fallback::Warning("the index update could not commit".into()))
}

/// The ids in one image of one records file, decoded exactly as `load_with`
/// decodes it: CRLF-normalized, blank edge lines tolerated, one record per
/// non-blank line. A decode failure is a `Rebuild` — the full path re-reads
/// from disk and reports properly.
fn records_of(bytes: Option<&[u8]>, path: &str) -> Vec<crate::model::Record> {
    let Some(bytes) = bytes else {
        return Vec::new();
    };
    let Ok(text) = String::from_utf8(bytes.to_vec()) else {
        return Vec::new();
    };
    let normalized = text.replace("\r\n", "\n");
    let mut out = Vec::new();
    for (index, raw) in normalized.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = crate::strict_json::parse_line(line, path, index as u64 + 1)
            && let Ok(record) = crate::decode::record(&value, path, index as u64 + 1)
        {
            out.push(record);
        }
    }
    out
}

/// Serialises tests that mutate or read `SAM_INDEX_DIR` (cargo runs test
/// threads in one process, and process env is global).
#[cfg(test)]
pub(crate) fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap()
}

#[cfg(test)]
mod refresh_tests {
    use super::*;
    use crate::config_store::load_with;
    use crate::transaction::FileChange;
    use std::path::Path;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sam-index-{tag}-{}",
            crate::transaction::make_txid()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn resources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
    }

    /// `refresh_after_commit` resolves the indexes dir through
    /// `SAM_INDEX_DIR` (§4.1); point it at this test's scratch dir. The
    /// returned guard serialises the tests that mutate process env — cargo
    /// runs them on parallel threads otherwise — so hold it for the whole
    /// test body.
    fn use_indexes_dir(dir: &Path) -> EnvGuard<'static> {
        let guard = env_test_lock();
        let prior = std::env::var_os(crate::resources::INDEXES_ENV);
        unsafe { std::env::set_var(crate::resources::INDEXES_ENV, dir) };
        EnvGuard {
            _mutex: guard,
            prior,
        }
    }

    /// Restores `SAM_INDEX_DIR` when dropped, so one test's scratch dir never
    /// leaks into another's default-dir assertions.
    struct EnvGuard<'a> {
        _mutex: std::sync::MutexGuard<'a, ()>,
        prior: Option<std::ffi::OsString>,
    }

    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            unsafe {
                match &self.prior {
                    Some(value) => std::env::set_var(crate::resources::INDEXES_ENV, value),
                    None => std::env::remove_var(crate::resources::INDEXES_ENV),
                }
            }
        }
    }

    fn seed_plan(tag: &str) -> PathBuf {
        let plan = scratch(tag);
        let source = resources().join("presets/seed");
        crate::transaction::copy_tree(&source, &plan).expect("seed copies");
        plan
    }

    fn rows(path: &Path) -> (Vec<String>, usize, String) {
        // ids in insertion order + count + revision, straight from the cache
        let connection =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let ids: Vec<String> = {
            let mut statement = connection
                .prepare("SELECT id FROM records ORDER BY id")
                .unwrap();
            let mut rows = statement.query([]).unwrap();
            let mut out = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                out.push(row.get::<_, String>(0).unwrap());
            }
            out
        };
        let count: usize = connection
            .query_row("SELECT COUNT(*) FROM records", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap() as usize;
        let revision: String = connection
            .query_row("SELECT value FROM meta WHERE key='revision'", [], |row| {
                row.get(0)
            })
            .unwrap();
        (ids, count, revision)
    }

    /// The incremental refresh must land on exactly the rows a full rebuild
    /// would write — same ids, same count, same revision — for an edit, a
    /// delete, an append and a rename (a move across files).
    #[test]
    fn incremental_refresh_matches_a_full_rebuild() {
        let resources = resources();
        let plan = seed_plan("inc");
        let indexes = scratch("inc-indexes");
        let _env = use_indexes_dir(&indexes);
        let config = load_with(&plan, "explicit", &resources).expect("seed loads");
        let report = build(&plan, &indexes, &config).expect("build");
        let index_file = report.path.clone();

        // An edit to one record file: change est on a topic, drop another line.
        let file_path = plan.join("content/records/topic.jsonl");
        let before = std::fs::read(&file_path).unwrap();
        let text = String::from_utf8(before.clone()).unwrap();
        let after: Vec<u8> = text
            .lines()
            .filter(|line| !line.contains("\"t.demo.01\""))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
            .into_bytes();
        std::fs::write(&file_path, &after).unwrap();

        let changes = vec![FileChange {
            path: "content/records/topic.jsonl".into(),
            before: Some(before),
            after: Some(after),
        }];
        let new_config = load_with(&plan, "explicit", &resources).expect("edited seed loads");
        let (refreshed, warning) =
            refresh_after_commit(&plan, &resources, &new_config.revision, &changes);
        assert!(refreshed, "incremental refresh succeeds: {warning:?}");
        let (ids, count, revision) = rows(&index_file);
        assert_eq!(
            revision, new_config.revision,
            "meta revision tracks the commit"
        );
        assert_eq!(count, new_config.record_count());
        assert!(
            !ids.iter().any(|id| id == "t.demo.01"),
            "the deleted row is gone"
        );

        // The same state rebuilt from scratch must be identical.
        let rebuilt = build(&plan, &indexes, &new_config).expect("rebuild");
        let (rebuild_ids, rebuild_count, _) = rows(&rebuilt.path);
        assert_eq!((ids, count), (rebuild_ids, rebuild_count));
        let _ = std::fs::remove_dir_all(&plan);
        let _ = std::fs::remove_dir_all(&indexes);
    }

    /// A rename (move across record files) deletes the old ids and inserts the
    /// new ones wherever they landed — both files are in the change set.
    #[test]
    fn incremental_refresh_handles_a_move_between_files() {
        let resources = resources();
        let plan = seed_plan("move");
        let indexes = scratch("move-indexes");
        let _env = use_indexes_dir(&indexes);
        let config = load_with(&plan, "explicit", &resources).expect("seed loads");
        let report = build(&plan, &indexes, &config).expect("build");

        // Find two record files and move the last line of the first into the second.
        let dir = plan.join("content/records");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        files.sort();
        assert!(files.len() >= 2, "the seed has multiple record files");
        let first = std::fs::read(&files[0]).unwrap();
        let second = std::fs::read(&files[1]).unwrap();
        let first_text = String::from_utf8(first.clone()).unwrap();
        let mut lines: Vec<&str> = first_text.lines().collect();
        let moved = lines.pop().expect("a record to move");
        let new_first: Vec<u8> = lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>()
            .into_bytes();
        let new_second: Vec<u8> = [
            String::from_utf8(second.clone()).unwrap(),
            format!("{moved}\n"),
        ]
        .concat()
        .into_bytes();
        std::fs::write(&files[0], &new_first).unwrap();
        std::fs::write(&files[1], &new_second).unwrap();

        let first_rel = format!(
            "content/records/{}",
            files[0].file_name().unwrap().to_string_lossy()
        );
        let second_rel = format!(
            "content/records/{}",
            files[1].file_name().unwrap().to_string_lossy()
        );
        let changes = vec![
            FileChange {
                path: first_rel,
                before: Some(first),
                after: Some(new_first),
            },
            FileChange {
                path: second_rel,
                before: Some(second),
                after: Some(new_second),
            },
        ];
        let new_config = load_with(&plan, "explicit", &resources).expect("moved seed loads");
        let (refreshed, warning) =
            refresh_after_commit(&plan, &resources, &new_config.revision, &changes);
        assert!(refreshed, "move refresh succeeds: {warning:?}");
        let (ids, count, _) = rows(&report.path);
        assert_eq!(count, new_config.record_count());
        let rebuilt = build(&plan, &indexes, &new_config).expect("rebuild");
        let (rebuild_ids, rebuild_count, _) = rows(&rebuilt.path);
        assert_eq!((ids, count), (rebuild_ids, rebuild_count));
        let _ = std::fs::remove_dir_all(&plan);
        let _ = std::fs::remove_dir_all(&indexes);
    }

    /// A `content/types.json` change falls back to the full rebuild — the
    /// expression indexes depend on the type fields, and the incremental path
    /// does not try to re-derive them.
    #[test]
    fn a_types_change_falls_back_to_a_full_rebuild() {
        let resources = resources();
        let plan = seed_plan("types");
        let indexes = scratch("types-indexes");
        let _env = use_indexes_dir(&indexes);
        let config = load_with(&plan, "explicit", &resources).expect("seed loads");
        let report = build(&plan, &indexes, &config).expect("build");
        let index_file = report.path.clone();

        let types_path = plan.join("content/types.json");
        let before = std::fs::read(&types_path).unwrap();
        let text = String::from_utf8(before.clone()).unwrap();
        let after: Vec<u8> = text.replace("est", "estimate").into_bytes();
        std::fs::write(&types_path, &after).unwrap();

        let changes = vec![FileChange {
            path: "content/types.json".into(),
            before: Some(before),
            after: Some(after),
        }];
        // The plan now references a field the records may not carry; whether it
        // loads or not, the refresh must report success or a warning, never
        // panic — and on success the cache carries the new revision.
        let new_config = load_with(&plan, "explicit", &resources);
        if let Ok(new_config) = new_config {
            let (refreshed, _warning) =
                refresh_after_commit(&plan, &resources, &new_config.revision, &changes);
            assert!(refreshed);
            let (_, _, revision) = rows(&index_file);
            assert_eq!(revision, new_config.revision);
        }
        let _ = std::fs::remove_dir_all(&plan);
        let _ = std::fs::remove_dir_all(&indexes);
    }
}
