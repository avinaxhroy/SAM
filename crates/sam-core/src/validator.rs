//! Semantic validation (§3.7, Phase 3, Phase 4).
//!
//! Runs post-decode across structured models and raw syntax trees, accumulating all
//! diagnostics. Integrity failures block writes; advisory findings are surfaced as warnings.
//!
//! Expression validation (§4.4) checks syntax and path resolution for formula `expr`,
//! view filters, sorts, and groups. Formula dependency cycles are identified from
//! parsed path references. Evaluation runtime lives in [`crate::evaluator`].
//! Also validates keybinding uniqueness, schema shapes, and command identifiers.

use std::collections::{BTreeMap, BTreeSet};

use crate::command_registry;
use crate::config_store::PositionedRecord;
use crate::diagnostic::{Diagnostic, escape_pointer_segment};
use crate::json_value::JSONValue;
use crate::model::{
    BLOCK_KINDS, BLOCK_REDUCES, BlockDef, FieldDef, MAX_BLOCK_DEPTH, RECORD_BLOCK_KINDS, RulesFile,
    ShellFile, TypeDef, ViewDef,
};

pub const FIELD_TYPES: [&str; 15] = [
    "text",
    "longtext",
    "number",
    "duration",
    "date",
    "daterange",
    "bool",
    "select",
    "multiSelect",
    "rating",
    "url",
    "relation",
    "progress",
    "formula",
    "json",
];

const LAYOUTS: [&str; 8] = RECORD_BLOCK_KINDS;

/// An empty gate table, for the same reason as [`EMPTY_PIPELINES`].
static EMPTY_GATES: std::sync::LazyLock<BTreeMap<String, crate::model::Gate>> =
    std::sync::LazyLock::new(BTreeMap::new);

/// An empty pipeline table, so `None` and `{}` read the same to a validator.
static EMPTY_PIPELINES: std::sync::LazyLock<BTreeMap<String, crate::model::PipelineDef>> =
    std::sync::LazyLock::new(BTreeMap::new);

/// An empty metric table, the same convention.
static EMPTY_METRICS: std::sync::LazyLock<BTreeMap<String, crate::rules::MetricDef>> =
    std::sync::LazyLock::new(BTreeMap::new);

pub fn validate(
    types: &BTreeMap<String, TypeDef>,
    views: &BTreeMap<String, ViewDef>,
    rules: &RulesFile,
    shell: Option<&ShellFile>,
    records: &[PositionedRecord],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_types(types, &mut diagnostics);
    validate_views(views, types, &mut diagnostics);
    validate_rules(rules, views, &mut diagnostics);
    diagnostics.extend(settings_schema(rules));
    validate_shell(shell, views, &mut diagnostics);
    validate_records(records, types, &mut diagnostics);
    validate_pipelines_referenced(types, rules, &mut diagnostics);
    diagnostics
}

/// Child types of `type`: any type whose relations or parent target it — the
/// reverse collections a path segment may name.
pub fn child_types(types: &BTreeMap<String, TypeDef>, of: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (name, def) in types {
        if name == of {
            continue;
        }
        if def.parent.as_deref() == Some(of) {
            out.insert(name.clone());
            continue;
        }
        if def
            .fields
            .iter()
            .any(|field| field.is_relation() && field.to.as_deref() == Some(of))
        {
            out.insert(name.clone());
        }
    }
    out
}

/// Walk one dotted path over the type graph (§4.4 static check). Returns the
/// formula fields the path terminates on, plus a path diagnostic for any
/// segment that does not resolve — fields, relations, the parent, child types
/// and the reserved `id` are the whole vocabulary a path may name.
fn walk_path(
    segments: &[String],
    from: &str,
    types: &BTreeMap<String, TypeDef>,
) -> (Vec<(String, String)>, Vec<Diagnostic>) {
    let mut terminals = Vec::new();
    let mut findings = Vec::new();
    let mut current: BTreeSet<String> = BTreeSet::from([from.to_string()]);
    for (index, segment) in segments.iter().enumerate() {
        let terminal = index + 1 == segments.len();
        if terminal {
            let mut resolved = false;
            for name in &current {
                if segment == "id"
                    || types
                        .get(name)
                        .is_some_and(|def| def.field(segment).is_some())
                    || types.get(name).and_then(|def| def.parent.as_deref())
                        == Some(segment.as_str())
                    || child_types(types, name).contains(segment)
                {
                    resolved = true;
                }
                if types
                    .get(name)
                    .and_then(|def| def.field(segment))
                    .is_some_and(FieldDef::is_formula)
                {
                    terminals.push((name.clone(), segment.clone()));
                }
            }
            if !resolved {
                let location = if index == 0 {
                    segment.clone()
                } else {
                    format!("{segment} at segment {}", index + 1)
                };
                findings.push(Diagnostic::error(
                    if index == 0 {
                        "expr.unknown-root"
                    } else {
                        "expr.bad-path"
                    },
                    "expression",
                    format!(
                        "\"{}\": {location} is not a field, relation, parent or child type reachable from {from}",
                        segments.join(".")
                    ),
                ));
            }
        } else {
            let mut next = BTreeSet::new();
            for name in &current {
                if let Some(field) = types.get(name).and_then(|def| def.field(segment)) {
                    if field.is_relation() {
                        if let Some(to) = &field.to {
                            next.insert(to.clone());
                        }
                    }
                }
                if types.get(name).and_then(|def| def.parent.as_deref()) == Some(segment.as_str()) {
                    next.insert(segment.clone());
                }
                if child_types(types, name).contains(segment) {
                    next.insert(segment.clone());
                }
            }
            if next.is_empty() {
                findings.push(Diagnostic::error(
                    "expr.bad-path",
                    "expression",
                    format!(
                        "\"{}\": {segment} at segment {} does not continue through a relation",
                        segments.join("."),
                        index + 1
                    ),
                ));
                return (terminals, findings);
            }
            current = next;
        }
    }
    (terminals, findings)
}

/// Validate one expression: parse, then static path check against a known
/// record type. Invalid expressions are diagnosed before a managed commit
/// (§4.4).
pub fn validate_expression(
    text: &str,
    from: &str,
    types: &BTreeMap<String, TypeDef>,
    at: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expr = match crate::expression::parse(text, at) {
        Ok(expr) => expr,
        Err(error) => {
            diagnostics.push(error.diagnostic);
            return;
        }
    };
    for segments in expr.paths() {
        let (_, findings) = walk_path(&segments, from, types);
        diagnostics.extend(findings.into_iter().map(|finding| Diagnostic {
            path: at.to_string(),
            ..finding
        }));
    }
}

