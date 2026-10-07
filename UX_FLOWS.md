# UX FLOWS — Interaction Flow Specifications & Budgets

Technical specifications for user workflows, gesture budgets, altitude models, and quality gates across SAM.

**Status:** Stable (2026-09-26).  
**Cross-references:** `UI_PLAN.md`, `UI_SPEC.md`, `SAM_PLAN.md` (§4.8), `COMPOSER.md`.

---

## 1 · What a flow is here

Each user workflow is defined by six parameters:

| Parameter | Definition |
| --- | --- |
| **Trigger** | Context or event prompting the workflow. |
| **Path** | Ordered sequence of user actions and interface surfaces. |
| **Budget** | Upper bound on gestures and surface hops. |
| **States** | Deterministic handling for loading, empty, error, saving, and undone states. |
| **Exit** | Guaranteed landing destination following completion or cancellation. |
| **Doors** | Primary UI route plus equivalent ⌘K, direct file, and CLI commands (`SAM_PLAN.md` §4.8). |

---

## 2 · The measured present

### 2.1 Baseline command coverage
Across 9 preset views, 31 commands render UI controls directly without menus; engine declares 63 write commands.

| Core Verb | Default Placement | Present on Today? |
| --- | --- | --- |
| `topic.new` (Add study item) | Today, Reviews | Yes |
| `session.new` (Log study time) | Progress only | No |
| `record.logReview` (Rate recall) | Reviews (when due > 0) | No |
| `problemset.new` (Log practice) | Practice | No |
| `assessment.new` (Log test/mock) | Mocks | No |
| `record.setField` (Edit field) | Everywhere (116 controls on Today) | Yes |

### 2.2 Critical workflow analysis
- **Study logging:** Requires navigating away from Today to Progress, opening a modal sheet, and manually entering relation IDs for week/course. Target is a single click on Today's focus card.
- **Recall reviews:** Renders rating controls only when items are scheduled due; requires manual stage transitions otherwise. Target is single-card presentation with consequence previews per grade.

### 2.3 Machine flows
- **Column creation:** Accessible via header menu (14-item type selector).
- **Kind creation:** Rail Add dialog initiates `type.new`.
- **View composition:** Form-based editor with raw JSON block structures.
- **Deletions:** `type.delete`, `view.delete`, and `list.delete` require explicit engine and UI support.

### 2.4 System flows
- **First run:** Full-window document — *what it is · how it works · get started* — three screens in order, stepped by the dots (the last carries the form, the plans already on disk, and the presets one text link away), each movement drawn as the app's own material at rest and then performed, not described, on the screen the student stands on: the one press writes the plan (the bundled `blank` schema) and opens it, the student's own course and first topic are written next from Today's own doors, the app's own timer runs the session, and then the plan's own pipeline is walked — the session records the learning, the student's press records the proof (the count the plan asks for, when its proof rule applies), and the first grade is a real review whose consequence the plan answered first. The engine will not let a review be a topic's first transition (`pipeline.gate`), so the walk carries every hop the blank preset's `flip` requires. The window then hands over to Today carrying the session and the graded topic. No wizard, nothing configured; re-readable any time over the plan as the Getting-started room (`app.gettingStarted`), and a machine whose plans exist but none is open opens on that same last screen, with the rows beside the form.
- **Settings:** Clean navigation back to active view.
- **Portability:** Export and import exposed via UI sheets, file drop, and CLI.
- **Recovery:** Visual undo stack and toast receipts for all mutations.

---

## 3 · The flow set

### 3.1 Intention & unit model

| Axis | Spectrum |
| --- | --- |
| **Unit** | Time (min/hr) · Work (chapter/lecture) · Count (problems/cards) · Queue (due items) · Date (obligation) |
| **Ordering** | Sequence (linear syllabus) · Scheduler (spaced review) · Deadline (chronological) · Freeform |
| **Commitment** | Committed (fixed promise) · Intended (aspirational) · Backlog (unallocated) |
| **Horizon** | Today · Current week · Term / Exam |

**Core rules:**
1. **Inherited units:** A commitment takes its unit from the target entity (topic → work; course → time; problem set → count).
2. **Unit integrity:** Units are never artificially converted or flattened into abstract aggregates.
3. **Reality takes precedence:** Actual logged activity overrides planned targets without penalizing the user.

### 3.2 Daily flows (session critical)

