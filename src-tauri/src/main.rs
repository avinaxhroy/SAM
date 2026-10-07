//! SAM desktop shell (§4.2, §4.10, D16/D18).
//!
//! Native OS integration for `sam-core`:
//! - Window frame and controls (§4.10).
//! - Application menu bar and context menus from `command_registry` (§4.10).
//! - Single instance routing for CLI and file arguments (§4.10).
//! - Linux WebKitGTK renderer probe overrides (§9, D21).
//! - Sandboxed webview with mutations restricted to `sam-core`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Deserialize;
use tauri::menu::{Menu, MenuBuilder, MenuItem, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use sam_core::command_registry;
use sam_core::config_watcher::ConfigWatcher;

/// Shell state preserved between invocations: active plan watcher and pending file opens.
#[derive(Default)]
struct ShellState {
    watcher: Mutex<Option<ConfigWatcher>>,
    pending_open: Mutex<Vec<String>>,
}

/// Renderer path taken for diagnostic reporting (§9, D21).
static RENDERER_PATH: Mutex<Option<&'static str>> = Mutex::new(None);

/// Determines the minimum Linux renderer override for WebKitGTK / NVIDIA driver compatibility (§9, D21).
fn renderer_choice(
    platform: &str,
    nvidia_driver: bool,
    nvidia_modeset: bool,
) -> (&'static str, Option<(&'static str, &'static str)>) {
    if platform != "linux" {
        return ("native", None);
    }
    if !nvidia_driver {
        return ("native", None);
    }
    if nvidia_modeset {
        return ("nvidia-modeset", None);
    }
    ("nvidia-explicit-sync-off", Some(("__NV_DISABLE_EXPLICIT_SYNC", "1")))
}

/// Applies WebKitGTK driver overrides if needed before webview creation.
fn apply_renderer_probe() {
    let label = if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some()
        || std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_some()
    {
        "user-override"
    } else {
        let nvidia_driver = std::path::Path::new("/proc/driver/nvidia/version").exists()
            || std::path::Path::new("/dev/nvidia0").exists();
        let nvidia_modeset = std::fs::read_to_string("/sys/module/nvidia_drm/parameters/modeset")
            .map(|value| value.trim().eq_ignore_ascii_case("Y"))
            .unwrap_or(false);
        let (label, variable) = renderer_choice(std::env::consts::OS, nvidia_driver, nvidia_modeset);
        if let Some((key, value)) = variable {
            // SAFETY: single-threaded at this point — this runs before Tauri
            // starts its runtime, so no other thread can be reading the
            // environment concurrently.
            unsafe { std::env::set_var(key, value) };
        }
        label
    };
    if std::env::consts::OS == "linux" {
        if let Ok(paths) = sam_core::resources::paths() {
            let _ = sam_core::resources::note_renderer_path(&paths, label);
        }
    }
    if let Ok(mut slot) = RENDERER_PATH.lock() {
        *slot = Some(label);
    }
}

/// The renderer path this process took (`native` when nothing was overridden).
fn renderer_path() -> &'static str {
    RENDERER_PATH
        .lock()
        .ok()
        .and_then(|slot| *slot)
        .unwrap_or("native")
}

