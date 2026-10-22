//! The assistant's engine (§4.7 extended): detect the coding-agent CLIs a
//! machine already has, run one against the open plan folder, and stream its
//! output back as a small event vocabulary.
//!
//! SAM ships **no agent and no API**: the `claude` / `opencode` / `agy` /
//! `copilot` / `omp` / `pi` binaries already on the user's `PATH` are the
//! engine (the architectural decision OpenDesign documents as
//! *"the code agent space has already converged; talking to all of them beats
//! reimplementing one"*). The agent authors the plan through the existing
//! `sam` CLI contract — dry-run, revision check, transactions — so the file
//! watcher, undo, and every other surface keep working unchanged.
//!
//! The module is framework-free like the rest of `sam-core` (D18): it uses
//! only `std::process`, threads, and channels, mirroring `selfcheck.rs`'s
//! spawn discipline but streaming instead of buffering.
//!
//! This file owns detection, prompt composition, the process session, and the
//! dispatcher arms for `agent.list` / `agent.run`. The protocol table is
//! `agent/defs.rs`; the stream parsers are `agent/stream.rs`.

mod defs;
mod stream;

pub use defs::{AgentDef, CustomProfile, Invocation, PromptDelivery, AGENT_DEFS};
pub use stream::{StreamFormat, StreamParser, StdinSlot};

use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::dispatch::{DispatchError, DispatchOutcome, DispatchRequest};
use crate::resources;

/// One thing that happened during a run, small enough for the deck to draw
/// without a second decoding layer. Serialized as `{"kind":"text", …}`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AgentEvent {
    /// The child started. Emitted by the session, not by a parser.
    Started { agent: String, bin: String },
    /// The CLI's own session handle, captured from the stream so the next deck
    /// turn can continue the same conversation (`claude --resume`,
    /// `opencode -s`). Emitted only by adapters that declare `resume_args`.
    Session { id: String },
    /// Assistant prose (a delta or a whole message, depending on the CLI).
    Text { text: String },
    /// Extended-thinking prose, shown collapsed.
    Thinking { text: String },
    /// A tool call, summarised to the argument a human would name it by.
    Tool { id: String, name: String, summary: String },
    /// A write-tool call naming the file it touched.
    File { path: String },
    /// A tool's result; the deck only draws failures, but the CLI's own
    /// diagnostics (a failed `sam apply` line) ride here.
    ToolResult {
        tool_use_id: String,
        is_error: bool,
        content: String,
    },
    /// A terminal summary frame from the CLI.
    Result { summary: String },
    /// Something the CLI reported as wrong (auth, provider, protocol).
    Error { message: String },
    /// A stdout line no parser recognised — visible drift, not silence.
    Log { line: String },
    /// A stderr line: the CLI's own diagnostics, drawn subdued.
    Stderr { line: String },
    /// The run ended. `code` is negative for a signal or a cancel.
    Exit { code: i32 },
}

/// What `agent.list` reports for one CLI.
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub id: String,
    pub name: String,
    pub bin: String,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub available: bool,
    pub install_hint: String,
}

impl AgentStatus {
    fn json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "bin": self.bin,
            "path": self.path.as_ref().map(|path| path.display().to_string()),
            "version": self.version,
            "available": self.available,
            "installHint": self.install_hint,
        })
    }
}

// ── detection ────────────────────────────────────────────────────────────────

/// How long a probe pass is trusted: long enough that a deck opened twice
/// pays once, short enough that a CLI installed mid-session appears after a
/// coffee's pause.
const DETECTION_TTL: Duration = Duration::from_secs(30);

type DetectionCache = Mutex<Option<(Instant, Vec<String>, Vec<AgentStatus>)>>;
static DETECTION: OnceLock<DetectionCache> = OnceLock::new();

/// The version flag a custom profile is probed with.
const PROFILE_VERSION_ARGS: &[&str] = &["--version"];

/// Probe every declared adapter (and every custom profile) for its binary.
/// Fault-isolated: one broken CLI never hides the others.
///
/// Availability is a `PATH` scan (no child process), so it stays on this
/// thread. The version is a spawn — a node CLI takes 60–350 ms just to print
/// one line — and the probes are independent, so they run together: the deck
/// waits for the slowest CLI instead of the sum of every CLI on the machine
/// (`agent.list` gates the deck's Send button until it answers).
pub fn detect_with(profiles: &[CustomProfile]) -> Vec<AgentStatus> {
    let found: Vec<Option<PathBuf>> = AGENT_DEFS
        .iter()
        .map(|def| find_binary(def.bin, def.fallback_bins))
        .collect();
    let versions = probe_versions(
        found
            .iter()
            .cloned()
            .zip(AGENT_DEFS.iter().map(|def| def.version_args))
            .collect(),
    );
    let mut statuses: Vec<AgentStatus> = AGENT_DEFS
        .iter()
        .zip(found)
        .zip(versions)
        .map(|((def, found), version)| AgentStatus {
            id: def.id.to_string(),
            name: def.name.to_string(),
            bin: def.bin.to_string(),
            available: found.is_some(),
            path: found,
            version,
            install_hint: def.install_hint.to_string(),
        })
        .collect();
    let profile_found: Vec<Option<PathBuf>> = profiles
        .iter()
        .map(|profile| find_binary(&profile.bin, &[]))
        .collect();
    let profile_versions = probe_versions(
        profile_found
            .iter()
            .cloned()
            .map(|bin| (bin, PROFILE_VERSION_ARGS))
            .collect(),
    );
    for ((profile, found), version) in profiles.iter().zip(profile_found).zip(profile_versions) {
        statuses.push(AgentStatus {
            id: profile.id.clone(),
            name: profile.display_name(),
            bin: profile.bin.clone(),
            available: found.is_some(),
            path: found,
            version,
            install_hint: "fix the `bin` in agent/profiles.json, or install the CLI".into(),
        });
    }
    statuses
}

/// `detect_with` behind a short TTL. The version probe spawns every installed
/// CLI (node CLIs are not cheap), and `agent.list` / `agent.models` must not
/// pay that on every deck open. The profile fingerprint is part of the cache
/// key, so editing `agent/profiles.json` invalidates it without a restart.
pub fn detect_cached(profiles: &[CustomProfile]) -> Vec<AgentStatus> {
    let fingerprint: Vec<String> = profiles
        .iter()
        .map(|profile| format!("{}:{}", profile.id, profile.bin))
        .collect();
    let cache = DETECTION.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some((at, held, statuses)) = guard.as_ref() {
            if at.elapsed() < DETECTION_TTL && held == &fingerprint {
                return statuses.clone();
            }
        }
    }
    let statuses = detect_with(profiles);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some((Instant::now(), fingerprint, statuses.clone()));
    }
    statuses
}

/// Resolve one adapter **cheaply** — a PATH scan, no `--version` spawn. A run
/// needs only the binary and the argv; versions are the deck's display
/// concern, and paying a probe per installed CLI on every Send is exactly the
/// freeze this replaces.
pub fn resolve_status(
    profiles: &[CustomProfile],
    wanted: Option<&str>,
) -> Result<AgentStatus, String> {
    let mut statuses: Vec<AgentStatus> = AGENT_DEFS
        .iter()
        .map(|def| {
            let found = find_binary(def.bin, def.fallback_bins);
            AgentStatus {
                id: def.id.to_string(),
                name: def.name.to_string(),
                bin: def.bin.to_string(),
                available: found.is_some(),
                path: found,
                version: None,
                install_hint: def.install_hint.to_string(),
            }
        })
        .collect();
    for profile in profiles {
        let found = find_binary(&profile.bin, &[]);
        statuses.push(AgentStatus {
            id: profile.id.clone(),
            name: profile.display_name(),
            bin: profile.bin.clone(),
            available: found.is_some(),
            path: found,
            version: None,
            install_hint: "fix the `bin` in agent/profiles.json, or install the CLI".into(),
        });
    }
    pick_status(&statuses, wanted)
}

