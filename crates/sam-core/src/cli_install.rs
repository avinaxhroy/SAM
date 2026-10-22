//! Putting the bundled `sam` onto the user's `PATH` (§4.10's machine seam).
//!
//! The CLI ships inside the app bundle (`Contents/MacOS/sam` on macOS, beside
//! `sam-shell.exe` on Windows, the app's resource `bin/` on Linux) and nothing
//! links it — so a terminal, or an independent agent like opencode, cannot
//! find `sam` at all. This module plans and applies that link: a symlink in a
//! directory already on `PATH` on Unix, an append of the app's own directory
//! to the *user* `PATH` on Windows. Framework-free like the rest of
//! `sam-core` — `std::fs`, `std::process`, `std::env` only.
//!
//! Elevation is planned, never guessed: when no writable directory on `PATH`
//! exists, the plan is a shell command for the caller to run with the
//! platform's own prompt (the desktop's `osascript` ask on macOS, `pkexec` on
//! Linux) — a headless caller prints the command instead of prompting (§4.9:
//! nothing prompts). Foreign files are never overwritten.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// What sits at one candidate directory's `sam` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Missing,
    /// A symlink to the binary we are installing.
    Ours,
    /// A symlink whose target does not exist (the app moved or was deleted).
    Stale,
    /// A symlink to some other existing binary — never touched.
    Foreign(PathBuf),
    /// A real file or directory named `sam` — never touched.
    ForeignFile,
}

/// The install state, as the shell and the CLI report it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Where `sam` is reachable, when a candidate holds an entry.
    pub path: Option<String>,
    /// What that entry points at, when it is a symlink.
    pub target: Option<String>,
    /// True when the entry points at a binary that no longer exists.
    pub stale: bool,
    /// True when the directory holding it is on `PATH` (Windows: the app's own
    /// directory).
    pub on_path: bool,
}

/// One install plan: the source binary plus what to do with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub source: PathBuf,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Unix: create/refresh a symlink at `link`.
    Link { link: PathBuf },
    /// Windows: append `dir` (the app's own directory) to the user `PATH`.
    AppendPath { dir: PathBuf },
    /// The link needs rights this process does not have: the shell runs
    /// `command` with the platform's own prompt; a headless caller prints it.
    NeedsElevation { link: PathBuf, command: String },
    /// Nothing to do — the entry already points at this binary.
    Already { path: PathBuf },
}

/// What an install/uninstall did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    /// "installed" · "refreshed" · "already" · "removed" · "path-appended".
    pub action: String,
    pub path: String,
    pub target: Option<String>,
}

impl Receipt {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "action": self.action,
            "path": self.path,
            "target": self.target,
        })
    }
}

/// The outcome of an install attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done(Receipt),
    NeedsElevation { link: PathBuf, command: String },
}

impl Outcome {
    /// The engine's JSON shape, shared by the CLI door and the shell command.
    pub fn json(&self) -> serde_json::Value {
        match self {
            Outcome::Done(receipt) => serde_json::json!({
                "installed": true,
                "needsElevation": false,
                "action": receipt.action,
                "path": receipt.path,
                "target": receipt.target,
            }),
            Outcome::NeedsElevation { link, command } => serde_json::json!({
                "installed": false,
                "needsElevation": true,
                "path": link.display().to_string(),
                "command": command,
            }),
        }
    }
}

// ── where a link can live ────────────────────────────────────────────────────

/// The directories that make a `sam` reachable for a Unix login shell, in the
/// order a plan prefers them.
fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/local/bin")];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".cargo/bin"));
    }
    dirs
}

/// `sam` on Unix has no extension; Windows resolves `sam.exe`.
fn link_name() -> &'static str {
    if cfg!(windows) {
        "sam.exe"
    } else {
        "sam"
    }
}

/// The bundled binary beside an executable directory — what an install links.
/// `None` means this build did not bundle one (a dev build without the
/// sidecar): nothing is guessed from `PATH`, because the link must not point
/// at a checkout copy that vanishes with the next `cargo clean`.
pub fn bundled_sam(exe_dir: Option<&Path>) -> Option<PathBuf> {
    let candidate = exe_dir?.join(link_name());
    crate::agent::is_executable(&candidate).then_some(candidate)
}

