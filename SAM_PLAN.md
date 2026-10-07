# SAM — Configurable Study OS

**Development plan**

| | |
| --- | --- |
| **Status** | Approved. Product spec stable; stack revised 2026-09-25 (§2 D16–D22). Swift engine (historical Phases 0–3) passed gates, superseded as implementation, retained as port parity spec (§6.1). |
| **As of** | 2026-09-25 (stack, §2/§9) · 2026-09-24 (product spec) |
| **Target** | Cargo workspace: `crates/sam-core` (engine, no shell dependency) · `crates/sam-cli` (`SAM` binary) · `src-tauri` (app shell) · `ui/` (Svelte 5 frontend) |
| **Platforms** | macOS 13+ · Windows 10+ · Linux/webkit2gtk 2.40+ (equal release targets, §9, D21). |
| **Build environment** | Rust 1.96.0 · Node 26.9.0 / pnpm 12.5.1 (§9). No Xcode, Swift, Dart, or JDK at build or run time. |
| **Release** | Single complete release. Phases in §6 are single-session work orders, not phased user releases. |

---

## 0 · Plan rules

1. **Greenfield.** Build from `design/`; no compat shims or legacy-state migrations. App internal version upgrades require migrations. No pixel-identity gate.
2. **One input artifact, not code.** `design/` token system (v4.7). No pre-authored content ships; bundle preset is a schema-only seed template; user terms are authored in-app.
3. **Unbounded content/structure, bounded mechanism.** Small fixed vocabulary for user-defined schemas; users define all content and relations.
4. **Four surfaces per domain capability.** Click, name, text, terminal (§4.8, §4.9). Persistent edits require command ID, JSON location, and CLI dispatch. Headless commands return explicit targets for transient actions.

---

## 1 · Scope

### 1.1 Definition

A study OS enabling students to define curricula and progression mechanics without writing code: unbounded content and structure over a bounded mechanism.

### 1.2 Acceptance criteria

1. Non-author student runs a term (courses, topics, resources, sessions, assessments) via clicks only.
2. Student defines a new entity type (e.g. `problemset`) with fields, relations, and computed columns; app generates form, table, board, calendar, and validation.
3. New types authorable via JSON with identical resolved model and canonical serialization.
4. Study method is data: switching `topic` from pipeline `flip` to `check` requires zero code changes.
5. Runs curricula for other domains (JEE, NEET, generic university term) without code changes.
6. Agent authors term records from syllabus outline; clean git diff; single debounced reload.
7. In-app persisted mutations undoable via ⌘Z / Ctrl+Z restoring exact source bytes. External/CLI writes bypass in-app undo stack and invalidate stale undo entries (§4.8); recoverable backups protect external writes (§4.6).
8. Four-surface coverage for every domain capability verified by parity fixtures (§4.8; §6 Phase 4 Gate 2).
9. Headless CLI `SAM <command-id> --json` exposes all operations; `SAM --commands --json` enumerates them (§4.9).
10. Single source tree runs on macOS, Windows, and Linux from standard install directories (§9, D21).
11. Installed application behavior per §4.10: window geometry persistence, frameless drag regions, native menus and dialogs, single-instance routing, zero browser chrome.

### 1.3 Requirement ledger

| Requirement | Acceptance evidence |
| --- | --- |
| Any courses, names, fields | Add fifth course with custom field and colour without code changes. |
| Arbitrary term length/nesting | Edit week counts; add chapter level via declared relations. |
| New content kinds | Create §3.3 `problemset` across all four surfaces. |
| Pluggable study methods | Switch `flip` to `check` preserving progress history. |
| Configurable 2-problem rule | Modify proof threshold in rules; all surfaces enforce identically. |
| Opaque task IDs | Accept stable opaque IDs; never parse semantic fields from IDs. |
| Term calendar/labels as data | Edit dates/labels as records; no hardcoded week numbering. |
| Pluggable review schedules | Configure fixed intervals or FSRS via rules. |
| Arbitrary projects | Create project linked to course. |
| Milestones as data | Add/edit milestone targets and items. |
| Tokens for visuals | Resolve tokens/overrides; zero palette literals outside `design/tokens.css`. |
| Data-driven navigation/keys | Add shell navigation entry; modify keybinding via config. |
| Data-driven screens | Add view using supported block vocabulary. |
| Live reload without rebuild | UI and external file edits reload into identical resolved model. |
| Progress as JSON | Restart, restore backup, verify identical state. |
| Cross-platform single tree | Build, install, launch, gate app and CLI sidecar on macOS, Windows, Linux (§9). |
| Native application fidelity | Pass all §4.10 checks: drag, native menus, dialogs, single instance, no web chrome. |

### 1.4 Constraints the build starts from

**A. Token system contract.** UI imports `design/tokens.css` (v4.7, oklch, contrast ratios, washes, dark mode) and `base.css`. Component layer is written by the app against `components.css` as a reference (D17). `tokens.json` is read as data for inspection and tooling. Drift between CSS, JSON, and Tailwind fails the build via `node design/tools/tokens.mjs`. Contrast metadata describes default palette; recompute contrast dynamically for overrides (§5).

**B. Advisory taste defaults.** `design/design.md` §11 refusals (streaks, donuts, outlined chips, solid rules, chromatic page backgrounds) ship as advisory defaults with arguments attached. They are opt-in to enforce and opt-out to waive (§5), never blocking saves.

---

## 2 · Decision record

| # | Decision | Basis |
| --- | --- | --- |
| D1 | **User-defined types, fields, relations, records.** Starter type set matches Sem-1 plan. | Convergent pattern across Notion property schemas, Anki Notetypes, Obsidian Dataview, TypeDB. |
| D2 | **Content and progress decoupled.** Content is unbounded; progress is a closed vocabulary of state machines. | Prevents xAPI unbounded tracking traps; maintains discrete completion semantics. |
| D3 | **Records are typed `[String: JSONValue]` documents with whole-schema validation.** | Document model with strict schema contracts beats unstructured document and EAV approaches. |
| D4 | **No generic SDUI library.** Version documents; generate forms from closed field vocabulary. | Vocabulary is closed (task rows, recall cards, record tables, 14 field types). Generic SDUI engines introduce bloat and break surface parity. |
| D5 | **VS Code contribution model.** Declarative seams for types, views, rules, keybindings, settings. | Stable extension model without embedded runtime vulnerabilities; matches Zed and VS Code architecture. |
| D6 | **Three-tier expressiveness ceiling.** L1 Declarative JSON, L2 Safe expressions, L3 User scripts (deferred). | Non-Turing-complete configs remain inspectable, validatable offline, and deterministic. |
| D7 | **L3 scripts in webview, conditional on measured L2 gaps (Phase 8).** | Presentation layer is already JS; avoids embedding separate scripting runtime until proven necessary. |
| D8 | **Typed block renderer registry with finite recursion boundary and stable IDs.** | Prevents unbounded `{#if}` branches; bounds container nesting (`columns`/`conditional`/`repeat`). |
| D9 | **Watch parent root; hash content; drop no-ops; debounce ~150ms.** | Mitigates FSEvents/inotify directory delete-recreate, event replay, and bulk write storms. |
| D10 | **Schema versions in documents; additive migrations; `--configcheck` build gate.** | Predictable document evolution; breaking changes require explicit migration transactions. |
| D11 | **Pluggable schedulers via `rules.json`.** Fixed ladder default; FSRS crate option. | Engine supports fixed review intervals, SM-2, and official Rust FSRS crate with pinned parameters. |
| D12 | **JSONL records canonical; SQLite is a derived, read-only index.** | JSONL provides reviewable line-diffs. Safe upserts rewrite affected files transactionally. |
| D13 | **One model, four surfaces, unified command registry.** Name-first, JSON identity one door away. | All capabilities accessible via click, name (⌘K), text (JSON), and terminal (CLI). |
| D14 | **In-app undo for persisted mutations via platform accelerators.** | ⌘Z on macOS, Ctrl+Z on Windows/Linux; transactional inverse operations with revision guards. |
| D15 | **Terminal is Surface D.** `SAM <cmd> --json` over shared registry; stdin batches via `SAM apply -`. | Shared dispatcher ensures CLI never diverges from GUI capabilities. |
| D16 | **Tauri v2 shell.** Rust engine + system webview over custom protocol (no localhost server/port). | Native OS integration with lightweight bundle size and cross-platform web UI rendering. |
| D17 | **`design/` tokens imported; component layer written by app.** | Tokens and base CSS are imported; app implements dense data UI components referencing `components.css`. |
| D18 | **Cargo workspace with three crates: `sam-core`, `sam-cli`, `src-tauri`.** | Compile-time enforcement that engine and CLI link no GUI or Tauri dependencies. |
| D19 | **Svelte 5 + Vite + TS; TanStack Table/Virtual; CodeMirror 6.** | Headless UI libraries preserve design system token styling; compiled frontend minimizes runtime overhead. |
| D20 | **Bundled pinned SQLite (`libsqlite3-sys` with FTS5, JSON1, STRICT).** | Eliminates system SQLite version variance across OS distributions. |
| D21 | **Equal release priority for macOS, Windows, Linux.** Documented Linux renderer mitigations. | Named Linux WebKitGTK DMABUF/Wayland workarounds without dropping platform priority. |
| D22 | **Swift engine is parity spec and test corpus, not scaffold.** | Preserves 45 fixture plans and 38 self-check contracts; UI decisions harvested into Appendix C. |

