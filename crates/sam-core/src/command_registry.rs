//! Command registry (§4.2, §4.8 Principle 2).
//!
//! Central definition of domain commands with identifier, parameters, effect class,
//! and UI placement declarations. Shared across CLI verbs, palette search, menu bar items,
//! and frontend dispatches.

use std::sync::LazyLock;

use std::collections::BTreeMap;

use serde::Serialize;

use crate::model::TypeDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandEffect {
    Read,
    Write,
    Presentation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    /// CLI spelling, e.g. `--dry-run`.
    pub flag: Option<String>,
    pub required: bool,
    pub positional: bool,
    /// May appear multiple times (Phase 4's `--field`).
    pub repeats: bool,
    pub help: String,
}

impl ParamDef {
    fn new(name: &str, type_: &str, help: &str) -> Self {
        Self {
            name: name.into(),
            type_: type_.into(),
            flag: None,
            required: false,
            positional: false,
            repeats: false,
            help: help.into(),
        }
    }

    fn flag(mut self, flag: &str) -> Self {
        self.flag = Some(flag.into());
        self
    }

    fn required(mut self) -> Self {
        self.required = true;
        self
    }

    fn positional(mut self) -> Self {
        self.positional = true;
        self
    }

    fn repeats(mut self) -> Self {
        self.repeats = true;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDef {
    pub id: String,
    pub title: String,
    pub category: String,
    pub effect: CommandEffect,
    pub params: Vec<ParamDef>,
    /// Documented inspection flags (§4.9).
    pub aliases: Vec<String>,
    pub plan_required: bool,
    pub help: String,
    /// Surface A anchor (§4.8 Principle 2: "Registration must include … UI
    /// placement"). Every def declares one; [`KNOWN_PLACEMENTS`] is the closed
    /// set the UI actually renders, and the Phase 4 parity gate asserts
    /// membership.
    pub ui_placement: String,
    /// The text surface's identity (§4.8: the UI never hides the JSON identity
    /// of what it edits): the document path a write lands in, or an invoke
    /// payload note for a transient action.
    pub json_path: Option<String>,
}

/// Placements the UI really renders (the Phase 4 parity gate asserts
/// membership). Declared here because a def cannot omit one.
pub const KNOWN_PLACEMENTS: [&str; 21] = [
    "sidebar",         // shell navigation entries
    "sidebar.add",     // + Add list designer
    "sidebar.context", // sidebar item context menu
    "system",          // the System surface: the machine's own place (S14)
    "recordTable.toolbar",
    "recordTable.cell",
    "recordTable.rowContext",
    "recordTable.paste",
    "columnMenu",      // every table header
    "columnMenu.plus", // + at the header's right edge
    "settings",
    "sourcePane",
    "palette",        // discoverability net: every command
    "app.window",     // window chrome / global keys
    "app.planPicker", // the start screen: the plans on disk, the starts the bundle
    // ships, and the press that makes the plan — drawn whenever
    // no plan is open
    "view.head",      // the screen head: its layout control and "Edit this view…"
    "view.editor",    // one row per query key and per block, in the view's editor
    "reviews.panel",  // the Reviews panel: stage controls and review buttons
    "progress.panel", // the Progress panel: derived metrics
    "today.screen",   // the Today screen: the day's facts, its controls, the timer
    "composer.frame", // the composer's frame: the drag handle, name chip and its menu (COMPOSER §3.1)
];

fn plan_param() -> ParamDef {
    ParamDef::new("plan", "string", "plan root").flag("--plan")
}

static COMMANDS: LazyLock<Vec<CommandDef>> = LazyLock::new(|| {
    vec![
        // ===== inspection =====
        CommandDef {
            id: "commands".into(),
            title: "Enumerate commands".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("plan", "string", "plan root (adds per-type commands)")
                    .flag("--plan"),
            ],
            aliases: vec!["--commands".into()],
            plan_required: false,
            help: "list the registry: ids, effects, params, placements, json paths".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "selfcheck".into(),
            title: "Engine self-checks".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec!["--selfcheck".into()],
            plan_required: false,
            help: "run engine self-checks against bundled fixtures".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "tokens".into(),
            title: "Resolve tokens".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec!["--tokens".into()],
            plan_required: false,
            help: "resolve the bundled token register".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "rendercheck".into(),
            title: "Dump content tree".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![plan_param().required()],
            aliases: vec!["--rendercheck".into()],
            plan_required: true,
            help: "dump a plan's resolved content tree".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "configcheck".into(),
            title: "Validate plan".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![plan_param()],
            aliases: vec!["--configcheck".into()],
            plan_required: true,
            help: "load and validate a plan, path-precisely".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "views".into(),
            title: "List the saved views and the sidebar".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec!["--views".to_string()],
            plan_required: true,
            help: "the type system's views, their layouts, and the shell's navigation".into(),
            ui_placement: "sidebar".into(),
            json_path: Some("content/views.json#/views".into()),
        },
        CommandDef {
            id: "uicheck".into(),
            title: "Dump the declared UI structure".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec!["--uicheck".to_string()],
            plan_required: true,
            help: "Gate 2's structural half: every control, its registry id, its placement".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "paths".into(),
            title: "Plan paths & revision".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![plan_param()],
            aliases: vec!["--paths".into()],
            plan_required: false,
            help: "plan root, source revision, index path — answered with no plan too".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "preset".into(),
            title: "Show the shipped preset".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![ParamDef::new("name", "string", "which preset; the blank plan by default")
                .flag("--name")],
            aliases: vec!["--preset".into()],
            plan_required: false,
            help: "the bundled preset's documents as data — the read-only default".into(),
            ui_placement: "settings".into(),
            json_path: None,
        },
        CommandDef {
            id: "plans".into(),
            title: "List plans".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec!["--plans".into()],
            plan_required: false,
            help: "the plans directory and every plan in it — what the picker draws".into(),
            ui_placement: "app.planPicker".into(),
            json_path: None,
        },
        CommandDef {
            id: "schema".into(),
            title: "Emit the type system".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![plan_param()],
            aliases: vec!["--schema".into()],
            plan_required: true,
            help: "the active type system generated from the TypeDefs — the agent never guesses field names".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        CommandDef {
            id: "guide".into(),
            title: "Emit the authoring guide".into(),
            category: "inspection".into(),
            effect: CommandEffect::Read,
            params: vec![plan_param()],
            aliases: vec!["--guide".into()],
            plan_required: true,
            help: "authoring spec plus worked examples generated from the active schema".into(),
            ui_placement: "palette".into(),
            json_path: None,
        },
        // ===== queries =====
        CommandDef {
            id: "records".into(),
            title: "Read records of a kind".into(),
            category: "queries".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("type", "string", "the kind of thing").flag("--type").required(),
                ParamDef::new("limit", "number", "return at most this many")
                    .flag("--limit")
                    .required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "every record of one kind — what a relation picker lists".into(),
            ui_placement: "recordTable.cell".into(),
            json_path: Some("content/records/<type>.jsonl".into()),
        },
        CommandDef {
            id: "view".into(),
            title: "Resolve a saved view".into(),
            category: "queries".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("name", "string", "saved view id")
                    .required()
                    .positional(),
                plan_param(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "a saved view's records as deterministic JSON — the primary read interface".into(),
            ui_placement: "sidebar".into(),
            json_path: None,
        },
        // The evaluator, exposed: a live preview needs the engine's own answer,
        // and a second evaluator in the frontend would be a second set of
        // semantics (§4.4). It reads; it never writes.
        CommandDef {
            id: "expr.eval".into(),
            title: "Evaluate an expression".into(),
            category: "queries".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("expr", "string", "an L2 expression (§4.4)")
                    .required()
                    .positional(),
                ParamDef::new("id", "string", "the record to evaluate it against")
                    .flag("--id"),
                plan_param(),
            ],
            aliases: vec![],
            plan_required: false,
            help: "one expression's value, from the engine's evaluator — what a picker previews with".into(),
            ui_placement: "view.editor".into(),
            json_path: None,
        },
        // ===== plan =====
        CommandDef {
            id: "plan.new".into(),
            title: "Create a plan".into(),
            category: "plan".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new(
                    "name",
                    "string",
                    "plan name under the default plans directory",
                )
                .flag("--name"),
                ParamDef::new("target", "string", "explicit directory to create").flag("--plan"),
                ParamDef::new("preset", "string", "which preset; the blank plan by default")
                    .flag("--preset"),
            ],
            aliases: vec![],
            plan_required: false,
            help: "materialize a bundled preset into a new writable plan".into(),
            ui_placement: "app.planPicker".into(),
            json_path: Some("plans/<name>/ (a complete preset copy, §4.1 ownership)".into()),
        },
        // ===== records =====
        CommandDef {
            id: "apply".into(),
            title: "Apply a record batch".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new(
                    "batch",
                    "string",
                    "batch file path, or '-' for JSONL on stdin",
                )
                .required()
                .positional(),
                ParamDef::new(
                    "dry-run",
                    "boolean",
                    "preview the record-level diff without writing",
                )
                .flag("--dry-run"),
                ParamDef::new(
                    "if-revision",
                    "string",
                    "commit only if the plan is still at this revision",
                )
                .flag("--if-revision"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "idempotent record upserts through the transaction engine — the bulk door".into(),
            ui_placement: "recordTable.paste".into(),
            json_path: Some("content/records/*.jsonl".into()),
        },
        // ===== recovery =====
        CommandDef {
            id: "tx.list".into(),
            title: "List transaction backups".into(),
            category: "recovery".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: true,
            help: "retained before-images with their revisions and summaries".into(),
            ui_placement: "system".into(),
            json_path: None,
        },
        CommandDef {
            id: "tx.restore".into(),
            title: "Restore a backup".into(),
            category: "recovery".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("txid", "string", "backup id from tx.list")
                    .required()
                    .positional(),
                ParamDef::new("dry-run", "boolean", "preview the restoration").flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "return the plan to a backup's before-state, previewed".into(),
            ui_placement: "system".into(),
            json_path: Some("<affected files>".into()),
        },

        // ===== records, types, columns, settings, the text door (Phase 4) =====
        CommandDef {
            id: "record.new".into(),
            title: "New record".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("type", "string", "the kind to create").flag("--type").required(),
                ParamDef::new("id", "string", "stable id; omitted takes the engine's next free")
                    .flag("--id"),
                ParamDef::new("value", "string", "one field as key=value; repeatable")
                    .flag("--value")
                    .repeats(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "append one record of <type>, validated whole-plan".into(),
            ui_placement: "recordTable.toolbar".into(),
            json_path: Some("content/records/<type>.jsonl".into()),
        },
        CommandDef {
            id: "record.paste".into(),
            title: "Paste records".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("type", "string", "the kind every row becomes")
                    .flag("--type")
                    .required(),
                ParamDef::new("records", "string", "a JSON array of row objects")
                    .flag("--records")
                    .required(),
                ParamDef::new("dry-run", "boolean", "preview the batch").flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "the bulk door: many rows, one batch, one validation, one transaction".into(),
            ui_placement: "recordTable.paste".into(),
            json_path: Some("content/records/<type>.jsonl".into()),
        },
        CommandDef {
            id: "record.setField".into(),
            title: "Set a field".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("id", "string", "the record").flag("--id").required(),
                ParamDef::new("field", "string", "the FieldDef key").flag("--field").required(),
                ParamDef::new("value", "string", "the value; `null` clears").flag("--value"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one field (or relation link) of one record".into(),
            ui_placement: "recordTable.cell".into(),
            json_path: Some("content/records/<type>.jsonl#<id>/fields/<key>".into()),
        },
        CommandDef {
            id: "record.delete".into(),
            title: "Delete a record".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("id", "string", "the record").flag("--id").required(),
                ParamDef::new("policy", "string", "reject · unlink · cascade").flag("--policy"),
                ParamDef::new("dry-run", "boolean", "preview what would go").flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "delete one record, refusing while anything still links to it".into(),
            ui_placement: "recordTable.rowContext".into(),
            json_path: Some("content/records/<type>.jsonl".into()),
        },
        CommandDef {
            id: "column.new".into(),
            title: "New column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("kind", "string", "a §3.4 field type").flag("--kind"),
                ParamDef::new("label", "string", "the human label; the key stays").flag("--label"),
                ParamDef::new("options", "string", "select/multiSelect choices, comma-separated")
                    .flag("--options"),
                ParamDef::new("to", "string", "the relation's target type").flag("--to"),
                ParamDef::new("expr", "string", "the formula body").flag("--expr"),
                ParamDef::new("before", "string", "insert before this key").flag("--before"),
                ParamDef::new("required", "boolean", "the field is required").flag("--required"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "add one typed field; form, table, validation and agent schema follow".into(),
            ui_placement: "columnMenu.plus".into(),
            json_path: Some("content/types.json#/types/<type>/fields".into()),
        },
        CommandDef {
            id: "column.rename".into(),
            title: "Rename a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("label", "string", "the new label; empty removes it")
                    .flag("--label"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "change a column's LABEL — the key never changes (§4.8 P1)".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/types.json#/types/<type>/fields/<key>".into()),
        },
        CommandDef {
            id: "column.retype".into(),
            title: "Retype a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("kind", "string", "the new field type").flag("--kind").required(),
                ParamDef::new("options", "string", "choices for a select kind").flag("--options"),
                ParamDef::new("expr", "string", "the formula body").flag("--expr"),
                ParamDef::new("dry-run", "boolean", "preview the conversions").flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "migrate a populated column's values, previewed, with its inverse".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/types.json#/types/<type>/fields/<key>".into()),
        },
        CommandDef {
            id: "column.choices".into(),
            title: "Edit column choices".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("options", "string", "comma-separated").flag("--options").required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "replace a select's options; records must still conform".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/types.json#/types/<type>/fields/<key>/options".into()),
        },
        CommandDef {
            id: "column.duplicate".into(),
            title: "Duplicate a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("as", "string", "the new key; defaults to <key>2").flag("--as"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "copy one FieldDef under a free key".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/types.json#/types/<type>/fields".into()),
        },
        CommandDef {
            id: "column.hide".into(),
            title: "Hide a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("view", "string", "the saved view").flag("--view").required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "drop one column from ONE view — hide is a property of the view".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/views.json#/views/<view>/columns".into()),
        },
        CommandDef {
            id: "column.show".into(),
            title: "Show a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("view", "string", "the saved view").flag("--view").required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "put a hidden column back into one view".into(),
            ui_placement: "columnMenu.plus".into(),
            json_path: Some("content/views.json#/views/<view>/columns".into()),
        },
        CommandDef {
            id: "column.reorder".into(),
            title: "Move a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("before", "string", "the key it moves in front of")
                    .flag("--before")
                    .required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "move one column before another in the type's field order".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/types.json#/types/<type>/fields".into()),
        },
        CommandDef {
            id: "column.delete".into(),
            title: "Delete a column".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("spec", "string", "<type>.<key>").required().positional(),
                ParamDef::new("dry-run", "boolean", "preview the data it would remove")
                    .flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "remove a field, its data in every record, and every view reference".into(),
            ui_placement: "columnMenu".into(),
            json_path: Some("content/types.json#/types/<type>/fields".into()),
        },
        // ===== views and screens (Phase 5) =====
        //
        // A saved view is a query plus a shape; a screen is a saved view whose
        // `blocks` compose other saved views. Every write below names the JSON
        // key it lands on, and `view.setLayout`'s `--block` is the design's
        // *Table · Board · Calendar* control: a record block's kind **is** its
        // layout, so the control is the same command either way.
        CommandDef {
            id: "view.setLayout".into(),
            title: "Show as…".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("layout", "string", "list · table · board · timeline · calendar · tree · cardGrid · graph")
                    .required()
                    .flag("--layout"),
                ParamDef::new("block", "number", "a block index, to change that block's kind")
                    .flag("--block"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "draw this view as another of §3.6's layouts — a control, not a screen".into(),
            ui_placement: "view.head".into(),
            json_path: Some("content/views.json#/views/<name>/layout".into()),
        },
        CommandDef {
            id: "view.setFilter".into(),
            title: "Filter a view".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("filter", "string", "an L2 expression; empty clears it")
                    .flag("--filter"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one saved view's filter — the same L2 §4.4 evaluates".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/filter".into()),
        },
        CommandDef {
            id: "view.setSort".into(),
            title: "Sort a view".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("sort", "string", "comma-separated keys, `-` for descending")
                    .flag("--sort"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one saved view's sort keys (stable, with the id tie-breaker)".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/sort".into()),
        },
        CommandDef {
            id: "view.setGroup".into(),
            title: "Group a view".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("group", "string", "an L2 expression; empty clears it")
                    .flag("--group"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one saved view's group key — what a board's columns are".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/group".into()),
        },
        CommandDef {
            id: "view.setLimit".into(),
            title: "Limit a view".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("limit", "number", "how many records to draw; empty clears it")
                    .flag("--limit"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one saved view's limit".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/limit".into()),
        },
        CommandDef {
            id: "view.setColumns".into(),
            title: "Choose columns".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("columns", "string", "comma-separated keys; empty shows every column")
                    .flag("--columns"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one saved view's column list (column.hide is the one-column door)".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/columns".into()),
        },
        CommandDef {
            id: "view.setComponents".into(),
            title: "Compose a screen".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("surfaces", "string", "comma-separated surface keys, in the order they stack")
                    .flag("--surfaces"),
                ParamDef::new("spans", "string", "one width per surface, in the same order: 1 is half the row, 2 the whole row")
                    .flag("--spans"),
                ParamDef::new("clear", "boolean", "remove the composition — the screen falls back to its panel, or to blocks")
                    .flag("--clear"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one screen's whole component list in one transaction — the student's own arrangement (COMPOSER §2)".into(),
            ui_placement: "composer.frame".into(),
            json_path: Some("content/views.json#/views/<name>/components".into()),
        },
        CommandDef {
            id: "view.block.add".into(),
            title: "Add a block".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "the view the block joins").required().positional(),
                ParamDef::new("kind", "string", "a block kind: a layout, or stat · chart · callout · columns · conditional · repeat")
                    .required()
                    .flag("--kind"),
                ParamDef::new("source", "string", "the saved view it draws or folds; default: this view")
                    .flag("--source"),
                ParamDef::new("parent", "string", "the container to insert into, e.g. 0.blocks; default: the view's top level")
                    .flag("--parent"),
                ParamDef::new("index", "number", "insert before this block; default: append")
                    .flag("--index"),
                ParamDef::new("value", "string", "one initial key as key=value; repeatable, e.g. text=Hello")
                    .flag("--value")
                    .repeats(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "add one block to a screen — the view becomes composed rather than single".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/blocks".into()),
        },
        CommandDef {
            id: "view.block.set".into(),
            title: "Set a block key".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("index", "number", "which block at the top level").flag("--index"),
                ParamDef::new("path", "string", "the block's address when it nests, e.g. 0.blocks.2")
                    .flag("--path"),
                ParamDef::new("key", "string", "the block key, e.g. text · expr · reduce · when · view")
                    .required()
                    .flag("--key"),
                ParamDef::new("value", "string", "JSON, or a string; empty removes the key")
                    .flag("--value"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one key of one block — the same JSON the file holds".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/blocks/<index>".into()),
        },
        CommandDef {
            id: "view.block.remove".into(),
            title: "Remove a block".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("index", "number", "which block at the top level").flag("--index"),
                ParamDef::new("path", "string", "the block's address when it nests, e.g. 0.blocks.2")
                    .flag("--path"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "remove one block; removing the last returns the view to a single list".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/blocks".into()),
        },
        CommandDef {
            id: "view.block.move".into(),
            title: "Move a block".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("path", "string", "the block's address, e.g. 0.blocks.2")
                    .required()
                    .flag("--path"),
                ParamDef::new("parent", "string", "the container it lands in, e.g. 0.blocks; default: the top level")
                    .flag("--parent"),
                ParamDef::new("index", "number", "where it lands, counted after it leaves; default: append")
                    .flag("--index"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "move one block to a position in a container — the tree's insert/reorder/nest".into(),
            ui_placement: "view.editor".into(),
            json_path: Some("content/views.json#/views/<name>/blocks".into()),
        },
        CommandDef {
            id: "type.new".into(),
            title: "New kind".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "what you want to track").flag("--name").required(),
                ParamDef::new("id", "string", "type id; defaults to a slug of the name")
                    .flag("--id"),
                ParamDef::new("icon", "string", "an icon name — data, not a symbol")
                    .flag("--icon"),
                ParamDef::new("parent", "string", "the parent type").flag("--parent"),
                ParamDef::new("trackable", "boolean", "has progress").flag("--trackable"),
                ParamDef::new("pipeline", "string", "the stage machine").flag("--pipeline"),
                ParamDef::new("field", "string", "starter column as key:kind[:a|b]")
                    .flag("--field")
                    .repeats(),
            ],
            aliases: vec!["list.new-kind".to_string()],
            plan_required: true,
            help: "create a kind: TypeDef + default view + sidebar entry, one transaction".into(),
            ui_placement: "sidebar.add".into(),
            json_path: Some("content/types.json#/types".into()),
        },
        CommandDef {
            id: "list.new".into(),
            title: "New list".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "the list's title in the sidebar")
                    .flag("--name")
                    .required(),
                ParamDef::new("type", "string", "the kind it shows; absent creates a composed screen instead")
                    .flag("--type"),
                ParamDef::new("id", "string", "view id; defaults to a slug").flag("--id"),
                ParamDef::new("layout", "string", "list · table · board · calendar …")
                    .flag("--layout"),
                ParamDef::new("icon", "string", "the glyph the sidebar entry draws")
                    .flag("--icon"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "add a saved view of an existing kind, plus its sidebar entry — or, with no kind, an empty composed screen (COMPOSER §2.2).".into(),
            ui_placement: "sidebar.add".into(),
            json_path: Some("content/views.json#/views".into()),
        },
        CommandDef {
            id: "view.setPanel".into(),
            title: "Choose a screen".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
                ParamDef::new("panel", "string", "the designed screen this destination draws; empty returns it to its blocks")
                    .flag("--panel"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "give a destination one of the designed screens (today · plan · subjects · practice · mocks · library · notes · reviews · progress)".into(),
            ui_placement: "system".into(),
            json_path: Some("content/views.json#/views/<name>/panel".into()),
        },
        CommandDef {
            id: "record.defer".into(),
            title: "Move a backlog".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("ids", "string", "the records to move; repeat --id once per record")
                    .flag("--id")
                    .repeats()
                    .required(),
                ParamDef::new("date", "string", "the day they move to, YYYY-MM-DD")
                    .flag("--date")
                    .required(),
                ParamDef::new("field", "string", "the date field each kind carries; default focus")
                    .flag("--field"),
                ParamDef::new("reviews", "boolean", "also move the scheduler's due date, review history untouched")
                    .flag("--reviews"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "move many records to one day in one transaction — the catch-up after a gap".into(),
            ui_placement: "reviews.panel".into(),
            json_path: Some("content/records/<kind>.jsonl".into()),
        },
        CommandDef {
            id: "type.delete".into(),
            title: "Delete a kind".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "the kind's id").required().positional(),
                ParamDef::new("mode", "string", "records deletes the kind, its records and the links to them; schema refuses while any record exists")
                    .flag("--mode"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "remove a kind and every reference to it — records, relations, views, destinations, formulas".into(),
            ui_placement: "system".into(),
            json_path: Some("content/types.json#/types/<name>".into()),
        },
        CommandDef {
            id: "view.delete".into(),
            title: "Delete a view".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "saved view id").required().positional(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "remove a view, its destinations and every block or metric that drew it; a kind left with none keeps a default".into(),
            ui_placement: "system".into(),
            json_path: Some("content/views.json#/views/<name>".into()),
        },
        CommandDef {
            id: "list.set".into(),
            title: "Name a destination".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("view", "string", "the view the destination opens").required().positional(),
                ParamDef::new("title", "string", "the name to give it").flag("--title"),
                ParamDef::new("find-title", "string", "which entry, when one view is on the rail twice")
                    .flag("--find-title"),
                ParamDef::new("icon", "string", "the glyph it draws (data, from the plan's own vocabulary)")
                    .flag("--icon"),
                ParamDef::new("clear-icon", "boolean", "draw the app's own fallback instead").flag("--clear-icon"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "rename a rail entry, or give it an icon — the two data fields a destination has".into(),
            ui_placement: "system".into(),
            json_path: Some("content/shell.json#/navigation".into()),
        },
        CommandDef {
            id: "list.delete".into(),
            title: "Remove a destination".into(),
            category: "views".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "the view the destination opens").required().positional(),
                ParamDef::new("title", "string", "the destination's title, when one view is on the rail twice")
                    .flag("--title"),
                ParamDef::new("keepView", "boolean", "true (the default) leaves the view in the plan as a saved view; the terminal spells the destructive case --discard-view, and the JSON door sends false")
                    .flag("--keep-view"),
                ParamDef::new("discard-view", "boolean", "delete the view too, not just its destination")
                    .flag("--discard-view"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "remove one rail entry; the view survives unless keepView is false".into(),
            ui_placement: "system".into(),
            json_path: Some("content/shell.json#/navigation".into()),
        },
        CommandDef {
            id: "settings.set".into(),
            title: "Change a setting".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("key", "string", "the setting's config key")
                    .flag("--key")
                    .required(),
                ParamDef::new("value", "string", "the value; `null` returns to the default")
                    .flag("--value")
                    .required(),
            ],
            aliases: vec!["study.setDailyTarget".into(), "study.setTimezone".into()],
            plan_required: true,
            help: "write one row of the generated settings screen (rules.json#/study, #/scheduler)".into(),
            ui_placement: "settings".into(),
            json_path: Some("content/rules.json".into()),
        },
        // ===== appearance, profiles and the design doctor (Phase 7) =====
        CommandDef {
            id: "theme.list".into(),
            title: "List the themes".into(),
            category: "settings".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: false,
            help: "every theme in the resources directory, with the plan's pinned one".into(),
            ui_placement: "settings".into(),
            json_path: None,
        },
        CommandDef {
            id: "theme.set".into(),
            title: "Change the theme".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("id", "string", "the theme id").flag("--id").required(),
            ],
            aliases: vec!["appearance.theme".into()],
            plan_required: true,
            help: "pin a theme: content/appearance.json#/theme, resolved from themes/<id>.json".into(),
            ui_placement: "settings".into(),
            json_path: Some("content/appearance.json#/theme".into()),
        },
        CommandDef {
            id: "appearance.resolve".into(),
            title: "Resolve the appearance".into(),
            category: "settings".into(),
            effect: CommandEffect::Read,
            params: vec![ParamDef::new(
                "mode",
                "string",
                "the machine's light|dark preference; a pinned theme overrides it",
            )
            .flag("--mode")],
            aliases: vec![],
            plan_required: false,
            help: "the pinned theme, its mode, the text scale and every resolved CSS custom property".into(),
            ui_placement: "settings".into(),
            json_path: None,
        },
        CommandDef {
            id: "appearance.setOverride".into(),
            title: "Override a token".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("key", "string", "the token path, e.g. color.surface.backdrop")
                    .flag("--key")
                    .required(),
                ParamDef::new("value", "string", "the new value").flag("--value").required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one token override into content/appearance.json#/overrides".into(),
            ui_placement: "settings".into(),
            json_path: Some("content/appearance.json#/overrides".into()),
        },
        CommandDef {
            id: "appearance.clearOverride".into(),
            title: "Reset a token override".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![ParamDef::new("key", "string", "the token path to reset")
                .flag("--key")
                .required()],
            aliases: vec!["appearance.reset".into()],
            plan_required: true,
            help: "remove one override — the theme's value returns".into(),
            ui_placement: "settings".into(),
            json_path: Some("content/appearance.json#/overrides".into()),
        },
        CommandDef {
            id: "appearance.setTextScale".into(),
            title: "Change the text size".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![ParamDef::new("value", "number", "a multiple, 0.8–2.0; 1 is the design's scale")
                .flag("--value")
                .required()],
            aliases: vec![],
            plan_required: true,
            help: "scale every font-size token: content/appearance.json#/textScale".into(),
            ui_placement: "settings".into(),
            json_path: Some("content/appearance.json#/textScale".into()),
        },
        CommandDef {
            id: "design.check".into(),
            title: "Run the design doctor".into(),
            category: "settings".into(),
            effect: CommandEffect::Read,
            params: vec![ParamDef::new("mode", "string", "the machine's light|dark preference")
                .flag("--mode")],
            aliases: vec![],
            plan_required: true,
            help: "§5's lints over resolved colours, types and labels — errors, advisories, waivers".into(),
            ui_placement: "settings".into(),
            json_path: None,
        },
        CommandDef {
            id: "type.setPrivate".into(),
            title: "Make a kind private".into(),
            category: "schema".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("type", "string", "the kind").flag("--type").required(),
                ParamDef::new("private", "boolean", "true excludes its records from a shared profile")
                    .flag("--private"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "the private-record rule: personal kinds never leave in a *.samprofile by default".into(),
            ui_placement: "settings".into(),
            json_path: Some("content/types.json#/types/<type>/private".into()),
        },
        CommandDef {
            id: "profile.export".into(),
            title: "Export a profile".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("out", "string", "where to write the *.samprofile document")
                    .flag("--out")
                    .required(),
                ParamDef::new(
                    "personal",
                    "boolean",
                    "include private records and progress state",
                )
                .flag("--personal"),
                ParamDef::new("name", "string", "what the profile calls itself").flag("--name"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write a shareable profile: schema, views, rules, shell and public records".into(),
            ui_placement: "settings".into(),
            json_path: Some("outside the plan — a *.samprofile document".into()),
        },
        CommandDef {
            id: "profile.import".into(),
            title: "Import a profile".into(),
            category: "settings".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("file", "string", "the *.samprofile document")
                    .flag("--file")
                    .required(),
                ParamDef::new("name", "string", "the plan's name under the plans directory")
                    .flag("--name"),
                // `--plan` is the *active* plan selector for every command, so
                // a creation target gets its own spelling: `--into <directory>`
                // is unambiguous and binds like every other parameter.
                ParamDef::new("target", "string", "an explicit directory to import into")
                    .flag("--into"),
            ],
            aliases: vec![],
            plan_required: false,
            help: "materialize a profile into a new writable plan, validated before it is visible".into(),
            ui_placement: "settings".into(),
            json_path: Some("plans/<name>/ (a complete profile copy, §4.1 ownership)".into()),
        },
        // ===== pipelines, reviews and structure (Phase 6) =====
        CommandDef {
            id: "record.advanceStage".into(),
            title: "Advance a stage".into(),
            category: "reviews".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("id", "string", "the record")
                    .flag("--id")
                    .required(),
                ParamDef::new(
                    "stage",
                    "string",
                    "the stage to record; omitted advances to the next",
                )
                .flag("--stage"),
                ParamDef::new("problems", "string", "solved problems — the proof gate's evidence")
                    .flag("--problems"),
                ParamDef::new("reason", "string", "the anchor-skip reason")
                    .flag("--reason"),
                ParamDef::new("signal", "string", "a signal to keep for later")
                    .flag("--signal"),
                ParamDef::new("at", "string", "the instant to record, ISO-8601 UTC")
                    .flag("--at"),
                ParamDef::new("dry-run", "boolean", "preview the transition")
                    .flag("--dry-run"),
            ],
            aliases: vec!["stage.advance".into()],
            plan_required: true,
            help: "record (or backtrack) one pipeline stage with its evidence, in one transaction"
                .into(),
            ui_placement: "reviews.panel".into(),
            json_path: Some("state/state.json#/progress/<id>".into()),
        },
        CommandDef {
            id: "record.logReview".into(),
            title: "Log a review".into(),
            category: "reviews".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("id", "string", "the record")
                    .flag("--id")
                    .required(),
                ParamDef::new("rating", "string", "again | hard | good | easy")
                    .flag("--rating")
                    .required(),
                ParamDef::new("at", "string", "the instant, ISO-8601 UTC (defaults to now)")
                    .flag("--at"),
                ParamDef::new("dry-run", "boolean", "preview the schedule")
                    .flag("--dry-run"),
            ],
            aliases: vec!["review.log".into()],
            plan_required: true,
            help: "log a timestamped rating; the scheduler computes the next due date".into(),
            ui_placement: "reviews.panel".into(),
            json_path: Some("state/state.json#/progress/<id>/review".into()),
        },
        CommandDef {
            id: "type.setPipeline".into(),
            title: "Change the study method".into(),
            category: "reviews".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("type", "string", "the type whose machine changes")
                    .flag("--type")
                    .required(),
                ParamDef::new("pipeline", "string", "the new pipeline, e.g. check")
                    .flag("--pipeline")
                    .required(),
                ParamDef::new("map", "string", "old=new, one per recorded stage; repeatable")
                    .flag("--map")
                    .repeats(),
                ParamDef::new("fresh", "boolean", "start a new active state, keeping history")
                    .flag("--fresh"),
                ParamDef::new("dry-run", "boolean", "preview the history mapping")
                    .flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "switch a type between pipelines with a previewed stage mapping or a fresh state"
                .into(),
            ui_placement: "reviews.panel".into(),
            json_path: Some("content/types.json#/types/<type>/pipeline".into()),
        },
        CommandDef {
            id: "record.move".into(),
            title: "Move to another kind".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("id", "string", "the record")
                    .flag("--id")
                    .required(),
                ParamDef::new("to", "string", "the target type")
                    .flag("--to")
                    .required(),
                ParamDef::new("parent", "string", "the parent record to attach it to")
                    .flag("--parent"),
                ParamDef::new("dry-run", "boolean", "preview what the move drops")
                    .flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "move one record across types; values the target cannot hold are reported".into(),
            ui_placement: "recordTable.rowContext".into(),
            json_path: Some("content/records/<to>.jsonl".into()),
        },
        CommandDef {
            id: "records.renumber".into(),
            title: "Renumber".into(),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("type", "string", "the type to renumber, e.g. unit")
                    .flag("--type")
                    .required(),
                ParamDef::new("field", "string", "the number field (default index)")
                    .flag("--field"),
                ParamDef::new("parent", "string", "renumber only one parent's children")
                    .flag("--parent"),
                ParamDef::new("start", "string", "the first number (default 1)")
                    .flag("--start"),
                ParamDef::new("dry-run", "boolean", "preview the numbering")
                    .flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "assign consecutive numbers in the records' current order, in one transaction"
                .into(),
            ui_placement: "recordTable.toolbar".into(),
            json_path: Some("content/records/<type>.jsonl".into()),
        },
        CommandDef {
            id: "reviews.due".into(),
            title: "Read the review queue".into(),
            category: "reviews".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: true,
            help: "what is due, what is coming, and what waits on a stage — as data".into(),
            ui_placement: "reviews.panel".into(),
            json_path: Some("state/state.json#/progress".into()),
        },
        // The Today screen's read (UI P2 · U1): facts, never a ranking. It is a
        // read like any other, so the CLI prints the same projection the screen
        // draws — and an agent can ask what the day holds without a window.
        CommandDef {
            id: "today.view".into(),
            title: "Read today".into(),
            category: "queries".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("date", "string", "read another day, as YYYY-MM-DD")
                    .flag("--date"),
                ParamDef::new("window", "string", "day | week (default day)")
                    .flag("--window"),
                ParamDef::new("limit", "string", "items per group (default 12)")
                    .flag("--limit"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "the day's facts — planned, late, due, next in the plan, stale, coming up".into(),
            ui_placement: "today.screen".into(),
            json_path: Some("state/state.json#/progress".into()),
        },
        // The System surface's one read (UI P3 · U6): the whole machine in one
        // payload, with the dependents each row has to state. The counts come
        // from the engine, not from the UI's guesses, so "used by n views" is
        // the same number the parity gate and a script see.
        CommandDef {
            id: "system.view".into(),
            title: "Read the machine".into(),
            category: "queries".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: true,
            help: "kinds, columns, views, blocks, rules, schedulers and pipelines, with what uses each".into(),
            ui_placement: "system".into(),
            json_path: Some("content/types.json#/types".into()),
        },
        CommandDef {
            id: "metrics".into(),
            title: "Read derived metrics".into(),
            category: "progress".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: true,
            help: "resolve rules.json#/metrics through the same evaluator a view uses".into(),
            ui_placement: "progress.panel".into(),
            json_path: Some("content/rules.json#/metrics".into()),
        },
        CommandDef {
            id: "metric.set".into(),
            title: "Write a derived figure".into(),
            category: "progress".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("name", "string", "the figure's name, e.g. problems.solved")
                    .required()
                    .positional(),
                ParamDef::new("label", "string", "what the panel calls it")
                    .flag("--label"),
                ParamDef::new("view", "string", "the saved view its fold runs over")
                    .flag("--view"),
                ParamDef::new("expr", "string", "evaluated once per record; empty removes it")
                    .flag("--expr"),
                ParamDef::new("reduce", "string", "sum · avg · min · max · count; empty removes it")
                    .flag("--reduce"),
                ParamDef::new("unit", "string", "shown beside the value, e.g. min; empty removes it")
                    .flag("--unit"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "write one derived figure — the panel's own numbers, edited as data".into(),
            ui_placement: "system".into(),
            json_path: Some("content/rules.json#/metrics/<name>".into()),
        },
        CommandDef {
            id: "search".into(),
            title: "Search records".into(),
            category: "index".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("query", "string", "words to find; every word must match")
                    .positional()
                    .required(),
                ParamDef::new("limit", "string", "maximum results (default 200)")
                    .flag("--limit"),
                ParamDef::new("engine", "string", "auto | index | oracle")
                    .flag("--engine"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "full-text search over records; the index answers when it is fresh".into(),
            ui_placement: "palette".into(),
            json_path: Some("<data dir>/indexes/<plan>/index.sqlite".into()),
        },
        CommandDef {
            id: "index.status".into(),
            title: "Index status".into(),
            category: "index".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: true,
            help: "is the derived cache present, fresh and healthy — as data".into(),
            ui_placement: "palette".into(),
            json_path: Some("<data dir>/indexes/<plan>/index.sqlite".into()),
        },
        CommandDef {
            id: "index.rebuild".into(),
            title: "Rebuild the index".into(),
            category: "index".into(),
            effect: CommandEffect::Read,
            params: vec![],
            aliases: vec![],
            plan_required: true,
            help: "rebuild the derived cache from validated source (never touches source)".into(),
            ui_placement: "palette".into(),
            json_path: Some("<data dir>/indexes/<plan>/index.sqlite".into()),
        },
        CommandDef {
            id: "source".into(),
            title: "Read a document or record".into(),
            category: "source".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("file", "string", "plan-relative document path").flag("--file"),
                ParamDef::new("id", "string", "a record id, instead of a file").flag("--id"),
            ],
            aliases: vec!["--source".into()],
            plan_required: true,
            help: "the text of one document or record, canonical; with no target, the pickable list".into(),
            ui_placement: "sourcePane".into(),
            json_path: Some("<the file argument, or content/records/<type>.jsonl#<id>>".into()),
        },
        CommandDef {
            id: "source.check".into(),
            title: "Check a draft".into(),
            category: "source".into(),
            effect: CommandEffect::Read,
            params: vec![
                ParamDef::new("file", "string", "the document the draft belongs to")
                    .flag("--file")
                    .required(),
                ParamDef::new("content", "string", "the draft text").flag("--content").required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "the draft's syntax findings, line-precise, from the loader's own parser".into(),
            ui_placement: "sourcePane".into(),
            json_path: Some("{ \"id\": \"source.check\", \"params\": { \"file\": \"<path>\", \"content\": \"<text>\" } }".into()),
        },
        CommandDef {
            id: "source.apply".into(),
            title: "Commit a document".into(),
            category: "source".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("file", "string", "plan-relative document path")
                    .flag("--file")
                    .required(),
                ParamDef::new("content", "string", "the new text; `-` reads stdin")
                    .flag("--content")
                    .required(),
                ParamDef::new("dry-run", "boolean", "validate without writing").flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: "the text door's commit: one document, canonicalized and validated".into(),
            ui_placement: "sourcePane".into(),
            json_path: Some("<the file argument>".into()),
        },
        // ===== undo (D14, §4.8 Principle 6) =====
        // `transaction::restore_plan` restores a committed transaction's
        // before-images through the validated path using the recorded txid.
        // Per §4.9, CLI writes do not enter the app's stack, and stale
        // undo entries are rejected.
        CommandDef {
            id: "edit.undo".into(),
            title: "Undo".into(),
            category: "edit".into(),
            effect: CommandEffect::Write,
            params: vec![ParamDef::new(
                "txid",
                "string",
                "the transaction to reverse — the id the edit's own commit returned",
            )
            .flag("--txid")
            .required()
            .positional()],
            aliases: vec![],
            plan_required: true,
            help: "reverse one committed edit: its before-images are restored byte-exact".into(),
            ui_placement: "app.window".into(),
            json_path: Some(".sam/backups/<txid>".into()),
        },
        CommandDef {
            id: "edit.redo".into(),
            title: "Redo".into(),
            category: "edit".into(),
            effect: CommandEffect::Write,
            params: vec![ParamDef::new(
                "txid",
                "string",
                "the undo's own transaction — redo restores what it replaced",
            )
            .flag("--txid")
            .required()
            .positional()],
            aliases: vec![],
            plan_required: true,
            help: "reapply the edit an undo reversed; the same mechanism, the other direction".into(),
            ui_placement: "app.window".into(),
            json_path: Some(".sam/backups/<txid>".into()),
        },
        // ===== presentation (§4.9: a destination or an explicit unavailable) =====
        CommandDef {
            id: "record.reveal".into(),
            title: "Reveal in the source pane".into(),
            category: "records".into(),
            effect: CommandEffect::Presentation,
            params: vec![
                ParamDef::new("id", "string", "the record to point the pane at")
                    .flag("--id")
                    .required(),
            ],
            aliases: vec![],
            plan_required: true,
            help: "point the source pane at one record and open it (Appendix C.2's row menu)".into(),
            ui_placement: "recordTable.rowContext".into(),
            json_path: Some(
                "{ \"id\": \"record.reveal\", \"params\": { \"id\": \"<record>\" } }".into(),
            ),
        },
        CommandDef {
            id: "view.edit".into(),
            title: "Edit this view…".into(),
            category: "views".into(),
            effect: CommandEffect::Presentation,
            params: vec![ParamDef::new("name", "string", "the saved view to edit")
                .flag("--name")
                .required()],
            aliases: vec![],
            plan_required: true,
            help: "open the view's own editor: its query rows and its block tree, as JSON (§4.8 P3)".into(),
            ui_placement: "view.head".into(),
            json_path: Some("{ \"id\": \"view.edit\", \"params\": { \"name\": \"<view>\" } }".into()),
        },
        CommandDef {
            id: "record.panel".into(),
            title: "Show a record's details".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![
                ParamDef::new("id", "string", "the record").flag("--id").required(),
                ParamDef::new("type", "string", "the record's kind").flag("--type"),
            ],
            aliases: vec![],
            plan_required: false,
            help: "open the detail panel on one record — the same surface a row's own control opens".into(),
            ui_placement: "recordTable.rowContext".into(),
            json_path: None,
        },
        CommandDef {
            id: "app.palette".into(),
            title: "Open the console".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![],
            aliases: vec!["app.search".to_string()],
            plan_required: false,
            help: "show the command palette; headless it reports the destination".into(),
            ui_placement: "app.window".into(),
            json_path: Some("{ \"id\": \"app.palette\" }".into()),
        },
        CommandDef {
            id: "app.changePlan".into(),
            title: "Change the plan folder".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![
                ParamDef::new("path", "string", "a plan folder to open — without it, the choices are shown")
                    .flag("--path"),
            ],
            aliases: vec![],
            plan_required: false,
            help: "the folder this window works in — the plans on this machine, the ones it remembers, or one you pick".into(),
            ui_placement: "app.window".into(),
            json_path: Some("{ \"id\": \"app.changePlan\" }".into()),
        },
        CommandDef {
            id: "app.openSystem".into(),
            title: "Open the System surface".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![
                ParamDef::new("section", "string", "kinds · views · rules · schedulers · pipelines · recently")
                    .flag("--section"),
            ],
            aliases: vec![],
            plan_required: false,
            help: "the machine's own place — kinds, columns, views, rules, schedules (§5 D12)".into(),
            ui_placement: "sidebar".into(),
            json_path: Some("{ \"id\": \"app.openSystem\" }".into()),
        },
        CommandDef {
            id: "screen.edit".into(),
            title: "Edit the screen".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![],
            aliases: vec![],
            plan_required: false,
            help: "enter the screen editor — add, move, restyle and remove the screen's components (COMPOSER §1)".into(),
            ui_placement: "app.window".into(),
            json_path: Some("{ \"id\": \"screen.edit\" }".into()),
        },
        CommandDef {
            id: "app.openSettings".into(),
            title: "Open Settings".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![],
            aliases: vec![],
            plan_required: false,
            help: "show Settings, generated from the app's own schema".into(),
            ui_placement: "app.window".into(),
            json_path: Some("{ \"id\": \"app.openSettings\" }".into()),
        },
        CommandDef {
            id: "app.gettingStarted".into(),
            title: "Open Getting started".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![],
            aliases: vec![],
            plan_required: false,
            help: "the document: what SAM is, how it works, and where to start — reopenable".into(),
            ui_placement: "app.window".into(),
            json_path: Some("{ \"id\": \"app.gettingStarted\" }".into()),
        },
        CommandDef {
            id: "app.toggleSourcePane".into(),
            title: "Toggle the source pane".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![],
            aliases: vec!["app.sourcePane".to_string()],
            plan_required: false,
            help: "show or hide the live JSON pane for the selection".into(),
            ui_placement: "app.window".into(),
            json_path: Some("{ \"id\": \"app.toggleSourcePane\" }".into()),
        },
        CommandDef {
            id: "rail.select".into(),
            title: "Go to a destination".into(),
            category: "app".into(),
            effect: CommandEffect::Presentation,
            params: vec![ParamDef::new("view", "string", "the saved view").flag("--view").required()],
            aliases: vec![],
            plan_required: false,
            help: "select a sidebar destination (Appendix C.0: navigation is a domain operation)".into(),
            ui_placement: "sidebar".into(),
            json_path: Some("{ \"id\": \"rail.select\", \"params\": { \"view\": \"<view>\" } }".into()),
        },
    ]
});

pub fn commands() -> &'static [CommandDef] {
    &COMMANDS
}

/// The menu bar as a declared projection (§4.10: *"a menu bar … projected from
/// the command registry, so the menu is a fourth projection of one registry
/// rather than a hand-maintained list"*).
///
/// Title and category order are data here, once; the shell renders them with
/// the platform's own menu API, the browser renders the same items as DOM, and
/// `--uicheck` prints them — three renderings, one list. Platform items (About,
/// Services, Hide, Quit) come from the OS API and are not listed.
pub const MENUS: [(&str, &[&str]); 4] = [
    (
        "Plan",
        &["plan", "records", "schema", "views", "settings", "source"],
    ),
    ("Edit", &["edit"]),
    ("View", &["app"]),
    ("Help", &["inspection", "recovery", "queries"]),
];

/// The commands a menu carries, in registry order, each with its declared key
/// (when one is bound) — the shape the shell's menu builder and the browser's
/// menu both read.
pub fn menu_json(keybindings: &[(String, String)]) -> serde_json::Value {
    let mut menus = Vec::new();
    for (title, categories) in MENUS {
        let items: Vec<serde_json::Value> = COMMANDS
            .iter()
            .filter(|def| categories.contains(&def.category.as_str()))
            .map(|def| {
                let key = keybindings
                    .iter()
                    .find(|(_, command)| command == &def.id)
                    .map(|(key, _)| key.clone());
                serde_json::json!({
                    "id": def.id,
                    "title": def.title,
                    "category": def.category,
                    "effect": def.effect,
                    "key": key,
                })
            })
            .collect();
        if !items.is_empty() {
            menus.push(serde_json::json!({ "title": title, "items": items }));
        }
    }
    serde_json::json!({ "menus": menus })
}

/// The accelerators the app answers to **before** a plan declares any (§4.8 P2,
/// D14: *"the binding is data, and it displays through `data-os`"*). They are
/// the platform's own conventions — ⌘Z is not SAM's invention — so undo works on
/// a plan whose `shell.json` declares nothing, and a plan that binds the same
/// key to another command overrides this list.
pub const DEFAULT_KEYBINDINGS: [(&str, &str); 5] = [
    ("mod+z", "edit.undo"),
    ("mod+shift+z", "edit.redo"),
    ("mod+k", "app.palette"),
    ("mod+shift+j", "app.toggleSourcePane"),
    ("mod+comma", "app.openSettings"),
];

/// The modifiers a binding may carry (`mod` is the platform's own: ⌘ on macOS,
/// Ctrl elsewhere — §4.8, D14).
pub const KEY_MODIFIERS: [&str; 5] = ["mod", "shift", "alt", "opt", "ctrl"];

/// The named keys a binding may end with; a letter or a digit is also a key
/// name. One list, read by the UI's resolver *and* by the validator, so a typo
/// is a load warning instead of a key that silently never fires.
pub const KEY_NAMES: [&str; 10] = [
    "comma",
    "period",
    "slash",
    "space",
    "enter",
    "escape",
    "tab",
    "backspace",
    "minus",
    "equal",
];

/// Is this binding one the app can actually resolve? `mod+shift+j` yes,
/// `mod+` and `cmd+hyper` no.
pub fn key_is_readable(key: &str) -> bool {
    let tokens: Vec<&str> = key.split('+').map(str::trim).collect();
    let Some((name, modifiers)) = tokens.split_last() else {
        return false;
    };
    if name.is_empty() {
        return false;
    }
    let named = name.chars().count() == 1 && name.chars().all(|c| c.is_ascii_alphanumeric());
    if !named && !KEY_NAMES.contains(name) {
        return false;
    }
    modifiers.iter().all(|token| KEY_MODIFIERS.contains(token))
}

/// The per-type pair (`<type>.new`, `<type>.paste`), generated from the active
/// type system rather than hand-listed. §4.8 gives both a placement in every
/// surface, and the Phase 4 parity gate enumerates them alongside the static set.
pub fn dynamic_commands(types: &BTreeMap<String, TypeDef>) -> Vec<CommandDef> {
    let mut out = Vec::new();
    for (name, def) in types {
        out.push(CommandDef {
            id: format!("{name}.new"),
            title: format!("New {name}"),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new(
                    "id",
                    "string",
                    "stable id; omitted takes the engine's next free",
                )
                .flag("--id"),
                ParamDef::new("value", "string", "one field as key=value; repeatable")
                    .flag("--value")
                    .repeats(),
            ],
            aliases: vec![],
            plan_required: true,
            help: format!("append one {name} record, validated whole-plan"),
            ui_placement: "recordTable.toolbar".into(),
            json_path: Some(format!("content/records/{name}.jsonl")),
        });
        out.push(CommandDef {
            id: format!("{name}.paste"),
            title: format!("Paste {name} lines"),
            category: "records".into(),
            effect: CommandEffect::Write,
            params: vec![
                ParamDef::new("records", "string", "a JSON array of row objects")
                    .flag("--records")
                    .required(),
                ParamDef::new("id", "string", "set the selected field on every row").flag("--id"),
                ParamDef::new("dry-run", "boolean", "preview the batch").flag("--dry-run"),
            ],
            aliases: vec![],
            plan_required: true,
            help: format!("paste TSV/CSV as {name} records — commits as one apply batch"),
            ui_placement: "recordTable.paste".into(),
            json_path: Some(format!("content/records/{name}.jsonl")),
        });
        let _ = def;
    }
    out
}

/// id or documented alias → def. The verb on the terminal IS the registry id
/// (§4.9: the same ids ⌘K will show).
pub fn resolve(token: &str) -> Option<&'static CommandDef> {
    COMMANDS
        .iter()
        .find(|def| def.id == token || def.aliases.iter().any(|alias| alias == token))
}

/// The write commands whose handlers exist in [`crate::dispatch`]. The Phase 4
/// parity gate asserts this set matches the registry's write effects.
pub fn registered_write_ids() -> Vec<&'static str> {
    COMMANDS
        .iter()
        .filter(|def| def.effect == CommandEffect::Write)
        .map(|def| def.id.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_def_declares_a_placement_the_ui_can_render() {
        for def in commands() {
            assert!(
                KNOWN_PLACEMENTS.contains(&def.ui_placement.as_str()),
                "{} declares an unknown placement \"{}\"",
                def.id,
                def.ui_placement
            );
        }
    }

    #[test]
    fn every_write_def_declares_its_json_path() {
        for def in commands() {
            if def.effect == CommandEffect::Write {
                assert!(
                    def.json_path.is_some(),
                    "{} hides its JSON identity",
                    def.id
                );
            }
        }
    }

    #[test]
    fn ids_are_unique_and_aliases_resolve() {
        let mut ids: Vec<&str> = commands().iter().map(|def| def.id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            count,
            "a duplicate id would make dispatch ambiguous"
        );

        assert_eq!(
            resolve("--configcheck").map(|def| def.id.as_str()),
            Some("configcheck")
        );
        assert_eq!(
            resolve("apply").map(|def| def.effect),
            Some(CommandEffect::Write)
        );
        assert!(resolve("nope").is_none());
    }

    #[test]
    fn the_write_set_is_every_operation_that_changes_the_plan() {
        // Phase 2's set (apply · plan.new · tx.restore) plus Phase 4's domain
        // set, Phase 5's view and block writes, and Phase 6's pipelines,
        // reviews and structural edits. Pinned as a list because a write command
        // that quietly loses its projection is exactly what Gate 2 exists to
        // catch.
        let mut ids = registered_write_ids();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                "appearance.clearOverride",
                "appearance.setOverride",
                "appearance.setTextScale",
                "apply",
                "column.choices",
                "column.delete",
                "column.duplicate",
                "column.hide",
                "column.new",
                "column.rename",
                "column.reorder",
                "column.retype",
                "column.show",
                "edit.redo",
                "edit.undo",
                "list.delete",
                "list.new",
                "list.set",
                "metric.set",
                "plan.new",
                "profile.export",
                "profile.import",
                "record.advanceStage",
                "record.defer",
                "record.delete",
                "record.logReview",
                "record.move",
                "record.new",
                "record.paste",
                "record.setField",
                "records.renumber",
                "settings.set",
                "source.apply",
                "theme.set",
                "tx.restore",
                "type.delete",
                "type.new",
                "type.setPipeline",
                "type.setPrivate",
                "view.block.add",
                "view.block.move",
                "view.block.remove",
                "view.block.set",
                "view.delete",
                "view.setColumns",
                "view.setComponents",
                "view.setFilter",
                "view.setGroup",
                "view.setLayout",
                "view.setLimit",
                "view.setPanel",
                "view.setSort",
            ]
        );
    }

    #[test]
    fn a_binding_is_readable_only_when_the_app_can_resolve_it() {
        assert!(key_is_readable("mod+z"));
        assert!(key_is_readable("mod+shift+z"));
        assert!(key_is_readable("mod+comma"));
        assert!(key_is_readable("alt+1"));
        assert!(
            !key_is_readable("cmd+z"),
            "`mod` is the logical modifier (§4.8 P2)"
        );
        assert!(
            !key_is_readable("mod+"),
            "a modifier with no key is not a binding"
        );
        assert!(!key_is_readable("mod+hyper"), "an unknown key never fires");
        assert!(
            !key_is_readable("mod+shift+"),
            "a trailing separator is a typo"
        );
    }

    #[test]
    fn the_menu_is_a_projection_of_the_registry() {
        let menus = menu_json(&[("mod+z".to_string(), "edit.undo".to_string())]);
        let menus = menus["menus"].as_array().expect("menus is an array");
        assert!(!menus.is_empty());
        for menu in menus {
            for item in menu["items"].as_array().expect("items is an array") {
                let id = item["id"].as_str().expect("every item names an id");
                assert!(resolve(id).is_some(), "menu item {id} dispatches nothing");
            }
        }
        // The bound key rides along, so the shell's accelerator and the
        // browser's shortcut hint come from one list.
        let edit = menus
            .iter()
            .find(|menu| menu["title"] == serde_json::json!("Edit"))
            .expect("there is an Edit menu");
        let undo = edit["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == serde_json::json!("edit.undo"))
            .expect("Edit carries Undo");
        assert_eq!(undo["key"], serde_json::json!("mod+z"));
    }

    #[test]
    fn every_kind_gets_its_own_new_and_paste_commands() {
        let mut types = BTreeMap::new();
        types.insert(
            "problemset".to_string(),
            TypeDef {
                fields: vec![],
                ..Default::default()
            },
        );
        let dynamic = dynamic_commands(&types);
        let ids: Vec<&str> = dynamic.iter().map(|def| def.id.as_str()).collect();
        assert_eq!(ids, vec!["problemset.new", "problemset.paste"]);
        for def in &dynamic {
            assert!(KNOWN_PLACEMENTS.contains(&def.ui_placement.as_str()));
            assert!(def.json_path.is_some());
        }
    }
}