/// What sits at `dir/sam` relative to `source`.
pub fn entry_at(dir: &Path, source: &Path) -> Entry {
    let path = dir.join(link_name());
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return Entry::Missing;
    };
    if meta.file_type().is_symlink() {
        let Ok(target) = std::fs::read_link(&path) else {
            return Entry::ForeignFile;
        };
        if target == source {
            return Entry::Ours;
        }
        if target.exists() {
            return Entry::Foreign(target);
        }
        return Entry::Stale;
    }
    Entry::ForeignFile
}

/// The status as the shell and the CLI report it: the first candidate entry,
/// whether it is stale, and whether its directory is on `PATH`.
pub fn status(source: &Path) -> Status {
    for dir in candidate_dirs() {
        match entry_at(&dir, source) {
            Entry::Missing => continue,
            Entry::Ours => {
                return Status {
                    path: Some(dir.join(link_name()).display().to_string()),
                    target: Some(source.display().to_string()),
                    stale: false,
                    on_path: is_on_path(&dir),
                };
            }
            Entry::Stale => {
                return Status {
                    path: Some(dir.join(link_name()).display().to_string()),
                    target: entry_target(&dir).map(|t| t.display().to_string()),
                    stale: true,
                    on_path: is_on_path(&dir),
                };
            }
            Entry::Foreign(target) => {
                return Status {
                    path: Some(dir.join(link_name()).display().to_string()),
                    target: Some(target.display().to_string()),
                    stale: false,
                    on_path: is_on_path(&dir),
                };
            }
            Entry::ForeignFile => {
                return Status {
                    path: Some(dir.join(link_name()).display().to_string()),
                    target: None,
                    stale: false,
                    on_path: is_on_path(&dir),
                };
            }
        }
    }
    Status {
        path: None,
        target: None,
        stale: false,
        on_path: false,
    }
}

fn entry_target(dir: &Path) -> Option<PathBuf> {
    std::fs::read_link(dir.join(link_name())).ok()
}

/// Whether `dir` is a `PATH` entry of this process (the environment the user
/// actually launches shells with; a link elsewhere is invisible to them).
pub fn is_on_path(dir: &Path) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|entry| entry == dir)
}

/// Can this process create a file in `dir`? Probed by writing a uniquely named
/// dotfile and removing it — the only portable answer without libc.
fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".sam-write-probe-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// The elevated command that makes the link, for a caller with a prompt.
fn elevation_command(link: &Path, source: &Path) -> String {
    let parent = link.parent().unwrap_or(Path::new("/usr/local/bin"));
    format!(
        "mkdir -p {} && ln -sf {} {}",
        sh_quote(parent),
        sh_quote(source),
        sh_quote(link)
    )
}

fn sh_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

// ── planning ─────────────────────────────────────────────────────────────────

/// Plan an install for this machine. `source` is the bundled binary
/// (`bundled_sam`); the plan never points a link anywhere else.
pub fn plan(source: &Path) -> Plan {
    if cfg!(windows) {
        return plan_windows(source);
    }
    plan_unix(source, &candidate_dirs())
}

fn plan_windows(source: &Path) -> Plan {
    let dir = source.parent().unwrap_or(Path::new(".")).to_path_buf();
    if is_on_path(&dir) {
        return Plan {
            source: source.to_path_buf(),
            action: Action::Already { path: dir },
        };
    }
    Plan {
        source: source.to_path_buf(),
        action: Action::AppendPath { dir },
    }
}

