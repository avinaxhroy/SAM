# Changelog

## [0.2.0-beta] - 2026-10-22

### Added

- **Lecture links.** `topic` → `resource`, one hop: watch link before landing page, `http(s)` only. `today.view` row `link`. *Watch* on the Today focus card, each queue row, each course topic row. `openUrl` opens the system browser (`window.open` is absent in the webview).
- **Invalid plan is a state with two doors.** Canvas shows the first validator finding (`content/views.json:2 — … [json.invalid]`), counts the rest, offers *Open the file in the source pane* and *Try again*. `source`, `source.check`, `tx.restore`, `edit.undo`, `edit.redo` answer on the raw bytes; the rest refuse in the loader's words.
- **`plan.new` takes an empty folder.** Non-empty refused as before. A refused open offers *Make a new plan here*. `.DS_Store`, `.localized`, `Thumbs.db`, `desktop.ini`, `Icon\r` are not content.
- **`app.installCli`.** `View → Install command line tool`, palette, `sam app.installCli --json`. Symlink into a PATH dir on macOS/Linux (`osascript`, `pkexec` for rights), user-PATH entry on Windows. Answers with the receipt or the exact elevated command.
- **The assistant.** Titlebar sparkle (⌘J): one turn of an installed CLI against the open plan — Claude Code, OpenCode, Copilot, Antigravity (`agy`), Oh My Pi, Pi, or a declared profile. No model, API or key of its own. The agent writes through `sam` (dry-run → `--if-revision`). Screens reload as it works; the turn ends with a receipt and one Undo. `sam agent.list`, `sam agent.run`; new verbs `agent.list`, `agent.models`, `agent.run`, `agent.cancel`, `app.assistant`.
- **App context per turn.** Open screen, selected record, held revision, rail destinations, composed screens.
- **Plan per command.** Quoted `--plan "<folder>"` in argv and `SAM_PLAN` in the env; the flag wins. Sessions resume per CLI and per plan.
- **Model picker.** Searchable, provider grouping, *Default* first, *Custom…* last. Read on open; a second open joins it.
- **`sam <id> --help`.** Full registry definition (parameters, types, effect, JSON id) in text or `--json`, no plan load. A wrong flag lists the command's parameters; a mistyped verb suggests near ids. `sam help <id>` for humans.
- **`sam --schema --json`.** Block vocabulary: `list` … `repeat`, keys, folds, a minimal example each. Generated from the loader/renderer tables.
- **Conversation resume.** Claude Code and OpenCode sessions captured from the stream.
- **The run island.** One ink capsule (black light, white dark) at the head of the room, naming the state or newest tool; press grows it into the process (agent, model, activity, steps, *Put the process away*). Escape puts the process away first.
- **The veil.** 240px halftone tile, 8px pitch, dot radius = local tone, ink, one polarity. Light: WebGL2, five incommensurate sine sheets, warped twice, anisotropic sample, gated to ribbons. No `backdrop-filter`, no `mix-blend-mode`. Without WebGL2: two blurred bands. Reduced motion: one frame.
- **Composer sidebar.** Edit mode, screen acts (rename, icon, reset, remove); a pressed topic or record row brings the full record form — every field type, *Reveal in source*, *Delete record…*.
- **Sidebar toggle.** ✕ or the edit bar's key (`screen.inspector`, presentation only, `aria-pressed`). Returns on any component press (screen settings) or record row press; `Enter` on a focused frame.
- **Sheet head glyph.** `Sheet.svelte` takes `icon`/`wash`: a square from the icon family on the well or a room wash.

### Changed

- **Settings is a room.** Index plate of five places, pane rows with live readings in a mono column. One row species: glyph, name, key chip, sentence, control. One control family: budget (reading, presets, slider, stepper), zone field, switch, theme check. Find opens the hit's place and lands on its control. Rail 260px; values wrap before they ellipsise. No 1060px cap.
- **The deck.** `--pane` and the glass rim. Head is name, model pill, CLI pill, ✕ — the island owns the run state.
- **Detection.** Cached, off the main thread. Four probes in parallel, 0.56 s → 0.33–0.36 s on the dev machine; 30 s cache. Model list reads only when the picker opens.
- **Claude Code.** `--include-partial-messages`, deduped against whole-message frames; the deck paints once per frame.
- **Thinking.** No longer forwarded across the shell; `sam agent.run --json` still reports it.
- **The prompt.** Names the `view.*` family, `--schema`, `sam <id> --help`. `docs/ai-agents.md` matches.
- **The gallery.** Sheet 700px (`.cd-sheet--wide`; question sheets stay 460), search well with count, band chips that drop when empty, folding counted bands, one card per row, drawn widget with a tick.
- **New screen.** Three full-width mode segments, searchable icon grid with the app fallback tile, `A name, revision, or anything`, rail preview. `type.new` has no `clear-icon`.
- **Plan folder sheet.** `Browse · Type path`, chosen pill as a card, OS dialog and typed path split by "or", footer terminal commands. Place card (machine glyph, plan name, mono path), folder rows, *Open now*. The OS picker is absent, not refusing.
- **Composer chrome.** Sidebar acts are quiet rows, one sentence of consequence each, a state card, the destructive act in words on red. The bar's keys are rectangular; *Add component* is the plus and the ink; *Done* is a ghost; `Session card · A`; ✕ is the danger pair at rest.
- **Edit-mode behaviour.** Record rows select; other widget controls inert. *Reset to SAM's design*. An error is one line plus `(+N more)`; a widget card says only what it knows. The canvas error card offers take-off / leave edit mode / another screen.

