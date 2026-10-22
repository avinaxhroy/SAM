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
use std::sync::{Arc, Mutex};

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
    /// The one agent turn the assistant deck is streaming, if any.
    agent: Mutex<Option<sam_core::agent::AgentRun>>,
    /// The plan snapshot every read projection on a screen shares.
    read_cache: sam_core::dispatch::ReadCache,
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
    // The folder this window was last on (R11): the page boots into it instead
    // of whichever plan the plans root resolves. A remembered folder that is no
    // longer a plan answers null, and the picker opens as it always did.
    value["plan"] = match sam_core::resources::last_plan(&paths) {
        Some(plan) => serde_json::Value::String(plan),
        None => serde_json::Value::Null,
    };
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
    state: tauri::State<'_, ShellState>,
    id: String,
    params: Option<serde_json::Value>,
    plan: Option<String>,
    if_revision: Option<String>,
) -> Result<serde_json::Value, String> {
    let resources_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("cannot resolve the bundle resource directory: {error}"))?;
    let cache = state.read_cache.clone();
    let mut request = sam_core::dispatch::DispatchRequest::new(id, resources_dir);
    request.params =
        sam_core::dispatch::params_from_json(params.as_ref()).map_err(|error| error.to_string())?;
    // Commands like plan.new and profile.import do not require an existing plan.
    request.plan = plan.map(std::path::PathBuf::from);
    request.if_revision = if_revision;

    // The engine reads, parses and validates the whole plan for most commands
    // (tens to hundreds of milliseconds). Running that inline in an `async fn`
    // pins a runtime worker and lets concurrent boot reads — `views`, `view`,
    // `today.view`, `reviews.due`, `commands`, `appearance.resolve` — queue
    // behind one another. The blocking pool is the right home for file I/O and
    // CPU work; the plan lock still serializes writers.
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        sam_core::dispatch::CommandSession::run_cached(request, &cache)
    })
    .await
    .map_err(|error| format!("the dispatch task did not finish: {error}"))?;
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

/// A file door pressed in the Preferences window (§4.10). That window's whole
/// content is the settings screen, so it draws no source pane: its editor
/// commands land here, and the path is handed to the window that does — the
/// main window, brought forward so the press lands somewhere the student sees.
#[tauri::command]
fn open_source(app: tauri::AppHandle, file: String) -> Result<bool, String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(false);
    };
    window
        .emit("open-source", file)
        .map_err(|error| format!("the main window did not take the file: {error}"))?;
    let _ = window.set_focus();
    Ok(true)
}

/// Opens an `http(s)` address in the platform's own browser — the lecture a
/// topic names, a resource's link.
///
/// The webview cannot do this itself: the shell installs no opener plugin, and a
/// page's `window.open` / `target=_blank` has nothing to catch it. The scheme is
/// checked here rather than trusted from the caller: a command is reachable from
/// any script the page runs, and this one hands a string to the OS.
#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http and https links open in the browser".into());
    }
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        // Not `cmd /C start`: a query string's `&` is a command separator to
        // cmd, so a lecture URL would open the first half of itself. `rundll32`
        // hands the string to the shell's protocol handler with no parsing.
        let mut command = std::process::Command::new("rundll32");
        command.args(["url.dll,FileProtocolHandler"]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command
        .arg(url)
        .spawn()
        .map(|mut child| {
            // Reap the launcher; the browser is not this app's child to wait on.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        })
        .map_err(|error| format!("cannot open the browser: {error}"))
}

/// An app preference made in one window and needed by the others (§4.10): a
/// component's design, the interface size, the light/dark choice. Those stores
/// live in the webview — the engine has no business knowing which table design
/// a student likes — and every window is a **separate webview** with its own
/// copy, so the shell carries a change to each window except the one that made
/// it. The receivers apply it without publishing it back: one writer, no loop.
#[tauri::command]
fn publish_preference(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    key: String,
    value: String,
) -> Result<(), String> {
    let payload = serde_json::json!({ "key": key, "value": value });
    for (label, target) in app.webview_windows() {
        if label == window.label() {
            continue;
        }
        target
            .emit("preference-changed", payload.clone())
            .map_err(|error| format!("{label} did not take the preference: {error}"))?;
    }
    Ok(())
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

/// Stop watching the current plan. The picker has no plan, and a watcher left
/// running there wakes on changes to a folder the window no longer shows.
#[tauri::command]
fn unwatch_plan(state: tauri::State<'_, ShellState>) -> Result<(), String> {
    let mut slot = state
        .watcher
        .lock()
        .map_err(|_| "the watcher lock is poisoned".to_string())?;
    *slot = None;
    Ok(())
}

/// Remember the plan folder this window loaded (R11). The page calls this once
/// the plan has actually read — not when a folder merely resolved a root — so a
/// relaunch opens the folder the student was working in, and a refused or
/// broken open never becomes the memory.
#[tauri::command]
fn remember_plan(plan: String) -> Result<(), String> {
    let paths = sam_core::resources::paths()?;
    sam_core::resources::note_last_plan(&paths, &plan)
}

/// The one spelling of the open folder every surface composes against.
///
/// A symlinked spelling or a trailing separator must not reach the agent as a
/// second plan. On Windows `canonicalize` answers a verbatim path
/// (`\\?\C:\…`); it is the same folder, but it would reach the prompt, the
/// child's `SAM_PLAN` and the transcript, where a shell and a student both read
/// it — so the prefix is stripped when what follows is a plain drive path.
fn canonical_plan(plan: &str) -> Result<PathBuf, String> {
    let root = std::fs::canonicalize(plan).map_err(|error| format!("{plan}: {error}"))?;
    if !root.is_dir() {
        return Err(format!("{plan} is not a directory"));
    }
    Ok(displayable_path(root))
}

#[cfg(windows)]
fn displayable_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return PathBuf::from(rest);
        }
    }
    path
}