/// Formula dependency analysis (§4.4: acyclic, deterministic). A formula may
/// depend on formula fields only through declared relations — the graph is
/// walked over type sets.
pub fn validate_formula_dependencies(
    types: &BTreeMap<String, TypeDef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut nodes: Vec<(String, String)> = Vec::new();
    for (type_name, def) in types {
        for field in def.fields.iter().filter(|field| field.is_formula()) {
            nodes.push((type_name.clone(), field.key.clone()));
        }
    }
    nodes.sort();

    let mut edges: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
    for node in &nodes {
        let text = types
            .get(&node.0)
            .and_then(|def| def.field(&node.1))
            .and_then(|field| field.expr.clone());
        let mut unique = BTreeSet::new();
        if let Some(text) = text {
            if let Ok(expr) = crate::expression::parse(&text, "formula") {
                for segments in expr.paths() {
                    let (terminals, _) = walk_path(&segments, &node.0, types);
                    for terminal in terminals {
                        if nodes.contains(&terminal) {
                            unique.insert(terminal);
                        }
                    }
                }
            }
        }
        edges.insert(node.clone(), unique.into_iter().collect());
    }

    for node in &nodes {
        let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
        let mut stack: Vec<(String, String)> = edges.get(node).cloned().unwrap_or_default();
        stack.reverse();
        while let Some(next) = stack.pop() {
            if !seen.insert(next.clone()) {
                continue;
            }
            if next == *node {
                let index = types
                    .get(&node.0)
                    .and_then(|def| def.fields.iter().position(|field| field.key == node.1))
                    .unwrap_or(0);
                diagnostics.push(Diagnostic::error(
                    "formula.cycle",
                    format!(
                        "content/types.json#/types/{}/fields/{index}/expr",
                        escape_pointer_segment(&node.0)
                    ),
                    format!(
                        "formula \"{}\" on {} participates in a dependency cycle",
                        node.1, node.0
                    ),
                ));
                break;
            }
            for dependency in edges.get(&next).cloned().unwrap_or_default().iter().rev() {
                stack.push(dependency.clone());
            }
        }
    }
}

// ── types.json ──────────────────────────────────────────────────────────────

pub fn validate_types(types: &BTreeMap<String, TypeDef>, diagnostics: &mut Vec<Diagnostic>) {
    if types.is_empty() {
        diagnostics.push(Diagnostic::error(
            "types.empty",
            "content/types.json#/types",
            "at least one type must be declared",
        ));
    }

    for (name, def) in types {
        let base = format!("content/types.json#/types/{}", escape_pointer_segment(name));
        if name.is_empty() {
            diagnostics.push(Diagnostic::error(
                "types.empty-name",
                base.clone(),
                "type name must not be empty",
            ));
        }
        if let Some(parent) = &def.parent {
            if !parent.is_empty() && !types.contains_key(parent) {
                diagnostics.push(Diagnostic::error(
                    "types.parent-unknown",
                    format!("{base}/parent"),
                    format!("parent \"{parent}\" is not a declared type"),
                ));
            }
        }
        if def.fields.is_empty() {
            diagnostics.push(Diagnostic::error(
                "types.no-fields",
                format!("{base}/fields"),
                format!("type \"{name}\" declares no fields"),
            ));
        }

        let mut seen_keys = BTreeSet::new();
        for (index, field) in def.fields.iter().enumerate() {
            let field_path = format!("{base}/fields/{index}");
            if field.key.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "field.empty-key",
                    format!("{field_path}/key"),
                    "field key must not be empty",
                ));
            }
            if !seen_keys.insert(field.key.clone()) {
                diagnostics.push(Diagnostic::error(
                    "field.duplicate-key",
                    format!("{field_path}/key"),
                    format!("duplicate field key \"{}\" on {name}", field.key),
                ));
            }
            if !FIELD_TYPES.contains(&field.type_.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "field.unknown-type",
                    format!("{field_path}/type"),
                    format!(
                        "field \"{}\": unknown type \"{}\"; vocabulary is {}",
                        field.key,
                        field.type_,
                        sorted_join(&FIELD_TYPES)
                    ),
                ));
            }
            if field.type_ == "relation" {
                let Some(target) = field.to.as_deref().filter(|to| !to.is_empty()) else {
                    diagnostics.push(Diagnostic::error(
                        "relation.missing-to",
                        format!("{field_path}/to"),
                        format!("relation \"{}\" must declare \"to\"", field.key),
                    ));
                    continue;
                };
                if !types.contains_key(target) {
                    diagnostics.push(Diagnostic::error(
                        "relation.target-unknown",
                        format!("{field_path}/to"),
                        format!(
                            "relation \"{}\" → \"{target}\" is not a declared type",
                            field.key
                        ),
                    ));
                }
            }
            if field.type_ == "formula" && field.expr.as_deref().unwrap_or("").is_empty() {
                diagnostics.push(Diagnostic::error(
                    "formula.missing-expr",
                    format!("{field_path}/expr"),
                    format!("formula field \"{}\" must declare \"expr\"", field.key),
                ));
            }
            if field.type_ == "formula" {
                if let Some(expr) = field.expr.as_deref().filter(|expr| !expr.is_empty()) {
                    validate_expression(
                        expr,
                        name,
                        types,
                        &format!("{field_path}/expr"),
                        diagnostics,
                    );
                }
            }
            if let Some(cardinality) = &field.cardinality {
                if cardinality != "one" && cardinality != "many" {
                    diagnostics.push(Diagnostic::error(
                        "relation.bad-cardinality",
                        format!("{field_path}/cardinality"),
                        format!("cardinality must be \"one\" or \"many\", found \"{cardinality}\""),
                    ));
                }
            }
            if field.type_ == "select" || field.type_ == "multiSelect" {
                match field.options.as_ref().filter(|options| !options.is_empty()) {
                    None => {
                        diagnostics.push(Diagnostic::error(
                            "select.missing-options",
                            format!("{field_path}/options"),
                            format!(
                                "{} field \"{}\" must declare options",
                                field.type_, field.key
                            ),
                        ));
                        continue;
                    }
                    Some(options) => {
                        let unique: BTreeSet<&String> = options.iter().collect();
                        if unique.len() != options.len() {
                            diagnostics.push(Diagnostic::error(
                                "select.duplicate-options",
                                format!("{field_path}/options"),
                                format!(
                                    "{} field \"{}\" has duplicate options",
                                    field.type_, field.key
                                ),
                            ));
                        }
                    }
                }
            } else if field.options.is_some() {
                diagnostics.push(Diagnostic::warning(
                    "select.options-misplaced",
                    format!("{field_path}/options"),
                    "options only apply to select/multiSelect fields",
                ));
            }
        }
    }

    validate_formula_dependencies(types, diagnostics);

    // Parent cycles at the type level (§3.1: parent edges must be acyclic).
    for name in types.keys() {
        let mut visited = BTreeSet::new();
        let mut cursor = Some(name.as_str());
        while let Some(current) = cursor {
            if !visited.insert(current.to_string()) {
                diagnostics.push(Diagnostic::error(
                    "types.parent-cycle",
                    "content/types.json#/types",
                    format!("parent cycle at \"{current}\" reached from \"{name}\""),
                ));
                break;
            }
            cursor = types.get(current).and_then(|def| def.parent.as_deref());
        }
    }
}

// ── views.json ──────────────────────────────────────────────────────────────