fn plan_unix(source: &Path, dirs: &[PathBuf]) -> Plan {
    // Refresh an existing entry in place, wherever it is.
    for dir in dirs {
        match entry_at(dir, source) {
            Entry::Ours => {
                return Plan {
                    source: source.to_path_buf(),
                    action: Action::Already {
                        path: dir.join(link_name()),
                    },
                };
            }
            Entry::Stale => {
                return Plan {
                    source: source.to_path_buf(),
                    action: Action::Link {
                        link: dir.join(link_name()),
                    },
                };
            }
            Entry::Foreign(_) | Entry::ForeignFile => {
                // A different `sam` owns this directory: do not fight it, and
                // do not silently shadow it — fall through to the next
                // candidate so the plan names a link we can make.
            }
            Entry::Missing => {}
        }
    }

    // The nicest link: a directory a login shell already looks in. A missing
    // `~/.local/bin` that is on PATH is created rather than skipped.
    for dir in dirs {
        if !is_on_path(dir) {
            continue;
        }
        if !dir.exists() {
            if std::fs::create_dir_all(dir).is_err() {
                continue;
            }
        }
        if writable(dir) {
            return Plan {
                source: source.to_path_buf(),
                action: Action::Link {
                    link: dir.join(link_name()),
                },
            };
        }
    }

    // Nothing writable on PATH: plan the elevated command for the first
    // candidate that exists (a prompt the shell owns), else the canonical
    // `/usr/local/bin` with its own `mkdir -p`.
    let link_dir = dirs
        .iter()
        .find(|dir| dir.exists())
        .cloned()
        .unwrap_or_else(|| PathBuf::from("/usr/local/bin"));
    let link = link_dir.join(link_name());
    Plan {
        source: source.to_path_buf(),
        action: Action::NeedsElevation {
            command: elevation_command(&link, source),
            link,
        },
    }
}

// ── applying ─────────────────────────────────────────────────────────────────

/// Apply a plan this process can carry out. `NeedsElevation` is refused here —
/// the caller owns the prompt and calls the command it names.
pub fn apply(plan: &Plan) -> Result<Receipt, String> {
    match &plan.action {
        Action::Already { path } => Ok(Receipt {
            action: "already".into(),
            path: path.display().to_string(),
            target: Some(plan.source.display().to_string()),
        }),
        Action::Link { link } => {
            let parent = link.parent().unwrap_or(Path::new("."));
            match entry_at(parent, &plan.source) {
                Entry::ForeignFile => Err(format!(
                    "{} already exists and is not a link this app made — move it aside and retry",
                    link.display()
                )),
                Entry::Foreign(target) => Err(format!(
                    "{} points at {} (not this app) — remove it first if you want SAM's `sam` there",
                    link.display(),
                    target.display()
                )),
                Entry::Ours => Ok(Receipt {
                    action: "already".into(),
                    path: link.display().to_string(),
                    target: Some(plan.source.display().to_string()),
                }),
                stale @ (Entry::Missing | Entry::Stale) => {
                    if stale == Entry::Stale {
                        let _ = std::fs::remove_file(link);
                    }
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&plan.source, link).map_err(|error| {
                        format!("cannot link {}: {error}", link.display())
                    })?;
                    #[cfg(not(unix))]
                    {
                        std::fs::copy(&plan.source, link).map_err(|error| {
                            format!("cannot copy {}: {error}", link.display())
                        })?;
                    }
                    Ok(Receipt {
                        action: if stale == Entry::Stale {
                            "refreshed".into()
                        } else {
                            "installed".into()
                        },
                        path: link.display().to_string(),
                        target: Some(plan.source.display().to_string()),
                    })
                }
            }
        }
        Action::AppendPath { dir } => append_user_path(dir),
        Action::NeedsElevation { command, .. } => Err(format!(
            "needs elevation — run: {command}"
        )),
    }
}

/// Windows: append the app's own directory to the *user* `PATH` (no copy —
/// `sam.exe` stays correct across app updates). PowerShell owns the registry
/// edit; `setx` truncates long `PATH` values and is never used.
fn append_user_path(dir: &Path) -> Result<Receipt, String> {
    let dir_text = dir.display().to_string();
    let script = format!(
        "$p=[Environment]::GetEnvironmentVariable('Path','User'); \
         if (($p -split ';') -notcontains '{}') {{ \
           [Environment]::SetEnvironmentVariable('Path', ('{};' + $p), 'User') \
         }}",
        dir_text.replace('\'', "''"),
        dir_text.replace('\'', "''")
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|error| format!("cannot run PowerShell: {error}"))?;
    if !status.success() {
        return Err("PowerShell refused the PATH edit".into());
    }
    Ok(Receipt {
        action: "path-appended".into(),
        path: dir_text,
        target: None,
    })
}

