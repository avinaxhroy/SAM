//! Content model (§3) — the `serde` types the decode stage fills in.
//!
//! Decoded, never validated: the §3.7 contract lives in `validator.rs` and the
//! path-precise decode in `decode.rs`. What is here is the §3.1 shape — types
//! with typed fields, records as `[String: JSONValue]` documents with `links`,
//! views as saved queries, pipelines as named stage machines.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::json_value::JSONValue;

/// The **closed** block vocabulary of this build (§3.6's eight layouts plus the
/// six presentation blocks Phase 5 renders). Closed for the renderer, open at
/// the file: a kind this build does not know is a warning and a placeholder
/// (`unknown block: "X" · known: …`), never a load failure and never a blank
/// screen — a plan authored for a newer build still opens (§0 rule 3, D8).
pub const BLOCK_KINDS: [&str; 14] = [
    // Record blocks — one renderer per §3.6 layout.
    "list",
    "table",
    "board",
    "timeline",
    "calendar",
    "tree",
    "cardGrid",
    "graph",
    // Presentation blocks.
    "stat",
    "chart",
    "callout",
    "columns",
    "conditional",
    "repeat",
];

/// The eight record layouts, which are also block kinds (§3.6).
pub const RECORD_BLOCK_KINDS: [&str; 8] = [
    "list", "table", "board", "timeline", "calendar", "tree", "cardGrid", "graph",
];

/// The five folds a `stat`/`chart` block may apply to its per-record expression.
pub const BLOCK_REDUCES: [&str; 5] = ["sum", "avg", "min", "max", "count"];

/// The recursion boundary (D8): container blocks nest at most this deep. Deeper
/// trees are a **warning** at load and a placeholder at render — the finite
/// boundary is a property of the renderer, so an over-deep screen degrades to a
/// stated finding instead of unbounded recursion.
pub const MAX_BLOCK_DEPTH: usize = 6;

/// One render primitive (§3.1) — the L1 shape of an entry in a view's `blocks`.
///
/// Every field is optional but `kind`: a record block names a saved view to
/// draw (`None` = the view it lives in), a `stat`/`chart` names one plus the
/// expression to fold, a `conditional` names its test, and the three container
/// kinds (`columns`, `conditional`, `repeat`) carry child blocks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockDef {
    pub kind: String,
    /// A saved view this block draws, or folds over. Absent = the enclosing view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// A heading for the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A `stat`'s caption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// A `callout`'s sentence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// `stat`/`chart`: the expression evaluated once per record of the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    /// `stat`/`chart`: the fold — [`BLOCK_REDUCES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce: Option<String>,
    /// `conditional`: evaluated in the enclosing record context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Container children (`columns`, `conditional`, `repeat`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<BlockDef>>,
    /// `conditional`'s other branch.
    #[serde(rename = "else", default, skip_serializing_if = "Option::is_none")]
    pub else_blocks: Option<Vec<BlockDef>>,
    /// `repeat`: how many instances to draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// `chart`: the bucket key expression (a date or a scalar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    /// `chart`: the value expression, folded per bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    /// `chart`: fill the last N calendar days ending today, empty days as zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<i64>,
}

impl BlockDef {
    pub fn is_record(&self) -> bool {
        RECORD_BLOCK_KINDS.contains(&self.kind.as_str())
    }

    pub fn is_container(&self) -> bool {
        matches!(self.kind.as_str(), "columns" | "conditional" | "repeat")
    }

    /// The fold this block applies. `count` when the block names no expression
    /// and no fold — "how many records this view returns" is the common case and
    /// does not need spelling out.
    pub fn reduce_kind(&self) -> &str {
        self.reduce
            .as_deref()
            .unwrap_or(if self.expr.is_none() { "count" } else { "sum" })
    }
}

/// One typed column (§3.1). `type` is the §3.4 closed vocabulary; `to`,
/// `cardinality`, `options` and `expr` are only meaningful for some of it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub key: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Relation target type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// `"one"` (default) or `"many"` (§3.1 cardinality).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// `select` / `multiSelect` choices; options are data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Layer-2 expression for a `formula` field (§4.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
}

impl FieldDef {
    pub fn is_relation(&self) -> bool {
        self.type_ == "relation"
    }

    pub fn is_formula(&self) -> bool {
        self.type_ == "formula"
    }

    pub fn is_one(&self) -> bool {
        self.cardinality.as_deref() != Some("many")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TypeDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The single-valued parent relation, named for the parent type (§3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(rename = "colorRole", default, skip_serializing_if = "Option::is_none")]
    pub color_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trackable: Option<bool>,
    /// Non-null iff the type is trackable (§3.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<String>,
    /// Marks records of this type as private (§6 Phase 7), excluding them
    /// from profile exports unless `--personal` is specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    pub fields: Vec<FieldDef>,
}

impl TypeDef {
    pub fn field(&self, key: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// Link keys a record of this type may carry: its declared relation fields
    /// plus the single-valued parent edge named for the parent type (§3.1).
    pub fn allowed_link_keys(&self) -> BTreeSet<String> {
        let mut keys: BTreeSet<String> = self
            .fields
            .iter()
            .filter(|f| f.is_relation())
            .map(|f| f.key.clone())
            .collect();
        if let Some(parent) = &self.parent {
            keys.insert(parent.clone());
        }
        keys
    }
}

/// A record (§3.1). Non-relation values live only in `fields`; relation values
/// only in `links`, always as arrays of ids — including single-valued ones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    #[serde(
        rename = "schemaVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub schema_version: Option<u64>,
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub fields: BTreeMap<String, JSONValue>,
    #[serde(default)]
    pub links: BTreeMap<String, Vec<String>>,
}

/// Component declaration for a composed screen (COMPOSER §2.1).
/// `surface` is an open UI component name. `span` defaults to 2 (full row);
/// 1 represents half-width.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentDef {
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<i64>,
}