#### F1 · Today — Decide next action
- **Trigger:** Opening the application during study sessions.
- **Path:** Lands on Today. Displays focus card (course chip, task name, factual reason, `Start` duration, `Not this`) above grouped queue (`late → due → next → stale`).
- **Budget:** 0 gestures to view; 1 to start.
- **States:** Loading (skeleton), Empty (invitation + sample plan door), Active, All-clear.
- **Exit:** Active task surface or timer.
- **Doors:** UI focus card · ⌘K candidate list · `SAM today.view`.

#### F2 · Log study time
- **Trigger:** Completing or initiating study intervals.
- **Path:** Press `Start` on focus card (spawns titlebar timer) or manually enter elapsed minutes. Auto-infers topic and course from context.
- **Budget:** ≤ 3 gestures total; 0 required keystrokes.
- **States:** Running, Paused, Minimum threshold guard (< 1 min ignored), Persisted (+ undo toast).
- **Exit:** Today with updated progress totals.
- **Doors:** UI timer/card · ⌘K `session.new` · JSONL file write · `SAM session.new`.

#### F3 · Quick capture
- **Trigger:** Recording ideas, resources, or tasks mid-session.
- **Path:** Press global capture key (`a` or ⌘K capture mode) → input title → `Enter`. Infers kind from active view or leading `#tag`.
- **Budget:** 2 gestures (Key, Enter). `Esc` preserves draft.
- **States:** Draft, Persisted (+ undo receipt), Disambiguation sheet.
- **Exit:** Returns to originating surface with captured item visible.
- **Doors:** Capture hotkey · ⌘K palette · JSONL file write · CLI.

#### F4 · Review — Recall loop
- **Trigger:** Scheduled reviews ready in queue.
- **Path:** Open Reviews → single card display → `Space` reveals answer → `1`/`2` grades (`Again` / `Good`) with interval consequences previewed.
- **Budget:** Keyboard-only navigation (`Space` + `1`/`2`).
- **States:** Queue empty ("next review in X days"), Active session, Session complete.
- **Exit:** Today with decremented queue count.
- **Doors:** Keyboard flow · ⌘K `record.logReview` · CLI review log.

#### F5 · Log practice — Problem sets
- **Trigger:** Completing problem batches or exercises.
- **Path:** Select problem set → enter count attempted/solved → engine computes score percentages and derives mastery status.
- **Budget:** ≤ 4 gestures; numeric input only.
- **States:** Empty, In-progress, Evaluated.
- **Exit:** Problem set view with updated progress metrics.

### 3.3 Periodic & episodic flows

#### F6 · Log mock assessment
- **Trigger:** Completing an exam or practice test.
- **Path:** Enter date, score, and section breakdown → view performance analysis highlighting flagged weak topics and upcoming plan adjustments.
- **Budget:** ≤ 6 gestures for score input; 0 for read-back.
- **States:** Undated, Scored, Analyzed.
- **Exit:** Assessment summary and linked topic reviews.

#### F7 · Extend plan structure
- **Trigger:** Adding syllabus units, chapters, or rescheduling weeks.
- **Path:** Click inline add between plan rows → input unit details (dates optional) → drag or enter dates to re-balance.
- **Budget:** ≤ 2 gestures to add row; ≤ 3 to adjust dates.
- **States:** Empty plan, Partial schedule, Adjusted.
- **Exit:** Updated plan deck.

#### F8 · Check progress standing
- **Trigger:** Reviewing overall progress status.
- **Path:** Navigate to Progress → inspect 4 denominator-backed metrics (e.g. "62% revised, 14 topics unstudied") and ceiling-benchmarked charts. No streak counters.
- **Budget:** 1 gesture; fits viewport without scrolling.
- **States:** Initial (unstarted), Partial, Comprehensive.
- **Exit:** Returns to Today or drilled-down topic view.

### 3.4 Configuration & data model flows

#### F9 · Reshape data model (Kinds, Columns, Views, Blocks)
- **Trigger:** Modifying schemas to match course requirements.
- **Path:** Open System → select target object. Columns managed inline via header menu; blocks managed via visual tree with JSON code view tab; kinds/views deleted with dependency checks.
- **Budget:** ≤ 3 gestures per modification.
- **States:** Pre-mutation dependency preview, Persisted with undo receipt, Quarantine mode on deletion.
- **Exit:** Modified target surface.
- **Doors:** System UI · ⌘K · Schema JSON files · CLI.

#### F10 · Configure rules and schedulers
- **Trigger:** Adjusting review intervals, retention targets, or milestone deadlines.
- **Path:** System › Rules → edit configuration parameters → inspect real-time daily workload simulation → Accept or Cancel.
- **Budget:** ≤ 3 gestures.
- **States:** Simulating, Committed, Declined.
- **Exit:** Today with re-computed schedule.
- **Doors:** System UI · ⌘K · `rules.json` · CLI.

