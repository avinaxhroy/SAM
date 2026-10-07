# Content model and schema definition

SAM operates on a strict rule: bounded mechanism, unbounded content. The application defines a small, fixed vocabulary of schema constructs, while students define their own entities, relations, formulas, and workflows.

---

## Core concepts

A study plan models learning through six structural primitives:

| Primitive | Purpose |
|---|---|
| **TypeDef** | A named entity kind defining typed fields, relations, parent edges, and an optional progression pipeline. |
| **FieldDef** | A typed attribute on an entity kind (text, number, date, duration, select, relation). |
| **Record** | A concrete instance document with an opaque identifier, field values, and relation links. |
| **ViewDef** | A saved query (filters, sorting, grouping) combined with a display layout or screen composition. |
| **Pipeline** | A progression state machine defining learning stages, proof gates, and completion criteria. |
| **Block** | A visual layout building block (tables, task rows, stat counters, charts, callouts). |

---

## Record structure contract

Every record stored in `content/records/<type>.jsonl` adheres to this exact contract:

```json
{
  "schemaVersion": 1,
  "id": "t.calculus.04",
  "type": "topic",
  "fields": {
    "title": "Integration by parts",
    "kind": "watch",
    "est": 45,
    "focus": "2026-10-07"
  },
  "links": {
    "course": ["c.math.101"],
    "unit": ["u.calc.02"],
    "week": ["w.term1.05"]
  }
}
```

### Invariants

1. **Opaque IDs**: Record identifiers are globally unique, non-empty strings. The engine treats IDs as opaque tokens and never parses semantic information out of an identifier.
2. **Separation of fields and links**: Scalar values and choices reside in `fields`. Relational references to other record IDs reside exclusively in `links` as string arrays.
3. **No derived state in records**: Computed formulas, aggregate rollups, and pipeline progress states are evaluated on the fly at runtime. They are never written into `fields`.
4. **Acyclic hierarchy**: If a type declares a `parent` relationship (for example, `unit` belonging to `course`), the parent-child graph must remain strictly acyclic. Cycles fail schema validation.

---

## Field types

Field definitions in `content/types.json` support ten field types:

| Type | Description | Stored value |
|---|---|---|
| `text` | Single-line string | `"Physics I"` |
| `longtext` | Multi-line text or Markdown notes | `"Detailed summary..."` |
| `number` | Floating-point or integer number | `4` or `3.14` |
| `date` | Civil date in ISO format | `"2026-10-07"` |
| `duration` | Time duration in minutes | `45` |
| `select` | Choice from a fixed array of options | `"watch"` |
| `multiSelect` | Array of chosen options | `["theory", "lab"]` |
| `daterange` | Start and end date pair | `{"start": "2026-09-01", "end": "2026-12-15"}` |
| `relation` | Target entity type reference | Handled via `links` dictionary |
| `json` | Arbitrary structured JSON data | Structured object or array |

---

## Expression engine (L2)

SAM includes a safe, deterministic expression language for computed columns and metric definitions. Expressions do not permit loops, variable assignment, network I/O, or user-defined functions.

### Grammar and operators

- **Path resolution**: Dot notation walks fields and relations (e.g. `course.code`, `topic.est`, `week.term.label`).
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparisons**: `<`, `<=`, `>`, `>=`, `==`, `!=`
- **Logic**: `&&`, `||`, `!` (short-circuiting evaluation)
- **Conditionals**: Ternary operator (`condition ? if_true : if_false`)

### Null semantics

`null` represents missing data, distinct from `0` or `false`:
- Arithmetic with `null` yields `null` (for example, `null + 5` evaluates to `null`).
- String-to-number coercion is not permitted; mismatched types generate an explicit evaluation error.
- Equality comparison treats `null == null` as `true`.

### Built-in functions

