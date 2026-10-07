//! SAM CLI dispatcher (§4.9, D15, D18).
//!
//! Dispatches commands directly through `sam-core` without windowing dependencies.
//! Routes mutations through `CommandSession` with consistent exit codes (§4.9).

use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sam_core::apply::ApplyError;
use sam_core::command_registry::{self, CommandEffect};
use sam_core::config_store::{self, ConfigStoreError};
use sam_core::diagnostic::{Diagnostic, Severity};
use sam_core::dispatch::{
    self, CommandSession, DispatchError, DispatchOutcome, ParamValue, Params,
};
use sam_core::plan_lock::PlanLockError;
use sam_core::resources;
use sam_core::transaction::{self, RecoveryConflict, TransactionError};
use serde_json::json;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// §4.9 exit codes.
const OK: u8 = 0;
const INVALID: u8 = 1;
const USAGE: u8 = 2;
const CONFLICT: u8 = 3;
const IO: u8 = 4;
const PRESENTATION: u8 = 5;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let json_flag = argv.iter().any(|arg| arg == "--json");
    let json = json_flag || !std::io::stdout().is_terminal();

    let cli = match parse(&argv, json) {
        Ok(cli) => cli,
        Err(message) => return usage(&message, json),
    };

    let Some(command) = cli.command.clone() else {
        help(json);
        return ExitCode::from(OK);
    };

    match command.as_str() {
        "help" => {
            help(json);
            ExitCode::from(OK)
        }
        "version" => version(json),
        "selfcheck" => selfcheck(json),
        "tokens" => tokens(json),
        "paths" => paths(cli.plan.as_deref(), json),
        "configcheck" => configcheck(cli.plan.as_deref(), json),
        "rendercheck" => match cli.plan.as_deref() {
            Some(plan) => rendercheck(plan, json),
            None => usage("rendercheck requires --plan <directory>", json),
        },
        "schema" => schema(cli.plan.as_deref(), json),
        "guide" => guide(cli.plan.as_deref(), json),
        "view" => match dispatch::optional(&cli.params, "name") {
            Some(name) => view(&name, cli.plan.as_deref(), json),
            None => usage("view requires one saved-view name (see SAM --schema)", json),
        },
        "commands" => commands(cli.plan.as_deref(), json),
        "tx.list" => tx_list(cli.plan.as_deref(), json),
        "plan.new" => plan_new(
            cli.plan.as_deref(),
            cli.name.as_deref(),
            dispatch::optional(&cli.params, "preset").as_deref(),
            json,
        ),
        "apply" => {
            let mut params = cli.params.clone();
            match params.get("batch") {
                Some(ParamValue::Str(source)) if source == "-" => {
                    let mut data = Vec::new();
                    if let Err(error) = std::io::stdin().read_to_end(&mut data) {
                        return io_failure(&format!("cannot read stdin: {error}"), json);
                    }
                    params.insert("batch".into(), ParamValue::Bytes(data));
                }
                None => {
                    return usage(
                        "apply requires one batch source: a file path or '-' for stdin",
                        json,
                    );
                }
                _ => {}
            }
            run_session("apply", params, cli.plan, cli.if_revision, json, json_flag)
        }
        "tx.restore" => {
            if cli.params.get("txid").is_none() {
                return usage("tx.restore requires one backup id from SAM tx.list", json);
            }
            run_session(
                "tx.restore",
                cli.params,
                cli.plan,
                cli.if_revision,
                json,
                json_flag,
            )
        }
        "uicheck" => uicheck(cli.plan.as_deref(), json),
        "invoke" => invoke(cli.plan, cli.if_revision, json, json_flag),
        // Remaining commands resolved dynamically by the engine (§4.9, D15).
        other => {
            let mut params = cli.params.clone();
            if other == "source.apply" {
                if let Some(ParamValue::Str(source)) = params.get("content").cloned() {
                    if source == "-" {
                        let mut data = Vec::new();
                        if let Err(error) = std::io::stdin().read_to_end(&mut data) {
                            return io_failure(&format!("cannot read stdin: {error}"), json);
                        }
                        params.insert("content".into(), ParamValue::Bytes(data));
                    }
                }
            }
            run_session(other, params, cli.plan, cli.if_revision, json, json_flag)
        }
    }
}