#### F11 · Appearance & themes
- **Trigger:** Toggling color mode or UI scaling.
- **Path:** Click theme icon in rail or open Settings › Appearance → select theme/scale.
- **Budget:** 1 gesture to open, 1 to switch.
- **Exit:** Immediate in-place update.

### 3.5 System & recovery flows

#### F12 · Import & export plan
- **Trigger:** Backing up, sharing, or restoring a study plan.
- **Path:** Settings › This plan → Export this plan, or Bring a plan in → file picker dialog → inspect preview diff before confirming import.
- **Budget:** ≤ 3 gestures.
- **States:** File selection, Diff preview, Successfully applied, Validation refusal with clear diagnostics.

#### F13 · Return after study gap
- **Trigger:** Launching app after ≥ 3 inactive days.
- **Path:** Launch screen detects hiatus → displays objective summary and catch-up proposal (e.g. 5-day phased backlog reallocation) → Accept, Customize, or Ignore.
- **Budget:** 1 gesture to accept; 0 required.
- **States:** Hiatus prompt, Catch-up active, Dismissed.

#### F14 · Recovery & conflict resolution
- **Trigger:** Undoing unintended edits or resolving external file modifications.
- **Path:** Press `⌘Z` / `Ctrl+Z` to revert transaction, or access Recently Deleted via System menu to restore records. External concurrent changes present a visual diff with Keep/Discard choices.
- **Budget:** 1 gesture for direct undo; ≤ 2 to inspect and restore from trash.
- **States:** Clean stack, Undo available, Conflict detected.

#### F15 · Switch the plan folder
- **Trigger:** Working in another plan — a second term, a copied folder, a plan someone shared.
- **Path:** Titlebar folder (`app.changePlan`) → a sheet over the working plan: the plans on this machine, the ones this window remembers, *Open a folder…* (directory dialog), and a typed path → Open.
- **Budget:** 1 gesture to open, 1 to choose.
- **States:** Sheet over the plan — the window behind it never moves · Opened: the sheet closes and the window re-reads onto the new plan · Refused (a folder that is not a plan): the engine's own words in the toast, and the window stays exactly where it was, running session included.

---

## 4 · Altitude model & design rules

### 4.1 Altitude boundaries

| Altitude | Target Surfaces | Allowed Vocabulary | Prohibited Vocabulary |
| --- | --- | --- | --- |
| **1 · Daily Execution** | Today, Capture, Session Timer, Reviews | User domain nouns (chapter, minutes, questions, due, late) | Schema keys, internal IDs, JSON paths, system types |
| **2 · Local Structuring** | Plan, Subjects, Practice, Mocks, Library, Column Menus | Domain nouns plus identity pairs (Label + Key) for the active object | Global schema internals |
| **3 · System Engine** | System (Kinds, Columns, Views, Blocks, Rules, Schedulers) | Full technical vocabulary (keys, relations, expressions, JSON schemas) | None |

### 4.2 Interaction design rules

| ID | Rule | Description |
| --- | --- | --- |
| **R1** | **No dead ends** | Every screen and modal workflow must exit to a defined user surface. |
| **R2** | **Equal surface parity** | Every action must be executable via UI without requiring CLI or file edits. |
| **R3** | **Frequency-driven depth** | Core daily actions (F1–F5) must require the fewest interactions (max 2 levels). |
| **R4** | **Persistent drafts** | Form inputs and capture drafts persist across cancellations (`Esc`) and reboots. |
| **R5** | **Undo over dialogs** | Destructive actions provide immediate undo toasts; explicit confirmation used only for mass deletions. |
| **R6** | **Inline priority** | Prefer inline table/list editing over modal sheets; reserve sheets for complex records. |
| **R7** | **Explicit scent** | Disclosures and sub-menus must clearly state their contents in plain language. |
| **R8** | **Informative empty states** | Empty views state why they are empty, guide next steps, and provide an action button. |
| **R9** | **Objective progress** | Progress metrics use counts, percentages, and denominators; no gamified streaks. |
| **R10** | **Keyboard accessibility** | Full pointer-keyboard equivalence; ⌘K command palette displays shortcut badges. |
| **R11** | **Actionable onboarding** | One document in three movements — what it is, how it works, get started — drawn as the app's own material, then performed (not described) on the screen the student stands on: the one press writes the plan (`plan.new`, the bundled schema) and opens it; the student's own course and first topic are written from Today's own doors (`record.new`); the app's own timer runs the session (`session.new`, its minute floor intact); the plan's own pipeline is then walked — learn recorded from the session, prove from the student's press (`record.advanceStage`, carrying the count when the plan's proof rule applies, as the blank preset's `flip` asks of `watch` kinds), and the first grade is a real review (`record.logReview`) whose consequence the plan answered before the grade was offered. No review can be a topic's first transition — the engine refuses with `pipeline.gate` — so the walk carries every hop. The window then hands over to Today carrying the session and the graded topic. No wizard, nothing configured. |
| **R12** | **Minimal configuration** | Avoid unnecessary user settings; ship resilient defaults backed by data. |
| **R13** | **Immediate consequence** | Parameter adjustments show downstream workload projections before saving. |