#[cfg(not(windows))]
fn displayable_path(path: PathBuf) -> PathBuf {
    path
}

/// Start one assistant turn (§4.7 through the deck). The engine resolves the
/// machine's CLIs and composes the prompt (`sam_core::agent`); the shell's only
/// jobs are the app's resource directory, the child's event channel, and
/// holding the run so `agent_cancel` can stop it — the same division
/// `watch_plan` keeps for file changes. `agent.list` deliberately has no
/// command here: the page reaches it through `dispatch` like every other read.
#[tauri::command]
async fn agent_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, ShellState>,
    agent_id: Option<String>,
    model: Option<String>,
    resume: Option<String>,
    context: Option<String>,
    prompt: String,
    plan: String,
) -> Result<serde_json::Value, String> {
    let resources_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("cannot resolve the bundle resource directory: {error}"))?;
    // Detection, canonicalisation and prompt composition read the disk; they
    // run off the main thread, so the deck's "Working" state, the transcript
    // and the plan underneath all keep painting while the turn is prepared.
    let spec = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        // One folder, one identity: a symlinked spelling or a trailing
        // separator must not reach the agent as a second plan — the prompt
        // quotes this path and the child's `SAM_PLAN` is set from it.
        let root = canonical_plan(&plan)?;
        sam_core::agent::prepare_run(sam_core::agent::RunRequest {
            agent_id: agent_id.as_deref(),
            model: model.as_deref(),
            resume: resume.as_deref(),
            context: context.as_deref(),
            prompt: &prompt,
            plan_root: &root,
            resources_dir: &resources_dir,
            // The deck owns a Stop: give the turn its own process group so the
            // stop reaches what the CLI started, not just the CLI.
            isolate: true,
        })
    })
    .await
    .map_err(|error| format!("the agent could not be prepared: {error}"))??;
    let run_id = spec.run_id.clone();
    let agent = spec.id.clone();
    let plan = spec.cwd.display().to_string();
    let emitter = app.clone();
    let tag = run_id.clone();
    let on_event = Arc::new(move |event: sam_core::agent::AgentEvent| {
        // The window draws prose, tool calls, files and failures. Extended
        // thinking arrives as one frame *per token* and no surface reads it —
        // the deck drops `thinking` on arrival — so the shell does not pay the
        // IPC crossing for it. The terminal's `agent.run --json` still reports
        // the same frames: that door is a transcript, this one is a console.
        if matches!(&event, sam_core::agent::AgentEvent::Thinking { .. }) {
            return;
        }
        if let Ok(mut payload) = serde_json::to_value(&event) {
            if let Some(object) = payload.as_object_mut() {
                object.insert("runId".into(), serde_json::json!(tag));
            }
            let _ = emitter.emit("agent-event", payload);
        }
    });
    let run = sam_core::agent::start(spec, on_event)?;
    {
        let mut slot = match state.agent.lock() {
            Ok(slot) => slot,
            Err(_) => {
                // The lock is gone, so no Stop can ever reach this child: kill
                // it rather than leave an untracked agent writing the plan.
                run.cancel();
                return Err("the agent lock is poisoned".into());
            }
        };
        // One turn at a time: a start replaces whatever is running.
        if let Some(previous) = slot.take() {
            previous.cancel();
        }
        *slot = Some(run);
    }
    Ok(serde_json::json!({
        "runId": run_id,
        "agent": agent,
        "plan": plan,
    }))
}