/// Dispatches batch or stdin commands using `CommandSession` (§4.9).
fn invoke(
    plan: Option<PathBuf>,
    if_revision: Option<String>,
    json: bool,
    _json_flag: bool,
) -> ExitCode {
    let mut payload = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut payload) {
        return io_failure(&format!("cannot read stdin: {error}"), json);
    }
    let text = match String::from_utf8(payload) {
        Ok(text) => text,
        Err(_) => return usage("invoke payload is not valid UTF-8", json),
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => return usage(&format!("invoke payload is not JSON: {error}"), json),
    };
    let requests = match value {
        serde_json::Value::Array(items) => items,
        single => vec![single],
    };

    let mut results = Vec::new();
    for request in requests {
        let Some(id) = request.get("id").and_then(|id| id.as_str()) else {
            return usage("each invoke request needs an \"id\"", json);
        };
        let params = match dispatch::params_from_json(request.get("params")) {
            Ok(params) => params,
            Err(error) => return map_dispatch_error(error, json),
        };
        match dispatch_once(id, params, plan.clone(), if_revision.clone(), json) {
            Ok(outcome) => results.push(dispatch::outcome_json(outcome)),
            Err(code) => return code,
        }
    }
    if json {
        let envelope = json!({ "ok": true, "results": results });
        match serde_json::to_string(&envelope) {
            Ok(text) => {
                println!("{text}");
                ExitCode::from(OK)
            }
            Err(error) => io_failure(&format!("invoke: {error}"), json),
        }
    } else {
        for result in &results {
            println!("{}", serde_json::to_string(result).unwrap_or_default());
        }
        ExitCode::from(OK)
    }
}

/// Declared structural UI dump from registry and active config (§6 Phase 4, Appendix C.6).
fn uicheck(plan: Option<&Path>, json: bool) -> ExitCode {
    let paths = match resolve_paths(json) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let selected = match config_store::plan_root(plan) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&selected.0) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    match config_store::load_with(&selected.0, &selected.1, &paths.resources_dir) {
        Ok(config) => {
            if json {
                print_envelope(
                    Some(&config.revision),
                    sam_core::uicheck::dump_json(&config),
                    &config.load_warnings,
                )
            } else {
                for line in sam_core::uicheck::dump(&config) {
                    println!("{line}");
                }
                ExitCode::from(OK)
            }
        }
        Err(error) => map_config_error(error, json),
    }
}

// ── argv → registry params (§4.9) ───────────────────────────────────────────

/// Command-line flags that resolve commands in pass 1 without values.
const COMMAND_FLAGS: [&str; 5] = ["--help", "-h", "--version", "-V", "--invoke"];

struct Cli {
    json: bool,
    plan: Option<PathBuf>,
    name: Option<String>,
    if_revision: Option<String>,
    command: Option<String>,
    params: Params,
    dry_run: bool,
}

/// Dynamic per-type commands (`<type>.new`, `<type>.paste`) (§4.7).
fn is_per_type_id(token: &str) -> bool {
    matches!(
        token.rsplit_once('.'),
        Some((type_name, "new" | "paste")) if !type_name.is_empty()
    )
}

/// Parameter value converted to text representation for promotion to repeated lists.
fn slot_text(value: &ParamValue) -> String {
    match value {
        ParamValue::Str(text) => text.clone(),
        ParamValue::Bool(flag) => flag.to_string(),
        ParamValue::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
        ParamValue::Strings(values) => values.first().cloned().unwrap_or_default(),
    }
}