---

## 3 · Content model — specification

### 3.1 Concepts

```text
TypeDef    — named entity kind + typed fields + relations + pipeline
FieldDef   — single typed attribute
Record     — { schemaVersion, id, type, fields: [String: JSONValue], links: [String: [String]] }
ViewDef    — query over records of a type + block layout definition
Pipeline   — named progression state machine
Block      — layout and rendering primitive
```

Auxiliary configurations: `appearance.json` (tokens/themes), `shell.json` (navigation, commands, keybindings), `rules.json` (pipelines, schedulers, metrics).

**Record contract:** `id` is non-empty, globally unique, and opaque. Non-relation attributes reside in `fields` (including arrays/objects for `multiSelect`, `daterange`, `json`). Relations reside in `links` as string arrays of target IDs. `FieldDef` defines cardinality (`one` or `many`, default `one`), requiredness (default false), and optional defaults. Parent edges must be acyclic. Formulas and progress states are derived dynamically, never persisted in `fields`. All config and record documents carry `schemaVersion: 1`.

### 3.2 Default type set — the seed preset

The seed preset ships as a schema-only template (`presets/seed/`), accompanied by an empty plan preset (`presets/blank/`).

```jsonc
{
  "types": {
    "program":  { "icon": "graduationcap", "fields": [
                    {"key":"institute","type":"text"}, {"key":"degree","type":"text"},
                    {"key":"start","type":"date"},     {"key":"semesters","type":"number"}] },
    "term":     { "icon":"calendar", "parent":"program", "fields": [
                    {"key":"label","type":"text"}, {"key":"start","type":"date"},
                    {"key":"weeks","type":"number"}, {"key":"credits","type":"number"}] },
    "course":   { "icon":"book", "parent":"term", "colorRole":"identity", "fields": [
                    {"key":"code","type":"text"}, {"key":"name","type":"text"},
                    {"key":"short","type":"text"}, {"key":"credits","type":"number"},
                    {"key":"instructor","type":"text"}, {"key":"note","type":"longtext"}] },
    "unit":     { "icon":"squareStack", "parent":"course", "fields": [
                    {"key":"label","type":"text"}, {"key":"index","type":"number"},
                    {"key":"start","type":"date"}, {"key":"end","type":"date"}] },
    "topic":    { "icon":"bolt", "trackable": true, "pipeline":"flip", "fields": [
                    {"key":"unit","type":"relation","to":"unit","cardinality":"one","required":false},
                    {"key":"course","type":"relation","to":"course","cardinality":"one"},
                    {"key":"title","type":"text"},
                    {"key":"kind","type":"select","options":["watch","practice","read","write","project","revise"]},
                    {"key":"est","type":"duration"},
                    {"key":"resource","type":"relation","to":"resource"},
                    {"key":"part","type":"text"},
                    {"key":"week","type":"relation","to":"week","cardinality":"one"}] },
    "week":     { "icon":"calendar", "parent":"term", "fields": [
                    {"key":"index","type":"number"}, {"key":"label","type":"text"},
                    {"key":"start","type":"date"}, {"key":"end","type":"date"}] },
    "resource": { "icon":"trayFull", "fields": [
                    {"key":"label","type":"text"}, {"key":"kind","type":"select","options":["video","pdf","article","book","tool"]},
                    {"key":"url","type":"url"}, {"key":"watchUrl","type":"url"}, {"key":"provider","type":"text"}] },
    "assessment":{ "icon":"calendar", "fields":[
                    {"key":"label","type":"text"}, {"key":"date","type":"date"},
                    {"key":"course","type":"relation","to":"course"}] },
    "session":  { "icon":"clock", "fields": [
                    {"key":"date","type":"date"}, {"key":"min","type":"duration"},
                    {"key":"course","type":"relation","to":"course"}, {"key":"note","type":"text"}] },
    "note":     { "icon":"pencil", "fields": [
                    {"key":"title","type":"text"}, {"key":"body","type":"longtext"},
                    {"key":"course","type":"relation","to":"course"}] },
    "milestone":{ "icon":"flag", "fields":[ {"key":"label","type":"text"}, {"key":"month","type":"text"}, {"key":"target","type":"number"}] },
    "project":  { "icon":"hammer", "fields":[ {"key":"name","type":"text"}, {"key":"course","type":"relation","to":"course"} ] }
  }
}
```

### 3.3 Extension — the `problemset` type

Reference custom entity created entirely via configuration without code modifications:

```jsonc
"problemset": {
  "icon": "checklist", "parent": "course", "trackable": true, "pipeline": "progress",
  "fields": [
    {"key":"chapter","type":"text"},
    {"key":"difficulty","type":"select","options":["easy","medium","hard"]},
    {"key":"total","type":"number"},
    {"key":"attempted","type":"number"},
    {"key":"solved","type":"number"},
    {"key":"pct","type":"formula","expr":"pct(solved, total)"},
    {"key":"source","type":"url"},
    {"key":"lastAttempt","type":"date"}
  ]
}
```

### 3.4 Field types — closed vocabulary

| Type | Renders as | Notes |
| --- | --- | --- |
| `text` · `longtext` | single-line · multi-line | UTF-8 strings. |
| `number` · `duration` | stepper/number · "1h 35m" | Numbers finite with declared bounds; duration in non-negative minutes. |
| `date` · `daterange` | Date picker / text | Civil dates formatted `YYYY-MM-DD`. |
| `bool` | toggle / circle | Boolean value. |
| `select` · `multiSelect` | menu · chips | Declared option list; optional `colorRole`. |
| `rating` | 1–5 stars | Discrete integer score. |
| `url` | link with action button | Scheme validated before opening. |
| `relation` | entity picker | References target type; indexed in `links`. |
| `progress` | visual bar | Derived automatically from associated `pipeline`. |
| `formula` | read-only computed value | Layer-2 safe expression (§4.4). |
| `json` | raw editor | Escape hatch for arbitrary JSON values. |

### 3.5 Pipelines

Progression state machines defined in `rules.json`:

```jsonc
"pipelines": {
  "check":    { "stages": ["done"] },
  "flip":     { "stages": ["learned","proved","anchored"],
                "gates":  { "proved": { "require":"learned" },
                            "anchored": { "require":"proved" } },
                "completeWhen":"anchored",
                "proof": { "appliesToKinds":["watch"], "minProblems":2 },
                "anchorSkip": { "allowed":true, "requireReason":true } },
  "progress": { "stages": ["started","midway","done"], "progress":"pct" },
  "srs":      { "stages": ["new","learning","review","mature"], "scheduler":"fsrs" },
  "pages":    { "stages": ["reading","read"], "counter":true }
}
```

Progress state in `state/state.json`:

```jsonc
"progress": {
  "t_8f2a": { "pipeline":"flip", "stages":{"learned":"2026-09-12T10:00:00Z","proved":"2026-09-13T10:00:00Z"},
              "data":{"problemsSolved":2,"signal":"Prof stresses Bayes denominator"} }
}
```

**Transition rules:** `advanceStage` validates prerequisite stages and required evidence in a single transaction. `flip` gates `proved` on non-negative `problemsSolved >= minProblems`. `anchored` requires evidence or a recorded skip reason. Backtracking invalidates dependent downstream stages. Pipeline changes require explicit stage mappings. Fixed review schedules use offsets `[1, 7, 30]` calendar days from the anchor instant.

### 3.6 Views

Saved queries and layout configurations:

```jsonc
"views": {
  "today.topics": { "type":"topic", "layout":"list",
                    "filter":"week.term.id == currentTerm() && !complete()",
                    "sort":"week.index, est",
                    "group":"kind", "limit": 12 },
  "plan.timeline":{ "type":"unit", "layout":"timeline", "sort":"index", "group":"course" },
  "courses.board":{ "type":"course", "layout":"board", "group":"credits" },
  "math.problems":{ "type":"problemset", "layout":"table",
                    "filter":"course.id == 'math'", "sort":"chapter",
                    "columns":["chapter","difficulty","solved","total","pct","source"] }
}
```