---

## 5 · Quality budgets & test gates

| Gate | Acceptance Criteria | Verification Instrument |
| --- | --- | --- |
| **G1** | Launch to logged study action ≤ 3 gestures | Driven UI test |
| **G2** | Quick capture ≤ 2 gestures; drafts survive reboot | UI test + process relaunch |
| **G3** | Reviews fully navigable via keyboard (`Space`, `1`/`2`) with interval previews | Keyboard interaction test |
| **G4** | All declared write commands exposed in UI controls | Parity gate (`tools/uicheck-diff.mjs`) |
| **G5** | Zero technical schema identifiers displayed in Altitude 1 surfaces | Census gate (`tools/uicensus.mjs`) |
| **G6** | Zero dead ends across all routes and modals | Automated route traversal |
| **G7** | Inactive hiatus prompt offers 1-click catch-up | Fixture test with backdated schedule |
| **G8** | Scheduling rule changes project workload before write | Read-back simulation test |
| **G9** | Visible undo stack and persistent Recently Deleted list | Transaction test |
| **G10** | Altitude 1 navigation depth ≤ 1 hop; Altitude 2 ≤ 2 hops | View hierarchy check |
| **G11** | Full study loop executable without opening System view | End-to-end integration test |
| **G12** | All schema fields in presets carry plain-language labels | `sam schema --json` audit |
| **G13** | View clarity passes 5-second comprehensibility test | Usability audit |

---

## 6 · Implementation delivery plan

| Flow | Phase / Session | Target Scope |
| --- | --- | --- |
| F1 (Today) | P2 · U1 | Presentation query read; focus card mount |
| F2 (Study Logging) | P2 · U1 + U3 | Focus card timer + titlebar slot |
| F3 (Quick Capture) | P2 · U5 | ⌘K capture mode with draft caching |
| F4 (Review Loop) | P2 · U3 | Single card review surface with interval tags |
| F5 (Practice) | P2 · U2 | Problem set input grid |
| F6 (Mock Assessment) | P2 · U2 | Assessment breakdown and weak-topic routing |
| F7 (Plan Deck) | P2 · U2 | Inline row creation and timeline shifts |
| F8 (Progress Standing) | P2 · U4 | Clean metrics with explicit denominators |
| F9 (Model Reshaping) | P3 · U6 | System surface, block tree, kind deletions |
| F10 (Rules & Schedulers) | P3 · U6 | Impact simulation before write |
| F11 (Appearance) | P3 · U6 | Theme selector window and scaling |
| F12 (Portability) | P3 · U6 | Export/import sheets with preview diffs |
| F13 (Hiatus Catch-up) | P2 · U1 + P3 · U6 | Backlog reallocation prompt |
| F14 (Recovery & Trash) | P1 · U0 + P3 · U6 | Undo toast receipt and Recently Deleted panel |

---

## 7 · References & precedents

- **Interaction Design:** NN/g on progressive disclosure, modal interactions, confirmation patterns, empty states, and product customization.
- **Product Mechanics:** VS Code / Zed (settings and command palettes), Linear (draft preservation and undo-first deletion), Things 3 (quick capture), SuperMemo / Anki / FSRS (recall scheduling, backlog recovery).
- **Software Architecture:** Brooks (*No Silver Bullet* on essential vs. accidental complexity), Cooper (*Perpetual Intermediate*), Ink & Switch (*Malleable Software* on gradual abstraction).

---

## 8 · Open design items

1. **Signature workflow:** Review recall loop (F4) vs. daily planning execution (F1).
2. **Timer mechanics:** Active countdown stopwatch vs. direct elapsed duration logging.
3. **Hiatus recalculation:** Dynamic algorithmic rescheduling vs. simple linear deferrals.
4. **Intention data structure:** Dedicated `commitment` kind vs. inline metadata attributes on existing records.
5. **Estimate calibration:** Advisory proposal in status hint vs. automated duration multipliers.
6. **Weakness tracking:** Native `gap` entity vs. derived queries over problem set results.