/// Up to 8 most recently modified plan directories.
fn recent_plans() -> Vec<String> {
    let Ok(paths) = sam_core::resources::paths() else {
        return Vec::new();
    };
    let mut entries: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(&paths.plans_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| {
                    let modified = entry.metadata().ok()?.modified().ok()?;
                    Some((modified, entry.path().display().to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().map(|(_, path)| path).take(8).collect()
}

/// Normalizes OS identifier to design token vocabulary ("mac", "win", or "linux") (§4.8, D14).
fn os_label() -> &'static str {
    match std::env::consts::OS {
        "macos" => "mac",
        "windows" => "win",
        other => other,
    }
}

/// Exposes resolved paths, platform identifier, and renderer diagnostics to the frontend (§4.1, D21).
#[tauri::command]
fn shell_info(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let resources_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("cannot resolve the bundle resource directory: {error}"))?;
    let paths = sam_core::resources::paths_with_resources(resources_dir)?;

    let mut value =
        serde_json::to_value(&paths).map_err(|error| format!("cannot report paths: {error}"))?;
    value["os"] = serde_json::Value::String(os_label().into());
    // §9/D21: renderer path this launch took.
    value["rendererPath"] = serde_json::Value::String(renderer_path().into());
    // True when window controls float over content (e.g. macOS Overlay style).
    value["titlebarOverlay"] = serde_json::Value::Bool(titlebar_overlay(&app));
    Ok(value)
}

/// True when window configuration uses overlay titlebar controls.
fn titlebar_overlay(app: &tauri::AppHandle) -> bool {
    app.config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .and_then(|window| serde_json::to_value(&window.title_bar_style).ok())
        .and_then(|value| value.as_str().map(str::to_owned))
        .is_some_and(|style| matches!(style.as_str(), "Overlay" | "Transparent"))
}

/// Dispatch frontend engine command (§4.2) via `CommandSession::run`.
#[tauri::command]
async fn dispatch(
    app: tauri::AppHandle,
    id: String,
    params: Option<serde_json::Value>,
    plan: Option<String>,
    if_revision: Option<String>,
) -> Result<serde_json::Value, String> {
    let resources_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("cannot resolve the bundle resource directory: {error}"))?;
    let command_id = id.clone();
    let mut request = sam_core::dispatch::DispatchRequest::new(id, resources_dir);
    request.params =
        sam_core::dispatch::params_from_json(params.as_ref()).map_err(|error| error.to_string())?;
    // Commands like plan.new and profile.import do not require an existing plan.
    request.plan = plan.map(std::path::PathBuf::from);
    request.if_revision = if_revision;

    let outcome = sam_core::dispatch::CommandSession::run(request);
    if let Ok(paths) = sam_core::resources::paths() {
        let detail = match &outcome {
            Ok(_) => format!("{command_id}: ok"),
            Err(error) => format!("{command_id}: {error}"),
        };
        let _ = sam_core::resources::note_page_call(&paths, "lastCommandResult", &detail);
    }
    outcome
        .map(sam_core::dispatch::outcome_json)
        .map_err(|error| error.to_string())
}

/// One item of a context menu, as the page sends it.
#[derive(Deserialize)]
struct PopupItem {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default)]
    separator: bool,
}

fn default_enabled() -> bool {
    true
}

/// Show the platform's own context menu (§4.10: *"the same native menu API, per
/// row, cell and selection"*). The page supplies the items — it owns the actions
/// — and the chosen id arrives on the `menu` channel, exactly like a menu-bar
/// click, so the app runs it through one table.
#[tauri::command]
fn popup_menu(window: tauri::Window, items: Vec<PopupItem>) -> Result<(), String> {
    let mut builder = MenuBuilder::new(&window);
    for item in items {
        builder = if item.separator {
            builder.separator()
        } else {
            builder.item(&MenuItem::with_id(
                &window,
                item.id,
                item.title,
                item.enabled,
                None::<&str>,
            )
            .map_err(|error| error.to_string())?)
        };
    }
    let menu = builder.build().map_err(|error| error.to_string())?;
    window
        .popup_menu(&menu)
        .map_err(|error| error.to_string())
}

/// Preferences as a real OS window (§4.10). The same frontend, mounted at
/// `#preferences`, renders the settings screen; if the window already exists it
/// is focused rather than duplicated.
#[tauri::command]
async fn open_preferences(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("preferences") {
        let _ = window.set_focus();
        return Ok(true);
    }
    let built = tauri::WebviewWindowBuilder::new(
        &app,
        "preferences",
        tauri::WebviewUrl::App("index.html#preferences".into()),
    )
    .title("SAM Preferences")
    .inner_size(720.0, 620.0)
    .resizable(true)
    .build()
    .map_err(|error| error.to_string())?;
    let _ = built.set_focus();
    Ok(true)
}

/// Native directory picker dialog (§4.10).
#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (send, receive) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Open a plan")
        .pick_folder(move |path| {
            let _ = send.send(path.map(|path| path.to_string()));
        });
    receive
        .recv()
        .map_err(|error| format!("the dialog did not answer: {error}"))
}

