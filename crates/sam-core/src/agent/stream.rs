//! Stream parsers (§4.7, extended by the assistant): one line of an agent
//! CLI's stdout becomes zero or more [`AgentEvent`]s.
//!
//! The mappings are not guesses. Each branch below is the translation of the
//! parser OpenDesign ships in production for the same CLI
//! (`docs/agent-adapters.md` → `claude-stream.ts`, `json-event-stream.ts`,
//! `copilot-stream.ts`, `agent-protocol/pi-rpc/events.ts`), narrowed to SAM's
//! smaller event vocabulary. When a CLI changes its frames, this file is where
//! the fix lands, one branch at a time.
//!
//! Unknown frames are not dropped silently: a JSON line no branch recognises
//! becomes [`AgentEvent::Log`], so a drifting protocol shows up in the deck
//! instead of vanishing.

use std::collections::HashSet;
use std::io::Write;
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::AgentEvent;

/// How a CLI's stdout is framed, and which branch reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamFormat {
    /// Claude Code `-p --output-format stream-json` JSONL.
    ClaudeStreamJson,
    /// OpenCode `run --format json` JSONL (`{type, sessionID, part}` frames).
    OpenCodeStream,
    /// GitHub Copilot CLI `--output-format json` JSONL (`assistant.*` frames).
    CopilotStreamJson,
    /// Antigravity `agy … --output-format stream-json` NDJSON (`{event, …}` frames).
    AntigravityStream,
    /// Pi / Oh My Pi `--mode rpc` command/event lines.
    PiRpc,
    /// Anything else: each stdout line is transcript text.
    Plain,
}

/// The child's stdin, shared between the writer and a parser that must answer
/// mid-stream (Pi's `extension_ui_request`), so `PiRpc` does not deadlock.
pub type StdinSlot = Arc<Mutex<Option<ChildStdin>>>;

/// One run's parser state. Cheap to construct; owns only the dedup sets a
/// protocol needs to emit one row per call.
pub struct StreamParser {
    format: StreamFormat,
    seen_tool_uses: HashSet<String>,
    seen_tool_results: HashSet<String>,
    antigravity_text_emitted: bool,
    /// Claude only: `stream_event` deltas already streamed this message's
    /// text, so the `assistant` frame that repeats it whole is skipped (the
    /// tools it carries are still read).
    claude_partials: bool,
}

impl StreamParser {
    pub fn new(format: StreamFormat) -> Self {
        Self {
            format,
            seen_tool_uses: HashSet::new(),
            seen_tool_results: HashSet::new(),
            antigravity_text_emitted: false,
            claude_partials: false,
        }
    }

    /// Parse one stdout line. `stdin` is the live child stdin when the
    /// protocol needs to answer a frame before more output will come.
    pub fn feed(&mut self, line: &str, stdin: Option<&StdinSlot>) -> Vec<AgentEvent> {
        if self.format == StreamFormat::Plain {
            return vec![AgentEvent::Text {
                text: format!("{}\n", strip_ansi(line)),
            }];
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![AgentEvent::Log {
                line: clip(&strip_ansi(line), 500),
            }];
        };
        let mut events = Vec::new();
        match self.format {
            StreamFormat::ClaudeStreamJson => self.claude(&value, &mut events),
            StreamFormat::OpenCodeStream => self.opencode(&value, &mut events),
            StreamFormat::CopilotStreamJson => self.copilot(&value, &mut events),
            StreamFormat::AntigravityStream => self.antigravity(&value, &mut events),
            StreamFormat::PiRpc => self.pi(&value, stdin, &mut events),
            StreamFormat::Plain => unreachable!("handled above"),
        }
        events
    }
}