/// Where a packaged desktop app has to look beyond its own `PATH`. A macOS app
/// opened from Finder inherits launchd's `PATH` — `/usr/bin:/bin:/usr/sbin:/sbin`
/// — so the Homebrew, npm-global and per-user bin directories that hold every
/// coding-agent CLI are invisible, and the deck reports "no CLI found" beside a
/// working `which claude`. `cli_install.rs` already treats these as the places a
/// login shell looks; detection reads the same list so the two surfaces agree.
#[cfg(unix)]
const PATH_EXTRAS: &[&str] = &["/usr/local/bin", "/opt/homebrew/bin", "/opt/local/bin"];

/// Per-user bin directories a login shell adds and a GUI app does not.
/// `.opencode/bin` is OpenCode's own installer location — a CLI that is on the
/// user's shell `PATH` (because their rc file puts it there) but invisible to a
/// Finder-launched app otherwise.
#[cfg(unix)]
const HOME_BINS: &[&str] = &[
    ".local/bin",
    ".cargo/bin",
    ".bun/bin",
    ".npm-global/bin",
    ".opencode/bin",
    "bin",
];

fn home_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var_os("HOME")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
    }
    #[cfg(not(unix))]
    {
        std::env::var_os("USERPROFILE")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
    }
}

/// The search space for a CLI: the app's own `PATH`, then the directories a
/// login shell would have added. A plan may name one by absolute path too
/// (`find_binary` honours that first), which is the custom-profile escape hatch.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    #[cfg(unix)]
    {
        dirs.extend(PATH_EXTRAS.iter().map(PathBuf::from));
        if let Some(home) = home_dir() {
            dirs.extend(HOME_BINS.iter().map(|sub| home.join(sub)));
        }
    }
    dirs
}

fn find_binary(bin: &str, fallback_bins: &[&str]) -> Option<PathBuf> {
    let names: Vec<&str> = std::iter::once(bin).chain(fallback_bins.iter().copied()).collect();
    // An absolute `bin` (a custom profile naming `/opt/homebrew/bin/claude`) is
    // the answer to a `PATH` the app cannot see; never search for it.
    let named = Path::new(bin);
    if named.is_absolute() {
        return is_executable(named).then(|| named.to_path_buf());
    }
    find_binary_in(&search_dirs(), &names)
}

/// The name is the outer loop: a primary name found anywhere beats a fallback
/// name found earlier. This matters for OpenCode, whose two binaries are one
/// generation apart (`opencode2` reads a V2 config, a 1.x `opencode` refuses
/// it); a stale 1.x install in an earlier `PATH` directory must not shadow the
/// V2 binary the adapter's argv is written for. Name-major is also the honest
/// reading of "a drop-in fork when the real one is absent" — the fallback is
/// only ever a substitute.
fn find_binary_in(dirs: &[PathBuf], names: &[&str]) -> Option<PathBuf> {
    for name in names {
        for dir in dirs {
            for candidate in executable_candidates(dir, name) {
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

#[cfg(unix)]
fn executable_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    vec![dir.join(name)]
}

#[cfg(windows)]
fn executable_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![dir.join(name)];
    if let Some(exts) = std::env::var_os("PATHEXT") {
        for ext in std::env::split_paths(&exts) {
            let ext = ext.to_string_lossy().to_lowercase();
            candidates.push(dir.join(format!("{name}{ext}")));
        }
    }
    candidates
}

#[cfg(unix)]
pub(crate) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Drain one pipe on its own thread, so a child that talks while the clock runs
/// cannot block on a full buffer.
fn drain_pipe(mut pipe: impl io::Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = pipe.read_to_end(&mut buffer);
        buffer
    })
}

/// Wait for a drain thread only as long as the caller can afford. A grandchild
/// that inherited the pipe keeps it open after the CLI exits, and joining it
/// would hang the caller; the thread is detached instead and ends when the last
/// writer lets go. Nothing leaks but the thread.
fn join_bounded(handle: thread::JoinHandle<Vec<u8>>, grace: Duration) -> Option<Vec<u8>> {
    let deadline = Instant::now() + grace;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
    handle.join().ok()
}

/// The line a CLI says about itself, when it says anything. A CLI whose
/// `--version` fails writes its error to stderr; reporting that as the version
/// puts a stack trace in the deck's CLI pill (OpenCode's unwritable log
/// directory did exactly that), so a line has to look like a version to count.
/// A JS stack frame (`    at Pp (/$bunfs/root/chunk.js:2:956)`) is the shape
/// that slips past a word list — it names no error, but it is not a version.
fn version_line(stdout: &str, stderr: &str) -> Option<String> {
    const NOISE: &[&str] = &[
        "error", "panic", "traceback", "denied", "eperm", "cannot", "failed", "warning",
        "not found", "unknown",
    ];
    let clean = |line: &str| -> Option<String> {
        let line = line.trim();
        if line.is_empty() || line.chars().count() > 80 {
            return None;
        }
        let lowered = line.to_ascii_lowercase();
        if lowered.starts_with("at ") || NOISE.iter().any(|word| lowered.contains(word)) {
            return None;
        }
        line.chars().any(|c| c.is_ascii_digit()).then(|| line.to_string())
    };
    stdout.lines().find_map(clean).or_else(|| stderr.lines().find_map(clean))
}

/// `<bin> --version` with a hard bound. A CLI that hangs on its version flag
/// is "present, version unknown" — never a stuck detection pass.
fn probe_version(bin: &Path, args: &[&str]) -> Option<String> {
    let mut command = Command::new(bin);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // The probe runs in the same search space detection read: a Homebrew CLI's
    // `#!/usr/bin/env node` has to find its interpreter.
    if let Some(path) = path_for_children(None) {
        command.env("PATH", path);
    }
    let mut child = command.spawn().ok()?;
    let out_thread = drain_pipe(child.stdout.take()?);
    let err_thread = drain_pipe(child.stderr.take()?);
    let deadline = Instant::now() + Duration::from_millis(2500);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => return None,
        }
    }
    let stdout = join_bounded(out_thread, Duration::from_millis(400)).unwrap_or_default();
    let stderr = join_bounded(err_thread, Duration::from_millis(400)).unwrap_or_default();
    version_line(
        &String::from_utf8_lossy(&stdout),
        &String::from_utf8_lossy(&stderr),
    )
}

/// Probe every found binary's version together, keeping the order of `jobs`.
///
/// One probe is one child process with its own 2.5 s bound; run serially they
/// put the sum of every installed CLI's startup in front of `agent.list` — and
/// that answer is what the deck's Send button waits on. They run in a small
/// pool rather than one thread each: a probe is a whole CLI boot, and a machine
/// with every adapter installed would otherwise start seven of them at once.
/// A probe that panics reads as "version unknown", never as a missing CLI:
/// availability was decided before the spawn and stays whatever the probe did.
fn probe_versions(jobs: Vec<(Option<PathBuf>, &'static [&'static str])>) -> Vec<Option<String>> {
    /// The most CLI boots one deck open may start at once.
    const WORKERS: usize = 4;
    if jobs.is_empty() {
        return Vec::new();
    }
    let chunk = jobs.len().div_ceil(jobs.len().min(WORKERS));
    let mut versions: Vec<Option<String>> = Vec::with_capacity(jobs.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .chunks(chunk)
            .map(|slice| {
                let len = slice.len();
                let handle = scope.spawn(move || {
                    slice
                        .iter()
                        .map(|(bin, args)| bin.as_ref().and_then(|bin| probe_version(bin, args)))
                        .collect::<Vec<_>>()
                });
                (len, handle)
            })
            .collect();
        for (len, handle) in handles {
            match handle.join() {
                Ok(part) => versions.extend(part),
                Err(_) => versions.extend(std::iter::repeat_n(None, len)),
            }
        }
    });
    versions
}

/// The user's escape hatch file: `data_dir/agent/profiles.json`
/// (`{ "agents": [ … ] }` or a bare array). A missing or malformed file is
/// an empty list — profiles are optional, never a startup failure.
pub fn profiles_path(data_dir: &Path) -> PathBuf {
    data_dir.join("agent").join("profiles.json")
}