/// One call the shells can share: plan, then apply only what needs no prompt.
pub fn install(source: &Path) -> Result<Outcome, String> {
    if !crate::agent::is_executable(source) {
        return Err(format!(
            "{} is not the bundled `sam` binary",
            source.display()
        ));
    }
    let plan = plan(source);
    match &plan.action {
        Action::NeedsElevation { link, command } => Ok(Outcome::NeedsElevation {
            link: link.clone(),
            command: command.clone(),
        }),
        _ => apply(&plan).map(Outcome::Done),
    }
}

/// The `app.installCli` door as JSON: resolve the bundled binary beside
/// `exe_dir`, install what needs no prompt, and answer with the receipt —
/// including `needsElevation` plus the command when a prompt is required
/// (headless callers print it; the desktop shows the platform's own ask).
pub fn install_json(exe_dir: Option<&Path>) -> Result<serde_json::Value, String> {
    let source = bundled_sam(exe_dir).ok_or_else(|| {
        "this build carries no bundled `sam` beside its executable — run the packaged app"
            .to_string()
    })?;
    let outcome = install(&source)?;
    let mut value = outcome.json();
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "source".into(),
            serde_json::json!(source.display().to_string()),
        );
        object.insert(
            "status".into(),
            serde_json::to_value(status(&source)).map_err(|error| error.to_string())?,
        );
    }
    Ok(value)
}

/// Remove a link in `dir` only when it is one this app's installs could have
/// made (ours, or stale because the app moved).
fn remove_entry(dir: &Path, source: &Path) -> Result<Receipt, String> {
    match entry_at(dir, source) {
        Entry::Ours | Entry::Stale => {
            let link = dir.join(link_name());
            std::fs::remove_file(&link)
                .map_err(|error| format!("cannot remove {}: {error}", link.display()))?;
            Ok(Receipt {
                action: "removed".into(),
                path: link.display().to_string(),
                target: None,
            })
        }
        _ => Err(format!(
            "{} is not a link this app made — nothing removed",
            dir.join(link_name()).display()
        )),
    }
}