Layout types: `list`, `table`, `board`, `timeline`, `calendar`, `tree`, `cardGrid`, `graph`. Filters, sorts, and column definitions use Layer-2 expressions (§4.4). Types with zero declared views trigger a linter warning (§5).

### 3.7 Validation rules

Enforced at config load and before managed commits:
- Every record references an existing `type`.
- All field values conform to the target `FieldDef.type`.
- All relation targets exist; no dangling IDs.
- Pipeline stages are non-empty; gates reference valid declared stages; `completeWhen` is valid.
- `select` values match declared options.
- Integrity failures (malformed JSON, duplicate IDs, missing types/fields, link cycles, invalid schema versions) block commits.
- Invalid source-pane edits remain drafts with line-level diagnostics without corrupting published state.
- External files with syntax errors preserve untouched bytes; UI loads last-known-good snapshot and displays diagnostic report.
- Advisory taste lints never block data persistence.

---

## 4 · Architecture — specification

### 4.1 Scopes and layout

```text
1  Bundle preset   Resource directory           read-only seed
                  macOS    SAM.app/Contents/Resources/presets/<name>/
                  Windows  <install dir>\resources\presets\<name>\
                  Linux    <install dir>/resources/presets/<name>/
2  Active plan     Platform data directory      writable default
                  macOS    ~/Library/Application Support/SAM/plans/<id>/
                  Windows  %APPDATA%\SAM\plans\<id>\
                  Linux    $XDG_DATA_HOME/SAM/plans/<id>/ (~/.local/share/SAM/…)
3  Session         In-memory                    transient UI state only
```

CLI flag `--plan <directory>` specifies an explicit plan root. Path resolution is unified in `sam-core` and exposed via `SAM paths --json`.

```text
content/    types.json         schema definitions
            records/*.jsonl    one JSON line per record
            views.json         saved queries and layouts
            rules.json         pipelines, schedulers, metrics, lints
            appearance.json    theme IDs and token overrides
            shell.json         navigation, commands, keybindings
state/      state.json         progress state and scheduler tracking
```

**Serialization:** UTF-8, LF line endings, sorted keys, one record per JSONL line, trailing newline. Upserts replace records in-place; deletions remove lines transactionally. External CRLF and missing trailing newlines are normalized without breaking untouched content.

### 4.2 Engine components

```rust
// crates/sam-core (Zero Tauri or UI dependencies)
TypeDef / FieldDef / Record / ViewDef / Pipeline   // serde data models
ConfigStore    // load, validate, watch, transactional apply
ResolvedConfig // immutable snapshot consumed by readers
CommandRegistry // catalog of IDs, parameters, effects, surfaces
Action         // domain verbs (nav, createRecord, setField, advanceStage, etc.)
CommandDispatch // CLI execution engine mapping args to ConfigStore
Indexer        // Phase 6 SQLite derived cache (bundled rusqlite)

// crates/sam-cli -> SAM binary dispatcher
// src-tauri      -> Native desktop shell with #[tauri::command] wrappers
// ui/            -> Svelte 5 frontend (generated controls, tables, palette, source pane)
```

Form generation is unified: one generator produces record forms from `TypeDef` and settings forms from app configuration schemas.

### 4.3 Two doors, one object

**Addressing contract:** File-relative path + RFC 6901 JSON Pointer (e.g. `content/views.json#/views/today.topics/filter`). JSONL records use file path + record ID + pointer (e.g. `content/records/topic.jsonl#t_8f2a/fields/title`).

- **UI door:** Records lists, `+ New <type>` generated forms, inline table cells, TSV/CSV paste sheets.
- **JSON door:** Raw `.jsonl` and `.json` files with live reload and validation.
- **Bridge:** Context menu *Reveal in JSON* / *Copy path*, two-way synchronized source pane, File mode toggles.
- **Command Palette:** `> new: <type>`, `> config: reveal <path>`, `> import: paste table`.

### 4.4 Expression language (Layer 2)

| Tier | Capabilities | Scope | Constraints |
| --- | --- | --- | --- |
| **L1 Declarative JSON** | Types, fields, records, views, pipelines, tokens, commands | Routine configuration | Fully validated offline. |
| **L2 Safe expressions** | Computed fields, filters, visibility rules (`pct(solved, total)`) | Queries & formulas | Deterministic parser/evaluator; no loops, no I/O, no user functions. |
| **L3 User scripts** | Webview JS scripts (Phase 8 conditional) | Advanced extensions | Requires measured gap; isolated sandbox. |

**L2 Grammar:** Literals, dotted paths, arithmetic (`+ - * / %`), comparisons (`< <= > >= == !=`), logic (`&& || !`), ternary, parentheses.
**Function whitelist:** `sum count avg min max clamp round pct daysBetween weekOf today currentTerm complete`.
Divisions by zero and undefined relations yield `null` with inline diagnostics. `pct(a, b)` returns `null` when `b <= 0`.

### 4.5 Runtime constraints

- Typed block renderer registry with fixed recursion boundary (D8).
- `ConfigWatcher` monitors parent root, hashes content, discards no-ops, debounces ~150ms (D9).
- Document versioning: unknown schema versions rejected; migrations are additive; `SAM --configcheck` gates builds (D10).
- Strict separation: `sam-cli` contains no Tauri/GUI dependencies; inspection commands execute without webview initialization.
- Window chrome: frameless window with native controls on macOS/Windows; custom controls on Linux; dragging via `data-tauri-drag-region` (§4.10).
- Packaging: Bundles include assets, `tokens.json`, presets, and target-specific CLI binary sidecar (`sam-{target-triple}`).

### 4.6 Storage

```text
RECORDS   content/records/*.jsonl        Authoritative source of truth
STATE     state/state.json               Progress and scheduler state
INDEX     <data dir>/indexes/<id>/index.sqlite Derived disposable cache
```

**Transaction protocol (Phase 2):**
1. Acquire plan advisory lock; verify source revision hashes.
2. Validate operations in memory; stage complete modified files on same filesystem.
3. Write transaction journal (before/after bytes, affected paths, status markers). Atomically replace files via temporary file rename; write commit marker. Startup performs automatic recovery of incomplete transactions.
4. Maintain rolling backup of last 20 committed transactions for restoration. External unmanaged writes trigger conflict reports rather than silent overwrite.
5. Notify UI and trigger asynchronous refresh of SQLite index.

**Index schema (Phase 6):**
```sql
CREATE TABLE records(id TEXT PRIMARY KEY, type TEXT NOT NULL,
                     data TEXT NOT NULL CHECK(json_valid(data))) STRICT;
CREATE INDEX idx_solved ON records(type, json_extract(data,'$.fields.solved'));
CREATE VIRTUAL TABLE search USING fts5(id UNINDEXED, type UNINDEXED, body);
```

SQLite linked via `rusqlite` bundled build (D20). Index failures are treated as cache misses, never mutating or failing source storage.

### 4.7 Agent contract

| # | Contract | Rationale |
| --- | --- | --- |
| 1 | Canonical JSONL (sorted keys, single line) | Clean, deterministic `git diff` lines. |
| 2 | Stable opaque IDs | Deterministic references without semantic parsing. |
| 3 | Idempotent upserts by ID | Safe batch retries without duplicate records. |
| 4 | Recoverable transactions (§4.6) | Atomic multi-file modifications with rollback. |
| 5 | Line-level diagnostics | All-or-nothing validation preserving failing input. |
| 6 | `--configcheck --json` | Structured diagnostic reporting for autonomous agent repair. |
| 7 | `--dry-run` | Full record diff preview prior to commitment. |
| 8 | `--schema` | Emits active schema definitions directly from `TypeDef`. |
| 9 | `--guide` | Emits syntax guide with validated examples. |
| 10 | Debounced watcher (§4.5) | Single reload event for multi-record batch writes. |

**Agent execution loop:**
```text
1  agent: SAM --schema --json                      -> inspect active types
2  agent: SAM view today.topics --json            -> inspect state and revision
3  agent: write batch.jsonl outside plan           -> stage records with IDs
4  agent: SAM apply batch.jsonl --dry-run --json  -> preview diff and validate
5  user:  reviews preview
6  agent: SAM apply batch.jsonl --if-revision <hash> --json -> commit batch
7  app:   single debounced reload -> derived index refresh
```

