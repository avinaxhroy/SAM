//! Phase 4: `--uicheck` structural verification dump (Appendix C.6, §4.8, §4.9).
//!
//! Emits declared UI command structures from the registry and active config headlessly
//! (without requiring a GUI window or webview runtime).
//!
//! Verifies that menu items, settings controls, and palette entries bind to valid
//! registry command IDs with declared surface placement and document target paths.

use crate::command_registry::{self, CommandEffect};
use crate::resolved_config::ResolvedConfig;

/// The dump, one line per fact, in the scaffold's format (Appendix C.6).
pub fn dump(config: &ResolvedConfig) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "SAM uicheck · plan {} · revision {}",
        config
            .plan_root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "?".into()),
        &config.revision[..8.min(config.revision.len())]
    ));

    lines.push("titlebar".into());
    lines.push("  search pill → dispatch app.palette".into());
    lines.push("  gear → dispatch app.openSettings".into());
    lines.push("  source → dispatch app.toggleSourcePane".into());

    lines.push("rail".into());
    match config
        .shell
        .as_ref()
        .and_then(|shell| shell.navigation.as_ref())
    {
        Some(entries) if !entries.is_empty() => {
            for entry in entries {
                lines.push(format!(
                    "  disc {} → view {} · dispatch rail.select",
                    entry.title, entry.view
                ));
            }
        }
        _ => lines.push("  (no shell navigation declared — the rail shows no destinations)".into()),
    }
    lines.push("  disc + (quiet) → dispatch list.new · type.new [sidebar.add]".into());

    // The card head describes the destination the app would open first, which is
    // the same rule `AppModel` uses: the shell's first entry, else the first
    // view. A dump that guessed differently from the app would be a gate that
    // cannot fail (Appendix C.5).
    let selected = config
        .shell
        .as_ref()
        .and_then(|shell| shell.navigation.as_ref())
        .and_then(|entries| entries.first())
        .map(|entry| entry.view.clone())
        .or_else(|| config.views.keys().next().cloned());

    match selected.and_then(|view| config.views.get(&view).map(|def| (view, def))) {
        Some((view, view_def)) => {
            // A composed screen declares no type (COMPOSER §2): its head is the
            // composer's — add, reorder, restyle, remove — not the record
            // table's. Both are dumped when a view oddly carries both, because
            // the file is the thing being described.
            if view_def.components.is_some() {
                lines.push(format!(
                    "screen [{view} · composed: {}]",
                    view_def
                        .components
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .map(|component| component.surface.as_str())
                        .collect::<Vec<_>>()
                        .join(" · ")
                ));
                lines.push("  add component → view.setComponents [view.head]".into());
                lines.push("  edit screen → screen.edit [view.head]".into());
            }
            let Some(type_name) = view_def.type_.as_deref() else {
                lines.push("card head [a composed screen — no record table]".into());
                plan_sections(&mut lines, config);
                return lines;
            };
            lines.push(format!("card head [{view} · {type_name}]"));
            lines.push(format!(
                "  + New {type_name} → dispatch {type_name}.new [recordTable.toolbar]"
            ));
            lines.push(format!(
                "  Paste lines → dispatch {type_name}.paste → apply [recordTable.paste]"
            ));
            // The column menu belongs to the layouts that draw cells — `table`
            // and `list` (§3.6). A destination whose first record block is a
            // cardGrid, board, calendar or chart draws no header, so the dump
            // must not declare one: a gate that asserts a control the app does
            // not have is the failure this whole section exists to prevent
            // (§4.8 P3's header is the schema editor *of a cell layout*).
            let drawn = match view_def.blocks.as_deref().and_then(|blocks| blocks.first()) {
                Some(block) => block.kind.clone(),
                None => view_def.layout.clone().unwrap_or_else(|| "(none)".into()),
            };
            let cells = drawn == "table" || drawn == "list";
            lines.push(format!(
                "columnMenu [{}]",
                if cells {
                    format!("every header of the {drawn} block")
                } else {
                    format!("none — this destination draws {drawn}; columns are edited through view.setLayout, the palette and the view editor")
                }
            ));
            let shown = view_def.columns.clone().unwrap_or_else(|| {
                config
                    .types
                    .get(type_name)
                    .map(|def| {
                        def.fields
                            .iter()
                            .filter(|field| field.type_ != "progress")
                            .map(|field| field.key.clone())
                            .collect()
                    })
                    .unwrap_or_default()
            });
            for key in &shown {
                let Some(field) = config.types.get(type_name).and_then(|def| def.field(key)) else {
                    continue;
                };
                if !cells {
                    continue;
                }
                // Appendix C.2's known defect: the scaffold printed
                // `column.choices` for every column while the menu offered it
                // only for the select kinds. The dump reads the same predicate
                // the menu does, or the gate asserts a control that is not there.
                let choices = if field.type_ == "select" || field.type_ == "multiSelect" {
                    " · column.choices"
                } else {
                    ""
                };
                lines.push(format!(
                    "  header {}:{} → column.rename · column.retype{} · column.duplicate · column.hide · column.reorder · column.delete · copy-json · copy-path",
                    field.key, field.type_, choices
                ));
            }
            let hidden: Vec<&str> = config
                .types
                .get(type_name)
                .map(|def| {
                    def.fields
                        .iter()
                        .map(|field| field.key.as_str())
                        .filter(|key| !shown.iter().any(|column| column == key))
                        .collect()
                })
                .unwrap_or_default();
            if cells {
                lines.push(format!(
                    "  header + → column.show ({}) · column.new [columnMenu.plus]",
                    if hidden.is_empty() {
                        "none hidden".to_string()
                    } else {
                        hidden.join(" · ")
                    }
                ));
            }

            // Phase 5's screen head and view editor: the layout control is the
            // design's *Table · Board · Calendar* control (§4.8 P3), and the
            // editor's rows are the query keys and the block tree as JSON. Every
            // id below must render, or the diff says so.
            lines.push(format!("view head [{view}]"));
            lines.push(format!(
                "  show as… → view.setLayout ({}) [view.head]",
                crate::model::RECORD_BLOCK_KINDS.join(" · ")
            ));
            lines.push("  edit this view… → view.edit [view.head]".into());
            lines.push(format!("view editor [{view}]"));
            lines.push(format!(
                "  filter {} → view.setFilter [view.editor]",
                view_def.filter.as_deref().unwrap_or("(none)")
            ));
            lines.push(format!(
                "  sort {} → view.setSort [view.editor]",
                view_def.sort.as_deref().unwrap_or("(none)")
            ));
            lines.push(format!(
                "  group {} → view.setGroup [view.editor]",
                view_def.group.as_deref().unwrap_or("(none)")
            ));
            lines.push(format!(
                "  limit {} → view.setLimit [view.editor]",
                view_def
                    .limit
                    .map(|limit| limit.to_string())
                    .unwrap_or_else(|| "(none)".into())
            ));
            lines.push(format!(
                "  columns {} → view.setColumns [view.editor]",
                shown.join(" · ")
            ));
            lines.push("  add a block → view.block.add [view.editor]".into());
            match view_def.blocks.as_deref() {
                Some(blocks) if !blocks.is_empty() => {
                    lines.push(format!("blocks [{view}]"));
                    for (index, block) in blocks.iter().enumerate() {
                        lines.push(format!(
                            "  {index} · {} · {} → view.setLayout · view.block.set · view.block.remove [view.editor]",
                            block.kind,
                            match &block.view {
                                Some(source) => format!("view {source}"),
                                None => "this view".to_string(),
                            }
                        ));
                    }
                }
                _ => lines.push(format!(
                    "blocks [{view}] — none declared: this view draws itself as one {} block",
                    view_def.layout.as_deref().unwrap_or("(none)")
                )),
            }

            // Phase 6's panel (§3.6: the view document chooses its surface).
            // The dump names the ids the panel renders, so a panel that stops
            // rendering one is a diffable finding rather than a screen nobody
            // looked at.
            if let Some(panel) = &view_def.panel {
                lines.push(format!("panel [{panel} · {view}]"));
                match panel.as_str() {
                    "reviews" => {
                        lines.push(
                            "  queue row → record.logReview (again · hard · good · easy) · record.advanceStage [reviews.panel]"
                                .into(),
                        );
                        lines.push("  study method → type.setPipeline [reviews.panel]".into());
                    }
                    "progress" => lines.push("  metric row → metrics [progress.panel]".into()),
                    // A designed screen this build ships and the app renders from
                    // the view's own data: named, and carrying no controls of its
                    // own. The dump used to call every one of these "unknown
                    // panel" — a misstatement about the app's own vocabulary.
                    panel if crate::model::PANEL_KINDS.contains(&panel) => {
                        lines.push(format!("  {panel} · a designed screen"));
                    }
                    other => lines.push(format!(
                        "  unknown panel: \"{other}\" · known: {}",
                        crate::model::PANEL_KINDS.join(" · ")
                    )),
                }
            }
        }
        None => {
            lines.push("card head [no view — every type with no view is a lint finding]".into())
        }
    }

    plan_sections(&mut lines, config);
    lines
}

