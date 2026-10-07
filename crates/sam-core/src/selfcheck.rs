//! Shipped selfcheck gates (§6: `sam --selfcheck`, D22).
//!
//! Preserves check names as part of the contract across Phase 1 and Phase 2.
//! Write gates verify process-level behavior (lock contention, crash recovery at journal boundaries,
//! read-only filesystem handling, external modifications) by executing the CLI binary
//! and validating exit codes (§9 rule 1).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};

use crate::apply;
use crate::command_registry;
use crate::config_store::{self, ConfigStoreError};
use crate::config_watcher;
use crate::evaluator::{EvalContext, Evaluation, L2Value};
use crate::expression::{self, MAX_EVALUATION_STEPS};
use crate::model::Record;
use crate::plan_lock::PlanLock;
use crate::resolved_config::ResolvedConfig;
use crate::resources;
use crate::transaction::{self, CrashPoint, TransactionError};
use crate::views::{self, PlanState};

/// One shipped gate.
pub struct CheckResult {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

fn check(name: &str, ok: bool, detail: impl Into<String>) -> CheckResult {
    CheckResult {
        name: name.into(),
        ok,
        detail: detail.into(),
    }
}

// ── the harness ─────────────────────────────────────────────────────────────

struct ProcOutcome {
    status: i32,
    out: String,
    err: String,
    signaled: bool,
}

struct Harness {
    binary: PathBuf,
    resources: PathBuf,
}

#[cfg(unix)]
fn status_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => code,
        None => status.signal().unwrap_or(-1),
    }
}

#[cfg(not(unix))]
fn status_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

impl Harness {
    fn new(binary: &Path, resources: &Path) -> Self {
        Self {
            binary: binary.to_path_buf(),
            resources: resources.to_path_buf(),
        }
    }

    fn command(&self, args: &[String], env: &[(&str, &str)], with_stdin: bool) -> Child {
        let mut command = Command::new(&self.binary);
        command
            .args(args)
            .env(resources::RESOURCES_ENV, &self.resources)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(if with_stdin {
                Stdio::piped()
            } else {
                Stdio::null()
            });
        for (key, value) in env {
            command.env(key, value);
        }
        command
            .spawn()
            .expect("the built binary spawns (--selfcheck runs from the CLI it ships)")
    }

    fn finish(child: Child) -> ProcOutcome {
        let output = child.wait_with_output().expect("the child is waited for");
        ProcOutcome {
            status: status_code(output.status),
            out: String::from_utf8_lossy(&output.stdout).to_string(),
            err: String::from_utf8_lossy(&output.stderr).to_string(),
            signaled: output.status.code().is_none(),
        }
    }

    fn run(&self, args: &[&str], env: &[(&str, &str)], stdin: Option<&[u8]>) -> ProcOutcome {
        let owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        let mut child = self.command(&owned, env, stdin.is_some());
        if let Some(data) = stdin {
            if let Some(mut pipe) = child.stdin.take() {
                let _ = pipe.write_all(data);
            }
        }
        Self::finish(child)
    }

    fn spawn(&self, args: &[&str], env: &[(&str, &str)]) -> Child {
        let owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        self.command(&owned, env, false)
    }