Index read-only inspection query (Phase 6):
```bash
/usr/bin/sqlite3 -readonly "$INDEX_PATH"   "SELECT id, json_extract(data,'$.fields.chapter') FROM records
   WHERE type='problemset'
     AND json_extract(data,'$.fields.solved') < json_extract(data,'$.fields.total');"
```

### 4.8 One model, four surfaces

#### The rule

> One underlying model, one set of operations, one file format. Surfaces differ only in interaction altitude.

#### Principle 1 — One concept

A `TypeDef` defines attributes for its `Record`s. A `ViewDef` queries and formats records. Persistent edits map to identical operations across UI, palette, file, and CLI.

#### Principle 2 — One registry, four surfaces

```text
        ┌──────────────────────────────────────────────────────────────┐
        │  Command Registry: ID · Title · Params · Shortcut · JSONPath │
        └───────┬───────────────┬───────────────┬───────────────┬──────┘
                ▼               ▼               ▼               ▼
           SURFACE A       SURFACE B       SURFACE C       SURFACE D
           Click / Menu    ⌘K Palette      Raw File/Pane   CLI Terminal
```

| Operation | Surface A — Direct | Surface B — ⌘K | Surface C — File | Surface D — Terminal |
| --- | --- | --- | --- | --- |
| Add topic | `+ Add` button | `topic.new` | `{"id":"t.01",...}` | `SAM topic.new --id t.01 --title "Bayes"` |
| Rename column | Click header | `column.rename` | FieldDef `"label"` | `SAM column.rename topic.est --label "Min"` |
| Set layout | View selector | `view.setLayout` | `"layout": "board"` | `SAM view.setLayout today.topics --layout board` |
| Daily target | Settings slider | `study.setDailyTarget` | `"dailyTargetMin": 180` | `SAM study.setDailyTarget 180` |

Controls provide direct access to JSON identity: right-click offers *Copy as JSON*, *Copy ID*, and *Copy path*.

#### Principle 3 — Edit what you see

| Target | Interaction | Output |
| --- | --- | --- |
| Record field | Click cell | Inline cell edit. |
| Course color | Dot in sidebar | Swatch picker. |
| Columns | Column header | Menu: Rename, Retype, Choices, Hide, Delete, *Copy as JSON*. |
| Column order | Header drag | Reorder columns. |
| Add column | Header `+` | Menu of hidden columns + *New column*. |
| Sidebar item | Right-click item | Rename, Icon, Duplicate, Delete, *Copy as JSON*. |
| Any object | ⌘⇧J | Open live source pane at target selection. |

#### Principle 4 — Three altitudes, superset-ordered

| Altitude | Mechanism | Prerequisite knowledge | Denials |
| --- | --- | --- | --- |
| **Click** | Direct manipulation | None (standard UI conventions) | None (all operations exposed) |
| **Speak** | ⌘K Palette | Command names (from tooltips/menus) | None |
| **Type** | Source pane / raw file | JSON schema and syntax | None |

Surface D (Terminal) spans Speak (command IDs) and Type (JSON I/O).

#### Principle 5 — The source pane makes the four surfaces one

```text
  ┌─────────────────────────────┬───────────────────────┐
  │  Topics             +  ⌘⇧J  │  {"schemaVersion":1,  │
  │  ─────────────────────────  │   "id":"t.bayes.01",  │
  │  ▸ Bayes' theorem    30m    │   "type":"topic",     │
  │  ▸ Naive Bayes       45m    │   "fields":{...}}     │
  └─────────────────────────────┴───────────────────────┘
```

Live two-way binding with draft isolation. External disk changes trigger a *Reload* prompt instead of clobbering uncommitted drafts. Validation errors display inline with table cell cross-highlighting.

#### Principle 6 — Undo in-app persisted changes

Committed transactions register inverse file transformations and expected revision tokens in the in-app undo stack (⌘Z / Ctrl+Z). Stale entries are pruned if external changes intervene. Field-level typing undo remains isolated from transaction undo (§4.10).

#### Surfaces and their owners

| Surface | Ownership scope | Primary commands |
| --- | --- | --- |
| **Sidebar** | List navigation, order, icons | `list.new`, `rail.select` |
| **Table / Card** | Visible columns, cell values, grouping | `column.rename`, `record.setField` |
| **Toolbar / Head** | Entity creation, bulk import | `record.new`, `record.paste` |
| **Settings** | Themes, targets, rules, design doctor | `settings.set`, `design.check` |
| **Command Palette** | Projections of all commands by name | Universal dispatch |
| **Source Pane** | Raw JSON text projection | `source.apply`, `apply` |
| **Terminal** | CLI execution of all commands by ID | `SAM <cmd> --json` |

#### Door priority

| Door | Frequency | Priority |
| --- | --- | --- |
| **Click** | Dozens of times daily | Primary polished user experience. |
| **⌘K Palette** | Several times daily | Keyboard discoverability and rapid navigation. |
| **Bulk Import / CLI** | Several times per term | High-throughput batch initialization and agent scripting. |

#### Walkthroughs — three jobs at three altitudes

##### A · Add a course and its topics

| Altitude | Interaction |
| --- | --- |
| **Click** | Sidebar -> *Courses* -> *Add course...* -> enter details -> *Add topics* -> paste lines. |
| **⌘K** | `course.new` form -> `topic.new` or `topic.paste`. |
| **File** | Add course record to `course.jsonl`; append topic records to `topic.jsonl`. |
| **Terminal** | `SAM course.new --id math --name "Math I" --json` -> pipe topic JSONL into `SAM apply -`. |

##### B · Track something the app never heard of — §3.3's `problemset`

| Altitude | Interaction |
| --- | --- |
| **Click** | `+` -> "New kind" -> "Problem sets" -> configure columns on table header. |
| **⌘K** | `list.new` -> `column.new` (number, relation, formula). |
| **File** | Define `problemset` in `types.json`; write records to `problemset.jsonl`. |
| **Terminal** | `SAM list.new "Problem sets" --id problemset` -> `SAM column.new` -> `SAM apply -`. |

##### C · Change the look

| Altitude | Interaction |
| --- | --- |
| **Click** | Settings -> *Appearance* -> choose theme/swatches; *Design doctor* -> apply fixes. |
| **⌘K** | `appearance.theme`, `design.check`. |
| **File** | Edit token overrides in `content/appearance.json`. |
| **Terminal** | `SAM appearance.theme cadence-dark --json` -> `SAM design.check --json`. |

#### Shared mechanisms and remaining work

| Component | Responsibility |
| --- | --- |
| **Command Registry** | Parameter schemas, effects, discovery, execution routing. |
| **⌘K Palette** | Filtering, parameter collection draft forms, keyboard traps. |
| **Copy as JSON** | Emits RFC 6901 pointer and canonical JSON for selected context. |
| **Form Generator** | Generates UI forms from `FieldDef` specifications. |
| **Column Menu** | Rename, retype, choices, visibility, and deletion dispatches. |
| **Source Pane** | CodeMirror 6 editor, inline lint diagnostics, conflict handling. |
| **Undo Engine** | Transaction inverse execution with revision guarding. |

#### Learning curve

| Altitude | Learning investment | Value |
| --- | --- | --- |
| **Click** | Zero | Full term management via standard GUI patterns. |
| **Speak (⌘K)** | Minimal | Fast keyboard navigation via displayed command names. |
| **Connect** | Moderate | Schema relations and formula definitions. |
| **Type** | Moderate | Direct JSON schema and file manipulation. |
| **Terminal** | Minimal for shell users | Scripting and automated agent management. |

#### No phasing of doors

All four surfaces ship simultaneously in the single release. Capabilities never ship with missing surfaces.

---

### 4.9 Surface D — the terminal

`SAM` executes directly against `sam-core` without launching GUI windows (D18).

```bash
SAM                                  # help and command catalog
SAM --commands --json                # enumerate registry definitions
SAM view today.topics --json         # execute saved query
SAM topic.new --id t.bayes.01 --title "Bayes" --est 30 --week w1 --json
SAM column.rename topic.est --label "Minutes" --json
SAM apply - --dry-run --json         # validate and preview batch from stdin
SAM apply batch.jsonl --if-revision "$REV" --json
SAM --schema --json                 # export active schema
SAM --guide --json                  # export authoring instructions
SAM --configcheck --json            # validate plan offline
SAM paths --json                    # print resolved roots and index paths
```

**CLI Rules:**
1. Verb matches command registry ID.
2. Zero interactive prompts; missing parameters or conflicts return error codes.
3. Structured output: clean data on stdout, errors on stderr, `--json` returns structured envelope.
4. Idempotent upserts, dry-run support, atomic transactions.
5. `SAM apply -` provides bulk input processing with single whole-plan validation.

