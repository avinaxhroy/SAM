//! The adapter table (§4.7, extended by the assistant): one **data row** per
//! agent CLI — nothing here runs a process or parses a byte. The engine in
//! `agent.rs` reads these fields, which is why adding a CLI is one row rather
//! than one code path (the contract OpenDesign documents as its
//! "adapter = data, not a class" decision, `docs/agent-adapters.md`).
//!
//! Invocations, flags, prompt delivery, model discovery and session resume are
//! the ones OpenDesign ships for the same binaries (`runtimes/defs/*.ts`), with
//! one deliberate difference: SAM passes each CLI's **headless auto-approval
//! flag unconditionally** where OpenDesign probes help text first. SAM runs the
//! CLI without a TTY, so an approval prompt would hang the run — and a flag a
//! build does not know fails loudly in the transcript, where a hang fails
//! silently. The CLI's own authentication and policy still apply; SAM stores
//! and passes no credentials.
//!
//! Flags drift between CLI releases. When a row fails on a user's machine the
//! error is its own streamed transcript line, and the fix is this table — not
//! a parser. Rows marked `verify` were written from published help output and
//! must be re-checked against `<bin> --help` on a machine that has the binary.

use super::stream::StreamFormat;

/// One turn's invocation inputs. Pure data — a `build_args` may not run
/// anything, probe anything, or read the disk.
pub struct Invocation<'a> {
    pub prompt: &'a str,
    /// The model the student picked, already sanitized; `None` = the CLI's own
    /// configured default (the picker's *Default* row).
    pub model: Option<&'a str>,
}

/// How the composed prompt reaches the CLI. Stdin is preferred everywhere it
/// works: argv caps are real (Linux `E2BIG`, Windows `CreateProcess`), and two
/// protocols put the prompt inside a framed command instead of raw text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptDelivery {
    /// Plain text written to stdin, then EOF (`claude -p`, `opencode run`).
    StdinText,
    /// One `{"id":1,"type":"prompt","message":…}` line (Pi's RPC mode).
    StdinPiRpc,
    /// One `{"event":"user","message":{"content":…}}` NDJSON line (Antigravity).
    StdinAntigravity,
    /// Mode where stdin carries nothing structured: `{prompt}` substitution
    /// happens in `build_args` and stdin is `/dev/null`.
    Argv,
}

impl PromptDelivery {
    pub fn needs_stdin(self) -> bool {
        self != PromptDelivery::Argv
    }
}

/// What `detect()` needs to probe one CLI, plus everything a run needs to
/// build its argv. `build_args` is a plain function pointer so the table
/// stays a `static`.
pub struct AgentDef {
    pub id: &'static str,
    pub name: &'static str,
    /// The executable to probe on `PATH`.
    pub bin: &'static str,
    /// Alternate executable names, tried in order when `bin` is absent.
    pub fallback_bins: &'static [&'static str],
    /// Args for the availability / version probe.
    pub version_args: &'static [&'static str],
    pub stream_format: StreamFormat,
    pub delivery: PromptDelivery,
    pub build_args: fn(&Invocation) -> Vec<String>,
    /// The CLI's own model-listing command, when it has one. `None` means the
    /// picker offers `fallback_models` only.
    pub models_args: Option<&'static [&'static str]>,
    /// Model ids offered when discovery is unavailable. The *Default* row is
    /// synthetic (`None`); these are the concrete hints a student can pick.
    pub fallback_models: &'static [&'static str],
    /// The args that continue a captured session id (`claude --resume`,
    /// `opencode -s`). `None` = the CLI starts fresh each turn.
    pub resume_args: Option<fn(&str) -> Vec<String>>,
    /// One line shown when the binary is missing, with a way to get it.
    pub install_hint: &'static str,
}