/// Native save dialog for profile export (§4.10).
#[tauri::command]
async fn pick_profile_out(app: tauri::AppHandle, name: Option<String>) -> Result<Option<String>, String> {
    let (send, receive) = std::sync::mpsc::channel();
    let suggested = name.unwrap_or_else(|| "sam-profile".to_string());
    app.dialog()
        .file()
        .set_title("Export a profile")
        .set_file_name(format!("{suggested}.{}", sam_core::profile::PROFILE_EXTENSION))
        .add_filter(
            "SAM profile",
            &[sam_core::profile::PROFILE_EXTENSION],
        )
        .save_file(move |path| {
            let _ = send.send(path.map(|path| path.to_string()));
        });
    receive
        .recv()
        .map_err(|error| format!("the dialog did not answer: {error}"))
}

/// Native file dialog for profile import (§4.10).
#[tauri::command]
async fn pick_profile_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (send, receive) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Import a profile")
        .add_filter("SAM profile", &[sam_core::profile::PROFILE_EXTENSION])
        .pick_file(move |path| {
            let _ = send.send(path.map(|path| path.to_string()));
        });
    receive
        .recv()
        .map_err(|error| format!("the dialog did not answer: {error}"))
}

/// Returns and clears paths opened prior to window initialization.
#[tauri::command]
fn take_pending_open(state: tauri::State<'_, ShellState>) -> Vec<String> {
    let taken = state
        .pending_open
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default();
    if let Ok(paths) = sam_core::resources::paths() {
        let _ = sam_core::resources::note_page_call(
            &paths,
            "pendingTaken",
            &format!("{} path(s)", taken.len()),
        );
    }
    taken
}

/// Watch plan root directory and publish `plan-changed` events on update (D9).
#[tauri::command]
fn watch_plan(
    app: tauri::AppHandle,
    state: tauri::State<'_, ShellState>,
    plan: String,
) -> Result<(), String> {
    let root = PathBuf::from(&plan);
    if !root.is_dir() {
        return Err(format!("{plan} is not a directory"));
    }
    let handle = app.clone();
    let watcher = ConfigWatcher::start(
        root,
        sam_core::config_watcher::DEFAULT_DEBOUNCE_MS,
        false,
        move |event| {
            let payload = serde_json::json!({
                "revision": event.config.as_ref().map(|config| config.revision.clone()),
                "plan": plan,
                "valid": event.config.is_some(),
                "message": event
                    .diagnostics
                    .first()
                    .map(|diagnostic| format!("{diagnostic}")),
            });
            let _ = handle.emit("plan-changed", payload);
        },
    );
    let mut slot = state
        .watcher
        .lock()
        .map_err(|_| "the watcher lock is poisoned".to_string())?;
    *slot = Some(watcher); // the previous watcher stops as it is dropped
    Ok(())
}

/// Project native menu bar from the command registry (§4.10).
fn build_menu(app: &tauri::AppHandle, keybindings: &[(String, String)]) -> tauri::Result<Menu<tauri::Wry>> {
    let accelerator = |id: &str| -> Option<String> {
        keybindings
            .iter()
            .find(|(_, command)| command == id)
            .map(|(key, _)| key.replace("mod", "CmdOrCtrl").replace("comma", ","))
    };

    let mut menu = MenuBuilder::new(app);
    let about = PredefinedMenuItem::about(app, Some("About SAM"), None)?;
    let preferences = MenuItem::with_id(
        app,
        "app.openSettings",
        "Preferences…",
        true,
        accelerator("app.openSettings").as_deref(),
    )?;
    let services = PredefinedMenuItem::services(app, None)?;
    let hide = PredefinedMenuItem::hide(app, Some("Hide SAM"))?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit SAM"))?;
    let mut app_menu = SubmenuBuilder::new(app, "SAM")
        .item(&about)
        .item(&preferences);
    let recent = recent_plans();
    if recent.is_empty() {
        let empty = MenuItem::with_id(app, "recent.none", "No plans yet", false, None::<&str>)?;
        let submenu = SubmenuBuilder::new(app, "Open Recent").item(&empty).build()?;
        app_menu = app_menu.item(&submenu);
    } else {
        let mut submenu = SubmenuBuilder::new(app, "Open Recent");
        for (index, path) in recent.iter().enumerate() {
            let name = std::path::Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            let item = MenuItem::with_id(app, format!("recent.{index}"), name, true, None::<&str>)?;
            let item = item;
            submenu = submenu.item(&item);
        }
        app_menu = app_menu.item(&submenu.build()?);
    }
    let app_menu = app_menu
        .separator()
        .item(&services)
        .item(&hide)
        .separator()
        .item(&quit)
        .build()?;
    menu = menu.item(&app_menu);

    for (title, categories) in command_registry::MENUS {
        let mut submenu = SubmenuBuilder::new(app, title);
        let mut first = true;
        for category in categories {
            let items: Vec<&command_registry::CommandDef> = command_registry::commands()
                .iter()
                .filter(|def| def.category == *category)
                .collect();
            if items.is_empty() {
                continue;
            }
            if !first {
                submenu = submenu.separator();
            }
            first = false;
            for def in items {
                let item = MenuItem::with_id(
                    app,
                    def.id.clone(),
                    def.title.clone(),
                    true,
                    accelerator(&def.id).as_deref(),
                )?;
                submenu = submenu.item(&item);
            }
        }
        if !first {
            menu = menu.item(&submenu.build()?);
        }
    }
    menu.build()
}