pub fn load_profiles(data_dir: &Path) -> Vec<CustomProfile> {
    let Ok(text) = std::fs::read_to_string(profiles_path(data_dir)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let array = match &value {
        Value::Array(items) => items.clone(),
        Value::Object(map) => map
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    array
        .iter()
        .filter_map(|entry| serde_json::from_value::<CustomProfile>(entry.clone()).ok())
        .collect()
}

/// Why the escape hatch did not load, when it is there at all. A malformed
/// `profiles.json` is an empty list by design (profiles are optional), but a
/// silent one is a typo the student cannot see, so `agent.list` reports the
/// line and the deck draws it.
pub fn profiles_problem(data_dir: &Path) -> Option<String> {
    let path = profiles_path(data_dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            return path
                .exists()
                .then(|| format!("{}: cannot be read", path.display()))
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => return Some(format!("{}: {error}", path.display())),
    };
    let array = match &value {
        Value::Array(items) => items.as_slice(),
        Value::Object(map) => map.get("agents").and_then(Value::as_array)?.as_slice(),
        _ => {
            return Some(format!(
                "{}: expected an array or {{\"agents\": [...]}}",
                path.display()
            ))
        }
    };
    let loaded = array
        .iter()
        .filter(|entry| serde_json::from_value::<CustomProfile>((*entry).clone()).is_ok())
        .count();
    (loaded < array.len()).then(|| {
        format!(
            "{}: {} of {} profiles ignored — each needs id, bin and args",
            path.display(),
            array.len() - loaded,
            array.len()
        )
    })
}

fn data_dir(resources_dir: &Path) -> Option<PathBuf> {
    resources::paths_with_resources(resources_dir.to_path_buf())
        .ok()
        .map(|paths| paths.data_dir)
}

pub fn load_profiles_for(resources_dir: &Path) -> Vec<CustomProfile> {
    data_dir(resources_dir)
        .map(|dir| load_profiles(&dir))
        .unwrap_or_default()
}

// ── the plan's own CLI, and the child's environment ─────────────────────────

/// The `sam` binary the agent should call: the sidecar next to the executable
/// Tauri bundles, else whatever `PATH` resolves. `None` is a visible blocker
/// in the deck, never a silently wrong invocation.
pub fn find_sam_binary(exe_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = exe_dir {
        let candidate = dir.join(exe_name("sam"));
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    find_binary("sam", &[])
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// The `PATH` a child should inherit: the resolved `sam` directory first, then
/// every directory the app can see a CLI in.
///
/// Detection reads beyond `PATH` (a GUI app's is launchd's four directories),
/// so the child must be given the same search space the CLI was *found* in. A
/// Homebrew CLI is a `#!/usr/bin/env node` script: finding `/opt/homebrew/bin/pi`
/// is not enough if `node` — also in `/opt/homebrew/bin` — is not on the child's
/// `PATH`, and the turn fails at launch with "node: command not found".
fn path_for_children(sam: Option<&Path>) -> Option<std::ffi::OsString> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(dir) = sam.and_then(Path::parent) {
        dirs.push(dir.to_path_buf());
    }
    for dir in search_dirs() {
        // Only what exists: `PATH` entries that cannot be used are noise a
        // shell spends a `stat` on for every lookup.
        if dir.is_dir() && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    std::env::join_paths(dirs).ok()
}

/// Environment for the agent child: the resolved `sam` directory first on
/// `PATH` (so a bundled sidecar and a PATH install cannot disagree about which
/// CLI the agent calls), `SAM_RESOURCES` pointed at this app's resources — the
/// same seam `selfcheck.rs` uses — and `SAM_PLAN` naming the open plan, so a
/// `sam` call that forgets `--plan` still lands on the folder the window has
/// open instead of the machine's default.
pub fn child_env(
    sam: Option<&Path>,
    resources_dir: &Path,
    plan_root: &Path,
    run_id: &str,
) -> Vec<(String, String)> {
    let mut env = vec![
        (
            resources::RESOURCES_ENV.to_string(),
            resources_dir.display().to_string(),
        ),
        (
            resources::PLAN_ENV.to_string(),
            plan_root.display().to_string(),
        ),
        // The turn's identity, so a transaction the agent commits can be
        // attributed to this run instead of guessed from a timestamp.
        (resources::RUN_ENV.to_string(), run_id.to_string()),
    ];
    if let Some(path) = path_for_children(sam) {
        env.push(("PATH".to_string(), path.to_string_lossy().into_owned()));
    }
    env
}

// ── prompt composition ───────────────────────────────────────────────────────

/// The one prompt every run starts from: the plan folder, what the app is
/// showing right now, the safe authoring loop, and the `sam-authoring` skill
/// body (shipped inside the binary, so the CLI and the desktop can never
/// compose different prompts).
pub fn compose_prompt(
    plan_root: &Path,
    sam: Option<&Path>,
    context: Option<&str>,
    user_prompt: &str,
) -> String {
    let skill = strip_frontmatter(include_str!("../../../.agents/skills/sam-authoring/SKILL.md"));
    let sam_line = match sam {
        Some(path) => format!(
            "`sam` is at `{}` — if your shell cannot find `sam`, call it by that absolute path.",
            path.display()
        ),
        None => "`sam` was not found next to this app or on PATH — stop and tell the student \
                 the CLI is missing instead of hand-editing files."
            .to_string(),
    };
    let window = match context.map(str::trim).filter(|text| !text.is_empty()) {
        Some(context) => clip_text(context, 2000),
        None => "No window context was handed over (a terminal run): the plan folder is the whole brief."
            .to_string(),
    };
    // Escape plan path for shell execution (shell_quote).
    let plan = plan_root.display().to_string();
    let plan_q = shell_quote(&plan);
    format!(
        "You are the assistant inside SAM, a study-plan app. The plan this window has open is:\n\n    {plan}\n\n\
The folder may live anywhere the student chose — never rely on the machine's default: name it on every \
call as `--plan {plan_q}` (single-quoted exactly like that; the path may contain spaces). The child's \
environment already names it too (`SAM_PLAN`), so a call that forgets the flag still lands here.\n\n\
Author it only through the `sam` CLI — {sam_line} Never edit the plan's JSON or JSONL files by hand: \
every write goes through the CLI so it stays transactional, revision-checked and undoable.\n\n\
# What the app is showing right now\n\n{window}\n\n\
Treat that snapshot as where the student is standing: when the request does not name a screen, it is \
about what they are looking at; when it is about the plan at large, say so in your reply before you touch \
anything. Stale detail is cheap to re-read — `sam view <name> --plan {plan_q} --json` and \
`sam records --type <kind> --plan {plan_q} --json` are one call each.\n\n\
Work the safe loop: `sam paths --plan {plan_q} --json` gives the plan's revision; stage candidate records \
in a temp file OUTSIDE the plan (for example `${{TMPDIR:-/tmp}}/batch.jsonl`), preview with \
`sam apply <file> --dry-run --json --plan {plan_q}`, fix every diagnostic, then commit with \
`sam apply <file> --if-revision <hash> --json --plan {plan_q}`. Verify with \
`sam configcheck --plan {plan_q} --json` and `sam today.view --plan {plan_q} --json`.\n\n\
Screen work — composing or changing views and their blocks — is the `view.*` family \
(`view.block.add`, `view.block.set`, `view.setComponents`, …): `sam --schema --json --plan {plan_q}` \
carries the block vocabulary, and every command answers `sam <id> --help` with its exact flags. \
`sam --commands --plan {plan_q} --json` lists them all.\n\n\
Say briefly what you are about to do, then do it.\n\n---\n\n# The sam-authoring skill\n\n{skill}\n\n---\n\n# Task\n\n{task}\n",
        plan = plan,
        plan_q = plan_q,
        sam_line = sam_line,
        window = window,
        skill = skill,
        task = user_prompt.trim(),
    )
}

/// A path as a shell word. The taught commands are pasted into a shell, so a
/// folder named `week "4"` must not close the argument and `$(…)` must not run.
/// Single quotes are literal in POSIX shells and in PowerShell alike; a path
/// that itself carries `'` closes and re-opens the word (`'\''`) — the POSIX
/// form, because a plan folder with a quote in its name is rarer than a
/// PowerShell-only agent.
pub fn shell_quote(path: &str) -> String {
    let mut quoted = String::with_capacity(path.len() + 2);
    quoted.push('\'');
    for character in path.chars() {
        if character == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}

/// Clip to a character budget with a visible ellipsis (the same rule the
/// stream parsers use for payloads).
fn clip_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut clipped: String = text.chars().take(max).collect();
    clipped.push('…');
    clipped
}

/// Model ids and session ids reach a child process as argv, so they must never
/// look like flags or carry whitespace (OpenDesign's `sanitizeCustomModel`,
/// kept: same shape `[A-Za-z0-9][A-Za-z0-9._/:@-]*`, same 200-char ceiling).
pub fn sanitize_choice(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return None;
    }
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._/:@-".contains(c))
    {
        return None;
    }
    Some(trimmed.to_string())
}

fn strip_frontmatter(text: &str) -> &str {
    let rest = text.strip_prefix("---\n");
    match rest {
        Some(rest) => match rest.find("\n---\n") {
            Some(end) => rest[end + 5..].trim_start_matches('\n'),
            None => text,
        },
        None => text,
    }
}

// ── the session ──────────────────────────────────────────────────────────────

/// Everything one run needs after detection and composition have happened.
pub struct RunSpec {
    /// Stable per-process id: the shell tags every streamed event with it so a
    /// late `Exit` from a cancelled turn cannot end the next one.
    pub run_id: String,
    pub id: String,
    pub name: String,
    pub bin: PathBuf,
    pub args: Vec<String>,
    pub delivery: PromptDelivery,
    pub format: StreamFormat,
    pub prompt: String,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    /// The turn owns a process group, so Stop reaches what the CLI started too.
    /// The deck sets it; a terminal run leaves the child in the foreground
    /// group so Ctrl-C still reaches both.
    pub isolate: bool,
    /// What the engine wants said before the first frame (a dropped model id, a
    /// clipped prompt) — drawn as a transcript line, never swallowed.
    pub notes: Vec<String>,
}

/// A running (or finished) session. Cloning is not needed: the shell keeps
/// one `AgentRun` and calls `cancel` on it.
pub struct AgentRun {
    pub run_id: String,
    child: Arc<Mutex<Option<Child>>>,
    outcome: Arc<(Mutex<Option<i32>>, Condvar)>,
    isolate: bool,
}

impl AgentRun {
    /// Stop the child now. SIGKILL composes with SAM's own crash story: the
    /// plan can only be mid-write inside a transaction, and transactions
    /// recover at their journal boundary (`selfcheck` gates that path).
    pub fn cancel(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                // The CLI's own tools are children of it: a `bash -c "sam
                // apply …"` still holding the plan lock at the instant the
                // deck says "Stopped" is exactly what must not happen.
                if self.isolate {
                    kill_group(child.id() as i32);
                }
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    /// Block until the supervisor has emitted `Exit` and recorded the code.
    pub fn wait(&self) -> i32 {
        let (lock, cvar) = &*self.outcome;
        let mut guard = lock.lock().unwrap_or_else(|error| error.into_inner());
        while guard.is_none() {
            guard = cvar
                .wait(guard)
                .unwrap_or_else(|error| error.into_inner());
        }
        guard.unwrap_or(-2)
    }
}

/// Signal a whole process group. No `libc` dependency (§9): the platform's own
/// `kill` is asked, and the direct child is killed by the caller either way.
#[cfg(unix)]
fn kill_group(pid: i32) {
    if pid <= 1 {
        return;
    }
    let group = format!("-{pid}");
    let exe = ["/bin/kill", "/usr/bin/kill"]
        .iter()
        .find(|exe| Path::new(exe).exists())
        .copied();
    let Some(exe) = exe else { return };
    // `--` ends options (GNU needs it for a negative pid); BSD `kill` accepts
    // it too, and the second form is the fallback when it does not.
    let ok = Command::new(exe)
        .args(["-9", "--", group.as_str()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !ok {
        let _ = Command::new(exe)
            .args(["-9", group.as_str()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// `taskkill /T` walks the child tree; the group flag is not needed for it.
#[cfg(not(unix))]
fn kill_group(pid: i32) {
    if pid <= 1 {
        return;
    }
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn next_run_id() -> String {
    format!("run-{}", RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed))
}

/// One line of a child's stream as text, byte-safe and bounded.
///
/// `BufRead::lines()` gives up on the first non-UTF-8 byte, which ends a turn's
/// transcript and closes the pipe under the CLI (it dies of SIGPIPE on its next
/// write). A stray byte is a decoding problem, not the end of the turn: it
/// becomes U+FFFD and the line is kept. The line length is capped so one
/// enormous frame — the complete `assistant` message carrying a whole file's
/// content — cannot grow the reader without bound; the rest is drained.
const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

fn read_line_capped(reader: &mut impl BufRead) -> io::Result<Option<(String, bool)>> {
    read_line_capped_with(reader, MAX_LINE_BYTES)
}

fn read_line_capped_with(
    reader: &mut impl BufRead,
    cap: usize,
) -> io::Result<Option<(String, bool)>> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut read_any = false;
    loop {
        let (newline, consumed) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                break;
            }
            read_any = true;
            let newline = available.iter().position(|byte| *byte == b'\n');
            let end = newline.unwrap_or(available.len());
            if !truncated {
                let room = cap.saturating_sub(bytes.len());
                if end <= room {
                    bytes.extend_from_slice(&available[..end]);
                } else {
                    bytes.extend_from_slice(&available[..room]);
                    truncated = true;
                }
            }
            (newline, end + usize::from(newline.is_some()))
        };
        reader.consume(consumed);
        if newline.is_some() {
            break;
        }
    }
    if !read_any {
        return Ok(None);
    }
    Ok(Some((String::from_utf8_lossy(&bytes).into_owned(), truncated)))
}

/// Write the prompt without holding the stdin lock for the whole of it. A
/// prompt larger than the pipe buffer would otherwise keep the lock across a
/// blocking write while the parser needs the same lock to answer a Pi dialog —
/// the reader stops draining, the child waits for the answer, and nobody
/// moves. Releasing the lock between chunks bounds that to one pipe write.
fn write_chunked(slot: &StdinSlot, bytes: &[u8]) {
    const CHUNK: usize = 4 * 1024;
    for chunk in bytes.chunks(CHUNK) {
        let Ok(mut guard) = slot.lock() else { return };
        let Some(stdin) = guard.as_mut() else { return };
        if stdin.write_all(chunk).is_err() {
            return;
        }
        let _ = stdin.flush();
    }
}

/// Spawn one turn and stream its events to `on_event` from reader threads.
/// The returned handle's supervisor emits [`AgentEvent::Exit`] when the child
/// is done — callers do not emit it themselves.
pub fn start(
    spec: RunSpec,
    on_event: Arc<dyn Fn(AgentEvent) + Send + Sync>,
) -> Result<AgentRun, String> {
    let mut command = Command::new(&spec.bin);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if spec.delivery.needs_stdin() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    #[cfg(unix)]
    if spec.isolate {
        // Its own group, so Stop can signal the CLI and everything it started.
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", spec.bin.display()))?;

    let stdin_slot: StdinSlot = Arc::new(Mutex::new(child.stdin.take()));
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child_shared = Arc::new(Mutex::new(Some(child)));
    let outcome = Arc::new((Mutex::new(None), Condvar::new()));
    let run_id = spec.run_id.clone();

    on_event(AgentEvent::Started {
        agent: spec.id.clone(),
        bin: spec.bin.display().to_string(),
    });
    for note in &spec.notes {
        on_event(AgentEvent::Log { line: note.clone() });
    }

    // The prompt goes to stdin once, on its own thread so a CLI that does not
    // read stdin until it is ready cannot stall the readers.
    if spec.delivery.needs_stdin() {
        let prompt = spec.prompt.clone();
        let delivery = spec.delivery;
        let slot = stdin_slot.clone();
        thread::spawn(move || {
            match delivery {
                PromptDelivery::StdinText => write_chunked(&slot, prompt.as_bytes()),
                PromptDelivery::StdinPiRpc => {
                    let command = json!({ "id": 1, "type": "prompt", "message": prompt });
                    if let Ok(mut guard) = slot.lock() {
                        if let Some(stdin) = guard.as_mut() {
                            let _ = writeln!(stdin, "{command}");
                            let _ = stdin.flush();
                        }
                    }
                }
                PromptDelivery::StdinAntigravity => {
                    let frame = json!({ "event": "user", "message": { "content": prompt } });
                    if let Ok(mut guard) = slot.lock() {
                        if let Some(stdin) = guard.as_mut() {
                            let _ = writeln!(stdin, "{frame}");
                            let _ = stdin.flush();
                        }
                    }
                }
                PromptDelivery::Argv => {}
            }
            if delivery != PromptDelivery::StdinPiRpc {
                // EOF is the protocol's "that is the whole turn" for every
                // delivery but Pi's RPC, which closes stdin on `agent_end`
                // (the parser owns that half).
                if let Ok(mut guard) = slot.lock() {
                    guard.take();
                }
            }
        });
    }

    let stdout_thread = {
        let emitter = on_event.clone();
        let slot = stdin_slot.clone();
        let format = spec.format;
        thread::spawn(move || {
            let Some(stdout) = stdout else { return };
            let mut reader = BufReader::new(stdout);
            let mut parser = StreamParser::new(format);
            loop {
                match read_line_capped(&mut reader) {
                    Ok(Some((line, truncated))) => {
                        if truncated {
                            emitter(AgentEvent::Log {
                                line: format!(
                                    "an output line was longer than {} MB; the rest was dropped",
                                    MAX_LINE_BYTES / (1024 * 1024)
                                ),
                            });
                        }
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        for event in parser.feed(line, Some(&slot)) {
                            emitter(event);
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        })
    };
    let stderr_thread = {
        let emitter = on_event.clone();
        thread::spawn(move || {
            let Some(stderr) = stderr else { return };
            let mut reader = BufReader::new(stderr);
            loop {
                match read_line_capped(&mut reader) {
                    Ok(Some((line, _))) => {
                        // A CLI's own colour codes are not a message: strip
                        // them before they reach the deck's subdued line.
                        let line = crate::agent::stream::strip_ansi(line.trim());
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        emitter(AgentEvent::Stderr {
                            line: line.chars().take(500).collect(),
                        });
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        })
    };

    // Supervisor: the child's exit is the truth and the streams are a courtesy.
    // Poll the process while the readers run — a CLI that exits while a
    // grandchild still holds its pipe must not keep the turn alive — and give
    // a closed stream a short grace to be read before stopping a CLI that is
    // only lingering.
    let shared = child_shared.clone();
    let result_slot = outcome.clone();
    let emitter = on_event.clone();
    thread::spawn(move || {
        let mut streams_closed_at: Option<Instant> = None;
        let code: i32 = loop {
            let state: Option<i32> = {
                let mut guard = shared.lock().unwrap_or_else(|error| error.into_inner());
                match guard.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => Some(status_code(status)),
                        // Still running.
                        Ok(None) => None,
                        Err(_) => Some(-1),
                    },
                    // Cancel took the child out of the slot; it already
                    // killed and reaped it.
                    None => Some(-2),
                }
            };
            if let Some(status) = state {
                break status;
            }
            let closed = stdout_thread.is_finished() && stderr_thread.is_finished();
            match streams_closed_at {
                None if closed => streams_closed_at = Some(Instant::now()),
                Some(at) if closed && at.elapsed() >= Duration::from_millis(1500) => {
                    // Both streams have closed: the CLI said everything it had
                    // to say and is only lingering (a node CLI winding down).
                    // Stopping it is housekeeping, not a failed turn — the code
                    // is success, and one line says so, so a genuinely stuck
                    // CLI stays visible.
                    if let Ok(mut guard) = shared.lock() {
                        if let Some(child) = guard.as_mut() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                    emitter(AgentEvent::Log {
                        line: "the CLI lingered after its stream closed; stopped after 1.5 s".into(),
                    });
                    break 0;
                }
                _ => {}
            }
            thread::sleep(Duration::from_millis(40));
        };
        // The process is done; a reader still draining is given a moment to
        // finish its buffered lines and then left to end on its own. It exits
        // when the last writer closes the pipe — a grandchild's problem, not
        // the turn's.
        let drain_deadline = Instant::now() + Duration::from_millis(2000);
        while !(stdout_thread.is_finished() && stderr_thread.is_finished())
            && Instant::now() < drain_deadline
        {
            thread::sleep(Duration::from_millis(10));
        }
        {
            let (lock, cvar) = &*result_slot;
            let mut guard = lock.lock().unwrap_or_else(|error| error.into_inner());
            *guard = Some(code);
            cvar.notify_all();
        }
        emitter(AgentEvent::Exit { code });
    });

    Ok(AgentRun {
        run_id,
        child: child_shared,
        outcome,
        isolate: spec.isolate,
    })
}

#[cfg(unix)]
fn status_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => code,
        None => -status.signal().unwrap_or(1),
    }
}

#[cfg(not(unix))]
fn status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

// ── the dispatcher's door ────────────────────────────────────────────────────

/// Detection for one machine, shaped for `agent.list` and the deck.
pub fn list_json(resources_dir: &Path) -> Value {
    let profiles = load_profiles_for(resources_dir);
    let statuses = detect_cached(&profiles);
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let sam = find_sam_binary(exe_dir.as_deref());
    let data = data_dir(resources_dir);
    json!({
        "sam": {
            "path": sam.as_ref().map(|path| path.display().to_string()),
            "available": sam.is_some(),
        },
        "agents": statuses.iter().map(AgentStatus::json).collect::<Vec<_>>(),
        "profiles": data.as_ref().map(|dir| profiles_path(dir).display().to_string()),
        // Why the escape hatch did not load, when it is there: a typo in a
        // custom profile must not look like a machine with no profiles.
        "profilesError": data.as_ref().and_then(|dir| profiles_problem(dir)),
    })
}

/// Pick the adapter to run: the named one when the caller was explicit, else
/// the first installed one.
fn pick_status(statuses: &[AgentStatus], wanted: Option<&str>) -> Result<AgentStatus, String> {
    match wanted {
        Some(id) => statuses
            .iter()
            .find(|status| status.id == id)
            .filter(|status| status.available)
            .cloned()
            .ok_or_else(|| {
                format!("agent \"{id}\" is not installed — agent.list shows what this machine has")
            }),
        None => statuses
            .iter()
            .find(|status| status.available)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "no agent CLI found on PATH — install one of: {}",
                    AGENT_DEFS
                        .iter()
                        .map(|def| def.id)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }),
    }
}

/// One model row for the picker.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
}

/// Everything one run needs after detection and composition: the adapter, the
/// composed prompt, the child environment, and the `sam` binary.
pub struct RunRequest<'a> {
    pub agent_id: Option<&'a str>,
    /// The picker's choice; `None` = the CLI's own configured default.
    pub model: Option<&'a str>,
    /// A session id captured from a previous turn's stream, when the adapter
    /// can continue it.
    pub resume: Option<&'a str>,
    /// What the app is showing right now (built by the UI, clipped here).
    pub context: Option<&'a str>,
    pub prompt: &'a str,
    pub plan_root: &'a Path,
    pub resources_dir: &'a Path,
    /// The caller can stop this run, so it gets its own process group.
    pub isolate: bool,
}

/// The command line this platform accepts for an argv-delivered prompt.
/// Windows caps a whole `CreateProcess` line at 32 767 units; Unix caps one
/// argument at 128 KiB. The prompt carries the authoring skill (~7 kB), so a
/// long request can reach the Windows ceiling — naming the limit beats a spawn
/// failure the student cannot read.
fn argv_budget() -> usize {
    if cfg!(windows) {
        30_000
    } else {
        120 * 1024
    }
}

fn argv_length(args: &[String]) -> usize {
    args.iter().map(|arg| arg.chars().count() + 1).sum()
}

fn check_argv_budget(args: &[String]) -> Result<(), String> {
    let used = argv_length(args);
    let budget = argv_budget();
    if used <= budget {
        return Ok(());
    }
    Err(format!(
        "this turn's prompt is {used} characters, over the {budget} this platform accepts on a command \
         line — use a CLI that reads the prompt on stdin (Claude Code, OpenCode, Copilot, Antigravity, \
         Pi or Oh My Pi), or shorten the request"
    ))
}

pub fn prepare_run(request: RunRequest) -> Result<RunSpec, String> {
    let profiles = load_profiles_for(request.resources_dir);
    // Cheap resolution only: a PATH scan. The version probes belong to
    // `agent.list`'s display and are paid there (cached), never on Send.
    let status = resolve_status(&profiles, request.agent_id)?;
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let sam = find_sam_binary(exe_dir.as_deref());
    // The turn's identity exists before the child does: it is the environment
    // the CLI's own `sam apply` stamps into its transaction, and the key the
    // deck's receipt is read by.
    let run_id = next_run_id();
    let prompt = compose_prompt(
        request.plan_root,
        sam.as_deref(),
        request.context,
        request.prompt,
    );
    let mut notes: Vec<String> = Vec::new();
    // A model id reaches a child process as argv — it must never look like a
    // flag (OpenDesign's `sanitizeCustomModel`, kept). A chosen id that fails
    // the shape is dropped, not an error: the CLI's default is a safe answer —
    // but the deck is told, because a pill showing one model while the turn
    // runs another is a lie the student cannot see.
    let model = match request.model {
        Some(chosen) => {
            let sanitized = sanitize_choice(chosen);
            if sanitized.is_none() {
                notes.push(format!(
                    "the model id \"{chosen}\" is not usable as a flag value (letters, digits and \
                     ._/:@- only, ≤ 200 characters) — this turn uses the CLI's own default"
                ));
            }
            sanitized
        }
        None => None,
    };
    let model = model.as_deref();
    let resume = match request.resume {
        Some(id) => {
            let usable = sanitize_choice(id);
            if usable.is_none() {
                notes.push(
                    "the remembered conversation id was not usable as a flag value — this turn \
                     starts a fresh conversation"
                        .to_string(),
                );
            }
            usable
        }
        None => None,
    };

    if let Some(def) = AGENT_DEFS.iter().find(|def| def.id == status.id) {
        let invocation = Invocation { prompt: &prompt, model };
        let mut args = (def.build_args)(&invocation);
        if let (Some(build), Some(session)) = (def.resume_args, resume.as_deref()) {
            args.extend(build(session));
        }
        if def.delivery == PromptDelivery::Argv {
            check_argv_budget(&args)?;
        }
        return Ok(RunSpec {
            run_id: run_id.clone(),
            id: def.id.to_string(),
            name: def.name.to_string(),
            bin: status.path.clone().unwrap_or_else(|| PathBuf::from(def.bin)),
            args,
            delivery: def.delivery,
            format: def.stream_format,
            prompt,
            cwd: request.plan_root.to_path_buf(),
            env: child_env(
                sam.as_deref(),
                request.resources_dir,
                request.plan_root,
                &run_id,
            ),
            isolate: request.isolate,
            notes,
        });
    }

    // A custom profile: argv delivery with `{prompt}` / `{model}` substitution.
    let profile = profiles
        .iter()
        .find(|profile| profile.id == status.id)
        .ok_or_else(|| format!("agent \"{}\" is not declared", status.id))?;
    let args = profile.build_args(&prompt, model);
    check_argv_budget(&args)?;
    Ok(RunSpec {
        run_id: run_id.clone(),
        id: profile.id.clone(),
        name: profile.display_name(),
        bin: status
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&profile.bin)),
        args,
        delivery: PromptDelivery::Argv,
        format: profile.stream_format(),
        prompt,
        cwd: request.plan_root.to_path_buf(),
        env: child_env(
            sam.as_deref(),
            request.resources_dir,
            request.plan_root,
            &run_id,
        ),
        isolate: request.isolate,
        notes,
    })
}

// ── model discovery ──────────────────────────────────────────────────────────

/// The picker's models for one CLI: the CLI's own listing when it has one,
/// else the def's fallback hints. `source` says which the student is looking
/// at, so the deck can be honest about a guessed list.
pub fn models_json(agent_id: Option<&str>, resources_dir: &Path) -> Result<Value, String> {
    let profiles = load_profiles_for(resources_dir);
    let statuses = detect_cached(&profiles);
    let status = pick_status(&statuses, agent_id)?;
    let (models, source) = if let Some(def) = AGENT_DEFS.iter().find(|def| def.id == status.id) {
        let live = status.path.as_deref().and_then(|bin| {
            def.models_args
                .and_then(|args| list_models(bin, args, def.id))
        });
        match live.filter(|models| !models.is_empty()) {
            Some(models) => (models, "live"),
            None => (
                def.fallback_models
                    .iter()
                    .map(|id| ModelOption {
                        id: (*id).to_string(),
                        label: (*id).to_string(),
                    })
                    .collect(),
                "fallback",
            ),
        }
    } else if let Some(profile) = profiles.iter().find(|profile| profile.id == status.id) {
        (
            profile
                .models
                .iter()
                .map(|id| ModelOption {
                    id: id.clone(),
                    label: id.clone(),
                })
                .collect(),
            "profile",
        )
    } else {
        (Vec::new(), "none")
    };
    Ok(json!({
        "agent": status.id,
        "source": source,
        "models": models,
    }))
}

/// Run one CLI's model-listing command with a hard bound. `opencode models`
/// and `agy models` are network round-trips (OpenDesign measured >8 s), so the
/// picker asks lazily and this timeout is generous; detection never pays it.
///
/// The pipes are drained on their own threads while the deadline runs: a list
/// is unbounded output (`omp models` prints ~120 KB), and a reader that only
/// looks after exit would let the child block on a full pipe until the timeout
/// killed it — the failure that made Oh My Pi's list silently fall back to
/// "none" the first time this ran.
fn list_models(bin: &Path, args: &[&str], id: &str) -> Option<Vec<ModelOption>> {
    let mut command = Command::new(bin);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Same search space the CLI was found in: the listing command is often a
    // script whose interpreter lives beside it.
    if let Some(path) = path_for_children(None) {
        command.env("PATH", path);
    }
    let mut child = command.spawn().ok()?;
    let out_thread = drain_pipe(child.stdout.take()?);
    let err_thread = drain_pipe(child.stderr.take()?);
    let deadline = Instant::now() + Duration::from_millis(15_000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(40)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => return None,
        }
    }
    // A model list is still read while the CLI's own helpers wind down; a
    // reader that never ends (a grandchild holding the pipe) is dropped rather
    // than joined, so the picker always answers.
    let bytes = join_bounded(out_thread, Duration::from_millis(2_000)).unwrap_or_default();
    let _ = join_bounded(err_thread, Duration::from_millis(500));
    Some(parse_models(id, &String::from_utf8_lossy(&bytes)))
}

/// Per-CLI list shapes, verified against the binaries on the development
/// machine (`agent/defs.rs` carries each argv):
/// - `opencode models` — one `provider/model` per line;
/// - `agy models` — `id<TAB>Label` per line, after a progress line;
/// - `pi --list-models` — a TSV table whose first two columns are
///   `provider` and `model`;
/// - `omp models` — a boxed table under `provider (N)` group headers.
fn parse_models(id: &str, stdout: &str) -> Vec<ModelOption> {
    let mut models: Vec<ModelOption> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push = |id: String, label: String, models: &mut Vec<ModelOption>| {
        if id.is_empty() || models.len() >= 500 || !seen.insert(id.clone()) {
            return;
        }
        models.push(ModelOption { id, label });
    };
    match id {
        "opencode" => {
            for line in stdout.lines().map(str::trim) {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                push(line.to_string(), line.to_string(), &mut models);
            }
        }
        "antigravity" => {
            for line in stdout.lines() {
                let Some((id, label)) = line.split_once('\t') else {
                    continue; // the "Fetching available models…" line
                };
                let (id, label) = (id.trim(), label.trim());
                push(id.to_string(), label.to_string(), &mut models);
            }
        }
        "pi" => {
            for line in stdout.lines() {
                let mut columns = line.split_whitespace();
                let Some(provider) = columns.next() else {
                    continue;
                };
                if provider == "provider" {
                    continue; // header
                }
                let Some(model) = columns.next() else { continue };
                if provider.starts_with("Warning:") {
                    continue;
                }
                let id = format!("{provider}/{model}");
                push(id.clone(), id, &mut models);
            }
        }
        "omp" => {
            let mut provider = String::new();
            for line in stdout.lines() {
                let trimmed = line.trim();
                // A group header is a bare `name (N)` line.
                if let Some((name, count)) = trimmed.rsplit_once(" (") {
                    if name.split_whitespace().count() == 1
                        && count.trim_end_matches(')').chars().all(|c| c.is_ascii_digit())
                    {
                        provider = name.trim().to_string();
                        continue;
                    }
                }
                if !trimmed.starts_with('│') {
                    continue;
                }
                let cells: Vec<&str> = trimmed.split('│').map(str::trim).collect();
                let Some(model) = cells.get(1) else { continue };
                if model.is_empty() || *model == "model" || model.chars().all(|c| c == '─') {
                    continue;
                }
                let label = if provider.is_empty() {
                    (*model).to_string()
                } else {
                    format!("{model} · {provider}")
                };
                push((*model).to_string(), label, &mut models);
            }
        }
        _ => {}
    }
    models
}

/// Run one turn to completion, collecting its events — the `sam agent.run` /
/// `sam invoke` door. The desktop streams instead (the shell calls `start`).
pub fn run_headless(spec: RunSpec) -> Result<(i32, Vec<AgentEvent>), String> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let collector = events.clone();
    let run = start(
        spec,
        Arc::new(move |event| {
            if let Ok(mut guard) = collector.lock() {
                guard.push(event);
            }
        }),
    )?;
    let code = run.wait();
    let events = events
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    Ok((code, events))
}

/// The `agent.*` dispatcher arms. Called from `CommandSession::run` **before**
/// any plan lock is taken — the agent's own `sam` calls need the write lock
/// while this command is still in flight.
pub fn dispatch_arm(
    id: &str,
    request: &DispatchRequest,
) -> Result<DispatchOutcome, DispatchError> {
    match id {
        "agent.list" => Ok(DispatchOutcome::Read(list_json(&request.resources_dir))),
        "agent.models" => {
            let agent_id = crate::dispatch::optional(&request.params, "agent");
            models_json(agent_id.as_deref(), &request.resources_dir)
                .map(DispatchOutcome::Read)
                .map_err(DispatchError::Usage)
        }
        "agent.run" => {
            let prompt = crate::dispatch::require(&request.params, "prompt")?;
            if prompt.trim().is_empty() {
                return Err(DispatchError::Usage(
                    "agent.run needs a non-empty prompt".into(),
                ));
            }
            let (root, _origin) = crate::config_store::plan_root(request.plan.as_deref())
                .map_err(DispatchError::from)?;
            let agent_id = crate::dispatch::optional(&request.params, "agent");
            let model = crate::dispatch::optional(&request.params, "model");
            let resume = crate::dispatch::optional(&request.params, "resume");
            let context = crate::dispatch::optional(&request.params, "context");
            let spec = prepare_run(RunRequest {
                agent_id: agent_id.as_deref(),
                model: model.as_deref(),
                resume: resume.as_deref(),
                context: context.as_deref(),
                prompt: &prompt,
                plan_root: &root,
                resources_dir: &request.resources_dir,
                // Nothing can stop a one-shot `sam agent.run`: no group needed.
                isolate: false,
            })
            .map_err(DispatchError::Usage)?;
            let agent = spec.id.clone();
            let (exit_code, output) = run_headless(spec).map_err(DispatchError::Io)?;
            Ok(DispatchOutcome::Read(json!({
                "agent": agent,
                "exitCode": exit_code,
                "plan": root.display().to_string(),
                "output": output,
            })))
        }
        other => Err(DispatchError::Usage(format!(
            "unknown agent command \"{other}\""
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_carries_the_plan_the_task_the_skill_and_the_window() {
        let prompt = compose_prompt(
            Path::new("/tmp/plan"),
            Some(Path::new("/bin/sam")),
            Some("looking at: Today"),
            "  add week 3  ",
        );
        assert!(prompt.contains("/tmp/plan"));
        assert!(prompt.contains("add week 3"));
        assert!(prompt.contains("# The sam-authoring skill"));
        assert!(prompt.contains("/bin/sam"));
        assert!(prompt.contains("looking at: Today"));
        // The YAML frontmatter belongs to skill hosts, not to a prompt.
        assert!(!prompt.contains("name: sam-authoring"));
        assert!(prompt.contains("Ingestion loop"));
        // Every taught command names the open folder as a shell word — the
        // plan may live anywhere, its path may contain spaces, and a quote in
        // a folder name must not close the argument.
        assert!(prompt.contains("--plan '/tmp/plan'"), "{prompt}");
        assert!(
            prompt.contains("sam apply <file> --dry-run --json --plan '/tmp/plan'"),
            "{prompt}"
        );
    }

    #[test]
    fn a_quote_or_a_substitution_in_the_folder_name_stays_one_word() {
        let hostile = Path::new("/tmp/week \"4\" $(touch /tmp/pwned) plan");
        let prompt = compose_prompt(hostile, None, None, "task");
        // The whole path is one single-quoted word: no `"` closes it and no
        // `$(…)` survives into the shell the agent pastes the command into.
        let quoted = shell_quote("/tmp/week \"4\" $(touch /tmp/pwned) plan");
        assert_eq!(quoted, "'/tmp/week \"4\" $(touch /tmp/pwned) plan'");
        assert!(prompt.contains(&format!("--plan {quoted}")), "{prompt}");
        // A path that carries a single quote closes and re-opens the word.
        assert_eq!(shell_quote("/tmp/o'brien"), "'/tmp/o'\\''brien'");
    }

    #[test]
    fn a_failed_version_probe_is_not_reported_as_a_version() {
        // OpenCode's `--version` failed on an unwritable log directory and
        // wrote the error to stderr; that sentence must not become the pill's
        // version. A stderr line is only read when it looks like a version.
        assert_eq!(version_line("", "1.4.0\n"), Some("1.4.0".into()));
        assert_eq!(
            version_line("", "PlatformError: Unknown: FileSystem.open (/x/log)\n"),
            None
        );
        // The same failure continues with a JS stack, and no word list names a
        // frame: the first `    at Pp (/$bunfs/…)` line used to become the pill.
        assert_eq!(
            version_line(
                "",
                "PlatformError: Unknown: FileSystem.open (/x/log)\n    at Pp (/$bunfs/root/chunk.js:2:956)\n"
            ),
            None
        );
        assert_eq!(version_line("2.0.27\n", "warning: x\n"), Some("2.0.27".into()));
        assert_eq!(version_line("", "no version here\n"), None);
    }

    #[test]
    fn the_search_space_reaches_a_gui_apps_missing_directories() {
        // The app's own PATH is launchd's four directories; the home bins and
        // the package-manager prefixes are what a login shell would add.
        let dirs = search_dirs();
        #[cfg(unix)]
        {
            assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
            assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
            if let Some(home) = home_dir() {
                assert!(dirs.contains(&home.join(".local/bin")));
            }
        }
        // An absolute bin is honoured without any search at all.
        let temp = std::env::temp_dir().join("sam-agent-abs-bin");
        std::fs::create_dir_all(&temp).unwrap();
        let bin = temp.join("probe-abs");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&bin).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&bin, permissions).unwrap();
        }
        assert_eq!(find_binary(bin.to_str().unwrap(), &[]), Some(bin));
    }

    #[test]
    fn a_primary_name_anywhere_beats_a_fallback_earlier_on_the_path() {
        // OpenCode's shape: a 1.x `opencode` in an early directory (Homebrew)
        // and the V2 `opencode2` in a later one. The fallback must not shadow
        // the binary the adapter's argv is written for.
        let root = std::env::temp_dir().join(format!("sam-agent-order-{}", crate::transaction::make_txid()));
        let early = root.join("early");
        let late = root.join("late");
        std::fs::create_dir_all(&early).unwrap();
        std::fs::create_dir_all(&late).unwrap();
        let make = |path: &Path| {
            std::fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = std::fs::metadata(path).unwrap().permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(path, permissions).unwrap();
            }
        };
        make(&early.join("opencode"));
        make(&late.join("opencode2"));
        assert_eq!(
            find_binary_in(&[early.clone(), late.clone()], &["opencode2", "opencode"]),
            Some(late.join("opencode2")),
            "the primary name wins even from a later directory"
        );
        // With no primary anywhere, the fallback in the early directory is the
        // substitute — that is what a fallback is for.
        assert_eq!(
            find_binary_in(&[early.clone(), late], &["not-a-binary", "opencode"]),
            Some(early.join("opencode"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_argv_delivered_prompt_is_bounded_before_it_is_spawned() {
        let short = vec!["-p".to_string(), "hello".to_string()];
        assert!(check_argv_budget(&short).is_ok());
        let huge = vec!["-p".to_string(), "x".repeat(argv_budget() + 10)];
        let refused = check_argv_budget(&huge).unwrap_err();
        assert!(refused.contains("over the"), "{refused}");
    }

    #[test]
    fn a_missing_sam_binary_is_named_not_promised() {
        let prompt = compose_prompt(Path::new("/tmp/plan"), None, None, "task");
        assert!(prompt.contains("`sam` was not found"), "{prompt}");
        assert!(
            !prompt.contains("on your PATH"),
            "a missing binary must never be promised: {prompt}"
        );
    }

    #[test]
    fn the_child_environment_names_the_open_plan_and_the_turn() {
        let env = child_env(
            Some(Path::new("/bundle/sam")),
            Path::new("/res"),
            Path::new("/plans/mine"),
            "run-7",
        );
        assert!(env
            .iter()
            .any(|(key, value)| key == resources::PLAN_ENV && value == "/plans/mine"));
        assert!(env
            .iter()
            .any(|(key, value)| key == resources::RESOURCES_ENV && value == "/res"));
        // The turn's identity rides the child, so the transaction it commits is
        // attributable to this run rather than guessed from a timestamp.
        assert!(env
            .iter()
            .any(|(key, value)| key == resources::RUN_ENV && value == "run-7"));
        assert!(env.iter().any(|(key, _)| key == "PATH"));
    }

    #[test]
    fn the_child_path_carries_the_directories_detection_read() {
        // A GUI app's PATH is launchd's; the child has to be able to *launch*
        // what detection found — a Homebrew CLI is a `#!/usr/bin/env node`
        // script, and `node` lives in the same directory.
        let path = path_for_children(Some(Path::new("/bundle/sam"))).expect("a path");
        let dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
        assert_eq!(dirs.first(), Some(&PathBuf::from("/bundle")));
        #[cfg(unix)]
        if Path::new("/opt/homebrew/bin").is_dir() {
            assert!(
                dirs.contains(&PathBuf::from("/opt/homebrew/bin")),
                "the directory a Homebrew CLI lives in is missing from the child's PATH"
            );
        }
    }

    #[test]
    fn a_terminal_run_says_it_has_no_window() {
        let prompt = compose_prompt(Path::new("/tmp/plan"), None, None, "task");
        assert!(prompt.contains("No window context"));
    }

    #[test]
    fn a_stray_byte_does_not_end_the_line_or_the_turn() {
        // `BufRead::lines()` gives up here; the turn's transcript must not.
        let raw = b"good\nbad \xff byte\nafter\nno newline at the end".to_vec();
        let mut reader = io::Cursor::new(raw);
        let mut lines: Vec<String> = Vec::new();
        while let Some((line, truncated)) = read_line_capped(&mut reader).unwrap() {
            assert!(!truncated);
            lines.push(line);
        }
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert_eq!(lines[0], "good");
        assert!(lines[1].contains('\u{fffd}'), "{lines:?}");
        assert_eq!(lines[2], "after");
        assert_eq!(lines[3], "no newline at the end");
    }

    #[test]
    fn a_frame_longer_than_the_cap_is_truncated_not_unbounded() {
        let long = format!("{}\nnext\n", "x".repeat(64));
        let mut reader = io::Cursor::new(long.into_bytes());
        let (line, truncated) = read_line_capped_with(&mut reader, 16).unwrap().unwrap();
        assert!(truncated);
        assert_eq!(line, "x".repeat(16));
        // The remainder of the giant line is drained, so the next line is whole.
        let (next, truncated) = read_line_capped_with(&mut reader, 16).unwrap().unwrap();
        assert!(!truncated);
        assert_eq!(next, "next");
    }

    #[test]
    fn a_model_choice_must_look_like_an_id_not_a_flag() {
        assert_eq!(sanitize_choice("sonnet"), Some("sonnet".into()));
        assert_eq!(
            sanitize_choice("anthropic/claude-sonnet-4-5"),
            Some("anthropic/claude-sonnet-4-5".into())
        );
        assert_eq!(sanitize_choice("--dangerously-skip-permissions"), None);
        assert_eq!(sanitize_choice("-m"), None);
        assert_eq!(sanitize_choice("two words"), None);
        assert_eq!(sanitize_choice("  "), None);
        assert_eq!(sanitize_choice(&"x".repeat(201)), None);
    }

    #[test]
    fn model_list_shapes_are_read_from_their_real_output() {
        let opencode = "cline-pass/_anthropic/claude-fable-latest\nanthropic/claude-sonnet-4-5\n";
        let models = parse_models("opencode", opencode);
        assert_eq!(models.len(), 2);
        assert_eq!(models[1].id, "anthropic/claude-sonnet-4-5");

        let agy = "Fetching available models...\ngemini-3.8-flash-high\tGemini 3.8 Flash (High)\n";
        let models = parse_models("antigravity", agy);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].label, "Gemini 3.8 Flash (High)");

        let pi = "provider               model\nanthropic              claude-fable-5\nopenai   gpt-5\n";
        let models = parse_models("pi", pi);
        assert_eq!(models[0].id, "anthropic/claude-fable-5");
        assert_eq!(models[1].id, "openai/gpt-5");

        let omp = "aerolink (1)\n│ model │ context │\n├───────┼─────────┤\n│ claude-opus-4-8 │ 1M │\n";
        let models = parse_models("omp", omp);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-opus-4-8");
        assert!(models[0].label.contains("aerolink"));
    }

    #[test]
    fn frontmatter_stripping_is_safe() {
        assert_eq!(strip_frontmatter("no frontmatter"), "no frontmatter");
        assert_eq!(strip_frontmatter("---\nname: x\n---\nbody"), "body");
    }

    #[test]
    fn picking_an_unavailable_agent_is_an_error_not_a_fallback() {
        let statuses = vec![AgentStatus {
            id: "ghost".into(),
            name: "Ghost".into(),
            bin: "ghost".into(),
            path: None,
            version: None,
            available: false,
            install_hint: String::new(),
        }];
        assert!(pick_status(&statuses, Some("ghost")).is_err());
        assert!(pick_status(&statuses, None).is_err());
    }

    #[test]
    fn empty_path_detects_nothing_and_does_not_fail() {
        // `find_binary` reads PATH; a deliberately empty search space is the
        // "machine with no CLIs" case the deck renders as a blocker.
        let dir = std::env::temp_dir().join("sam-agent-detect-test");
        let _ = std::fs::create_dir_all(&dir);
        assert!(find_binary("definitely-not-a-real-binary-xyz", &[]).is_none());
    }

    #[test]
    fn detection_answers_from_the_cache_until_the_fingerprint_changes() {
        let profile = |bin: &str| CustomProfile {
            id: "probe".into(),
            name: None,
            bin: bin.into(),
            args: vec![],
            stream_format: None,
            models: vec![],
        };
        let profiles = vec![profile("sam-not-a-real-binary-xyz")];
        let first = detect_cached(&profiles);
        let second = detect_cached(&profiles);
        assert_eq!(first.len(), second.len());
        assert!(second
            .iter()
            .filter(|status| status.id == "probe")
            .all(|status| !status.available));
        // A different profile set is a different cache key and still answers.
        let third = detect_cached(&[profile("sam-also-not-real-abc")]);
        assert!(!third.iter().any(|status| status.id == "probe" && status.available));
    }

    #[test]
    fn a_cheap_resolution_carries_no_version() {
        // `resolve_status` must not spawn anything: on this machine every
        // answer has `version: None` — that is the whole point of it.
        let statuses = resolve_status(&[], None);
        assert!(statuses
            .iter()
            .all(|status| status.version.is_none()));
    }
}