fn parse(argv: &[String], json: bool) -> Result<Cli, String> {
    // Pass 1: resolve command verb (alias flags or explicit identifiers).
    let mut command: Option<String> = None;
    for arg in argv {
        if arg == "--help" || arg == "-h" {
            command = command.or_else(|| Some("help".into()));
            continue;
        }
        if arg == "--version" || arg == "-V" {
            command = command.or_else(|| Some("version".into()));
            continue;
        }
        // invoke transport handler (§4.9).
        if arg == "invoke" || arg == "--invoke" {
            command = command.or_else(|| Some("invoke".into()));
            continue;
        }
        if let Some(def) = command_registry::resolve(arg) {
            if def.aliases.iter().any(|alias| alias == arg) {
                command = command.or_else(|| Some(def.id.clone()));
            } else if !arg.starts_with('-') {
                command = Some(def.id.clone());
                break;
            }
        } else if is_per_type_id(arg) {
            command = Some(arg.clone());
            break;
        }
    }
    if command.is_none() {
        for arg in argv {
            if !arg.starts_with('-') && (arg == "help" || arg == "version") {
                command = Some(arg.clone());
                break;
            }
        }
    }

    let def = command.as_deref().and_then(command_registry::resolve);
    let positional_defs: Vec<&str> = def
        .map(|def| {
            def.params
                .iter()
                .filter(|param| param.positional)
                .map(|param| param.name.as_str())
                .collect()
        })
        .unwrap_or_default();

    let mut cli = Cli {
        json,
        plan: None,
        name: None,
        if_revision: None,
        command: command.clone(),
        params: Params::new(),
        dry_run: false,
    };
    let mut positionals: Vec<String> = Vec::new();
    let mut positional_index = 0;
    let mut index = 0;

    while index < argv.len() {
        let arg = argv[index].as_str();
        if arg.starts_with('-') && arg != "-" {
            // Consume flags that resolved the command in pass 1.
            if COMMAND_FLAGS.contains(&arg)
                || def.is_some_and(|def| def.aliases.iter().any(|alias| alias == arg))
            {
                index += 1;
                continue;
            }
            match arg {
                "--json" => cli.json = true,
                "--dry-run" => {
                    cli.dry_run = true;
                    cli.params.insert("dry-run".into(), ParamValue::Bool(true));
                }
                "--stdin" => {}
                "--plan" | "--name" | "--if-revision" => {
                    let Some(value) = argv.get(index + 1) else {
                        return Err(format!("{arg} requires a value"));
                    };
                    match arg {
                        "--plan" => cli.plan = Some(PathBuf::from(value)),
                        "--name" => {
                            cli.name = Some(value.clone());
                            cli.params
                                .insert("name".into(), ParamValue::Str(value.clone()));
                        }
                        _ => cli.if_revision = Some(value.clone()),
                    }
                    index += 1;
                }
                other => {
                    let param = def.and_then(|def| {
                        def.params
                            .iter()
                            .find(|param| param.flag.as_deref() == Some(other))
                    });
                    match param {
                        Some(param) if param.type_ == "boolean" => {
                            cli.params
                                .insert(param.name.clone(), ParamValue::Bool(true));
                        }
                        Some(param) if param.repeats => {
                            let Some(value) = argv.get(index + 1) else {
                                return Err(format!("{other} requires a value"));
                            };
                            match cli.params.get_mut(&param.name) {
                                Some(ParamValue::Strings(values)) => values.push(value.clone()),
                                _ => {
                                    cli.params.insert(
                                        param.name.clone(),
                                        ParamValue::Strings(vec![value.clone()]),
                                    );
                                }
                            }
                            index += 1;
                        }
                        Some(param) => {
                            let Some(value) = argv.get(index + 1) else {
                                return Err(format!("{other} requires a value"));
                            };
                            cli.params
                                .insert(param.name.clone(), ParamValue::Str(value.clone()));
                            index += 1;
                        }
                        // For per-type commands, map unknown flags directly to type fields (§4.8).
                        None if def.is_none() && command.is_some() => {
                            let Some(value) = argv.get(index + 1) else {
                                return Err(format!("{other} requires a value"));
                            };
                            let name = other.trim_start_matches('-').to_string();
                            match cli.params.get_mut(&name) {
                                Some(ParamValue::Strings(values)) => values.push(value.clone()),
                                Some(slot) => {
                                    let first = slot_text(slot);
                                    *slot = ParamValue::Strings(vec![first, value.clone()]);
                                }
                                None => {
                                    cli.params.insert(name, ParamValue::Str(value.clone()));
                                }
                            }
                            index += 1;
                        }
                        None => return Err(format!("unknown argument '{other}'")),
                    }
                }
            }
        } else {
            if Some(arg) == command.as_deref() {
                index += 1;
                continue; // the verb itself
            }
            if positional_index < positional_defs.len() {
                cli.params.insert(
                    positional_defs[positional_index].to_string(),
                    ParamValue::Str(arg.to_string()),
                );
                positional_index += 1;
            }
            positionals.push(arg.to_string());
        }
        index += 1;
    }

    if cli.command.is_none() && !positionals.is_empty() {
        return Err(format!("unknown command '{}'", positionals[0]));
    }
    Ok(cli)
}

// ── output contract (§4.9) ───────────────────────────────────────────────────
//
// Data on stdout as one envelope on success; structured diagnostics on stderr
// on failure, with no partial success payload.

fn print_envelope(
    revision: Option<&str>,
    data: serde_json::Value,
    warnings: &[Diagnostic],
) -> ExitCode {
    let envelope = json!({
        "ok": true,
        "revision": revision,
        "data": data,
        "warnings": warnings,
    });
    match serde_json::to_string(&envelope) {
        Ok(text) => {
            println!("{text}");
            ExitCode::from(OK)
        }
        Err(error) => {
            eprintln!("SAM: cannot serialize the result envelope: {error}");
            ExitCode::from(IO)
        }
    }
}

