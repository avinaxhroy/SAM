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

## 3. Using a chatbot (ChatGPT, Claude, Gemini)

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

## 4. Troubleshooting and safety

- **Custom Folders**: You can point any command to your plan directory using `--plan <directory>`. SAM never forces a specific folder location.
- **Rollback / Undo**: Every batch applied through `sam apply` automatically takes a pre-image backup. If you want to revert an import:
  ```bash
  sam tx.list --plan <path/to/any/folder>
  sam tx.restore --txid <txid> --plan <path/to/any/folder>
  ```
- **Live Sync**: If SAM desktop is running while you or an agent runs `sam apply`, the desktop interface reloads within ~150ms via its debounced file watcher.
