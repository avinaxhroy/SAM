---
name: sam-authoring
description: Use when authoring, modifying, or bulk-importing study plans, courses, topics, or syllabi into SAM (the configurable study OS). Instructs agents on how to discover active schemas, introspect allowed fields and options, preview diffs via dry-run, and safely commit batches.
metadata:
  version: "1.0.0"
  date: "2026-10-07"
---

# SAM Study Plan Authoring Skill

Use this skill whenever asked to create, import, update, or convert a syllabus or study schedule into SAM.

SAM is an offline desktop study OS backed by canonical JSONL records and transactional CLI commands (`sam`).

---

## 1. Creating a new plan from scratch (Zero-to-One)

When starting with a blank slate or a new topic:
1. **Choose a starter preset**:
   - `self-study`: Best for self-paced courses, textbooks, programming milestones, projects.
   - `university-term`: Best for college semesters with weekly lectures, units, and credits.
   - `jee` or `neet`: Best for competitive exams with high-volume problem practice.
   - `language`: Best for vocabulary drills and grammar rules.
2. **Materialize the plan on disk**:
   ```bash
   sam plan.new --name "<Plan Name>" --preset <preset>
   # or
   sam plan.new --plan ./custom-plan-dir --preset <preset>
   ```
   This creates the full directory structure (`types.json`, `views.json`, `rules.json`, `shell.json`) from the engine's bundled templates.

---

## 2. Discovery protocol (How to get all context)

> **Name the folder on every call.** A plan lives wherever its user chose — any folder on disk —
> and one machine may hold several. Pass `--plan "<folder>"` on **every** command, quoted exactly
> like that (the path may contain spaces). Inside the SAM app the assistant also exports
> `SAM_PLAN` for its agent, but the flag is the contract: never make SAM guess.

Never guess field names, types, or select options. Query the engine directly:

```bash
# 1. Locate the active plan and get the current revision hash
sam paths --plan "<folder>" --json

# 2. Discover all declared entity types, fields, allowed options, and relation targets
sam --schema --json --plan "<folder>"

# 3. Retrieve pre-generated, valid record examples and syntax constraints
sam --guide --json --plan "<folder>"

# 4. Check existing courses so you link to existing IDs instead of duplicating them
sam records --type course --json --plan "<folder>"
```

### Understanding the schema from `sam --schema --json`
- **Entity types**: Keys in `.types` (e.g. `course`, `week`, `topic`, `unit`, `problemset`).
- **Fields**: `.types.<name>.fields`:
  - `key`: The exact field name (e.g., `title`, `est`, `kind`).
  - `type`: Data type (`text`, `number`, `duration`, `date`, `select`, `relation`).
  - `options`: **The exact allowed choices for select fields**. For example, if `kind` has `options: ["watch", "practice", "read", "write", "project", "revise"]`, only use these values.
  - `to`: For relation fields, the target entity type (e.g., `to: "course"`).
  - `required`: Whether the field must be present.
- **Relations (`links`)**: In SAM, relation references are stored in the `"links"` object as arrays of IDs:
  `"links": { "course": ["c.cs101"], "week": ["w.cs101.1"] }`.
  Scalar values belong in `"fields"`.

---

## 3. Ingestion loop (§4.7 Agent Contract)

Always follow the 4-step safe authoring cycle:

### Step 1: Stage candidate records outside the plan
Write records to `/tmp/batch.jsonl` (or a temporary scratch directory). Each line must be a self-contained JSON record:
```jsonl
{"schemaVersion":1,"id":"c.cs101","type":"course","fields":{"code":"CS101","name":"Data Structures","short":"DSA","credits":4},"links":{}}
{"schemaVersion":1,"id":"w.cs101.1","type":"week","fields":{"index":1,"label":"Week 1: Algorithmic Complexity"},"links":{}}
{"schemaVersion":1,"id":"t.cs101.01","type":"topic","fields":{"title":"Big-O Notation and Recurrence","kind":"practice","est":45},"links":{"course":["c.cs101"],"week":["w.cs101.1"]}}
```

### Step 2: Validate with dry-run
```bash
sam apply /tmp/batch.jsonl --dry-run --json --plan "<folder>"
```
- Inspect the output:
  - If `"ok": true`, check `"counts"` (`added`, `updated`, `moved`).
  - If `"ok": false`, read the line number and diagnostic in `"diagnostics"`. Correct the specific line in `/tmp/batch.jsonl` and re-run.

### Step 3: Commit with optimistic revision check
```bash
sam apply /tmp/batch.jsonl --if-revision <hash_from_step_1> --json --plan "<folder>"
```

### Step 4: Verify whole-plan integrity
```bash
sam configcheck --json --plan "<folder>"
sam today.view --json --plan "<folder>"
```

---

## 4. Editing screens and views (blocks)

Users ask for screens as often as they ask for records ("add a chart to Today"). A view in
`content/views.json` is either a plain list (a type + layout) or a **screen**: a tree of blocks.
Screens are edited with the `view.*` commands — never by hand, and never by guessing flags:
`sam <id> --help` prints any command's exact parameters, and `sam --schema --json --plan "<folder>"`
carries the block vocabulary (kinds, keys, examples).

```bash
# Always read the view and its blocks first — this is the shape you are editing
sam view today.screen --plan "<folder>" --json

# Add a block: --kind is a layout (list · table · board · calendar) or one of
# stat · chart · callout · columns · conditional · repeat; --value sets the
# block's own keys, repeatable
sam view.block.add today.screen --kind chart \
  --source progress.sessions \
  --value x=date --value y=min --value reduce=sum \
  --value days=7 --value label="Minutes per day, this week" \
  --plan "<folder>"

# Change one key of one block — the same JSON the file holds
sam view.block.set today.screen --index 0 --key label --value "Study time" --plan "<folder>"

# Move one block (its address, e.g. 0.blocks.2) into a container at a position
sam view.block.move today.screen --path 2 --index 0 --plan "<folder>"

# Remove one block (the last removal returns the view to a single list)
sam view.block.remove today.screen --index 2 --plan "<folder>"
```

- `chart` keys: `x` buckets the source view's records (e.g. `date`), `y` is the value drawn
  (e.g. `min`), `reduce` folds each bucket (`sum`, `avg`, …), `days` fills the last N calendar
  days with zero, and `label`/`title` name it.
- `stat` reads `view` + `expr` + `reduce`; `callout` reads `text`; `columns`/`panel` arrange
  other views; `conditional` holds `when` + `blocks`/`else`; `repeat` reuses `view` + `limit`.
- Whole-document edits to `content/views.json` go through `sam source --file content/views.json`
  (read) and `sam source.apply` (write, validated + transactional) — still through the CLI.
- Verify every screen edit with `sam view <name> --plan "<folder>" --json` and
  `sam configcheck --plan "<folder>" --json`.