| Function | Signature | Description |
|---|---|---|
| `today()` | `() -> date` | Current civil date in the plan's configured timezone. |
| `currentTerm()` | `() -> string` | ID of the active term record covering the current date. |
| `daysBetween(a, b)` | `(date, date) -> number` | Whole calendar days between date `b` and date `a`. |
| `weekOf(date)` | `(date) -> number` | Index of the week record covering the specified date. |
| `pct(a, b)` | `(number, number) -> number` | Percentage calculation `(a / b) * 100`; returns null if `b <= 0`. |
| `clamp(x, min, max)`| `(number, number, number) -> number` | Restricts value `x` within `[min, max]`. |
| `round(x)` | `(number) -> number` | Rounds to the nearest integer (half away from zero). |
| `complete()` | `() -> boolean` | Returns true if the record has reached its pipeline completion stage. |
| `sum(path)` | `(relation.field) -> number` | Sums field values across related records. |
| `count(path)` | `(relation.field) -> number` | Counts non-null records in the relation. |
| `avg(path)` | `(relation.field) -> number` | Arithmetic mean of field values across related records. |
| `min(path)` | `(relation.field) -> number` | Minimum value in the related collection. |
| `max(path)` | `(relation.field) -> number` | Maximum value in the related collection. |

### Evaluation limits

To prevent performance degradation, the evaluator enforces hard limits:
- Maximum expression length: 512 characters
- Maximum AST depth: 32 nodes
- Maximum path segments: 8
- Maximum relation hops: 5
- Maximum evaluation steps: 10,000 steps

---

## Progression pipelines

Pipelines define how trackable entities move from unstarted to complete. Configured in `content/rules.json`, pipelines convert subjective studying into verified milestones.

### Built-in pipeline machines

1. **`flip` (Lecture & Proof)**:
   - Stages: `learned` -> `proved` -> `anchored`
   - Completion stage: `anchored`
   - Proof gate: Advancing to `proved` requires solving practice problems (`--problems >= proof.minProblems`).
   - Anchor gate: Advancing to `anchored` requires a logged review, or an explicit override reason if `anchorSkip` is permitted.

2. **`check` (Simple completion)**:
   - Stages: `done`
   - Completion stage: `done`
   - Used for straightforward checklists and reading logs.

3. **`pages` (Reading progress)**:
   - Stages: `reading` -> `read`
   - Completion stage: `read`

4. **`progress` (Multi-step tasks)**:
   - Stages: `started` -> `midway` -> `done`
   - Completion stage: `done`

5. **`srs` (Full Spaced Repetition)**:
   - Stages: `new` -> `learning` -> `review` -> `mature`
   - Completion stage: `mature`
   - Driven directly by the review scheduler.

### Switching study methods

A student can switch a topic type from `flip` to `check` without losing historical data:
```bash
sam type.setPipeline topic --pipeline check --map learned=done --map proved=done --plan ./my-plan
```
Prior stage transitions are archived in the record's progress history in `state/state.json`, preserving past logs while adopting new completion criteria.

---

## Review schedulers

When an item enters the review queue, its next review date calculates through one of three schedulers:

### 1. Fixed interval ladder (`fixed`)

The simplest scheduler uses explicit day intervals. The default ladder uses `[1, 7, 30]`:
- First successful review: due in 1 day.
- Second successful review: due in 7 days.
- Third successful review: due in 30 days.

### 2. SuperMemo-2 (`sm2`)

Calculates intervals based on review ratings (`again`, `hard`, `good`, `easy`):
- Tracks repetition count and ease factor (default 2.5).
- A rating of `again` resets repetitions to 0 and schedules immediate review.
- High ratings increase the ease factor and expand subsequent intervals.

### 3. Free Spaced Repetition Scheduler (`fsrs`)

SAM embeds the official Rust `fsrs` crate (FSRS-6), the modern spaced repetition algorithm used in Anki:
- Evaluates memory stability and memory retrievability.
- Accepts standard 21-parameter weight arrays in `rules.json#/scheduler/fsrs/weights`.
- Adjusts intervals based on difficulty, stability, and elapsed time since the previous review.