pub fn validate_views(
    views: &BTreeMap<String, ViewDef>,
    types: &BTreeMap<String, TypeDef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, view) in views {
        let base = format!("content/views.json#/views/{}", escape_pointer_segment(name));

        // COMPOSER §2.1: a view draws something — a query over a type, or the
        // components the student composed. Neither is the one case with nothing
        // to draw.
        if view.type_.is_none() && view.components.is_none() {
            diagnostics.push(Diagnostic::error(
                "view.empty",
                base.clone(),
                format!("view \"{name}\" declares neither a type nor components — a view must draw something"),
            ));
            // No `continue`: the pair check below still names a stray `layout`
            // (a view with a layout and no type draws nothing either), and the
            // type-gated rules below skip themselves because the type is absent.
        }
        // `type` and `layout` are a pair: a composed screen may declare neither,
        // but one without the other names no complete query (§2.1).
        match (&view.type_, &view.layout) {
            (Some(_), None) => diagnostics.push(Diagnostic::error(
                "view.layout-missing",
                format!("{base}/layout"),
                format!("view \"{name}\" declares a type but no layout"),
            )),
            (None, Some(_)) => diagnostics.push(Diagnostic::error(
                "view.type-missing",
                format!("{base}/type"),
                format!("view \"{name}\" declares a layout but no type"),
            )),
            _ => {}
        }
        // Validate composed screen components: each component must specify a surface,
        // and duplicate surfaces emit an advisory warning (§2.1).
        if let Some(components) = &view.components {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut duplicated = false;
            for (index, component) in components.iter().enumerate() {
                if component.surface.trim().is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "component.surface-missing",
                        format!("{base}/components/{index}/surface"),
                        format!("component {index} of \"{name}\" names no surface"),
                    ));
                } else if !seen.insert(component.surface.as_str()) {
                    duplicated = true;
                }
                // A span is how much of the row the widget takes (§2.1): 1 is
                // half, 2 the whole row — the default, so absent is fine. A
                // width below 1 cannot be drawn at all: an error. A width above
                // 2 is one this build does not draw, and it draws it full width
                // rather than dropping the widget: a warning, the file stays as
                // the newer build wrote it.
                if let Some(span) = component.span {
                    if span < 1 {
                        diagnostics.push(Diagnostic::error(
                            "component.span-invalid",
                            format!("{base}/components/{index}/span"),
                            format!("component {index} of \"{name}\" declares span {span}; a span is an integer of 1 or 2"),
                        ));
                    } else if span > 2 {
                        diagnostics.push(Diagnostic::warning(
                            "component.span-wide",
                            format!("{base}/components/{index}/span"),
                            format!("component {index} of \"{name}\" declares span {span}; this build draws it full width"),
                        ));
                    }
                }
            }
            if duplicated {
                diagnostics.push(Diagnostic::warning(
                    "view.duplicate-component",
                    format!("{base}/components"),
                    format!("view \"{name}\" draws the same surface more than once"),
                ));
            }
        }

        // Everything below reads the query, which only a typed view declares.
        let Some(type_name) = view.type_.as_deref() else {
            continue;
        };
        let Some(def) = types.get(type_name) else {
            diagnostics.push(Diagnostic::error(
                "view.unknown-type",
                format!("{base}/type"),
                format!("view \"{name}\" queries undeclared type \"{type_name}\""),
            ));
            continue;
        };
        if let Some(layout) = &view.layout
            && !LAYOUTS.contains(&layout.as_str())
        {
            diagnostics.push(Diagnostic::error(
                "view.unknown-layout",
                format!("{base}/layout"),
                format!("layout \"{layout}\" is not in {}", sorted_join(&LAYOUTS)),
            ));
        }
        for (index, column) in view
            .columns
            .as_deref()
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            if def.field(column).is_none() {
                diagnostics.push(Diagnostic::error(
                    "view.unknown-column",
                    format!("{base}/columns/{index}"),
                    format!("column \"{column}\" is not a field of {type_name}"),
                ));
            }
        }
        if let Some(nulls) = &view.nulls {
            if nulls != "first" && nulls != "last" {
                diagnostics.push(Diagnostic::error(
                    "view.bad-nulls",
                    format!("{base}/nulls"),
                    "nulls must be \"first\" or \"last\"",
                ));
            }
        }
        // Phase 6's panels are chosen by the view document like every other part
        // of the screen. An unknown name is a **warning** and a stated
        // placeholder at render — a plan authored for a newer build still opens
        // (D8's rule for blocks, applied to the panel).
        if let Some(panel) = &view.panel
            && !crate::model::PANEL_KINDS.contains(&panel.as_str())
        {
            diagnostics.push(Diagnostic::warning(
                "view.unknown-panel",
                format!("{base}/panel"),
                format!(
                    "unknown panel: \"{panel}\" · known: {}",
                    crate::model::PANEL_KINDS.join(" · ")
                ),
            ));
        }
        // Filter, sort keys and group are L2 expressions (§3.6): any path
        // expression over the view's type. Invalid expressions are diagnosed
        // here, before any managed commit (§4.4).
        if let Some(filter) = &view.filter {
            validate_expression(
                filter,
                type_name,
                types,
                &format!("{base}/filter"),
                diagnostics,
            );
        }
        if let Some(sort) = &view.sort {
            for part in sort.split(',') {
                let text = part.trim();
                if text.is_empty() {
                    continue;
                }
                let body = text.strip_prefix('-').unwrap_or(text);
                validate_expression(body, type_name, types, &format!("{base}/sort"), diagnostics);
            }
        }
        if let Some(group) = &view.group {
            validate_expression(
                group,
                type_name,
                types,
                &format!("{base}/group"),
                diagnostics,
            );
        }
        if let Some(limit) = view.limit {
            if limit < 0 {
                diagnostics.push(Diagnostic::error(
                    "view.bad-limit",
                    format!("{base}/limit"),
                    "limit must be non-negative",
                ));
            }
        }
        // Phase 5: the screen's blocks (§3.6, D8). The recursion boundary and
        // the kind vocabulary are checked here rather than at render, because
        // an over-deep or unknown block is a data problem, and the renderer's
        // placeholder is a fallback rather than the diagnostic.
        if let Some(blocks) = &view.blocks {
            validate_blocks(
                blocks,
                &base,
                view.type_.as_deref(),
                views,
                types,
                0,
                diagnostics,
            );
        }
    }

    // §5: a type with zero views is an advisory authoring warning.
    for name in types.keys() {
        if !views
            .values()
            .any(|view| view.type_.as_deref() == Some(name.as_str()))
        {
            diagnostics.push(Diagnostic::warning(
                "content.type.views",
                format!("content/types.json#/types/{}", escape_pointer_segment(name)),
                format!("type \"{name}\" has no views — unviewed types rot into a junk drawer"),
            ));
        }
    }
}

