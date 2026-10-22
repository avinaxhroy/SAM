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

use crate::config_store::ConfigStoreError;
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
        // Cheap no-op drop first: the revision fingerprint reads and hashes the
        // source bytes, while a full load also parses, decodes and validates
        // every record. A save that changed no byte (an FSEvents replay, an
        // editor's write-twice) stops here.
        let raw = crate::config_store::raw_revision(&plan_root);
        if last_revision.as_deref() == Some(raw.as_str()) {
            continue;
        }
        let event = load_event(&plan_root);
        // A commit renames one file at a time (§4.6 step 4), so a load that
        // lands in that window sees a mixed plan. That is a transaction in
        // flight, not a broken plan: the last good publication stands until the
        // commit finishes and the journal is gone. A crashed journal is bounded
        // by `transaction_in_flight`'s age check, so it cannot hide a real
        // failure for long.
        if event.config.is_none() && transaction_in_flight(&plan_root) {
            continue;
        }
        let key = event_key(&event);
        if last_revision.as_deref() == Some(key.as_str()) {
            continue;
        }
        last_revision = Some(key);
        on_event(event);
    }
}

/// Whether a multi-file commit is between `journal(prepared)` and finalize.
/// A journal file exists for exactly that span, and the age bound keeps a
/// crashed one from suppressing publications forever.
fn transaction_in_flight(plan_root: &Path) -> bool {
    let journal = plan_root.join(".sam").join("tx").join("journal");
    let Ok(entries) = std::fs::read_dir(journal) else {
        return false;
    };
    let now = std::time::SystemTime::now();
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            return false;
        }
        entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| now.duration_since(modified).ok())
            .map(|age| age.as_secs() < 60)
            .unwrap_or(false)
    })
}

fn load_event(plan_root: &Path) -> WatchEvent {
    let resources = match crate::resources::paths() {
        Ok(paths) => paths.resources_dir,
        Err(message) => {
            return WatchEvent {
                config: None,
                diagnostics: vec![Diagnostic::error("watch.load-failed", plan_root.to_string_lossy().to_string(), message)],
            };
        }
    };
    // The watcher only needs the published revision and the diagnostics, never
    // the raw source bytes a writer retains; a read-only load is the same
    // validation without copying every file into memory.
    match crate::config_store::load_readonly(plan_root, "explicit", &resources) {
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

/// What "unchanged" means: a published revision, or the *shape of the
/// failure*. A count was the first pass's key, and two different broken states
/// with the same number of findings were "unchanged" — during an agent's screen
/// work that is exactly the sequence a reader sees, and the second set of
/// diagnostics was never published.
fn event_key(event: &WatchEvent) -> String {
    match &event.config {
        Some(config) => config.revision.clone(),
        None => {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            event.diagnostics.len().hash(&mut hasher);
            for diagnostic in &event.diagnostics {
                diagnostic.code.hash(&mut hasher);
                diagnostic.path.hash(&mut hasher);
                diagnostic.line.hash(&mut hasher);
                diagnostic.message.hash(&mut hasher);
            }
            format!("invalid-{:016x}", hasher.finish())
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;

    fn invalid(messages: &[&str]) -> WatchEvent {
        WatchEvent {
            config: None,
            diagnostics: messages
                .iter()
                .map(|message| Diagnostic::error("test.invalid", "content/views.json", *message))
                .collect(),
        }
    }

    #[test]
    fn two_different_broken_states_are_two_publications() {
        // The count is not the shape of the failure: an agent iterating on a
        // screen can produce two different single-finding states, and the
        // second must still reach the canvas.
        let first = invalid(&["blocks[0]: unknown kind"]);
        let second = invalid(&["blocks[1]: missing title"]);
        assert_ne!(event_key(&first), event_key(&second));
        // The same findings are the same state, whatever order they arrive in
        // is not — the list is ordered by the loader, so equal lists are equal.
        assert_eq!(event_key(&first), event_key(&invalid(&["blocks[0]: unknown kind"])));
    }

    #[test]
    fn a_journal_in_flight_is_recognised_and_ages_out() {
        let root = std::env::temp_dir().join("sam-watcher-journal-test");
        let journal = root.join(".sam").join("tx").join("journal");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&journal).unwrap();
        assert!(!transaction_in_flight(&root), "an empty journal directory is not a commit");
        let file = journal.join("tx-1.json");
        std::fs::write(&file, b"{}").unwrap();
        assert!(
            transaction_in_flight(&root),
            "a fresh journal is a commit between prepared and finalize"
        );
        // An old journal is a crash that recovery has not cleaned up: it must
        // not suppress publications forever.
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        let handle = std::fs::File::options().write(true).open(&file).unwrap();
        handle.set_modified(old).unwrap();
        assert!(!transaction_in_flight(&root));
        let _ = std::fs::remove_dir_all(&root);
    }
}