/// View definition representing a saved query layout (§3.6) or a composed
/// screen of components (COMPOSER §2.1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewDef {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
    /// Declared null order for sort and group keys (§4.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nulls: Option<String>,
    /// The screen's blocks, when this view is composed rather than a single
    /// list. Absent = the view draws itself as one record block in `layout`
    /// (so every pre-Phase-5 view keeps working, and `SAM view` keeps its
    /// top-level `records`): the renderer has one path, not two.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<BlockDef>>,
    /// Phase 6's richer surface for a destination, chosen by the view document
    /// like any other part of the screen: `today`, `plan`, `subjects`,
    /// `practice`, `mocks`, `library`, `notes`, `reviews` (the stage machine and
    /// the due queue) or `progress` (derived metrics). Unknown names are a
    /// warning at load and a stated placeholder at render — the same forward
    /// compatibility blocks have (D8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<String>,
    /// The screen the **student** composed (COMPOSER §2.1): the stack of widgets
    /// this view draws, in the student's own order. `components` present — even
    /// as `[]` — makes the view a composed screen and outranks both `panel` and
    /// `blocks` at render; absent means the view behaves exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ComponentDef>>,
}

/// The closed panel vocabulary of this build. Open at the file: a plan authored
/// for a newer build still opens, with a stated placeholder.
///
/// The first seven are P2's designed screens — the destination's own kind
/// decides which one a preset declares (UI_SPEC §1's census, `UI_PLAN` D6):
/// `today` (the day's facts), `plan` (weeks and the work in them), `subjects`
/// (the identity layer), `practice` (question banks), `mocks` (dated
/// assessments), `library` and `notes` (reference lists).
pub const PANEL_KINDS: [&str; 9] = [
    "today", "plan", "subjects", "practice", "mocks", "library", "notes", "reviews", "progress",
];

/// A named stage machine (§3.5) — the only place "how you make progress" lives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineDef {
    pub stages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<BTreeMap<String, Gate>>,
    #[serde(
        rename = "completeWhen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub complete_when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
    #[serde(
        rename = "anchorSkip",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub anchor_skip: Option<AnchorSkip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    #[serde(
        rename = "appliesToKinds",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub applies_to_kinds: Option<Vec<String>>,
    #[serde(
        rename = "minProblems",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub min_problems: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorSkip {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<bool>,
    #[serde(
        rename = "requireReason",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub require_reason: Option<bool>,
}

/// `content/types.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesFile {
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: Option<u64>,
    pub types: BTreeMap<String, TypeDef>,
}

/// `content/views.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewsFile {
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: Option<u64>,
    pub views: BTreeMap<String, ViewDef>,
}

/// `content/rules.json` — pipelines, scheduler, derived metrics and the app's
/// own settings. Phase 6 types the scheduler and the metrics: they are read as
/// data by the engine ([`crate::scheduler`], [`crate::pipeline`]) rather than
/// kept as raw JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RulesFile {
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipelines: Option<BTreeMap<String, PipelineDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<crate::rules::SchedulerDef>,
    /// Named folds over saved views (§3.1's "derived metrics"), shown by the
    /// Progress panel and readable as `SAM metrics --json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<BTreeMap<String, crate::rules::MetricDef>>,
    /// §5's design lints: severity overrides and dated waivers, surfaced by the
    /// Design doctor (`design.check`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint: Option<crate::rules::LintFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub study: Option<JSONValue>,
    /// IANA timezone for day and week calculations (§3.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// `content/shell.json` navigation entry (§4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellNavEntry {
    pub title: String,
    pub view: String,
    /// The rail's icon name, as the plan declares it (§1 R3). Optional: a plan
    /// written before the rail drew icons still loads, and the frame falls back
    /// to a glyph of its own rather than the engine inventing a name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// `content/shell.json` — navigation, commands and keybindings.
///
/// Navigation and keybindings arrived as arrays in the seed preset (§6 Phase 1,
/// deviation 1: "shell.json navigation/commands/keybindings are arrays, not
/// objects"). Commands and keybindings stay raw here: their registry ids and
/// parameter schemas are Phase 2's, and the validator only needs the two string
/// members a keybinding carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellFile {
    #[serde(
        rename = "schemaVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub schema_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation: Option<Vec<ShellNavEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<JSONValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybindings: Option<Vec<JSONValue>>,
}

/// `content/appearance.json` — the pinned theme plus token overrides (§4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceFile {
    #[serde(
        rename = "schemaVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub schema_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// §6 Phase 7's text-size preference: every `fontSize` token resolves at
    /// this multiple. 1 (or absent) is the design system's own scale.
    #[serde(rename = "textScale", default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<JSONValue>,
}