pub static AGENT_DEFS: &[AgentDef] = &[
    AgentDef {
        id: "claude",
        name: "Claude Code",
        bin: "claude",
        // Drop-in forks that ship a `claude`-compatible argv (OpenDesign's
        // observation, kept).
        fallback_bins: &["openclaude"],
        version_args: &["--version"],
        stream_format: StreamFormat::ClaudeStreamJson,
        delivery: PromptDelivery::StdinText,
        build_args: |invocation| {
            let mut args: Vec<String> = vec![
                "-p".into(),
                "--output-format".into(),
                "stream-json".into(),
                "--verbose".into(),
                // Token-level frames (`stream_event` wrappers) so the deck has
                // something to draw from the first token — verify against the
                // installed build; an unknown flag fails loudly in the
                // transcript, which is this table's own rule.
                "--include-partial-messages".into(),
            ];
            if let Some(model) = invocation.model {
                args.push("--model".into());
                args.push(model.to_string());
            }
            // Headless posture: no TTY, so approvals must not be awaited.
            args.push("--permission-mode".into());
            args.push("bypassPermissions".into());
            args
        },
        // `claude` has no list-models subcommand; these are its documented
        // aliases, which always resolve, plus two pinned ids (OpenDesign's
        // finding; the picker's *Custom…* row covers anything newer).
        models_args: None,
        fallback_models: &[
            "sonnet",
            "opus",
            "haiku",
            "fable",
            "claude-sonnet-4-5",
            "claude-opus-4-5",
        ],
        resume_args: Some(|session| vec!["--resume".into(), session.into()]),
        install_hint: "install Claude Code — https://claude.com/product/claude-code",
    },
    AgentDef {
        id: "opencode",
        name: "OpenCode",
        // `opencode2` is the V2 binary, and this row's argv (`--auto`, model
        // ids) is written for the V2 CLI. A 1.x install is still a working
        // substitute on a machine that has no V2 build — but it refuses a V2
        // `opencode.json` outright ("V2 permissions are not supported by
        // OpenCode V1"), so it must not shadow the V2 binary when both exist.
        bin: "opencode2",
        fallback_bins: &["opencode", "opencode-cli"],
        version_args: &["--version"],
        stream_format: StreamFormat::OpenCodeStream,
        // `opencode run` with no message argv reads the prompt from stdin
        // (OpenDesign's finding: a literal `-` becomes a message, so omit it).
        delivery: PromptDelivery::StdinText,
        build_args: |invocation| {
            let mut args: Vec<String> = vec![
                "run".into(),
                "--format".into(),
                "json".into(),
                // 2.x headless approval ("auto-approve permissions not
                // explicitly denied"). 1.18-era builds used
                // `--dangerously-skip-permissions`; this table is the fix.
                "--auto".into(),
            ];
            if let Some(model) = invocation.model {
                args.push("-m".into());
                args.push(model.to_string());
            }
            args
        },
        // `opencode models` prints one `provider/model` per line.
        models_args: Some(&["models"]),
        fallback_models: &[],
        // The session id is captured from the stream (`step_start.sessionID`).
        resume_args: Some(|session| vec!["-s".into(), session.into()]),
        install_hint: "install OpenCode — https://opencode.ai",
    },
    AgentDef {
        id: "copilot",
        name: "GitHub Copilot CLI",
        bin: "copilot",
        fallback_bins: &[],
        version_args: &["--version"],
        stream_format: StreamFormat::CopilotStreamJson,
        // Omit `-p` entirely under a non-TTY pipe — Copilot reads stdin.
        delivery: PromptDelivery::StdinText,
        build_args: |invocation| {
            let mut args: Vec<String> = vec![
                "--allow-all-tools".into(),
                "--output-format".into(),
                "json".into(),
            ];
            if let Some(model) = invocation.model {
                args.push("--model".into());
                args.push(model.to_string());
            }
            args
        },
        // No models subcommand; OpenDesign's evidence-based hints.
        models_args: None,
        fallback_models: &["claude-sonnet-4.6", "gpt-5.2"],
        resume_args: None,
        install_hint: "install the Copilot CLI — https://github.com/features/copilot/cli",
    },
    AgentDef {
        id: "antigravity",
        name: "Antigravity",
        bin: "agy",
        fallback_bins: &["antigravity"],
        version_args: &["--version"],
        stream_format: StreamFormat::AntigravityStream,
        delivery: PromptDelivery::StdinAntigravity,
        build_args: |invocation| {
            let mut args: Vec<String> = vec!["--dangerously-skip-permissions".into()];
            if let Some(model) = invocation.model {
                args.push("--model".into());
                args.push(model.to_string());
            }
            args.push("--input-format".into());
            args.push("stream-json".into());
            args.push("--output-format".into());
            args.push("stream-json".into());
            args
        },
        // `agy models` prints `id<TAB>Label` lines (verified against 1.3.3).
        models_args: Some(&["models"]),
        fallback_models: &[],
        resume_args: None,
        install_hint: "install the Antigravity CLI (`agy`) — https://antigravity.google/cli",
    },
    AgentDef {
        id: "gemini",
        name: "Gemini CLI",
        bin: "gemini",
        fallback_bins: &[],
        version_args: &["--version"],
        // verify: Gemini's structured `--output-format` frames differ across
        // releases; until a machine here can confirm the shape, its print mode
        // is read as text (the run still works — the transcript is prose).
        stream_format: StreamFormat::Plain,
        delivery: PromptDelivery::Argv,
        build_args: |invocation| {
            let mut args: Vec<String> = vec!["-p".into(), invocation.prompt.to_string()];
            if let Some(model) = invocation.model {
                args.push("--model".into());
                args.push(model.to_string());
            }
            args.push("--yolo".into());
            args
        },
        models_args: None,
        fallback_models: &[],
        resume_args: None,
        install_hint: "install the Gemini CLI — https://github.com/google-gemini/gemini-cli",
    },
    AgentDef {
        id: "omp",
        name: "Oh My Pi",
        bin: "omp",
        fallback_bins: &["oh-my-pi"],
        version_args: &["--version"],
        // OMP is a Pi fork and drives the same `--mode rpc` protocol; the
        // parser answers its `extension_ui_request` dialogs so it never blocks.
        stream_format: StreamFormat::PiRpc,
        delivery: PromptDelivery::StdinPiRpc,
        build_args: |invocation| {
            let mut args: Vec<String> = vec!["--mode".into(), "rpc".into()];
            if let Some(model) = invocation.model {
                args.push("--model".into());
                args.push(model.to_string());
            }
            args
        },
        // `omp models` prints a boxed table grouped by provider; the parser in
        // `agent.rs` reads the model cell and keeps the provider in the label.
        models_args: Some(&["models"]),
        fallback_models: &[],
        // verify: session resume would ride the RPC `new_session` command; the
        // def stays fresh-per-turn until a binary here confirms it.
        resume_args: None,
        install_hint: "install Oh My Pi (a Pi fork) and make `omp` available on PATH",
    },
    AgentDef {
        id: "pi",
        name: "Pi",
        bin: "pi",
        fallback_bins: &[],
        version_args: &["--version"],
        stream_format: StreamFormat::PiRpc,
        delivery: PromptDelivery::StdinPiRpc,
        build_args: |invocation| {
            let mut args: Vec<String> = vec!["--mode".into(), "rpc".into()];
            if let Some(model) = invocation.model {
                args.push("--model".into());
                args.push(model.to_string());
            }
            args
        },
        // `pi --list-models` prints a TSV table whose first two columns are
        // `provider` and `model`.
        models_args: Some(&["--list-models"]),
        fallback_models: &[],
        resume_args: None,
        install_hint: "install pi — https://github.com/badlogic/pi-mono",
    },
];

