<p align="center">
  <a href="https://github.com/avinaxhroy/SAM">
    <img src="src-tauri/icons/icon.png" width="128" height="128" alt="SAM Logo" />
  </a>
</p>
<p align="center">
  <a href="https://github.com/avinaxhroy/SAM/releases">
    <img src="https://img.shields.io/badge/Download%20SAM-Latest%20Release-238636?style=for-the-badge&logo=github&logoColor=white" alt="Download SAM" />
  </a>
</p>
<h1 align="center">SAM</h1>

<p align="center">
  A desktop study OS and command-line tool that links course syllabi, practice problems, and spaced reviews into a single system.
</p>

Most study setups isolate these pieces: to-do lists do not understand syllabus prerequisites, flashcard apps disconnect questions from the course context, and spreadsheets break down once you need review schedulers or session timers.

SAM organizes your curriculum into a concrete loop: you learn a topic, prove understanding by solving practice problems, and retain it through spaced repetition scheduled directly against your course outline.

---

## How to use SAM

### 1. Set up your syllabus (Plan and Subjects)

Add your courses (e.g. Discrete Mathematics, Computer Systems) with credits and instructor notes. Break each course into units or weeks, then add topics. Topics hold estimated study durations, topic kinds (watch, read, practice, revise), and links to external materials like lecture videos or textbook chapters.

### 2. Work from the daily queue (Today)

The **Today** screen is your starting point:

- **Focus card**: Highlights the single next item to work on, backed by an objective reason (due date, syllabus sequence, or late status) rather than an arbitrary priority score.
- **Session timer**: Click **Start** to run the titlebar timer. When you finish, elapsed minutes save directly to the active topic and parent course.
- **Queue groups**: Remaining work is organized into `late`, `due`, `next`, and `stale` groups so you can work straight down the list.

### 3. Practice and reviews

Sitting through material does not guarantee retention:

- **Practice (Proof gates)**: Topics advance from `learned` to `proved` once you satisfy proof gates (by default, logging 2 solved problems) or track standalone problem sets with accuracy metrics.
- **Reviews (Spaced repetition)**: Proved topics enter a scheduled recall queue (`Space` to reveal, `1` Again, `2` Good) scheduled via SM-2, FSRS-6, or fixed ladders.

### 4. Check progress and handle gaps (Progress)

The **Progress** screen replaces streak counters with four denominator-backed ratios: revised topics out of total syllabus items, solved problems against targets, logged study minutes, and ceiling-benchmarked subject charts.

If you step away for 3 or more days, SAM detects the gap on launch and offers a phased catch-up schedule to spread backlogged reviews over upcoming days.

---

## Four interaction surfaces

Every capability in SAM is accessible across four surfaces:

| Surface | Interface | How to use it |
|---|---|---|
| **Click** | Desktop app | Graphical controls, tables, boards, and timers in Svelte 5. |
| **Name** | Command palette | Press `⌘K` (or `Ctrl+K`) to search screens, run commands, or use quick capture (`a`). |
| **Text** | File system | Edit JSON and JSONL files in your editor; the watcher reloads within ~150ms. |
| **Terminal** | Headless CLI | Run `sam <command-id>` for headless automation and scripting. |

---

## Authoring study plans with AI

You can design and populate study plans with AI without installing extra skills or plugins. Plans can be stored in **any directory of your choice** using `--plan <directory>`.

### 1. With an AI Coding Agent (Claude Code, Cursor, Windsurf, OpenHands, Antigravity)

Paste this prompt directly into your coding agent's chat or terminal:

```text
Create a study plan for [my learning goal or syllabus] in ./my-plan using SAM. Follow the workflow in https://github.com/avinaxhroy/SAM/blob/main/docs/ai-agents.md:
1. Initialize the plan folder: sam plan.new --plan ./my-plan
2. Query active schema and options: sam --schema --json --plan ./my-plan
3. Break down the curriculum into courses, weeks, and topics, staging them in /tmp/batch.jsonl
4. Test with sam apply /tmp/batch.jsonl --dry-run --plan ./my-plan and commit with sam apply /tmp/batch.jsonl --plan ./my-plan
5. Verify today's queue: sam today.view --plan ./my-plan
```