impl StreamParser {
    /// Claude Code `-p --output-format stream-json --verbose
    /// --include-partial-messages` frames (`claude-stream.ts`). Text and
    /// thinking arrive twice — once as `stream_event` token deltas, once
    /// whole in the `assistant` message wrapper — and are drawn once: the
    /// deltas win, and the wrapper's text is skipped for that message. A
    /// build without partial messages (no `stream_event` frames at all)
    /// falls back to reading the wrapper whole.
    fn claude(&mut self, value: &Value, out: &mut Vec<AgentEvent>) {
        match str_of(value, "type") {
            // `system`/`init` opens the turn and is the one frame that names
            // the session — captured so the next deck turn can `--resume` it.
            Some("system") if str_of(value, "subtype") == Some("init") => {
                if let Some(id) = non_empty(str_of(value, "session_id")) {
                    out.push(AgentEvent::Session { id });
                }
            }
            Some("stream_event") => {
                let Some(event) = value.get("event") else { return };
                // A new message starts with a clean slate: the dedup flag
                // belongs to *this* message's text, not to whatever the last
                // one happened to stream. The `assistant` wrapper also clears
                // it, but a protocol can interleave frames, and a flag left
                // set would drop the next message's prose.
                if str_of(event, "type") == Some("message_start") {
                    self.claude_partials = false;
                    return;
                }
                if str_of(event, "type") != Some("content_block_delta") {
                    return;
                }
                let Some(delta) = event.get("delta") else { return };
                match str_of(delta, "type") {
                    Some("text_delta") => {
                        if let Some(text) = non_empty(str_of(delta, "text")) {
                            self.claude_partials = true;
                            out.push(AgentEvent::Text { text });
                        }
                    }
                    Some("thinking_delta") => {
                        if let Some(text) = non_empty(str_of(delta, "thinking")) {
                            self.claude_partials = true;
                            out.push(AgentEvent::Thinking { text });
                        }
                    }
                    // `input_json_delta` / `signature_delta`: the tools are
                    // read complete from the `assistant` frame.
                    _ => {}
                }
            }
            Some("assistant") => {
                if let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array) {
                    for block in blocks {
                        match str_of(block, "type") {
                            Some("text") => {
                                if self.claude_partials {
                                    continue;
                                }
                                if let Some(text) = non_empty(str_of(block, "text")) {
                                    out.push(AgentEvent::Text { text });
                                }
                            }
                            Some("thinking") => {
                                if self.claude_partials {
                                    continue;
                                }
                                if let Some(text) = non_empty(str_of(block, "thinking")) {
                                    out.push(AgentEvent::Thinking { text });
                                }
                            }
                            Some("tool_use") => {
                                let name = str_of(block, "name").unwrap_or_default().to_string();
                                let input = block.get("input").cloned().unwrap_or(Value::Null);
                                if is_write_tool(&name) {
                                    if let Some(path) = write_path(&input) {
                                        out.push(AgentEvent::File { path });
                                    }
                                }
                                out.push(AgentEvent::Tool {
                                    id: str_owned(block, "id"),
                                    name,
                                    summary: tool_summary(&input),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                // The frame closes the message's text delivery either way.
                self.claude_partials = false;
            }
            Some("user") => {
                let Some(blocks) = value.pointer("/message/content").and_then(Value::as_array)
                else {
                    return;
                };
                for block in blocks {
                    if str_of(block, "type") != Some("tool_result") {
                        continue;
                    }
                    out.push(AgentEvent::ToolResult {
                        tool_use_id: str_owned(block, "tool_use_id"),
                        is_error: block.get("is_error").and_then(Value::as_bool).unwrap_or(false),
                        content: clip(&stringify(block.get("content")), 2000),
                    });
                }
            }
            Some("result") => {
                let is_error = value.get("is_error").and_then(Value::as_bool).unwrap_or(false);
                if is_error {
                    out.push(AgentEvent::Error {
                        message: clip(&claude_result_message(value), 2000),
                    });
                } else if let Some(summary) = non_empty(str_of(value, "result")) {
                    out.push(AgentEvent::Result {
                        summary: clip(&summary, 2000),
                    });
                }
            }
            _ => {}
        }
    }

    /// OpenCode `run --format json` frames (`json-event-stream.ts` →
    /// `handleOpenCodeEvent`): `{type, sessionID, part}` envelopes.
    fn opencode(&mut self, value: &Value, out: &mut Vec<AgentEvent>) {
        let part = value.get("part").cloned().unwrap_or(Value::Null);
        match str_of(value, "type") {
            Some("text") => {
                if let Some(text) = non_empty(part.pointer("/text").and_then(Value::as_str)) {
                    out.push(AgentEvent::Text { text });
                }
            }
            Some("tool_use") => {
                let name = str_of(&part, "tool").unwrap_or_default().to_string();
                let call_id = str_owned(&part, "callID");
                let key = format!(
                    "{}:{}",
                    str_of(value, "sessionID").unwrap_or("session"),
                    call_id
                );
                let state = part.get("state").cloned().unwrap_or(Value::Null);
                // `state.input` may be an object or a JSON string.
                let input = match state.get("input") {
                    Some(Value::String(text)) => {
                        serde_json::from_str(text).unwrap_or(Value::String(text.clone()))
                    }
                    Some(other) => other.clone(),
                    None => Value::Null,
                };
                if self.seen_tool_uses.insert(key.clone()) {
                    if is_write_tool(&name) {
                        if let Some(path) = write_path(&input) {
                            out.push(AgentEvent::File { path });
                        }
                    }
                    out.push(AgentEvent::Tool {
                        id: call_id.clone(),
                        name,
                        summary: tool_summary(&input),
                    });
                }
                let status = str_of(&state, "status").unwrap_or("").to_ascii_lowercase();
                if matches!(status.as_str(), "completed" | "error" | "failed")
                    && self.seen_tool_results.insert(key)
                {
                    let error = state.get("error").filter(|e| !e.is_null());
                    let exit_non_zero = ["exit", "exitCode"]
                        .iter()
                        .filter_map(|key| state.get(*key).and_then(Value::as_i64))
                        .chain(
                            state
                                .pointer("/metadata/exit")
                                .and_then(Value::as_i64)
                                .into_iter(),
                        )
                        .any(|code| code != 0);
                    let is_error = matches!(status.as_str(), "error" | "failed")
                        || error.is_some()
                        || exit_non_zero;
                    let body = error
                        .map(|value| message_of(value, ""))
                        .filter(|message| !message.is_empty())
                        .unwrap_or_else(|| stringify(state.get("output")));
                    out.push(AgentEvent::ToolResult {
                        tool_use_id: call_id,
                        is_error,
                        content: clip(&body, 2000),
                    });
                }
            }
            Some("file") | Some("patch") => {
                if let Some(path) = non_empty(
                    part.get("path")
                        .or_else(|| value.get("path"))
                        .and_then(Value::as_str),
                ) {
                    out.push(AgentEvent::File { path });
                }
            }
            Some("error") => {
                let message = message_of(value.get("error").unwrap_or(value), "OpenCode error");
                out.push(AgentEvent::Error { message });
            }
            Some("step_start") => {
                // The turn opener carries OpenCode's own session handle
                // (`ses_…`) — captured so the next deck turn can `-s` it.
                if let Some(id) = non_empty(str_of(value, "sessionID")) {
                    out.push(AgentEvent::Session { id });
                }
            }
            Some("step_finish") | Some("session") => {}
            _ => out.push(AgentEvent::Log {
                line: clip(&value.to_string(), 500),
            }),
        }
    }

    /// GitHub Copilot CLI `--output-format json` frames (`copilot-stream.ts`):
    /// dotted top-level types with the payload under `data`.
    fn copilot(&self, value: &Value, out: &mut Vec<AgentEvent>) {
        let data = value.get("data").cloned().unwrap_or(Value::Null);
        match str_of(value, "type") {
            Some("assistant.reasoning_delta") => {
                if let Some(text) = non_empty(str_of(&data, "deltaContent")) {
                    out.push(AgentEvent::Thinking { text });
                }
            }
            Some("assistant.message_delta") => {
                if let Some(text) = non_empty(str_of(&data, "deltaContent")) {
                    out.push(AgentEvent::Text { text });
                }
            }
            Some("tool.execution_start") => {
                let name = str_of(&data, "toolName").unwrap_or_default().to_string();
                let input = data.get("arguments").cloned().unwrap_or(Value::Null);
                if is_write_tool(&name) {
                    if let Some(path) = write_path(&input) {
                        out.push(AgentEvent::File { path });
                    }
                }
                out.push(AgentEvent::Tool {
                    id: str_owned(&data, "toolCallId"),
                    name,
                    summary: tool_summary(&input),
                });
            }
            Some("tool.execution_complete") => {
                out.push(AgentEvent::ToolResult {
                    tool_use_id: str_owned(&data, "toolCallId"),
                    is_error: data.get("success").and_then(Value::as_bool) == Some(false),
                    content: clip(&stringify(data.get("result")), 2000),
                });
            }
            Some("result") => {
                let ok = value.get("success").and_then(Value::as_bool) == Some(true)
                    || value.get("exitCode").and_then(Value::as_i64) == Some(0);
                if !ok {
                    out.push(AgentEvent::Error {
                        message: message_of(&value, "Copilot run ended without success"),
                    });
                }
            }
            _ => {} // session.* / user.* / turn boundaries carry nothing SAM draws
        }
    }

    /// Antigravity `agy --output-format stream-json` frames
    /// (`json-event-stream.ts` → `handleAntigravityEvent`).
    fn antigravity(&mut self, value: &Value, out: &mut Vec<AgentEvent>) {
        match str_of(value, "event") {
            Some("step_update") => {
                let Some(step) = value.get("step_update") else {
                    return;
                };
                match str_of(step, "step_type") {
                    Some("agent_response") => {
                        if let Some(text) = non_empty(str_of(step, "text_delta")) {
                            self.antigravity_text_emitted = true;
                            out.push(AgentEvent::Text { text });
                        }
                    }
                    Some("tool") => {
                        let tool = step.get("tool_info").cloned().unwrap_or(Value::Null);
                        let name = str_of(&tool, "name")
                            .or_else(|| str_of(step, "tool_name"))
                            .unwrap_or("tool");
                        let id = format!(
                            "{}:{}",
                            step.get("conversation_id")
                                .or_else(|| value.get("conversation_id"))
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            stringify(step.get("step_index"))
                        );
                        let input = tool.get("parameters").cloned().unwrap_or(Value::Null);
                        if self.seen_tool_uses.insert(id.clone()) {
                            out.push(AgentEvent::Tool {
                                id: id.clone(),
                                name: name.to_string(),
                                summary: tool_summary(&input),
                            });
                        }
                        let state = str_of(step, "state").unwrap_or("");
                        if matches!(state, "DONE" | "ERROR") && self.seen_tool_results.insert(id.clone()) {
                            if let Some(path) = write_path(&input) {
                                out.push(AgentEvent::File { path });
                            }
                            out.push(AgentEvent::ToolResult {
                                tool_use_id: id,
                                is_error: state == "ERROR",
                                content: clip(&stringify(tool.get("output")), 2000),
                            });
                        }
                    }
                    _ => {} // user_input / checkpoint / subagent echo no user-visible payload
                }
            }
            Some("result") => {
                let Some(result) = value.get("result") else {
                    return;
                };
                if str_of(result, "status") != Some("SUCCESS") {
                    let message = non_empty(str_of(result, "error")).unwrap_or_else(|| {
                        format!(
                            "Antigravity run ended with status {}",
                            str_of(result, "status").unwrap_or("unknown")
                        )
                    });
                    out.push(AgentEvent::Error { message });
                    return;
                }
                if !self.antigravity_text_emitted {
                    if let Some(text) = non_empty(str_of(result, "response")) {
                        self.antigravity_text_emitted = true;
                        out.push(AgentEvent::Text { text });
                    }
                }
            }
            Some("init") => {}
            _ => {}
        }
    }

    /// Pi / Oh My Pi `--mode rpc` lines (`agent-protocol/pi-rpc/events.ts`).
    /// The only branch with an outbound half: an `extension_ui_request` must
    /// be answered or the agent blocks on a dialog no one can see.
    fn pi(&self, value: &Value, stdin: Option<&StdinSlot>, out: &mut Vec<AgentEvent>) {
        match str_of(value, "type") {
            Some("extension_ui_request") => {
                if let Some(stdin) = stdin {
                    if let Some(note) = answer_extension_ui(stdin, value) {
                        out.push(AgentEvent::Log { line: note });
                    }
                }
            }
            Some("response") => {
                if value.get("success").and_then(Value::as_bool) == Some(false) {
                    out.push(AgentEvent::Error {
                        message: message_of(
                            value.get("error").unwrap_or(&Value::Null),
                            "Pi rejected the request",
                        ),
                    });
                }
            }
            Some("turn_end") => {
                let message = value.get("message").cloned().unwrap_or(Value::Null);
                if str_of(&message, "stopReason") == Some("error") {
                    let text = non_empty(str_of(&message, "errorMessage"))
                        .unwrap_or_else(|| "Pi agent error".into());
                    out.push(AgentEvent::Error { message: text });
                }
            }
            Some("message_update") => {
                let Some(event) = value.get("assistantMessageEvent") else {
                    return;
                };
                match str_of(event, "type") {
                    Some("text_delta") => {
                        if let Some(text) = non_empty(str_of(event, "delta")) {
                            out.push(AgentEvent::Text { text });
                        }
                    }
                    Some("thinking_delta") => {
                        if let Some(text) = non_empty(str_of(event, "delta")) {
                            out.push(AgentEvent::Thinking { text });
                        }
                    }
                    Some("error") => {
                        let message = non_empty(str_of(event, "reason"))
                            .or_else(|| non_empty(str_of(event, "delta")))
                            .unwrap_or_else(|| "Agent error".into());
                        out.push(AgentEvent::Error { message });
                    }
                    _ => {}
                }
            }
            Some("tool_execution_start") => {
                let name = str_of(value, "toolName").unwrap_or_default().to_string();
                let args = value.get("args").cloned().unwrap_or(Value::Null);
                if is_write_tool(&name) {
                    if let Some(path) = write_path(&args) {
                        out.push(AgentEvent::File { path });
                    }
                }
                out.push(AgentEvent::Tool {
                    id: str_owned(value, "toolCallId"),
                    name,
                    summary: tool_summary(&args),
                });
            }
            Some("tool_execution_end") => {
                out.push(AgentEvent::ToolResult {
                    tool_use_id: str_owned(value, "toolCallId"),
                    is_error: value.get("isError").and_then(Value::as_bool).unwrap_or(false),
                    content: clip(&pi_result_text(value.get("result")), 2000),
                });
            }
            Some("extension_error") => {
                let message = non_empty(str_of(value, "error"))
                    .unwrap_or_else(|| "Extension error".into());
                out.push(AgentEvent::Error { message });
            }
            Some("auto_retry_end") => {
                if value.get("success").and_then(Value::as_bool) == Some(false) {
                    let message = non_empty(str_of(value, "finalError"))
                        .unwrap_or_else(|| "Auto-retry exhausted".into());
                    out.push(AgentEvent::Error { message });
                }
            }
            Some("agent_end") => {
                // Pi's RPC process outlives the turn by design; closing stdin
                // is the protocol's end signal (the supervisor owns the Exit
                // event and any grace-period kill).
                if let Some(stdin) = stdin {
                    if let Ok(mut guard) = stdin.lock() {
                        guard.take();
                    }
                }
            }
            // Lifecycle frames SAM draws no row for: agent_start, turn_start,
            // message_end, compaction_start, auto_retry_start.
            _ => {}
        }
    }
}

/// Answer a Pi `extension_ui_request` the way OpenDesign does: dialogs are
/// auto-resolved (confirm → true, select → first option), fire-and-forget
/// notifications are consumed silently. Every answered dialog returns the line
/// the transcript draws, because an approval nobody saw is a decision the
/// student is entitled to read afterwards.
fn answer_extension_ui(stdin: &StdinSlot, raw: &Value) -> Option<String> {
    let id = raw.get("id")?;
    let method = str_of(raw, "method").unwrap_or("");
    if ["setStatus", "setWidget", "notify", "setTitle", "set_editor_text"].contains(&method) {
        return None;
    }
    let note;
    let reply = if method == "confirm" {
        note = format!("the agent's `{method}` dialog was answered \"yes\" (there is no window to ask in)");
        json!({ "type": "extension_ui_response", "id": id, "confirmed": true })
    } else {
        let options = raw
            .pointer("/params/options")
            .or_else(|| raw.get("options"))
            .and_then(Value::as_array);
        match options.and_then(|options| options.first()) {
            Some(Value::String(first)) => {
                note = format!(
                    "the agent's `{method}` dialog was answered \"{first}\" (the first option)"
                );
                json!({ "type": "extension_ui_response", "id": id, "value": first })
            }
            Some(Value::Object(first)) => {
                let value = first.get("label").or_else(|| first.get("value")).cloned();
                let chosen = value
                    .as_ref()
                    .map(|value| match value {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default();
                note = format!(
                    "the agent's `{method}` dialog was answered \"{chosen}\" (the first option)"
                );
                json!({
                    "type": "extension_ui_response",
                    "id": id,
                    "value": value.unwrap_or(Value::String(String::new())),
                })
            }
            _ => {
                note = format!("the agent's `{method}` dialog had no options and was dismissed");
                json!({ "type": "extension_ui_response", "id": id, "cancelled": true })
            }
        }
    };
    if let Ok(mut guard) = stdin.lock() {
        if let Some(handle) = guard.as_mut() {
            let _ = writeln!(handle, "{reply}");
            let _ = handle.flush();
        }
    }
    Some(note)
}

/// Pi's `tool_execution_end.result.content` is an array of `{type:"text"}` blocks.
fn pi_result_text(result: Option<&Value>) -> String {
    let Some(content) = result.and_then(|result| result.get("content")) else {
        return String::new();
    };
    match content {
        Value::Array(items) => items
            .iter()
            .map(|item| match str_of(item, "type") {
                Some("text") => stringify(item.get("text")),
                _ => stringify(Some(item)),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::String(text) => text.clone(),
        other => stringify(Some(other)),
    }
}

/// Claude's `result` frame: `errors[]`, else `result`, else the subtype.
fn claude_result_message(value: &Value) -> String {
    if let Some(errors) = value.get("errors").and_then(Value::as_array) {
        let parts: Vec<String> = errors
            .iter()
            .filter_map(Value::as_str)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        if !parts.is_empty() {
            return parts.join("\n");
        }
    }
    non_empty(str_of(value, "result"))
        .or_else(|| str_of(value, "subtype").map(|subtype| format!("Claude run failed: {subtype}")))
        .unwrap_or_else(|| "Claude run failed".into())
}

/// The generic message extractor (`json-event-stream.ts`): unwraps nested
/// `{error:{data:{error:{message}}}}` shapes and JSON-within-a-string.
fn message_of(value: &Value, fallback: &str) -> String {
    match value {
        Value::String(text) => {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                if parsed.is_object() {
                    return message_of(&parsed, text);
                }
            }
            text.clone()
        }
        Value::Object(map) => {
            if let Some(detail) = map.get("detail").and_then(Value::as_str).filter(|s| !s.is_empty())
            {
                return detail.to_string();
            }
            if let Some(message) = map.get("message") {
                if !message.is_null() {
                    return message_of(message, fallback);
                }
            }
            if let Some(error) = map.get("error") {
                if !error.is_null() {
                    return message_of(error, fallback);
                }
            }
            if let Some(data) = map.get("data") {
                let nested = message_of(data, "");
                if !nested.is_empty() {
                    return nested;
                }
            }
            if let Some(name) = map.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                return name.to_string();
            }
            fallback.to_string()
        }
        _ => fallback.to_string(),
    }
}

fn str_of<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn str_owned(value: &Value, key: &str) -> String {
    str_of(value, key).unwrap_or_default().to_string()
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value.filter(|text| !text.is_empty()).map(str::to_string)
}

fn stringify(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// One line for the deck's tool row: the argument a human would name the call
/// by — the shell command, the file, the pattern — else compact JSON.
fn tool_summary(input: &Value) -> String {
    if let Some(map) = input.as_object() {
        for key in [
            "command",
            "file_path",
            "filePath",
            "path",
            "pattern",
            "query",
            "url",
            "description",
            "prompt",
        ] {
            if let Some(text) = map.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()) {
                return clip(text, 200);
            }
        }
    }
    if input.is_null() {
        return String::new();
    }
    clip(&input.to_string(), 200)
}

/// Tools that write a file — the set the deck turns into a file-change row.
fn is_write_tool(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "write" | "edit" | "multiedit" | "notebookedit" | "write_file" | "replace" | "patch"
    )
}

fn write_path(input: &Value) -> Option<String> {
    ["file_path", "filePath", "path"]
        .iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

/// Bound every payload at the source, so the event stream, the deck's memory
/// and the transcript hold the same clipped line.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut clipped: String = text.chars().take(max).collect();
    clipped.push('…');
    clipped
}

/// Drop ANSI escape sequences from a CLI's own diagnostics. A tool that
/// colourises its stderr (`\x1b[91m\x1b[1mError:\x1b[0m …`) is not saying
/// anything about colour to the student; it is saying "error". The deck draws
/// text, not a terminal, so the escapes would otherwise be read as literal
/// bracket noise. Both forms a CLI emits are handled: CSI (`ESC [ … final`)
/// and OSC (`ESC ] … BEL | ESC \`).
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                // Parameter and intermediate bytes, ended by a final byte in
                // 0x40..=0x7e (`m`, `K`, `H`, …).
                for code in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&code) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(code) = chars.next() {
                    if code == '\u{7}' {
                        break;
                    }
                    if code == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            // A two-byte designator escape (`ESC (`), or a lone ESC: the
            // former takes its second byte with it, the latter only itself —
            // a stray ESC must not eat the character that follows it.
            Some(next) if matches!(next, '(' | ')' | '*' | '+' | '#' | '%' | '-' | '.' | '/') => {
                chars.next();
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(format: StreamFormat, line: &str) -> Vec<AgentEvent> {
        StreamParser::new(format).feed(line, None)
    }

    #[test]
    fn claude_assistant_text_and_tool() {
        let line = r#"{"type":"assistant","message":{"content":[
            {"type":"text","text":"Writing the batch."},
            {"type":"tool_use","id":"t1","name":"Bash","input":{"command":"sam apply /tmp/b.jsonl --json"}}
        ]}}"#;
        let events = feed(StreamFormat::ClaudeStreamJson, line);
        assert!(matches!(&events[0], AgentEvent::Text { text } if text == "Writing the batch."));
        assert!(matches!(&events[1], AgentEvent::Tool { name, summary, .. }
            if name == "Bash" && summary == "sam apply /tmp/b.jsonl --json"));
    }

    #[test]
    fn claude_write_tool_names_its_file() {
        let line = r#"{"type":"assistant","message":{"content":[
            {"type":"tool_use","id":"t2","name":"Write","input":{"file_path":"content/records/topic.jsonl","content":"…"}}
        ]}}"#;
        let events = feed(StreamFormat::ClaudeStreamJson, line);
        assert!(events.iter().any(|event| matches!(event, AgentEvent::File { path } if path == "content/records/topic.jsonl")));
    }