**Exit codes:** `0` success · `1` validation error · `2` usage error · `3` revision conflict · `4` I/O or storage failure · `5` presentation action unavailable headlessly.

---

### 4.10 · Desktop integration — what makes this an application and not a page

| Behavior | Implementation | Verification Gate |
| --- | --- | --- |
| Window geometry persistence | `decorations: false` + `tauri-plugin-window-state` | Phase 0B |
| Native drag region | `data-tauri-drag-region` on titlebar element | Phase 0B |
| Native menu bar | `tauri::menu::Menu` projected from command registry | Phase 4 |
| Native context menus | Context menu bindings per row, cell, header | Phase 4 |
| Native file dialogs | `tauri-plugin-dialog` | Phase 4, Phase 7 |
| File associations | `bundle.fileAssociations` (`.samprofile`) | Phase 7 |
| Single instance routing | `tauri-plugin-single-instance` forwarding launch args | Phase 4 |
| Native About/Preferences | Native About panel; preferences bound to ⌘, / Ctrl+, | Phase 4 |
| Tray and notifications | System tray icon and native OS notifications | Phase 7 |
| Instant theme application | Blocking theme script before first paint in HTML head | Phase 0B |
| Platform undo | Field-level typing undo isolated from transactional undo | Phase 4 |
| Native text conventions | IME, spellcheck, OS keyboard accelerators | Phase 7 |
| Accessibility | Semantic accessibility tree and screen reader labels | Phase 7 |
| Zero browser chrome | No URL bar, navigation buttons, or web context menus | Phase 7 |

**Platform Traps:**
1. `-webkit-app-region: drag` is inert on macOS and Linux WebKit. Titlebar dragging requires `data-tauri-drag-region`. Interactive controls must be non-dragging siblings.
2. Undo ownership: Text fields own typing undo; app transaction undo triggers only when focus is outside editable fields.
3. Frontend capabilities: `fs` and `sql` plugins are omitted; all operations cross Tauri IPC into `sam-core`.

---

## 5 · Design constraints and lints

**Design assets:**
- **Import:** `design/tokens.css`, `design/tokens.json`, `design/base.css`, `design/tailwind.css`.
- **Reference:** `design/components.css` and `design/demo/` guide component implementations in `ui/src/styles/components.css`.
- **Intent:** `design/design.md` and `design/ux.md` specify product rationale; they supersede component CSS.

`rules.json` defines advisory design lints displayed in the Design Doctor:
```text
design.no.streak-guilt        "streaks, scores, projections"      (§11)   advisory
design.no.donut-ring          "a ring implies a whole"            (§11)   advisory
design.no.outlined-chip       "a chip is a wash"                  (§11)   advisory
design.no.solid-row-rule      dashed only, inside a card          (§11)   advisory
design.no.chromatic-page-bg   (§11)                                       advisory
design.no.fake-os-chrome      "the window controls are the OS's"  (§11)   advisory
design.max.floating-surfaces  = 2 (one rail, one console)         (§11)   advisory
design.contrast.on-wash       >= 4.5 for normal text, recomputed from resolved colours
design.type.scale             monotonic
design.motion.duration        single curve, declared durations only
content.relation.target       must exist; no dangling relations           integrity (blocking)
content.pipeline.stages       non-empty; declared stage gates             integrity (blocking)
content.type.views            warns if type has zero views                advisory
content.field.no-label        every field requires plain label            advisory
```

Advisory lints support waivers recorded in `rules.json` with timestamps and reasons. Integrity lints block commits.

---

## 6 · Build order — one phase per session

Phases are discrete work orders sized for single engineering sessions with concrete verification gates.

| Gate category | Runner | Scope |
| --- | --- | --- |
| Engine tests | `cargo test --workspace` | Unit tests, transaction engine, parity fixtures. |
| Shipped CLI gates | `sam --selfcheck` · `--configcheck` · `--rendercheck` · `--uicheck` | Packaged application binaries without toolchains. |
| Design & token gates | `node design/tools/tokens.mjs` · `node tools/plancheck.mjs` · `pnpm -C ui build` | Token drift, reference resolution, UI builds. |
| Platform integration | Manual / interaction scripts | Window drag, native menus, screen readers. |

| Release status | Phases | Description |
| --- | --- | --- |
| **Core Release** | **0A · 0B · 1 · 2 · 3 · 4 · 5 · 6 · 7** | Full product specifications and platform gates. |
| **Conditional** | **8** | Scripting sandbox (contingent on measured L2 limits). |

### 6.0 · The phase-card template

Each phase card specifies: Goal, Entry state, Artifacts, Spikes, Steps, Gate, Not here.

| Port phase | Historical Swift lines | Rust core implementation scope |
| --- | --- | --- |
| 1 | 2,043 | JSON parser, cursor, validator, resolved config, config store, watcher. |
| 2 | 1,671 | Plan lock, transaction journal, apply, command registry, dispatch. |
| 3 | 1,350 | L2 expression parser, evaluator, view resolver, schema/guide generator. |
| 4 | 1,250 | DocEdit mutation layer, table/palette/pane bindings. |
| — | 1,504 | Port of 38 self-check test cases into `cargo test`. |
| — | 0 | TokenRegister eliminated (tokens imported directly). |

### 6.1 · The port protocol — Swift engine → Rust core

| Artifact | Treatment |
| --- | --- |
| `Sources/SAMCore/Resources/fixtures/**` (45 files) | Carried across unchanged as test corpus. |
| 38 Self-check specifications | Reimplemented as `cargo test` cases. |
| Engine implementation | Reimplemented in Rust crates. |
| `Sources/SAMUI/` (3,220 lines) | Decisions harvested into Appendix C; Swift code deleted. |
| `Theme.swift` & Swift build scripts | Discarded outright. |

**Port sequence:**
1. Corpus test fixture assertions in `cargo test`.
2. Validator and `--configcheck`.
3. `--selfcheck` matching 38 verification checks.
4. Transaction engine and journal recovery.
5. Layer-2 expressions and saved view resolution.
6. Token inspection via `--tokens`.
7. Harvest UI prototype into Appendix C before deleting Swift files.

---

### Phase 0A — Ground: workspace, CLI skeleton, corpus

**Goal.** Working Cargo workspace, CLI skeleton, and fixture corpus oracle.
**Entry state.** `design/tokens.json`; fixture corpus (45 files).
**Artifacts.** New: `Cargo.toml`, `crates/sam-core/`, `crates/sam-cli/`, `src-tauri/tauri.conf.json`, `ui/package.json`, `crates/sam-core/tests/corpus.rs`.
**Spikes.** None.
**Steps.**
1. Initialize Cargo workspace; build order ensures `pnpm -C ui build` runs before Cargo.
2. Pin dependencies in `Cargo.lock`.
3. Assert `tauri` is absent from `sam-cli` dependency tree.
4. Copy fixture corpus byte-for-byte; assert count and integrity in `corpus.rs`.
5. Implement path resolution in `sam-core` (`directories` crate).
6. Implement CLI flags: `--help`, `--version`, `--selfcheck`, `--tokens --json`.
7. Implement throwaway `--rendercheck` for fixture validation.
**Gate.** `pnpm -C ui build`, `cargo test --workspace`, `node design/tools/tokens.mjs`, `node tools/plancheck.mjs` pass; `sam --tokens --json` outputs valid tokens.
**Not here.** No database, windowing, or full config loader.

---

### Phase 0B — The shell: window, chrome, design system, real install

**Goal.** Native OS window with custom chrome, imported token styles, and packaged install.
**Entry state.** Phase 0A passing.
**Artifacts.** New: `src-tauri/` configurations and platform overlays; `ui/src/{main.ts,App.svelte,styles/components.css,shell/Titlebar.svelte}`.
**Spikes.**
- S1: Window decorations per OS; macOS uses native decorations or frameless titlebar with 0-inset tokens.
- S2: Linux GTK application ID vs `tauri-plugin-single-instance`.
- S3: Sidecar binary naming conventions (`sam-{target-triple}`).
**Steps.**
1. Configure OS-specific window settings.
2. Add `data-tauri-drag-region` to titlebar; position interactive controls as non-drag siblings.
3. Set `data-os` attribute on `<html>` on boot.
4. Import token layer (`tokens.css`, `base.css`, `tailwind.css`); initialize `ui/src/styles/components.css`.
5. Render test components (`.cd-pill`, `.cd-window`).
6. Enforce theme setting before first paint to prevent flashes.
7. Generate icons via `tauri icon`.
8. Integrate window state and single instance plugins.
9. Verify capabilities omit `fs` and `sql` permissions.
10. Build bundle with CLI sidecar.
**Gate.** Packaged app launches on macOS, Windows, Linux; window drags and controls respond; geometry persists across restarts; zero token literals in Svelte components.
**Not here.** No record tables, views, or menus.