/// One screen's blocks (§3.6, D8) — structural findings are errors and block a
/// managed commit; the two forward-compatibility cases are **warnings**, because
/// both render as a stated placeholder rather than as a broken screen:
///
/// - a `kind` this build does not know, carried for a newer build (§0 rule 3);
/// - a tree deeper than [`MAX_BLOCK_DEPTH`], which the renderer refuses to
///   recurse into — the boundary is finite and this is where it is named.
fn validate_blocks(
    blocks: &[BlockDef],
    base: &str,
    enclosing: Option<&str>,
    views: &BTreeMap<String, ViewDef>,
    types: &BTreeMap<String, TypeDef>,
    depth: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (index, block) in blocks.iter().enumerate() {
        let path = format!("{base}/blocks/{index}");
        if !BLOCK_KINDS.contains(&block.kind.as_str()) {
            diagnostics.push(Diagnostic::warning(
                "view.unknown-block",
                format!("{path}/kind"),
                format!(
                    "unknown block: \"{}\" · known: {} — carried through, drawn as a placeholder",
                    block.kind,
                    BLOCK_KINDS.join(" · ")
                ),
            ));
            continue; // a shape this build cannot read is not checked further
        }
        if depth >= MAX_BLOCK_DEPTH {
            diagnostics.push(Diagnostic::warning(
                "view.block-depth",
                path.clone(),
                format!(
                    "blocks nest deeper than {MAX_BLOCK_DEPTH} — the renderer stops here and draws a placeholder"
                ),
            ));
            continue; // beyond the boundary nothing is rendered, so nothing is checked
        }

        // A block that names a view draws or folds that view; otherwise the view
        // it lives in. Expressions are checked against that view's type, because
        // that is the record context the evaluator will hand them.
        let source = match &block.view {
            None => None,
            Some(name) => match views.get(name) {
                // A composed screen the block names has no type of its own —
                // there is no record context to check its expressions against,
                // so they are left to the enclosing view, exactly as an
                // untyped source always was.
                Some(view) => view.type_.clone(),
                None => {
                    diagnostics.push(Diagnostic::error(
                        "view.unknown-view",
                        format!("{path}/view"),
                        format!("block names undeclared view \"{name}\""),
                    ));
                    None
                }
            },
        };
        let context_type = source.as_deref().or(enclosing);
        let known_type = context_type.is_some_and(|type_name| types.contains_key(type_name));

        let mut check = |text: &Option<String>, key: &str| {
            if let (Some(text), Some(context_type)) = (text, context_type) {
                if known_type {
                    validate_expression(
                        text,
                        context_type,
                        types,
                        &format!("{path}/{key}"),
                        diagnostics,
                    );
                }
            }
        };
        check(&block.expr, "expr");
        check(&block.when, "when");
        check(&block.x, "x");
        check(&block.y, "y");

        if let Some(reduce) = &block.reduce {
            if !BLOCK_REDUCES.contains(&reduce.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "view.bad-reduce",
                    format!("{path}/reduce"),
                    format!(
                        "reduce \"{reduce}\" is not in {}",
                        BLOCK_REDUCES.join(" · ")
                    ),
                ));
            }
        }
        if let Some(days) = block.days {
            if days < 0 {
                diagnostics.push(Diagnostic::error(
                    "view.bad-days",
                    format!("{path}/days"),
                    "days must be non-negative",
                ));
            }
        }
        if let Some(limit) = block.limit {
            if limit < 0 {
                diagnostics.push(Diagnostic::error(
                    "view.bad-limit",
                    format!("{path}/limit"),
                    "limit must be non-negative",
                ));
            }
        }

        // Per-kind shape. Only known kinds reach here, so a missing member is a
        // real authoring mistake rather than a newer build's vocabulary.
        let shape = |missing: &str, diagnostics: &mut Vec<Diagnostic>| {
            diagnostics.push(Diagnostic::error(
                "view.block-shape",
                path.clone(),
                format!("{} block needs {missing}", block.kind),
            ));
        };
        match block.kind.as_str() {
            "callout" => {
                if block.text.as_deref().unwrap_or("").is_empty() {
                    shape("a text", diagnostics);
                }
            }
            "stat" => {}
            "chart" => {
                if block.x.is_none() {
                    shape("an x expression", diagnostics);
                }
                if block.y.is_none() {
                    shape("a y expression", diagnostics);
                }
            }
            "columns" | "repeat" | "conditional" => {
                if block.blocks.as_deref().unwrap_or_default().is_empty() {
                    // An empty container is a **stated** state, not a broken
                    // screen: the renderer draws an empty-state card, which is
                    // where "add a block to this row" belongs. Nothing here
                    // blocks a commit.
                    diagnostics.push(Diagnostic::warning(
                        "view.block-empty",
                        path.clone(),
                        format!("{} block has no child blocks yet", block.kind),
                    ));
                }
                if block.kind == "conditional" && block.when.is_none() {
                    shape("a when expression", diagnostics);
                }
            }
            _ => {}
        }

        for children in [block.blocks.as_deref(), block.else_blocks.as_deref()] {
            if let Some(children) = children {
                validate_blocks(
                    children,
                    &path,
                    context_type,
                    views,
                    types,
                    depth + 1,
                    diagnostics,
                );
            }
        }
    }
}