    #[test]
    fn claude_error_result() {
        let line = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"errors":["boom"]}"#;
        let events = feed(StreamFormat::ClaudeStreamJson, line);
        assert!(matches!(&events[0], AgentEvent::Error { message } if message == "boom"));
    }

    #[test]
    fn opencode_text_and_tool_pair() {
        let text = r#"{"type":"text","sessionID":"ses_1","part":{"text":"On it."}}"#;
        assert!(matches!(&feed(StreamFormat::OpenCodeStream, text)[0], AgentEvent::Text { text } if text == "On it."));
        let tool = r#"{"type":"tool_use","sessionID":"ses_1","part":{"tool":"bash","callID":"c1",
            "state":{"status":"completed","input":{"command":"sam apply b.jsonl"},"output":"ok"}}}"#;
        let events = feed(StreamFormat::OpenCodeStream, tool);
        assert!(matches!(&events[0], AgentEvent::Tool { name, summary, .. } if name == "bash" && summary == "sam apply b.jsonl"));
        assert!(matches!(&events[1], AgentEvent::ToolResult { is_error, .. } if !is_error));
    }

    #[test]
    fn opencode_error_frame() {
        let line = r#"{"type":"error","sessionID":"ses_1","error":{"type":"provider.invalid-request","message":"model not supported"}}"#;
        let events = feed(StreamFormat::OpenCodeStream, line);
        assert!(matches!(&events[0], AgentEvent::Error { message } if message == "model not supported"));
    }

    #[test]
    fn copilot_deltas_and_result() {
        let text = r#"{"type":"assistant.message_delta","data":{"deltaContent":"Working"}}"#;
        assert!(matches!(&feed(StreamFormat::CopilotStreamJson, text)[0], AgentEvent::Text { .. }));
        let failed = r#"{"type":"result","success":false,"exitCode":1}"#;
        assert!(matches!(&feed(StreamFormat::CopilotStreamJson, failed)[0], AgentEvent::Error { .. }));
    }

    #[test]
    fn antigravity_tool_then_result() {
        let step = r#"{"event":"step_update","conversation_id":"c1","step_update":{"step_type":"tool","step_index":2,"state":"DONE","tool_info":{"name":"bash","parameters":{"command":"sam paths"},"output":"ok"}}}"#;
        let events = feed(StreamFormat::AntigravityStream, step);
        assert!(matches!(&events[0], AgentEvent::Tool { name, .. } if name == "bash"));
        assert!(matches!(&events[1], AgentEvent::ToolResult { .. }));
        let result = r#"{"event":"result","result":{"status":"SUCCESS","response":"Done."}}"#;
        assert!(matches!(&feed(StreamFormat::AntigravityStream, result)[0], AgentEvent::Text { text } if text == "Done."));
    }

    #[test]
    fn pi_message_update_and_tools() {
        let text = r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"hello"}}"#;
        assert!(matches!(&feed(StreamFormat::PiRpc, text)[0], AgentEvent::Text { text } if text == "hello"));
        let tool = r#"{"type":"tool_execution_start","toolCallId":"c1","toolName":"bash","args":{"command":"sam configcheck"}}"#;
        let events = feed(StreamFormat::PiRpc, tool);
        assert!(matches!(&events[0], AgentEvent::Tool { summary, .. } if summary == "sam configcheck"));
        let end = r#"{"type":"tool_execution_end","toolCallId":"c1","isError":false,"result":{"content":[{"type":"text","text":"clean"}]}}"#;
        assert!(matches!(&feed(StreamFormat::PiRpc, end)[0], AgentEvent::ToolResult { content, .. } if content == "clean"));
    }

    #[test]
    fn plain_lines_are_text_and_unknown_json_is_visible() {
        let events = feed(StreamFormat::Plain, "just a line");
        assert!(matches!(&events[0], AgentEvent::Text { text } if text == "just a line\n"));
        let events = feed(StreamFormat::OpenCodeStream, r#"{"type":"something-new"}"#);
        assert!(matches!(&events[0], AgentEvent::Log { .. }));
    }

    #[test]
    fn claude_partial_messages_are_drawn_once() {
        let mut parser = StreamParser::new(StreamFormat::ClaudeStreamJson);
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}}"#;
        let events = parser.feed(delta, None);
        assert!(matches!(&events[0], AgentEvent::Text { text } if text == "Hel"));
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"lo"}}}"#;
        let events = parser.feed(delta, None);
        assert!(matches!(&events[0], AgentEvent::Text { text } if text == "lo"));

        // The whole message follows; its text was already streamed, but its
        // tool call is the only place the tools appear.
        let whole = r#"{"type":"assistant","message":{"content":[
            {"type":"text","text":"Hello"},
            {"type":"tool_use","id":"t1","name":"Bash","input":{"command":"sam paths"}}
        ]}}"#;
        let events = parser.feed(whole, None);
        assert!(
            !events.iter().any(|event| matches!(event, AgentEvent::Text { .. })),
            "text streamed as deltas must not repeat: {events:?}"
        );
        assert!(events
            .iter()
            .any(|event| matches!(event, AgentEvent::Tool { name, .. } if name == "Bash")));

        // A message with no partials (an older build, or a non-partial frame)
        // falls back to the whole wrapper text.
        let whole = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Done."}]}}"#;
        let events = parser.feed(whole, None);
        assert!(matches!(&events[0], AgentEvent::Text { text } if text == "Done."));
    }

    #[test]
    fn claude_partial_thinking_is_thinking_and_tool_deltas_draw_nothing() {
        let mut parser = StreamParser::new(StreamFormat::ClaudeStreamJson);
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}}"#;
        let events = parser.feed(delta, None);
        assert!(matches!(&events[0], AgentEvent::Thinking { text } if text == "hmm"));
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"a\":"}}}"#;
        assert!(
            parser.feed(delta, None).is_empty(),
            "tool JSON deltas are the assistant frame's business"
        );
        // Lifecycle frames carry nothing to draw.
        let start = r#"{"type":"stream_event","event":{"type":"message_start","message":{"id":"m1"}}}"#;
        assert!(parser.feed(start, None).is_empty());
    }

    #[test]
    fn a_new_message_clears_the_dedup_flag_of_the_last_one() {
        // Message 1 streams its prose as deltas; the wrapper that repeats it is
        // skipped. Message 2 opens with `message_start` and *no* deltas (a
        // tool-only message or a build without partials): its wrapper text must
        // be drawn, not mistaken for a repeat of message 1.
        let mut parser = StreamParser::new(StreamFormat::ClaudeStreamJson);
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"First."}}}"#;
        assert!(matches!(&parser.feed(delta, None)[0], AgentEvent::Text { text } if text == "First."));
        let start = r#"{"type":"stream_event","event":{"type":"message_start","message":{"id":"m2"}}}"#;
        assert!(parser.feed(start, None).is_empty());
        let whole = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Second."}]}}"#;
        let events = parser.feed(whole, None);
        assert!(
            events.iter().any(|event| matches!(event, AgentEvent::Text { text } if text == "Second.")),
            "the new message's prose was dropped by the previous message's flag: {events:?}"
        );
    }

    #[test]
    fn a_cli_colour_code_is_not_part_of_its_message() {
        // The exact shape OpenCode writes when its config is refused.
        assert_eq!(
            strip_ansi("\u{1b}[91m\u{1b}[1mError: \u{1b}[0mConfiguration is invalid"),
            "Error: Configuration is invalid"
        );
        assert_eq!(strip_ansi("plain text"), "plain text");
        // A hyperlink OSC (title, then the text, then the reset).
        assert_eq!(
            strip_ansi("\u{1b}]8;;https://opencode.ai\u{7}opencode\u{1b}]8;;\u{7}"),
            "opencode"
        );
        // A lone ESC never eats the rest of the line.
        assert_eq!(strip_ansi("a\u{1b}b"), "ab");
    }

    #[test]
    fn a_plain_stream_line_is_drawn_without_its_escape_codes() {
        let events = feed(StreamFormat::Plain, "\u{1b}[31mthinking…\u{1b}[0m");
        assert!(matches!(&events[0], AgentEvent::Text { text } if text == "thinking…\n"));
    }
}