fn fail(code: &str, path: &str, message: &str, json: bool, exit: u8) -> ExitCode {
    if json {
        let value = json!({ "code": code, "path": path, "line": null, "message": message });
        match serde_json::to_string(&value) {
            Ok(text) => eprintln!("{text}"),
            Err(_) => eprintln!("SAM: {message} [{code}]"),
        }
    } else {
        eprintln!("SAM: {message} [{code}]");
    }
    ExitCode::from(exit)
}

fn usage(message: &str, json: bool) -> ExitCode {
    fail("cli.usage", "-", message, json, USAGE)
}

fn io_failure(message: &str, json: bool) -> ExitCode {
    fail("io.failure", "-", message, json, IO)
}

fn conflict(code: &str, message: &str, json: bool) -> ExitCode {
    fail(code, ".sam", message, json, CONFLICT)
}

fn recovery(conflicts: &[RecoveryConflict], json: bool) -> ExitCode {
    if json {
        for conflict in conflicts {
            let value = json!({
                "code": "tx.recovery-conflict",
                "path": conflict.path,
                "line": null,
                "message": conflict.message,
            });
            eprintln!("{}", serde_json::to_string(&value).unwrap_or_default());
        }
    } else {
        for conflict in conflicts {
            eprintln!("SAM: ✗ tx {}: {}", conflict.txid, conflict.message);
        }
    }
    ExitCode::from(IO)
}

/// A validation failure is not one error but all of them (§3.7). JSON output
/// carries the errors — a warning is advisory context, not a failure — and the
/// human form prints everything with a count so nothing is silently dropped.
fn fail_validation(diagnostics: &[Diagnostic], json: bool) -> ExitCode {
    let errors: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    let warnings = diagnostics.len() - errors.len();
    if json {
        for diagnostic in &errors {
            match serde_json::to_string(diagnostic) {
                Ok(text) => eprintln!("{text}"),
                Err(_) => eprintln!("SAM: {diagnostic}"),
            }
        }
    } else {
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
        }
        eprintln!(
            "✗ {} error{}{}",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" },
            if warnings == 0 {
                String::new()
            } else {
                format!(
                    " · {warnings} warning{}",
                    if warnings == 1 { "" } else { "s" }
                )
            }
        );
    }
    ExitCode::from(INVALID)
}

fn presentation(destination: &str, json: bool) -> ExitCode {
    if json {
        print_envelope(
            None,
            json!({
                "destination": destination,
                "available": false,
                "reason": "no GUI in this context",
            }),
            &[],
        );
    } else {
        println!("presentation unavailable headless: {destination}");
    }
    ExitCode::from(PRESENTATION)
}

/// Map engine and lock failures onto the §4.9 exit codes. Nothing prompts.
fn map_dispatch_error(error: DispatchError, json: bool) -> ExitCode {
    match error {
        DispatchError::Usage(message) => usage(&message, json),
        DispatchError::Io(message) => io_failure(&message, json),
        DispatchError::Apply(ApplyError::Usage(message)) => usage(&message, json),
        DispatchError::Apply(ApplyError::Validation(diagnostics))
        | DispatchError::Invalid(diagnostics) => fail_validation(&diagnostics, json),
        DispatchError::Transaction(TransactionError::ExternalConflict(message)) => {
            conflict("tx.external-conflict", &message, json)
        }
        DispatchError::Transaction(TransactionError::RecoveryConflict(conflicts)) => {
            recovery(&conflicts, json)
        }
        DispatchError::Transaction(TransactionError::Io(message)) => io_failure(&message, json),
        DispatchError::Lock(PlanLockError::Busy(message)) => {
            conflict("tx.lock-busy", &message, json)
        }
        DispatchError::Lock(PlanLockError::Io(message)) => io_failure(&message, json),
    }
}

fn map_config_error(error: ConfigStoreError, json: bool) -> ExitCode {
    match error {
        ConfigStoreError::Io(message) => io_failure(&message, json),
        ConfigStoreError::Validation(diagnostics) => fail_validation(&diagnostics, json),
    }
}

// ── commands ─────────────────────────────────────────────────────────────────