/// The plan-independent half of the dump: the row context, the settings
/// sections, and the registry projections (palette, presentation, menu). Split
/// out so a destination section that answers "no record table here" — a
/// composed screen (COMPOSER §2) — still emits them rather than truncating the
/// dump.
fn plan_sections(lines: &mut Vec<String>, config: &ResolvedConfig) {
    lines.push("rowContext [recordTable.rowContext]".into());
    lines.push(
        "  record → copy-json · copy-path · record.reveal · record.move · record.delete".into(),
    );
    lines.push("  cell → record.setField [recordTable.cell]".into());
    lines.push("  card head more → records.renumber · record.move [recordTable.toolbar]".into());

    lines.push("settings [settings]".into());
    for def in crate::doc_edit::settings_ops::FIELDS {
        lines.push(format!(
            "  row {}:{} → settings.set · shows key {}",
            def.key, def.type_, def.key
        ));
    }
    // Phase 7's two settings sections. The appearance controls and the sharing
    // controls are declared here so Gate 2's diff can see them: a theme swatch
    // that stops rendering, or a privacy toggle that stops dispatching, is a
    // stated missing control rather than a screen nobody looked at.
    lines.push("appearance [settings]".into());
    lines.push("  theme swatch → theme.set · themes from theme.list [settings]".into());
    lines.push("  text size row → appearance.setTextScale (100 · 115 · 130%) [settings]".into());
    lines.push(
        "  override key+value → appearance.setOverride · override reset → appearance.clearOverride [settings]"
            .into(),
    );
    lines.push(
        "  run the doctor → design.check · findings carry their rule id and JSON path [settings]"
            .into(),
    );
    lines.push("sharing [settings]".into());
    lines.push(
        "  export profile → profile.export (personal toggle) · import profile → profile.import [settings]"
            .into(),
    );
    lines.push(format!(
        "  private kind → type.setPrivate ({}) [settings]",
        config.types.keys().cloned().collect::<Vec<_>>().join(" · ")
    ));

    let dynamic = command_registry::dynamic_commands(&config.types);
    lines.push(format!(
        "palette [{} static + {} dynamic commands]",
        command_registry::commands().len(),
        dynamic.len()
    ));
    for def in command_registry::commands()
        .iter()
        .filter(|def| def.effect == CommandEffect::Write)
        .chain(
            dynamic
                .iter()
                .filter(|def| def.effect == CommandEffect::Write),
        )
    {
        lines.push(format!(
            "  {} · placement {} · json {}",
            def.id,
            def.ui_placement,
            def.json_path.as_deref().unwrap_or("-")
        ));
    }
    lines.push("presentation [headless: destination or unavailable, exit 5]".into());
    for def in command_registry::commands()
        .iter()
        .filter(|def| def.effect == CommandEffect::Presentation)
    {
        lines.push(format!("  {} · placement {}", def.id, def.ui_placement));
    }

    // The menu bar (§4.10): the same registry, a fourth projection. The shell
    // renders it with the platform's own menu API; this is the list both it and
    // the browser's DOM menu read, so a menu item that dispatches nothing is a
    // finding here rather than a dead click.
    let bindings = keybindings(config);
    lines.push("menu [projected from the registry]".into());
    for menu in command_registry::menu_json(&bindings)["menus"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let title = menu["title"].as_str().unwrap_or("?");
        lines.push(format!("  {title}"));
        for item in menu["items"].as_array().cloned().unwrap_or_default() {
            let key = item["key"].as_str().unwrap_or("-");
            lines.push(format!(
                "    {} · {} · key {}",
                item["id"].as_str().unwrap_or("?"),
                item["category"].as_str().unwrap_or("?"),
                key
            ));
        }
    }
}