    fn revision_of(&self, plan: &Path) -> String {
        let outcome = self.run(
            &["paths", "--plan", &plan.to_string_lossy(), "--json"],
            &[],
            None,
        );
        envelope(&outcome.out)
            .and_then(|value| {
                value
                    .get("revision")
                    .and_then(|r| r.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default()
    }
}

fn envelope(text: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(text).ok()
}

/// A bounded prefix for a detail string. Byte slicing panics inside a
/// multi-byte character ("→" in a diagnostic), so this counts characters.
fn prefix(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    text.chars().take(max).collect()
}

/// A name that is unique across every test thread in this process and across
/// processes. `make_txid` serialises a process-wide counter; `SystemTime`
/// alone collides at the granularity this machine reports.
fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", crate::transaction::make_txid())
}

/// A writable copy of the seed preset — never the resource tree, which is input.
fn temp_plan(resources: &Path) -> Result<PathBuf, String> {
    let seed = resources.join("presets/seed");
    let temp = std::env::temp_dir().join(unique("sam-p2"));
    transaction::copy_tree(&seed, &temp)
        .map_err(|error| format!("cannot stage the seed: {error}"))?;
    Ok(temp)
}

fn topic_line(id: &str, title: &str) -> String {
    format!(
        "{{\"fields\":{{\"est\":10,\"kind\":\"watch\",\"title\":\"{title}\"}},\"id\":\"{id}\",\"links\":{{}},\"schemaVersion\":1,\"type\":\"topic\"}}"
    )
}

fn resource_line(id: &str) -> String {
    format!(
        "{{\"fields\":{{\"kind\":\"pdf\",\"label\":\"{id}\",\"url\":\"https://x.example/r\"}},\"id\":\"{id}\",\"links\":{{}},\"schemaVersion\":1,\"type\":\"resource\"}}"
    )
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// All authoritative bytes under `content/` and `state/` (bookkeeping excluded).
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for top in ["content", "state"] {
        collect(&root.join(top), root, &mut out);
    }
    out
}

fn collect(path: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let child = entry.path();
        if child.is_dir() {
            collect(&child, root, out);
        } else if let Ok(rel) = child.strip_prefix(root) {
            if let Ok(bytes) = std::fs::read(&child) {
                out.insert(rel.to_string_lossy().replace('\\', "/"), bytes);
            }
        }
    }
}

fn journal_count(plan: &Path) -> usize {
    std::fs::read_dir(plan.join(".sam/tx/journal"))
        .map(|entries| entries.filter_map(|entry| entry.ok()).count())
        .unwrap_or(0)
}

fn read_to_string(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

// ── the check list ──────────────────────────────────────────────────────────

/// Runs Phase 1 read checks and Phase 2 safe-write checks.
/// `binary` is the CLI executable spawned to exercise real process isolation and exit codes.
pub fn run(binary: &Path) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let resources_dir = match resources::paths() {
        Ok(paths) => {
            results.push(check(
                "resources-resolve",
                true,
                format!("resources {}", paths.resources_dir.display()),
            ));
            paths.resources_dir
        }
        Err(error) => {
            results.push(check("resources-resolve", false, error));
            return results;
        }
    };
    let harness = Harness::new(binary, &resources_dir);

    results.extend(token_checks(&resources_dir));
    results.push(seed_check(&resources_dir));
    results.push(seed_tree_check(&resources_dir));
    results.extend(problemset_checks(&resources_dir));
    results.push(invalid_fixtures_check(&resources_dir));
    results.push(crlf_check(&resources_dir));
    results.push(watcher_check(&resources_dir));
    results.extend(write_checks(&harness, &resources_dir));
    results.extend(phase_three_checks(&harness, &resources_dir));
    results.extend(phase_five_checks(&harness, &resources_dir));
    results.extend(phase_six_checks(&harness, &resources_dir));
    results.extend(phase_seven_checks(&harness, &resources_dir));
    results.push(machine_delete_check(&harness, &resources_dir));
    results.push(composer_check(&harness, &resources_dir));
    results
}

/// COMPOSER §2.3: the composer's write path, through the shipped binary. A
/// screen is created with no type, its component list is written, reordered and
/// cleared, and the plan is read back after each write — every read is a fresh
/// process, so each one is a reload round-trip.
///
/// The clear on the student's own screen is **refused**, deliberately: that
/// view declares no type, so removing its composition would leave a view that
/// draws nothing and the validator says so (`view.empty`). `--clear` is *Use
/// SAM's design again*, which only a screen with a designed fallback can
/// answer — so the check proves the refusal left the file untouched, then
/// clears a screen that does have one.
fn composer_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    /// The surfaces the `view` read projects, or `None` when the view declares
    /// no composition at all (which is what a clear leaves behind).
    fn surfaces(out: &str) -> Option<Vec<String>> {
        components(out).map(|components| {
            components
                .iter()
                .filter_map(|component| {
                    component
                        .get("surface")
                        .and_then(|surface| surface.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
    }

    /// Each component's own `span` member as the plan states it: `Some(1)` when
    /// the half-row is written, `None` when the member is absent — the whole row
    /// is the default and is never written, so `None` here is what "full width"
    /// looks like in the file.
    fn spans(out: &str) -> Option<Vec<Option<i64>>> {
        components(out).map(|components| {
            components
                .iter()
                .map(|component| component.get("span").and_then(|span| span.as_i64()))
                .collect()
        })
    }

    /// The composition the `view` read projects.
    fn components(out: &str) -> Option<Vec<serde_json::Value>> {
        envelope(out)?
            .pointer("/data/components")
            .map(|components| components.as_array().cloned().unwrap_or_default())
    }

    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("composer", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();
    let screen = "focus.all";
    // A screen with no type: an empty composition plus the rail entry, icon and
    // all, in one transaction.
    let created = harness.run(
        &[
            "list.new", "--name", "Focus", "--icon", "target", "--plan", &plan_arg, "--json",
        ],
        &[],
        None,
    );
    // Read it back as created, then write the list, read it, reorder it, read it.
    let fresh = harness.run(&["view", screen, "--plan", &plan_arg, "--json"], &[], None);
    let added = harness.run(
        &[
            "view.setComponents",
            screen,
            "--surfaces",
            "week-chart,today-focus",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let before = harness.run(&["view", screen, "--plan", &plan_arg, "--json"], &[], None);
    let reordered = harness.run(
        &[
            "view.setComponents",
            screen,
            "--surfaces",
            "today-focus,week-chart",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let between = harness.run(&["view", screen, "--plan", &plan_arg, "--json"], &[], None);
    // A width write (§2.1): the same list, each component's own span. 1 is
    // written, the default 2 is not, and the read-back is the members the plan
    // holds — so the screen the UI sizes its frames by is the file's own.
    let sized = harness.run(
        &[
            "view.setComponents",
            screen,
            "--surfaces",
            "today-focus,week-chart",
            "--spans",
            "1,2",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let sized_back = harness.run(&["view", screen, "--plan", &plan_arg, "--json"], &[], None);
    // A width that is not 1 or 2, and a list that does not line up with the
    // surfaces, are both refused before anything is written.
    let wide = harness.run(
        &[
            "view.setComponents",
            screen,
            "--surfaces",
            "today-focus,week-chart",
            "--spans",
            "1,3",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let short = harness.run(
        &[
            "view.setComponents",
            screen,
            "--surfaces",
            "today-focus,week-chart",
            "--spans",
            "1",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let refused_spans = harness.run(&["view", screen, "--plan", &plan_arg, "--json"], &[], None);
    // Clearing the student's own screen is the one write the plan refuses: it
    // would leave a view that draws nothing.
    let refused = harness.run(
        &[
            "view.setComponents",
            screen,
            "--clear",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let after = harness.run(&["view", screen, "--plan", &plan_arg, "--json"], &[], None);
    // A screen with a designed fallback clears cleanly — its panel and blocks
    // are what it draws again.
    let designed_set = harness.run(
        &[
            "view.setComponents",
            "today.screen",
            "--surfaces",
            "week-chart",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let designed_clear = harness.run(
        &[
            "view.setComponents",
            "today.screen",
            "--clear",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let designed_back = harness.run(
        &["view", "today.screen", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let check_run = harness.run(&["--configcheck", "--plan", &plan_arg, "--json"], &[], None);

    // The rail entry, read from the plan's own bytes before the temp directory
    // goes away.
    let shell: serde_json::Value =
        serde_json::from_str(&read_to_string(&plan.join("content/shell.json")))
            .unwrap_or(serde_json::Value::Null);
    let _ = std::fs::remove_dir_all(&plan);

    let created_empty = surfaces(&fresh.out) == Some(Vec::new());
    let entry = shell
        .get("navigation")
        .and_then(|navigation| navigation.as_array())
        .and_then(|entries| entries.last())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let entry_ok = entry.get("view").and_then(|view| view.as_str()) == Some(screen)
        && entry.get("title").and_then(|title| title.as_str()) == Some("Focus")
        && entry.get("icon").and_then(|icon| icon.as_str()) == Some("target");

    let wrote = surfaces(&before.out) == Some(vec!["week-chart".into(), "today-focus".into()]);
    let reorder_ok =
        surfaces(&between.out) == Some(vec!["today-focus".into(), "week-chart".into()]);
    // The width write: half the row is written, the default whole row is not —
    // the read-back is the members the plan holds, and the payload agrees.
    let sized_ok = sized.status == 0
        && spans(&sized_back.out) == Some(vec![Some(1), None])
        && envelope(&sized.out).and_then(|value| value.pointer("/data/spans").cloned())
            == Some(serde_json::json!([1, 2]));
    // A width that is not 1 or 2, and a list that does not line up with the
    // surfaces, are usage errors — exit 2, and the file is left exactly as it
    // was either way.
    let spans_refused_ok = wide.status == 2
        && wide.err.contains("cli.usage")
        && short.status == 2
        && short.err.contains("cli.usage")
        && spans(&refused_spans.out) == Some(vec![Some(1), None]);
    // The refusal is the validator's own finding, and the file is untouched.
    let refused_ok = refused.status == 1
        && refused.err.contains("view.empty")
        && surfaces(&after.out) == Some(vec!["today-focus".into(), "week-chart".into()]);
    let cleared_ok = designed_set.status == 0
        && designed_clear.status == 0
        && surfaces(&designed_back.out).is_none();

    let ok = created.status == 0
        && created_empty
        && entry_ok
        && added.status == 0
        && wrote
        && reordered.status == 0
        && reorder_ok
        && sized_ok
        && spans_refused_ok
        && refused_ok
        && cleared_ok
        && check_run.status == 0
        && check_run.out.contains("\"ok\":true");
    check(
        "composer",
        ok,
        format!(
            "list.new (no type) → {} · components: [] {created_empty} + icon entry {entry_ok} · setComponents add/reorder → {}/{} · the view reads back written {wrote} · reordered {reorder_ok} · spans 1,2 → {} written {sized_ok} and 1,3 or a short list → {}/{} refused {spans_refused_ok} · clear on a type-less screen → {} ({}) and left it as it was {refused_ok} · clear where a panel is the fallback → {} · configcheck → {}",
            created.status,
            added.status,
            reordered.status,
            sized.status,
            wide.status,
            short.status,
            refused.status,
            if refused.err.contains("view.empty") {
                "view.empty"
            } else {
                "wrong reason"
            },
            designed_clear.status,
            check_run.status,
        ),
    )
}

/// UI · P3/U6's destructive paths, through the shipped binary: a kind delete
/// with its cascade, the schema-mode refusal, a view delete, a destination
/// removal, a screen assignment and a bulk defer — each followed by the engine's
/// own `configcheck`, because a cascade that leaves an invalid plan is the one
/// failure this gate exists for.
fn machine_delete_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("machine-deletes", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();

    // A kind with records refuses the schema-only reading, and says the count.
    let refused = harness.run(
        &[
            "type.delete",
            "topic",
            "--mode",
            "schema",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    // The full reading takes them, and the plan still loads.
    let kind = harness.run(
        &[
            "type.delete",
            "note",
            "--mode",
            "records",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let after_kind = harness.run(&["configcheck", "--plan", &plan_arg, "--json"], &[], None);
    // A destination removal keeps the view; the view delete takes it.
    let list = harness.run(
        &["list.delete", "plan.screen", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let view = harness.run(
        &["view.delete", "plan.units", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    // A screen assignment and a bulk defer, through the same door.
    let panel = harness.run(
        &[
            // `programs.all` survives the kind delete above (the check's own
            // order matters: `note`'s views are gone by now).
            "view.setPanel",
            "programs.all",
            "--panel",
            "notes",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let deferred = harness.run(
        &[
            "record.defer",
            "--id",
            "t.demo.01",
            "--id",
            "t.demo.02",
            "--date",
            "2026-11-11",
            "--reviews",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let after = harness.run(&["configcheck", "--plan", &plan_arg, "--json"], &[], None);
    let kinds_gone = harness
        .run(&["views", "--plan", &plan_arg, "--json"], &[], None)
        .out
        .contains("notes.screen")
        == false;

    let ok = refused.status == 1
        && refused.err.contains("type.has-records")
        && kind.status == 0
        && after_kind.status == 0
        && after_kind.out.contains("\"ok\":true")
        && list.status == 0
        && view.status == 0
        && panel.status == 0
        && deferred.status == 0
        && after.status == 0
        && after.out.contains("\"ok\":true")
        && kinds_gone;
    check(
        "machine-deletes",
        ok,
        format!(
            "type.delete --mode schema → exit {} ({}) · kind/view/list deletes → {}/{}/{} · setPanel {} · defer {} · configcheck after → {} · the kind's views are gone: {kinds_gone}",
            refused.status,
            if refused.err.contains("type.has-records") {
                "refused by count"
            } else {
                "wrong reason"
            },
            kind.status,
            view.status,
            list.status,
            panel.status,
            deferred.status,
            after.status,
        ),
    )
}

// ── Phase 7: themes, profiles, presets and the design doctor ────────────────

fn phase_seven_checks(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    let mut results = vec![
        theme_contrast_check(resources_dir),
        theme_resolution_check(harness, resources_dir),
        design_doctor_check(harness, resources_dir),
        profile_round_trip_check(harness, resources_dir),
    ];
    results.extend(preset_checks(harness, resources_dir));
    results.push(guide_sharing_check(harness, resources_dir));
    results
}

/// §6 Phase 7's contrast gate: every shipped theme resolves in both modes with
/// every normal-text pair at or above 4.5:1 — **recomputed** from the resolved
/// colours, not read from the design system's metadata (§1.4.A). The deliberate
/// break: one token pinned to near-black card must drop a pair below the floor.
fn theme_contrast_check(resources_dir: &Path) -> CheckResult {
    let register = match resources::TokenRegister::bundled(resources_dir) {
        Ok(register) => register,
        Err(error) => return check("theme-contrast", false, error),
    };
    let themes = match crate::theme::list(resources_dir) {
        Ok(themes) => themes,
        Err(error) => return check("theme-contrast", false, error),
    };
    if themes.len() < 4 {
        return check(
            "theme-contrast",
            false,
            format!("expected the four shipped themes, found {}", themes.len()),
        );
    }
    let mut checked = 0usize;
    for theme in &themes {
        for mode in ["light", "dark"] {
            let resolved = match crate::theme::resolve(&register, Some(theme), mode, 1.0, None) {
                Ok(resolved) => resolved,
                Err(error) => {
                    return check("theme-contrast", false, format!("{}: {error}", theme.id));
                }
            };
            for pair in &resolved.contrast {
                if !pair.computed {
                    return check(
                        "theme-contrast",
                        false,
                        format!(
                            "{} in {mode}: {} on {} could not be computed",
                            theme.id, pair.fg, pair.bg
                        ),
                    );
                }
                if !pair.ok {
                    return check(
                        "theme-contrast",
                        false,
                        format!(
                            "{} in {mode}: {} on {} is {:.2}:1",
                            theme.id, pair.fg, pair.bg, pair.ratio
                        ),
                    );
                }
                checked += 1;
            }
        }
    }
    // The deliberate break: a card forced near-black fails a light theme.
    // (The theme must be the light one — a pinned dark theme would resolve its
    // own dark overlay first and legitimately keep the contrast.)
    let light_theme = themes
        .iter()
        .find(|theme| theme.id == "cadence-light")
        .or_else(|| themes.first());
    let broken = crate::theme::resolve(
        &register,
        light_theme,
        "light",
        1.0,
        Some(&serde_json::json!({ "color": { "surface": { "card": "oklch(12% 0 0)" } } })),
    )
    .expect("the broken override resolves");
    if broken.failures().count() == 0 {
        return check(
            "theme-contrast",
            false,
            "a near-black card did not fail the contrast gate",
        );
    }
    // And the text-scale preference moves the type, not the colours.
    let scaled = crate::theme::resolve(&register, light_theme, "light", 1.15, None)
        .expect("the scaled register resolves");
    let grew = scaled
        .css
        .get("--text-base")
        .map(|value| value.as_str() != "15px")
        .unwrap_or(false);
    if !grew {
        return check(
            "theme-contrast",
            false,
            "textScale did not move --text-base",
        );
    }
    check(
        "theme-contrast",
        true,
        format!(
            "{} themes × 2 modes · {checked} pairs ≥ 4.5:1 · a broken card fails",
            themes.len()
        ),
    )
}

/// `theme.set`, `appearance.resolve`, `appearance.setOverride` and the text
/// scale, through the shipped binary.
fn theme_resolution_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("theme-resolve", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();
    let resolve = |mode: &str| -> Option<serde_json::Value> {
        let outcome = harness.run(
            &[
                "appearance.resolve",
                "--plan",
                &plan_arg,
                "--mode",
                mode,
                "--json",
            ],
            &[],
            None,
        );
        if outcome.status != 0 {
            return None;
        }
        envelope(&outcome.out).map(|env| env["data"].clone())
    };

    let light = resolve("light").unwrap_or(serde_json::Value::Null);
    if light["resolvedMode"] != "light" || light["theme"] != "cadence-light" {
        return check(
            "theme-resolve",
            false,
            format!("the seed's default resolved as {light}"),
        );
    }
    let dark = resolve("dark").unwrap_or(serde_json::Value::Null);
    if dark["resolvedMode"] != "dark" || dark["css"]["--card"] == light["css"]["--card"] {
        return check(
            "theme-resolve",
            false,
            format!("dark did not resolve dark: {dark}"),
        );
    }

    // Pin the dark theme: a plan keeps its room whatever the machine says.
    let pinned = harness.run(
        &[
            "theme.set",
            "--id",
            "cadence-dark",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    if pinned.status != 0 {
        return check(
            "theme-resolve",
            false,
            format!(
                "theme.set: exit {} {}",
                pinned.status,
                prefix(&pinned.err, 200)
            ),
        );
    }
    let pinned_light = resolve("light").unwrap_or(serde_json::Value::Null);
    if pinned_light["theme"] != "cadence-dark" || pinned_light["resolvedMode"] != "dark" {
        return check(
            "theme-resolve",
            false,
            format!("a pinned dark theme followed the machine: {pinned_light}"),
        );
    }

    // Unknown ids and out-of-range values are refused before a byte is staged.
    let unknown = harness.run(
        &["theme.set", "--id", "nope", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let bad_key = harness.run(
        &[
            "appearance.setOverride",
            "--key",
            "color.nope.deep",
            "--value",
            "oklch(50% 0 0)",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let bad_scale = harness.run(
        &[
            "appearance.setTextScale",
            "--value",
            "9",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    if unknown.status != 2 || bad_key.status != 2 || bad_scale.status != 2 {
        return check(
            "theme-resolve",
            false,
            format!(
                "refusals: theme {} key {} scale {}",
                unknown.status, bad_key.status, bad_scale.status
            ),
        );
    }

    // A real override resolves, and the scale multiplies every rung.
    let set = harness.run(
        &[
            "appearance.setOverride",
            "--key",
            "color.surface.backdrop",
            "--value",
            "oklch(95% 0.004 250)",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let scale = harness.run(
        &[
            "appearance.setTextScale",
            "--value",
            "1.15",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let resolved = resolve("light").unwrap_or(serde_json::Value::Null);
    let moved = resolved["css"]["--backdrop"] == "oklch(95% 0.004 250)"
        && resolved["css"]["--text-base"] == "17.25px";
    // Reset returns the theme's value (§4.1: reset removes the key).
    let cleared = harness.run(
        &[
            "appearance.clearOverride",
            "--key",
            "color.surface.backdrop",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let after = resolve("light").unwrap_or(serde_json::Value::Null);
    let reset = after["css"]["--backdrop"] != "oklch(95% 0.004 250)";

    let ok = set.status == 0 && scale.status == 0 && moved && cleared.status == 0 && reset;
    check(
        "theme-resolve",
        ok,
        if ok {
            "cadence-light resolves · dark resolves dark · a pinned theme overrides the machine · unknown ids and out-of-range scales → exit 2 · override set and reset · textScale 1.15 → 17.25px".into()
        } else {
            format!(
                "set {} scale {} moved {moved} cleared {} reset {reset}",
                set.status, scale.status, cleared.status
            )
        },
    )
}

/// §5's doctor over resolved colours: a clean plan has no errors, a bad override
/// produces one, a waiver records the decision, and a severity change makes an
/// advisory an error.
fn design_doctor_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("design-doctor", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();
    let doctor = || -> Option<serde_json::Value> {
        let outcome = harness.run(&["design.check", "--plan", &plan_arg, "--json"], &[], None);
        if outcome.status != 0 {
            return None;
        }
        envelope(&outcome.out).map(|env| env["data"].clone())
    };

    let clean = doctor().unwrap_or(serde_json::Value::Null);
    if clean["errors"] != 0 {
        return check(
            "design-doctor",
            false,
            format!("the seed reports {} errors: {clean}", clean["errors"]),
        );
    }
    if clean["unobservable"].as_array().map(Vec::len).unwrap_or(0) == 0 {
        return check(
            "design-doctor",
            false,
            "no rule is reported unobservable — the vocabulary should say which rules the engine cannot see".to_string(),
        );
    }

    // A backdrop forced dark under light ink fails the contrast rule.
    let broken = harness.run(
        &[
            "appearance.setOverride",
            "--key",
            "color.surface.backdrop",
            "--value",
            "oklch(12% 0 0)",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let failing = doctor().unwrap_or(serde_json::Value::Null);
    if broken.status != 0 || failing["errors"].as_u64().unwrap_or(0) == 0 {
        return check(
            "design-doctor",
            false,
            format!(
                "a near-black backdrop reported {} errors",
                failing["errors"]
            ),
        );
    }

    // A waiver records the decision instead of silencing the finding.
    let rules = plan.join("content/rules.json");
    if let Err(error) = edit_json(&rules, |rules| {
        rules["lint"] = serde_json::json!({
            "waivers": [
                { "id": "design.contrast.on-wash", "waivedAt": "2026-09-26", "reason": "Personal preference" }
            ]
        });
    }) {
        return check("design-doctor", false, error);
    }
    let waived = doctor().unwrap_or(serde_json::Value::Null);
    if waived["errors"] != 0 || waived["waived"].as_u64().unwrap_or(0) == 0 {
        return check(
            "design-doctor",
            false,
            format!("a waiver did not clear the error: {waived}"),
        );
    }

    // A severity change turns an advisory finding into an error: a view named
    // after a streak is exactly what §11 refuses.
    if let Err(error) = edit_json(&plan.join("content/views.json"), |views| {
        views["views"]["streak.board"] = serde_json::json!({ "type": "topic", "layout": "board" });
    }) {
        return check("design-doctor", false, error);
    }
    if let Err(error) = edit_json(&rules, |rules| {
        rules["lint"]["rules"] =
            serde_json::json!({ "design.no.streak-guilt": { "severity": "error" } });
    }) {
        return check("design-doctor", false, error);
    }
    let promoted = doctor().unwrap_or(serde_json::Value::Null);
    let streak = promoted["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|rule| rule["id"] == "design.no.streak-guilt");
    let found = streak
        .map(|rule| rule["findings"].as_array().map(Vec::len).unwrap_or(0))
        .unwrap_or(0);
    let promoted_to_error = streak
        .map(|rule| rule["severity"] == "error")
        .unwrap_or(false);
    // One error: the streak finding. The contrast failure is waived, so it is
    // reported as waived rather than counted.
    let is_error = promoted["errors"] == 1 && promoted["waived"].as_u64().unwrap_or(0) >= 1;
    let ok = broken.status == 0 && found > 0 && promoted_to_error && is_error;
    check(
        "design-doctor",
        ok,
        if ok {
            "seed clean · near-black backdrop → contrast error · a waiver marks it waived without hiding it · severity error promotes a streak label".into()
        } else {
            format!(
                "findings {found} errors {} promoted {is_error}",
                promoted["errors"]
            )
        },
    )
}

/// §6 Phase 7's export/import round trip, with and without personal state.
fn profile_round_trip_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let source = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("profile-round-trip", false, error),
    };
    let source_arg = source.to_string_lossy().to_string();
    let scratch = std::env::temp_dir().join(unique("sam-p7-profile"));
    let _ = std::fs::create_dir_all(&scratch);
    let shared = scratch.join("shared.samprofile");
    let mine = scratch.join("mine.samprofile");

    // Give the source real progress so a personal export has state to carry.
    let advance = harness.run(
        &[
            "record.advanceStage",
            "--id",
            "t.demo.01",
            "--stage",
            "learned",
            "--plan",
            &source_arg,
            "--json",
        ],
        &[],
        None,
    );

    let public_export = harness.run(
        &[
            "profile.export",
            "--out",
            &shared.to_string_lossy(),
            "--plan",
            &source_arg,
            "--json",
        ],
        &[],
        None,
    );
    let personal_export = harness.run(
        &[
            "profile.export",
            "--out",
            &mine.to_string_lossy(),
            "--personal",
            "--plan",
            &source_arg,
            "--json",
        ],
        &[],
        None,
    );
    if advance.status != 0 || public_export.status != 0 || personal_export.status != 0 {
        return check(
            "profile-round-trip",
            false,
            format!(
                "advance {} export {} personal {}",
                advance.status, public_export.status, personal_export.status
            ),
        );
    }
    let shared_bytes = std::fs::read(&shared).unwrap_or_default();
    let mine_bytes = std::fs::read(&mine).unwrap_or_default();
    let shared_doc: serde_json::Value = serde_json::from_slice(&shared_bytes).unwrap_or_default();
    let mine_doc: serde_json::Value = serde_json::from_slice(&mine_bytes).unwrap_or_default();
    // The schema travels (views over a note are legitimate); the records do not.
    let shared_records = shared_doc["records"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    if shared_records.contains_key("note") || shared_records.contains_key("session") {
        return check(
            "profile-round-trip",
            false,
            format!(
                "a default export carried private records: {:?}",
                shared_records.keys().collect::<Vec<_>>()
            ),
        );
    }
    if shared_doc["includes"]["excludedTypes"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        == 0
    {
        return check(
            "profile-round-trip",
            false,
            "a default export named no excluded kinds".to_string(),
        );
    }
    if mine_doc["state"].is_null() {
        return check(
            "profile-round-trip",
            false,
            "a personal export carried no progress state".to_string(),
        );
    }
    if !mine_doc["records"]
        .as_object()
        .map(|r| r.contains_key("note"))
        .unwrap_or(false)
    {
        return check(
            "profile-round-trip",
            false,
            "a personal export left the private records out".to_string(),
        );
    }

    let public_plan = scratch.join("public-plan");
    let personal_plan = scratch.join("personal-plan");
    let imported_public = harness.run(
        &[
            "profile.import",
            "--file",
            &shared.to_string_lossy(),
            "--into",
            &public_plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );
    let imported_personal = harness.run(
        &[
            "profile.import",
            "--file",
            &mine.to_string_lossy(),
            "--into",
            &personal_plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );
    if imported_public.status != 0 || imported_personal.status != 0 {
        return check(
            "profile-round-trip",
            false,
            format!(
                "import public {} personal {} · {}",
                imported_public.status,
                imported_personal.status,
                prefix(&imported_public.err, 200)
            ),
        );
    }
    let counts = |plan: &Path| -> (usize, usize) {
        let records = harness.run(
            &[
                "records",
                "--type",
                "topic",
                "--limit",
                "1000",
                "--plan",
                &plan.to_string_lossy(),
                "--json",
            ],
            &[],
            None,
        );
        let views = harness.run(
            &["views", "--plan", &plan.to_string_lossy(), "--json"],
            &[],
            None,
        );
        (
            envelope(&records.out)
                .and_then(|env| env["data"]["records"].as_array().map(Vec::len))
                .unwrap_or(0),
            envelope(&views.out)
                .and_then(|env| env["data"]["views"].as_object().map(|views| views.len()))
                .unwrap_or(0),
        )
    };
    let (source_topics, source_views) = counts(&source);
    let (public_topics, public_views) = counts(&public_plan);
    let (personal_topics, personal_views) = counts(&personal_plan);
    let state_moved = personal_plan.join("state/state.json").is_file();

    // A broken record refuses with its own line, and leaves nothing behind.
    let broken = scratch.join("broken.samprofile");
    let mut value: serde_json::Value = serde_json::from_slice(&mine_bytes).unwrap_or_default();
    if let Some(rows) = value["records"]["topic"].as_array_mut()
        && let Some(first) = rows.first_mut()
    {
        first["fields"]["est"] = serde_json::json!("not a duration");
    }
    let _ = std::fs::write(
        &broken,
        serde_json::to_vec_pretty(&value).unwrap_or_default(),
    );
    let broken_plan = scratch.join("broken-plan");
    let refused = harness.run(
        &[
            "profile.import",
            "--file",
            &broken.to_string_lossy(),
            "--into",
            &broken_plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );
    let ok = source_topics == public_topics
        && source_views == public_views
        && source_topics == personal_topics
        && personal_views == source_views
        && state_moved
        && refused.status == 1
        && refused.err.contains("topic.jsonl")
        && !broken_plan.exists();

    let detail = if ok {
        format!(
            "public export {public_topics} topics × {public_views} views · personal carries state · import reproduces both · a broken record → exit 1 naming topic.jsonl, nothing written"
        )
    } else {
        format!(
            "topics {source_topics}/{public_topics}/{personal_topics} · views {source_views}/{public_views}/{personal_views} · state {state_moved} · refused {} ({}) · leftover {}",
            refused.status,
            prefix(&refused.err, 120),
            broken_plan.exists()
        )
    };
    let _ = std::fs::remove_dir_all(&scratch);
    check("profile-round-trip", ok, detail)
}

/// Every preset in the bundle: structural, example and one interaction — the
/// Phase 7 gate's first clause, run against the shipped files.
fn preset_checks(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    let dir = resources_dir.join("presets");
    let mut names: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
            .collect(),
        Err(error) => {
            return vec![check(
                "preset-valid",
                false,
                format!("cannot list {}: {error}", dir.display()),
            )];
        }
    };
    names.sort();
    let mut results = Vec::new();
    let mut summary = Vec::new();
    for name in &names {
        let preset = dir.join(name);
        let outcome = harness.run(
            &[
                "--configcheck",
                "--plan",
                &preset.to_string_lossy(),
                "--json",
            ],
            &[],
            None,
        );
        if outcome.status != 0 {
            results.push(check(
                "preset-valid",
                false,
                format!(
                    "{name}: configcheck exit {} · {}",
                    outcome.status,
                    prefix(&outcome.err, 200)
                ),
            ));
            continue;
        }
        let loaded = crate::config_store::load_with(&preset, "explicit", resources_dir);
        let config = match loaded {
            Ok(config) => config,
            Err(error) => {
                results.push(check("preset-valid", false, format!("{name}: {error}")));
                continue;
            }
        };
        // Structural: every type has a view and every view names a real type.
        let viewless: Vec<&String> = config
            .types
            .keys()
            .filter(|type_id| {
                !config
                    .views
                    .values()
                    .any(|view| view.type_.as_deref() == Some(type_id.as_str()))
            })
            .collect();
        if !viewless.is_empty() {
            results.push(check(
                "preset-valid",
                false,
                format!("{name}: types with no view: {viewless:?}"),
            ));
            continue;
        }
        // Labels: every field names itself in plain language (§5 I2) — the label
        // is what the UI shows, and a preset with none has nothing to show.
        let unlabelled: Vec<String> = config
            .types
            .iter()
            .flat_map(|(type_id, def)| {
                def.fields
                    .iter()
                    .filter(|field| {
                        field
                            .label
                            .as_deref()
                            .is_none_or(|label| label.trim().is_empty())
                    })
                    .map(move |field| format!("{type_id}.{}", field.key))
            })
            .collect();
        if !unlabelled.is_empty() {
            results.push(check(
                "preset-valid",
                false,
                format!("{name}: fields with no label: {unlabelled:?}"),
            ));
            continue;
        }
        // Example: a populated preset carries at least one record per type, so
        // the preset shows what the kind is for rather than an empty table.
        // `blank` (docs/presets.md §blank) ships the full schema with zero
        // record files by design — its tables are the empty state.
        let populated = config.record_count() > 0;
        let exempless: Vec<&String> = if populated {
            config
                .types
                .keys()
                .filter(|type_id| {
                    !config
                        .records_iter()
                        .any(|record| record.type_ == **type_id)
                })
                .collect()
        } else {
            Vec::new()
        };
        if !exempless.is_empty() {
            results.push(check(
                "preset-valid",
                false,
                format!("{name}: types with no example record: {exempless:?}"),
            ));
            continue;
        }
        // Interaction: a copy of the preset takes a stage advance and a view
        // read — the same commands a student's first session runs.
        let copy = match temp_plan_from(&preset, name) {
            Ok(copy) => copy,
            Err(error) => {
                results.push(check("preset-valid", false, format!("{name}: {error}")));
                continue;
            }
        };
        let copy_arg = copy.to_string_lossy().to_string();
        let trackable = config
            .types
            .iter()
            .find(|(_, def)| def.pipeline.is_some())
            .and_then(|(type_id, def)| {
                let pipeline = def.pipeline.clone()?;
                let stages = config
                    .rules
                    .pipelines
                    .as_ref()
                    .and_then(|pipelines| pipelines.get(&pipeline))
                    .map(|pipeline| pipeline.stages.clone())
                    .unwrap_or_default();
                let record = config
                    .records_iter()
                    .find(|record| record.type_ == *type_id)?;
                stages
                    .first()
                    .map(|stage| (record.id.clone(), stage.clone()))
            });
        let mut interactions = 0;
        if let Some((id, stage)) = trackable {
            let advanced = harness.run(
                &[
                    "record.advanceStage",
                    "--id",
                    &id,
                    "--stage",
                    &stage,
                    "--plan",
                    &copy_arg,
                    "--json",
                ],
                &[],
                None,
            );
            if advanced.status != 0 {
                results.push(check(
                    "preset-valid",
                    false,
                    format!(
                        "{name}: advanceStage {id} → {stage} exit {} · {}",
                        advanced.status,
                        prefix(&advanced.err, 200)
                    ),
                ));
                continue;
            }
            interactions += 1;
        }
        if let Some(first) = config
            .shell
            .as_ref()
            .and_then(|shell| shell.navigation.as_ref())
            .and_then(|navigation| navigation.first())
        {
            let viewed = harness.run(
                &["view", &first.view, "--plan", &copy_arg, "--json"],
                &[],
                None,
            );
            if viewed.status != 0 {
                results.push(check(
                    "preset-valid",
                    false,
                    format!("{name}: view {} exit {}", first.view, viewed.status),
                ));
                continue;
            }
            interactions += 1;
        }
        let _ = std::fs::remove_dir_all(&copy);
        summary.push(format!(
            "{name} {} types · {} views · {interactions} interaction(s)",
            config.types.len(),
            config.views.len()
        ));
    }
    results.push(check(
        "preset-valid",
        results.is_empty() && names.len() >= 6,
        format!("{} presets · {}", names.len(), summary.join(" · ")),
    ));
    results
}

/// A writable copy of one bundled preset.
fn temp_plan_from(preset: &Path, tag: &str) -> Result<PathBuf, String> {
    let temp = std::env::temp_dir().join(unique(&format!("sam-p7-{tag}")));
    transaction::copy_tree(preset, &temp)
        .map_err(|error| format!("cannot stage {tag}: {error}"))?;
    Ok(temp)
}

/// The guide's sharing section: the ids it teaches are the files this build
/// ships.
fn guide_sharing_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("guide-sharing", false, error),
    };
    let outcome = harness.run(
        &["--guide", "--plan", &plan.to_string_lossy(), "--json"],
        &[],
        None,
    );
    let guide = envelope(&outcome.out).map(|env| env["data"].clone());
    let Some(guide) = guide else {
        return check(
            "guide-sharing",
            false,
            "the guide emitted nothing".to_string(),
        );
    };
    let presets = guide["sharing"]["presets"]["available"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let themes = guide["sharing"]["themes"]["available"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut on_disk: Vec<String> = std::fs::read_dir(resources_dir.join("presets"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    on_disk.sort();
    let mut taught: Vec<String> = presets
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect();
    taught.sort();
    let profiles = guide["sharing"]["profiles"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    let ok = taught == on_disk && themes.len() >= 4 && profiles >= 4;
    check(
        "guide-sharing",
        ok,
        format!(
            "{} presets taught and on disk · {} themes · {profiles} profile rules",
            taught.len(),
            themes.len()
        ),
    )
}

// ── tokens (Phase 0's register, alias-resolving and override-aware) ─────────

fn token_checks(resources_dir: &Path) -> Vec<CheckResult> {
    let register = match resources::TokenRegister::bundled(resources_dir) {
        Ok(register) => register,
        Err(error) => {
            return vec![
                check("tokens-load", false, &error),
                check("tokens-alias", false, "no register"),
                check("tokens-hex-mirror", false, "no register"),
                check("tokens-override-merge", false, "no register"),
                check("tokens-override-unknown", false, "no register"),
                check("tokens-cycle", false, "no register"),
            ];
        }
    };

    let unresolved = register
        .tokens
        .values()
        .filter(|token| resources::contains_reference(&token.value))
        .count();
    let with_hex = register
        .tokens
        .values()
        .filter(|token| token.hex.is_some())
        .count();

    let mut results = vec![
        check(
            "tokens-load",
            register.tokens.len() > 100 && unresolved == 0,
            format!(
                "{} tokens, version {}, {unresolved} unresolved",
                register.tokens.len(),
                register.version
            ),
        ),
        check(
            "tokens-alias",
            register.get("color.focus").map(|token| &token.value)
                == register
                    .get("color.ink-level.ink")
                    .map(|token| &token.value),
            "color.focus resolves to color.ink-level.ink",
        ),
        check(
            "tokens-hex-mirror",
            with_hex > 20,
            format!("{with_hex} tokens carry $extensions.hex"),
        ),
    ];

    let overrides = serde_json::json!({
        "color": {"focus": "#000000", "surface": {"card": "oklch(50% 0 0)"}}
    });
    match register.merged(&overrides) {
        Ok(merged) => {
            let card = merged
                .get("color.surface.card")
                .map(|token| token.value.clone());
            let alias_picks_up = merged.get("color.focus").map(|token| &token.value)
                == Some(&serde_json::Value::String("#000000".into()));
            results.push(check(
                "tokens-override-merge",
                card == Some(serde_json::Value::String("oklch(50% 0 0)".into())) && alias_picks_up,
                "scalar replaced · group descended · alias re-resolved",
            ));
        }
        Err(message) => results.push(check("tokens-override-merge", false, message)),
    }
    results.push(check(
        "tokens-override-unknown",
        register
            .merged(&serde_json::json!({"color": {"nope": "x"}}))
            .is_err(),
        "unknown override key rejected",
    ));
    results.push(check(
        "tokens-cycle",
        resources::TokenRegister::parse(
            br#"{"$version":"t","a":{"$type":"color","$value":"{b}"},
                 "b":{"$type":"color","$value":"{a}"}}"#,
        )
        .is_err(),
        "alias cycle rejected",
    ));
    results
}

// ── the seed preset ─────────────────────────────────────────────────────────

fn load_seed(resources_dir: &Path) -> Result<ResolvedConfig, ConfigStoreError> {
    config_store::load_with(
        &resources_dir.join("presets/seed"),
        "explicit",
        resources_dir,
    )
}

fn seed_check(resources_dir: &Path) -> CheckResult {
    match (load_seed(resources_dir), load_seed(resources_dir)) {
        (Ok(config), Ok(again)) => {
            // Phase 5 turns the seed from four lists into seven destinations
            // composed of saved views, so the shape it pins is the screen set.
            let destinations = config
                .shell
                .as_ref()
                .and_then(|shell| shell.navigation.as_deref())
                .map(|entries| entries.len())
                .unwrap_or(0);
            check(
                "seed-valid",
                config.types.len() == 12
                    && config.record_count() == 17
                    && config.views.len() == 23
                    && destinations == 7
                    && config.revision == again.revision,
                format!(
                    "12 types · 17 records · 23 views · {destinations} destinations · revision {} stable · {} advisory",
                    prefix(&config.revision, 8),
                    config.load_warnings.len()
                ),
            )
        }
        (Err(error), _) | (_, Err(error)) => check("seed-valid", false, error.to_string()),
    }
}

fn seed_tree_check(resources_dir: &Path) -> CheckResult {
    match load_seed(resources_dir) {
        Ok(config) => {
            let lines = config.content_tree().len();
            check(
                "seed-tree",
                lines > 30,
                format!("content tree {lines} lines"),
            )
        }
        Err(error) => check("seed-tree", false, error.to_string()),
    }
}

// ── §3.3 model test (§10) — same expectations, real loader ──────────────────

fn problemset_checks(resources_dir: &Path) -> Vec<CheckResult> {
    let config = match config_store::load_with(
        &resources_dir.join("fixtures/problemset"),
        "explicit",
        resources_dir,
    ) {
        Ok(config) => config,
        Err(error) => {
            let message = error.to_string();
            return vec![
                check("problemset-fields", false, &message),
                check("problemset-cardinality", false, &message),
                check("problemset-formula", false, &message),
                check("problemset-canonical", false, &message),
            ];
        }
    };

    let Some(kind) = config.types.get("problemset") else {
        return vec![check("problemset-fields", false, "the type is missing")];
    };
    let expected: std::collections::BTreeSet<&str> = [
        "chapter",
        "difficulty",
        "total",
        "attempted",
        "solved",
        "pct",
        "source",
        "lastAttempt",
    ]
    .into_iter()
    .collect();
    let found: std::collections::BTreeSet<&str> =
        kind.fields.iter().map(|field| field.key.as_str()).collect();
    let fields_ok = found == expected;
    let mut results = vec![check(
        "problemset-fields",
        fields_ok,
        format!("8 fields, pct = pct(solved, total) — found {found:?}"),
    )];
    if !fields_ok {
        return results;
    }

    let records: Vec<&Record> = config
        .records_iter()
        .filter(|record| record.type_ == "problemset")
        .collect();
    let cardinality_ok = records.iter().all(|record| {
        !record.fields.contains_key("course")
            && !record.fields.contains_key("pct")
            && record.links.get("course").is_some_and(|ids| ids.len() == 1)
    });
    results.push(check(
        "problemset-cardinality",
        cardinality_ok,
        "course relation cardinality=one, values in links, pct never persisted",
    ));

    let expr = kind
        .field("pct")
        .and_then(|field| field.expr.clone())
        .unwrap_or_default();
    let pct_of = |id: &str| {
        config
            .record(id)
            .map(|record| crate::evaluator::formula_scalar(&config, record, &expr))
    };
    results.push(check(
        "problemset-formula",
        pct_of("p.dp.01") == Some(Some(L2Value::Int(60)))
            && pct_of("p.dp.02") == Some(Some(L2Value::Null))
            && pct_of("p.dp.03") == Some(Some(L2Value::Int(100))),
        "pct: 12/20→60 · 0/0→null · 9/9→100",
    ));
    results.push(check(
        "problemset-canonical",
        config.records_iter().all(ResolvedConfig::round_trips),
        "records round-trip byte-identical",
    ));
    results
}

// ── invalid fixtures ────────────────────────────────────────────────────────

const INVALID_FIXTURES: [(&str, &str, &str); 10] = [
    (
        "invalid-value",
        "record.invalid-value",
        "content/records/topic.jsonl",
    ),
    (
        "unknown-schema-version",
        "schema-version.unsupported",
        "content/types.json",
    ),
    (
        "type-parent-cycle",
        "types.parent-cycle",
        "content/types.json",
    ),
    ("formula-cycle", "formula.cycle", "content/types.json"),
    (
        "dangling-relation",
        "record.dangling-link",
        "content/records/topic.jsonl",
    ),
    (
        "duplicate-ids",
        "record.duplicate-id",
        "content/records/topic.jsonl",
    ),
    ("empty-input", "json.eof", "content/types.json"),
    ("invalid-utf8", "encoding.invalid", "content/types.json"),
    (
        "malformed-jsonl",
        "json.invalid",
        "content/records/topic.jsonl",
    ),
    ("duplicate-keys", "json.duplicate-key", "content/types.json"),
];

fn invalid_fixtures_check(resources_dir: &Path) -> CheckResult {
    let mut failures = Vec::new();
    for (dir, code, file) in INVALID_FIXTURES {
        let root = resources_dir.join("fixtures/invalid").join(dir);
        match config_store::load_with(&root, "explicit", resources_dir) {
            Ok(_) => failures.push(format!("{dir}=loaded(!!)")),
            Err(ConfigStoreError::Validation(diagnostics)) => {
                let wants_line = file.ends_with(".jsonl");
                let hit = diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == code
                        && diagnostic.path.starts_with(file)
                        && (!wants_line || diagnostic.line.is_some())
                });
                if !hit {
                    failures.push(format!("{dir}=wrong-code({diagnostics:?})"));
                }
            }
            Err(error) => failures.push(format!("{dir}=io({error})")),
        }
    }
    check(
        "invalid-fixtures",
        failures.is_empty(),
        if failures.is_empty() {
            format!(
                "{} fixtures fail with the expected code/path",
                INVALID_FIXTURES.len()
            )
        } else {
            failures.join(" · ")
        },
    )
}

fn crlf_check(resources_dir: &Path) -> CheckResult {
    match config_store::load_with(
        &resources_dir.join("fixtures/crlf-ok"),
        "explicit",
        resources_dir,
    ) {
        Ok(config) => check(
            "crlf-tolerated",
            config.record_count() == 2,
            "2 records loaded from CRLF, no-final-newline JSONL",
        ),
        Err(error) => check("crlf-tolerated", false, error.to_string()),
    }
}

// ── watcher (D9) ────────────────────────────────────────────────────────────

fn watcher_check(resources_dir: &Path) -> CheckResult {
    use std::time::Duration;

    let temp = match temp_plan(resources_dir) {
        Ok(temp) => temp,
        Err(error) => return check("watcher", false, error),
    };
    // 60 ms for the check; D9's shipped default is 150.
    let (watcher, events) = config_watcher::channel_events(temp.clone(), 60);

    let initial = match events.recv_timeout(Duration::from_secs(5)) {
        Ok(event) => event,
        Err(error) => {
            return finish_watch(temp, watcher, false, format!("no initial publish: {error}"));
        }
    };
    let initial_records = initial.config.as_ref().map(|config| config.record_count());
    if initial_records != Some(17) {
        return finish_watch(
            temp,
            watcher,
            false,
            format!("initial publish had {initial_records:?} records, expected 17"),
        );
    }

    let topic_file = temp.join("content/records/topic.jsonl");
    let probe = r#"{"fields":{"est":10,"kind":"revise","title":"Watcher probe"},"id":"t.watch","links":{},"schemaVersion":1,"type":"topic"}"#;
    let append = std::fs::OpenOptions::new()
        .append(true)
        .open(&topic_file)
        .and_then(|mut handle| writeln!(handle, "{probe}"));
    if let Err(error) = append {
        return finish_watch(
            temp,
            watcher,
            false,
            format!("cannot append the probe: {error}"),
        );
    }

    match events.recv_timeout(Duration::from_secs(5)) {
        Ok(event) => {
            let records = event.config.as_ref().map(|config| config.record_count());
            if records != Some(18) {
                return finish_watch(
                    temp,
                    watcher,
                    false,
                    format!("reload after change had {records:?} records, expected 18"),
                );
            }
        }
        Err(error) => {
            return finish_watch(
                temp,
                watcher,
                false,
                format!("no reload after change: {error}"),
            );
        }
    }

    // Burst settles: give stragglers a window, then assert nothing more
    // published (content-hash no-op drop).
    std::thread::sleep(Duration::from_millis(600));
    match events.try_recv() {
        Err(std::sync::mpsc::TryRecvError::Empty) => finish_watch(
            temp,
            watcher,
            true,
            "1 publish on change (18 records), burst-debounced",
        ),
        Ok(_) => finish_watch(temp, watcher, false, "a burst published twice"),
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            finish_watch(temp, watcher, false, "watcher thread died")
        }
    }
}

fn finish_watch(
    temp: PathBuf,
    mut watcher: config_watcher::ConfigWatcher,
    ok: bool,
    detail: impl Into<String>,
) -> CheckResult {
    watcher.stop();
    let _ = std::fs::remove_dir_all(&temp);
    check("watcher", ok, detail)
}

// ── Phase 2: safe writes (§6 gates) ─────────────────────────────────────────

fn write_checks(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    let mut results = Vec::new();
    results.extend(apply_basic_check(harness, resources_dir));
    results.push(apply_batch_reject_check(harness, resources_dir));
    results.push(apply_revision_pin_check(harness, resources_dir));
    results.push(two_writers_check(harness, resources_dir));
    results.push(crash_boundaries_check(harness, resources_dir));
    results.push(readonly_check(harness, resources_dir));
    results.push(external_conflict_check(harness, resources_dir));
    results.push(backups_check(harness, resources_dir));
    results.push(plan_new_check(harness, resources_dir));
    results.push(registry_check(harness));
    results.push(agent_loop_check(harness, resources_dir));
    results
}

/// adds append · updates replace in place · untouched lines byte-preserved ·
/// new files canonical (§4.1, §4.7 #1), and a repeated batch changes no bytes
/// (§4.7 #3).
fn apply_basic_check(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return vec![check("apply-basic", false, error)],
    };
    let topic_file = plan.join("content/records/topic.jsonl");
    let seed_text = read_to_string(&topic_file);
    let seed_lines: Vec<&str> = seed_text.split('\n').collect();
    let updated = seed_lines[0].replace("\"est\":30", "\"est\":99");
    let batch = [
        updated.as_str(),
        seed_lines[1],
        &topic_line("t.p2.a", "Phase 2 add A"),
        &resource_line("r.p2.a"),
    ]
    .join("\n")
        + "\n";
    let batch_url = plan.parent().unwrap_or(Path::new(".")).join("batch.jsonl");
    if let Err(error) = write_file(&batch_url, &batch) {
        return vec![check("apply-basic", false, error)];
    }

    let outcome = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );
    let env = envelope(&outcome.out);
    let counts = env
        .as_ref()
        .and_then(|value| value.pointer("/data/counts"))
        .cloned();
    let new_lines: Vec<String> = read_to_string(&topic_file)
        .split('\n')
        .map(str::to_string)
        .collect();
    let resource_lines: Vec<String> = read_to_string(&plan.join("content/records/resource.jsonl"))
        .split('\n')
        .map(str::to_string)
        .collect();

    let count = |key: &str| {
        counts
            .as_ref()
            .and_then(|counts| counts.get(key))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(999)
    };
    let ok = outcome.status == 0
        && count("added") == 2
        && count("updated") == 1
        && count("unchanged") == 1
        && new_lines.len() == 6
        && new_lines[0].contains("\"est\":99")
        && new_lines[1..4] == seed_lines[1..4]
        && new_lines[4].contains("t.p2.a")
        && resource_lines.len() >= 2
        && resource_lines[resource_lines.len() - 2].contains("r.p2.a");

    let mut results = vec![check(
        "apply-basic",
        ok,
        if ok {
            "+2 added · 1 updated · 1 unchanged; untouched lines byte-identical".to_string()
        } else {
            format!(
                "status {} out={} err={}",
                outcome.status,
                prefix(&outcome.out, 200),
                prefix(&outcome.err, 300)
            )
        },
    )];

    // A repeated batch changes no bytes.
    let before = snapshot(&plan);
    let second = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );
    let second_env = envelope(&second.out);
    let same_revision = second_env
        .as_ref()
        .and_then(|value| value.get("revision").and_then(|r| r.as_str()))
        == env
            .as_ref()
            .and_then(|value| value.get("revision").and_then(|r| r.as_str()));
    let noop = second.status == 0 && same_revision && snapshot(&plan) == before;
    results.push(check(
        "apply-idempotent",
        noop,
        if noop {
            "second run: 0 bytes changed, revision stable".to_string()
        } else {
            format!("status {} revision stable={same_revision}", second.status)
        },
    ));
    let _ = std::fs::remove_file(&batch_url);
    let _ = std::fs::remove_dir_all(&plan);
    results
}

/// One bad line in 500 rejects the entire batch with line-level diagnostics;
/// nothing commits (§6 gate, §4.7 #5).
fn apply_batch_reject_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("apply-batch-reject", false, error),
    };
    let batch_url = plan.parent().unwrap_or(Path::new(".")).join("big.jsonl");
    let mut malformed: Vec<String> = (1..=500)
        .map(|n| topic_line(&format!("t.b.{n}"), &format!("B{n}")))
        .collect();
    malformed[249] = "{\"broken\"".into();
    if let Err(error) = write_file(&batch_url, &(malformed.join("\n"))) {
        return check("apply-batch-reject", false, error);
    }
    let before = snapshot(&plan);
    let lexical = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );

    let mut dangling = malformed.clone();
    dangling[249] = "{\"fields\":{\"est\":10,\"kind\":\"watch\",\"title\":\"B250\"},\"id\":\"t.b.250\",\"links\":{\"course\":[\"nope\"]},\"schemaVersion\":1,\"type\":\"topic\"}".into();
    if let Err(error) = write_file(&batch_url, &(dangling.join("\n"))) {
        return check("apply-batch-reject", false, error);
    }
    let semantic = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );

    let ok = lexical.status == 1
        && lexical.err.contains("json.invalid")
        && lexical.err.contains("\"line\":250")
        && semantic.status == 1
        && semantic.err.contains("record.dangling-link")
        && semantic.err.contains("\"line\":250")
        && snapshot(&plan) == before;
    let _ = std::fs::remove_file(&batch_url);
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "apply-batch-reject",
        ok,
        if ok {
            "500-line batch · 1 bad line (lexical + semantic) → exit 1, line 250, zero writes"
                .to_string()
        } else {
            format!(
                "lexical={}/{} semantic={}/{}",
                lexical.status,
                prefix(&lexical.err, 160),
                semantic.status,
                prefix(&semantic.err, 160)
            )
        },
    )
}

/// A stale `--if-revision` conflicts without touching source (§4.6 step 1).
fn apply_revision_pin_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("apply-revision-pin", false, error),
    };
    let revision = harness.revision_of(&plan);
    let batch_url = plan.parent().unwrap_or(Path::new(".")).join("pin.jsonl");
    if let Err(error) = write_file(&batch_url, &topic_line("t.pin.1", "Pin")) {
        return check("apply-revision-pin", false, error);
    }
    let before = snapshot(&plan);
    let stale = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--if-revision",
            "deadbeefdeadbeef",
            "--json",
        ],
        &[],
        None,
    );
    let after_stale = snapshot(&plan);
    let fresh = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--if-revision",
            &revision,
            "--json",
        ],
        &[],
        None,
    );
    let ok =
        stale.status == 3 && after_stale == before && fresh.status == 0 && !revision.is_empty();
    let _ = std::fs::remove_file(&batch_url);
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "apply-revision-pin",
        ok,
        if ok {
            "stale pin → exit 3, bytes untouched; fresh pin → exit 0".to_string()
        } else {
            format!("stale={} fresh={}", stale.status, fresh.status)
        },
    )
}

/// Two competing CLI writers: one success, one exit-3 conflict.
fn two_writers_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("tx-two-writers", false, error),
    };
    let revision = harness.revision_of(&plan);
    if revision.is_empty() {
        return check("tx-two-writers", false, "the plan has no revision");
    }
    let dir = plan.parent().unwrap_or(Path::new("."));
    let a = dir.join("race-a.jsonl");
    let b = dir.join("race-b.jsonl");
    if let Err(error) = write_file(&a, &topic_line("t.race.a", "Race A")) {
        return check("tx-two-writers", false, error);
    }
    if let Err(error) = write_file(&b, &topic_line("t.race.b", "Race B")) {
        return check("tx-two-writers", false, error);
    }

    let args = |path: &Path| -> Vec<String> {
        vec![
            "apply".into(),
            path.to_string_lossy().to_string(),
            "--plan".into(),
            plan.to_string_lossy().to_string(),
            "--if-revision".into(),
            revision.clone(),
            "--json".into(),
        ]
    };
    let first_args = args(&a);
    let second_args = args(&b);
    let first_refs: Vec<&str> = first_args.iter().map(String::as_str).collect();
    let second_refs: Vec<&str> = second_args.iter().map(String::as_str).collect();
    let first = harness.spawn(&first_refs, &[]);
    let second = harness.spawn(&second_refs, &[]);
    let mut statuses = [
        Harness::finish(first).status,
        Harness::finish(second).status,
    ];
    statuses.sort();
    let topic = read_to_string(&plan.join("content/records/topic.jsonl"));
    let has_a = topic.contains("t.race.a");
    let has_b = topic.contains("t.race.b");
    let ok = statuses == [0, 3] && (has_a != has_b);
    let _ = std::fs::remove_file(&a);
    let _ = std::fs::remove_file(&b);
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "tx-two-writers",
        ok,
        if ok {
            "exits [0, 3] · exactly one batch landed (revision conflict under the lock)".to_string()
        } else {
            format!("exits {statuses:?} hasA={has_a} hasB={has_b}")
        },
    )
}

/// Process death at every journal boundary recovers to a complete
/// before/after state (§6 gate, §4.6 step 3).
fn crash_boundaries_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let mut details: Vec<String> = Vec::new();
    let mut all_ok = true;

    for point in CrashPoint::ALL {
        let plan = match temp_plan(resources_dir) {
            Ok(plan) => plan,
            Err(error) => return check("tx-crash-boundaries", false, error),
        };
        let before = snapshot(&plan);
        let batch_url = plan.parent().unwrap_or(Path::new(".")).join("crash.jsonl");
        let batch = [
            topic_line("t.crash.1", "Crash A"),
            resource_line("r.crash.1"),
        ]
        .join("\n");
        if let Err(error) = write_file(&batch_url, &batch) {
            return check("tx-crash-boundaries", false, error);
        }

        let crashed = harness.run(
            &[
                "apply",
                &batch_url.to_string_lossy(),
                "--plan",
                &plan.to_string_lossy(),
                "--json",
            ],
            &[(transaction::CRASH_ENV, point.name())],
            None,
        );
        if !(crashed.signaled && crashed.status == 9) {
            all_ok = false;
            details.push(format!("{}=not-killed({})", point.name(), crashed.status));
            let _ = std::fs::remove_dir_all(&plan);
            continue;
        }

        // The next command runs recovery (every read and write session does).
        let recovered = harness.run(
            &["paths", "--plan", &plan.to_string_lossy(), "--json"],
            &[],
            None,
        );
        let mut point_ok = recovered.status == 0;
        if point == CrashPoint::Committed {
            let topic = read_to_string(&plan.join("content/records/topic.jsonl"));
            let resource = read_to_string(&plan.join("content/records/resource.jsonl"));
            point_ok = point_ok
                && topic.contains("t.crash.1")
                && resource.contains("r.crash.1")
                && journal_count(&plan) == 0;
        } else {
            point_ok = point_ok && snapshot(&plan) == before;
        }
        // The plan stays writable after any recovery.
        let follow_up = harness.run(
            &[
                "apply",
                &batch_url.to_string_lossy(),
                "--plan",
                &plan.to_string_lossy(),
                "--json",
            ],
            &[],
            None,
        );
        point_ok = point_ok && follow_up.status == 0;

        if point_ok {
            details.push(format!("{}=✓", point.name()));
        } else {
            all_ok = false;
            details.push(format!(
                "{}=bad(rec={} follow={})",
                point.name(),
                recovered.status,
                follow_up.status
            ));
        }
        let _ = std::fs::remove_file(&batch_url);
        let _ = std::fs::remove_dir_all(&plan);
    }

    check(
        "tx-crash-boundaries",
        all_ok,
        if all_ok {
            "5 boundaries: SIGKILL → recovery → complete before/after state".to_string()
        } else {
            details.join(" · ")
        },
    )
}

/// A reader that cannot open the lock — or an unwritable plan root — exits 4 and
/// never panics (§4.9).
fn readonly_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("tx-readonly", false, error),
    };
    let batch_url = plan.parent().unwrap_or(Path::new(".")).join("ro.jsonl");
    if let Err(error) = write_file(&batch_url, &topic_line("t.ro.1", "RO")) {
        return check("tx-readonly", false, error);
    }

    // On Unix the gate is the real thing: a read-only plan directory. Elsewhere
    // there is no directory-permission equivalent, so the portable stand-in is
    // a directory where the lock file must be, which fails the same open.
    #[cfg(unix)]
    let restore = {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = match std::fs::metadata(&plan) {
            Ok(metadata) => metadata.permissions(),
            Err(error) => return check("tx-readonly", false, error.to_string()),
        };
        let original = permissions.clone();
        permissions.set_mode(0o500);
        if let Err(error) = std::fs::set_permissions(&plan, permissions) {
            return check("tx-readonly", false, error.to_string());
        }
        original
    };
    #[cfg(not(unix))]
    let restore = {
        std::fs::create_dir_all(plan.join(".sam/lock")).ok();
        ()
    };

    let outcome = harness.run(
        &[
            "apply",
            &batch_url.to_string_lossy(),
            "--plan",
            &plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&plan, std::fs::Permissions::from_mode(restore.mode()));
    }

    let _ = std::fs::remove_file(&batch_url);
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "tx-readonly",
        outcome.status == 4,
        if outcome.status == 4 {
            if cfg!(unix) {
                "read-only plan → exit 4".to_string()
            } else {
                "unopenable lock → exit 4".to_string()
            }
        } else {
            format!("status {}", outcome.status)
        },
    )
}

/// An external edit during a draft produces the conflict report, never a silent
/// clobber (§6 gate, §4.6 step 4). In-process, because the edit must land
/// between the load and the commit.
fn external_conflict_check(_harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("tx-external-conflict", false, error),
    };
    let lock = match PlanLock::acquire_write(&plan, crate::plan_lock::WRITE_RETRY) {
        Ok(lock) => lock,
        Err(error) => return check("tx-external-conflict", false, error.to_string()),
    };
    if let Err(error) = transaction::recover(&plan) {
        return check("tx-external-conflict", false, error.to_string());
    }
    let config = match config_store::load_with(&plan, "explicit", resources_dir) {
        Ok(config) => config,
        Err(error) => return check("tx-external-conflict", false, error.to_string()),
    };

    // The external edit lands on a file THIS transaction affects — that is the
    // clobber the draft window must refuse.
    let topic_file = plan.join("content/records/topic.jsonl");
    let external = format!(
        "{}{}\n",
        read_to_string(&topic_file),
        topic_line("t.ext.1", "External during draft")
    );
    if let Err(error) = write_file(&topic_file, &external) {
        return check("tx-external-conflict", false, error);
    }

    let batch = format!("{}\n", topic_line("t.aff.1", "Affected"));
    let planned = match apply::plan(&config, batch.as_bytes(), "batch") {
        Ok(planned) => planned,
        Err(error) => return check("tx-external-conflict", false, error.to_string()),
    };
    let conflicted = matches!(
        apply::commit(&plan, &planned),
        Err(TransactionError::ExternalConflict(_))
    );
    let after = read_to_string(&topic_file);
    let ok = conflicted && after == external;

    drop(lock);
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "tx-external-conflict",
        ok,
        if ok {
            "external bytes intact; commit refused with a conflict".to_string()
        } else {
            format!("conflicted={conflicted} clobbered={}", after != external)
        },
    )
}

/// Backups: retention (default 20, configurable) plus a previewed restore that
/// round-trips to the recorded before-state (§4.6 step 4).
fn backups_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("tx-backups", false, error),
    };
    let dir = plan.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut snapshots: BTreeMap<String, BTreeMap<String, Vec<u8>>> = BTreeMap::new();

    let apply_n = |n: usize| -> Result<ProcOutcome, String> {
        let batch = dir.join(format!("ret{n}.jsonl"));
        write_file(
            &batch,
            &topic_line(&format!("t.ret.{n}"), &format!("Ret {n}")),
        )?;
        Ok(harness.run(
            &[
                "apply",
                &batch.to_string_lossy(),
                "--plan",
                &plan.to_string_lossy(),
                "--json",
            ],
            &[],
            None,
        ))
    };

    for n in 1..=3 {
        match apply_n(n) {
            Ok(outcome) if outcome.status == 0 => {
                snapshots.insert(format!("after{n}"), snapshot(&plan));
            }
            Ok(outcome) => {
                return check(
                    "tx-backups",
                    false,
                    format!("retention apply {n} failed: {}", outcome.status),
                );
            }
            Err(error) => return check("tx-backups", false, error),
        }
    }

    let list = harness.run(
        &["tx.list", "--plan", &plan.to_string_lossy(), "--json"],
        &[],
        None,
    );
    let first = envelope(&list.out);
    let first_count = first
        .as_ref()
        .and_then(|value| value.pointer("/data/backups"))
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let first_retention = first
        .as_ref()
        .and_then(|value| value.pointer("/data/retention"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(999);

    // Configure retention to 2 and add two more transactions.
    if let Err(error) = write_file(&plan.join(".sam/settings.json"), "{\"backupRetention\":2}") {
        return check("tx-backups", false, error);
    }
    for n in 4..=5 {
        match apply_n(n) {
            Ok(outcome) if outcome.status == 0 => {}
            Ok(outcome) => {
                return check(
                    "tx-backups",
                    false,
                    format!("retention apply {n} failed: {}", outcome.status),
                );
            }
            Err(error) => return check("tx-backups", false, error),
        }
    }

    let list2 = harness.run(
        &["tx.list", "--plan", &plan.to_string_lossy(), "--json"],
        &[],
        None,
    );
    let second = envelope(&list2.out);
    let txids: Vec<String> = second
        .as_ref()
        .and_then(|value| value.pointer("/data/backups"))
        .and_then(serde_json::Value::as_array)
        .map(|backups| {
            backups
                .iter()
                .filter_map(|backup| {
                    backup
                        .get("txid")
                        .and_then(|t| t.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    let Some(target) = txids.last().cloned() else {
        return check("tx-backups", false, "no retained backups to restore");
    };

    let before_restore = snapshot(&plan);
    let dry = harness.run(
        &[
            "tx.restore",
            &target,
            "--plan",
            &plan.to_string_lossy(),
            "--dry-run",
            "--json",
        ],
        &[],
        None,
    );
    let dry_ok = dry.status == 0 && snapshot(&plan) == before_restore;
    let real = harness.run(
        &[
            "tx.restore",
            &target,
            "--plan",
            &plan.to_string_lossy(),
            "--json",
        ],
        &[],
        None,
    );
    // The oldest retained backup is transaction 4, so its before-state is the
    // state after transaction 3.
    let restored = snapshot(&plan) == snapshots["after3"];

    let ok = first_retention == 20
        && first_count == 3
        && txids.len() == 2
        && dry_ok
        && real.status == 0
        && restored;
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "tx-backups",
        ok,
        if ok {
            "default 20 → configurable 2 prunes; restore round-trips to the recorded before-state"
                .to_string()
        } else {
            format!(
                "list1={first_count} retention={first_retention} txids={} dry={dry_ok} real={} restored={restored}",
                txids.len(),
                real.status
            )
        },
    )
}

/// `plan.new` materializes a validating writable copy; duplicates fail (§4.1).
fn plan_new_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let dir = std::env::temp_dir().join(unique("sam-p2-new"));
    let target = dir.join("fresh");
    let target_arg = target.to_string_lossy().to_string();

    let created = harness.run(&["plan.new", "--plan", &target_arg, "--json"], &[], None);
    let checked = harness.run(
        &["--configcheck", "--plan", &target_arg, "--json"],
        &[],
        None,
    );
    let duplicate = harness.run(&["plan.new", "--plan", &target_arg, "--json"], &[], None);
    let both = harness.run(
        &[
            "plan.new",
            "--plan",
            &dir.join("x").to_string_lossy(),
            "--name",
            "y",
            "--json",
        ],
        &[],
        None,
    );
    let lock_exists = target.join(".sam/lock").is_file();
    let ok = created.status == 0
        && checked.status == 0
        && duplicate.status == 4
        && both.status == 2
        && lock_exists;
    let _ = std::fs::remove_dir_all(&dir);
    check(
        "plan-new",
        ok,
        if ok {
            "copy validates clean · duplicate target → exit 4 · --plan+--name → exit 2".to_string()
        } else {
            format!(
                "new={} check={} dup={} both={} lock={lock_exists} resources={}",
                created.status,
                checked.status,
                duplicate.status,
                both.status,
                resources_dir.display()
            )
        },
    )
}

/// The registry enumerates every command with its effect, and a write with no
/// batch source is a usage error rather than a lock attempt (§4.9).
fn registry_check(harness: &Harness) -> CheckResult {
    let listed = harness.run(&["--commands", "--json"], &[], None);
    let env = envelope(&listed.out);
    let commands = env
        .as_ref()
        .and_then(|value| value.pointer("/data/commands"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let by_id: BTreeMap<String, serde_json::Value> = commands
        .iter()
        .filter_map(|command| {
            command
                .get("id")
                .and_then(|id| id.as_str())
                .map(|id| (id.to_string(), command.clone()))
        })
        .collect();

    let required = [
        "commands",
        "selfcheck",
        "tokens",
        "rendercheck",
        "configcheck",
        "paths",
        "plan.new",
        "apply",
        "tx.list",
        "tx.restore",
    ];
    let effect = |id: &str| -> String {
        by_id
            .get(id)
            .and_then(|def| def.get("effect"))
            .and_then(|e| e.as_str())
            .unwrap_or("-")
            .to_string()
    };
    let usage = harness.run(&["apply", "--plan", "/tmp"], &[], None);
    let ok = listed.status == 0
        && required.iter().all(|id| by_id.contains_key(*id))
        && effect("apply") == "write"
        && effect("configcheck") == "read"
        && usage.status == 2;

    // A registered write must have a handler, or `--commands` advertises a door
    // that does not open.
    let declared: Vec<String> = by_id
        .iter()
        .filter(|(_, def)| def.get("effect").and_then(|e| e.as_str()) == Some("write"))
        .map(|(id, _)| id.clone())
        .collect();
    let implemented: Vec<String> = command_registry::registered_write_ids()
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut declared_sorted = declared.clone();
    let mut implemented_sorted = implemented.clone();
    declared_sorted.sort();
    implemented_sorted.sort();
    let wired = declared_sorted == implemented_sorted;

    check(
        "registry",
        ok && wired,
        if ok && wired {
            format!(
                "{} commands registered · apply[write] · missing batch source → exit 2",
                by_id.len()
            )
        } else {
            format!(
                "status {} · {} commands · declared={declared_sorted:?} implemented={implemented_sorted:?}",
                listed.status,
                by_id.len()
            )
        },
    )
}

/// The §4.7 agent loop end to end: `paths` → dry-run → `apply --if-revision` →
/// retry (idempotent). Stdin (`-`) covers the piped path.
fn agent_loop_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("agent-loop", false, error),
    };
    let revision = harness.revision_of(&plan);
    if revision.is_empty() {
        return check("agent-loop", false, "the plan has no revision");
    }
    let batch = format!(
        "{}\n{}\n",
        topic_line("t.agent.1", "Agent A"),
        topic_line("t.agent.2", "Agent B")
    );
    let before = snapshot(&plan);
    let plan_arg = plan.to_string_lossy().to_string();
    let batch_bytes = batch.as_bytes();

    let dry = harness.run(
        &[
            "apply",
            "-",
            "--plan",
            &plan_arg,
            "--dry-run",
            "--if-revision",
            &revision,
            "--json",
        ],
        &[],
        Some(batch_bytes),
    );
    let dry_env = envelope(&dry.out);
    let after_dry = snapshot(&plan);
    let commit = harness.run(
        &[
            "apply",
            "-",
            "--plan",
            &plan_arg,
            "--if-revision",
            &revision,
            "--json",
        ],
        &[],
        Some(batch_bytes),
    );
    let commit_env = envelope(&commit.out);
    let retry = harness.run(
        &["apply", "-", "--plan", &plan_arg, "--json"],
        &[],
        Some(batch_bytes),
    );
    let after_commit = snapshot(&plan);
    let retry_env = envelope(&retry.out);

    let revision_field = |env: &Option<serde_json::Value>| -> String {
        env.as_ref()
            .and_then(|value| value.get("revision").and_then(|r| r.as_str()))
            .unwrap_or_default()
            .to_string()
    };
    let would_change = dry_env
        .as_ref()
        .and_then(|value| value.pointer("/data/wouldChange"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let ok = dry.status == 0
        && revision_field(&dry_env) == revision
        && after_dry == before
        && would_change
        && commit.status == 0
        && revision_field(&commit_env) != revision
        && retry.status == 0
        && revision_field(&retry_env) == revision_field(&commit_env)
        && snapshot(&plan) == after_commit;

    let _ = std::fs::remove_dir_all(&plan);
    check(
        "agent-loop",
        ok,
        if ok {
            "candidate on stdin → dry-run (no writes) → commit at pinned revision → retry no-op"
                .to_string()
        } else {
            format!(
                "dry={} (revision match={}) commit={} retry={} wouldChange={would_change}",
                dry.status,
                revision_field(&dry_env) == revision,
                commit.status,
                retry.status
            )
        },
    )
}

// ── Phase 3: expressions and headless queries (§6 gate) ─────────────────────

/// The frozen clock the L2 checks evaluate at. It is mid-day in UTC so a civil
/// date is unambiguous in any offset this evaluator resolves (§4.4: freeze the
/// clock and timezone in tests).
const FROZEN_NOW: i64 = 1_768_924_800; // 2026-01-20, 16:00 UTC

fn eval_l2(
    text: &str,
    config: &ResolvedConfig,
    record: Option<&Record>,
    budget: usize,
) -> Result<L2Value, crate::diagnostic::DiagnosticError> {
    let expr = expression::parse(text, "selfcheck")?;
    let context = EvalContext::frozen(config, None, FROZEN_NOW);
    let mut evaluation = Evaluation::new(context, budget);
    evaluation.evaluate(&expr, record)
}

fn phase_three_checks(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    let config = match load_seed(resources_dir) {
        Ok(config) => config,
        Err(error) => return vec![check("l2-precedence", false, error.to_string())],
    };
    vec![
        l2_precedence_check(&config),
        l2_null_check(&config),
        l2_division_check(&config),
        l2_short_circuit_check(&config),
        l2_dates_check(&config),
        l2_limits_check(&config),
        l2_cycle_check(resources_dir),
        view_resolve_check(resources_dir, &config),
        view_determinism_check(harness, resources_dir),
        schema_guide_check(harness, resources_dir, &config),
    ]
}

fn l2_precedence_check(config: &ResolvedConfig) -> CheckResult {
    let Some(topic) = config.record("t.demo.01") else {
        return check("l2-precedence", false, "t.demo.01 is missing from the seed");
    };
    let cases: [(&str, L2Value); 8] = [
        ("2 + 3 * 4 == 14", L2Value::Bool(true)),
        ("(2 + 3) * 4 == 20", L2Value::Bool(true)),
        ("10 % 3 == 1", L2Value::Bool(true)),
        ("2 - -3 == 5", L2Value::Bool(true)),
        ("!false && 1 < 2", L2Value::Bool(true)),
        ("false ? 1 : 2 == 2", L2Value::Bool(true)),
        ("1 + 2 == 3 && 4 > 2", L2Value::Bool(true)),
        ("'math' == 'math' && 'a' < 'b'", L2Value::Bool(true)),
    ];
    let all = cases.iter().all(|(text, want)| {
        eval_l2(text, config, Some(topic), MAX_EVALUATION_STEPS).ok() == Some(want.clone())
    });
    check(
        "l2-precedence",
        all,
        if all {
            format!("{} expression fixtures", cases.len())
        } else {
            "a precedence case diverged".to_string()
        },
    )
}

fn l2_null_check(config: &ResolvedConfig) -> CheckResult {
    let Some(topic) = config.record("t.demo.01") else {
        return check(
            "l2-null-semantics",
            false,
            "t.demo.01 is missing from the seed",
        );
    };
    // "part" is a DECLARED field absent from t.demo.02 — the runtime meaning of
    // missing (an undeclared key is a static error).
    let missing = config
        .record("t.demo.02")
        .and_then(|record| eval_l2("part + 1", config, Some(record), MAX_EVALUATION_STEPS).ok());
    let cases: [(&str, L2Value); 8] = [
        ("est + null == null", L2Value::Bool(true)),
        ("null == 0", L2Value::Bool(false)),
        ("null != 0", L2Value::Bool(true)),
        ("est > null", L2Value::Null),
        ("null && false", L2Value::Bool(false)),
        ("null && true", L2Value::Null),
        ("null || true", L2Value::Bool(true)),
        ("null || false", L2Value::Null),
    ];
    let mut all = missing == Some(L2Value::Null);
    for (text, want) in cases {
        if eval_l2(text, config, Some(topic), MAX_EVALUATION_STEPS).ok() != Some(want) {
            all = false;
        }
    }
    check(
        "l2-null-semantics",
        all,
        if all {
            "null ≠ 0/false · Kleene && ||".to_string()
        } else {
            "a null case diverged".to_string()
        },
    )
}

fn l2_division_check(config: &ResolvedConfig) -> CheckResult {
    let Some(topic) = config.record("t.demo.01") else {
        return check("l2-division", false, "t.demo.01 is missing from the seed");
    };
    let divide = eval_l2("1 / 0", config, Some(topic), MAX_EVALUATION_STEPS);
    let half = eval_l2("5 / 2 == 2.5", config, Some(topic), MAX_EVALUATION_STEPS).ok();
    let zero_n = eval_l2("0 / 5 == 0", config, Some(topic), MAX_EVALUATION_STEPS).ok();
    let pct = eval_l2(
        "pct(1, 0) == null",
        config,
        Some(topic),
        MAX_EVALUATION_STEPS,
    )
    .ok();
    let divide_code = divide.err().map(|error| error.diagnostic.code);
    check(
        "l2-division",
        divide_code.as_deref() == Some("expr.divide-by-zero")
            && half == Some(L2Value::Bool(true))
            && zero_n == Some(L2Value::Bool(true))
            && pct == Some(L2Value::Bool(true)),
        "1/0 → error+null · 5/2 → 2.5 · pct(1,0) → null",
    )
}

fn l2_short_circuit_check(config: &ResolvedConfig) -> CheckResult {
    let Some(topic) = config.record("t.demo.01") else {
        return check(
            "l2-short-circuit",
            false,
            "t.demo.01 is missing from the seed",
        );
    };
    let and_case = eval_l2(
        "false && (1 / 0 == 1)",
        config,
        Some(topic),
        MAX_EVALUATION_STEPS,
    )
    .ok();
    let or_case = eval_l2(
        "true || (1 / 0 == 1)",
        config,
        Some(topic),
        MAX_EVALUATION_STEPS,
    )
    .ok();
    check(
        "l2-short-circuit",
        and_case == Some(L2Value::Bool(false)) && or_case == Some(L2Value::Bool(true)),
        "false && (1/0) → false · true || (1/0) → true — right side never ran",
    )
}

fn l2_dates_check(config: &ResolvedConfig) -> CheckResult {
    let Some(topic) = config.record("t.demo.01") else {
        return check("l2-dates", false, "t.demo.01 is missing from the seed");
    };
    let cases: [(&str, L2Value); 8] = [
        ("today() == '2026-01-20'", L2Value::Bool(true)),
        (
            "daysBetween('2026-01-20', '2026-01-11') == 9",
            L2Value::Bool(true),
        ),
        (
            "daysBetween('2026-01-11', '2026-01-20') == -9",
            L2Value::Bool(true),
        ),
        ("weekOf('2026-01-20') == 1", L2Value::Bool(true)),
        ("weekOf('2026-01-27') == 2", L2Value::Bool(true)),
        ("weekOf('2026-03-01') == null", L2Value::Bool(true)),
        ("currentTerm() == 'term.demo'", L2Value::Bool(true)),
        ("week.term.id == currentTerm()", L2Value::Bool(true)),
    ];
    let all = cases.iter().all(|(text, want)| {
        eval_l2(text, config, Some(topic), MAX_EVALUATION_STEPS).ok() == Some(want.clone())
    });
    check(
        "l2-dates",
        all,
        if all {
            "today/daysBetween/weekOf/currentTerm at the frozen clock".to_string()
        } else {
            "a date case diverged".to_string()
        },
    )
}

fn l2_limits_check(config: &ResolvedConfig) -> CheckResult {
    let Some(topic) = config.record("t.demo.01") else {
        return check("l2-limits", false, "t.demo.01 is missing from the seed");
    };
    let mut parts: Vec<String> = Vec::new();

    let long = format!("{}1", "1 + ".repeat(128)); // 513 characters
    if long.chars().count() != 513 {
        parts.push(format!("fixture is {} chars", long.chars().count()));
    } else {
        match expression::parse(&long, "t") {
            Err(error) if error.diagnostic.code == "expr.too-long" => {}
            Err(error) => parts.push(format!("tooLong={}", error.diagnostic.code)),
            Ok(_) => parts.push("tooLong=no error".into()),
        }
    }
    let deep = format!("{}1{}", "(".repeat(33), ")".repeat(33));
    match expression::parse(&deep, "t") {
        Err(error) if error.diagnostic.code == "expr.too-deep" => {}
        Err(error) => parts.push(format!("tooDeep={}", error.diagnostic.code)),
        Ok(_) => parts.push("tooDeep=no error".into()),
    }
    let segments = format!("{}x", "x.".repeat(9));
    match expression::parse(&segments, "t") {
        Err(error) if error.diagnostic.code == "expr.path-too-long" => {}
        Err(error) => parts.push(format!("segs={}", error.diagnostic.code)),
        Ok(_) => parts.push("segs=no error".into()),
    }
    let budget = eval_l2("1 + 2 * 3", config, Some(topic), 3)
        .err()
        .map(|error| error.diagnostic.code);
    if budget.as_deref() != Some("expr.limit") {
        parts.push("budget missed".into());
    }
    let hops = eval_l2(
        "course.topic.course.topic.course.topic.est",
        config,
        Some(topic),
        MAX_EVALUATION_STEPS,
    )
    .err()
    .map(|error| error.diagnostic.code);
    if hops.as_deref() != Some("expr.limit") {
        parts.push("hops missed".into());
    }

    check(
        "l2-limits",
        parts.is_empty(),
        if parts.is_empty() {
            "513-char / 33-deep / 9-segment rejected · step budget + relation-hop cap hit"
                .to_string()
        } else {
            parts.join(" · ")
        },
    )
}

/// A formula dependency cycle through RELATIONS is rejected at validation (the
/// bundled fixture covers same-type bare refs).
fn l2_cycle_check(resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("l2-cycle", false, error),
    };
    let cycle = r#"{"schemaVersion":1,"types":{
      "xa":{"fields":[{"key":"n","type":"number"},{"key":"f","type":"formula","expr":"clamp(xb.f, 0, 100)"},{"key":"xb","type":"relation","to":"xb"}]},
      "xb":{"fields":[{"key":"f","type":"formula","expr":"clamp(xa.f, 0, 100)"},{"key":"xa","type":"relation","to":"xa"}]}
    }}"#;
    let found = match write_file(&plan.join("content/types.json"), cycle) {
        Ok(()) => match config_store::load_with(&plan, "explicit", resources_dir) {
            Err(ConfigStoreError::Validation(diagnostics)) => diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "formula.cycle"),
            _ => false,
        },
        Err(_) => false,
    };
    let _ = std::fs::remove_dir_all(&plan);
    check(
        "l2-cycle",
        found,
        if found {
            "cross-type formula cycle rejected at load".to_string()
        } else {
            "cycle NOT rejected".to_string()
        },
    )
}

fn view_resolve_check(resources_dir: &Path, config: &ResolvedConfig) -> CheckResult {
    let frozen = EvalContext {
        config,
        state: None,
        now_unix: FROZEN_NOW,
        tz_offset_minutes: 0,
    };
    let today = match views::resolve(config, "today.topics", &frozen) {
        Ok(outcome) => outcome,
        Err(error) => return check("view-resolve", false, error.to_string()),
    };
    let order: Vec<&str> = today
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect();
    let group_keys = |outcome: &views::ViewOutcome| -> Vec<Option<String>> {
        outcome
            .groups
            .as_ref()
            .map(|groups| groups.iter().map(|group| group.key.clone()).collect())
            .unwrap_or_default()
    };
    let mut ok = order == ["t.demo.01", "t.demo.02", "t.demo.04", "t.demo.03"]
        && group_keys(&today)
            == vec![
                Some("practice".into()),
                Some("read".into()),
                Some("revise".into()),
                Some("watch".into()),
            ]
        && today.candidates == 4;
    ok = ok
        && views::resolve(config, "courses.board", &frozen)
            .is_ok_and(|board| board.records.len() == 2);
    let math = config.record("c.demo.a");
    ok =
        ok && math.is_some_and(|record| {
            eval_l2("count(topic)", config, Some(record), MAX_EVALUATION_STEPS).ok()
                == Some(L2Value::Int(2))
        }) && math.is_some_and(|record| {
            eval_l2("sum(topic.est)", config, Some(record), MAX_EVALUATION_STEPS).ok()
                == Some(L2Value::Int(75))
        });

    // state: marking t.demo.01 complete removes it (complete())
    match temp_plan(resources_dir) {
        Ok(plan) => {
            let state_dir = plan.join("state");
            if std::fs::create_dir_all(&state_dir).is_err()
                || write_file(
                    &state_dir.join("state.json"),
                    r#"{"schemaVersion":1,"progress":{"t.demo.01":{"pipeline":"flip","stages":{"anchored":"2026-01-15T10:00:00Z"}}}}"#,
                )
                .is_err()
            {
                let _ = std::fs::remove_dir_all(&plan);
                return check("view-resolve", false, "cannot stage state/state.json");
            }
            let state = PlanState::load(&plan);
            match config_store::load_with(&plan, "explicit", resources_dir) {
                Ok(with_state) => {
                    let context = EvalContext {
                        config: &with_state,
                        state: Some(&state),
                        now_unix: FROZEN_NOW,
                        tz_offset_minutes: 0,
                    };
                    match views::resolve(&with_state, "today.topics", &context) {
                        Ok(filtered) => {
                            let ids: Vec<&str> = filtered
                                .records
                                .iter()
                                .map(|record| record.id.as_str())
                                .collect();
                            ok = ok
                                && ids == ["t.demo.02", "t.demo.04", "t.demo.03"]
                                && group_keys(&filtered)
                                    == vec![
                                        Some("practice".into()),
                                        Some("read".into()),
                                        Some("revise".into()),
                                    ];
                        }
                        Err(_) => ok = false,
                    }
                    // limit: cap today.topics at 2
                    let views_path = plan.join("content/views.json");
                    let limited_text =
                        read_to_string(&views_path).replace("\"limit\": 12", "\"limit\": 2");
                    let _ = write_file(&views_path, &limited_text);
                    match config_store::load_with(&plan, "explicit", resources_dir) {
                        Ok(limited) => {
                            let context = EvalContext {
                                config: &limited,
                                state: Some(&state),
                                now_unix: FROZEN_NOW,
                                tz_offset_minutes: 0,
                            };
                            match views::resolve(&limited, "today.topics", &context) {
                                Ok(capped) => {
                                    let ids: Vec<&str> = capped
                                        .records
                                        .iter()
                                        .map(|record| record.id.as_str())
                                        .collect();
                                    ok = ok && ids == ["t.demo.02", "t.demo.04"];
                                }
                                Err(_) => ok = false,
                            }
                        }
                        Err(_) => ok = false,
                    }
                }
                Err(_) => ok = false,
            }
            let _ = std::fs::remove_dir_all(&plan);
        }
        Err(_) => ok = false,
    }

    check(
        "view-resolve",
        ok,
        if ok {
            "filter/sort/group/limit · complete() honors state · count(topic)/sum(topic.est)"
                .to_string()
        } else {
            "view semantics diverged".to_string()
        },
    )
}

fn view_determinism_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("view-determinism", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();
    let first = harness.run(
        &["view", "today.topics", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let second = harness.run(
        &["view", "today.topics", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );

    let fixture = resources_dir.join("fixtures/problemset");
    let fixture_arg = fixture.to_string_lossy().to_string();
    let problems = harness.run(
        &["view", "math.problems", "--plan", &fixture_arg, "--json"],
        &[],
        None,
    );
    let ids: Vec<String> = envelope(&problems.out)
        .as_ref()
        .and_then(|value| value.pointer("/data/records"))
        .and_then(serde_json::Value::as_array)
        .map(|records| {
            records
                .iter()
                .filter_map(|record| {
                    record
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();

    let unknown = harness.run(&["view", "nope", "--plan", &plan_arg, "--json"], &[], None);
    let _ = std::fs::remove_dir_all(&plan);

    let ok = first.status == 0
        && second.status == 0
        && !first.out.is_empty()
        && first.out == second.out
        && problems.status == 0
        && ids == ["p.dp.01", "p.dp.02"]
        && unknown.status == 1;
    check(
        "view-determinism",
        ok,
        format!(
            "two runs byte-identical · math.problems → {ids:?} · unknown view → exit {}",
            unknown.status
        ),
    )
}

fn schema_guide_check(
    harness: &Harness,
    resources_dir: &Path,
    config: &ResolvedConfig,
) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("schema-guide", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();

    let schema = harness.run(&["--schema", "--plan", &plan_arg, "--json"], &[], None);
    let schema_value = envelope(&schema.out);
    let types = schema_value
        .as_ref()
        .and_then(|value| value.pointer("/data/types"));
    let kinds: Vec<String> = types
        .and_then(|types| types.get("topic"))
        .and_then(|topic| topic.get("fields"))
        .and_then(serde_json::Value::as_array)
        .and_then(|fields| {
            fields
                .iter()
                .find(|field| field.get("key").and_then(serde_json::Value::as_str) == Some("kind"))
        })
        .and_then(|field| field.get("options"))
        .and_then(serde_json::Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| option.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let limits_step = schema_value
        .as_ref()
        .and_then(|value| value.pointer("/data/limits/maxEvaluationSteps"))
        .and_then(serde_json::Value::as_u64);
    let schema_ok = schema.status == 0
        && types
            .and_then(serde_json::Value::as_object)
            .map(|map| map.len())
            == Some(12)
        && kinds == ["watch", "practice", "read", "write", "project", "revise"]
        && limits_step == Some(MAX_EVALUATION_STEPS as u64);

    let guide = harness.run(&["--guide", "--plan", &plan_arg, "--json"], &[], None);
    let guide_value = envelope(&guide.out);
    let lines: Vec<String> = guide_value
        .as_ref()
        .and_then(|value| value.pointer("/data/examples"))
        .and_then(serde_json::Value::as_array)
        .map(|examples| {
            examples
                .iter()
                .filter_map(|example| example.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut guide_ok = guide.status == 0 && lines.len() == 12;
    let mut detail = String::new();
    if guide_ok {
        // Examples validate through the loader (whole-plan), and their links
        // close within the example set itself.
        let batch = format!("{}\n", lines.join("\n"));
        match apply::plan(config, batch.as_bytes(), "guide-examples") {
            Ok(planned) => {
                let mut ids = BTreeSet::new();
                for line in &lines {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
                            ids.insert(id.to_string());
                        }
                    }
                }
                let mut closed = true;
                for line in &lines {
                    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                        continue;
                    };
                    let Some(links) = value.get("links").and_then(serde_json::Value::as_object)
                    else {
                        continue;
                    };
                    for targets in links.values() {
                        if let Some(targets) = targets.as_array() {
                            for target in targets {
                                if !ids.contains(target.as_str().unwrap_or_default()) {
                                    closed = false;
                                }
                            }
                        }
                    }
                }
                guide_ok = closed && planned.counts.added == 12;
                detail = format!(
                    "{} · {} examples",
                    if closed {
                        "links closed"
                    } else {
                        "links not closed"
                    },
                    planned.counts.added
                );
            }
            Err(error) => {
                guide_ok = false;
                detail = format!("examples failed validation: {error}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&plan);

    let ok = schema_ok && guide_ok;
    check(
        "schema-guide",
        ok,
        format!(
            "schema: 12 types, kind options, limits · guide: {}",
            if detail.is_empty() {
                "examples"
            } else {
                &detail
            }
        ),
    )
}

// ── Phase 5: views, screens and the block renderer's data ───────────────────

fn phase_five_checks(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    vec![
        screen_blocks_check(resources_dir),
        screen_placeholder_check(harness, resources_dir),
        screen_depth_check(harness, resources_dir),
        view_write_check(harness, resources_dir),
        expr_preview_check(harness, resources_dir),
    ]
}

/// The seed's screens resolve with the **engine** doing the arithmetic: a stat
/// is a fold over a view's records, and a record block carries the projection a
/// table draws.
fn screen_blocks_check(resources_dir: &Path) -> CheckResult {
    let config = match load_seed(resources_dir) {
        Ok(config) => config,
        Err(error) => return check("screen-blocks", false, error.to_string()),
    };
    let frozen = EvalContext::frozen(&config, None, FROZEN_NOW);
    let outcome = match views::resolve(&config, "today.screen", &frozen) {
        Ok(outcome) => outcome,
        Err(error) => return check("screen-blocks", false, error.to_string()),
    };
    let kinds: Vec<&str> = outcome
        .blocks
        .iter()
        .map(|block| block["kind"].as_str().unwrap_or("?"))
        .collect();
    let open = outcome.blocks[0]["value"].as_i64().unwrap_or(-1);
    let minutes = outcome.blocks[1]["value"].as_i64().unwrap_or(-1);
    let planned: i64 = outcome
        .records
        .iter()
        .filter_map(|record| record.fields.get("est"))
        .filter_map(|value| value.as_i64())
        .sum();
    let ok = kinds == ["stat", "stat", "list"]
        && open == outcome.records.len() as i64
        && minutes == planned
        && outcome.blocks[2]["records"].as_array().map(Vec::len) == Some(outcome.records.len());
    check(
        "screen-blocks",
        ok,
        format!("{kinds:?} · {open} open · {minutes} planned minutes"),
    )
}

/// A kind this build does not know must **load** and draw as a stated
/// placeholder — the fixture's bytes, through the shipped binary (§0 rule 3).
fn screen_placeholder_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let fixture = resources_dir.join("fixtures/screen");
    let fixture_arg = fixture.to_string_lossy().to_string();
    let check_run = harness.run(
        &["--configcheck", "--plan", &fixture_arg, "--json"],
        &[],
        None,
    );
    let resolved = harness.run(
        &["view", "screen.demo", "--plan", &fixture_arg, "--json"],
        &[],
        None,
    );
    let value = envelope(&resolved.out);
    let placeholder = value
        .as_ref()
        .and_then(|value| value.pointer("/data/blocks/2/blocks/0"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    // The unknown kind is a **load** finding (§3.7: warn at load, draw a
    // placeholder at render), so it is reported by `--configcheck`.
    let warned = envelope(&check_run.out)
        .and_then(|value| {
            value
                .pointer("/warnings")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
        .iter()
        .any(|warning| {
            warning.get("code").and_then(serde_json::Value::as_str) == Some("view.unknown-block")
        });
    let ok = check_run.status == 0
        && resolved.status == 0
        && placeholder["kind"] == serde_json::json!("placeholder")
        && placeholder["unknown"] == serde_json::json!("sparkline")
        && placeholder["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("unknown block: \"sparkline\" · known: "))
        && warned;
    check(
        "screen-placeholder",
        ok,
        format!(
            "configcheck exit {} · unknown kind → {} · warning {}",
            check_run.status,
            placeholder["reason"].as_str().unwrap_or("?"),
            if warned { "reported" } else { "missing" }
        ),
    )
}

/// Past the recursion boundary the renderer stops with a stated placeholder,
/// and the loader says so as a warning rather than refusing the plan.
fn screen_depth_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("screen-depth", false, error),
    };
    let mut nested = serde_json::json!({ "kind": "callout", "text": "deep" });
    for _ in 0..(crate::model::MAX_BLOCK_DEPTH + 1) {
        nested = serde_json::json!({ "kind": "columns", "blocks": [nested] });
    }
    // Added to the seed's own views, not written over them: the shell's
    // navigation names them, and a plan whose navigation dangles fails for a
    // reason that has nothing to do with block depth.
    let existing = read_to_string(&plan.join("content/views.json"));
    let mut document: serde_json::Value = serde_json::from_str(&existing)
        .unwrap_or(serde_json::json!({ "schemaVersion": 1, "views": {} }));
    if let Some(views) = document
        .get_mut("views")
        .and_then(|views| views.as_object_mut())
    {
        views.insert(
            "deep.screen".into(),
            serde_json::json!({
                "type": "topic",
                "layout": "list",
                "blocks": [nested],
            }),
        );
    }
    let written = write_file(
        &plan.join("content/views.json"),
        &format!("{}\n", serde_json::to_string_pretty(&document).unwrap()),
    );
    if let Err(error) = written {
        let _ = std::fs::remove_dir_all(&plan);
        return check("screen-depth", false, error);
    }
    let plan_arg = plan.to_string_lossy().to_string();
    let check_run = harness.run(&["--configcheck", "--plan", &plan_arg, "--json"], &[], None);
    let resolved = harness.run(
        &["view", "deep.screen", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let _ = std::fs::remove_dir_all(&plan);
    let warnings = envelope(&check_run.out)
        .and_then(|value| {
            value
                .pointer("/warnings")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default();
    let depth_warned = warnings.iter().any(|warning| {
        warning.get("code").and_then(serde_json::Value::as_str) == Some("view.block-depth")
    });
    let message = envelope(&resolved.out)
        .and_then(|value| {
            value
                .pointer("/data/blocks/0")
                .map(|node| node["kind"].as_str().unwrap_or("?").to_string())
        })
        .unwrap_or_default();
    let ok = check_run.status == 0 && depth_warned && resolved.status == 0;
    check(
        "screen-depth",
        ok,
        format!(
            "configcheck exit {} · depth {} · top block {message}",
            check_run.status,
            if depth_warned { "warned" } else { "silent" }
        ),
    )
}

/// The view and block writes, end to end through the binary: a block's layout,
/// a new block with its keys, a removal, a cleared key — and a refusal that
/// names why, with the plan still valid afterwards.
fn view_write_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("view-writes", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();
    let layout = harness.run(
        &[
            "view.setLayout",
            "today.screen",
            "--block",
            "1",
            "--layout",
            "board",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let added = harness.run(
        &[
            "view.block.add",
            "today.screen",
            "--kind",
            "callout",
            "--value",
            "title=Note",
            "--value",
            "text=Added by the self-check",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let removed = harness.run(
        &[
            "view.block.remove",
            "today.screen",
            "--index",
            "0",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let cleared = harness.run(
        &[
            "view.setFilter",
            "today.screen",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let refused = harness.run(
        &[
            "view.setLayout",
            "today.screen",
            "--block",
            "9",
            "--layout",
            "board",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let after = harness.run(
        &["view", "today.screen", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let check_run = harness.run(&["--configcheck", "--plan", &plan_arg, "--json"], &[], None);
    // Read the plan *before* the temp directory goes away: the cleared key is a
    // fact about the file, not about the resolved projection.
    let document = read_to_string(&plan.join("content/views.json"));
    let _ = std::fs::remove_dir_all(&plan);
    let parsed: serde_json::Value =
        serde_json::from_str(&document).unwrap_or(serde_json::Value::Null);
    let filter_gone = parsed
        .pointer("/views/today.screen")
        .is_some_and(|screen| screen.get("filter").is_none());

    let kinds: Vec<String> = envelope(&after.out)
        .and_then(|value| {
            value
                .pointer("/data/blocks")
                .and_then(serde_json::Value::as_array)
                .map(|blocks| {
                    blocks
                        .iter()
                        .filter_map(|block| block["kind"].as_str().map(str::to_string))
                        .collect()
                })
        })
        .unwrap_or_default();
    let ok = layout.status == 0
        && added.status == 0
        && removed.status == 0
        && cleared.status == 0
        // Naming a block that does not exist is a usage error (exit 2): the
        // write never reached the validator.
        && refused.status == 2
        && after.status == 0
        && check_run.status == 0
        && kinds == ["board", "list", "callout"]
        && filter_gone
        && refused.err.contains("out of range");
    check(
        "view-writes",
        ok,
        format!(
            "layout/block.add/block.remove/setFilter → exit {} · refused block 9 → exit {} · after {kinds:?}",
            layout.status, refused.status
        ),
    )
}

/// `expr.eval` is the engine's evaluator as a read — what a picker previews
/// with, and therefore the only thing a preview may trust (§4.4).
fn expr_preview_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("expr-preview", false, error),
    };
    let plan_arg = plan.to_string_lossy().to_string();
    let doubled = harness.run(
        &[
            "expr.eval",
            "est * 2",
            "--id",
            "t.demo.01",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let aggregate = harness.run(
        &[
            "expr.eval",
            "sum(topic.est)",
            "--id",
            "c.demo.a",
            "--plan",
            &plan_arg,
            "--json",
        ],
        &[],
        None,
    );
    let malformed = harness.run(
        &["expr.eval", "est +", "--plan", &plan_arg, "--json"],
        &[],
        None,
    );
    let _ = std::fs::remove_dir_all(&plan);

    let value = envelope(&doubled.out)
        .and_then(|envelope| envelope.pointer("/data/value").cloned())
        .unwrap_or(serde_json::Value::Null);
    let kind = envelope(&doubled.out)
        .and_then(|envelope| {
            envelope
                .pointer("/data/type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    let total = envelope(&aggregate.out)
        .and_then(|envelope| envelope.pointer("/data/value").cloned())
        .unwrap_or(serde_json::Value::Null);
    let ok = doubled.status == 0
        && value == serde_json::json!(60)
        && kind == "number"
        && aggregate.status == 0
        && total == serde_json::json!(75)
        && malformed.status == 2;
    check(
        "expr-preview",
        ok,
        format!(
            "est * 2 on t.demo.01 → {value} ({kind}) · sum(topic.est) on c.demo.a → {total} · malformed → exit {}",
            malformed.status
        ),
    )
}

// ── Phase 6: pipelines, schedulers and the derived index ────────────────────

fn phase_six_checks(harness: &Harness, resources_dir: &Path) -> Vec<CheckResult> {
    vec![
        pipeline_gates_check(harness, resources_dir),
        pipeline_switch_check(harness, resources_dir),
        scheduler_fixed_check(harness, resources_dir),
        scheduler_sm2_check(harness, resources_dir),
        scheduler_fsrs_check(harness, resources_dir),
        review_queue_check(resources_dir),
        metrics_fold_check(resources_dir),
        index_parity_check(harness, resources_dir),
        structure_ops_check(harness, resources_dir),
        guide_study_check(harness, resources_dir),
    ]
}

/// The first structured diagnostic on stderr — the CLI's error contract (§4.9).
fn error_code(text: &str) -> String {
    text.lines()
        .next()
        .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .and_then(|value| {
            value
                .get("code")
                .and_then(|code| code.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn edit_json(path: &Path, edit: impl FnOnce(&mut serde_json::Value)) -> Result<(), String> {
    let text = read_to_string(path);
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    edit(&mut value);
    let out = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    write_file(path, &format!("{out}\n"))
}

/// Gate 1 (§6 Phase 6): the flip loop as data — proof thresholds, an anchor's
/// two evidence kinds, backtracking invalidation, and a dry run that writes
/// nothing.
fn pipeline_gates_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("pipeline-gates", false, error),
    };
    let arg = plan.to_string_lossy().to_string();
    let step = |stage: &str, extra: &[&str]| {
        let mut args: Vec<&str> = vec![
            "record.advanceStage",
            "--id",
            "t.demo.01",
            "--stage",
            stage,
            "--plan",
            &arg,
            "--json",
        ];
        args.extend_from_slice(extra);
        harness.run(&args, &[], None)
    };

    let learned = step("learned", &[]);
    let bare = step("proved", &[]);
    let short = step("proved", &["--problems", "1"]);
    let proved = step(
        "proved",
        &["--problems", "3", "--signal", "expects a Bayes question"],
    );
    let anchor_bare = step("anchored", &[]);
    let after_proved = snapshot(&plan);
    let dry = step("anchored", &["--reason", "already knew it", "--dry-run"]);
    let dry_unchanged = snapshot(&plan) == after_proved;
    let skipped = step("anchored", &["--reason", "already knew it"]);
    let complete = envelope(&skipped.out)
        .and_then(|value| value.pointer("/data/complete").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    let back = step("proved", &[]);
    let invalidated: Vec<String> = envelope(&back.out)
        .and_then(|value| {
            value
                .pointer("/data/invalidated")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
        })
        .unwrap_or_default();

    let state = read_to_string(&plan.join("state/state.json"));
    let _ = std::fs::remove_dir_all(&plan);
    let parsed: serde_json::Value = serde_json::from_str(&state).unwrap_or(serde_json::Value::Null);
    let entry = parsed.pointer("/progress/t.demo.01");
    let mut stages: Vec<String> = entry
        .and_then(|entry| entry.pointer("/stages"))
        .and_then(|stages| stages.as_object())
        .map(|stages| stages.keys().cloned().collect())
        .unwrap_or_default();
    stages.sort();
    let problems = entry
        .and_then(|entry| entry.pointer("/data/problemsSolved"))
        .and_then(|value| value.as_i64())
        .unwrap_or(-1);
    let signal = entry
        .and_then(|entry| entry.pointer("/data/signal"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();

    let ok = learned.status == 0
        && bare.status == 1
        && error_code(&bare.err) == "pipeline.proof-required"
        && short.status == 1
        && error_code(&short.err) == "pipeline.proof-short"
        && proved.status == 0
        && anchor_bare.status == 1
        && error_code(&anchor_bare.err) == "pipeline.anchor-required"
        && dry.status == 0
        && dry_unchanged
        && skipped.status == 0
        && complete
        && back.status == 0
        && invalidated == ["anchored"]
        && stages == ["learned", "proved"]
        && problems == 3
        && signal == "expects a Bayes question";
    check(
        "pipeline-gates",
        ok,
        format!(
            "proof exit {}/{}/{} · anchor exit {}/{} · dry-run wrote {} · back invalidated {invalidated:?} · stages {stages:?}",
            learned.status,
            bare.status,
            proved.status,
            anchor_bare.status,
            skipped.status,
            if dry_unchanged { "nothing" } else { "bytes!" },
        ),
    )
}

/// §3.5's pipeline switch: a previewed mapping or a fresh state, and prior
/// stages preserved in `history` — never silently re-interpreted.
fn pipeline_switch_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let source = resources_dir.join("fixtures/phase6");
    let plan = std::env::temp_dir().join(unique("sam-p6-switch"));
    if let Err(error) = transaction::copy_tree(&source, &plan) {
        return check("pipeline-switch", false, error.to_string());
    }
    let arg = plan.to_string_lossy().to_string();
    let before = snapshot(&plan);
    let dry = harness.run(
        &[
            "type.setPipeline",
            "--type",
            "topic",
            "--pipeline",
            "check",
            "--map",
            "learned=done",
            "--map",
            "proved=done",
            "--map",
            "anchored=done",
            "--dry-run",
            "--plan",
            &arg,
            "--json",
        ],
        &[],
        None,
    );
    let dry_unchanged = snapshot(&plan) == before;
    let unmapped = harness.run(
        &[
            "type.setPipeline",
            "--type",
            "topic",
            "--pipeline",
            "check",
            "--plan",
            &arg,
            "--json",
        ],
        &[],
        None,
    );
    let commit = harness.run(
        &[
            "type.setPipeline",
            "--type",
            "topic",
            "--pipeline",
            "check",
            "--map",
            "learned=done",
            "--map",
            "proved=done",
            "--map",
            "anchored=done",
            "--plan",
            &arg,
            "--json",
        ],
        &[],
        None,
    );
    let validate = harness.run(&["--configcheck", "--plan", &arg, "--json"], &[], None);

    let types = read_to_string(&plan.join("content/types.json"));
    let state = read_to_string(&plan.join("state/state.json"));
    let _ = std::fs::remove_dir_all(&plan);
    let types: serde_json::Value = serde_json::from_str(&types).unwrap_or(serde_json::Value::Null);
    let state: serde_json::Value = serde_json::from_str(&state).unwrap_or(serde_json::Value::Null);
    let pipeline = types
        .pointer("/types/topic/pipeline")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let entry = state.pointer("/progress/t.bayes");
    let entry_pipeline = entry
        .and_then(|entry| entry.pointer("/pipeline"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let mut stages: Vec<String> = entry
        .and_then(|entry| entry.pointer("/stages"))
        .and_then(|stages| stages.as_object())
        .map(|stages| stages.keys().cloned().collect())
        .unwrap_or_default();
    stages.sort();
    let history_pipeline = state
        .pointer("/progress/t.bayes/history/0/pipeline")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let mut history_stages: Vec<String> = state
        .pointer("/progress/t.bayes/history/0/stages")
        .and_then(|stages| stages.as_object())
        .map(|stages| stages.keys().cloned().collect())
        .unwrap_or_default();
    history_stages.sort();

    let ok = dry.status == 0
        && dry_unchanged
        && unmapped.status == 1
        && error_code(&unmapped.err) == "pipeline.mapping-required"
        && commit.status == 0
        && validate.status == 0
        && pipeline == "check"
        && entry_pipeline == "check"
        && stages == ["done"]
        && history_pipeline == "flip"
        && history_stages == ["anchored", "learned", "proved"];
    check(
        "pipeline-switch",
        ok,
        format!(
            "flip→check: dry-run wrote {} · no-mapping exit {} · commit exit {} · entry {entry_pipeline} {stages:?} · history {history_pipeline} {history_stages:?}",
            if dry_unchanged { "nothing" } else { "bytes!" },
            unmapped.status,
            commit.status,
        ),
    )
}

/// The fixed ladder: `[1, 7, 30]` **from the completion anchor**, not chained
/// gaps — the anchor review consumes no offset.
fn scheduler_fixed_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("scheduler-fixed", false, error),
    };
    let arg = plan.to_string_lossy().to_string();
    let run = |args: &[&str]| harness.run(args, &[], None);
    let advance = |stage: &str, extra: &[&str]| {
        let mut args: Vec<&str> = vec![
            "record.advanceStage",
            "--id",
            "t.demo.02",
            "--stage",
            stage,
            "--plan",
            &arg,
            "--json",
        ];
        args.extend_from_slice(extra);
        run(&args)
    };
    let review = |rating: &str, at: &str| {
        run(&[
            "record.logReview",
            "--id",
            "t.demo.02",
            "--rating",
            rating,
            "--at",
            at,
            "--plan",
            &arg,
            "--json",
        ])
    };
    let learned = advance("learned", &[]);
    let proved = advance("proved", &["--problems", "2"]);
    let anchor = review("good", "2026-09-25T10:00:00Z");
    let second = review("hard", "2026-09-26T10:00:00Z");
    let third = review("good", "2026-10-02T10:00:00Z");
    let fourth = review("easy", "2026-10-25T10:00:00Z");
    let _ = std::fs::remove_dir_all(&plan);

    let due = |outcome: &ProcOutcome| {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data/due").cloned())
            .unwrap_or(serde_json::Value::Null)
    };
    let step = |outcome: &ProcOutcome| {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data/review/step").and_then(|v| v.as_i64()))
            .unwrap_or(-1)
    };
    let anchored = envelope(&anchor.out)
        .and_then(|value| value.pointer("/data/anchored").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    let ok = learned.status == 0
        && proved.status == 0
        && anchor.status == 0
        && anchored
        && due(&anchor) == serde_json::json!("2026-09-26")
        && due(&second) == serde_json::json!("2026-10-02")
        && due(&third) == serde_json::json!("2026-10-25")
        && due(&fourth) == serde_json::Value::Null
        && step(&anchor) == 0
        && step(&fourth) == 3;
    check(
        "scheduler-fixed",
        ok,
        format!(
            "anchor +1 → {} · +7 → {} · +30 → {} · exhausted → {} · anchored {anchored}",
            due(&anchor),
            due(&second),
            due(&third),
            due(&fourth)
        ),
    )
}

/// SM-2: published intervals and ease arithmetic, through the plan's own
/// settings (`scheduler.name` is data, so the check switches it the way a
/// student would).
fn scheduler_sm2_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("scheduler-sm2", false, error),
    };
    let arg = plan.to_string_lossy().to_string();
    // A stage machine that names no scheduler of its own: the plan's choice is
    // what acts.
    if let Err(error) = edit_json(&plan.join("content/rules.json"), |rules| {
        rules["scheduler"] = serde_json::json!({ "name": "sm2" });
        rules["pipelines"]["drill"] = serde_json::json!({
            "stages": ["new", "learning", "review"],
        });
    }) {
        return check("scheduler-sm2", false, error);
    }
    if let Err(error) = edit_json(&plan.join("content/types.json"), |types| {
        types["types"]["topic"]["pipeline"] = serde_json::json!("drill");
    }) {
        return check("scheduler-sm2", false, error);
    }
    let review = |rating: &str, at: &str| {
        harness.run(
            &[
                "record.logReview",
                "--id",
                "t.demo.03",
                "--rating",
                rating,
                "--at",
                at,
                "--plan",
                &arg,
                "--json",
            ],
            &[],
            None,
        )
    };
    let first = review("good", "2026-02-01T10:00:00Z");
    let second = review("good", "2026-02-02T10:00:00Z");
    let third = review("good", "2026-02-08T10:00:00Z");
    let failed = review("again", "2026-02-23T10:00:00Z");
    let state = read_to_string(&plan.join("state/state.json"));
    let _ = std::fs::remove_dir_all(&plan);
    let state: serde_json::Value = serde_json::from_str(&state).unwrap_or(serde_json::Value::Null);
    let review_state = state.pointer("/progress/t.demo.03/review");
    let value = |pointer: &str| {
        review_state
            .and_then(|review| review.pointer(pointer))
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    let due = |outcome: &ProcOutcome| {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data/due").cloned())
            .unwrap_or(serde_json::Value::Null)
    };
    let ok = first.status == 0
        && second.status == 0
        && third.status == 0
        && failed.status == 0
        && due(&first) == serde_json::json!("2026-02-02")
        && due(&second) == serde_json::json!("2026-02-08")
        && due(&third) == serde_json::json!("2026-02-23")
        && due(&failed) == serde_json::json!("2026-02-23")
        && value("/scheduler") == serde_json::json!("sm2")
        && value("/reps") == serde_json::json!(0)
        && value("/lapses") == serde_json::json!(1)
        && value("/ease") == serde_json::json!(2.18)
        && value("/log").as_array().map(Vec::len) == Some(4);
    check(
        "scheduler-sm2",
        ok,
        format!(
            "1 → {} · 6 → {} · 15 → {} · again → {} (reps 0, lapses 1, ease 2.18)",
            due(&first),
            due(&second),
            due(&third),
            due(&failed)
        ),
    )
}

/// FSRS: the pinned crate's own model, its recorded provenance, and a review
/// instant that resolves to the plan timezone's civil date.
fn scheduler_fsrs_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("scheduler-fsrs", false, error),
    };
    let arg = plan.to_string_lossy().to_string();
    // The scheduler is a settings row (§3.5, D11): the check switches it the
    // way a student would, not by hand-editing a different key path.
    let chosen = harness.run(
        &[
            "settings.set",
            "--key",
            "scheduler",
            "--value",
            "fsrs",
            "--plan",
            &arg,
            "--json",
        ],
        &[],
        None,
    );
    if chosen.status != 0 {
        return check("scheduler-fsrs", false, "settings.set scheduler failed");
    }
    // The seed's `srs` pipeline names FSRS itself, so the pipeline's own
    // scheduler is what acts here.
    if let Err(error) = edit_json(&plan.join("content/types.json"), |types| {
        types["types"]["topic"]["pipeline"] = serde_json::json!("srs");
    }) {
        return check("scheduler-fsrs", false, error);
    }
    let review = |id: &str, at: &str| {
        harness.run(
            &[
                "record.logReview",
                "--id",
                id,
                "--rating",
                "good",
                "--at",
                at,
                "--plan",
                &arg,
                "--json",
            ],
            &[],
            None,
        )
    };
    let first = review("t.demo.03", "2026-02-01T10:00:00Z");
    let second = review("t.demo.03", "2026-02-03T10:00:00Z");
    // A plan timezone of -05:00: a review instant at 23:30 local is still the
    // 8th, so the due date is computed from the 8th — not from the UTC day.
    let timezone_set = harness.run(
        &[
            "settings.set",
            "--key",
            "timezone",
            "--value",
            "-05:00",
            "--plan",
            &arg,
            "--json",
        ],
        &[],
        None,
    );
    if timezone_set.status != 0 {
        return check("scheduler-fsrs", false, "settings.set timezone failed");
    }
    let timezone = review("t.demo.04", "2026-03-08T23:30:00-05:00");

    let state = read_to_string(&plan.join("state/state.json"));
    let _ = std::fs::remove_dir_all(&plan);
    let state: serde_json::Value = serde_json::from_str(&state).unwrap_or(serde_json::Value::Null);
    // The **first** review's own values: the file holds the newest state, so
    // reading stability from it would grade the wrong review.
    let first_stability = envelope(&first.out)
        .and_then(|value| {
            value
                .pointer("/data/review/stability")
                .and_then(|v| v.as_f64())
        })
        .unwrap_or(-1.0);
    let algorithm = envelope(&first.out)
        .and_then(|value| {
            value
                .pointer("/data/algorithm")
                .and_then(|a| a.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let final_stability = state
        .pointer("/progress/t.demo.03/review/stability")
        .and_then(|value| value.as_f64())
        .unwrap_or(-1.0);
    let due = |outcome: &ProcOutcome| {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data/due").cloned())
            .unwrap_or(serde_json::Value::Null)
    };
    let stage = |outcome: &ProcOutcome| {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data/stage").cloned())
            .unwrap_or(serde_json::Value::Null)
    };
    // The pinned crate's documented first review for `good` at desired
    // retention 0.9: stability 2.3065, interval 2.3065 → 2 civil days. The
    // replay after 2 days grows the stability, which is what "it learns" means.
    let ok = first.status == 0
        && second.status == 0
        && timezone.status == 0
        && due(&first) == serde_json::json!("2026-02-03")
        && (first_stability - 2.3065).abs() < 0.01
        && final_stability > first_stability
        && due(&second) == serde_json::json!("2026-02-14")
        && algorithm == "FSRS-6 · default parameters"
        && due(&timezone) == serde_json::json!("2026-03-10")
        && stage(&first) == serde_json::json!("new")
        && stage(&second) == serde_json::json!("learning");
    check(
        "scheduler-fsrs",
        ok,
        format!(
            "good → {} (stability {first_stability:.4}, {algorithm}) · replay → {} (stability {final_stability:.4}) · -05:00 23:30 on 03-08 → {}",
            due(&first),
            due(&second),
            due(&timezone)
        ),
    )
}

/// The queue as data, on the fixture's bytes: what is due, what waits, and the
/// evidence each next transition asks for.
fn review_queue_check(resources_dir: &Path) -> CheckResult {
    let plan = resources_dir.join("fixtures/phase6");
    let config = match config_store::load_with(&plan, "explicit", resources_dir) {
        Ok(config) => config,
        Err(error) => return check("review-queue", false, error.to_string()),
    };
    let state = PlanState::load(&plan);
    let context = EvalContext::frozen(&config, Some(&state), FROZEN_NOW);
    let queue = crate::scheduler::due_json(&config, &context);
    let due_ids: Vec<String> = queue["due"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let waiting: Vec<String> = queue["waiting"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let overdue = queue
        .pointer("/due/0/overdueDays")
        .and_then(|value| value.as_i64())
        .unwrap_or(-1);
    let next_naive = queue
        .pointer("/waiting/3/next")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let naive_asks: Vec<String> = queue["waiting"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["id"] == serde_json::json!("t.naive"))
        })
        .and_then(|row| row["asks"].as_array())
        .map(|asks| {
            asks.iter()
                .filter_map(|ask| ask["key"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let counts = queue["counts"].clone();
    let ok = due_ids == ["t.bayes"]
        && overdue == 9
        && waiting == ["p.dp.01", "p.gr.01", "t.cond", "t.naive"]
        && next_naive == "anchored"
        && naive_asks == ["reason", "signal"]
        && counts["complete"] == serde_json::json!(1)
        && counts["trackable"] == serde_json::json!(5)
        && queue["scheduler"] == serde_json::json!("fixed");
    check(
        "review-queue",
        ok,
        format!(
            "due {due_ids:?} ({overdue}d late) · waiting {waiting:?} · t.naive asks {naive_asks:?} · {} of {} complete",
            counts["complete"], counts["trackable"]
        ),
    )
}

/// Derived metrics are the evaluator's own folds — compared against a manual
/// evaluation over the same view, so "the engine computes it" is checked rather
/// than assumed.
fn metrics_fold_check(resources_dir: &Path) -> CheckResult {
    let plan = resources_dir.join("fixtures/phase6");
    let config = match config_store::load_with(&plan, "explicit", resources_dir) {
        Ok(config) => config,
        Err(error) => return check("metrics-fold", false, error.to_string()),
    };
    let state = PlanState::load(&plan);
    let context = EvalContext::frozen(&config, Some(&state), FROZEN_NOW);
    let metrics = crate::rules::metrics_json(&config, &context);
    let value = |id: &str| {
        metrics["metrics"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"] == serde_json::json!(id)))
            .map(|row| row["value"].clone())
            .unwrap_or(serde_json::Value::Null)
    };
    let of = |id: &str| {
        metrics["metrics"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"] == serde_json::json!(id)))
            .and_then(|row| row["of"].as_i64())
            .unwrap_or(-1)
    };
    // The same fold by hand, from the same view resolution.
    let outcome = match views::query_records(&config, "sets.all", &context, None) {
        Ok(outcome) => outcome,
        Err(error) => return check("metrics-fold", false, error.to_string()),
    };
    let manual: i64 = outcome
        .records
        .iter()
        .filter_map(|record| record.fields.get("solved"))
        .filter_map(|value| value.as_i64())
        .sum();
    let ok = value("problems.solved") == serde_json::json!(manual)
        && manual == 15
        && of("problems.solved") == 2
        && value("topics.open") == serde_json::json!(2);
    check(
        "metrics-fold",
        ok,
        format!(
            "problems.solved {} (manual {manual}) of {} · topics.open {}",
            value("problems.solved"),
            of("problems.solved"),
            value("topics.open")
        ),
    )
}

/// Gate 3 (§6 Phase 6): **index parity** — every view and search result is
/// identical before and after indexing, a corrupt or deleted cache changes
/// nothing, and source bytes survive every one of those states.
fn index_parity_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let source = resources_dir.join("fixtures/phase6");
    let plan = std::env::temp_dir().join(unique("sam-p6-index"));
    let indexes = std::env::temp_dir().join(unique("sam-p6-indexes"));
    if let Err(error) = transaction::copy_tree(&source, &plan) {
        return check("index-parity", false, error.to_string());
    }
    let arg = plan.to_string_lossy().to_string();
    let indexes_arg = indexes.to_string_lossy().to_string();
    let env: [(&str, &str); 1] = [(crate::resources::INDEXES_ENV, &indexes_arg)];
    let run = |args: &[&str]| harness.run(args, &env, None);

    let before = snapshot(&plan);
    let oracle_view = run(&["view", "sets.hard", "--plan", &arg, "--json"]);
    let built = run(&["index.rebuild", "--plan", &arg, "--json"]);
    let indexed_view = run(&["view", "sets.hard", "--plan", &arg, "--json"]);
    let indexed_search = run(&["search", "bayes", "--plan", &arg, "--json"]);
    let oracle_search = run(&[
        "search", "bayes", "--engine", "oracle", "--plan", &arg, "--json",
    ]);
    let status = run(&["index.status", "--plan", &arg, "--json"]);
    let paths = run(&["paths", "--plan", &arg, "--json"]);

    let view_data = |outcome: &ProcOutcome| {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data").cloned())
            .map(|data| serde_json::to_string(&data).unwrap_or_default())
            .unwrap_or_default()
    };
    let search_ids = |outcome: &ProcOutcome| -> Vec<String> {
        envelope(&outcome.out)
            .and_then(|value| value.pointer("/data/records").cloned())
            .and_then(|records| records.as_array().cloned())
            .map(|records| {
                records
                    .iter()
                    .filter_map(|record| record["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    // Corrupt the cache: results must not move, and no source byte may change.
    let index_file = envelope(&built.out)
        .and_then(|value| {
            value
                .pointer("/data/path")
                .and_then(|p| p.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let corrupt_write = write_file(Path::new(&index_file), "not a database");
    let corrupted_view = run(&["view", "sets.hard", "--plan", &arg, "--json"]);
    let corrupted_search = run(&["search", "bayes", "--plan", &arg, "--json"]);
    let rebuilt = run(&["index.rebuild", "--plan", &arg, "--json"]);
    let after_rebuild = run(&["search", "bayes", "--plan", &arg, "--json"]);
    let source_intact = snapshot(&plan) == before;
    let _ = std::fs::remove_file(&index_file);
    let missing_view = run(&["view", "sets.hard", "--plan", &arg, "--json"]);
    let missing_search = run(&["search", "bayes", "--plan", &arg, "--json"]);
    let _ = std::fs::remove_dir_all(&plan);
    let _ = std::fs::remove_dir_all(&indexes);

    let indexed_ok = envelope(&indexed_search.out)
        .and_then(|value| {
            value
                .pointer("/data/engine")
                .and_then(|e| e.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let records = envelope(&built.out)
        .and_then(|value| value.pointer("/data/records").and_then(|r| r.as_i64()))
        .unwrap_or(-1);
    let fresh = envelope(&paths.out)
        .and_then(|value| value.pointer("/data/indexFresh").and_then(|f| f.as_bool()))
        .unwrap_or(false);
    let ok = oracle_view.status == 0
        && built.status == 0
        && built.out.contains("3.5")   // the linked SQLite reports its own version
        && records == 6
        && index_file.ends_with("index.sqlite")
        && view_data(&oracle_view) == view_data(&indexed_view)
        && indexed_ok == "index"
        && search_ids(&indexed_search) == search_ids(&oracle_search)
        && search_ids(&indexed_search) == ["t.bayes", "t.naive"]
        && status.status == 0
        && fresh
        && corrupt_write.is_ok()
        && view_data(&corrupted_view) == view_data(&oracle_view)
        && search_ids(&corrupted_search) == search_ids(&oracle_search)
        && rebuilt.status == 0
        && search_ids(&after_rebuild) == search_ids(&oracle_search)
        && source_intact
        && view_data(&missing_view) == view_data(&oracle_view)
        && search_ids(&missing_search) == search_ids(&oracle_search);
    check(
        "index-parity",
        ok,
        format!(
            "sqlite {} · view identical indexed/corrupt/missing · search {} vs oracle {} · source {}",
            if built.status == 0 {
                "rebuilt"
            } else {
                "FAILED"
            },
            search_ids(&indexed_search).len(),
            search_ids(&oracle_search).len(),
            if source_intact { "intact" } else { "CHANGED" },
        ),
    )
}

/// Bulk structural editing: renumber (idempotent, then real) and a cross-type
/// move that reports what the target cannot hold.
fn structure_ops_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let plan = match temp_plan(resources_dir) {
        Ok(plan) => plan,
        Err(error) => return check("structure-ops", false, error),
    };
    let arg = plan.to_string_lossy().to_string();
    let run = |args: &[&str]| harness.run(args, &[], None);

    let idempotent = run(&[
        "records.renumber",
        "--type",
        "week",
        "--plan",
        &arg,
        "--json",
    ]);
    let bumped = run(&[
        "record.setField",
        "--id",
        "w.02",
        "--field",
        "index",
        "--value",
        "5",
        "--plan",
        &arg,
        "--json",
    ]);
    let renumbered = run(&[
        "records.renumber",
        "--type",
        "week",
        "--plan",
        &arg,
        "--json",
    ]);
    let before_move = snapshot(&plan);
    let dry_move = run(&[
        "record.move",
        "--id",
        "t.demo.04",
        "--to",
        "note",
        "--dry-run",
        "--plan",
        &arg,
        "--json",
    ]);
    let dry_unchanged = snapshot(&plan) == before_move;
    let moved = run(&[
        "record.move",
        "--id",
        "t.demo.04",
        "--to",
        "note",
        "--plan",
        &arg,
        "--json",
    ]);
    let unknown = run(&[
        "record.move",
        "--id",
        "t.demo.03",
        "--to",
        "planets",
        "--plan",
        &arg,
        "--json",
    ]);
    let validate = run(&["--configcheck", "--plan", &arg, "--json"]);

    let topics = read_to_string(&plan.join("content/records/topic.jsonl"));
    let notes = read_to_string(&plan.join("content/records/note.jsonl"));
    let weeks = read_to_string(&plan.join("content/records/week.jsonl"));
    let _ = std::fs::remove_dir_all(&plan);

    let changed: Vec<serde_json::Value> = envelope(&renumbered.out)
        .and_then(|value| {
            value
                .pointer("/data/changed")
                .and_then(|c| c.as_array())
                .cloned()
        })
        .unwrap_or_default();
    let dropped_fields: Vec<String> = envelope(&moved.out)
        .and_then(|value| {
            value
                .pointer("/data/droppedFields")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
        })
        .unwrap_or_default();
    let dropped_links: Vec<String> = envelope(&moved.out)
        .and_then(|value| {
            value
                .pointer("/data/droppedLinks")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
        })
        .unwrap_or_default();
    let week_two = weeks
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|record| record["id"] == serde_json::json!("w.02"))
        .and_then(|record| record.pointer("/fields/index").and_then(|v| v.as_i64()))
        .unwrap_or(-1);

    let ok = idempotent.status == 0
        && envelope(&idempotent.out).and_then(|value| {
            value
                .pointer("/data/changed")
                .and_then(|c| c.as_array())
                .map(Vec::len)
        }) == Some(0)
        && bumped.status == 0
        && renumbered.status == 0
        && changed.len() == 1
        && changed[0]["id"] == serde_json::json!("w.02")
        && changed[0]["from"] == serde_json::json!(5)
        && changed[0]["to"] == serde_json::json!(2)
        && week_two == 2
        && dry_move.status == 0
        && dry_unchanged
        && moved.status == 0
        && dropped_fields == ["est", "kind"]
        && dropped_links == ["week"]
        && !topics.contains("t.demo.04")
        && notes.contains("t.demo.04")
        && unknown.status == 2
        && validate.status == 0;
    check(
        "structure-ops",
        ok,
        format!(
            "renumber idempotent → {} changed → {} · move dropped {dropped_fields:?} {dropped_links:?} · unknown type exit {}",
            changed.len(),
            if changed.is_empty() { 0 } else { 1 },
            unknown.status
        ),
    )
}

/// The guide teaches the rules, not only the fields: an agent learns the
/// scheduler, the machines and the metrics from `--guide --json`.
fn guide_study_check(harness: &Harness, resources_dir: &Path) -> CheckResult {
    let fixture = resources_dir.join("fixtures/phase6");
    let arg = fixture.to_string_lossy().to_string();
    let guide = harness.run(&["--guide", "--plan", &arg, "--json"], &[], None);
    let data = envelope(&guide.out).unwrap_or(serde_json::Value::Null);
    let scheduler = data
        .pointer("/data/study/scheduler/name")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let intervals = data
        .pointer("/data/study/scheduler/fixedIntervals")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let stages = data
        .pointer("/data/study/pipelines/flip/stages")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let metrics = data
        .pointer("/data/study/metrics")
        // The guide ships metrics as a name→shape map (schema_guide.rs),
        // each entry carrying its rules.json path.
        .and_then(|value| {
            value
                .as_object()
                .map(|map| map.keys().cloned().collect::<Vec<_>>())
        })
        .unwrap_or_default();
    let transitions = data
        .pointer("/data/study/transitions")
        .and_then(|value| value.as_array().map(Vec::len))
        .unwrap_or(0);
    let index_notes = data
        .pointer("/data/index")
        .and_then(|value| value.as_array().map(Vec::len))
        .unwrap_or(0);
    let mentions = |needle: &str| guide.out.contains(needle);
    let ok = guide.status == 0
        && scheduler == "fixed"
        && intervals == serde_json::json!([1, 7, 30])
        && stages == serde_json::json!(["learned", "proved", "anchored"])
        && metrics.iter().any(|metric| metric == "problems.solved")
        && transitions >= 4
        && index_notes >= 3
        && mentions("record.logReview")
        && mentions("search");
    check(
        "guide-study",
        ok,
        format!(
            "scheduler {scheduler} {intervals} · flip {stages} · {} metrics · {transitions} transitions · {index_notes} index notes",
            metrics.len()
        ),
    )
}