fn help(json: bool) {
    if json {
        print_envelope(
            None,
            json!({
                "version": VERSION,
                "commands": command_registry::commands()
                    .iter()
                    .map(|def| json!({
                        "id": def.id,
                        "category": def.category,
                        "effect": def.effect,
                        "help": def.help,
                    }))
                    .collect::<Vec<_>>(),
            }),
            &[],
        );
        return;
    }
    println!("SAM {VERSION} — Configurable Study OS (Phase 3: expressions and headless queries)\n");
    println!("Usage: SAM <command-id> [flags]\n");
    println!(
        "The verb is the registry id — the same id ⌘K will show (§4.9).\nNothing prompts; data on stdout, diagnostics on stderr; JSON when piped or --json.\n"
    );
    println!("Commands:");
    let mut defs: Vec<_> = command_registry::commands().iter().collect();
    defs.sort_by(|a, b| (&a.category, &a.id).cmp(&(&b.category, &b.id)));
    let mut category = "";
    for def in defs {
        if def.category != category {
            category = &def.category;
            println!("  {category}");
        }
        let aliases = if def.aliases.is_empty() {
            String::new()
        } else {
            format!("  ({})", def.aliases.join(", "))
        };
        let effect = match def.effect {
            CommandEffect::Read => "read",
            CommandEffect::Write => "write",
            CommandEffect::Presentation => "presentation",
        };
        println!("    {:<14} [{effect}] {}{aliases}", def.id, def.help);
    }
    println!(
        "\nAgent loop (§4.7):\n\
         \x20 SAM paths --json                                    → plan root + revision\n\
         \x20 SAM apply batch.jsonl --dry-run --json              → record-level diff, no writes\n\
         \x20 SAM apply batch.jsonl --if-revision <hash> --json   → commit at the reviewed revision\n\
         \n\
         Exit codes: 0 success · 1 validation failure · 2 usage error\n\
         \x20           3 revision conflict / lock busy · 4 I/O or recovery failure\n\
         \x20           5 presentation unavailable\n\
         \n\
         --plan <dir> selects a plan root; without it the single plan under\n\
         the platform data directory is used. plan.new creates one."
    );
}

fn version(json: bool) -> ExitCode {
    if json {
        print_envelope(None, json!({ "name": "SAM", "version": VERSION }), &[])
    } else {
        println!("SAM {VERSION}");
        ExitCode::from(OK)
    }
}

fn selfcheck(json: bool) -> ExitCode {
    let binary = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("sam"));
    let results = sam_core::run_selfcheck(&binary);
    let failed = results.iter().filter(|result| !result.ok).count();
    let all_ok = failed == 0;

    if json {
        let envelope = json!({
            "ok": all_ok,
            "checks": results
                .iter()
                .map(|result| json!({
                    "name": result.name,
                    "ok": result.ok,
                    "detail": result.detail,
                }))
                .collect::<Vec<_>>(),
        });
        match serde_json::to_string(&envelope) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("SAM: cannot serialize selfcheck: {error}");
                return ExitCode::from(IO);
            }
        }
    } else {
        for result in &results {
            println!(
                "{} {} — {}",
                if result.ok { "ok  " } else { "FAIL" },
                result.name,
                result.detail
            );
        }
        println!(
            "\n{}/{} checks passed",
            results.len() - failed,
            results.len()
        );
    }
    ExitCode::from(if all_ok { OK } else { INVALID })
}

fn tokens(json: bool) -> ExitCode {
    let paths = match resolve_paths(json) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    match resources::TokenRegister::bundled(&paths.resources_dir) {
        Ok(register) => {
            if json {
                let map: serde_json::Map<String, serde_json::Value> = register
                    .tokens
                    .iter()
                    .map(|(path, token)| {
                        (
                            path.clone(),
                            serde_json::to_value(token).unwrap_or_default(),
                        )
                    })
                    .collect();
                print_envelope(
                    None,
                    json!({
                        "version": register.version,
                        "count": register.tokens.len(),
                        "source": paths.resources_dir.display().to_string(),
                        "tokens": map,
                    }),
                    &[],
                )
            } else {
                println!(
                    "SAM tokens · version {} · {} tokens · {}",
                    register.version,
                    register.tokens.len(),
                    paths.resources_dir.display()
                );
                for (path, token) in &register.tokens {
                    println!(
                        "{path}\t{}\t{}",
                        token.type_.as_deref().unwrap_or("-"),
                        resources::token_text(token)
                    );
                }
                ExitCode::from(OK)
            }
        }
        Err(message) => io_failure(&format!("tokens: {message}"), json),
    }
}