// ── rules.json ──────────────────────────────────────────────────────────────
pub fn validate_rules(
    rules: &RulesFile,
    views: &BTreeMap<String, ViewDef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let base = "content/rules.json#/pipelines";
    for (name, pipeline) in rules.pipelines.as_ref().unwrap_or(&EMPTY_PIPELINES) {
        let path = format!("{base}/{}", escape_pointer_segment(name));
        if pipeline.stages.is_empty() {
            diagnostics.push(Diagnostic::error(
                "pipeline.empty-stages",
                format!("{path}/stages"),
                format!("pipeline \"{name}\" declares no stages"),
            ));
        }
        let unique: BTreeSet<&String> = pipeline.stages.iter().collect();
        if unique.len() != pipeline.stages.len() {
            diagnostics.push(Diagnostic::error(
                "pipeline.duplicate-stages",
                format!("{path}/stages"),
                format!("pipeline \"{name}\" has duplicate stages"),
            ));
        }
        for (stage, gate) in pipeline.gates.as_ref().unwrap_or(&EMPTY_GATES) {
            let gate_path = format!("{path}/gates/{}", escape_pointer_segment(stage));
            if !pipeline.stages.contains(stage) {
                diagnostics.push(Diagnostic::error(
                    "pipeline.gate-undeclared-stage",
                    gate_path.clone(),
                    format!("gate \"{stage}\" is not a declared stage of {name}"),
                ));
            }
            if let Some(require) = &gate.require {
                if !pipeline.stages.contains(require) {
                    diagnostics.push(Diagnostic::error(
                        "pipeline.gate-undeclared-require",
                        format!("{gate_path}/require"),
                        format!("gate \"{stage}\" requires undeclared stage \"{require}\""),
                    ));
                }
            }
        }
        if let Some(complete) = &pipeline.complete_when {
            if !pipeline.stages.contains(complete) {
                diagnostics.push(Diagnostic::error(
                    "pipeline.bad-complete-when",
                    format!("{path}/completeWhen"),
                    format!("completeWhen \"{complete}\" is not a declared stage of {name}"),
                ));
            }
        }
        if let Some(min) = pipeline.proof.as_ref().and_then(|proof| proof.min_problems) {
            if min < 0 {
                diagnostics.push(Diagnostic::error(
                    "pipeline.bad-proof",
                    format!("{path}/proof/minProblems"),
                    "minProblems must be non-negative",
                ));
            }
        }
    }

    for (name, pipeline) in rules.pipelines.as_ref().unwrap_or(&EMPTY_PIPELINES) {
        let path = format!("content/rules.json#/pipelines/{name}");
        // The two stage-name conventions the engine encodes (§3.5): `proof`
        // guards `proved`, `anchorSkip` governs `anchored`. A pipeline that
        // declares one without its stage gets an advisory warning rather than
        // silence — the mechanism is data, and this is the one place it is
        // bound to a name.
        if pipeline.proof.is_some() && !pipeline.stages.iter().any(|stage| stage == "proved") {
            diagnostics.push(Diagnostic::warning(
                "pipeline.proof-without-stage",
                format!("{path}/proof"),
                format!(
                    "pipeline \"{name}\" declares a proof gate but no \"proved\" stage — the gate will never fire"
                ),
            ));
        }
        if pipeline.anchor_skip.is_some()
            && !pipeline.stages.iter().any(|stage| stage == "anchored")
        {
            diagnostics.push(Diagnostic::warning(
                "pipeline.anchor-without-stage",
                format!("{path}/anchorSkip"),
                format!(
                    "pipeline \"{name}\" declares anchorSkip but no \"anchored\" stage — the rule will never fire"
                ),
            ));
        }
    }

    if let Some(scheduler) = &rules.scheduler {
        if !crate::rules::SCHEDULERS.contains(&scheduler.name.as_str()) {
            diagnostics.push(Diagnostic::error(
                "scheduler.unknown",
                "content/rules.json#/scheduler/name",
                format!(
                    "scheduler \"{}\" is not one of {}",
                    scheduler.name,
                    sorted_join(&crate::rules::SCHEDULERS)
                ),
            ));
        }
        // The fixed ladder is explicit offsets from the completion anchor
        // (§3.5): a zero or negative offset is not a schedule.
        if let Some(fixed) = &scheduler.fixed {
            for (index, interval) in fixed
                .intervals
                .as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
            {
                if *interval <= 0 {
                    diagnostics.push(Diagnostic::error(
                        "scheduler.bad-interval",
                        format!("content/rules.json#/scheduler/fixed/intervals/{index}"),
                        format!("fixed intervals are whole days from the anchor; got {interval}"),
                    ));
                }
            }
        }
        if let Some(retention) = scheduler
            .fsrs
            .as_ref()
            .and_then(|fsrs| fsrs.desired_retention)
            && !(0.5..=0.99).contains(&retention)
        {
            diagnostics.push(Diagnostic::error(
                "scheduler.bad-retention",
                "content/rules.json#/scheduler/fsrs/desiredRetention",
                format!("desiredRetention must be between 0.5 and 0.99; got {retention}"),
            ));
        }
        if let Some(weights) = scheduler
            .fsrs
            .as_ref()
            .and_then(|fsrs| fsrs.weights.as_ref())
            && weights.len() != 21
        {
            diagnostics.push(Diagnostic::error(
                "scheduler.bad-weights",
                "content/rules.json#/scheduler/fsrs/weights",
                format!(
                    "FSRS-6 takes 21 weights; got {} — omit the key to use the default set",
                    weights.len()
                ),
            ));
        }
    }

    // Derived metrics (§3.1): each names a saved view and a fold over it. A
    // metric pointing at an undeclared view or an unknown fold is a finding at
    // load, not a blank number on the Progress panel.
    for (name, metric) in rules.metrics.as_ref().unwrap_or(&EMPTY_METRICS) {
        let base = format!(
            "content/rules.json#/metrics/{}",
            escape_pointer_segment(name)
        );
        if !views.contains_key(&metric.view) {
            diagnostics.push(Diagnostic::error(
                "metric.unknown-view",
                format!("{base}/view"),
                format!(
                    "metric \"{name}\" folds view \"{}\", which is not declared",
                    metric.view
                ),
            ));
        }
        if !crate::model::BLOCK_REDUCES.contains(&metric.reduce_kind()) {
            diagnostics.push(Diagnostic::error(
                "metric.bad-reduce",
                format!("{base}/reduce"),
                format!(
                    "metric \"{name}\" reduces with \"{}\" — declared: {}",
                    metric.reduce_kind(),
                    crate::model::BLOCK_REDUCES.join(", ")
                ),
            ));
        }
    }

    // §5's lints: an id must be in the vocabulary, a severity must be one of
    // the two declared values, and a waiver must name a real rule and say why.
    // None of these block a commit — the doctor reports, the student decides.
    if let Some(lint) = &rules.lint {
        for (id, setting) in lint.rules.as_ref().into_iter().flatten() {
            let path = format!("content/rules.json#/lint/rules/{id}/severity");
            if crate::rules::lint_rule(id).is_none() {
                diagnostics.push(Diagnostic::warning(
                    "lint.unknown-rule",
                    path.replace("/severity", ""),
                    format!(
                        "\"{id}\" is not a rule this build knows — declared: {}",
                        crate::rules::LINT_RULES
                            .iter()
                            .map(|rule| rule.id)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
            if let Some(severity) = &setting.severity
                && !crate::rules::LINT_SEVERITIES.contains(&severity.as_str())
            {
                diagnostics.push(Diagnostic::error(
                    "lint.bad-severity",
                    path,
                    format!(
                        "severity \"{severity}\" is not one of {}",
                        crate::rules::LINT_SEVERITIES.join(", ")
                    ),
                ));
            }
        }
        let mut waived = BTreeSet::new();
        for (index, waiver) in lint
            .waivers
            .as_deref()
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let path = format!("content/rules.json#/lint/waivers/{index}");
            if crate::rules::lint_rule(&waiver.id).is_none() {
                diagnostics.push(Diagnostic::warning(
                    "lint.unknown-waiver",
                    format!("{path}/id"),
                    format!("\"{}\" is not a rule this build knows", waiver.id),
                ));
            }
            if !waived.insert(waiver.id.clone()) {
                diagnostics.push(Diagnostic::warning(
                    "lint.duplicate-waiver",
                    format!("{path}/id"),
                    format!("\"{}\" is waived more than once", waiver.id),
                ));
            }
        }
    }
}

pub fn validate_pipelines_referenced(
    types: &BTreeMap<String, TypeDef>,
    rules: &RulesFile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let declared: BTreeSet<&String> = rules
        .pipelines
        .as_ref()
        .unwrap_or(&EMPTY_PIPELINES)
        .keys()
        .collect();
    for (name, def) in types {
        match &def.pipeline {
            Some(pipeline) if !declared.contains(pipeline) => {
                diagnostics.push(Diagnostic::error(
                    "pipeline.unknown",
                    format!(
                        "content/types.json#/types/{}/pipeline",
                        escape_pointer_segment(name)
                    ),
                    format!("type \"{name}\" uses undeclared pipeline \"{pipeline}\""),
                ));
            }
            None if def.trackable == Some(true) => {
                diagnostics.push(Diagnostic::error(
                    "pipeline.missing",
                    format!("content/types.json#/types/{}", escape_pointer_segment(name)),
                    format!("type \"{name}\" is trackable but declares no pipeline"),
                ));
            }
            _ => {}
        }
    }
}

// ── settings (§4.2: the settings screen is generated from FieldDefs) ────────

/// `rules.json#/study` is the app's own settings document, validated with the
/// same `FieldDef` vocabulary the settings screen is generated from — and from
/// the same row table, so a row cannot be added to the screen without a
/// validator that understands it.
pub fn settings_schema(rules: &RulesFile) -> Vec<Diagnostic> {
    let study_rows = crate::doc_edit::settings_ops::FIELDS
        .iter()
        .filter(|def| def.path.first() == Some(&"study"))
        .collect::<Vec<_>>();
    let Some(JSONValue::Object(study)) = &rules.study else {
        return Vec::new();
    };
    let declared = study_rows
        .iter()
        .filter_map(|def| def.path.get(1).copied())
        .collect::<Vec<_>>()
        .join(", ");
    let mut diagnostics = Vec::new();
    for (key, value) in study {
        let Some(def) = study_rows
            .iter()
            .find(|def| def.path.get(1) == Some(&key.as_str()))
        else {
            // A row that moved keeps being read (the reader falls back), so the
            // file that carries it must not stop loading — but it is told where
            // the value lives now.
            let moved = crate::doc_edit::settings_ops::find(key)
                .filter(|def| def.path.first() != Some(&"study"));
            match moved {
                Some(def) => diagnostics.push(Diagnostic::warning(
                    "settings.moved-key",
                    format!("content/rules.json#/study/{key}"),
                    format!(
                        "setting \"{key}\" lives at content/rules.json#/{} now — this file's value is still read",
                        def.path.join("/")
                    ),
                )),
                None => diagnostics.push(Diagnostic::error(
                    "settings.unknown-key",
                    format!("content/rules.json#/study/{key}"),
                    format!("unknown setting \"{key}\" — declared: {declared}"),
                )),
            }
            continue;
        };
        let Some(field) = crate::doc_edit::settings_ops::field(def.key) else {
            continue;
        };
        if let Some(problem) = field_value_problem(&field, value) {
            diagnostics.push(Diagnostic::error(
                "settings.invalid-value",
                format!("content/rules.json#/study/{key}"),
                problem,
            ));
        }
    }
    diagnostics
}

// ── shell.json ──────────────────────────────────────────────────────────────

pub fn validate_shell(
    shell: Option<&ShellFile>,
    views: &BTreeMap<String, ViewDef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(shell) = shell else {
        return;
    };
    for (index, entry) in shell
        .navigation
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        if !views.contains_key(&entry.view) {
            diagnostics.push(Diagnostic::error(
                "shell.unknown-view",
                format!("content/shell.json#/navigation/{index}/view"),
                format!(
                    "navigation entry \"{}\" references missing view \"{}\"",
                    entry.title, entry.view
                ),
            ));
        }
    }

    // Keybindings (§4.8 P2): optional, but collisions are diagnosed — two
    // commands on one key is an ambiguous config, an integrity finding.
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for (index, raw) in shell
        .keybindings
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let base = format!("content/shell.json#/keybindings/{index}");
        let command = raw.get("command").and_then(JSONValue::as_str);
        let key = raw.get("key").and_then(JSONValue::as_str);
        let (Some(command), Some(key)) = (command, key) else {
            diagnostics.push(Diagnostic::error(
                "shell.keybinding-shape",
                base,
                "a keybinding needs string \"command\" and \"key\"",
            ));
            continue;
        };
        match seen.get(key) {
            Some(first) if first != command => {
                diagnostics.push(Diagnostic::error(
                    "shell.keybinding-collision",
                    format!("{base}/key"),
                    format!(
                        "key \"{key}\" is bound to both \"{first}\" and \"{command}\" — collisions are diagnosed, bindings stay optional"
                    ),
                ));
            }
            _ => {
                seen.insert(key.to_string(), command.to_string());
            }
        }

        // Unknown commands warn rather than fail so plans authored against
        // newer feature registries can still load (§4.8 P2).
        if !binding_resolves(command) {
            diagnostics.push(Diagnostic::warning(
                "shell.keybinding-unknown-command",
                format!("{base}/command"),
                format!(
                    "keybinding names \"{command}\", which is not in the registry — SAM --commands lists the ids"
                ),
            ));
        }
        if !command_registry::key_is_readable(key) {
            diagnostics.push(Diagnostic::warning(
                "shell.keybinding-unknown-key",
                format!("{base}/key"),
                format!(
                    "key \"{key}\" is not readable — write `mod+z`-style bindings; modifiers are {} and named keys are {}",
                    command_registry::KEY_MODIFIERS.join("|"),
                    command_registry::KEY_NAMES.join("|")
                ),
            ));
        }
    }
}

/// Is this id something the dispatcher would accept? The static registry, or
/// the per-type pair the registry generates from the active types.
fn binding_resolves(command: &str) -> bool {
    if command_registry::resolve(command).is_some() {
        return true;
    }
    match command.rsplit_once('.') {
        Some((type_name, "new" | "paste")) => !type_name.is_empty(),
        _ => false,
    }
}

// ── records ─────────────────────────────────────────────────────────────────

pub fn validate_records(
    positioned: &[PositionedRecord],
    types: &BTreeMap<String, TypeDef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Full id map first so forward references between records resolve
    // (§3.7: a link may point at a record defined later in the batch).
    // First occurrence by input order wins the slot.
    let mut by_id: BTreeMap<&str, &PositionedRecord> = BTreeMap::new();
    for positioned_record in positioned {
        by_id
            .entry(positioned_record.record.id.as_str())
            .or_insert(positioned_record);
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut parent_chain_checked: BTreeSet<String> = BTreeSet::new();

    for positioned_record in positioned {
        let record = &positioned_record.record;
        let file = positioned_record.file.as_str();
        let line = positioned_record.line;
        let base = format!("{file}#{}", escape_pointer_segment(&record.id));

        if record.id.is_empty() {
            diagnostics.push(
                Diagnostic::error("record.empty-id", file, "record id must be non-empty")
                    .at_line(line),
            );
        }
        // `by_id` is pre-populated for forward references; `seen` tracks
        // first-occurrence-in-input, so a repeated id is the duplicate.
        if !seen.insert(record.id.as_str()) {
            let first = by_id[record.id.as_str()];
            diagnostics.push(
                Diagnostic::error(
                    "record.duplicate-id",
                    file,
                    format!(
                        "id \"{}\" already used at {}:{}",
                        record.id, first.file, first.line
                    ),
                )
                .at_line(line),
            );
        }

        let Some(def) = types.get(&record.type_) else {
            diagnostics.push(
                Diagnostic::error(
                    "record.unknown-type",
                    file,
                    format!(
                        "record \"{}\": unknown type \"{}\"",
                        record.id, record.type_
                    ),
                )
                .at_line(line),
            );
            continue;
        };

        // Fields: declared, non-relation, value matches FieldDef.type.
        for (key, value) in &record.fields {
            let field_path = format!("{base}/fields/{}", escape_pointer_segment(key));
            let Some(field) = def.field(key) else {
                diagnostics.push(
                    Diagnostic::error(
                        "record.unknown-field",
                        field_path,
                        format!(
                            "record \"{}\": field \"{key}\" is not declared on {}",
                            record.id, record.type_
                        ),
                    )
                    .at_line(line),
                );
                continue;
            };
            if field.is_relation() {
                diagnostics.push(
                    Diagnostic::error(
                        "record.relation-in-fields",
                        field_path,
                        format!("relation \"{key}\" must live in links, not fields"),
                    )
                    .at_line(line),
                );
                continue;
            }
            if field.is_formula() {
                diagnostics.push(
                    Diagnostic::error(
                        "record.formula-persisted",
                        field_path,
                        format!("formula \"{key}\" is derived — never persist it in fields"),
                    )
                    .at_line(line),
                );
                continue;
            }
            if field.type_ == "progress" {
                diagnostics.push(
                    Diagnostic::error(
                        "record.progress-persisted",
                        field_path,
                        format!("progress \"{key}\" is derived — never persist it in fields"),
                    )
                    .at_line(line),
                );
                continue;
            }
            if let Some(problem) = field_value_problem(field, value) {
                diagnostics.push(
                    Diagnostic::error(
                        "record.invalid-value",
                        field_path,
                        format!("record \"{}\", field \"{key}\": {problem}", record.id),
                    )
                    .at_line(line),
                );
            }
        }

        // Links: declared relation or parent; targets exist and are of the
        // declared target type; cardinality respected.
        let allowed = def.allowed_link_keys();
        for (key, targets) in &record.links {
            let link_path = format!("{base}/links/{}", escape_pointer_segment(key));
            if !allowed.contains(key) {
                diagnostics.push(
                    Diagnostic::error(
                        "record.unknown-link",
                        link_path,
                        format!(
                            "record \"{}\": link \"{key}\" is not a declared relation of {}",
                            record.id, record.type_
                        ),
                    )
                    .at_line(line),
                );
                continue;
            }
            let is_parent = def.parent.as_deref() == Some(key.as_str());
            let expected_type = if is_parent {
                def.parent.clone().unwrap_or_default()
            } else {
                def.field(key)
                    .and_then(|field| field.to.clone())
                    .unwrap_or_default()
            };
            let single = is_parent || def.field(key).is_some_and(FieldDef::is_one);
            if single && targets.len() > 1 {
                diagnostics.push(
                    Diagnostic::error(
                        "record.cardinality",
                        link_path.clone(),
                        format!(
                            "link \"{key}\" holds {} targets but declares cardinality one",
                            targets.len()
                        ),
                    )
                    .at_line(line),
                );
            }
            for target in targets {
                if !by_id.contains_key(target.as_str()) {
                    diagnostics.push(
                        Diagnostic::error(
                            "record.dangling-link",
                            link_path.clone(),
                            format!("link \"{key}\" → \"{target}\" does not exist"),
                        )
                        .at_line(line),
                    );
                }
            }
            for target in targets {
                if let Some(found) = by_id.get(target.as_str()) {
                    if found.record.type_ != expected_type {
                        diagnostics.push(
                            Diagnostic::error(
                                "record.link-type-mismatch",
                                link_path.clone(),
                                format!(
                                    "link \"{key}\" → \"{target}\" is of type {}; expected {expected_type}",
                                    found.record.type_
                                ),
                            )
                            .at_line(line),
                        );
                    }
                }
            }
        }

        // Record-level parent cycle: parent edges must terminate (§3.7).
        if def.parent.is_some() && !parent_chain_checked.contains(&record.id) {
            let mut visited: BTreeSet<String> = BTreeSet::new();
            let mut cursor = Some(record);
            while let Some(current) = cursor {
                if !visited.insert(current.id.clone()) {
                    diagnostics.push(
                        Diagnostic::error(
                            "record.parent-cycle",
                            file,
                            format!(
                                "parent chain from \"{}\" cycles at \"{}\"",
                                record.id, current.id
                            ),
                        )
                        .at_line(line),
                    );
                    break;
                }
                cursor = types
                    .get(&current.type_)
                    .and_then(|def| def.parent.as_deref())
                    .and_then(|parent_type| current.links.get(parent_type))
                    .and_then(|ids| ids.first())
                    .and_then(|id| by_id.get(id.as_str()))
                    .map(|positioned| &positioned.record);
            }
            parent_chain_checked.extend(visited);
        }
    }
}

/// §3.4 field-type conformance for one persisted value.
pub fn field_value_problem(field: &FieldDef, value: &JSONValue) -> Option<String> {
    let number = || match value {
        JSONValue::Number(number) => number.as_f64(),
        _ => None,
    };
    match field.type_.as_str() {
        "text" | "longtext" => match value {
            JSONValue::String(_) => None,
            _ => Some("expected a string".into()),
        },
        "number" => match number() {
            None => Some("expected a number".into()),
            Some(value) if !value.is_finite() => Some("number must be finite".into()),
            _ => None,
        },
        "duration" => match number() {
            None => Some("expected a number of minutes".into()),
            Some(value) if !(value >= 0.0 && value.is_finite()) => {
                Some("duration must be non-negative minutes".into())
            }
            _ => None,
        },
        "date" => match value {
            JSONValue::String(text) => {
                if is_date(text) {
                    None
                } else {
                    Some(format!("\"{text}\" is not a YYYY-MM-DD date"))
                }
            }
            _ => Some("expected a YYYY-MM-DD date string".into()),
        },
        "daterange" => match value {
            JSONValue::Object(map) => {
                for part in ["start", "end"] {
                    let valid =
                        matches!(map.get(part), Some(JSONValue::String(text)) if is_date(text));
                    if !valid {
                        return Some(format!("{part} must be a YYYY-MM-DD date"));
                    }
                }
                None
            }
            _ => Some("expected {start, end}".into()),
        },
        "bool" => match value {
            JSONValue::Bool(_) => None,
            _ => Some("expected a boolean".into()),
        },
        "select" => match value {
            JSONValue::String(text) => {
                let declared = field
                    .options
                    .as_ref()
                    .is_some_and(|options| options.contains(text));
                if declared {
                    None
                } else {
                    Some(format!("\"{text}\" is not an option of {}", field.key))
                }
            }
            _ => Some(format!(
                "expected one of {:?}",
                field.options.clone().unwrap_or_default()
            )),
        },
        "multiSelect" => match value {
            JSONValue::Array(items) => {
                for item in items {
                    let valid = matches!(item, JSONValue::String(text)
                        if field.options.as_ref().is_some_and(|options| options.contains(text)));
                    if !valid {
                        return Some(format!("values must be declared options of {}", field.key));
                    }
                }
                None
            }
            _ => Some("expected an array of options".into()),
        },
        "rating" => match number() {
            Some(value) if (1.0..=5.0).contains(&value) && value == value.round() => None,
            _ => Some("rating must be an integer 1–5".into()),
        },
        "url" => match value {
            JSONValue::String(text) => {
                if is_absolute_url(text) {
                    None
                } else {
                    Some(format!("\"{text}\" is not an absolute URL"))
                }
            }
            _ => Some("expected a URL string".into()),
        },
        // `json` is the escape hatch, and relation/formula/progress are the
        // caller's business.
        _ => None,
    }
}

/// `YYYY-MM-DD`, month and day ranges checked, no calendar arithmetic (§3.4
/// calls it a civil date, not an instant).
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let (year, month, day) = (parts[0], parts[1], parts[2]);
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    if !year.bytes().all(|b| b.is_ascii_digit())
        || !month.bytes().all(|b| b.is_ascii_digit())
        || !day.bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    (1..=9999).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// An absolute URL: a scheme, then `:`. Scheme-relative (`//host`) is refused
/// because §3.4 says "validate URL scheme before opening".
fn is_absolute_url(text: &str) -> bool {
    let Some(colon) = text.find(':') else {
        return false;
    };
    let scheme = &text[..colon];
    if scheme.is_empty() || !scheme.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return false;
    }
    if !scheme
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return false;
    }
    !text[colon + 1..].starts_with(|c: char| c.is_ascii_whitespace())
}

