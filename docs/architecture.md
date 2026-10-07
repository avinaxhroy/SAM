# Architecture and engine design

SAM is structured around a strict boundary between calculation logic and presentation. The calculation engine links no windowing or UI libraries, and the UI links no native platform toolchains directly.

---

## Workspace crate layout

The repository is organized as a Cargo workspace with three Rust crates and one frontend package:

```
SAM/
├── crates/
│   ├── sam-core/         # Pure Rust engine: data model, validation, persistence, schedulers
│   └── sam-cli/          # Terminal interface (sam binary) dispatching commands directly
├── src-tauri/             # Desktop application shell hosting the system webview
├── ui/                    # Svelte 5 frontend (pnpm package, outside the Cargo workspace)
├── design/                # OKLCH token system, base styles, and components reference
└── tools/                 # Quality gates, parity checks, and test runners
```

### Engine and shell isolation

`crates/sam-core` contains the data models, expressions, schedulers, and disk transaction handlers. It deliberately does not depend on Tauri, wry, or webview crates.

An automated integration test (`crates/sam-cli/tests/no_tauri.rs`) asserts that `tauri` never enters the dependency graph of `sam-core` or `sam-cli`. This guarantees that:
1. The engine builds and runs in headless environments, CI pipelines, and CLI automation.
2. Unit tests and self-checks run in milliseconds without launching a graphics context or browser runtime.
3. The terminal binary remains lightweight and independent of desktop window managers.

---

## Four-surface parity

Every operational capability in SAM is designed to be accessible across four surfaces:

| Surface | Projection | Method of interaction |
|---|---|---|
| **Click** | Graphical desktop UI | Mouse, touch, and focused keyboard interaction in Svelte 5 views. |
| **Name** | Command palette (`⌘K`) | Fuzzy search over all registered commands and destinations. |
| **Text** | File system JSON / JSONL | Direct editing in text editors or external tools; reloaded via file watcher. |
| **Terminal** | Headless CLI (`sam`) | Command-line invocation with structured JSON output and exit codes. |

The dispatcher in `crates/sam-core/src/command_registry.rs` maintains the central registry of all 90+ commands. Every mutation declares:
- A unique command ID (for example, `record.advanceStage`, `metric.set`, `view.setLayout`).
- An effect category (`read`, `write`, or `presentation`).
- A JSON pointer indicating where the mutation lands in the plan's files.
- Command-line arguments for CLI dispatch.

There are no GUI-exclusive or palette-exclusive mutations. If an action can be performed via a click, an AI agent or a shell script can perform the identical operation through `sam <command-id> --json`. See [`docs/ai-agents.md`](ai-agents.md) for the AI agent integration contract and workflows.

---

## Persistence and transactions

### Canonical storage in JSONL

All entity records are stored as line-delimited JSON files under `content/records/<type>.jsonl`. 

This format was chosen over relational databases or single large JSON files because:
- Every line represents an independent record document.
- Git diffs remain clean, readable, and mergeable.
- External tools (such as Python scripts or command-line pipelines) can inspect or append records using standard text processing tools.
- Storing one line per record avoids memory exhaustion on large collections.

Configuration files (`types.json`, `views.json`, `rules.json`, `shell.json`, `appearance.json`) are stored as formatted JSON documents with `schemaVersion: 1`.

### Transaction boundaries and crash safety

When an edit occurs through the engine, mutations execute through atomic file transactions:
1. The engine checks the plan revision lock to prevent concurrent write conflicts.
2. Staged changes are validated against the whole-schema validator before touching disk.
3. Before writing, a pre-image snapshot of affected files is recorded to the backup store (`tx.list`).
4. Files are written to temporary sibling files and renamed atomically into place.
5. A new plan revision hash is committed.

If an operation fails, the pre-image snapshot ensures the directory remains untouched. Previous transactions can be inspected with `sam tx.list` and restored with `sam tx.restore --txid <id>`.

---

## File system watcher and debouncing

External file modifications (such as editing a syllabus in a text editor or pulling changes from Git) are detected via a background watcher powered by the `notify` crate.

- **Debounce window**: File system notifications are debounced across an approximately 150ms window to absorb burst writes from editors and source control operations.
- **Hash verification**: When file events fire, file contents are hashed. If the file bytes match the known in-memory state, the event is discarded as a no-op to prevent reload loops.
- **Single reload**: Once debounced changes settle, the entire plan model resolves in a single pass, publishing updated state to the frontend over IPC.

---

## Derived SQLite index

While JSONL files are the source of truth, querying and searching thousands of records through disk scans would introduce UI latency. SAM includes a derived SQLite index (`index.sqlite`) backed by bundled SQLite (`libsqlite3-sys` with FTS5, JSON1, and STRICT mode enabled).

### Role of the index

- Powers fast full-text search (`sam search <query>`).
- Indexes relations and link foreign keys for rapid view filtering and sorting.
- Stores computed column caches.

### Invariant: Derived and disposable

The SQLite file is strictly an ephemeral cache. If `index.sqlite` is deleted or corrupted, the engine transparently rebuilds it from the canonical JSONL files upon the next startup or when invoking:

```bash
sam index.rebuild --plan ./my-plan
```

The database never stores uncommitted mutations or state that does not exist in the JSON or JSONL source files.

After a committed edit the refresh is incremental: the commit's changed-file set drives a delete-and-reinsert of just the affected record ids in one transaction — work proportional to the change, not the plan. A change the incremental path cannot prove safe (a `types.json` edit, a foreign or corrupt cache) falls back to the full rebuild; a failed commit leaves the cache untouched and reports the warning.

---

## Communication and IPC

### Desktop shell (Tauri v2)

In the desktop application, the Svelte frontend communicates with the Rust engine through Tauri commands over a custom protocol scheme (`tauri://`). SAM does not bind a local HTTP port or launch a background localhost server during normal desktop execution, eliminating network port conflicts and socket security concerns.

### Development and testing runner (`web-ipc.mjs`)

For automated headless browser testing, accessibility audits, and frontend development without recompiling the Tauri shell, SAM includes a lightweight runner (`tools/web-ipc.mjs`). 

The runner:
- Serves the frontend bundle via Vite or static HTTP.
- Proxies `/ipc` requests directly to the compiled `sam` CLI binary with `--json` output.
- Enforces freshness checks to verify the compiled binary matches the latest source files before running test suites.
