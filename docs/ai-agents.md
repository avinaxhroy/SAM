# Authoring study plans with AI agents and chatbots

SAM supports creating and populating study plans directly using AI. It requires **no special skills or plugins installed**: you can use any standard chatbot (ChatGPT, Claude, Gemini) or any terminal/coding AI agent (Claude Code, Cursor, Windsurf, OpenHands, Antigravity, Aider, or shell scripts) using native CLI commands.

A study plan in SAM is simply a directory of your choice on disk (e.g. `./my-plan`, `~/study-2026`, or any custom path).

---

## 1. How SAM represents a study plan

In a standard curriculum plan, SAM structures records into a clear hierarchy (or custom entity types defined in your schema):

1. **Courses (`course`)**: The high-level subjects or domains you are learning (e.g., Computer Systems, Biology, Microeconomics).
2. **Units / Weeks (`unit` / `week`)**: Optional temporal or modular grouping (e.g., Week 1, Unit 3).
3. **Topics (`topic`)**: The bite-sized, atomic study sessions that populate your **Today** queue and spaced repetition queue:
   - `title`: Name of the topic (e.g. "Binary Search Trees").
   - `kind`: Activity type — one of `"watch"`, `"read"`, `"practice"`, `"write"`, `"project"`, `"revise"`.
   - `est`: Estimated duration in minutes (e.g. `30`, `45`, `60`).
   - `focus`: Optional scheduled date (`"YYYY-MM-DD"`).
   - Relations: Linked to its parent course via `"links": { "course": ["c.<id>"] }`.

All records are stored as canonical line-delimited JSON (`.jsonl`). Any batch of records can be imported atomically in a single command with full rollback safety.

---

## 2. Using an AI coding agent (Terminal / CLI)

If you are using an AI agent with command-line access (such as Claude Code, Cursor, Windsurf, OpenHands, Antigravity, or shell scripts), **no plugins or skills need to be installed**. The agent can execute standard `sam` CLI commands directly against any plan folder.

**Getting `sam` on your PATH.** The desktop app carries the CLI inside its bundle. `View → Install command line tool` (or the palette's *Install command line tool*) links it where your login shell looks — `/usr/local/bin` or `~/.local/bin` on macOS/Linux, with the OS's own permission prompt when needed, and a user-PATH entry on Windows. From a source checkout, `cargo build --release` leaves the binary at `target/release/sam`.

**Name the folder on every call.** A plan lives in any folder you choose, and one machine may hold several — so every command takes `--plan "<folder>"` (quoted; paths may contain spaces). The in-app assistant always targets the folder the window has open; when you run an agent yourself, pass the folder you mean. `SAM_PLAN=<folder>` is the environment form of the same choice (an explicit `--plan` wins).

**Learn a command in one call.** Unsure of a flag? `sam <id> --help` prints the command's whole definition — parameters, types, effect, and the JSON identity a write lands in — and a mistyped flag replies with the command's own parameter list.

### Copy-paste prompt for coding agents

Paste this prompt into your agent's chat or terminal:

> Create a study plan for [my learning goal or syllabus] in `./my-plan` using SAM. Follow the workflow in [`docs/ai-agents.md`](ai-agents.md):
> 1. Initialize the plan folder: `sam plan.new --plan ./my-plan`
> 2. Query active schema and options: `sam --schema --json --plan ./my-plan`
> 3. Break down the curriculum into courses, weeks, and topics, staging them in `/tmp/batch.jsonl`
> 4. Test with `sam apply /tmp/batch.jsonl --dry-run --plan ./my-plan` and commit with `sam apply /tmp/batch.jsonl --plan ./my-plan`
> 5. Verify today's queue: `sam today.view --plan ./my-plan`

### The Agent workflow

The agent follows these 4 simple steps:

#### Step 1: Create the plan in the user's chosen folder
```bash
sam plan.new --plan <path/to/any/folder>
```
This generates the core folder structure and clean schemas (`content/types.json`, `views.json`, `rules.json`, `shell.json`).

#### Step 2: Query the active schema
The agent verifies the exact types, field names, and allowed select options directly from the CLI:
```bash
sam --schema --json --plan <path/to/any/folder>
```
The agent reads `.types` to confirm allowed keys and option values (such as `topic.kind`: `["watch", "practice", "read", "write", "project", "revise"]`).

#### Step 3: Stage and test the batch
The agent structures the user's curriculum and writes candidate records to a temporary file (e.g. `/tmp/batch.jsonl`). It tests the batch before touching disk:
```bash
sam apply /tmp/batch.jsonl --dry-run --json --plan <path/to/any/folder>
```
If any line has a typo or invalid relation, SAM reports the exact line number and diagnostic (e.g. `relation.unknown`). The agent fixes the line and retries.

#### Step 4: Commit and verify
```bash
# Commit the batch atomically
sam apply /tmp/batch.jsonl --plan <path/to/any/folder>

# Verify whole-plan integrity and check today's queue
sam configcheck --json --plan <path/to/any/folder>
sam today.view --json --plan <path/to/any/folder>
```

---

## 3. Using the assistant inside the app

The desktop app can run the agent for you. The titlebar's **sparkle** (⌘J, or *View → Ask
the assistant*) opens the assistant deck: pick one of the coding-agent CLIs already
installed on your machine — Claude Code, OpenCode, Copilot, Antigravity (`agy`), Oh My Pi,
Pi, or any CLI you declare in `agent/profiles.json` under SAM's data directory — and type
what you want. SAM runs no model of its own and stores no API keys: the CLI you already
authenticated does the work, exactly as if you had run it in a terminal in the plan
folder, and the prompt it receives is this document's workflow plus the authoring skill.
Detection looks where a login shell does (`/usr/local/bin`, `/opt/homebrew/bin`,
`~/.local/bin`, `~/.cargo/bin`, `~/.bun/bin`), not only at the app's own `PATH`, so a
Homebrew or npm-global install is found even when SAM was opened from Finder. A CLI
declared by absolute path in `profiles.json` always wins.