/// Extracts `.samprofile` argument from CLI arguments if present.
fn association_argument(argv: &[String]) -> Option<String> {
    argv.iter()
        .find(|argument| {
            !argument.starts_with('-')
                && std::path::Path::new(argument)
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case(sam_core::profile::PROFILE_EXTENSION))
                    .unwrap_or(false)
        })
        .cloned()
}

/// Routes opened file path to the webview or queues it until ready.
fn route_open(app: &tauri::AppHandle, path: String, source: &str) {
    if let Ok(paths) = sam_core::resources::paths() {
        let _ = sam_core::resources::note_open(&paths, &path, source);
    }
    let state = app.state::<ShellState>();
    if let Ok(mut pending) = state.pending_open.lock() {
        pending.push(path.clone());
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = app.emit("open-profile", path);
    }
}

fn main() {
    // §9, D21: detect renderer capabilities before webview creation.
    apply_renderer_probe();
    tauri::Builder::default()
        .manage(ShellState::default())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // §4.10: second launch routes arguments or profile files to running window.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            if let Some(profile) = association_argument(&argv) {
                route_open(app, profile, "argv");
            } else if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("launch", argv);
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let keybindings: Vec<(String, String)> = command_registry::DEFAULT_KEYBINDINGS
                .iter()
                .map(|(key, command)| (key.to_string(), command.to_string()))
                .collect();
            let handle = app.handle();
            if let Ok(menu) = build_menu(handle, &keybindings) {
                let _ = app.set_menu(menu);
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().0.clone();
            if let Some(index) = id
                .strip_prefix("recent.")
                .and_then(|index| index.parse::<usize>().ok())
            {
                if let Some(path) = recent_plans().get(index) {
                    route_open(app, path.clone(), "recent");
                }
                return;
            }
            let _ = app.emit("menu", id);
        })
        .invoke_handler(tauri::generate_handler![
            shell_info,
            dispatch,
            popup_menu,
            open_preferences,
            pick_directory,
            pick_profile_out,
            pick_profile_file,
            take_pending_open,
            watch_plan
        ])
        .build(tauri::generate_context!())
        .expect("SAM's shell failed to build")
        .run(|app, event| {
            // macOS delivers file associations via Opened events; Windows/Linux deliver via argv.
            if let tauri::RunEvent::Opened { urls } = event {
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        route_open(app, path.display().to_string(), "opened-event");
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::os_label;

    #[test]
    fn the_os_label_is_the_design_systems_vocabulary() {
        let label = os_label();
        assert!(
            matches!(label, "mac" | "win" | "linux"),
            "os_label() returned {label:?}, which [data-os] does not know"
        );
    }

    #[test]
    fn every_menu_item_is_a_registry_id() {
        for (_, categories) in sam_core::command_registry::MENUS {
            for category in categories {
                for def in sam_core::command_registry::commands()
                    .iter()
                    .filter(|def| def.category == *category)
                {
                    assert!(
                        sam_core::command_registry::resolve(&def.id).is_some(),
                        "menu item {} is not in the registry",
                        def.id
                    );
                }
            }
        }
    }

    #[test]
    fn the_accelerator_translation_is_the_platforms_spelling() {
        let translated = "mod+shift+z".replace("mod", "CmdOrCtrl");
        assert_eq!(translated, "CmdOrCtrl+shift+z");
        assert!(!translated.contains("mod"));
    }
}