The agent executes these native CLI commands autonomously and populates your study plan end-to-end.

### 2. With a Chatbot (ChatGPT, Claude, Gemini)

Paste this prompt into your chatbot along with your syllabus, textbook outline, or study goal:

```text
Convert the following syllabus or learning goal into a SAM batch.jsonl file strictly following the specification at https://github.com/avinaxhroy/SAM/blob/main/docs/ai-prompt.md:

[Paste your syllabus, course outline, or learning goal here]
```

Then initialize your plan folder and import the generated file:

```bash
# 1. Initialize a clean plan in any folder of your choice
sam plan.new --plan ./my-plan

# 2. Preview and validate the generated records (dry run)
sam apply batch.jsonl --dry-run --plan ./my-plan

# 3. Ingest the records into your plan
sam apply batch.jsonl --plan ./my-plan
```

The desktop app automatically detects the new records via its debounced file watcher (~150ms) and updates the **Plan**, **Subjects**, and **Today** views without restarting.

See [`docs/ai-agents.md`](https://github.com/avinaxhroy/SAM/blob/main/docs/ai-agents.md) for the complete terminal automation guide and [`docs/ai-prompt.md`](https://github.com/avinaxhroy/SAM/blob/main/docs/ai-prompt.md) for the token-optimized schema specification.

---

## Keyboard shortcuts

| Shortcut | Context | Action |
|---|---|---|
| `⌘K` / `Ctrl+K` | Global | Open command palette / search records |
| `a` | Global | Quick capture a new task or topic |
| `⌘Z` / `Ctrl+Z` | Global | Undo last persisted mutation (restores exact file bytes) |
| `Space` | Reviews | Reveal recall card answer |
| `1` | Reviews | Grade recall as **Again** |
| `2` | Reviews | Grade recall as **Good** |
| `Esc` | Global | Close open sheet, palette, or cancel draft |

---

## Getting started

### Prebuilt releases

Download prebuilt binaries for macOS, Windows, and Linux from [GitHub Releases](https://github.com/avinaxhroy/SAM/releases):

[![Download SAM](https://img.shields.io/badge/Download-SAM%20Releases-238636?style=for-the-badge&logo=github&logoColor=white)](https://github.com/avinaxhroy/SAM/releases)

#### macOS: bypassing the "damaged app" warning

The DMG is not code-signed (no Apple Developer Program membership yet), so after downloading from a browser, macOS Gatekeeper may show **"'SAM' is damaged and can't be opened. You should move it to the Bin."** The app is fine — Gatekeeper is rejecting the missing signature, not reporting real corruption.

Fix it either way:

```bash
# If the DMG itself won't mount ("damaged"), strip its flag first:
xattr -d com.apple.quarantine ~/Downloads/SAM_0.1.0-beta_aarch64.dmg

# Option A: after dragging SAM to /Applications, strip the quarantine flag
xattr -cr /Applications/SAM.app
```

**Option B:** download the DMG with `curl` instead of a browser — no quarantine flag is ever set:

```bash
curl -L -o ~/Downloads/SAM.dmg https://github.com/avinaxhroy/SAM/releases/latest/download/SAM_0.1.0-beta_aarch64.dmg && open ~/Downloads/SAM.dmg
```

Or install via Homebrew — the cask strips the quarantine flag for you, no manual step needed:

```bash
brew install --cask avinaxhroy/tap/sam
```


### Building from source

#### Prerequisites

- **Rust**: 1.80 or newer (uses 2024 edition).
- **Node.js**: v20 or newer.
- **pnpm**: v9 or newer.
- **Linux dependencies** (Linux only): WebKit2GTK development headers (`libwebkit2gtk-4.1-dev` on Debian/Ubuntu).

### Build and run

```bash
# 1. Install frontend dependencies and build web assets
pnpm -C ui install
pnpm -C ui build

# 2. Build workspace binaries
cargo build --workspace --release

# 3. Launch the desktop app
cargo tauri dev

# 4. Or run the headless CLI
./target/release/sam help
```

On first launch without an existing plan, SAM presents a start screen where you can choose a preset or create a blank plan folder.

---

## Command-line interface

Every graphical action maps to an engine command ID in the headless `sam` binary:

```bash
# Inspect the current plan and today's queue
sam today.view --plan ./my-plan
sam paths --plan ./my-plan --json

# Add a topic and advance it past the proof gate
sam topic.new --title "Graph Traversal" --kind practice --est 45 --course c.cs.dsa --plan ./my-plan
sam record.advanceStage --id topic.r1 --stage proved --problems 2 --plan ./my-plan

# Log a spaced review rating (calculates next due date)
sam record.logReview --id topic.r1 --rating good --plan ./my-plan

# Apply batch updates or validate configuration
sam apply updates.jsonl --plan ./my-plan
sam configcheck --plan ./my-plan
```

Standard exit codes signal status: `0` (success), `1` (validation or proof gate error), `2` (usage), `3` (concurrency conflict), `4` (I/O error).

For batch transactions via stdin, optimistic concurrency flags, and the full command reference, see [`docs/cli.md`](docs/cli.md).

---

## Plan structure on disk

A study plan is a standard directory on your computer:

```
my-plan/
├── content/
│   ├── types.json          # Entity schemas (course, topic, session, problemset)
│   ├── views.json          # Screen layouts, block arrangements, and saved filters
│   ├── rules.json          # Schedulers, progression pipelines, and proof gates
│   ├── shell.json          # Navigation destinations and icons
│   ├── appearance.json     # Pinned theme and font scaling
│   └── records/
│       ├── course.jsonl    # Line-by-line course records
│       ├── topic.jsonl     # Syllabus topics with estimates and links
│       └── session.jsonl   # Logged study sessions
├── state/
│   └── state.json          # Review history, timestamps, and recall stability
└── index.sqlite            # Read-only SQLite search cache (auto-generated)
```

- Records are stored as line-delimited JSON (`.jsonl`). Each line is a self-contained record, producing clean Git diffs.
- Plans live by default in your platform application data directory:
  - **macOS**: `~/Library/Application Support/SAM/plans/<Plan Name>`
  - **Linux**: `~/.local/share/SAM/plans/<Plan Name>`
  - **Windows**: `%APPDATA%\SAM\plans\<Plan Name>`
- You can target any custom directory with `--plan <path>`.

---

## Presets

SAM includes 8 curriculum presets:

- `blank`: Empty schema ready for custom courses.
- `university-term`: College semester with courses, units, topics, weekly schedules, and credits.
- `self-study`: Milestone-oriented plan for online courses, textbook reading, and practical projects.
- `jee`: High-volume problem practice across Physics, Chemistry, and Mathematics with accuracy metrics.
- `neet`: High-retention medical entrance syllabus covering Biology, Chemistry, and Physics paired with FSRS.
- `cbse11-pcm`: Class 11 secondary curriculum mapped to standard textbook chapters.
- `language`: Foreign language vocabulary drills, grammar rules, reading logs, and listening practice.
- `seed`: Reference starter with sample records used for engine tests.

See [`docs/presets.md`](docs/presets.md) for preset customization and authoring guides.

---

## Customizing schemas and rules

Customize your plan by editing files under `content/`:

- **Add entity types (`content/types.json`)**: Define custom entities (e.g. `problemset`, `lab`) with typed fields, relations, and computed formula expressions.
- **Configure proof gates and review engines (`content/rules.json`)**: Adjust stage pipelines (e.g. change required proof problem counts) or switch schedulers (`fixed`, `sm2`, or `fsrs` with custom retention targets).

See [`docs/content-model.md`](docs/content-model.md) for the complete schema specification and expression syntax.

---

## Development and testing

```bash
# Build frontend
pnpm -C ui install
pnpm -C ui build

# Run workspace unit and integration tests
cargo test --workspace

# Launch desktop app in development
cargo tauri dev
```

---

## Documentation

- [Architecture and engine design](docs/architecture.md)
- [Content model and schema definition](docs/content-model.md)
- [CLI reference and batch workflows](docs/cli.md)
- [Presets and custom curriculum templates](docs/presets.md)

---

## License

GNU Affero General Public License v3.0 (`AGPL-3.0-or-later`). See [LICENSE](LICENSE) for details.