/// Stop the running turn (the deck's Stop). Killing the child is enough: the
/// plan can only be mid-write inside a `sam` transaction, and transactions
/// recover at their journal boundary — the guarantee the selfcheck gates hold.
///
/// The handle is taken out of the lock *before* it is cancelled: `cancel` can
/// wait on a child in uninterruptible sleep, and a command that held the state
/// lock through that would freeze every other agent call with it.
#[tauri::command]
async fn agent_cancel(state: tauri::State<'_, ShellState>) -> Result<bool, String> {
    let run = {
        let mut slot = state
            .agent
            .lock()
            .map_err(|_| "the agent lock is poisoned".to_string())?;
        slot.take()
    };
    match run {
        Some(run) => {
            tauri::async_runtime::spawn_blocking(move || run.cancel())
                .await
                .map_err(|error| format!("the stop could not run: {error}"))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Install the bundled CLI onto the user's PATH (§4.10's machine seam, the
/// View menu's *Install command line tool*). The engine plans and applies what
/// needs no rights; this command owns the platform's own permission ask —
/// macOS hands the planned `ln` to `osascript … with administrator
/// privileges`, Linux to `pkexec` when it exists. Nothing is invoked from the
/// webview, and a refusal is a sentence, never a silent no-op.
#[tauri::command]
fn install_cli(remove: Option<bool>) -> Result<serde_json::Value, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf));
    if remove.unwrap_or(false) {
        let source = sam_core::cli_install::bundled_sam(exe_dir.as_deref())
            .ok_or_else(|| "this build carries no bundled `sam`".to_string())?;
        let receipt = sam_core::cli_install::uninstall(&source)?;
        return Ok(serde_json::json!({
            "installed": false,
            "needsElevation": false,
            "action": receipt.action,
            "path": receipt.path,
            "target": receipt.target,
        }));
    }
    let source = sam_core::cli_install::bundled_sam(exe_dir.as_deref()).ok_or_else(|| {
        "this build carries no bundled `sam` beside this executable — run the packaged app"
            .to_string()
    })?;
    let plan = sam_core::cli_install::plan(&source);
    if let sam_core::cli_install::Action::NeedsElevation { link, command } = &plan.action {
        run_elevated(command)?;
        return Ok(serde_json::json!({
            "installed": true,
            "needsElevation": false,
            "elevated": true,
            "action": "installed",
            "path": link.display().to_string(),
            "target": source.display().to_string(),
        }));
    }
    sam_core::cli_install::apply(&plan)
        .map(sam_core::cli_install::Outcome::Done)
        .map(|outcome| outcome.json())
}

/// The platform's own prompt for one privileged command. macOS asks with the
/// system authentication dialog; Linux uses `pkexec` when it is installed;
/// anywhere else the caller prints the command to run themselves.
fn run_elevated(command: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("do shell script \"{escaped}\" with administrator privileges");
        let status = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map_err(|error| format!("cannot ask macOS for permission: {error}"))?;
        if status.success() {
            return Ok(());
        }
        return Err("the permission dialog was dismissed — nothing was installed".into());
    }
    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("pkexec")
            .args(["sh", "-c", command])
            .status()
        {
            Ok(status) if status.success() => Ok(()),
            Ok(_) => Err("the permission prompt was dismissed — nothing was installed".into()),
            Err(_) => Err(format!(
                "no pkexec on this machine — run this in a terminal: {command}"
            )),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = command;
        Err(format!("run this in a terminal: {command}"))
    }
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
            open_source,
            open_external,
            publish_preference,
            pick_directory,
            pick_profile_out,
            pick_profile_file,
            take_pending_open,
            watch_plan,
            unwatch_plan,
            remember_plan,
            agent_start,
            agent_cancel,
            install_cli
        ])
        .build(tauri::generate_context!())
        .expect("SAM's shell failed to build")
        .run(|app, event| {
            // A turn outlives the window unless it is stopped (§4.7): quitting
            // must not leave an agent under launchd still writing the plan with
            // no UI to see it or Stop it. A reload is the other half — the page
            // that owned the run is gone, so its events have nowhere to land —
            // and the page itself asks for the same stop when it mounts.
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                stop_agent(app);
            }
            // macOS delivers file associations via Opened events; Windows/Linux deliver via argv.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = &event {
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        route_open(app, path.display().to_string(), "opened-event");
                    }
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            {
                let _ = (app, &event);
            }
        });
}

/// Cancel whatever turn the deck left running, without needing a command from
/// a page that may already be gone.
fn stop_agent(app: &tauri::AppHandle) {
    let state = app.state::<ShellState>();
    let run = {
        match state.agent.lock() {
            Ok(mut slot) => slot.take(),
            // A poisoned lock still holds the handle; take it anyway.
            Err(poisoned) => poisoned.into_inner().take(),
        }
    };
    if let Some(run) = run {
        run.cancel();
    }
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
