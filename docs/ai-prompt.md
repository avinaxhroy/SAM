# SAM Study Plan Schema Specification for AI

SAM is an offline, keyboard-driven study OS. This specification defines the exact schema, data types, and formatting rules required to convert any university syllabus, textbook outline, or self-directed learning goal into a valid SAM `batch.jsonl` file.

## 1. Storage & Envelope Contract
- **Output format**: Line-delimited JSON (`.jsonl`). Exactly one JSON object per line.
- **Pure output**: Output ONLY the raw JSONL records inside a single code block. Do NOT include conversational preambles, commentary, markdown within lines, or trailing commas.
- **Five required keys**: Every line MUST be an object with exactly these 5 top-level keys:
  - `"schemaVersion"`: `1` (integer literal)
  - `"id"`: Unique, stable lowercase string slug (e.g. `"c.cs101"`, `"w.cs101.1"`, `"t.cs101.01"`)
  - `"type"`: One of `"course"`, `"week"`, or `"topic"`
  - `"fields"`: Key-value map of scalar values only (`string`, `number`, `boolean`)
  - `"links"`: Key-value map of relation arrays only (`"key": ["target_id"]`)

## 2. Entity Types & Field Definitions

### `course`
Top-level subject or domain. Declare this BEFORE any topics that reference it.
- `id`: `"c.<slug>"` (e.g. `"c.dsa"`, `"c.operating-systems"`)
- `fields`:
  - `"name"`: string (full title, e.g. `"Data Structures & Algorithms"`) — **Required**
  - `"short"`: string (abbreviation for navigation badges, e.g. `"DSA"`, max 6 chars) — **Required**
  - `"code"`: string (e.g. `"CS 201"`) — *Optional*
  - `"credits"`: number (e.g. `4`) — *Optional*
  - `"instructor"`: string — *Optional*
- `links`: `{}` (empty object)

### `week`
Optional chronological or modular grouping under a course.
- `id`: `"w.<slug>.<index>"` (e.g. `"w.dsa.1"`, `"w.dsa.2"`)
- `fields`:
  - `"index"`: number (e.g. `1`, `2`, `3`) — **Required**
  - `"label"`: string (descriptive module name, e.g. `"Week 1: Algorithmic Complexity"`) — **Required**
- `links`: `{}` (empty object)

### `topic`
Atomic study units that populate the daily queue and spaced repetition scheduler.
- `id`: `"t.<slug>.<seq>"` (e.g. `"t.dsa.01"`, `"t.dsa.02"`)
- `fields`:
  - `"title"`: string (concise concept or task name) — **Required**
  - `"kind"`: select string from the 6 allowed values below — **Required**
  - `"est"`: integer duration in minutes (e.g. `45`, `60`) — **Required**. **MUST be a raw integer number**. NEVER use strings like `"45m"`, `"1 hr"`, or `"00:45"`.
  - `"focus"`: string date in strict ISO `"YYYY-MM-DD"` format (e.g. `"2026-10-15"`) — *Optional*
- `links`:
  - `"course"`: `["c.<slug>"]` (array with exactly one parent course ID) — **Required**
  - `"week"`: `["w.<slug>.<index>"]` (array with exactly one parent week ID) — *Optional*

## 3. Syllabus Activity Mapping (`topic.kind`)
Real syllabi use diverse terminology. You MUST map all syllabus activities into one of SAM's 6 fixed `kind` values:

| Syllabus Activity | SAM `kind` | Description & Typical Items |
| :--- | :--- | :--- |
| Lecture, Video, Recording, Screencast | `"watch"` | Video lectures, Coursera modules, recorded seminars |
| Reading, Textbook chapter, Paper, Article, Docs | `"read"` | Textbook chapters, research papers, documentation |
| Problem set, Homework, Exercises, Quiz, Drill | `"practice"` | Homework problem sets, exercises, quizzes, LeetCode |
| Essay, Report, Written summary, Reflection | `"write"` | Written assignments, synthesis notes, lab reports |
| Lab, Coding assignment, Capstone, Build task | `"project"` | Hands-on programming projects, hardware labs, builds |
| Review, Spaced repetition, Exam preparation, Flashcards | `"revise"` | Exam revision, concept recap, flashcard review |

> **Strict Invariant**: NEVER invent new values for `kind` (e.g. `"lecture"`, `"lab"`, `"quiz"` will fail schema validation). Always map them to one of the 6 words above.

## 4. Referential Closure & Relation Invariants
- **Link Closure**: Every ID referenced in `"links"` (e.g. `"course": ["c.dsa"]`) MUST have its corresponding `course` or `week` record defined in the same batch or pre-existing in the plan. Never emit dangling links.
- **Array Syntax**: Link targets MUST always be an array of strings: `"links": {"course": ["c.dsa"]}`. NEVER bare strings like `"links": {"course": "c.dsa"}`.
- **Separation of Concerns**:
  - NEVER place foreign relations inside `"fields"`.
  - NEVER place scalar attributes inside `"links"`.
  - NEVER emit computed or progress properties (e.g. completion percentage, status); SAM derives them dynamically.

## 5. Curriculum Sizing & Batch Guidelines
- **Atomic Study Sessions**: Every topic should represent an atomic 25–60 minute focused study block. Break large chapters or multi-hour lectures into 2–3 sequential topics.
- **Batch Scope**: Generate one course or one term per batch (typically 15–40 topics per batch). This prevents output token cutoffs and ensures complete JSON lines.
- **Chronological Order**: Emit the `course` record first, followed by each `week` and its child `topic` records in natural study sequence.

## 6. Canonical Example Output
```jsonl
{"schemaVersion":1,"id":"c.dsa","type":"course","fields":{"code":"CS201","name":"Data Structures & Algorithms","short":"DSA"},"links":{}}
{"schemaVersion":1,"id":"w.dsa.1","type":"week","fields":{"index":1,"label":"Week 1: Complexity Analysis"},"links":{}}
{"schemaVersion":1,"id":"t.dsa.01","type":"topic","fields":{"est":45,"kind":"watch","title":"Asymptotic Notation (Big-O, Omega, Theta)"},"links":{"course":["c.dsa"],"week":["w.dsa.1"]}}
{"schemaVersion":1,"id":"t.dsa.02","type":"topic","fields":{"est":60,"kind":"practice","title":"Recurrence Relations Problem Set"},"links":{"course":["c.dsa"],"week":["w.dsa.1"]}}
{"schemaVersion":1,"id":"t.dsa.03","type":"topic","fields":{"est":30,"kind":"read","title":"CLRS Chapter 3: Growth of Functions"},"links":{"course":["c.dsa"],"week":["w.dsa.1"]}}
{"schemaVersion":1,"id":"w.dsa.2","type":"week","fields":{"index":2,"label":"Week 2: Divide & Conquer"},"links":{}}
{"schemaVersion":1,"id":"t.dsa.04","type":"topic","fields":{"est":50,"kind":"project","title":"Implement Merge Sort and Quick Sort"},"links":{"course":["c.dsa"],"week":["w.dsa.2"]}}
{"schemaVersion":1,"id":"t.dsa.05","type":"topic","fields":{"est":25,"kind":"revise","title":"Flashcard Drill: Master Theorem Cases"},"links":{"course":["c.dsa"],"week":["w.dsa.2"]}}
```