While the agent works the room dims behind a swept wash, and the run's state sits in an
island at the head of the room — one word by default, the whole process behind a press.
The plan reloading in place is the point, so keep an eye on it. When the turn ends you get
a **receipt**: the transaction *this turn* committed and one **Undo** (the same restore as
`sam tx.restore --txid <id>`). The receipt is keyed by the run, so a write you made in a
terminal during the same minute is never mistaken for it, and it is offered however the
turn exited — a batch that landed before a failed check is still undoable. The deck's
**Stop** kills the turn and everything it started (its tools included); because every
write goes through the CLI's transactions, a stopped turn is as safe as a crashed one.
Quitting the app or reloading the window stops a running turn rather than leaving it
writing with no window to see it.

Two notes on posture: the deck runs each CLI in its **non-interactive mode** (each
adapter passes the CLI's own auto-approval flag — `--permission-mode bypassPermissions`
for Claude Code, `--auto` for OpenCode, `--allow-all-tools` for Copilot, and so on),
because there is no terminal to answer an approval prompt. The CLI's own account,
policies and sandbox still apply.

OpenCode's two binaries are one generation apart: `opencode2` reads a V2
`opencode.json`, and a 1.x `opencode` refuses one outright ("V2 permissions are not
supported by OpenCode V1"). Detection therefore prefers `opencode2` wherever it sits —
`PATH`, the package-manager prefixes, or `~/.opencode/bin`, where OpenCode's installer
puts it — and falls back to `opencode` only when no V2 binary exists. A fallback name
never shadows the primary name in an earlier directory.

The deck sends the agent a picture of the window with every turn: the screen that is
open, the record that is selected, the revision the app holds, which destinations the
rail has — so "add two topics here" means the screen you are looking at, and you do not
have to name things the app already knows. It also names the plan folder on every
composed command (`--plan "<folder>"`, with `SAM_PLAN` set in the child as the
environment form), so plans kept outside the default location — and plans that have been
moved — are never confused with anything else on the machine. **The model is yours to
pick**: the pill beside the CLI opens a searchable list of the models that CLI reports
(Claude Code and Copilot answer from their documented aliases; the others enumerate
themselves), grouped by provider, with *Custom…* for any id the CLI takes — the list is
read when you open it, so the deck itself never waits on a slow listing. Follow-up
prompts **continue the same conversation** in CLIs that support it (Claude Code,
OpenCode), so "now do the same for week 4" keeps the agent's working memory; session
handles live in memory for the app run only, and are kept per CLI *and per plan* — a
different folder starts a fresh conversation. The prompt teaches both authoring surfaces
now: records and batches (`sam apply …`), and screens and views (`view.block.add`,
`view.block.set`, …), with `sam --schema --json` carrying the block vocabulary and
`sam <id> --help` answering any flag question in one call.

`sam agent.list --json` prints what the app can run; `sam agent.models --agent <id> --json`
prints the picker's models; `sam agent.run "…" --plan <folder>` is the same turn from the
terminal.

## 4. Using a chatbot (ChatGPT, Claude, Gemini)

When using a web or mobile chatbot, the chatbot has no direct access to your computer. You simply provide the prompt below along with your syllabus, study goals, or textbook table of contents. The chatbot designs your curriculum and outputs a `batch.jsonl` file.

### Copy-paste prompt for chatbots

Provide this prompt to your chatbot along with your learning goals or syllabus text:

````markdown
You are an expert curriculum assistant for SAM (a configurable study OS).
Convert the learning goal, course outline, or syllabus text below into a valid SAM batch import file (`batch.jsonl`).

### Schema and storage rules
1. Format: Output MUST be valid line-delimited JSON (JSONL). Exactly one record per line. No markdown formatting inside or around individual lines, no trailing commas.
2. Every record must have:
   - "schemaVersion": 1
   - "id": A unique, stable lowercase string (e.g. "c.cs101", "w.1", "t.cs101.01").
   - "type": "course", "week", or "topic".
   - "fields": JSON object containing scalar values (text, numbers, dates, selects).
   - "links": JSON object containing relation arrays (e.g. "links": {"course": ["c.cs101"]}).
3. Separation: Never put relation links inside "fields". Never put scalar attributes inside "links". Never output progress or formula values.
4. Allowed types and fields:
   - "course": id "c.<slug>", fields: {"code": string, "name": string, "short": string, "credits": number, "instructor": string}, links: {}
   - "week": id "w.<slug>.<index>", fields: {"index": number, "label": string}, links: {}
   - "topic": id "t.<slug>.<seq>", fields: {"title": string, "kind": select, "est": integer_in_minutes, "focus": "YYYY-MM-DD" (optional)}, links: {"course": ["c.<slug>"], "week": ["w.<slug>.<index>"] (optional)}
   - "topic.kind" MUST be one of: "watch" (videos/lectures), "read" (books/papers), "practice" (homework/exercises/quizzes), "write" (essays/reports), "project" (labs/coding), "revise" (review/exam prep).
   - "est" MUST be a raw integer number (e.g. 45, never "45 min").
   - Every ID in "links" must be declared in this batch.
See `docs/ai-prompt.md` for complete schema invariants and canonical examples.

### Output format
Output ONLY a raw code block containing the line-by-line JSONL records. Do not include introductory text or commentary.

### Goal or syllabus to convert:
[PASTE YOUR SYLLABUS, TEXTBOOK CHAPTERS, OR LEARNING GOAL HERE]
````

### How to apply the plan on your computer

You can create and store your plan in **any folder of your choice**:

```bash
# 1. Create a clean plan folder anywhere you want
sam plan.new --plan ./my-plan

# 2. Save the chatbot's output to batch.jsonl and test it (dry-run preview)
sam apply batch.jsonl --dry-run --plan ./my-plan

# 3. Ingest the records into your plan
sam apply batch.jsonl --plan ./my-plan
```

Your plan is now live. Launching the desktop app on `./my-plan` displays your new curriculum in the **Plan** and **Today** screens immediately.

---

## 5. Troubleshooting and safety

- **Custom Folders**: You can point any command to your plan directory using `--plan <directory>`. SAM never forces a specific folder location.
- **Rollback / Undo**: Every batch applied through `sam apply` automatically takes a pre-image backup. If you want to revert an import:
  ```bash
  sam tx.list --plan <path/to/any/folder>
  sam tx.restore --txid <txid> --plan <path/to/any/folder>
  ```
- **Live Sync**: If SAM desktop is running while you or an agent runs `sam apply`, the desktop interface reloads within ~150ms via its debounced file watcher.