/// A user-defined profile from `data_dir/agent/profiles.json` — the escape
/// hatch for any CLI the table does not name. `{prompt}` and `{model}` in an
/// arg are substituted (argv delivery; profiles get no stdin protocol).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomProfile {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub bin: String,
    /// argv after the binary; `{prompt}` and `{model}` are substituted.
    pub args: Vec<String>,
    #[serde(default)]
    pub stream_format: Option<String>,
    /// Model ids this profile accepts, for the picker. `{model}` in `args`
    /// places the choice; without it the picker hides the model control.
    #[serde(default)]
    pub models: Vec<String>,
}

impl CustomProfile {
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("{} (custom)", self.id))
    }

    pub fn stream_format(&self) -> StreamFormat {
        match self.stream_format.as_deref() {
            Some("claude-stream-json") => StreamFormat::ClaudeStreamJson,
            Some("opencode-stream") => StreamFormat::OpenCodeStream,
            Some("copilot-stream-json") => StreamFormat::CopilotStreamJson,
            Some("antigravity-stream") => StreamFormat::AntigravityStream,
            Some("pi-rpc") => StreamFormat::PiRpc,
            _ => StreamFormat::Plain,
        }
    }

    /// Whether this profile's argv can carry a model choice.
    pub fn takes_model(&self) -> bool {
        self.args.iter().any(|arg| arg.contains("{model}"))
    }

    pub fn build_args(&self, prompt: &str, model: Option<&str>) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| {
                arg.replace("{prompt}", prompt)
                    .replace("{model}", model.unwrap_or(""))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invocation<'a>(prompt: &'a str, model: Option<&'a str>) -> Invocation<'a> {
        Invocation { prompt, model }
    }

    #[test]
    fn def_ids_are_unique() {
        let mut ids: Vec<&str> = AGENT_DEFS.iter().map(|def| def.id).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "two defs share an id");
    }

    #[test]
    fn every_def_builds_a_non_empty_argv() {
        for def in AGENT_DEFS {
            let args = (def.build_args)(&invocation("hi", None));
            assert!(!args.is_empty(), "{} builds no argv", def.id);
            if def.delivery == PromptDelivery::Argv {
                assert!(
                    args.iter().any(|arg| arg.contains("hi")),
                    "{} drops the prompt",
                    def.id
                );
            }
        }
    }

    #[test]
    fn a_model_choice_reaches_every_argv_that_takes_one() {
        let args = (AGENT_DEFS[0].build_args)(&invocation("hi", Some("sonnet")));
        assert!(
            args.iter().any(|arg| arg == "sonnet"),
            "claude drops the model"
        );
        let args = (AGENT_DEFS[1].build_args)(&invocation("hi", Some("anthropic/x")));
        assert!(
            args.iter().any(|arg| arg == "anthropic/x"),
            "opencode drops the model"
        );
        // The headless posture flag still lands when a model is chosen.
        assert!(args.iter().any(|arg| arg == "--auto"));
    }

    #[test]
    fn resume_args_pair_the_flag_with_the_id() {
        let def = AGENT_DEFS.iter().find(|def| def.id == "claude").unwrap();
        let args = (def.resume_args.unwrap())("ses-123");
        assert_eq!(args, vec!["--resume".to_string(), "ses-123".to_string()]);
    }

    #[test]
    fn custom_profiles_substitute_the_prompt_and_the_model() {
        let profile = CustomProfile {
            id: "mine".into(),
            name: None,
            bin: "mine".into(),
            args: vec![
                "chat".into(),
                "--print".into(),
                "{prompt}".into(),
                "--model".into(),
                "{model}".into(),
            ],
            stream_format: Some("plain".into()),
            models: vec!["fast".into()],
        };
        assert_eq!(
            profile.build_args("hello", Some("fast")),
            vec!["chat", "--print", "hello", "--model", "fast"]
        );
        assert!(profile.takes_model());
        assert_eq!(profile.stream_format(), StreamFormat::Plain);
    }
}