---

### Phase 1 — Load and validate: a plan becomes machine-checkable

**Goal.** `SAM --configcheck` loads and validates any plan root with path-precise diagnostics.
**Entry state.** Phase 0B passing; corpus tests green.
**Artifacts.** New: `crates/sam-core/src/{strict_json,json_cursor,decode,validator,resolved_config,config_store,config_watcher,diagnostic}.rs`.
**Spikes.** None.
**Steps.** Port Swift modules in order: StrictJSON/Cursor -> Decode -> Validator -> ResolvedConfig -> ConfigStore -> ConfigWatcher -> `--configcheck`.
**Gate.** Seed preset validates clean; invalid fixtures fail matching exact JSON pointer paths and lines; CRLF and missing trailing newlines tolerated; `--selfcheck` exits 0.
**Not here.** No write mutations or expression evaluations.

---

### Phase 2 — Safe writes: SAM apply and the transaction engine

**Goal.** Safe transactional plan modifications via terminal (`SAM apply`).
**Entry state.** Phase 1 passing.
**Artifacts.** New: `crates/sam-core/src/{plan_lock,transaction,apply,command_registry,dispatch}.rs`.
**Spikes.** None (journal boundary crash points pre-enumerated).
**Steps.** Advisory file lock -> revision hash check -> staged write & journal -> atomic rename -> startup journal recovery -> rolling 20-transaction backups -> `SAM apply` (file and stdin) -> `--dry-run` diff -> registry dispatch.
**Gate.** Rejects 500-line batch if one line fails; identical batch upsert is byte no-op; lock contention returns exit code 3; simulated crash at every journal stage recovers correctly; read-only plan returns exit code 4.
**Not here.** No UI bindings or expression engine.

---

### Phase 3 — Expressions and headless queries

**Goal.** Layer-2 expression parser, computed fields, and headless query execution (`SAM view`).
**Entry state.** Phase 2 passing.
**Artifacts.** New: `crates/sam-core/src/{expression,evaluator,views,schema_guide}.rs`.
**Spikes.** None (frozen clock/timezone fixtures verified).
**Steps.** Lexer & recursive descent parser -> evaluator (null rules, bounds, limits) -> dependency cycle detector -> in-memory view resolution -> `SAM view` -> generated `--schema` and `--guide`.
**Gate.** Evaluator fixtures pass (precedence, division by zero, nulls); dependency cycles rejected; `SAM view` outputs deterministic byte-identical JSON; generated schema examples validate cleanly.
**Not here.** No UI or SQLite indexing.

---

### Phase 4 — Records: generated editor, dual door, desktop integration, parity

**Goal.** Editable record tables, column menus, command palette, live source pane, and native OS desktop integration.
**Entry state.** Phase 3 passing; Appendix C harvest complete.
**Artifacts.** New: `crates/sam-core/src/doc_edit.rs`, `ui/src/records/`, `ui/src/commands/`, `crates/sam-cli/src/uicheck.rs`.
**Spikes.**
- S4: DOM walking for `--uicheck` verification using `data-command` attributes.
**Steps.**
1. Port `doc_edit` mutation engine.
2. Implement `FieldDef -> control` form generator per Appendix C.1.
3. Implement `RecordTable` and `ColumnMenu` per Appendix C.2.
4. Implement CodeMirror 6 source pane with draft isolation per Appendix C.4.
5. Wire transactional undo/redo with field focus isolation.
6. Implement settings screen from schema fields.
7. Implement `--uicheck` structural verification.
8. Wire native menu bar, dialogs, and single-instance routing.
**Gate.**
- Gate 1: Define `problemset` via UI and JSON; resulting canonical JSONL matches.
- Gate 2: Enumerate all commands; execute identical mutations across UI, palette, source pane, and CLI; diff `--uicheck` output against Appendix C.
- Gate 3: Agent authoring loop with dry-run diff inspection and revision checks.
- Gate 4: In-app edits undo cleanly restoring original source bytes.
- Gate 5: Native menus, context menus, and dialogs verified; zero web browser chrome artifacts.
**Not here.** No UI-only commands without matching CLI projections.

---

### Phase 5 — Views and the block renderer

**Goal.** Data-driven screen layouts (List, Table, Board, Timeline, Calendar, Tree, Graph).
**Entry state.** Phase 4 passing.
**Artifacts.** New: `ui/src/blocks/`, `ui/src/views/ViewEditor.svelte`.
**Spikes.**
- S5: Recursion depth limits on container blocks (`columns`/`conditional`/`repeat`).
**Steps.** Block registry -> record blocks -> presentation blocks -> unknown-block fallbacks -> saved views -> view editor.
**Gate.** Recursive block rendering passes limits; formula filtering and grouping verified; unrecognized blocks display diagnostic fallbacks without crashing; view configuration persists as JSON.
**Not here.** No query language extensions beyond §4.4.

---

### Phase 6 — Rules, schedulers, derived index

**Goal.** Study pipeline execution, spaced repetition schedulers, and disposable SQLite search index.
**Entry state.** Phase 5 passing.
**Artifacts.** New: `crates/sam-core/src/{pipeline,scheduler,index}.rs`, `ui/src/panels/{Reviews,Progress}.svelte`.
**Spikes.**
- S6: Pin FSRS crate parameters and verify historical test replay.
**Steps.** Pipeline state transitions (`flip`, `check`) -> schedulers (`fixed`, `sm2`, `fsrs`) -> derived metrics -> SQLite indexer (`records` table, FTS5 virtual table, expression index).
**Gate.** Switch `flip` to `check` preserving progress history; proof gates and skip reasons enforced; replay timestamped review histories deterministically; query results identical before and after index builds; corrupting SQLite index does not lose source data.
**Not here.** Index remains read-only to external consumers; canonical data remains JSONL.

---

### Phase 7 — Sharing, presets, and the release gate

**Goal.** Preset packages, profile import/export, theme customization, and cross-platform verification.
**Entry state.** Phase 6 passing.
**Artifacts.** New: `crates/sam-core/src/profile.rs`, presets in `presets/`, themes in `themes/`.
**Spikes.**
- S7: Verify `.samprofile` file association launch on Windows and Linux.
**Steps.** Profile export/import with private data filtering -> bundle 6 presets -> theme editor -> file associations -> cross-platform test gates.
**Gate.** All presets validate cleanly; themes maintain normal text contrast >= 4.5; screen readers, keyboard navigation, and OS text shortcuts pass; packaged app launches and passes gates on macOS, Windows, and Linux. All §1.2 acceptance criteria pass.
**Not here.** No executable code in presets or profiles.

---

### Phase 8 — Escape hatch *(optional, gated on measurement)*

**Goal.** Webview JavaScript scripting sandbox, built only if unmet L2 needs are proven.
**Entry state.** Formally approved, measured L2 limitation and reviewed sandbox design.
**Artifacts.** TBD upon approval.
**Spikes.** Sandbox security boundaries, resource limits, capability model.
**Steps.** Measure specific limitations -> specify process boundaries and CPU/memory constraints -> implementation.
**Gate.** Imported plans cannot execute scripts without explicit user consent.
**Not here.** No execution engine prior to approved design.

---

## 7 · Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Type system sprawl | High | Vocabulary of field types is closed (§3.4). No per-user custom field primitives. |
| Viewless data rot | High | Saved views are first-class; presets ship comprehensive views; lint warns on viewless types (§5). |
| Configuration complexity | Medium | Progressive disclosure in UI; preset templates for common study paths. |
| Schema migration breaks | High | Additive migrations default; renames require explicit migration actions; `--configcheck` gates builds. |
| Storage write latency | Medium | Realistic benchmarks; JSONL optimizes line diffs; SQLite indexes accelerate queries. |
| Index cache staleness | Medium | Revision-tagged cache; index rebuilt automatically on source revision change; in-memory fallback. |
| Concurrent write corruption | High | Advisory locks, revision checks, atomic renames, journal recovery, rolling backups (§4.6). |
| Invalid agent modifications | High | Offline whole-plan validation, `--dry-run` preview, explicit revision check (`--if-revision`). |
| Visual token drift | High | UI imports `design/tokens.css`; drift between CSS, JSON, and Tailwind checked via `tokens.mjs`. |
| CLI logic divergence | High | CLI and GUI share single `sam-core` command dispatcher and validator (§4.9). |
| Four-surface divergence | High | Shared command registry and parity test fixtures (Phase 4 Gate 2). |
| Data loss anxiety | High | In-app transactional undo (⌘Z / Ctrl+Z) + persistent 20-transaction backup rollbacks. |
| Engine port regression | High | Swift test corpus (45 fixture plans) and 38 self-check contracts preserved verbatim (D22). |
| External webview renderer changes | Medium | Restrict CSS to tested token system; avoid WebGL and `<canvas>`; render charts via DOM and CSS. |
| Linux renderer anomalies | Medium | Probe and apply minimal WebKitGTK DMABUF/Wayland overrides; log active mode in `paths --json`. |
| Windows path & lock quirks | Medium | Verified path normalization; explicit atomic rename testing across file systems. |
| Embedded runtime creep | High | Strictly Rust engine; build-time only Node/pnpm; shipped app contains zero scripting interpreters. |
| Webview feels like a webpage | High | Complete §4.10 checklist: geometry persistence, drag regions, native menus, isolated undo. |