/// The plan's own bindings, plus the platform defaults a plan that declares
/// nothing still answers to (§4.8 P2: the binding is data; D14: the accelerator
/// is the platform's own).
pub fn keybindings(config: &ResolvedConfig) -> Vec<(String, String)> {
    let mut bindings: Vec<(String, String)> = config
        .shell
        .as_ref()
        .and_then(|shell| shell.keybindings.as_deref())
        .unwrap_or_default()
        .iter()
        .filter_map(|raw| {
            let key = raw.get("key").and_then(|key| key.as_str())?;
            let command = raw.get("command").and_then(|id| id.as_str())?;
            Some((key.to_string(), command.to_string()))
        })
        .collect();
    for (key, command) in command_registry::DEFAULT_KEYBINDINGS {
        let overridden = bindings.iter().any(|(bound, _)| bound == key)
            || bindings.iter().any(|(_, target)| target == command);
        if !overridden {
            bindings.push((key.to_string(), command.to_string()));
        }
    }
    bindings
}

/// The dump as one JSON value: the lines, plus the counts a diff needs.
pub fn dump_json(config: &ResolvedConfig) -> serde_json::Value {
    let lines = dump(config);
    let dynamic = command_registry::dynamic_commands(&config.types);
    let bindings = keybindings(config);
    serde_json::json!({
        "lines": lines,
        "menu": command_registry::menu_json(&bindings),
        "keybindings": bindings
            .iter()
            .map(|(key, command)| serde_json::json!({ "key": key, "command": command }))
            .collect::<Vec<serde_json::Value>>(),
        "counts": {
            "staticCommands": command_registry::commands().len(),
            "dynamicCommands": dynamic.len(),
            "writeCommands": command_registry::commands()
                .iter()
                .filter(|def| def.effect == CommandEffect::Write)
                .count() + dynamic.len(),
            "presentationCommands": command_registry::commands()
                .iter()
                .filter(|def| def.effect == CommandEffect::Presentation)
                .count(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn seed() -> ResolvedConfig {
        let resources =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources");
        crate::config_store::load_with(&resources.join("presets/seed"), "explicit", &resources)
            .expect("the seed loads")
    }

    /// A plan whose selected destination draws **cells** (its view is a table),
    /// which is where the column menu lives: §4.8 P3's header-as-schema-editor
    /// belongs to the layouts that draw columns, and the dump follows the
    /// destination rather than the registry.
    fn cells() -> ResolvedConfig {
        let resources =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources");
        crate::config_store::load_with(
            &resources.join("fixtures/problemset"),
            "explicit",
            &resources,
        )
        .expect("the problemset fixture loads")
    }

    #[test]
    fn every_dump_line_that_claims_a_dispatch_names_a_real_registry_id() {
        // The screen set's destination draws a `stat` first, so the column menu
        // is declared by the *cells* plan; both halves are one dump each.
        let text = dump(&seed()).join("\n");
        let cells_text = dump(&cells()).join("\n");
        // Every id the dump mentions must resolve, or the gate is asserting a
        // control that has no handler (the failure Gate 2 exists to catch).
        for id in [
            "app.palette",
            "app.openSettings",
            "app.toggleSourcePane",
            "rail.select",
            "list.new",
            "type.new",
            "topic.new",
            "topic.paste",
            "column.rename",
            "column.retype",
            "column.duplicate",
            "column.hide",
            "column.reorder",
            "column.delete",
            "column.show",
            "column.new",
            "record.reveal",
            "record.delete",
            "record.setField",
            "settings.set",
            "apply",
            // Phase 5's screen head, view editor and block tree.
            "view.setLayout",
            "view.setFilter",
            "view.setSort",
            "view.setGroup",
            "view.setLimit",
            "view.setColumns",
            "view.block.add",
            "view.block.set",
            "view.block.remove",
            "view.edit",
        ] {
            let named = text.contains(id) || cells_text.contains(id);
            assert!(named, "the dump never mentions {id}");
            if id.starts_with("copy-") || id == "record.reveal" {
                continue; // clipboard and reveal are surface-local, not registry writes
            }
            assert!(
                command_registry::resolve(id).is_some()
                    || command_registry::dynamic_commands(&seed().types)
                        .iter()
                        .any(|def| def.id == id),
                "{id} is named by the dump but resolves to nothing"
            );
        }
    }

    /// The column menu follows the **destination**, not the registry: a screen
    /// whose first record block is a list or a table declares one per column,
    /// and one whose first block is a cardGrid declares none — because a tile
    /// draws no header, and a gate that asserts a control the app does not have
    /// is the failure this section exists to prevent (§4.8 P3).
    #[test]
    fn the_column_section_follows_what_the_destination_draws() {
        let lines = dump(&seed());
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("columnMenu") && line.contains("stat")),
            "the seed's Today screen draws a stat first: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| line.starts_with("  header ")),
            "and therefore declares no column headers"
        );

        let cells_lines = dump(&cells());
        let headers = cells_lines
            .iter()
            .filter(|line| line.starts_with("  header ") && !line.starts_with("  header +"))
            .count();
        assert_eq!(headers, 6, "the problemset view shows six columns");
        assert!(
            cells_lines
                .iter()
                .any(|line| line.starts_with("  header difficulty:select → ")
                    && line.contains("column.choices"))
        );
    }

    #[test]
    fn the_settings_section_carries_the_keys_it_writes() {
        let config = seed();
        let lines = dump(&config);
        for def in crate::doc_edit::settings_ops::FIELDS {
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains(def.key) && line.contains("settings.set")),
                "a settings row hides its config key: {}",
                def.key
            );
        }
    }

    #[test]
    fn the_dump_names_a_decoration_free_titlebar_and_the_real_plas() {
        let lines = dump(&seed());
        assert!(lines.iter().any(|line| line == "titlebar"));
        assert!(lines.iter().any(|line| line.starts_with("rail")));
        assert!(
            lines.iter().any(|line| line.contains("palette [")),
            "the palette section enumerates write commands"
        );
        // Appendix C.6: only write commands carry placement + json path.
        let palette_start = lines
            .iter()
            .position(|line| line.contains("palette ["))
            .expect("a palette section");
        let entries: Vec<&String> = lines[palette_start + 1..]
            .iter()
            .take_while(|line| !line.starts_with("presentation ["))
            .collect();
        assert!(!entries.is_empty());
        for entry in entries {
            assert!(entry.contains("placement"), "no placement: {entry}");
            assert!(entry.contains("json"), "no json path: {entry}");
        }
    }
}