/// Remove only a link this app's own installs could have made.
pub fn uninstall(source: &Path) -> Result<Receipt, String> {
    if cfg!(windows) {
        return Err(
            "Windows installs by PATH; remove the app's folder from your user PATH in System settings"
                .into(),
        );
    }
    let mut last = String::new();
    for dir in candidate_dirs() {
        match remove_entry(&dir, source) {
            Ok(receipt) => return Ok(receipt),
            Err(message) => last = message,
        }
    }
    Err(if last.is_empty() {
        "no `sam` link this app made was found — nothing to remove".into()
    } else {
        last
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sam-install-{name}-{}",
            crate::transaction::make_txid()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn fake_sam(dir: &Path) -> PathBuf {
        let binary = dir.join("sam");
        std::fs::write(&binary, b"#!/bin/sh\nexit 0\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        binary
    }

    #[test]
    fn a_missing_entry_plans_a_link_and_apply_makes_it() {
        let root = temp("missing");
        let bin = root.join("bundle");
        std::fs::create_dir_all(&bin).expect("bundle dir");
        let source = fake_sam(&bin);
        let target_dir = root.join("bin");
        std::fs::create_dir_all(&target_dir).expect("target dir");

        // A directory that is neither on PATH nor writable-by-plan ends in a
        // plan (elevated or not); what matters here is the link itself.
        let plan = plan_unix(&source, std::slice::from_ref(&target_dir));
        assert!(matches!(
            plan.action,
            Action::NeedsElevation { .. } | Action::Link { .. }
        ));

        let link_plan = Plan {
            source: source.clone(),
            action: Action::Link {
                link: target_dir.join("sam"),
            },
        };
        let receipt = apply(&link_plan).expect("the link lands");
        assert_eq!(receipt.action, "installed");
        assert_eq!(entry_at(&target_dir, &source), Entry::Ours);
        // Applying again is a no-op, not an error.
        let receipt = apply(&link_plan).expect("idempotent");
        assert_eq!(receipt.action, "already");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_stale_link_is_refreshed_and_a_foreign_file_is_refused() {
        let root = temp("stale");
        let bin = root.join("bundle");
        std::fs::create_dir_all(&bin).expect("bundle dir");
        let source = fake_sam(&bin);
        let target_dir = root.join("bin");
        std::fs::create_dir_all(&target_dir).expect("target dir");

        // A dangling link (the app moved) is ours to refresh.
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("gone/sam"), target_dir.join("sam"))
            .expect("stale link");
        assert_eq!(entry_at(&target_dir, &source), Entry::Stale);
        let plan = Plan {
            source: source.clone(),
            action: Action::Link {
                link: target_dir.join("sam"),
            },
        };
        let receipt = apply(&plan).expect("refresh");
        assert_eq!(receipt.action, "refreshed");
        assert_eq!(entry_at(&target_dir, &source), Entry::Ours);

        // A real file named `sam` is never overwritten.
        std::fs::remove_file(target_dir.join("sam")).expect("remove");
        std::fs::write(target_dir.join("sam"), b"not ours").expect("foreign file");
        assert_eq!(entry_at(&target_dir, &source), Entry::ForeignFile);
        let error = apply(&plan).expect_err("refused");
        assert!(error.contains("not a link this app made"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn a_foreign_link_to_an_existing_binary_is_refused() {
        let root = temp("foreign");
        let bin = root.join("bundle");
        std::fs::create_dir_all(&bin).expect("bundle dir");
        let source = fake_sam(&bin);
        let other_dir = root.join("other");
        std::fs::create_dir_all(&other_dir).expect("other dir");
        let other = fake_sam(&other_dir);
        let target_dir = root.join("bin");
        std::fs::create_dir_all(&target_dir).expect("target dir");
        std::os::unix::fs::symlink(&other, target_dir.join("sam")).expect("foreign link");

        let plan = Plan {
            source: source.clone(),
            action: Action::Link {
                link: target_dir.join("sam"),
            },
        };
        let error = apply(&plan).expect_err("refused");
        assert!(error.contains("points at"), "{error}");
        assert_eq!(entry_at(&target_dir, &source), Entry::Foreign(other));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn removal_takes_only_our_link() {
        let root = temp("uninstall");
        let bin = root.join("bundle");
        std::fs::create_dir_all(&bin).expect("bundle dir");
        let source = fake_sam(&bin);
        let target_dir = root.join("bin");
        std::fs::create_dir_all(&target_dir).expect("target dir");

        // Nothing there: refused.
        assert!(remove_entry(&target_dir, &source).is_err());
        // A foreign file: refused, untouched.
        std::fs::write(target_dir.join("sam"), b"not ours").expect("foreign");
        assert!(remove_entry(&target_dir, &source).is_err());
        assert!(target_dir.join("sam").exists());
        // Our link: removed. A stale one (app moved): also removed.
        std::fs::remove_file(target_dir.join("sam")).expect("remove");
        apply(&Plan {
            source: source.clone(),
            action: Action::Link {
                link: target_dir.join("sam"),
            },
        })
        .expect("installed");
        let receipt = remove_entry(&target_dir, &source).expect("removed");
        assert_eq!(receipt.action, "removed");
        assert!(!target_dir.join("sam").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_elevated_command_quotes_the_paths() {
        let command = elevation_command(
            Path::new("/usr/local/bin/sam"),
            Path::new("/Applications/SAM's app.app/Contents/MacOS/sam"),
        );
        assert!(command.starts_with("mkdir -p '/usr/local/bin'"));
        assert!(command.contains("ln -sf"), "{command}");
        assert!(command.contains("'\\''"), "a quote in the path is escaped: {command}");
    }
}