/// `paths` answers on a valid **and** an invalid plan (§4.7): the root and the
/// raw revision are what an agent needs before it can repair one, so this never
/// runs the validator.
fn paths(plan: Option<&Path>, json: bool) -> ExitCode {
    let paths = match resolve_paths(json) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    // A machine with no plan resolves nothing — `paths` is the command a bug
    // report runs before anything is open, and the registry marks it
    // `plan_required: false` (BUILDLOG 2026-10-06). The read session and the
    // fingerprint exist only when a root does.
    let selected = config_store::plan_root(plan).ok();
    if let Some((root, _)) = &selected {
        let _session = match dispatch::open_read(root) {
            Ok(session) => session,
            Err(error) => return map_dispatch_error(error, json),
        };
    }
    let revision = selected
        .as_ref()
        .map(|(root, _)| config_store::raw_revision(root));

    if json {
        // One projection for every surface: the shell's `dispatch` and the
        // browser harness read exactly this (§4.8 P2).
        let data = match dispatch::read_json(
            "paths",
            &Params::new(),
            None,
            &paths.resources_dir,
            plan,
        ) {
            Ok(data) => data,
            Err(error) => return map_dispatch_error(error, json),
        };
        print_envelope(revision.as_deref(), data, &[])
    } else {
        println!("data {}", paths.data_dir.display());
        println!("plans {}", paths.plans_dir.display());
        println!("indexes {}", paths.indexes_dir.display());
        println!("resources {}", paths.resources_dir.display());
        match (&selected, &revision) {
            (Some((root, layer)), Some(revision)) => {
                println!("plan {} ({layer})", root.display());
                println!("revision {revision}");
            }
            // No plan on the machine: the human form says so rather than
            // printing a path that does not exist.
            _ => {
                println!("plan — (no plan resolves)");
                println!("revision —");
            }
        }
        println!("index (derived; Phase 6)");
        ExitCode::from(OK)
    }
}

fn configcheck(plan: Option<&Path>, json: bool) -> ExitCode {
    let (root, layer) = match config_store::plan_root(plan) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&root) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    match config_store::load(&root, &layer) {
        Ok(config) => {
            let pipelines = config
                .rules
                .pipelines
                .as_ref()
                .map(|pipelines| pipelines.len())
                .unwrap_or(0);
            if json {
                print_envelope(
                    Some(&config.revision),
                    json!({
                        "plan": root.display().to_string(),
                        "layer": layer,
                        "types": config.types.len(),
                        "records": config.record_count(),
                        "views": config.views.len(),
                        "pipelines": pipelines,
                    }),
                    &config.load_warnings,
                )
            } else {
                for warning in &config.load_warnings {
                    println!("  advisory {warning}");
                }
                println!(
                    "✓ valid · {} types · {} records · {} views · {} pipelines · revision {}",
                    config.types.len(),
                    config.record_count(),
                    config.views.len(),
                    pipelines,
                    config.revision
                );
                ExitCode::from(OK)
            }
        }
        Err(error) => map_config_error(error, json),
    }
}

fn rendercheck(plan: &Path, json: bool) -> ExitCode {
    let (root, layer) = match config_store::plan_root(Some(plan)) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&root) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    match config_store::load(&root, &layer) {
        Ok(config) => {
            let tree = config.content_tree();
            if json {
                print_envelope(
                    Some(&config.revision),
                    json!({ "plan": root.display().to_string(), "tree": tree }),
                    &config.load_warnings,
                )
            } else {
                for line in &tree {
                    println!("{line}");
                }
                ExitCode::from(OK)
            }
        }
        Err(error) => map_config_error(error, json),
    }
}

/// `--schema` and `--guide` (§4.7 #8/#9): both generated from the same TypeDefs
/// the validator and the record batches use, so an agent never guesses a field
/// name or an option value.
fn schema(plan: Option<&Path>, json: bool) -> ExitCode {
    let (root, layer) = match config_store::plan_root(plan) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&root) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    match config_store::load(&root, &layer) {
        Ok(config) => emit_generated(
            sam_core::schema_guide::schema_json(&config),
            &config,
            json,
            "schema",
        ),
        Err(error) => map_config_error(error, json),
    }
}

fn guide(plan: Option<&Path>, json: bool) -> ExitCode {
    let paths = match resolve_paths(json) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let (root, layer) = match config_store::plan_root(plan) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&root) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    match config_store::load(&root, &layer) {
        Ok(config) => emit_generated(
            sam_core::schema_guide::guide_json(&config, &paths.resources_dir),
            &config,
            json,
            "guide",
        ),
        Err(error) => map_config_error(error, json),
    }
}

fn emit_generated(
    data: serde_json::Value,
    config: &sam_core::resolved_config::ResolvedConfig,
    json: bool,
    label: &str,
) -> ExitCode {
    if json {
        print_envelope(Some(&config.revision), data, &config.load_warnings)
    } else {
        match serde_json::to_string_pretty(&data) {
            Ok(text) => {
                println!("{text}");
                ExitCode::from(OK)
            }
            Err(error) => io_failure(&format!("{label}: {error}"), json),
        }
    }
}