### Fixed

- **Design and theme across windows.** `component-styles` and `theme` handed to the other windows (`BroadcastChannel` on web), applied live to each store.
- **Settings no longer resets to *The day*** on any design choice. A host keeps its drawing when its own design did not change; a write touches only the surface it names.
- **Settings reads the plan it edits.** Projection reads the loaded `rules.json`, not source bytes only a write retains. Target, timezone, reading, stepper, preset and summary follow a write.
- **The light/dark switch states its side.** Session state, `setMode` writes beside the store. A mode-pinning theme still ignores it.
- **Preferences file doors reach the editor.** Path handed to the main window (pane, brought forward); *Done* closes. In the browser the door says where the editor is.
- **Island close.** Dismisses a finished run's steps; a live turn keeps the island (its only Stop); no pill after the close.
- **A component press is a selection.** Frame rule solid, sidebar states name, place, design, width. Rows inside still open their record.
- **Last folder.** `remember_plan`, runtime note, `shell_info`, boot. No longer a plan → picker.
- **Folder switch.** Closes on the open's answer, not string equality. Waits for `today.view` and the review count. No unwatch/rewatch race; a stale folder's publication is ignored.
- **A failed widget is a card.** Per-component boundary with *Take this component off the screen* (`view.setComponents`; `--clear` still refused). The canvas boundary fires `reset()` on destination or revision change, and after a card action.
- **The spine guards its week.** No `weeks: []` dereference; loading and no-weeks drawn by the host.
- **Not a plan is one line.** No `content/` answers `plan.absent`; a plan that lost `types.json` still reports the document.
- **The new-plan offer is dry-run first.** `plan.new --dry-run` validates before the offer. A terminal dry run writes nothing.
- **OpenCode 2 wins.** Prefer `opencode2`, then `~/.opencode/bin`, then a fallback name. Stderr has no ANSI codes.
- **CLI detection.** Homebrew, npm-global, `~/.local/bin` after `PATH`; the child inherits the search space. A line starting `at ` is a frame, not a version.
- **Undo is the turn's.** `SAM_RUN` in journal and manifest, reported by `tx.list`. Any exit code; never a terminal write.
- **Folder names cannot inject.** The plan is one single-quoted shell word.
- **A refused model id is stated**, not swapped. argv prompts measured against the platform ceiling (Windows 30k) before spawn. A malformed `agent/profiles.json` is reported by `agent.list` and drawn in the blockers.
- **The deck survives.** IME Enter does not send. Stop drawn from the press, 30 s window, prompt restored on a failed spawn. Transcript 500 rows. A reload cancels the left turn. Subscription neither leaks nor double-binds. The model picker keeps its highlight while the list arrives.
- **The island travels its own measure.** Width and height between the pill's and the panel's measured boxes; the process stays mounted until the close lands; arriving rows grow the panel.
- **A turn ends when its CLI does.** Poll while readers run: 2 s drain, 1.5 s stop for a lingering CLI. A finished lingering turn is not *Stopped*; a stop claims no receipt.
- **Bytes.** Invalid UTF-8 → U+FFFD, >8 MB frames truncated with a visible line. `BufRead::lines()` no longer gives up (SIGPIPE, exit -13).
- **Stop reaches children.** Process group (`kill` on Unix, `taskkill /T` on Windows). The UI ends the turn after 3 s if the shell is silent. Quit and reload stop it.
- **No half-applied publication.** Journal-fresh hold; the state key is a hash of findings, not their count.
- **No promised `sam`.** The prompt tells the agent to stop and say so when the bundled binary is missing.
- **Contrast.** Disabled pill: quiet well, secondary ink (was ~1.9:1). *Remove screen…* / *Delete record…*: red on the card (≥ 8.18:1, was ~1.2:1). A glyph plate never stretches; *On this screen* sits in the card's flow; the assistant's field is `spellcheck="false"`.

## [0.1.0-beta] - 2026-10-07

### Added
- **Today Dashboard**: Objective focus card, titlebar session timer, and dynamic queue grouping (`late`, `due`, `next`, `stale`).
- **Curriculum Planning**: Course units, weekly syllabus breakdown, and topic tracking with estimates and prerequisite links.
- **Practice & Proof Gates**: Multi-stage learning progression (`unseen` -> `learning` -> `learned` -> `proved`) with required practice problem thresholds.
- **Spaced Repetition**: Review queue with keyboard-driven recall grading (`Space` to reveal, `1` Again, `2` Good) supporting Fixed ladders, SM-2, and FSRS-6 algorithms.
- **Progress Tracking**: Denominator-backed metrics replacing arbitrary streaks, tracking revised topics, solved problems, and study minutes.
- **Headless CLI (`sam`)**: Full terminal automation surface with JSONL batch mutations (`sam apply`), dry-run validation, and schema introspection.
- **Multi-Platform App**: Tauri v2 desktop application pairing Rust core engine with Svelte 5 frontend across macOS, Linux, and Windows.
- **Curriculum Presets**: 8 built-in templates (`blank`, `university-term`, `self-study`, `jee`, `neet`, `cbse11-pcm`, `language`, `seed`).