---

## 8 · Out of scope

1. Visual drag-and-drop schema designer (tabular forms provide sufficient editing fidelity).
2. Remote plugin marketplace or online preset repository (local file export/import sufficient).
3. Embedded scripting engines prior to Phase 8.
4. Unrestricted SQL query console (L2 expressions provide safe querying).
5. Multi-user real-time synchronization (single-user local-first architecture).
6. Automatic workspace `.sam/` directory traversal (explicit `--plan` root selection).
7. SQLite as authoritative storage (SQLite remains a disposable derived cache).
8. Interactive CLI REPL or persistent daemon (stateless CLI executions against plan roots).
9. Hand-maintained shell completions (generated from command registry when needed).

**Retained core capabilities:** Complete Surface D CLI (§4.9), agent interaction contract (§4.7), and bulk import doors (§4.8).

---

## 9 · Dependencies and toolchain

### Engine — `sam-core`

| Dependency | Purpose | Rationale |
| --- | --- | --- |
| `serde` + `serde_json` | JSON serialization & validation | De facto Rust standard; handles strict duplicate keys and UTF-8 validation. |
| `notify` | Filesystem event monitoring | Native FSEvents, inotify, and ReadDirectoryChangesW with debouncing. |
| `directories` | Standard OS path resolution | Cross-platform config, data, and cache directory locations. |
| `rusqlite` (`bundled`) | Derived search index (Phase 6) | Pinned SQLite amalgamation with FTS5, JSON1, and STRICT tables. |
| `fsrs` | Spaced repetition scheduler (Phase 6) | Official Rust implementation matching Anki algorithms. |

### Shell and presentation

| Dependency | Purpose | Rationale |
| --- | --- | --- |
| `tauri` (v2.x) | Desktop window & IPC bridge | Lightweight native desktop shell using system webview. |
| `svelte` (5.x) + `vite` + `typescript` | UI components & build | Compiled reactive components with minimal runtime overhead. |
| `@tanstack/svelte-table` + `virtual` | Record table data model | Headless table state and windowing; styling owned by app tokens. |
| `codemirror` (v6) | JSON source pane editor | Modular text editor with JSON syntax and inline lint markers. |
| `tailwindcss` (v4) | Utility styling mapped to tokens | Pre-mapped to design tokens; validated by `tokens.mjs`. |
| Tauri plugins (`window-state`, `single-instance`, `dialog`) | Native OS behaviors | Persisted geometry, single-instance routing, native dialogs. |

**Explicitly excluded:** `fs` and `sql` Tauri plugins (UI access limited to `sam-core` IPC), external component UI kits (e.g. shadcn, Material), generic JSON Schema renderers, and frontend state management libraries.

### Toolchain — verified on this machine, 2026-09-25

| Component | Target Version |
| --- | --- |
| `rustc` · `cargo` | 1.96.0 |
| `node` · `pnpm` | Node 26.9.0 · pnpm 12.5.1 |
| macOS host | macOS 27.0, arm64 (WebKit 22625.1.29.11.27) |
| Bundled SQLite | 3.45.3 prebuilt bindings (vendored amalgamation) |

**Toolchain Rules:**
1. `cargo test --workspace` is the primary automated harness; CLI flags (`--selfcheck`, `--configcheck`, `--rendercheck`, `--uicheck`) gate packaged release builds.
2. App icons generated from single SVG/PNG source via `tauri icon`.
3. Resource files (presets, tokens) reside in bundle assets, not compiled binary strings.
4. Platform floors enforced in configuration: macOS 13+, Windows 10+, Linux `webkit2gtk-4.1` >= 2.40.

#### Platform support — all three, and the one thing that differs

| Platform | Webview Renderer | Distribution Model |
| --- | --- | --- |
| **macOS 13+** | WKWebView (WebKit) | Shipped with OS. |
| **Windows 10+** | WebView2 (Chromium) | Evergreen; installer includes bootstrapper. |
| **Linux** | webkit2gtk-4.1 >= 2.40 | System package distribution. |

**Linux graphics probe:** If WebKitGTK DMABUF/NVIDIA failures occur, apply minimum necessary environment override (`WEBKIT_DISABLE_DMABUF_RENDERER=1`) during shell startup and record the active override in `SAM paths --json`.

**Build sequence:**
```bash
pnpm -C ui install                     # once per clean clone
pnpm -C ui build                       # frontend assets FIRST
cargo build --workspace --release      # engine + CLI + shell
cargo test --workspace                 # fixtures and unit tests
node design/tools/tokens.mjs           # CSS / JSON / Tailwind token drift check
node tools/tokenlint.mjs               # UI token literal check
node tools/plancheck.mjs               # plan reference resolution check
cargo tauri build                      # package desktop app and CLI sidecar
```

---

## 10 · Start here

**Initial workspace validation:**

```bash
cd /Users/avinash/IITPCLASSES/STUDYPLAN/SAM

# 1. Compile frontend and workspace
pnpm -C ui install
pnpm -C ui build
cargo build --workspace --release
cargo test --workspace
node design/tools/tokens.mjs
node tools/plancheck.mjs

# 2. Verify shipped gates
target/release/sam --selfcheck --json
target/release/sam --tokens --json
```

**Initial verification workflow:**
1. Execute `crates/sam-core/tests/corpus.rs` to confirm fixture corpus validity.
2. Verify `problemset` custom type resolution against Phase 0A assertions.
3. Validate seed preset (`presets/seed/`) using `sam --rendercheck`.
4. Render UI test components in Svelte to confirm design token linkage.
5. Follow §6 session protocol for subsequent phase implementations.

---

## Appendix A · Sources

### Stack revision — retrieved 2026-09-25