/// `SAM view <name>` — the primary read interface (§4.7). Deterministic by
/// construction: canonical records, declared null order, stable id tie-breaker.
fn view(name: &str, plan: Option<&Path>, json: bool) -> ExitCode {
    let (root, layer) = match config_store::plan_root(plan) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&root) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    let config = match config_store::load(&root, &layer) {
        Ok(config) => config,
        Err(error) => return map_config_error(error, json),
    };
    let state = sam_core::views::PlanState::load(&root);
    let context = sam_core::evaluator::EvalContext::new(&config, Some(&state));
    match sam_core::views::resolve(&config, name, &context) {
        Ok(outcome) => {
            if json {
                return print_envelope(Some(&config.revision), outcome.json(&config), &outcome.warnings);
            }
            println!(
                "{} · {} · {} · {}/{} records",
                outcome.view,
                // A composed screen names no type or layout (COMPOSER §2.1);
                // the terminal states that rather than inventing one.
                outcome.type_.as_deref().unwrap_or("—"),
                outcome.layout.as_deref().unwrap_or("—"),
                outcome.records.len(),
                outcome.candidates
            );
            for warning in &outcome.warnings {
                println!("  ⚠ {warning}");
            }
            if let Some(groups) = &outcome.groups {
                for group in groups {
                    println!(
                        "  [{}] {}",
                        group.key.as_deref().unwrap_or("-"),
                        group.records.len()
                    );
                    for record in &group.records {
                        print_record_line(record);
                    }
                }
            } else {
                for record in &outcome.records {
                    print_record_line(record);
                }
            }
            ExitCode::from(OK)
        }
        Err(sam_core::views::ViewError::Unknown(name)) => fail_validation(
            &[Diagnostic::error(
                "view.unknown",
                "content/views.json",
                format!("no saved view named \"{name}\" — SAM --schema lists views"),
            )],
            json,
        ),
        Err(sam_core::views::ViewError::Expression(error)) => {
            fail_validation(&[error.diagnostic], json)
        }
    }
}

fn print_record_line(record: &sam_core::model::Record) {
    let title = record
        .fields
        .get("title")
        .or_else(|| record.fields.get("label"))
        .map(sam_core::json_value::scalar_text)
        .unwrap_or_default();
    println!("  {}\t{title}", record.id);
}

/// The registry projection. `--plan` resolves a plan root and loads it, so the
/// per-type pair (`<type>.new` / `<type>.paste`) is enumerated too — the same
/// projection the palette, the menu bar and the shell read (§4.8 P2).
fn commands(plan: Option<&Path>, json: bool) -> ExitCode {
    let paths = match resolve_paths(json) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let mut config = None;
    if let Some(plan) = plan {
        let (root, layer) = match config_store::plan_root(Some(plan)) {
            Ok(selected) => selected,
            Err(error) => return map_config_error(error, json),
        };
        if let Err(error) = dispatch::open_read(&root) {
            return map_dispatch_error(error, json);
        }
        match config_store::load_with(&root, &layer, &paths.resources_dir) {
            Ok(loaded) => config = Some(loaded),
            Err(error) => return map_config_error(error, json),
        }
    }
    let data = match dispatch::read_json(
        "commands",
        &Params::new(),
        config.as_ref(),
        &paths.resources_dir,
        plan,
    ) {
        Ok(data) => data,
        Err(error) => return map_dispatch_error(error, json),
    };
    if json {
        return print_envelope(config.as_ref().map(|config| config.revision.as_str()), data, &[]);
    }

    let dynamic = config
        .as_ref()
        .map(|config| command_registry::dynamic_commands(&config.types))
        .unwrap_or_default();
    let mut sorted: Vec<&command_registry::CommandDef> =
        command_registry::commands().iter().chain(dynamic.iter()).collect();
    sorted.sort_by(|a, b| (&a.category, &a.id).cmp(&(&b.category, &b.id)));
    let mut category = "";
    for def in sorted {
        if def.category != category {
            category = &def.category;
            println!("{category}:");
        }
        println!(
            "  {:<20} [{}] {}",
            def.id,
            match def.effect {
                CommandEffect::Read => "read",
                CommandEffect::Write => "write",
                CommandEffect::Presentation => "presentation",
            },
            def.help
        );
    }
    ExitCode::from(OK)
}

