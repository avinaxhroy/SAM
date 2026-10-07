//! `CommandSession` (§4.2 CommandDispatch, §4.9 D15): unified dispatcher over the registry.
//!
//! Invocation pipeline for the CLI, UI, and test fixtures. Reads acquire the read
//! lock; writes acquire the write lock, run journal recovery, verify `--if-revision`,
//! and commit through [`crate::transaction`] (§4.8 Principle 2, Phase 2, Phase 4).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde_json::json;

use crate::apply::{self, ApplyError};
use crate::command_registry::{self, CommandDef, CommandEffect};
use crate::config_store::{self, ConfigStoreError, PositionedRecord};
use crate::diagnostic::Diagnostic;
use crate::doc_edit;
use crate::plan_lock::{PlanLock, PlanLockError, READ_RETRY, WRITE_RETRY};
use crate::resolved_config::ResolvedConfig;
use crate::transaction::{self, CommitResult, FileChange, OverlayError, TransactionError};

/// One parameter value. `Bytes` is how a piped batch reaches the dispatcher
/// without the dispatcher ever touching stdin.
#[derive(Debug, Clone)]
pub enum ParamValue {
    Str(String),
    Bool(bool),
    Bytes(Vec<u8>),
    /// A repeated flag's values, in order (`--field a:text --field b:number`).
    Strings(Vec<String>),
}

pub type Params = BTreeMap<String, ParamValue>;

pub fn flag(params: &Params, name: &str) -> bool {
    matches!(params.get(name), Some(ParamValue::Bool(true)))
}

pub fn optional(params: &Params, name: &str) -> Option<String> {
    match params.get(name) {
        Some(ParamValue::Str(value)) => Some(value.clone()),
        _ => None,
    }
}

pub fn require(params: &Params, name: &str) -> Result<String, DispatchError> {
    optional(params, name)
        .ok_or_else(|| DispatchError::Usage(format!("missing required parameter \"{name}\"")))
}