Primary source references backing architectural decisions in §2 (D16–D22), §4.5, §4.6, and §9:
- [Tauri v2 Documentation](https://v2.tauri.app/start/): Custom webview protocols without localhost HTTP servers; `data-tauri-drag-region` for frameless window dragging; native menus, dialogs, window state, and single-instance plugins.
- [MDN `-webkit-app-region`](https://caniuse.com/mdn-css_properties_-webkit-app-region): Confirms zero support in WebKit/Safari, necessitating Tauri drag attributes on macOS and Linux (§4.10).
- [Tauri Linux Graphics Troubleshooting](https://v2.tauri.app/develop/debug/linux-graphics/): Documents WebKitGTK DMABUF and Wayland driver collisions and defines ordered workaround overrides (§9).
- [Workspace architecture precedents](https://docs.portbay.app/architecture/): PortBay, Cork CLI sidecar, and QuantaBox patterns demonstrating shared engine core with lightweight CLI sidecar and Tauri GUI (D18).
- [Database & Storage Research](https://www.sqlite.org/stricttables.html): Apple `rename(2)` single-file replacement semantics, `flock(2)` advisory lock behavior, and SQLite STRICT table typing constraints.
- [Spaced Repetition](https://github.com/open-spaced-repetition/fsrs-rs): Official Rust implementation of FSRS algorithm (v6.6.2) backing Phase 6 schedulers.

### Evaluated and rejected alternatives
- **Compose Multiplatform (1.8.0):** Rejected due to JVM packaging overhead and lack of mature headless data-grid libraries.
- **Flutter Desktop:** Strong alternative; rejected due to weaker dense-data grid ecosystem (CodeMirror/TanStack equivalents) and lack of UI framework reversibility behind IPC.
- **Native Rust GUI (iced, egui, Slint):** Immature complex tabular editors, text panes, and command palettes; tightly couples UI markup to engine language.
- **Electron:** Bundles full Chromium and Node runtimes (~120MB+), violating lightweight bundle constraints.

---

## Appendix B · Measurements

Historical scratch benchmarks are recorded for provenance and superseded by §4.6 transaction engine requirements.

| Workload | Single `plan.json` | Canonical JSONL | Derived SQLite Index |
| --- | --- | --- | --- |
| Cold load (20,000 records) | 42.7 ms | 123 ms | 0.003 ms (indexed query) |
| Single record write | 41.7 ms | 2.9 ms (unsafe append) | 0.004 ms |
| Bulk append 500 records | 93.2 ms | 2.8 ms (unsafe append) | Derived asynchronously |
| On-disk size (20,000 records) | 5.0 MB | ~4.4 MB | 6.5 MB |

*Caveat:* Unsafe line appends do not satisfy whole-plan validation, advisory locks, journal recovery, or atomic replacements. Production benchmarks must measure the complete Phase 2 transaction path.

---

## Appendix C · The Phase-4 UI harvest

Decisions harvested from the historical Swift prototype (`Sources/SAMUI/`, 13 files, 4,470 lines) to guide Svelte implementations.

### C.0 · What the scaffold is, and the three things it is not

The prototype resolved structural interaction models and `--uicheck` tree representations. It was not a design reference (`Theme.swift` discarded per D17) and did not bind toolchain workarounds (`SState.swift` replaced by Svelte 5 runes).

### C.1 · Field controls — `FieldControls.swift`

Control resolution order: `isFormula || type == "progress"` -> **derived**; `isRelation` -> **relation picker**; then switch on `type`.

| Type | Control Rendering | Empty State | Invalid State |
| --- | --- | --- | --- |
| `text` · `longtext` · `number` · `duration` · `json` · `daterange` | Mono text cell, single line, click-to-edit inline | Blank | Cell retains draft; validator blocks commit (§3.7). |
| `bool` | 20px circular toggle | Off (`false`) | N/A |
| `select` | Borderless dropdown menu (first item `—` writes null) | `—` | Values outside declared options blocked. |
| `date` | Borderless dropdown showing date; click edits as `YYYY-MM-DD` | `—` | Malformed date blocked by validator. |
| `multiSelect` | Capsule chips with add/remove dropdown | No chips | Hand-edited invalid options blocked. |
| `rating` | 1–5 clickable stars (accessible `<button>` group) | 0 stars | Enforced 1–5 bounds. |
| `url` | Mono text with truncation + external open button | Blank | Missing scheme disables open button. |
| `relation` | Dropdown menu over target records; displays `title`/`label`/`name` | `—` | Dangling IDs blocked by validator. |
| `formula` | Read-only mono text displaying L2 evaluator value | `—` | Evaluation error renders `—`. |
| `progress` | Visual progress bar derived from pipeline | `·` | N/A |

**Commit Gestures:** Enter commits; Escape discards. Cell edits dispatch `record.setField` with `{id, field, value}`. Relations dispatch string arrays of target IDs.

**Generated Forms:** `GeneratedForm` renders `[FieldDef]` sequentially (excluding formulas/progress). Reused across new record sheets, settings panels, and command palette parameters.

### C.2 · The record table — `RecordTable.swift`

- **Surface:** Card container without zebra striping or outer borders; rows separated by dashed rules (`--rule-strong`).
- **Card Head:** Icon tile (34px, wash background), title, candidate count caption, and action pills (**New** in prominent ink, **Paste** in ghost).
- **Column Header:** Schema editor surface; chips display uppercase tracked labels with drag handles and menus.
  - Standard column widths: `number`/`duration`/`rating`/`bool`/`formula`: 92px; `date`/`select`: 112px; `relation`: 150px; `url`: 220px; default text: 200px.
- **Column Menu Actions:**
  - *Rename...*: Dispatches `column.rename` (modifies display label; keys are immutable).
  - *Retype...*: Two-phase dry-run preview before committing field type transformations.
  - *Choices...*: Edits option values for `select` and `multiSelect`.
  - *Duplicate*: Dispatches `column.duplicate`.
  - *Hide*: Dispatches `column.hide` (view-specific).
  - *Move Left / Right*: Dispatches `column.reorder`.
  - *Copy as JSON*: Copies full canonical `FieldDef`.
  - *Copy path*: Copies RFC 6901 pointer to field definition.
  - *Delete...*: Dry-run preview of affected records followed by transactional deletion.
  - *Header `+`*: Menu of hidden columns + *New column...* sheet.
- **Row Menu Actions:** *Copy as JSON* (canonical record line), *Copy path*, *Reveal in source pane*, *Delete...* (in-window confirmation card, not native alerts).

### C.3 · The command palette — `Palette.swift`

- 480×400 floating card with scrim.
- Substring search across command ID, title, and category.
- Parameter collection displays draft inputs with validation feedback.
- Presentation actions (`app.palette`, `app.toggleSourcePane`, `app.openSettings`) handled locally without engine dispatch.

### C.4 · The source pane — `SourcePane.swift`

- Targets: Specific record by ID or document by plan path.
- Draft binding: Active edits are isolated from external disk changes; external updates trigger a *Reload* banner.
- Commit via ⌘⏎: Pre-checks syntax, formats canonically, dispatches `apply` or `source.apply`.
- Inline linting via CodeMirror 6 with hover highlighting in table views.

### C.5 · The app-state machine — `AppModel.swift`

```text
booting      -> resolve active plan root
noneExists   -> no plan on disk -> the room, its first screen (one press writes the plan)
several      -> plans on disk, none open -> the room, its last screen (a row opens one)
ready        -> active plan validated and loaded into UI
invalid      -> validation failure without snapshot -> diagnostic report
```

**Undo entry contract:**
```text
UndoEntry { title, changes: [FileChange], expectsRevision, baseRevision }
```
Undo verifies `config.revision == expectsRevision`; mismatched revisions discard stale entries. Undo operations execute as transactions restoring exact before-bytes.

### C.6 · `--uicheck` — the dump format — `UICheck.swift`

Headless structural verification format:

```text
SAM uicheck · plan <name> · revision <hex>
titlebar
  search pill -> dispatch app.palette
  gear -> dispatch app.openSettings
  source -> dispatch app.toggleSourcePane
rail
  disc <Title> -> view <view> · dispatch rail.select
  disc + (quiet) -> dispatch type.new/list.new [sidebar.add]
card head [<view> · <type>]
  + New <type> -> dispatch <type>.new [recordTable.toolbar]
  Paste lines -> dispatch <type>.paste -> apply [recordTable.paste]
columnMenu (every header)
  header <key>:<type> -> column.rename · column.retype · column.choices · column.duplicate · column.hide · column.reorder · column.delete · copy-json · copy-path
  header + -> column.show · column.new [columnMenu.plus]
rowContext [recordTable.rowContext]
  record -> copy-json · copy-path · record.reveal · record.delete
settings [settings]
  row <key>:<type> -> settings.set · shows key <key>
palette [<N> static + <M> dynamic commands]
  <id> · placement <uiPlacement> · json <jsonPath>
```

Verified via DOM attributes: `data-command`, `data-placement`, `data-column`, `data-view`, `data-type`, `data-settings-key`, `data-record-id`.

### C.7 · The registry ids the scaffold already fixed

Core domain command IDs:
```text
app.palette · app.openSettings · app.toggleSourcePane · rail.select
type.new · list.new
column.rename · column.retype · column.choices · column.duplicate · column.hide
column.reorder · column.delete · column.show · column.new
record.setField · record.reveal · record.delete
settings.set
<type>.new · <type>.paste
apply · source.apply
```

Generic setters (`settings.set`) and dynamic type handlers (`<type>.new`, `<type>.paste`) resolve through engine dispatchers to ensure parity across CLI and GUI.

### C.8 · Explicitly not harvested

- `Theme.swift`: Eliminated; replaced by direct token CSS imports.
- `SAMApp.swift`: Framework-specific shell structure replaced by Tauri shell and Svelte root.
- SwiftUI class styling wrappers replaced by `.cd-*` token classes.

### C.9 · The five files that were unclassified — now classified

| File | Purpose & Decisions |
| --- | --- |
| `Sheets.swift` | `+ New <type>` dialogs with engine-generated IDs; `ListDesigner` creating new types in single transactions. |
| `PasteSheet.swift` | TSV/CSV bulk parser with header auto-mapping, generating single transactional `apply` batches. |
| `SettingsScreen.swift` | Schema-generated settings forms displaying raw config keys with inline collision warnings. |
| `Controls.swift` | Icon name mapping with fallback defaults; button styling hierarchy (prominent ink vs ghost paste). |
| `SState.swift` | Toolchain workaround shim; replaced by native Svelte 5 state runes. |

`DocEdit.swift` serves as the functional specification for `crates/sam-core/src/doc_edit.rs`.