fn sorted_join(values: &[&str]) -> String {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode;
    use crate::diagnostic::Severity;
    use crate::json_cursor::Cursor;
    use crate::strict_json;

    fn types_from(source: &str) -> BTreeMap<String, TypeDef> {
        let value = strict_json::parse(source, "content/types.json").expect("fixture parses");
        decode::types_file(&Cursor::root("content/types.json", &value)).expect("decodes")
    }

    #[test]
    fn a_formula_cycle_is_rejected_at_load() {
        let types = types_from(
            r#"{"schemaVersion":1,"types":{"t":{"fields":[
               {"key":"x","type":"number"},
               {"key":"f1","type":"formula","expr":"pct(f2, x)"},
               {"key":"f2","type":"formula","expr":"pct(f1, x)"}]}}}"#,
        );
        let mut diagnostics = Vec::new();
        validate_types(&types, &mut diagnostics);
        assert!(
            diagnostics.iter().any(|d| d.code == "formula.cycle"),
            "a formula cycle is an integrity finding: {diagnostics:?}"
        );
    }

    #[test]
    fn a_parent_cycle_is_rejected_at_load() {
        let types = types_from(
            r#"{"schemaVersion":1,"types":{
               "a":{"parent":"b","fields":[{"key":"x","type":"text"}]},
               "b":{"parent":"a","fields":[{"key":"x","type":"text"}]}}}"#,
        );
        let mut diagnostics = Vec::new();
        validate_types(&types, &mut diagnostics);
        assert!(diagnostics.iter().any(|d| d.code == "types.parent-cycle"));
    }

    #[test]
    fn field_values_are_checked_against_the_closed_vocabulary() {
        let duration = FieldDef {
            key: "est".into(),
            type_: "duration".into(),
            label: None,
            to: None,
            cardinality: None,
            required: None,
            options: None,
            expr: None,
        };
        assert!(field_value_problem(&duration, &serde_json::json!(30)).is_none());
        assert!(field_value_problem(&duration, &serde_json::json!(-1)).is_some());
        assert!(field_value_problem(&duration, &serde_json::json!("thirty")).is_some());

        let date = FieldDef {
            type_: "date".into(),
            key: "start".into(),
            ..duration.clone()
        };
        assert!(field_value_problem(&date, &serde_json::json!("2026-09-20")).is_none());
        assert!(field_value_problem(&date, &serde_json::json!("2026-13-20")).is_some());

        let url = FieldDef {
            type_: "url".into(),
            key: "source".into(),
            ..duration
        };
        assert!(field_value_problem(&url, &serde_json::json!("https://example.org/x")).is_none());
        assert!(field_value_problem(&url, &serde_json::json!("/relative")).is_some());
    }

    #[test]
    fn a_screen_is_checked_against_the_block_vocabulary_the_views_and_the_boundary() {
        let types = types_from(
            r#"{"schemaVersion":1,"types":{"topic":{
               "fields":[{"key":"est","type":"duration"},{"key":"title","type":"text"}]}}}"#,
        );
        let views_source = r#"{"schemaVersion":1,"views":{
           "topic.list":{"type":"topic","layout":"list"},
           "topic.screen":{"type":"topic","layout":"list","blocks":[
             {"kind":"stat","view":"topic.list","expr":"est","reduce":"median"},
             {"kind":"table","view":"nowhere.at.all"},
             {"kind":"callout"},
             {"kind":"chart","expr":"est"},
             {"kind":"sparkline"},
             {"kind":"stat","expr":"est +","reduce":"sum"},
             {"kind":"stat","label":"fine","expr":"est","reduce":"sum"},
             {"kind":"columns"}
           ]}}}"#;
        let value = strict_json::parse(views_source, "content/views.json").expect("fixture parses");
        let views =
            decode::views_file(&Cursor::root("content/views.json", &value)).expect("decodes");
        let mut diagnostics = Vec::new();
        validate_views(&views, &types, &mut diagnostics);
        let code = |want: &str| {
            diagnostics
                .iter()
                .find(|d| d.code == want)
                .unwrap_or_else(|| panic!("no {want} in {diagnostics:?}"))
        };
        assert_eq!(code("view.bad-reduce").severity, Severity::Error);
        assert_eq!(code("view.unknown-view").severity, Severity::Error);
        assert_eq!(code("view.block-shape").severity, Severity::Error);
        assert_eq!(code("expr.syntax").severity, Severity::Error);
        // Forward compatibility: a kind this build does not know loads, with a
        // warning and the placeholder as the renderer's answer (§0 rule 3).
        assert_eq!(code("view.unknown-block").severity, Severity::Warning);
        // An empty container is a stated state, not a broken screen.
        assert_eq!(code("view.block-empty").severity, Severity::Warning);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.code != "view.block-shape" || !d.path.contains("blocks/6")),
            "the well-formed stat is not flagged: {diagnostics:?}"
        );
    }

    #[test]
    fn a_composed_screen_is_checked_for_what_it_draws() {
        let types = types_from(
            r#"{"schemaVersion":1,"types":{"topic":{
               "fields":[{"key":"est","type":"duration"},{"key":"title","type":"text"}]}}}"#,
        );
        let views_source = r#"{"schemaVersion":1,"views":{
           "topic.list":{"type":"topic","layout":"list"},
           "nothing.at.all":{},
           "no.layout":{"type":"topic"},
           "no.type":{"layout":"list"},
           "composed.screen":{"components":[
             {"surface":"week-chart","span":1},
             {"surface":"week-chart","span":2},
             {"surface":"today-focus","span":0},
             {"surface":"plan-spine","span":3},
             {}]},
           "composed.empty":{"components":[]}}}"#;
        let value = strict_json::parse(views_source, "content/views.json").expect("fixture parses");
        let views =
            decode::views_file(&Cursor::root("content/views.json", &value)).expect("decodes");
        let mut diagnostics = Vec::new();
        validate_views(&views, &types, &mut diagnostics);
        let code = |want: &str| {
            diagnostics
                .iter()
                .find(|d| d.code == want)
                .unwrap_or_else(|| panic!("no {want} in {diagnostics:?}"))
        };
        // A view must draw something (COMPOSER §2.1).
        assert_eq!(code("view.empty").severity, Severity::Error);
        assert_eq!(code("view.layout-missing").severity, Severity::Error);
        assert_eq!(code("view.type-missing").severity, Severity::Error);
        assert_eq!(code("component.surface-missing").severity, Severity::Error);
        // Duplicate components produce an advisory warning rather than a fatal error.
        assert_eq!(code("view.duplicate-component").severity, Severity::Warning);
        // A span is judged here, not by the loader: a width below 1 cannot be
        // drawn at all (error), and a width above 2 is drawn full width
        // (warning) rather than dropped.
        assert_eq!(code("component.span-invalid").severity, Severity::Error);
        assert!(
            code("component.span-invalid")
                .path
                .contains("components/2/span"),
            "the finding stands on the member that carries it"
        );
        assert_eq!(code("component.span-wide").severity, Severity::Warning);
        assert!(
            code("component.span-wide")
                .path
                .contains("components/3/span")
        );
        // 1 and 2 are the widths this build draws, written or defaulted.
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.code.starts_with("component.span")
                    || !(d.path.contains("components/0/") || d.path.contains("components/1/"))),
            "the spans this build draws are not flagged: {diagnostics:?}"
        );
        // An empty composition is a stated screen, not an empty view.
        assert!(
            diagnostics
                .iter()
                .all(|d| d.code != "view.empty" || !d.path.contains("composed.empty")),
            "components: [] draws the empty state: {diagnostics:?}"
        );
        // The typed views are untouched by all of this: no new finding lands on
        // them, and none of their existing ones disappear.
        assert!(
            diagnostics.iter().all(|d| !d.path.contains("topic.list")),
            "a well-formed typed view is not flagged: {diagnostics:?}"
        );
    }

    #[test]
    fn a_tree_past_the_recursion_boundary_is_a_warning_rather_than_a_wall() {
        let types = types_from(
            r#"{"schemaVersion":1,"types":{"topic":{"fields":[{"key":"est","type":"duration"}]}}}"#,
        );
        // MAX_BLOCK_DEPTH + 1 nested columns, built as text so the fixture is
        // the file a student would have hand-edited.
        let mut nested = r#"{"kind":"callout","text":"deep"}"#.to_string();
        for _ in 0..(MAX_BLOCK_DEPTH + 1) {
            nested = format!(r#"{{"kind":"columns","blocks":[{nested}]}}"#);
        }
        let views_source = format!(
            r#"{{"schemaVersion":1,"views":{{"deep":{{"type":"topic","layout":"list","blocks":[{nested}]}}}}}}"#
        );
        let value = strict_json::parse(&views_source, "content/views.json").expect("parses");
        let views =
            decode::views_file(&Cursor::root("content/views.json", &value)).expect("decodes");
        let mut diagnostics = Vec::new();
        validate_views(&views, &types, &mut diagnostics);
        let finding = diagnostics
            .iter()
            .find(|d| d.code == "view.block-depth")
            .expect("the boundary is named");
        assert_eq!(finding.severity, Severity::Warning);
        assert!(finding.message.contains(&MAX_BLOCK_DEPTH.to_string()));
    }
}