// The dispatcher's whole error vocabulary, so the CLI maps exit codes once.
#[derive(Debug)]
pub enum DispatchError {
    Usage(String),
    Apply(ApplyError),
    Transaction(TransactionError),
    Lock(PlanLockError),
    Invalid(Vec<Diagnostic>),
    Io(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::Usage(message) | DispatchError::Io(message) => write!(f, "{message}"),
            DispatchError::Apply(error) => write!(f, "{error}"),
            DispatchError::Transaction(error) => write!(f, "{error}"),
            DispatchError::Lock(error) => write!(f, "{error}"),
            DispatchError::Invalid(diagnostics) => {
                for diagnostic in diagnostics {
                    writeln!(f, "{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DispatchError {}

impl From<PlanLockError> for DispatchError {
    fn from(error: PlanLockError) -> Self {
        DispatchError::Lock(error)
    }
}

impl From<TransactionError> for DispatchError {
    fn from(error: TransactionError) -> Self {
        DispatchError::Transaction(error)
    }
}

impl From<ApplyError> for DispatchError {
    fn from(error: ApplyError) -> Self {
        DispatchError::Apply(error)
    }
}

impl From<OverlayError> for DispatchError {
    fn from(error: OverlayError) -> Self {
        match error {
            OverlayError::Io(message) => DispatchError::Io(message),
            OverlayError::Invalid(diagnostics) => DispatchError::Invalid(diagnostics),
        }
    }
}

impl From<ConfigStoreError> for DispatchError {
    fn from(error: ConfigStoreError) -> Self {
        match error {
            ConfigStoreError::Io(message) => DispatchError::Io(message),
            ConfigStoreError::Validation(diagnostics) => DispatchError::Invalid(diagnostics),
        }
    }
}

pub struct DispatchRequest {
    pub id: String,
    pub params: Params,
    pub plan: Option<PathBuf>,
    pub if_revision: Option<String>,
    pub resources_dir: PathBuf,
}

impl DispatchRequest {
    pub fn new(id: impl Into<String>, resources_dir: PathBuf) -> Self {
        Self {
            id: id.into(),
            params: Params::new(),
            plan: None,
            if_revision: None,
            resources_dir,
        }
    }
}

pub enum DispatchOutcome {
    Read(serde_json::Value),
    /// A committed — or dry-run — write. `changes` carries the exact file
    /// changes the UI undo adapter registers the inverse of.
    Write {
        commit: CommitResult,
        changes: Vec<FileChange>,
        data: serde_json::Value,
        summary: String,
        dry_run: bool,
    },
    /// A transient action (§4.9): a destination where one exists, else an
    /// explicit headless-unavailable result.
    Presentation {
        destination: String,
    },
}

/// Destinations for presentation commands (§4.9: headless commands return the
/// destination when no GUI is available). Phase 4 fills this in with the
/// `app.*` window actions; the arm exists so a presentation id can never fall
/// through into the write path.
static DESTINATIONS: LazyLock<BTreeMap<&'static str, &'static str>> = LazyLock::new(|| {
    BTreeMap::from([
        ("app.palette", "the command palette"),
        (
            "app.changePlan",
            "the plans on this machine — the folder the app reads from",
        ),
        ("app.openSettings", "Settings"),
        (
            "app.gettingStarted",
            "getting started — the first-run story, reopenable",
        ),
        (
            "app.openSystem",
            "the System surface — the machine's own place",
        ),
        ("screen.edit", "the screen editor, in the app"),
        ("app.toggleSourcePane", "the source pane"),
        ("rail.select", "the selected sidebar destination"),
        (
            "record.reveal",
            "the source pane, at the record the parameters name",
        ),
        (
            "record.panel",
            "the detail panel, at the record the parameters name",
        ),
        (
            "view.edit",
            "the view editor, at the view the parameters name",
        ),
    ])
});

/// A held read session: the lock plus recovery. Readers publish one snapshot
/// only after commit (§4.6 step 5); on a plan root without `.sam/` this is a
/// no-op and behaves exactly as Phase 1.
pub struct ReadSession {
    _lock: PlanLock,
}

pub fn open_read(root: &Path) -> Result<ReadSession, DispatchError> {
    let lock = PlanLock::acquire_read(root, READ_RETRY)?;
    transaction::recover(root)?;
    Ok(ReadSession { _lock: lock })
}

pub struct CommandSession;

/// Resolves a command ID to its definition, matching static registry commands first
/// and falling back to dynamic per-type handlers (`<type>.new`, `<type>.paste`, §4.7 rule 2).
fn resolve_request(id: &str) -> Option<(&'static CommandDef, Vec<(String, String)>)> {
    if let Some(def) = command_registry::resolve(id) {
        // Map convenience verbs to generic settings keys (§6 Phase 4, Appendix C.7).
        let injected = match id {
            "study.setDailyTarget" => vec![("key".to_string(), "dailyTargetMin".to_string())],
            "study.setTimezone" => vec![("key".to_string(), "timezone".to_string())],
            _ => Vec::new(),
        };
        return Some((def, injected));
    }
    let (type_name, suffix) = id.rsplit_once('.')?;
    if type_name.is_empty() {
        return None;
    }
    let handler = match suffix {
        "new" => "record.new",
        "paste" => "record.paste",
        _ => return None,
    };
    command_registry::resolve(handler)
        .map(|def| (def, vec![("type".to_string(), type_name.to_string())]))
}

impl CommandSession {
    /// Run one request. Locking, recovery, revision checks and commit all
    /// happen here — callers cannot get them wrong (§4.6).
    pub fn run(mut request: DispatchRequest) -> Result<DispatchOutcome, DispatchError> {
        // Presentation commands never touch the plan.
        let Some((def, injected)) = resolve_request(&request.id) else {
            return Err(DispatchError::Usage(format!(
                "unknown command id \"{}\" — SAM --commands lists the registry",
                request.id
            )));
        };
        for (name, value) in injected {
            request.params.insert(name, ParamValue::Str(value));
        }
        if def.effect == CommandEffect::Presentation {
            return match DESTINATIONS.get(def.id.as_str()) {
                Some(destination) => Ok(DispatchOutcome::Presentation {
                    destination: (*destination).into(),
                }),
                None => Err(DispatchError::Usage(format!(
                    "no destination for {}",
                    def.id
                ))),
            };
        }

        // `plan.new` and `profile.import` have no plan root to lock: the staged
        // validation in `config_store` IS the transaction (§4.1 ownership), and
        // it must be reachable from every surface — the picker's button, the
        // palette, `SAM plan.new`, and `SAM invoke -`.
        if matches!(def.id.as_str(), "plan.new" | "profile.import") {
            return create_plan_outcome(&request, def.id.as_str());
        }

        // `plan.new` needs no root; everything else that requires one gets it.
        let root = if def.id == "plan.new" {
            None
        } else if def.plan_required || def.effect == CommandEffect::Write {
            Some(
                config_store::plan_root(request.plan.as_deref())
                    .map_err(DispatchError::from)?
                    .0,
            )
        } else {
            None
        };

        // Read commands: acquire lock, load snapshot, and evaluate projection.
        // Discovery commands (e.g. `commands`) resolve without an active plan if omitted.
        let root = if def.effect == CommandEffect::Read && root.is_none() && request.plan.is_some()
        {
            Some(
                config_store::plan_root(request.plan.as_deref())
                    .map_err(DispatchError::from)?
                    .0,
            )
        } else {
            root
        };
        if def.effect == CommandEffect::Read {
            let Some(root) = root else {
                return Ok(DispatchOutcome::Read(read_json(
                    &def.id,
                    &request.params,
                    None,
                    &request.resources_dir,
                    request.plan.as_deref(),
                )?));
            };
            let _session = open_read(&root)?;
            // The source pane's file read returns raw bytes, so it keeps the
            // full load; every other read only projects data (§4.6).
            let config = if def.id == "source" {
                config_store::load_with(&root, "explicit", &request.resources_dir)?
            } else {
                config_store::load_readonly(&root, "explicit", &request.resources_dir)?
            };
            return Ok(DispatchOutcome::Read(read_json(
                &def.id,
                &request.params,
                Some(&config),
                &request.resources_dir,
                Some(&root),
            )?));
        }

        // Writes under the write session.
        let root = root.ok_or_else(|| DispatchError::Usage(format!("{} needs a plan", def.id)))?;
        let lock = PlanLock::acquire_write(&root, WRITE_RETRY)?;
        transaction::recover(&root)?;
        let config = config_store::load_with(&root, "explicit", &request.resources_dir)?;
        if let Some(expected) = &request.if_revision {
            if expected != &config.revision {
                return Err(DispatchError::Transaction(
                    TransactionError::ExternalConflict(format!(
                        "plan revision is {} but --if-revision pins {expected} — re-read with SAM paths and re-plan",
                        config.revision
                    )),
                ));
            }
        }

        let dry_run = flag(&request.params, "dry-run");
        let outcome = match def.id.as_str() {
            "apply" => apply_batch(&root, &config, &request.params, dry_run)?,
            "tx.restore" => restore(&root, &config, &request, dry_run, "restore")?,
            "edit.undo" => restore(&root, &config, &request, dry_run, "undo")?,
            "edit.redo" => restore(&root, &config, &request, dry_run, "redo")?,
            "record.new" => {
                let planned = doc_edit::record_ops::new_record(&config, &request.params)?;
                batch_outcome(&root, planned, dry_run, "record.new")?
            }
            "record.paste" => {
                let planned = doc_edit::record_ops::paste_records(&config, &request.params)?;
                batch_outcome(&root, planned, dry_run, "record.paste")?
            }
            "record.setField" => {
                let id = require(&request.params, "id")?;
                let key = require(&request.params, "field")?;
                let value = optional(&request.params, "value");
                // A relation is an id LIST (§4.7: an id may contain a comma, so
                // a comma-joined string is the terminal's convenience, never the
                // model). The UI dispatches the array; the terminal's string
                // form still splits.
                let ids = match request.params.get("value") {
                    Some(ParamValue::Strings(values)) => Some(values.clone()),
                    _ => None,
                };
                let planned = doc_edit::record_ops::set_field(
                    &config,
                    &id,
                    &key,
                    value.as_deref(),
                    ids.as_deref(),
                )?;
                batch_outcome(&root, planned, dry_run, "record.setField")?
            }
            "record.defer" => {
                let edit =
                    doc_edit::record_ops::defer(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "record.delete" => {
                let id = require(&request.params, "id")?;
                let policy = optional(&request.params, "policy").unwrap_or_default();
                let edit = doc_edit::record_ops::delete(
                    &config,
                    &request.resources_dir,
                    &id,
                    &policy,
                    dry_run,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.new" => {
                let spec = require(&request.params, "spec")?;
                let edit = doc_edit::column_ops::new_column(
                    &config,
                    &request.resources_dir,
                    &spec,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.rename" => {
                let spec = require(&request.params, "spec")?;
                let label = optional(&request.params, "label");
                let edit = doc_edit::column_ops::rename(
                    &config,
                    &request.resources_dir,
                    &spec,
                    label.as_deref(),
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.retype" => {
                let spec = require(&request.params, "spec")?;
                let edit = doc_edit::column_ops::retype(
                    &config,
                    &request.resources_dir,
                    &spec,
                    &request.params,
                    dry_run,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.choices" => {
                let spec = require(&request.params, "spec")?;
                let options = require(&request.params, "options")?;
                let edit = doc_edit::column_ops::choices(
                    &config,
                    &request.resources_dir,
                    &spec,
                    &options,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.duplicate" => {
                let spec = require(&request.params, "spec")?;
                let as_key = optional(&request.params, "as");
                let edit = doc_edit::column_ops::duplicate(
                    &config,
                    &request.resources_dir,
                    &spec,
                    as_key.as_deref(),
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.delete" => {
                let spec = require(&request.params, "spec")?;
                let edit =
                    doc_edit::column_ops::delete(&config, &request.resources_dir, &spec, dry_run)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.reorder" => {
                let spec = require(&request.params, "spec")?;
                // `before` empty is "last" — the one position "before a field"
                // cannot name.
                let before = optional(&request.params, "before").unwrap_or_default();
                let edit =
                    doc_edit::column_ops::reorder(&config, &request.resources_dir, &spec, &before)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "column.hide" | "column.show" => {
                let spec = require(&request.params, "spec")?;
                let view = require(&request.params, "view")?;
                let hidden = def.id == "column.hide";
                let edit = doc_edit::column_ops::set_visibility(
                    &config,
                    &request.resources_dir,
                    &spec,
                    &view,
                    hidden,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "view.setLayout" | "view.setFilter" | "view.setSort" | "view.setGroup"
            | "view.setLimit" | "view.setColumns" | "view.setPanel" => {
                let name = require(&request.params, "name")?;
                let edit = match def.id.as_str() {
                    // The block form: a record block's kind is its layout.
                    "view.setLayout" => match optional(&request.params, "block") {
                        Some(index) => {
                            let index = index.parse::<usize>().map_err(|_| {
                                DispatchError::Usage(format!(
                                    "block must be a whole number, not \"{index}\""
                                ))
                            })?;
                            let layout = require(&request.params, "layout")?;
                            doc_edit::view_ops::set_block_layout(
                                &config,
                                &request.resources_dir,
                                &name,
                                index,
                                &layout,
                            )?
                        }
                        None => doc_edit::view_ops::set_query(
                            &config,
                            &request.resources_dir,
                            &name,
                            "layout",
                            Some(&require(&request.params, "layout")?),
                        )?,
                    },
                    other => {
                        // The id names the key it writes: `view.setFilter` → `filter`.
                        let key = match other {
                            "view.setFilter" => "filter",
                            "view.setSort" => "sort",
                            "view.setGroup" => "group",
                            "view.setLimit" => "limit",
                            "view.setPanel" => "panel",
                            _ => "columns",
                        };
                        let value = optional(&request.params, key);
                        doc_edit::view_ops::set_query(
                            &config,
                            &request.resources_dir,
                            &name,
                            key,
                            value.as_deref(),
                        )?
                    }
                };
                edit_outcome(&root, &config, edit, dry_run)?
            }
            // The composer's one write (COMPOSER §2.2): the screen's whole
            // component list lands in a single transaction, so a drag or an add
            // is one plan write and one undo step.
            "view.setComponents" => {
                let edit = doc_edit::view_ops::set_components(
                    &config,
                    &request.resources_dir,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "view.block.add" => {
                let edit = doc_edit::view_ops::block_add(
                    &config,
                    &request.resources_dir,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "view.block.set" => {
                let edit = doc_edit::view_ops::block_set(
                    &config,
                    &request.resources_dir,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "view.block.remove" => {
                let edit = doc_edit::view_ops::block_remove(
                    &config,
                    &request.resources_dir,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            // A block's address is a path (blocks nest) and its position is a
            // target — the tree's own verb (`UI_SPEC.md` §1 R18).
            "view.block.move" => {
                let edit = doc_edit::view_ops::block_move(
                    &config,
                    &request.resources_dir,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "type.new" => {
                let edit =
                    doc_edit::type_ops::new_type(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "list.new" => {
                let edit =
                    doc_edit::list_ops::new_list(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            // The three deletes (UI P3 · U6): a delete is a cascade, so each one
            // plans every document it touches and the whole-plan validator has
            // the last word before a byte lands (§4.6 step 2).
            "type.delete" => {
                let edit =
                    doc_edit::type_ops::delete(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "view.delete" => {
                let edit = doc_edit::view_ops::delete_view(
                    &config,
                    &request.resources_dir,
                    &request.params,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "list.set" => {
                let edit =
                    doc_edit::list_ops::set(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "list.delete" => {
                let edit =
                    doc_edit::list_ops::delete(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "settings.set" => {
                let key = require(&request.params, "key")?;
                let value = require(&request.params, "value")?;
                let edit =
                    doc_edit::settings_ops::set(&config, &request.resources_dir, &key, &value)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "metric.set" => {
                let edit =
                    doc_edit::metric_ops::set(&config, &request.resources_dir, &request.params)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            // Phase 7: appearance as data (theme, overrides, text size) and the
            // private-record rule. Every one is a whole-plan validated edit on
            // content/appearance.json or content/types.json.
            "theme.set" => {
                let id = require(&request.params, "id")?;
                let edit =
                    doc_edit::appearance_ops::set_theme(&config, &request.resources_dir, &id)?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "appearance.setTextScale" => {
                let value = require(&request.params, "value")?;
                let scale = value
                    .parse::<f64>()
                    .map_err(|_| DispatchError::Usage(format!("\"{value}\" is not a number")))?;
                let edit = doc_edit::appearance_ops::set_text_scale(
                    &config,
                    &request.resources_dir,
                    scale,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "appearance.setOverride" => {
                let key = require(&request.params, "key")?;
                let value = require(&request.params, "value")?;
                let edit = doc_edit::appearance_ops::set_override(
                    &config,
                    &request.resources_dir,
                    &key,
                    &value,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "appearance.clearOverride" => {
                let key = require(&request.params, "key")?;
                let edit = doc_edit::appearance_ops::clear_override(
                    &config,
                    &request.resources_dir,
                    &key,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "type.setPrivate" => {
                let type_id = require(&request.params, "type")?;
                let private = flag(&request.params, "private");
                let edit = doc_edit::type_ops::set_private(
                    &config,
                    &request.resources_dir,
                    &type_id,
                    private,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "profile.export" => {
                let out = require(&request.params, "out")?;
                let personal = flag(&request.params, "personal");
                let name = optional(&request.params, "name");
                let document = crate::profile::export(
                    &config,
                    personal,
                    name.as_deref(),
                    Some(crate::transaction::iso_now()),
                )
                .map_err(DispatchError::Usage)?;
                let bytes = crate::profile::to_bytes(&document).map_err(DispatchError::Usage)?;
                let destination = PathBuf::from(&out);
                if let Some(parent) = destination.parent()
                    && !parent.as_os_str().is_empty()
                    && !dry_run
                {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        DispatchError::Io(format!("cannot create {}: {error}", parent.display()))
                    })?;
                }
                if !dry_run {
                    std::fs::write(&destination, &bytes).map_err(|error| {
                        DispatchError::Io(format!(
                            "cannot write {}: {error}",
                            destination.display()
                        ))
                    })?;
                }
                DispatchOutcome::Write {
                    commit: CommitResult {
                        txid: if dry_run {
                            String::new()
                        } else {
                            crate::transaction::make_txid()
                        },
                        base_revision: config.revision.clone(),
                        new_revision: config.revision.clone(),
                        files: if dry_run {
                            Vec::new()
                        } else {
                            vec![destination.display().to_string()]
                        },
                    },
                    changes: Vec::new(),
                    data: serde_json::json!({
                        "profile": destination.display().to_string(),
                        "name": document.name,
                        "includes": document.includes,
                        "bytes": bytes.len(),
                    }),
                    summary: format!("profile.export {}", destination.display()),
                    dry_run,
                }
            }
            "source.apply" => {
                let file = require(&request.params, "file")?;
                let text = match request.params.get("content") {
                    Some(ParamValue::Bytes(bytes)) => String::from_utf8(bytes.clone())
                        .map_err(|_| DispatchError::Usage("--content is not valid UTF-8".into()))?,
                    Some(ParamValue::Str(text)) => text.clone(),
                    _ => {
                        return Err(DispatchError::Usage(
                            "missing required parameter --content".into(),
                        ));
                    }
                };
                let edit = doc_edit::source_ops::apply(
                    &config,
                    &request.resources_dir,
                    &file,
                    &text,
                    dry_run,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "record.advanceStage" => {
                let id = require(&request.params, "id")?;
                let stage = optional(&request.params, "stage").filter(|stage| !stage.is_empty());
                let evidence = crate::pipeline::Evidence {
                    problems: optional(&request.params, "problems")
                        .map(|text| {
                            text.parse::<i64>().map_err(|_| {
                                DispatchError::Usage(format!(
                                    "--problems takes a whole number, not \"{text}\""
                                ))
                            })
                        })
                        .transpose()?,
                    reason: optional(&request.params, "reason").filter(|reason| !reason.is_empty()),
                    signal: optional(&request.params, "signal").filter(|signal| !signal.is_empty()),
                    review: None,
                };
                let at = optional(&request.params, "at");
                let edit = crate::pipeline::advance(
                    &config,
                    &request.resources_dir,
                    &id,
                    stage.as_deref(),
                    &evidence,
                    at.as_deref(),
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "record.logReview" => {
                let id = require(&request.params, "id")?;
                let rating = require(&request.params, "rating")?;
                let at = optional(&request.params, "at");
                let edit = crate::scheduler::log_review(
                    &config,
                    &request.resources_dir,
                    &id,
                    &rating,
                    at.as_deref(),
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "type.setPipeline" => {
                let type_name = require(&request.params, "type")?;
                let pipeline = require(&request.params, "pipeline")?;
                let fresh = flag(&request.params, "fresh");
                let entries: Vec<String> = match request.params.get("map") {
                    Some(ParamValue::Strings(values)) => values.clone(),
                    Some(ParamValue::Str(value)) => vec![value.clone()],
                    _ => Vec::new(),
                };
                let mut mapping: BTreeMap<String, String> = BTreeMap::new();
                for entry in entries {
                    let Some((from, to)) = entry.split_once('=') else {
                        return Err(DispatchError::Usage(format!(
                            "--map takes old=new (e.g. --map learned=done), got \"{entry}\""
                        )));
                    };
                    mapping.insert(from.trim().to_string(), to.trim().to_string());
                }
                let edit = crate::pipeline::set_pipeline(
                    &config,
                    &request.resources_dir,
                    &type_name,
                    &pipeline,
                    &mapping,
                    fresh,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "record.move" => {
                let id = require(&request.params, "id")?;
                let to = require(&request.params, "to")?;
                let parent = optional(&request.params, "parent");
                let edit = doc_edit::structure_ops::move_record(
                    &config,
                    &request.resources_dir,
                    &id,
                    &to,
                    parent.as_deref(),
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            "records.renumber" => {
                let type_name = require(&request.params, "type")?;
                let field = optional(&request.params, "field").unwrap_or_else(|| "index".into());
                let parent = optional(&request.params, "parent");
                let start = optional(&request.params, "start")
                    .map(|text| {
                        text.parse::<i64>().map_err(|_| {
                            DispatchError::Usage(format!(
                                "--start takes a whole number, not \"{text}\""
                            ))
                        })
                    })
                    .transpose()?
                    .unwrap_or(1);
                let edit = doc_edit::structure_ops::renumber(
                    &config,
                    &request.resources_dir,
                    &type_name,
                    &field,
                    parent.as_deref(),
                    start,
                )?;
                edit_outcome(&root, &config, edit, dry_run)?
            }
            other => {
                return Err(DispatchError::Usage(format!(
                    "{other} has no session handler"
                )));
            }
        };
        // §4.6 step 5: notify once, refresh the derived index once. A cache that
        // does not exist is not created here (a plan with no index pays nothing),
        // and a refresh failure is a warning on a successful commit — never a
        // failed write inviting a duplicate retry (§4.9).
        let mut outcome = outcome;
        if let DispatchOutcome::Write {
            commit,
            changes,
            data,
            dry_run: false,
            ..
        } = &mut outcome
            && !commit.txid.is_empty()
        {
            let (refreshed, warning) = crate::index::refresh_after_commit(
                &root,
                &request.resources_dir,
                &commit.new_revision,
                changes,
            );
            if refreshed || warning.is_some() {
                if let Some(object) = data.as_object_mut() {
                    object.insert("indexRefreshed".into(), json!(refreshed));
                    if let Some(warning) = warning {
                        object.insert("indexWarning".into(), json!(warning));
                    }
                }
            }
        }
        // Hold the lock through commit and report.
        drop(lock);
        Ok(outcome)
    }
}

fn apply_batch(
    root: &Path,
    config: &ResolvedConfig,
    params: &Params,
    dry_run: bool,
) -> Result<DispatchOutcome, DispatchError> {
    let (batch, name) = match params.get("batch") {
        Some(ParamValue::Bytes(data)) => (data.clone(), "<stdin>".to_string()),
        Some(ParamValue::Str(path)) if path != "-" => {
            let data = std::fs::read(path)
                .map_err(|_| DispatchError::Io(format!("cannot read batch {path}")))?;
            (data, path.clone())
        }
        _ => {
            return Err(DispatchError::Usage(
                "apply requires one batch source: a file path or '-' for stdin".into(),
            ));
        }
    };

    let planned = apply::plan(config, &batch, &name)?;
    if dry_run {
        let commit = CommitResult {
            txid: String::new(),
            base_revision: planned.base_revision.clone(),
            new_revision: planned.base_revision.clone(),
            files: Vec::new(),
        };
        let data = json!({
            "dryRun": true,
            "baseRevision": planned.base_revision,
            "candidateRevision": planned.candidate_revision,
            "wouldChange": planned.changes_anything(),
            "counts": planned.counts,
            "files": planned.changes.iter().map(|change| change.path.clone()).collect::<Vec<_>>(),
            "diff": planned.diff,
        });
        return Ok(DispatchOutcome::Write {
            commit,
            changes: Vec::new(),
            data,
            summary: "apply (dry-run)".into(),
            dry_run: true,
        });
    }

    let applied = planned.counts.total() - planned.counts.unchanged;
    let result = apply::commit(root, &planned)?;
    let data = json!({
        "baseRevision": result.base_revision,
        "txid": result.txid,
        "applied": applied,
        "counts": planned.counts,
        "files": result.files,
    });
    Ok(DispatchOutcome::Write {
        commit: result,
        changes: planned.changes,
        data,
        summary: "apply".into(),
        dry_run: false,
    })
}

/// Restore one committed transaction's before-images. `tx.restore` is the
/// recovery door with a preview; `edit.undo` / `edit.redo` are the same
/// mechanism reached by the accelerator — the app supplies the txid it recorded
/// when the edit committed, and `--if-revision` (checked before this runs) is
/// what makes a stale entry refuse rather than clobber a newer write (§4.8 P6).
fn restore(
    root: &Path,
    config: &ResolvedConfig,
    request: &DispatchRequest,
    dry_run: bool,
    verb: &str,
) -> Result<DispatchOutcome, DispatchError> {
    let txid = require(&request.params, "txid")?;
    let restore = transaction::restore_plan(root, &txid)?;
    if restore.changes.is_empty() {
        return Ok(DispatchOutcome::Read(
            json!({ "txid": txid, "restored": false }),
        ));
    }
    if dry_run {
        let commit = CommitResult {
            txid: String::new(),
            base_revision: config.revision.clone(),
            new_revision: config.revision.clone(),
            files: Vec::new(),
        };
        let notes = restore.notes;
        return Ok(DispatchOutcome::Write {
            commit,
            changes: Vec::new(),
            data: json!({ "txid": txid, "restored": false, "dryRun": true, "notes": notes }),
            summary: format!("{verb} {txid}"),
            dry_run: true,
        });
    }

    // Whole-plan validation before anything is written (§4.6 step 2).
    transaction::validate_by_overlay(root, &restore.changes, &request.resources_dir)?;
    let base_hashes: BTreeMap<String, String> = config
        .source_files
        .iter()
        .map(|(path, bytes)| (path.clone(), config_store::fnv_hex(bytes)))
        .collect();
    let new_revision = transaction::revision_after(&base_hashes, &restore.changes);
    let summary = format!("{verb} {txid}");
    let result = transaction::commit(
        root,
        &restore.changes,
        &config.revision,
        &new_revision,
        &summary,
    )?;
    let notes = restore.notes;
    let data = json!({
        "txid": txid,
        "restored": true,
        "txid_applied": result.txid,
        "notes": notes,
    });
    Ok(DispatchOutcome::Write {
        commit: result,
        changes: restore.changes,
        data,
        summary,
        dry_run: false,
    })
}

/// A record batch as a dispatch outcome: `record.new` and `record.setField`
/// go through `apply`, so a record created by a click, by the palette and by
/// `SAM apply` are the same object with the same canonical bytes.
fn batch_outcome(
    root: &Path,
    planned: apply::PlannedBatch,
    dry_run: bool,
    summary: &str,
) -> Result<DispatchOutcome, DispatchError> {
    let data = json!({
        "baseRevision": planned.base_revision,
        "candidateRevision": planned.candidate_revision,
        "wouldChange": planned.changes_anything(),
        "counts": planned.counts,
        "diff": planned.diff,
    });
    if dry_run || !planned.changes_anything() {
        return Ok(DispatchOutcome::Write {
            commit: CommitResult {
                txid: String::new(),
                base_revision: planned.base_revision.clone(),
                new_revision: planned.candidate_revision.clone(),
                files: Vec::new(),
            },
            changes: planned.changes,
            data,
            summary: summary.into(),
            dry_run,
        });
    }
    let result = apply::commit(root, &planned)?;
    Ok(DispatchOutcome::Write {
        commit: result,
        changes: planned.changes,
        data,
        summary: summary.into(),
        dry_run: false,
    })
}

/// A document or column edit as a dispatch outcome. The edit was validated
/// whole-plan when it was planned (§4.6 step 2), so this only commits — through
/// the same transaction engine every other write uses.
fn edit_outcome(
    root: &Path,
    config: &ResolvedConfig,
    edit: doc_edit::PlannedEdit,
    dry_run: bool,
) -> Result<DispatchOutcome, DispatchError> {
    let data = edit.preview.clone().unwrap_or_else(|| json!({}));
    let summary = if edit.summary.is_empty() {
        "edit".to_string()
    } else {
        edit.summary.clone()
    };
    if dry_run || !edit.changes_anything() {
        return Ok(DispatchOutcome::Write {
            commit: CommitResult {
                txid: String::new(),
                base_revision: config.revision.clone(),
                new_revision: config.revision.clone(),
                files: Vec::new(),
            },
            changes: edit.changes,
            data,
            summary,
            dry_run,
        });
    }
    let base_hashes: BTreeMap<String, String> = config
        .source_files
        .iter()
        .map(|(path, bytes)| (path.clone(), config_store::fnv_hex(bytes)))
        .collect();
    let new_revision = transaction::revision_after(&base_hashes, &edit.changes);
    let commit = transaction::commit(
        root,
        &edit.changes,
        &config.revision,
        &new_revision,
        &summary,
    )?;
    Ok(DispatchOutcome::Write {
        commit,
        changes: edit.changes,
        data,
        summary,
        dry_run: false,
    })
}

/// `{ "params": { … } }` from JSON, into the one parameter vocabulary. JSON
/// strings, booleans, numbers and arrays map onto `Str`, `Bool`, `Str` and
/// `Strings`; `null` means "not supplied" rather than "supply nothing" (§4.9:
/// nothing prompts, and an omitted parameter is omitted).
pub fn params_from_json(value: Option<&serde_json::Value>) -> Result<Params, DispatchError> {
    let mut params = Params::new();
    let Some(serde_json::Value::Object(object)) = value else {
        return Ok(params);
    };
    for (name, value) in object {
        match value {
            serde_json::Value::Bool(boolean) => {
                params.insert(name.clone(), ParamValue::Bool(*boolean));
            }
            serde_json::Value::String(text) => {
                params.insert(name.clone(), ParamValue::Str(text.clone()));
            }
            serde_json::Value::Number(number) => {
                params.insert(name.clone(), ParamValue::Str(number.to_string()));
            }
            serde_json::Value::Array(items) => {
                params.insert(
                    name.clone(),
                    ParamValue::Strings(
                        items
                            .iter()
                            .map(|item| match item {
                                serde_json::Value::String(text) => text.clone(),
                                other => other.to_string(),
                            })
                            .collect(),
                    ),
                );
            }
            serde_json::Value::Null => {}
            other => {
                params.insert(name.clone(), ParamValue::Str(other.to_string()));
            }
        }
    }
    Ok(params)
}

/// The preset a `preset` read names, defaulting to the blank plan (§3.2).
fn request_preset(params: &Params) -> Result<String, DispatchError> {
    // The read's default is the write's default: what `plan.new` creates with no
    // preset named is what `preset` shows with none named, or the two doors
    // would disagree about the app's own starting point.
    let name = optional(params, "name").unwrap_or_else(|| "blank".to_string());
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(DispatchError::Usage(format!(
            "\"{name}\" is not a preset name"
        )));
    }
    Ok(name)
}

/// The plans directory as data, for the picker (Appendix C.5's `several`
/// phase). A directory counts as a plan when it carries `content/types.json`;
/// anything else in the directory is not offered as one.
fn list_plans(plans_dir: &Path) -> Vec<serde_json::Value> {
    let mut plans = Vec::new();
    let Ok(entries) = std::fs::read_dir(plans_dir) else {
        return plans;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("content/types.json").is_file() {
            continue;
        }
        plans.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "path": path.display().to_string(),
            "revision": config_store::raw_revision(&path),
        }));
    }
    plans.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    plans
}

/// An outcome as data — one shape for `SAM invoke`, the shell's `dispatch`
/// command and the browser harness. A presentation action reports its
/// destination and that no GUI is available rather than aborting a batch with
/// exit 5 (§4.9).
pub fn outcome_json(outcome: DispatchOutcome) -> serde_json::Value {
    match outcome {
        DispatchOutcome::Read(data) => json!({ "ok": true, "data": data }),
        DispatchOutcome::Write {
            commit,
            changes,
            data,
            summary,
            dry_run,
        } => json!({
            "ok": true,
            "dryRun": dry_run,
            "revision": commit.new_revision,
            "txid": commit.txid,
            "summary": summary,
            "files": changes.iter().map(|change| change.path.clone()).collect::<Vec<_>>(),
            "data": data,
        }),
        DispatchOutcome::Presentation { destination } => json!({
            "ok": true,
            "destination": destination,
            "available": false,
            "reason": "no GUI in this context",
        }),
    }
}

/// The derived index when one exists and matches the published revision —
/// `None` on a cache that is missing, stale, of another format or corrupt,
/// because the in-memory snapshot is always a correct answer and the index is
/// an optimization (§4.6). Callers that need a reason call the checks directly.
pub fn fresh_index(config: &ResolvedConfig, resources_dir: &Path) -> Option<crate::index::Index> {
    let paths = crate::resources::paths_with_resources(resources_dir.to_path_buf()).ok()?;
    crate::index::open_fresh(&config.plan_root, &paths.indexes_dir, &config.revision)
        .ok()
        .flatten()
}

/// Plan creation: `plan.new` (a preset) and `profile.import` (a
/// `*.samprofile` document). Both materialize a complete plan through
/// `config_store`'s staged path — filled, loaded by the real loader, then
/// renamed into place — so a validation failure leaves nothing behind and the
/// result is a plan every other command can already open.
fn create_plan_outcome(
    request: &DispatchRequest,
    id: &str,
) -> Result<DispatchOutcome, DispatchError> {
    let paths = crate::resources::paths_with_resources(request.resources_dir.clone())
        .map_err(DispatchError::Io)?;
    let name = optional(&request.params, "name");
    let target = optional(&request.params, "target").map(PathBuf::from);
    let dry_run = flag(&request.params, "dry-run");
    if id == "plan.new" && name.is_none() && target.is_none() {
        return Err(DispatchError::Usage(
            "plan.new needs --name <name> (a plan directory name, not a path) or --target <directory>"
                .into(),
        ));
    }
    // An import reads its document once and keeps it: the name it suggests is a
    // fact about the document, and reading the file twice would be two answers
    // to the same question.
    let document = if id == "profile.import" {
        let file = require(&request.params, "file")?;
        let bytes = std::fs::read(&file)
            .map_err(|error| DispatchError::Io(format!("cannot read {file}: {error}")))?;
        Some(crate::profile::from_bytes(&bytes).map_err(DispatchError::Usage)?)
    } else {
        None
    };

    let explicit = name.is_some() || target.is_some();
    let target = if explicit {
        config_store::plan_destination(&paths.plans_dir, name.as_deref(), target.as_deref())
            .map_err(DispatchError::Usage)?
    } else if let Some(document) = &document {
        // No name given: the profile's own name, beside any earlier import
        // rather than over it — a double-click twice is not a dead end.
        config_store::free_plan_destination(&paths.plans_dir, &document.default_plan_name())
    } else {
        config_store::plan_destination(&paths.plans_dir, name.as_deref(), None)
            .map_err(DispatchError::Usage)?
    };
    if target.exists() {
        return Err(DispatchError::Io(format!(
            "plan already exists: {}",
            target.display()
        )));
    }

    let mut data = serde_json::Map::new();
    data.insert(
        "plan".into(),
        serde_json::json!(target.display().to_string()),
    );
    let mut files = Vec::new();
    let is_import = document.is_some();
    if !dry_run {
        let created = match document {
            Some(document) => {
                data.insert(
                    "kind".into(),
                    serde_json::json!(crate::profile::PROFILE_KIND),
                );
                data.insert(
                    "profile".into(),
                    serde_json::json!({ "name": document.name, "includes": document.includes }),
                );
                crate::profile::import(&document, &target, &request.resources_dir)
            }
            None => {
                let preset = optional(&request.params, "preset").unwrap_or_else(|| "blank".into());
                data.insert("preset".into(), serde_json::json!(preset));
                config_store::create_plan(&target, &preset, &request.resources_dir)
            }
        }
        .map_err(DispatchError::from)?;
        files.push(created.display().to_string());
        data.insert(
            "revision".into(),
            serde_json::json!(config_store::raw_revision(&created)),
        );
    }
    let summary = if is_import {
        format!("profile.import {}", target.display())
    } else {
        format!("plan.new {}", target.display())
    };
    Ok(DispatchOutcome::Write {
        commit: CommitResult {
            // No txid: creating a plan is not a transaction *in* a plan, and
            // the app's undo stack keys on txids it can reverse (`edit.undo`).
            txid: String::new(),
            base_revision: String::new(),
            new_revision: data
                .get("revision")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            files: files.clone(),
        },
        changes: Vec::new(),
        data: serde_json::Value::Object(data),
        summary,
        dry_run,
    })
}

/// Machine layer JSON projection (UI P3 · U6, S14).
/// Aggregates schema kinds, views, destinations, blocks, rules, and pipelines (`UI_PLAN.md` D13).
fn machine_json(config: &ResolvedConfig) -> serde_json::Value {
    fn blocks_of(view: &crate::model::ViewDef) -> Vec<&crate::model::BlockDef> {
        fn walk<'a>(
            blocks: &'a [crate::model::BlockDef],
            into: &mut Vec<&'a crate::model::BlockDef>,
        ) {
            for block in blocks {
                into.push(block);
                walk(block.blocks.as_deref().unwrap_or_default(), into);
                walk(block.else_blocks.as_deref().unwrap_or_default(), into);
            }
        }
        let mut all = Vec::new();
        walk(view.blocks.as_deref().unwrap_or_default(), &mut all);
        all
    }

    // A kind has no name of its own: the rail title of the first destination
    // that opens one of its views is the word the student already reads.
    let title_of = |kind: &str| -> Option<String> {
        config.shell.as_ref().and_then(|shell| {
            shell.navigation.as_deref().and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| {
                        config
                            .views
                            .get(&entry.view)
                            .is_some_and(|view| view.type_.as_deref() == Some(kind))
                    })
                    .map(|entry| entry.title.clone())
            })
        })
    };

    let kinds: Vec<serde_json::Value> = config
        .types
        .iter()
        .map(|(id, def)| {
            let records = config.records_iter().filter(|r| &r.type_ == id).count();
            let views: Vec<&String> = config
                .views
                .iter()
                .filter(|(_, view)| view.type_.as_deref() == Some(id.as_str()))
                .map(|(name, _)| name)
                .collect();
            let fields: Vec<serde_json::Value> = def
                .fields
                .iter()
                .map(|field| {
                    // "Used by" for a column is exact and narrow: the views of
                    // this kind that pin it in their `columns` list. Anything
                    // wider (a filter that mentions the key) would be a guess.
                    let pinned = config
                        .views
                        .values()
                        .filter(|view| {
                            view.type_.as_deref() == Some(id.as_str())
                                && view
                                    .columns
                                    .as_deref()
                                    .is_some_and(|columns| columns.contains(&field.key))
                        })
                        .count();
                    json!({
                        "key": field.key,
                        "label": field.label,
                        "type": field.type_,
                        "to": field.to,
                        "options": field.options,
                        "expr": field.expr,
                        "required": field.required,
                        "pinned": pinned,
                    })
                })
                .collect();
            json!({
                "id": id,
                "title": title_of(id),
                "icon": def.icon,
                "parent": def.parent,
                "trackable": def.trackable,
                "pipeline": def.pipeline,
                "private": def.private,
                "records": records,
                "views": views,
                "fields": fields,
            })
        })
        .collect();

    let metrics = config.rules.metrics.clone().unwrap_or_default();
    let mut drawn_by: std::collections::BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, view) in &config.views {
        for block in blocks_of(view) {
            if let Some(source) = &block.view {
                drawn_by
                    .entry(source.clone())
                    .or_default()
                    .push(name.clone());
            }
        }
    }

    let views: Vec<serde_json::Value> = config
        .views
        .iter()
        .map(|(name, view)| {
            let destinations: Vec<serde_json::Value> = config
                .shell
                .as_ref()
                .and_then(|shell| shell.navigation.as_deref())
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|entry| &entry.view == name)
                        .map(|entry| json!({ "title": entry.title, "icon": entry.icon }))
                        .collect()
                })
                .unwrap_or_default();
            let folded: Vec<&String> = metrics
                .iter()
                .filter(|(_, metric)| &metric.view == name)
                .map(|(metric_name, _)| metric_name)
                .collect();
            let mut drawn: Vec<String> = drawn_by.get(name).cloned().unwrap_or_default();
            drawn.sort();
            drawn.dedup();
            json!({
                "name": name,
                "kind": view.type_,
                "layout": view.layout,
                // `null` when the screen is not composed, a count when it is —
                // zero is a real answer (an empty screen the student started).
                "components": view.components.as_ref().map(|list| list.len()),
                "panel": view.panel,
                "blocks": blocks_of(view).len(),
                "columns": view.columns,
                "filter": view.filter,
                "sort": view.sort,
                "group": view.group,
                "limit": view.limit,
                "destinations": destinations,
                "drawnBy": drawn,
                "foldedBy": folded,
            })
        })
        .collect();

    let pipelines: Vec<serde_json::Value> = config
        .rules
        .pipelines
        .iter()
        .flat_map(|pipelines| pipelines.iter())
        .map(|(name, pipeline)| {
            let used_by: Vec<&String> = config
                .types
                .iter()
                .filter(|(_, def)| def.pipeline.as_deref() == Some(name.as_str()))
                .map(|(id, _)| id)
                .collect();
            json!({
                "name": name,
                "stages": pipeline.stages,
                "gates": pipeline.gates,
                "completeWhen": pipeline.complete_when,
                "usedBy": used_by,
            })
        })
        .collect();

    let metrics: Vec<serde_json::Value> = metrics
        .iter()
        .map(|(name, metric)| {
            json!({
                "name": name,
                "label": metric.label,
                "view": metric.view,
                "expr": metric.expr,
                "reduce": metric.reduce,
                "unit": metric.unit,
                "viewMissing": !config.views.contains_key(&metric.view),
            })
        })
        .collect();

    let destinations: Vec<serde_json::Value> = config
        .shell
        .as_ref()
        .and_then(|shell| shell.navigation.as_deref())
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    json!({
                        "title": entry.title,
                        "view": entry.view,
                        "icon": entry.icon,
                        "kind": config.views.get(&entry.view).map(|view| view.type_.clone()),
                        "missing": !config.views.contains_key(&entry.view),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // The plan's own review load: how many scheduled reviews fall on each of
    // the next fourteen days. This is what a scheduler change is a decision
    // *about* (F10's consequence), and it is a fact of the stored state — the
    // dates already written — not a simulation of a scheduler this build would
    // have to re-implement to project.
    let state = crate::views::PlanState::load(&config.plan_root);
    let context = crate::evaluator::EvalContext::new(config, Some(&state));
    let today = crate::evaluator::civil_date(context.now_unix, context.tz_offset_minutes);
    let mut due_by_day: BTreeMap<String, usize> = BTreeMap::new();
    {
        for record in config.records_iter() {
            let Some(entry) = state.entry(&record.id) else {
                continue;
            };
            let due = entry
                .review
                .as_ref()
                .and_then(crate::scheduler::Review::from_json)
                .and_then(|review| review.due);
            if let Some(due) = due.filter(|due| due.as_str() >= today.as_str()) {
                *due_by_day.entry(due).or_default() += 1;
            }
        }
    }
    let due_curve: Vec<serde_json::Value> = due_by_day
        .iter()
        .take(14)
        .map(|(date, due)| json!({ "date": date, "due": due }))
        .collect();

    json!({
        "kinds": kinds,
        "views": views,
        "destinations": destinations,
        "today": today,
        "dueCurve": due_curve,
        "rules": serde_json::to_value(&config.rules).unwrap_or(serde_json::Value::Null),
        "pipelines": pipelines,
        "metrics": metrics,
        // The designed screens a destination may be given, from the engine's own
        // closed vocabulary — so the picker cannot offer a name the loader
        // warns about.
        "panels": crate::model::PANEL_KINDS,
        "blockKinds": crate::model::BLOCK_KINDS,
    })
}

/// `plan` serve `paths`, which answers even with no plan.
pub fn read_json(
    id: &str,
    params: &Params,
    config: Option<&ResolvedConfig>,
    resources_dir: &Path,
    plan: Option<&Path>,
) -> Result<serde_json::Value, DispatchError> {
    match id {
        "paths" => {
            // A machine with no plan at all still gets its paths: the registry
            // marks this read `plan_required: false`, it is the command a bug
            // report runs before anything is open, and the first run's own boot
            // asks it before a plan root exists (BUILDLOG 2026-10-06). Nothing
            // that resolves is invented — the plan is `null`, and `plans`
            // carries the picker's list either way.
            let selected = config_store::plan_root(plan).ok();
            let paths = crate::resources::paths_with_resources(resources_dir.to_path_buf())
                .map_err(DispatchError::Io)?;
            // The derived index's location, so a script never guesses it and a
            // bug in the resolver is visible in one command on every platform
            // (§4.1, §4.7). `index` is the path when a cache exists, else null;
            // `indexFresh` says whether it matches the current revision.
            let index_path = selected
                .as_ref()
                .map(|(root, _)| crate::index::index_path(root, &paths.indexes_dir));
            let index_present = index_path.as_ref().is_some_and(|path| path.is_file());
            // Freshness is the indexer's own tag compared against the source
            // fingerprint — the same comparison `open_fresh` makes, done here
            // from the raw revision so `paths` works without a loaded config.
            let revision = selected
                .as_ref()
                .map(|(root, _)| config_store::raw_revision(root));
            let stored = selected
                .as_ref()
                .map(|(root, _)| crate::index::status_json(root, &paths.indexes_dir, None));
            let index_fresh = index_present
                && stored
                    .as_ref()
                    .and_then(|stored| stored.get("healthy"))
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                && stored
                    .as_ref()
                    .and_then(|stored| stored.get("revision"))
                    .and_then(serde_json::Value::as_str)
                    == revision.as_deref();
            Ok(json!({
                "dataDir": paths.data_dir.display().to_string(),
                "plansDir": paths.plans_dir.display().to_string(),
                "indexesDir": paths.indexes_dir.display().to_string(),
                "resourcesDir": paths.resources_dir.display().to_string(),
                "plan": selected
                    .as_ref()
                    .map(|(root, _)| root.display().to_string()),
                "layer": selected.as_ref().map(|(_, layer)| layer.clone()),
                "index": index_path
                    .as_ref()
                    .filter(|path| path.is_file())
                    .map(|path| path.display().to_string()),
                "indexFresh": index_fresh,
                "revision": revision,
                // What the plan picker draws (Appendix C.5's `several` phase).
                // The webview has no filesystem by capability (§4.10), so the
                // list is the engine's, not the page's.
                "plans": list_plans(&paths.plans_dir),
                // §9/D21: which WebKitGTK renderer path the last shell launch
                // took ("native" when nothing was overridden). Written by the
                // shell, read here — so a bug report carries the fact instead of
                // the user being asked to guess.
                "rendererPath": crate::resources::renderer_path(&paths),
                // §4.10's file association, as the shell observed it: the last
                // file the OS handed the app, and which door it came through.
                "lastOpen": crate::resources::last_open(&paths),
            }))
        }
        "views" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("views needs a plan".into()));
            };
            Ok(json!({
                "views": config
                    .views
                    .iter()
                    .map(|(name, view)| {
                        // The **whole** ViewDef, not a summary: the view editor
                        // edits these keys by name (§4.8 P3 — *the UI never hides
                        // the JSON identity of what it edits*), so a projection
                        // that dropped `filter` would make the editor guess.
                        (
                            name.clone(),
                            serde_json::to_value(view).unwrap_or(serde_json::json!({})),
                        )
                    })
                    .collect::<BTreeMap<String, serde_json::Value>>(),
                "navigation": config
                    .shell
                    .as_ref()
                    .and_then(|shell| shell.navigation.clone())
                    .unwrap_or_default()
                    .iter()
                    // The rail draws a destination from these three keys (§1 R3):
                    // the label, the view it opens, and the icon the plan names —
                    // null when it names none, so the frame falls back rather than
                    // the engine inventing a glyph.
                    .map(|entry| {
                        json!({ "title": entry.title, "view": entry.view, "icon": entry.icon })
                    })
                    .collect::<Vec<serde_json::Value>>(),
                // The keymap is data (§4.8 P2: accelerators are data), and the
                // UI resolves `mod` against `data-os`. Collisions are the
                // validator's finding, never recomputed in the UI (§3.7: one
                // diagnostic source). `uicheck::keybindings` merges the plan's
                // declarations over the platform defaults, so the app answers to
                // ⌘Z even on a plan that declares no bindings.
                "keybindings": crate::uicheck::keybindings(config)
                    .iter()
                    .map(|(key, command)| json!({ "key": key, "command": command }))
                    .collect::<Vec<serde_json::Value>>(),
                // The settings screen is generated from the app's own schema
                // (§4.2), so the UI reads the same rows the CLI's `settings.set`
                // validates against — key, type, declared options, and the JSON
                // pointer each row persists at (§4.8 P3).
                "settings": doc_edit::settings_ops::FIELDS
                    .iter()
                    .map(|def| json!({
                        "key": def.key,
                        "type": def.type_,
                        "label": def.label,
                        "path": def.path.join("/"),
                        "to": null,
                        "options": if def.options.is_empty() {
                            serde_json::Value::Null
                        } else {
                            json!(def.options)
                        },
                    }))
                    .collect::<Vec<serde_json::Value>>(),
                // Advisory findings the loader published (§3.7): the UI reports
                // them, it never recomputes them (one diagnostic source).
                "warnings": config
                    .load_warnings
                    .iter()
                    .map(|warning| serde_json::to_value(warning).unwrap_or(serde_json::Value::Null))
                    .collect::<Vec<serde_json::Value>>(),
                // The settings screen's current values, keyed by row key — the
                // screen is generated from `settings` above and prefilled from
                // here, so a row never guesses its own storage.
                "study": doc_edit::settings_ops::values(config),
                // The frame's titlebar slot and the rail's date disc (§1 R2/R3):
                // "week n of N" in the plan's own calendar, else the civil date.
                // The engine owns both facts — the plan's timezone and its week
                // and term records — so the shell reads them, never computes
                // them. No `PlanState`: a date fact reads no progress.
                "today": crate::views::today_json(
                    config,
                    &crate::evaluator::EvalContext::new(config, None),
                ),
                // The identity washes (the four rooms, assigned over the type a
                // plan marks `colorRole: "identity"`, in the plan's own order) —
                // one assignment, so a course chip is the same colour on Today,
                // in a table and on the plan.
                "washes": crate::today::identity_washes(config),
                "types": config
                    .types
                    .iter()
                    .map(|(name, def)| {
                        (
                            name.clone(),
                            json!({
                                "icon": def.icon,
                                "trackable": def.trackable,
                                "pipeline": def.pipeline,
                                // Which kinds carry identity (the four washes) —
                                // the plan's own declaration, read by the screens
                                // that paint a subject.
                                "colorRole": def.color_role,
                                // The private-record rule (§6 Phase 7): the
                                // sharing screen shows which kinds a profile
                                // leaves out, so it reads the flag as data.
                                "private": def.private,
                                // The parent edge: a tree block nests by it, and
                                // a child type's records are reachable through it
                                // (§3.1), so a projection without it would make
                                // the renderer guess at hierarchy from field names.
                                "parent": def.parent,
                                // The **whole** FieldDef, not a summary: the
                                // header menu's *Copy as JSON* must hand back
                                // the definition the engine holds (§4.8 P3),
                                // which a four-key projection would truncate.
                                "fields": def
                                    .fields
                                    .iter()
                                    .map(|field| {
                                        serde_json::to_value(field)
                                            .unwrap_or(serde_json::Value::Null)
                                    })
                                    .collect::<Vec<serde_json::Value>>(),
                            }),
                        )
                    })
                    .collect::<BTreeMap<String, serde_json::Value>>(),
            }))
        }
        // Phase 7: the theme catalogue, the resolved appearance and the design
        // doctor. Three reads because they answer three questions — which themes
        // exist, what the active one resolves to (the UI applies exactly this),
        // and what §5's lints say about the plan.
        "theme.list" => {
            let themes = crate::theme::list(resources_dir).map_err(DispatchError::Io)?;
            let active = config
                .and_then(|config| config.appearance.as_ref())
                .and_then(|appearance| appearance.theme.clone())
                .unwrap_or_else(|| crate::theme::DEFAULT_THEME.to_string());
            Ok(json!({
                "themes": themes
                    .iter()
                    .map(|theme| json!({
                        "id": theme.id,
                        "name": theme.name,
                        "mode": theme.mode(),
                        "description": theme.description,
                    }))
                    .collect::<Vec<serde_json::Value>>(),
                "active": active,
                "dir": crate::theme::themes_dir(resources_dir).display().to_string(),
            }))
        }
        "appearance.resolve" => {
            let mode = optional(params, "mode").unwrap_or_else(|| "light".into());
            // With no plan there is still an appearance to resolve: the register
            // and the default theme are bundled, and the first run's theme disc
            // presses this command before any plan exists.
            let resolved = match config {
                Some(config) => crate::theme::resolve_for(config, resources_dir, &mode),
                None => crate::theme::resolve_defaults(resources_dir, &mode),
            }
            .map_err(DispatchError::Io)?;
            Ok(serde_json::to_value(resolved).unwrap_or(serde_json::Value::Null))
        }
        "design.check" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("design.check needs a plan".into()));
            };
            let mode = optional(params, "mode").unwrap_or_else(|| "light".into());
            let report = crate::theme::design_check(config, resources_dir, &mode)
                .map_err(DispatchError::Io)?;
            Ok(serde_json::to_value(report).unwrap_or(serde_json::Value::Null))
        }
        "uicheck" => config
            .map(crate::uicheck::dump_json)
            .ok_or_else(|| DispatchError::Usage("uicheck needs a plan".into())),
        // What the plan picker (Appendix C.5's `several`) draws. A read that
        // needs no plan: the webview has no filesystem by capability (§4.10),
        // so the list is the engine's, and the app must be able to enumerate
        // before it has opened anything.
        "plans" => {
            let paths = crate::resources::paths_with_resources(resources_dir.to_path_buf())
                .map_err(DispatchError::Io)?;
            let chosen = config_store::plan_root(plan).ok();
            Ok(json!({
                "plansDir": paths.plans_dir.display().to_string(),
                "plans": list_plans(&paths.plans_dir),
                "plan": chosen.as_ref().map(|selected| selected.0.display().to_string()),
                "layer": chosen.as_ref().map(|selected| selected.1.clone()),
            }))
        }
        // The registry as data: the palette lists it, the menu bar is projected
        // from it, and `SAM --commands --json` prints it — one projection, so a
        // command cannot exist in one surface and not another (§4.8 P2).
        "commands" => {
            let dynamic = config
                .map(|config| command_registry::dynamic_commands(&config.types))
                .unwrap_or_default();
            let mut defs: Vec<serde_json::Value> = command_registry::commands()
                .iter()
                .map(|def| serde_json::to_value(def).unwrap_or(serde_json::Value::Null))
                .collect();
            defs.extend(
                dynamic
                    .iter()
                    .map(|def| serde_json::to_value(def).unwrap_or(serde_json::Value::Null)),
            );
            let bindings = match config {
                Some(config) => crate::uicheck::keybindings(config),
                None => command_registry::DEFAULT_KEYBINDINGS
                    .iter()
                    .map(|(key, command)| (key.to_string(), command.to_string()))
                    .collect(),
            };
            Ok(json!({
                "commands": defs,
                "menu": command_registry::menu_json(&bindings),
                "keybindings": bindings
                    .iter()
                    .map(|(key, command)| json!({ "key": key, "command": command }))
                    .collect::<Vec<serde_json::Value>>(),
            }))
        }
        "schema" => config
            .map(crate::schema_guide::schema_json)
            .ok_or_else(|| DispatchError::Usage("schema needs a plan".into())),
        // The source pane's read (Appendix C.4): the text of exactly one thing —
        // a record by id, or a document by plan-relative path — and, with no
        // target, the list of things that can be pointed at. The list is the
        // engine's (`DOCUMENTS` plus every records file the config declares), so
        // a new type's file appears in the picker with no code change.
        "source" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("source needs a plan".into()));
            };
            match (params.get("file"), params.get("id")) {
                (Some(ParamValue::Str(file)), _) => {
                    let bytes = config.source_files.get(file).ok_or_else(|| {
                        DispatchError::Usage(format!("{file} is not part of this plan"))
                    })?;
                    Ok(json!({
                        "kind": "file",
                        "file": file,
                        "text": String::from_utf8_lossy(bytes),
                    }))
                }
                (_, Some(ParamValue::Str(id))) => {
                    let positioned = doc_edit::record_ops::positioned(config, id)
                        .ok_or_else(|| DispatchError::Usage(format!("no record named \"{id}\"")))?;
                    let text = ResolvedConfig::canonical_line(&positioned.record)
                        .map_err(|error| DispatchError::Io(error.to_string()))?;
                    Ok(json!({
                        "kind": "record",
                        "id": id,
                        "file": positioned.file,
                        "line": positioned.line,
                        "text": text,
                    }))
                }
                _ => {
                    let mut files: Vec<String> = doc_edit::DOCUMENTS
                        .iter()
                        .filter(|document| config.source_files.contains_key(**document))
                        .map(|document| (*document).to_string())
                        .collect();
                    for type_name in config.types.keys() {
                        files.push(format!("content/records/{type_name}.jsonl"));
                    }
                    Ok(json!({ "kinds": ["record", "file"], "files": files }))
                }
            }
        }
        // The pane's pre-commit syntax check (Appendix C.4): every finding,
        // line-precise, through the same strict parser the loader uses — a
        // `JSON.parse` in the frontend would not be the same parser (it accepts
        // duplicate keys and trailing commas that §4.1 refuses).
        "source.check" => {
            let file = require(params, "file")?;
            let content = match params.get("content") {
                Some(ParamValue::Str(text)) => text.clone(),
                Some(ParamValue::Bytes(bytes)) => String::from_utf8_lossy(bytes).to_string(),
                _ => {
                    return Err(DispatchError::Usage(
                        "source.check needs --content: the text to check".into(),
                    ));
                }
            };
            let diagnostics = doc_edit::pane_precheck::diagnostics(&file, &content);
            Ok(json!({
                "diagnostics": diagnostics
                    .iter()
                    .map(|diagnostic| serde_json::to_value(diagnostic).unwrap_or(json!({})))
                    .collect::<Vec<serde_json::Value>>(),
            }))
        }
        // The shipped preset as data — VS Code's *Open Default Settings (JSON)*
        // (§4.8 P2: *"Defaults are readable as data"*). It reads the **bundle**,
        // never the plan, so a student can always see what they started from.
        "preset" => {
            let paths = crate::resources::paths_with_resources(resources_dir.to_path_buf())
                .map_err(DispatchError::Io)?;
            let preset = request_preset(params)?;
            let dir = paths.resources_dir.join("presets").join(&preset);
            let mut documents = serde_json::Map::new();
            for document in doc_edit::DOCUMENTS {
                if let Ok(text) = std::fs::read_to_string(dir.join(document)) {
                    documents.insert(
                        document.to_string(),
                        crate::strict_json::parse(&text, document)
                            .map_err(|error| DispatchError::Io(error.diagnostic.to_string()))?,
                    );
                }
            }
            Ok(json!({ "preset": preset, "documents": documents }))
        }
        // "What exists of kind T" — the query a relation picker needs (Appendix
        // C.1's picker is *"a menu over the target type's records"*), and one
        // that does not require the target type to have a saved view (§3.6: a
        // viewless type is a lint finding, not a reason a link cannot be made).
        "records" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("records needs a plan".into()));
            };
            let type_name = require(params, "type")?;
            let limit = optional(params, "limit")
                .and_then(|text| text.parse::<usize>().ok())
                .unwrap_or(usize::MAX);
            let records: Vec<serde_json::Value> = config
                .records_iter()
                .filter(|record| record.type_ == type_name)
                .take(limit)
                .map(|record| serde_json::to_value(record).unwrap_or(serde_json::Value::Null))
                .collect();
            Ok(json!({
                "type": type_name,
                "count": records.len(),
                "records": records,
            }))
        }
        "view" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("view needs a plan".into()));
            };
            let name = require(params, "name")?;
            // The published snapshot's progress state, and — when a fresh index
            // exists — its candidate sets. `complete()` and the panels read the
            // same state the CLI does: a view resolved through the shell is the
            // view the terminal prints.
            let state = plan.map(crate::views::PlanState::load);
            let context = crate::evaluator::EvalContext::new(config, state.as_ref());
            let index = fresh_index(config, resources_dir);
            match crate::views::resolve_with(config, &name, &context, index.as_ref()) {
                Ok(outcome) => Ok(outcome.json(config)),
                Err(error) => Err(DispatchError::Usage(error.to_string())),
            }
        }
        "reviews.due" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("reviews.due needs a plan".into()));
            };
            let state = plan.map(crate::views::PlanState::load);
            let context = crate::evaluator::EvalContext::new(config, state.as_ref());
            Ok(crate::scheduler::due_json(config, &context))
        }
        // The Today screen's read (UI P2 · U1): the day's facts, in the plan's
        // own order — no ranking, no rewrite (D1). Parsed here so the CLI, the
        // palette and the screen all read one projection.
        // F14's recovery door (G9): the backups the transaction engine retains,
        // newest first, with the retention bound it enforces. The CLI has read
        // these since Phase 2 (`SAM tx.list`); this arm is what lets the app's
        // Recently deleted list show the same facts instead of a second story.
        "tx.list" => {
            let selected = config_store::plan_root(plan)?;
            let (manifests, retention, warning) = transaction::list_backups(&selected.0);
            Ok(json!({
                "backups": manifests
                    .iter()
                    .map(|manifest| json!({
                        "txid": manifest.txid,
                        "date": manifest.date,
                        "summary": manifest.summary,
                        "files": manifest.entries.iter().map(|entry| entry.path.clone()).collect::<Vec<String>>(),
                    }))
                    .collect::<Vec<serde_json::Value>>(),
                "retention": retention,
                "warning": warning,
            }))
        }
        "system.view" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("system.view needs a plan".into()));
            };
            Ok(machine_json(config))
        }
        "today.view" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("today.view needs a plan".into()));
            };
            let window = match optional(params, "window").as_deref() {
                None | Some("day") => crate::today::Window::Day,
                Some("week") => crate::today::Window::Week,
                Some(other) => {
                    return Err(DispatchError::Usage(format!(
                        "--window takes day or week — got \"{other}\""
                    )));
                }
            };
            let limit = optional(params, "limit")
                .map(|text| {
                    text.parse::<usize>().map_err(|_| {
                        DispatchError::Usage(format!(
                            "--limit takes a whole number, not \"{text}\""
                        ))
                    })
                })
                .transpose()?
                .unwrap_or(12);
            let request = crate::today::TodayRequest {
                date: optional(params, "date"),
                window,
                limit,
            };
            let state = plan.map(crate::views::PlanState::load);
            let context = crate::evaluator::EvalContext::new(config, state.as_ref());
            Ok(crate::today::today_view_json(config, &context, &request))
        }
        "metrics" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("metrics needs a plan".into()));
            };
            let state = plan.map(crate::views::PlanState::load);
            let context = crate::evaluator::EvalContext::new(config, state.as_ref());
            Ok(crate::rules::metrics_json(config, &context))
        }
        "search" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("search needs a plan".into()));
            };
            let query = require(params, "query")?;
            let limit = optional(params, "limit")
                .map(|text| {
                    text.parse::<usize>().map_err(|_| {
                        DispatchError::Usage(format!(
                            "--limit takes a whole number, not \"{text}\""
                        ))
                    })
                })
                .transpose()?
                .unwrap_or(crate::index::DEFAULT_SEARCH_LIMIT);
            let engine = optional(params, "engine").unwrap_or_else(|| "auto".into());
            if !matches!(engine.as_str(), "auto" | "index" | "oracle") {
                return Err(DispatchError::Usage(format!(
                    "--engine takes auto, index or oracle — got \"{engine}\""
                )));
            }
            let mut used = "oracle";
            let mut warning: Option<String> = None;
            let ids: Vec<String> = if engine == "oracle" {
                crate::index::oracle_ids(config, &query)
            } else {
                let index = fresh_index(config, resources_dir);
                match index {
                    Some(index) => match index.search_ids(&query, limit) {
                        Ok(found) => {
                            used = "index";
                            let wanted: std::collections::BTreeSet<&str> =
                                found.iter().map(String::as_str).collect();
                            config
                                .records_iter()
                                .filter(|record| wanted.contains(record.id.as_str()))
                                .map(|record| record.id.clone())
                                .take(limit)
                                .collect()
                        }
                        Err(error) => {
                            warning = Some(format!(
                                "the index could not answer ({error}); results come from the in-memory scan"
                            ));
                            crate::index::oracle_ids(config, &query)
                        }
                    },
                    None => {
                        if engine == "index" {
                            warning = Some(
                                "the index is missing or stale; results come from the in-memory scan — run index.rebuild"
                                    .into(),
                            );
                        }
                        crate::index::oracle_ids(config, &query)
                    }
                }
            };
            let found: std::collections::BTreeSet<&str> = ids.iter().map(String::as_str).collect();
            let records: Vec<serde_json::Value> = config
                .records_iter()
                .filter(|record| found.contains(record.id.as_str()))
                .take(limit)
                .map(|record| crate::views::record_json(config, record))
                .collect();
            Ok(json!({
                "query": query,
                "engine": used,
                "count": found.len(),
                "returned": records.len(),
                "records": records,
                "warning": warning,
            }))
        }
        "index.status" => {
            let paths = crate::resources::paths_with_resources(resources_dir.to_path_buf())
                .map_err(DispatchError::Io)?;
            let Some(config) = config else {
                // A status read that needs no plan root still names the path it
                // would use; without one there is nothing to compare a revision
                // against, so `current` stays null.
                let Some(selected) = plan else {
                    return Err(DispatchError::Usage("index.status needs a plan".into()));
                };
                return Ok(crate::index::status_json(
                    selected,
                    &paths.indexes_dir,
                    None,
                ));
            };
            Ok(crate::index::status_json(
                &config.plan_root,
                &paths.indexes_dir,
                Some(config),
            ))
        }
        "index.rebuild" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("index.rebuild needs a plan".into()));
            };
            let paths = crate::resources::paths_with_resources(resources_dir.to_path_buf())
                .map_err(DispatchError::Io)?;
            let report = crate::index::build(&config.plan_root, &paths.indexes_dir, config)
                .map_err(|error| DispatchError::Io(error.to_string()))?;
            Ok(report.json())
        }
        // One expression, from the engine's own evaluator (§4.4). This is the
        // read a picker previews with: a frontend that evaluated the expression
        // itself would be a second implementation with its own null and date
        // rules, and the preview would then disagree with the commit.
        "expr.eval" => {
            let Some(config) = config else {
                return Err(DispatchError::Usage("expr.eval needs a plan".into()));
            };
            let text = require(params, "expr")?;
            let at = "expression";
            let expr = crate::expression::parse(&text, at)
                .map_err(|error| DispatchError::Usage(error.diagnostic.message))?;
            let state = plan.map(crate::views::PlanState::load);
            let context = crate::evaluator::EvalContext::new(config, state.as_ref());
            let record = match optional(params, "id") {
                None => None,
                Some(id) => Some(
                    config
                        .record(&id)
                        .ok_or_else(|| DispatchError::Usage(format!("no record named \"{id}\"")))?
                        .clone(),
                ),
            };
            let mut evaluation =
                crate::evaluator::Evaluation::new(context, crate::expression::MAX_EVALUATION_STEPS);
            match evaluation.evaluate(&expr, record.as_ref()) {
                Ok(value) => {
                    let mut data = serde_json::Map::new();
                    data.insert("expr".into(), json!(text));
                    data.insert("id".into(), json!(record.as_ref().map(|r| r.id.clone())));
                    data.insert("type".into(), json!(l2_kind(&value)));
                    data.insert(
                        "size".into(),
                        json!(match &value {
                            crate::evaluator::L2Value::Records(records) => Some(records.len()),
                            crate::evaluator::L2Value::Values(values) => Some(values.len()),
                            _ => None,
                        }),
                    );
                    data.insert("value".into(), value.json());
                    Ok(serde_json::Value::Object(data))
                }
                Err(error) => Err(DispatchError::Usage(error.diagnostic.message)),
            }
        }
        other => Err(DispatchError::Usage(format!(
            "{other} is an inspection flag without a JSON projection here"
        ))),
    }
}

/// The domain kind of an evaluated value — what a preview shows beside the
/// number so "0" is never confused with "no value" (§4.4: null ≠ zero).
fn l2_kind(value: &crate::evaluator::L2Value) -> &'static str {
    use crate::evaluator::L2Value;
    match value {
        L2Value::Null => "null",
        L2Value::Bool(_) => "bool",
        L2Value::Int(_) | L2Value::Double(_) => "number",
        L2Value::String(_) => "string",
        L2Value::Record(_) => "record",
        L2Value::Records(_) => "records",
        L2Value::Values(_) => "values",
    }
}

/// The `changes` a dispatch outcome carried, for callers that need them after
/// the outcome is consumed by presentation.
pub fn outcome_changes(outcome: &DispatchOutcome) -> &[FileChange] {
    match outcome {
        DispatchOutcome::Write { changes, .. } => changes,
        _ => &[],
    }
}

/// A record's JSON identity and address, for `record.reveal`'s future handler
/// and for the source pane's breadcrumbs (§4.3). Kept beside the dispatcher
/// because the address format is the dispatcher's business.
pub fn record_address(positioned: &PositionedRecord) -> String {
    format!(
        "{}#{}",
        positioned.file,
        crate::diagnostic::escape_pointer_segment(&positioned.record.id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
    }

    /// A writable copy of the seed preset — the plan these tests mutate. The
    /// fixture corpus is input: a test must never write `.sam/` into it, and the
    /// seed is the plan whose types these batches use.
    fn temp_plan() -> PathBuf {
        // `make_txid` serialises a process-wide counter, so two tests in one
        // process never stage into the same directory (`SystemTime` alone
        // collides across threads).
        let dir =
            std::env::temp_dir().join(format!("sam-dispatch-{}", crate::transaction::make_txid()));
        crate::transaction::copy_tree(&resources().join("presets/seed"), &dir)
            .expect("the seed copies");
        dir
    }

    fn run(id: &str, plan: &Path, params: Params) -> Result<DispatchOutcome, DispatchError> {
        let mut request = DispatchRequest::new(id, resources());
        request.plan = Some(plan.to_path_buf());
        request.params = params;
        CommandSession::run(request)
    }

    #[test]
    fn an_unknown_id_is_a_usage_error_naming_the_registry() {
        let error = CommandSession::run(DispatchRequest::new("nope", resources()))
            .err()
            .expect("an unknown id does not run");
        let DispatchError::Usage(message) = error else {
            panic!("an unknown id is a usage error");
        };
        assert!(
            message.contains("--commands"),
            "the message points at the registry"
        );
    }

    /// The first run's theme disc. A machine with no plan has no
    /// `appearance.json` to read, and the register and the default theme are
    /// bundled — so the resolve answers anyway: the one control the rail draws
    /// before a plan must land, not refuse.
    #[test]
    fn appearance_resolves_without_a_plan() {
        let params = Params::from([("mode".to_string(), ParamValue::Str("dark".into()))]);
        let resolved = read_json("appearance.resolve", &params, None, &resources(), None)
            .expect("the bundled appearance resolves with no plan");
        assert_eq!(resolved["resolvedMode"], serde_json::json!("dark"));
        assert_eq!(resolved["textScale"], serde_json::json!(1.0));
        assert!(
            resolved["css"]
                .as_object()
                .is_some_and(|css| !css.is_empty()),
            "the resolved custom properties are the point: {resolved}"
        );
    }

    /// The first run's one write. `plan.new` with no `--preset` creates the
    /// `blank` plan (§3.2): the seed's five documents and **no records** — the
    /// fact the card's copy promises (*"Add your first course"*). The default
    /// was `seed` once, which is exactly the kind of fact that drifts silently.
    #[test]
    fn plan_new_defaults_to_the_blank_preset() {
        let dir =
            std::env::temp_dir().join(format!("sam-plan-new-{}", crate::transaction::make_txid()));
        let _ = std::fs::remove_dir_all(&dir);
        let params = Params::from([(
            "target".to_string(),
            ParamValue::Str(dir.to_string_lossy().into_owned()),
        )]);
        let outcome = run("plan.new", &dir, params).expect("plan.new creates a plan");
        assert!(
            matches!(outcome, DispatchOutcome::Write { .. }),
            "creating a plan is a write"
        );
        for document in [
            "types.json",
            "views.json",
            "rules.json",
            "shell.json",
            "appearance.json",
        ] {
            assert!(
                dir.join("content").join(document).exists(),
                "{document} ships in every plan"
            );
        }
        let records = dir.join("content").join("records");
        let count = std::fs::read_dir(&records).map_or(0, Iterator::count);
        assert_eq!(
            count, 0,
            "the blank plan carries no records — the seed's demo is not copied"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_without_a_batch_source_is_a_usage_error() {
        let plan = temp_plan();
        let error = run("apply", &plan, Params::new())
            .err()
            .expect("apply needs a batch");
        assert!(
            matches!(error, DispatchError::Usage(_)),
            "missing batch surface → usage, not a write"
        );
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn a_stale_pin_conflicts_without_touching_the_plan() {
        let plan = temp_plan();
        let before = crate::config_store::raw_revision(&plan);

        let mut request = DispatchRequest::new("apply", resources());
        request.plan = Some(plan.clone());
        request.if_revision = Some("deadbeefdeadbeef".into());
        request
            .params
            .insert("batch".into(), ParamValue::Str("/dev/null".into()));
        let error = CommandSession::run(request)
            .err()
            .expect("a stale pin conflicts");
        assert!(
            matches!(
                error,
                DispatchError::Transaction(TransactionError::ExternalConflict(_))
            ),
            "a stale pin is a conflict"
        );
        assert_eq!(
            crate::config_store::raw_revision(&plan),
            before,
            "the plan's authoritative bytes were not touched"
        );
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn an_apply_commits_and_a_retry_is_a_no_op() {
        let plan = temp_plan();
        let line = r#"{"fields":{"est":10,"kind":"watch","title":"Dispatch"},"id":"t.d.1","links":{},"schemaVersion":1,"type":"topic"}"#;
        let mut params = Params::new();
        params.insert(
            "batch".into(),
            ParamValue::Bytes(format!("{line}\n").into_bytes()),
        );

        let first = run("apply", &plan, params.clone()).expect("the batch commits");
        let DispatchOutcome::Write {
            commit, dry_run, ..
        } = first
        else {
            panic!("an apply is a write outcome");
        };
        assert!(!dry_run && !commit.txid.is_empty());

        let second = run("apply", &plan, params).expect("the retry succeeds");
        let DispatchOutcome::Write { commit, .. } = second else {
            panic!("an apply is a write outcome");
        };
        assert!(
            commit.txid.is_empty(),
            "a no-op batch never reaches the engine"
        );
        assert_eq!(commit.new_revision, commit.base_revision);

        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn a_dry_run_writes_nothing() {
        let plan = temp_plan();
        let before = crate::config_store::raw_revision(&plan);
        let line = r#"{"fields":{"est":10,"kind":"watch","title":"Dry"},"id":"t.dry.1","links":{},"schemaVersion":1,"type":"topic"}"#;
        let mut params = Params::new();
        params.insert(
            "batch".into(),
            ParamValue::Bytes(format!("{line}\n").into_bytes()),
        );
        params.insert("dry-run".into(), ParamValue::Bool(true));

        let outcome = run("apply", &plan, params).expect("a dry run plans");
        let DispatchOutcome::Write { dry_run, data, .. } = outcome else {
            panic!("a dry run is a write outcome");
        };
        assert!(dry_run);
        assert_eq!(
            data.get("wouldChange").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(crate::config_store::raw_revision(&plan), before);
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[allow(dead_code)]
    fn strings(values: &[&str]) -> ParamValue {
        ParamValue::Strings(values.iter().map(|value| value.to_string()).collect())
    }

    #[test]
    fn a_per_type_id_resolves_to_the_generic_handler() {
        let plan = temp_plan();
        // The plan's parameters ARE the type's field keys (§4.8's own example:
        // `SAM topic.new --title "Bayes" --est 30`).
        let outcome = run(
            "topic.new",
            &plan,
            Params::from([
                ("title".to_string(), ParamValue::Str("Per-type".into())),
                ("est".to_string(), ParamValue::Str("30".into())),
            ]),
        )
        .expect("the generated id dispatches");
        let DispatchOutcome::Write {
            commit, summary, ..
        } = outcome
        else {
            panic!("creating a record is a write");
        };
        assert!(!commit.txid.is_empty(), "the id reached a real handler");
        assert_eq!(summary, "record.new");
        let text = std::fs::read_to_string(plan.join("content/records/topic.jsonl")).unwrap();
        assert!(
            text.lines()
                .any(|line| line.contains(r#""title":"Per-type""#)),
            "the record is in the records file, canonically written"
        );
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn a_paste_is_one_batch_of_distinct_records() {
        let plan = temp_plan();
        let records = serde_json::json!([
            { "title": "Row one", "est": 10 },
            { "title": "Row two", "kind": "read" },
            { "id": "t.pasted.9", "title": "Named row" },
        ]);
        let before = crate::config_store::raw_revision(&plan);
        let outcome = run(
            "topic.paste",
            &plan,
            Params::from([("records".to_string(), ParamValue::Str(records.to_string()))]),
        )
        .expect("the paste commits");
        let DispatchOutcome::Write { data, .. } = outcome else {
            panic!("a paste is a write");
        };
        assert_eq!(
            data["counts"]["added"],
            serde_json::json!(3),
            "three rows, one batch"
        );
        assert_ne!(crate::config_store::raw_revision(&plan), before);

        let text = std::fs::read_to_string(plan.join("content/records/topic.jsonl")).unwrap();
        for id in ["topic.paste.1", "topic.paste.2", "t.pasted.9"] {
            assert!(
                text.lines()
                    .any(|line| line.contains(&format!(r#""id":"{id}""#))),
                "{id} is in the file"
            );
        }
        assert_eq!(
            text.lines()
                .filter(|line| line.contains("topic.paste."))
                .count(),
            2,
            "default ids are distinct — a paste never clobbers its own rows"
        );

        // A second paste continues past the first rather than rewriting it.
        let more = serde_json::json!([{ "title": "Row three" }]);
        let outcome = run(
            "topic.paste",
            &plan,
            Params::from([("records".to_string(), ParamValue::Str(more.to_string()))]),
        )
        .expect("a second paste commits");
        let DispatchOutcome::Write { data, .. } = outcome else {
            panic!("a paste is a write");
        };
        assert_eq!(data["counts"]["added"], serde_json::json!(1));
        let text = std::fs::read_to_string(plan.join("content/records/topic.jsonl")).unwrap();
        assert!(
            text.lines()
                .any(|line| line.contains(r#""id":"topic.paste.3""#))
        );

        // An explicit id that already exists is refused, not silently upserted.
        let error = run(
            "topic.paste",
            &plan,
            Params::from([("records".to_string(), ParamValue::Str(records.to_string()))]),
        )
        .err()
        .expect("a clobbering paste does not commit");
        assert!(
            matches!(error, DispatchError::Usage(message) if message.contains("already exists")),
            "an existing id is refused by name"
        );
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn a_paste_refuses_an_unknown_field_by_name() {
        let plan = temp_plan();
        let records = serde_json::json!([{ "titel": "typo" }]);
        let error = run(
            "topic.paste",
            &plan,
            Params::from([("records".to_string(), ParamValue::Str(records.to_string()))]),
        )
        .err()
        .expect("a typo does not commit");
        let DispatchError::Usage(message) = error else {
            panic!("an unknown field is a usage error naming the type");
        };
        assert!(message.contains("has no field \"titel\""), "{message}");
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn undo_and_redo_restore_bytes_through_the_transaction_path() {
        let plan = temp_plan();
        let file = plan.join("content/records/topic.jsonl");
        let before = std::fs::read(&file).expect("the records file");

        let edited = run(
            "record.setField",
            &plan,
            Params::from([
                ("id".to_string(), ParamValue::Str("t.demo.03".into())),
                ("field".to_string(), ParamValue::Str("title".into())),
                ("value".to_string(), ParamValue::Str("Undo me".into())),
            ]),
        )
        .expect("the edit commits");
        let DispatchOutcome::Write { commit, .. } = edited else {
            panic!("an edit is a write");
        };
        let after_edit = std::fs::read(&file).expect("the records file");
        assert_ne!(before, after_edit, "the edit changed the bytes");

        let mut params = Params::new();
        params.insert("txid".into(), ParamValue::Str(commit.txid.clone()));
        let mut request = DispatchRequest::new("edit.undo", resources());
        request.plan = Some(plan.clone());
        request.params = params.clone();
        // A stale pin is refused before anything is read (exit 3, §4.8 P6).
        request.if_revision = Some("deadbeefdeadbeef".into());
        assert!(matches!(
            CommandSession::run(request).err(),
            Some(DispatchError::Transaction(_))
        ));

        let mut request = DispatchRequest::new("edit.undo", resources());
        request.plan = Some(plan.clone());
        request.params = params;
        request.if_revision = Some(commit.new_revision.clone());
        let undone = CommandSession::run(request).expect("the undo commits");
        let DispatchOutcome::Write { commit: undo, .. } = undone else {
            panic!("an undo is a write");
        };
        assert_eq!(
            std::fs::read(&file).expect("the records file"),
            before,
            "undo restores the before-image byte for byte"
        );

        // Redo is the same mechanism over the undo's own transaction.
        let mut params = Params::new();
        params.insert("txid".into(), ParamValue::Str(undo.txid.clone()));
        let mut request = DispatchRequest::new("edit.redo", resources());
        request.plan = Some(plan.clone());
        request.params = params;
        request.if_revision = Some(undo.new_revision.clone());
        let redone = CommandSession::run(request).expect("the redo commits");
        let DispatchOutcome::Write { .. } = redone else {
            panic!("a redo is a write");
        };
        assert_eq!(
            std::fs::read(&file).expect("the records file"),
            after_edit,
            "redo restores the edit byte for byte"
        );
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn a_named_setting_row_is_the_generic_setter() {
        let plan = temp_plan();
        let outcome = run(
            "study.setDailyTarget",
            &plan,
            Params::from([("value".to_string(), ParamValue::Str("180".into()))]),
        )
        .expect("the named row dispatches");
        let DispatchOutcome::Write { data, .. } = outcome else {
            panic!("a setting is a write");
        };
        assert_eq!(data["setting"], serde_json::json!("dailyTargetMin"));
        let rules = std::fs::read_to_string(plan.join("content/rules.json")).unwrap();
        assert!(rules.contains("dailyTargetMin"), "{rules}");
        let _ = std::fs::remove_dir_all(&plan);
    }

    /// Gate 2's first clause, as a test: **every** declared write reaches a
    /// handler. A def whose arm is missing fails with "has no session handler",
    /// and a registration that only exists in the registry is exactly what the
    /// gate is for — the palette would list a verb that cannot run.
    #[test]
    fn every_declared_write_reaches_a_handler() {
        let plan = temp_plan();
        let config = crate::config_store::load_with(&plan, "explicit", &resources())
            .expect("the copy loads");
        let mut ids: Vec<String> = crate::command_registry::registered_write_ids()
            .iter()
            .map(|id| (*id).to_string())
            .collect();
        ids.extend(
            crate::command_registry::dynamic_commands(&config.types)
                .iter()
                .filter(|def| def.effect == CommandEffect::Write)
                .map(|def| def.id.clone()),
        );
        assert!(
            ids.len() >= 22,
            "the write set is the whole mutation surface"
        );

        for id in ids {
            // Empty parameters: the handler may refuse the *arguments*, but never
            // the id. "has no session handler" is the failure this asserts.
            let error = run(&id, &plan, Params::new()).err();
            if let Some(DispatchError::Usage(message)) = &error {
                assert!(
                    !message.contains("has no session handler"),
                    "{id} is declared as a write and dispatch has no arm for it"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn a_presentation_id_reports_its_destination_without_a_gui() {
        let plan = temp_plan();
        let outcome = run(
            "record.reveal",
            &plan,
            Params::from([("id".to_string(), ParamValue::Str("t.demo.01".into()))]),
        )
        .expect("a presentation id resolves");
        let DispatchOutcome::Presentation { destination } = outcome else {
            panic!("reveal is a presentation action, not a write");
        };
        assert!(destination.contains("source pane"), "{destination}");
        let _ = std::fs::remove_dir_all(&plan);
    }

    #[test]
    fn the_views_read_carries_each_destinations_icon_or_null() {
        let plan = temp_plan();
        let shell = plan.join("content/shell.json");
        let mut document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&shell).expect("shell.json"))
                .expect("shell.json parses");
        document["navigation"][0]["icon"] = serde_json::json!("sun");
        // The preset names icons for its own destinations; this test owns its
        // two cases, so the second entry's icon is cleared rather than the
        // assertion depending on preset data.
        document["navigation"][1]
            .as_object_mut()
            .expect("a navigation entry")
            .remove("icon");
        std::fs::write(&shell, serde_json::to_string_pretty(&document).unwrap()).expect("write");

        let DispatchOutcome::Read(payload) = run("views", &plan, Params::new()).expect("a read")
        else {
            panic!("views is a read");
        };
        let navigation = payload["navigation"]
            .as_array()
            .expect("navigation is a list of destinations");
        assert_eq!(navigation[0]["title"], serde_json::json!("Today"));
        assert_eq!(navigation[0]["icon"], serde_json::json!("sun"));
        assert_eq!(
            navigation[1]["icon"],
            serde_json::Value::Null,
            "a destination that names no icon says so, and the frame picks the fallback"
        );
        let _ = std::fs::remove_dir_all(&plan);
    }
}