fn tx_list(plan: Option<&Path>, json: bool) -> ExitCode {
    let (root, _) = match config_store::plan_root(plan) {
        Ok(selected) => selected,
        Err(error) => return map_config_error(error, json),
    };
    let _session = match dispatch::open_read(&root) {
        Ok(session) => session,
        Err(error) => return map_dispatch_error(error, json),
    };
    let (manifests, retention, warning) = transaction::list_backups(&root);
    if json {
        let mut data = json!({
            "retention": retention,
            "backups": manifests
                .iter()
                .map(|manifest| json!({
                    "txid": manifest.txid,
                    "date": manifest.date,
                    "summary": manifest.summary,
                    "baseRevision": manifest.base_revision,
                    "newRevision": manifest.new_revision,
                    "files": manifest.entries.iter().map(|entry| entry.path.clone()).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>(),
        });
        if let Some(warning) = &warning {
            if let serde_json::Value::Object(map) = &mut data {
                map.insert("warning".into(), json!(warning));
            }
        }
        print_envelope(None, data, &[])
    } else {
        println!("retained transaction backups (retention {retention}, .sam/settings.json)");
        if let Some(warning) = &warning {
            println!("⚠ {warning}");
        }
        if manifests.is_empty() {
            println!("(none)");
        }
        for manifest in &manifests {
            println!("{}  {}  {}", manifest.txid, manifest.date, manifest.summary);
            println!(
                "    {}",
                manifest
                    .entries
                    .iter()
                    .map(|entry| entry.path.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        ExitCode::from(OK)
    }
}

/// `plan.new` has no plan root to lock, so it is the CLI's own verb. Exactly one
/// of `--name` or `--plan`, plus `--preset <name>` (§6 Phase 7 ships six
/// presets; the blank plan remains the default). The destination rule is the
/// engine's (`config_store::plan_destination`), so the terminal and the picker
/// refuse the same names the same way.
fn plan_new(
    target: Option<&Path>,
    name: Option<&str>,
    preset: Option<&str>,
    json: bool,
) -> ExitCode {
    if target.is_some() == name.is_some() {
        return usage(
            "plan.new needs exactly one of --name <name> or --plan <directory>",
            json,
        );
    }
    let paths = match resolve_paths(json) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let target = match config_store::plan_destination(&paths.plans_dir, name, target) {
        Ok(target) => target,
        Err(message) => return usage(&message, json),
    };
    let preset = preset.unwrap_or("blank");

    match config_store::create_plan(&target, preset, &paths.resources_dir) {
        Ok(created) => {
            let revision = config_store::raw_revision(&created);
            if json {
                print_envelope(
                    Some(&revision),
                    json!({
                        "plan": created.display().to_string(),
                        "preset": preset,
                    }),
                    &[],
                )
            } else {
                println!(
                    "✓ created {} from preset {preset} · revision {revision}",
                    created.display()
                );
                ExitCode::from(OK)
            }
        }
        Err(error) => map_config_error(error, json),
    }
}

/// Run one registry write (or presentation) request and project its outcome.
fn run_session(
    id: &str,
    params: Params,
    plan: Option<PathBuf>,
    if_revision: Option<String>,
    json: bool,
    json_flag: bool,
) -> ExitCode {
    let outcome = match dispatch_once(id, params, plan, if_revision, json) {
        Ok(outcome) => outcome,
        Err(code) => return code,
    };
    match outcome {
        DispatchOutcome::Read(data) => print_envelope(None, data, &[]),
        DispatchOutcome::Write {
            commit,
            changes,
            data,
            summary: _,
            dry_run,
        } => {
            if dry_run {
                return print_envelope(Some(&commit.base_revision), data, &[]);
            }
            let mut enriched = data;
            if let serde_json::Value::Object(map) = &mut enriched {
                map.insert("txid".into(), json!(commit.txid));
                map.insert(
                    "files".into(),
                    json!(
                        changes
                            .iter()
                            .map(|change| change.path.clone())
                            .collect::<Vec<_>>()
                    ),
                );
            }
            print_envelope(Some(&commit.new_revision), enriched, &[])
        }
        DispatchOutcome::Presentation { destination } => presentation(&destination, json_flag),
    }
}

/// One dispatch with path resolution — the whole of `run_session` except the
/// printing, so `invoke` reuses it rather than growing a second call path.
fn dispatch_once(
    id: &str,
    params: Params,
    plan: Option<PathBuf>,
    if_revision: Option<String>,
    json: bool,
) -> Result<DispatchOutcome, ExitCode> {
    let paths = resolve_paths(json)?;
    let mut request = dispatch::DispatchRequest::new(id, paths.resources_dir);
    request.params = params;
    request.plan = plan;
    request.if_revision = if_revision;
    CommandSession::run(request).map_err(|error| map_dispatch_error(error, json))
}

fn resolve_paths(json: bool) -> Result<resources::Paths, ExitCode> {
    resources::paths().map_err(|error| io_failure(&format!("cannot resolve paths: {error}"), json))
}
