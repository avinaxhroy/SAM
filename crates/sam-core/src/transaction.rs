//! Transaction engine (§4.6).
//!
//! Provides atomic, recoverable multi-file writes:
//! verify → stage → journal(prepared) → replace → journal(committed) → finalize.
//!
//! Atomic single-file renames combine with journal logging to guarantee multi-file atomicity.
//! Before-images are retained as backups (default: last 20 revisions). Durability is
//! ensured via fsync on files and journals (§4.6 step 3).
//!
//! Bookkeeping is stored under `.sam/`, which is ignored by config loading and does
//! not increment content revisions (§4.6).
//!
//! Crash points tested during recovery verification:
//! [`CrashPoint::Staged`], [`CrashPoint::Prepared`], [`CrashPoint::MidRenames`],
//! [`CrashPoint::Renamed`], [`CrashPoint::Committed`].

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config_store::{combined_revision, fnv_hex};
use crate::diagnostic::Diagnostic;
use crate::json_value::JSONValue;
use crate::model::RulesFile;
use crate::resolved_config::ResolvedConfig;

#[derive(Debug)]
pub enum TransactionError {
    /// Source changed externally since the caller loaded it — never a silent
    /// clobber (§4.6 step 4). → CLI exit 3.
    ExternalConflict(String),
    /// Bytes match neither the before- nor after-image during recovery —
    /// manual resolution, journal kept (§4.6 filesystem safety). → exit 4.
    RecoveryConflict(Vec<RecoveryConflict>),
    /// I/O failure: permissions, disk, unreadable journal. → exit 4.
    Io(String),
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionError::ExternalConflict(message) | TransactionError::Io(message) => {
                write!(f, "{message}")
            }
            TransactionError::RecoveryConflict(conflicts) => {
                for conflict in conflicts {
                    write!(f, "tx {}: {}; ", conflict.txid, conflict.message)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for TransactionError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryConflict {
    pub txid: String,
    pub path: String,
    pub message: String,
}

/// One complete file replacement inside a transaction. `after == None` removes
/// the file — and the removal is itself a recoverable rename.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Plan-relative, a validated document role.
    pub path: String,
    /// Bytes as the caller loaded them; `None` = the file was absent.
    pub before: Option<Vec<u8>>,
    /// Complete new content; `None` = remove.
    pub after: Option<Vec<u8>>,
}

impl FileChange {
    pub fn new(path: impl Into<String>, before: Option<Vec<u8>>, after: Option<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            before,
            after,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitResult {
    /// Empty for a no-op commit.
    pub txid: String,
    pub base_revision: String,
    pub new_revision: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    pub rolled_back: Vec<String>,
    pub finished: Vec<String>,
}

/// A retained before-image set plus its manifest (§4.6 step 4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub schema_version: u32,
    pub txid: String,
    pub date: String,
    pub summary: String,
    pub base_revision: String,
    pub new_revision: Option<String>,
    pub entries: Vec<BackupEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub path: String,
    pub existed: bool,
    /// FNV of the before-image — absent when the transaction created the file.
    pub hash: Option<String>,
    pub bytes: u64,
    /// Name inside the backup directory — absent when the transaction created
    /// the file.
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalDoc {
    schema_version: u32,
    txid: String,
    /// `"prepared"` | `"committed"`.
    state: String,
    base_revision: String,
    new_revision: Option<String>,
    summary: String,
    entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalEntry {
    path: String,
    existed: bool,
    before_hash: Option<String>,
    /// `None` = this transaction removes the file.
    after_hash: Option<String>,
    /// Plan-relative before-image path.
    backup: Option<String>,
    /// Plan-relative staging slot.
    staged: String,
}

/// The test seam (§9 rule 1): kill the process at a named boundary so the
/// self-check can prove recovery from process death at every step. Never set in
/// normal operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    Staged,
    Prepared,
    MidRenames,
    Renamed,
    Committed,
}

/// `SAM_TX_CRASH_POINT`, the only environment variable that changes engine
/// behaviour, and only to die.
pub const CRASH_ENV: &str = "SAM_TX_CRASH_POINT";

impl CrashPoint {
    pub const ALL: [CrashPoint; 5] = [
        CrashPoint::Staged,
        CrashPoint::Prepared,
        CrashPoint::MidRenames,
        CrashPoint::Renamed,
        CrashPoint::Committed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CrashPoint::Staged => "staged",
            CrashPoint::Prepared => "prepared",
            CrashPoint::MidRenames => "mid-renames",
            CrashPoint::Renamed => "renamed",
            CrashPoint::Committed => "committed",
        }
    }

    fn die_if_requested(self) {
        if std::env::var(CRASH_ENV).as_deref() == Ok(self.name()) {
            sigkill_self();
        }
    }
}

#[cfg(unix)]
fn sigkill_self() -> ! {
    // The C library is already linked; the signature is all this needs, so the
    // seam costs no dependency.
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    unsafe { kill(std::process::id() as i32, 9) };
    std::process::abort()
}

#[cfg(not(unix))]
fn sigkill_self() -> ! {
    // Windows has no SIGKILL; `abort()` is the closest a process can do to
    // itself. The crash-boundary gate records which status it observed.
    std::process::abort()
}

/// The shipped default (§4.6 step 4), written into `.sam/settings.json` so it
/// is visible rather than implicit.
pub const DEFAULT_RETENTION: usize = 20;

/// Plan-relative bookkeeping path.
fn sam(root: &Path, rel: &str) -> PathBuf {
    root.join(".sam").join(rel)
}

fn io(message: impl Into<String>) -> TransactionError {
    TransactionError::Io(message.into())
}

/// Create the `.sam/` skeleton (idempotent) with owner-only permissions.
pub fn ensure_layout(root: &Path) -> Result<(), TransactionError> {
    for sub in ["tx/journal", "tx/staging", "backups"] {
        let dir = sam(root, sub);
        std::fs::create_dir_all(&dir)
            .map_err(|error| io(format!("cannot create .sam/{sub}: {error}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
    }
    let lock = sam(root, "lock");
    if !lock.exists() {
        create_private(&lock, b"[]")?;
    }
    let settings = sam(root, "settings.json");
    if !settings.exists() {
        let data = serde_json::to_vec(&serde_json::json!({ "backupRetention": DEFAULT_RETENTION }))
            .map_err(|error| io(error.to_string()))?;
        create_private(&settings, &data)?;
    }
    Ok(())
}

/// Retention is visible and configurable (§4.6 step 4): read
/// `.sam/settings.json {backupRetention}`; fall back to the default with a
/// warning rather than failing a write over bookkeeping.
pub fn retention(root: &Path) -> (usize, Option<String>) {
    let rel = ".sam/settings.json";
    let fallback = || {
        (
            DEFAULT_RETENTION,
            Some(format!(
                "{rel}: backupRetention missing or out of range 0–200 — using {DEFAULT_RETENTION}"
            )),
        )
    };
    let Ok(bytes) = std::fs::read(sam(root, "settings.json")) else {
        return fallback();
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return fallback();
    };
    let Ok(value) = crate::strict_json::parse(text, rel) else {
        return fallback();
    };
    match value.get("backupRetention").and_then(JSONValue::as_i64) {
        Some(number) if (0..=200).contains(&number) => (number as usize, None),
        _ => fallback(),
    }
}

// ── commit (§4.6 steps 2–4) ─────────────────────────────────────────────────

pub fn commit(
    root: &Path,
    changes: &[FileChange],
    base_revision: &str,
    new_revision: &str,
    summary: &str,
) -> Result<CommitResult, TransactionError> {
    debug_assert!(
        !changes.is_empty(),
        "a no-op batch never reaches the engine"
    );
    for change in changes {
        validate_relative(&change.path)?;
    }
    ensure_layout(root)?;

    // 1 — the source must still be what the caller loaded. An external edit
    // during the draft produces a conflict, never a clobber.
    for change in changes {
        let current = std::fs::read(root.join(&change.path)).ok();
        if current != change.before {
            return Err(TransactionError::ExternalConflict(format!(
                "{} changed externally since it was loaded — refusing to overwrite; re-run SAM paths and retry with a fresh --if-revision",
                change.path
            )));
        }
    }

    let txid = make_txid();
    let staging_dir = sam(root, &format!("tx/staging/{txid}"));
    let backup_dir = sam(root, &format!("backups/{txid}"));
    create_dir_private(&staging_dir)?;
    create_dir_private(&backup_dir)?;

    // 2 — stage complete affected files on the same filesystem.
    let mut entries: Vec<JournalEntry> = Vec::new();
    for (index, change) in changes.iter().enumerate() {
        let staged_rel = format!(".sam/tx/staging/{txid}/{index}");
        let backup_rel = change
            .before
            .as_ref()
            .map(|_| format!(".sam/backups/{txid}/{index}"));
        if let Some(after) = &change.after {
            create_private(&sam(root, &format!("tx/staging/{txid}/{index}")), after)?;
        }
        if let Some(before) = &change.before {
            create_private(&sam(root, &format!("backups/{txid}/{index}")), before)?;
        }
        entries.push(JournalEntry {
            path: change.path.clone(),
            existed: change.before.is_some(),
            before_hash: change.before.as_deref().map(fnv_hex),
            after_hash: change.after.as_deref().map(fnv_hex),
            backup: backup_rel,
            staged: staged_rel,
        });
    }
    CrashPoint::Staged.die_if_requested();

    // 3 — journal(prepared), flushed before any source is touched.
    let journal_url = sam(root, &format!("tx/journal/{txid}.json"));
    let journal_data = |state: &str| -> Result<Vec<u8>, TransactionError> {
        let doc = JournalDoc {
            schema_version: 1,
            txid: txid.clone(),
            state: state.to_string(),
            base_revision: base_revision.to_string(),
            new_revision: Some(new_revision.to_string()),
            summary: summary.to_string(),
            entries: entries.clone(),
        };
        canonical_bytes(&doc)
    };
    create_private(&journal_url, &journal_data("prepared")?)?;
    CrashPoint::Prepared.die_if_requested();

    // 4 — replace files one rename at a time. A removal is itself a rename into
    // staging, so it is exactly as recoverable.
    for (index, change) in changes.iter().enumerate() {
        let target = root.join(&change.path);
        let staged = root.join(&entries[index].staged);
        if change.after.is_none() {
            rename_file(&target, &staged)?;
        } else {
            // A document SAM creates — `state/state.json` on the first stage
            // transition, a new type's records file — may live under a directory
            // that does not exist yet. The staging rename cannot create it, so
            // the directory is created before the replacement; an empty
            // directory is not journal state, and a rollback leaves it inert.
            if let Some(parent) = target.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)
                    .map_err(|error| io(format!("cannot create {} ({error})", parent.display())))?;
            }
            rename_file(&staged, &target)?;
        }
        if changes.len() > 1 && index == 0 {
            CrashPoint::MidRenames.die_if_requested();
        }
    }
    CrashPoint::Renamed.die_if_requested();

    // 5 — committed marker, crash-safe (temp + rename).
    let tmp = PathBuf::from(format!("{}.tmp", journal_url.display()));
    create_private(&tmp, &journal_data("committed")?)?;
    rename_file(&tmp, &journal_url)?;
    CrashPoint::Committed.die_if_requested();

    // 6 — finalize: manifest, cleanup, retention.
    let manifest = BackupManifest {
        schema_version: 1,
        txid: txid.clone(),
        date: iso_now(),
        summary: summary.to_string(),
        base_revision: base_revision.to_string(),
        new_revision: Some(new_revision.to_string()),
        entries: changes
            .iter()
            .enumerate()
            .map(|(index, change)| BackupEntry {
                path: change.path.clone(),
                existed: change.before.is_some(),
                hash: change.before.as_deref().map(fnv_hex),
                bytes: change
                    .before
                    .as_ref()
                    .map(|bytes| bytes.len() as u64)
                    .unwrap_or(0),
                file: change.before.as_ref().map(|_| index.to_string()),
            })
            .collect(),
    };
    if let Ok(data) = canonical_bytes(&manifest) {
        let _ = create_private(&backup_dir.join("manifest.json"), &data);
    }
    let _ = std::fs::remove_file(&journal_url);
    let _ = std::fs::remove_dir_all(&staging_dir);
    prune_backups(root);

    Ok(CommitResult {
        txid,
        base_revision: base_revision.to_string(),
        new_revision: new_revision.to_string(),
        files: changes.iter().map(|change| change.path.clone()).collect(),
    })
}

// ── startup recovery (§4.6 step 3) ──────────────────────────────────────────

/// Roll back every prepared transaction, finish every committed one, sweep
/// orphaned staging. A journal whose bytes match neither state is a manual
/// conflict: keep it and report, never blind-rollback.
pub fn recover(root: &Path) -> Result<RecoveryReport, TransactionError> {
    if !root.join(".sam").exists() {
        return Ok(RecoveryReport::default());
    }

    let journal_dir = sam(root, "tx/journal");
    let mut journal_names: Vec<String> = std::fs::read_dir(&journal_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .filter(|name| name.ends_with(".json"))
                .collect()
        })
        .unwrap_or_default();
    journal_names.sort();

    // Orphaned staging (death between staging and journaling) and the
    // marker-rewrite temporaries are garbage — sweep them.
    let live: BTreeSet<String> = journal_names
        .iter()
        .map(|name| name.trim_end_matches(".json").to_string())
        .collect();
    let staging_root = sam(root, "tx/staging");
    for name in std::fs::read_dir(&staging_root)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
    {
        let id = name.trim_end_matches(".tmp").to_string();
        if !live.contains(&id) {
            let _ = std::fs::remove_dir_all(staging_root.join(&name));
            let _ = std::fs::remove_file(staging_root.join(&name));
        }
    }

    let mut report = RecoveryReport::default();
    for name in &journal_names {
        let txid = name.trim_end_matches(".json").to_string();
        let url = journal_dir.join(name);
        let bytes = std::fs::read(&url)
            .map_err(|error| io(format!("cannot read journal {name}: {error}")))?;
        let journal: JournalDoc = serde_json::from_slice(&bytes).map_err(|_| {
            io(format!(
                "journal {name} is unreadable — resolve manually in .sam/tx/journal/"
            ))
        })?;
        match journal.state.as_str() {
            "prepared" => {
                rollback(root, &journal)?;
                let _ = std::fs::remove_dir_all(sam(root, &format!("backups/{txid}")));
                let _ = std::fs::remove_dir_all(sam(root, &format!("tx/staging/{txid}")));
                let _ = std::fs::remove_file(&url);
                report.rolled_back.push(txid);
            }
            "committed" => {
                finish_commit(root, &journal)?;
                let _ = std::fs::remove_dir_all(sam(root, &format!("tx/staging/{txid}")));
                let _ = std::fs::remove_file(&url);
                report.finished.push(txid);
            }
            other => {
                return Err(io(format!("journal {name} has unknown state \"{other}\"")));
            }
        }
    }
    Ok(report)
}

fn rollback(root: &Path, journal: &JournalDoc) -> Result<(), TransactionError> {
    for entry in &journal.entries {
        let path_url = root.join(&entry.path);
        let current = std::fs::read(&path_url).ok();
        let current_hash = current.as_deref().map(fnv_hex);
        if entry.after_hash.is_some() && current_hash == entry.after_hash {
            // Our write landed — put the before-image back.
            match &entry.backup {
                Some(backup_rel) => {
                    let before = std::fs::read(root.join(backup_rel))
                        .map_err(|error| io(format!("backup image unreadable: {error}")))?;
                    if Some(fnv_hex(&before)) != entry.before_hash {
                        return Err(io(format!("backup image corrupt for {}", entry.path)));
                    }
                    create_private(&path_url, &before)?;
                }
                None => {
                    // The transaction created it.
                    let _ = std::fs::remove_file(&path_url);
                }
            }
        } else if entry.after_hash.is_none() && current.is_none() {
            // Our removal landed — the before-bytes wait in staging.
            let staged_url = root.join(&entry.staged);
            let staged = std::fs::read(&staged_url).ok();
            if staged.as_deref().map(fnv_hex) != entry.before_hash {
                return Err(conflict(
                    journal,
                    entry,
                    current_hash,
                    "removed file's bytes are unrecoverable",
                ));
            }
            rename_file(&staged_url, &path_url)?;
        } else if current_hash == entry.before_hash || (current.is_none() && !entry.existed) {
            // Never landed — nothing to do.
        } else {
            return Err(conflict(
                journal,
                entry,
                current_hash,
                "bytes match neither before- nor after-image",
            ));
        }
    }
    Ok(())
}

fn finish_commit(root: &Path, journal: &JournalDoc) -> Result<(), TransactionError> {
    for entry in &journal.entries {
        let path_url = root.join(&entry.path);
        let current = std::fs::read(&path_url).ok();
        let current_hash = current.as_deref().map(fnv_hex);
        match &entry.after_hash {
            Some(after_hash) if current_hash.as_ref() == Some(after_hash) => continue,
            Some(after_hash) => {
                // A rename should not be missing after the marker, but finishing
                // is safe and idempotent when staging still holds the
                // after-image and the source is still the before-image.
                let staged_url = root.join(&entry.staged);
                let staged = std::fs::read(&staged_url).ok();
                let staged_matches = staged.as_deref().map(fnv_hex).as_ref() == Some(after_hash);
                let source_intact =
                    current_hash == entry.before_hash || (current.is_none() && !entry.existed);
                if staged_matches && source_intact {
                    rename_file(&staged_url, &path_url)?;
                } else {
                    return Err(conflict(
                        journal,
                        entry,
                        current_hash,
                        "bytes match neither before- nor after-image",
                    ));
                }
            }
            None => {
                // Committed removal: the file must be absent.
                if current.is_none() {
                    continue;
                }
                if current_hash == entry.before_hash {
                    if let Some(parent) = Path::new(&entry.staged).parent() {
                        let _ = create_dir_private(&root.join(parent));
                    }
                    rename_file(&path_url, &root.join(&entry.staged))?;
                } else {
                    return Err(conflict(
                        journal,
                        entry,
                        current_hash,
                        "removed file was externally rewritten",
                    ));
                }
            }
        }
    }

    // The finalize step may not have run before death — write the manifest if
    // missing so the backup stays restorable.
    let manifest_url = sam(root, &format!("backups/{}/manifest.json", journal.txid));
    if !manifest_url.exists() {
        let manifest = BackupManifest {
            schema_version: 1,
            txid: journal.txid.clone(),
            date: iso_now(),
            summary: format!("{} (recovered)", journal.summary),
            base_revision: journal.base_revision.clone(),
            new_revision: journal.new_revision.clone(),
            entries: journal
                .entries
                .iter()
                .map(|entry| BackupEntry {
                    path: entry.path.clone(),
                    existed: entry.existed,
                    hash: entry.before_hash.clone(),
                    bytes: 0,
                    file: entry.backup.as_ref().map(|_| "0".to_string()),
                })
                .collect(),
        };
        if let Ok(data) = canonical_bytes(&manifest) {
            let _ = create_private(&manifest_url, &data);
        }
    }
    Ok(())
}

fn conflict(
    journal: &JournalDoc,
    entry: &JournalEntry,
    found: Option<String>,
    why: &str,
) -> TransactionError {
    TransactionError::RecoveryConflict(vec![RecoveryConflict {
        txid: journal.txid.clone(),
        path: entry.path.clone(),
        message: format!(
            "{why} — expected before {} / after {}, found {}; resolve manually, journal kept in .sam/tx/journal/",
            entry.before_hash.clone().unwrap_or_else(|| "absent".into()),
            entry.after_hash.clone().unwrap_or_else(|| "removed".into()),
            found.unwrap_or_else(|| "absent".into())
        ),
    }])
}

// ── backups (§4.6 step 4) ───────────────────────────────────────────────────

pub fn list_backups(root: &Path) -> (Vec<BackupManifest>, usize, Option<String>) {
    let (keep, warning) = retention(root);
    let mut names: Vec<String> = std::fs::read_dir(sam(root, "backups"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let mut manifests = Vec::new();
    for name in names.iter().rev() {
        let url = sam(root, &format!("backups/{name}/manifest.json"));
        if let Ok(bytes) = std::fs::read(&url) {
            if let Ok(manifest) = serde_json::from_slice::<BackupManifest>(&bytes) {
                manifests.push(manifest);
            }
        }
    }
    (manifests, keep, warning)
}

/// txids are time-ordered strings, so a name sort is chronological.
fn prune_backups(root: &Path) {
    let (keep, _) = retention(root);
    let mut names: Vec<String> = std::fs::read_dir(sam(root, "backups"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    for name in names.iter().rev().skip(keep) {
        let _ = std::fs::remove_dir_all(sam(root, &format!("backups/{name}")));
    }
}

/// The file changes that return the plan to one transaction's before-state.
/// Planning writes nothing (§4.6: restoration is a previewed command).
pub struct RestorePlan {
    pub txid: String,
    pub changes: Vec<FileChange>,
    pub notes: Vec<String>,
}

pub fn restore_plan(root: &Path, txid: &str) -> Result<RestorePlan, TransactionError> {
    // `make_txid` emits exactly `{13 millis digits}-{8 hex}`; anything else is
    // not a transaction id but an attempt to retarget the backup path (the
    // read side has no validate_relative backstop, so the shape check is it).
    let valid = txid.len() == 22
        && txid.as_bytes()[13] == b'-'
        && txid[..13].bytes().all(|b| b.is_ascii_digit())
        && txid[14..].bytes().all(|b| b.is_ascii_hexdigit());
    if !valid {
        return Err(io(format!("not a transaction id: {txid}")));
    }
    let dir = sam(root, &format!("backups/{txid}"));
    let bytes = std::fs::read(dir.join("manifest.json"))
        .map_err(|_| io(format!("no retained backup named {txid} — see SAM tx.list")))?;
    let manifest: BackupManifest = serde_json::from_slice(&bytes)
        .map_err(|error| io(format!("backup {txid} manifest is unreadable: {error}")))?;

    let mut changes = Vec::new();
    let mut notes = Vec::new();
    for entry in &manifest.entries {
        let current = std::fs::read(root.join(&entry.path)).ok();
        match &entry.file {
            Some(file) => {
                let before = std::fs::read(dir.join(file))
                    .map_err(|error| io(format!("backup image unreadable: {error}")))?;
                if current.as_deref() == Some(before.as_slice()) {
                    continue; // already at the before-state
                }
                notes.push(format!(
                    "{}: {} → {} bytes",
                    entry.path,
                    current.as_ref().map(|bytes| bytes.len()).unwrap_or(0),
                    before.len()
                ));
                changes.push(FileChange::new(entry.path.clone(), current, Some(before)));
            }
            None => {
                if current.is_none() {
                    continue;
                }
                notes.push(format!(
                    "{}: remove (file was created by {txid})",
                    entry.path
                ));
                changes.push(FileChange::new(entry.path.clone(), current, None));
            }
        }
    }
    Ok(RestorePlan {
        txid: txid.to_string(),
        changes,
        notes,
    })
}

/// Why a whole-plan overlay can fail: the bytes do not load, or they load and
/// the plan is invalid. §4.9 keeps the two exit codes distinct.
#[derive(Debug)]
pub enum OverlayError {
    Io(String),
    Invalid(Vec<Diagnostic>),
}

impl std::fmt::Display for OverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverlayError::Io(message) => write!(f, "{message}"),
            OverlayError::Invalid(diagnostics) => {
                for diagnostic in diagnostics {
                    writeln!(f, "{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

/// Whole-plan validation for arbitrary byte-level changes (restore): apply them
/// to a throwaway copy and load it through the real loader.
pub fn validate_by_overlay(
    root: &Path,
    changes: &[FileChange],
    resources_dir: &Path,
) -> Result<ResolvedConfig, OverlayError> {
    let tmp = std::env::temp_dir().join(format!("sam-overlay-{}", make_txid()));
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&tmp);
    };
    std::fs::create_dir_all(&tmp).map_err(|error| OverlayError::Io(error.to_string()))?;
    for rel in ["content", "state"] {
        let src = root.join(rel);
        if src.exists() {
            if let Err(error) = copy_tree(&src, &tmp.join(rel)) {
                cleanup();
                return Err(OverlayError::Io(error.to_string()));
            }
        }
    }
    for change in changes {
        let dst = tmp.join(&change.path);
        if let Some(parent) = dst.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                cleanup();
                return Err(OverlayError::Io(error.to_string()));
            }
        }
        match &change.after {
            Some(after) => {
                if let Err(error) = std::fs::write(&dst, after) {
                    cleanup();
                    return Err(OverlayError::Io(error.to_string()));
                }
            }
            None => {
                let _ = std::fs::remove_file(&dst);
            }
        }
    }
    let loaded = crate::config_store::load_with(&tmp, "explicit", resources_dir);
    cleanup();
    match loaded {
        Ok(config) => Ok(config),
        Err(crate::config_store::ConfigStoreError::Validation(diagnostics)) => {
            Err(OverlayError::Invalid(diagnostics))
        }
        Err(crate::config_store::ConfigStoreError::Io(message)) => Err(OverlayError::Io(message)),
    }
}

/// Recompute what the revision will be after these changes, so a commit's
/// `newRevision` is the value a later load publishes.
pub fn revision_after(base: &BTreeMap<String, String>, changes: &[FileChange]) -> String {
    let mut hashes = base.clone();
    for change in changes {
        match &change.after {
            Some(after) => {
                hashes.insert(change.path.clone(), fnv_hex(after));
            }
            None => {
                hashes.remove(&change.path);
            }
        }
    }
    combined_revision(&hashes)
}

// ── small filesystem helpers ────────────────────────────────────────────────

pub fn validate_relative(path: &str) -> Result<(), TransactionError> {
    // Both separators: a backslash is a filename character on unix but a
    // real separator on Windows, and this crate ships there too.
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.split(['/', '\\']).any(|segment| segment == "..")
    {
        return Err(io(format!("unsafe plan-relative path: {path}")));
    }
    Ok(())
}

/// Write bytes with owner-only permissions, then flush them to the device.
/// Durability stops here: the journal is `fsync`ed, the containing directory is
/// not, and no power-loss guarantee is claimed (§4.6 step 3).
pub fn create_private(path: &Path, data: &[u8]) -> Result<(), TransactionError> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| io(format!("cannot write {} ({error})", path.display())))?;
    file.write_all(data)
        .map_err(|error| io(format!("cannot write {} ({error})", path.display())))?;
    file.sync_all()
        .map_err(|error| io(format!("cannot flush {} ({error})", path.display())))?;
    Ok(())
}

fn create_dir_private(path: &Path) -> Result<(), TransactionError> {
    std::fs::create_dir_all(path)
        .map_err(|error| io(format!("cannot create {} ({error})", path.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn rename_file(from: &Path, to: &Path) -> Result<(), TransactionError> {
    std::fs::rename(from, to).map_err(|error| {
        io(format!(
            "rename {} → {} failed ({error})",
            from.display(),
            to.display()
        ))
    })
}

fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, TransactionError> {
    // `serde_json`'s map is key-sorted, so this is §4.1's canonical form.
    let value = serde_json::to_value(value).map_err(|error| io(error.to_string()))?;
    serde_json::to_vec(&value).map_err(|error| io(error.to_string()))
}

static TXID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Time-ordered prefix so a name sort is chronological, plus entropy from the
/// process so two writers in one millisecond cannot collide.
pub fn make_txid() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0);
    let mut seed = Vec::new();
    seed.extend_from_slice(&millis.to_le_bytes());
    seed.extend_from_slice(&std::process::id().to_le_bytes());
    seed.extend_from_slice(&TXID_COUNTER.fetch_add(1, Ordering::SeqCst).to_le_bytes());
    seed.extend_from_slice(
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.subsec_nanos())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    let hash = fnv_hex(&seed);
    format!("{millis:013}-{}", &hash[..8])
}

/// ISO-8601 UTC, hand-rolled: `std` has no formatter and §9 permits no date
/// crate, so this is Hinnant's `civil_from_days` and a clock division.
pub fn iso_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    iso_from_unix(seconds)
}

fn iso_from_unix(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let remaining = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        remaining / 3600,
        (remaining % 3600) / 60,
        remaining % 60
    )
}

/// Parse an ISO-8601 instant to Unix seconds: `YYYY-MM-DDTHH:MM[:SS]` with a
/// trailing `Z` or a `±HH:MM` offset (absent = UTC), or a bare `YYYY-MM-DD`
/// (midnight UTC). Hand-rolled for the same reason [`iso_now`] is: §9 permits
/// no date crate. `[INFERENCE]`-free by construction — the round trip against
/// [`iso_now`] is a test.
pub fn parse_iso(text: &str) -> Option<i64> {
    let text = text.trim();
    let (date, rest) = if text.len() > 10 {
        (&text[..10], &text[10..])
    } else {
        (text, "")
    };
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut seconds = days_from_civil(year, month, day) * 86_400;
    if rest.is_empty() {
        return Some(seconds);
    }
    let rest = rest.strip_prefix('T').or_else(|| rest.strip_prefix(' '))?;
    // The offset is a suffix on the time; `Z` and absent are UTC.
    let (time, offset_seconds) = if let Some(stripped) = rest.strip_suffix('Z') {
        (stripped, 0)
    } else if let Some(index) = rest.rfind(['+', '-']) {
        let (time, offset) = rest.split_at(index);
        let sign = if offset.starts_with('+') { 1 } else { -1 };
        let offset = &offset[1..];
        let mut parts = offset.split(':');
        let hours: i64 = parts.next()?.parse().ok()?;
        let minutes: i64 = parts.next().unwrap_or("0").parse().ok()?;
        (time, sign * (hours * 3600 + minutes * 60))
    } else {
        (rest, 0)
    };
    let mut parts = time.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next().unwrap_or("0").parse().ok()?;
    let seconds_part: i64 = parts
        .next()
        .map(|value| value.split('.').next().unwrap_or("0"))
        .unwrap_or("0")
        .parse()
        .ok()?;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }
    seconds += hours * 3600 + minutes * 60 + seconds_part;
    Some(seconds - offset_seconds)
}

/// Days since 1970-01-01 → (year, month, day). Public-domain algorithm
/// (Howard Hinnant, `chrono`-compatible days-from-civil inverse).
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// (year, month, day) → days since 1970-01-01, the inverse above. Same
/// provenance and the same reason: §9 permits no date crate before Phase 6.
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Recursive copy, for staging a preset and for the restore overlay.
pub fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Load a plan through the engine, for callers that already hold the lock.
pub fn load_locked(root: &Path, resources_dir: &Path) -> Result<ResolvedConfig, TransactionError> {
    crate::config_store::load_with(root, "explicit", resources_dir).map_err(|error| match error {
        crate::config_store::ConfigStoreError::Io(message) => io(message),
        crate::config_store::ConfigStoreError::Validation(diagnostics) => TransactionError::Io(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        ),
    })
}

/// The plan's authoritative rules, for callers that need them without a load.
pub type Rules = RulesFile;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_dates_are_correct_at_the_known_boundaries() {
        assert_eq!(iso_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_from_unix(1_767_225_600), "2026-01-01T00:00:00Z");
        // A leap day, and the day after.
        assert_eq!(iso_from_unix(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(iso_from_unix(1_709_251_200), "2024-03-01T00:00:00Z");
    }

    #[test]
    fn a_txid_sorts_chronologically_and_is_unique_in_one_millisecond() {
        let a = make_txid();
        let b = make_txid();
        assert_eq!(a.len(), 22, "13 digit millis, dash, 8 hex");
        assert_ne!(a, b, "two writers in one millisecond cannot collide");
        let mut ids = vec![a, b];
        let c = make_txid();
        ids.push(c);
        let mut sorted = ids.clone();
        sorted.sort();
        assert!(sorted[0] <= sorted[1]);
    }

    #[test]
    fn unsafe_relative_paths_are_refused() {
        assert!(validate_relative("content/records/topic.jsonl").is_ok());
        assert!(validate_relative("").is_err());
        assert!(validate_relative("/etc/passwd").is_err());
        assert!(validate_relative("content/../.sam/lock").is_err());
        // A backslash is a filename character on unix but a real separator
        // on Windows, where this crate also ships.
        assert!(validate_relative("content\\..\\..\\pwned.jsonl").is_err());
        assert!(validate_relative("\\\\server\\share").is_err());
    }

    #[test]
    fn a_hostile_txid_cannot_retarget_the_backup_read() {
        let root = std::env::temp_dir();
        for hostile in ["../../secrets", "0000000000000-ffffffff\\x", "a/b"] {
            let refused = match restore_plan(&root, hostile) {
                Err(error) if error.to_string().contains("not a transaction id") => true,
                _ => false,
            };
            assert!(
                refused,
                "the refusal names the rule, not the filesystem: {hostile}"
            );
        }
        // The shape make_txid emits is still accepted (the backup it names
        // may not exist, but that is the documented "no retained backup"
        // error, not a shape refusal).
        let missing = match restore_plan(&root, "0000000000000-00000000") {
            Err(error) if error.to_string().contains("no retained backup") => true,
            _ => false,
        };
        assert!(
            missing,
            "a well-formed txid passes the shape check and fails only on the missing backup"
        );
    }

    #[test]
    fn iso_now_looks_like_an_instant() {
        let now = iso_now();
        assert_eq!(now.len(), 20);
        assert!(now.ends_with('Z'));
    }
}
