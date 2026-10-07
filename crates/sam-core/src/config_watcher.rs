//! `ConfigWatcher` (D9): file system watcher for plan directories.
//!
//! Watches plan directory changes, hashes content to discard no-op events,
//! debounces rapid edits (e.g. bulk writes), and emits updated configuration state.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config_store::{ConfigStoreError, load};
use crate::diagnostic::Diagnostic;
use crate::resolved_config::ResolvedConfig;

/// One publication. An invalid external save does not replace the last known
/// good config — it arrives as diagnostics with `config: None`.
pub struct WatchEvent {
    pub config: Option<ResolvedConfig>,
    pub diagnostics: Vec<Diagnostic>,
}

/// The debounce window D9 fixes. The self-check shortens it; nothing else
/// should.
pub const DEFAULT_DEBOUNCE_MS: u64 = 150;

pub struct ConfigWatcher {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl ConfigWatcher {
    /// Start watching `plan_root`. The callback runs on the watcher thread;
    /// `start` returns immediately.
    pub fn start<F>(
        plan_root: PathBuf,
        debounce_ms: u64,
        publish_initial: bool,
        on_event: F,
    ) -> Self
    where
        F: Fn(WatchEvent) + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join = thread::Builder::new()
            .name("sam-configwatcher".into())
            .spawn(move || {
                run(
                    plan_root,
                    debounce_ms,
                    publish_initial,
                    &thread_stop,
                    on_event,
                )
            })
            .ok();
        Self { stop, join }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run<F>(
    plan_root: PathBuf,
    debounce_ms: u64,
    publish_initial: bool,
    stop: &AtomicBool,
    on_event: F,
) where
    F: Fn(WatchEvent),
{
    let (sender, receiver) = channel::<()>();
    // `smith:` one watcher, one channel, one debounce — no per-path source
    // census, because the reload that follows hashes the content anyway and
    // the revision check is what decides whether anything was a change.
    let mut watcher = match RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| {
            if result.is_ok() {
                let _ = sender.send(());
            }
        },
        Config::default(),
    ) {
        Ok(watcher) => watcher,
        Err(_) => return,
    };

    attach(&mut watcher, &plan_root);

    let mut last_revision: Option<String> = None;
    if publish_initial {
        let event = load_event(&plan_root);
        last_revision = Some(event_key(&event));
        on_event(event);
    }

    let debounce = Duration::from_millis(debounce_ms.max(1));
    while !stop.load(Ordering::SeqCst) {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // Debounce (bocan-music): one reload per burst, not N.
        thread::sleep(debounce);
        while receiver.try_recv().is_ok() {}
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // The directory may have been deleted and recreated, and the file set
        // may have changed — re-attach before loading.
        attach(&mut watcher, &plan_root);
        let event = load_event(&plan_root);
        let key = event_key(&event);
        // Content-hash no-op drop (ble-scale-sync): a byte-identical save and
        // an FSEvents replay both publish nothing.
        if last_revision.as_deref() == Some(key.as_str()) {
            continue;
        }
        last_revision = Some(key);
        on_event(event);
    }
}

fn load_event(plan_root: &Path) -> WatchEvent {
    match load(plan_root, "explicit") {
        Ok(config) => WatchEvent {
            config: Some(config),
            diagnostics: Vec::new(),
        },
        Err(ConfigStoreError::Validation(diagnostics)) => WatchEvent {
            config: None,
            diagnostics,
        },
        Err(ConfigStoreError::Io(message)) => WatchEvent {
            config: None,
            diagnostics: vec![Diagnostic::error(
                "watch.load-failed",
                plan_root.to_string_lossy().to_string(),
                message,
            )],
        },
    }
}

/// What "unchanged" means: a published revision, or the shape of the failure.
fn event_key(event: &WatchEvent) -> String {
    match &event.config {
        Some(config) => config.revision.clone(),
        None => format!("invalid-{}", event.diagnostics.len()),
    }
}

/// Watch the stable parent root as well as the plan tree, and every document
/// file: an append inside `records/` does not touch the directory vnode, and a
/// replaced plan directory does not touch its own.
///
/// `smith:` on macOS `FSEvents` delivers subtree events regardless, so this can
/// watch more than it needs; the revision check is what decides. Narrow it with
/// per-path stream flags only if the extra events ever cost real CPU.
fn attach(watcher: &mut RecommendedWatcher, plan_root: &Path) {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(parent) = plan_root.parent() {
        paths.push(parent.to_path_buf());
    }
    paths.push(plan_root.to_path_buf());
    for rel in [
        "state",
        "content",
        "content/records",
        "content/types.json",
        "content/views.json",
        "content/rules.json",
        "content/shell.json",
        "content/appearance.json",
        "state/state.json",
    ] {
        let path = plan_root.join(rel);
        if path.exists() {
            paths.push(path);
        }
    }
    if let Ok(entries) = std::fs::read_dir(plan_root.join("content/records")) {
        paths.extend(
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl")),
        );
    }
    for path in paths {
        // A path already watched — or one that vanished between the census and
        // here — is not an error worth surfacing: the reload that follows is
        // the source of truth.
        let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
    }
}

/// A blocking reader for tests and callers that want one thread rather than a
/// callback: the same publisher, delivered over a channel.
pub fn channel_events(
    plan_root: PathBuf,
    debounce_ms: u64,
) -> (ConfigWatcher, Receiver<WatchEvent>) {
    let (sender, receiver) = channel();
    let watcher = ConfigWatcher::start(plan_root, debounce_ms, true, move |event| {
        let _ = sender.send(event);
    });
    (watcher, receiver)
}
