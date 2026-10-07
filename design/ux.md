# Cadence — UX Contract

Product and interaction specifications for the Cadence desktop interface: information architecture, screen contracts, state matrix, and interaction standards.

---

## 1 · Product definition

Cadence manages daily study scheduling, revision loops, and progress tracking:
- **Core scope:** Daily decision surface, term planning, topic mastery state, recall review queues, study session logging, factual progress reporting.
- **Out of scope:** Submissions, file management, institutional integrations, social comparison, gamified streaks.

---

## 2 · Target use cases

- **Immediate action:** Identify and initiate the single highest-priority task in ≤ 10 seconds without scrolling.
- **Term planning:** Evaluate semester deadline distributions and collision weeks at a glance.
- **Exam revision:** Review batches of scheduled recall cards via keyboard in < 4 minutes without modal interruptions.
- **Hiatus recovery:** Re-engage after study breaks with factual backlog summaries and catch-up proposals without penalty or guilt framing.

---

## 3 · Information architecture

Five primary destinations (single-level hierarchy):

```text
Today      #/today      Daily focus decision and schedule queue (default)
Plan       #/plan       Term timeline, upcoming deadlines, exam milestones
Courses    #/courses    Course catalog, syllabi, topic mastery states
           #/course/:id Course detail (sequential topics and deadlines)
Review     #/review     Spaced repetition recall queue
Progress   #/progress   Logged study time, completion metrics, session history
System     #/system     Design system specimens and token inspectors
```

**Navigation rules:**
- Fixed 5-destination rail on desktop; switches to horizontal strip at < 760px.
- Destination hotkeys `1`–`5`; global capture `a`; command console `⌘K` / `Ctrl+K` / `/`.
- Rail uses glass disc icons with labels revealed on hover/focus.
- Today displays the current date (`23` over `WED`); Review displays the pending recall count.

---

## 4 · Recommendation engine

The "Next up" engine computes one actionable task with a factual rationale:

- **Inputs:** Deadline urgency, review lapse intervals, exam proximity, 7-day logged study time, estimated task duration.
- **Output:** One topic, one action verb, and one factual why-line (e.g. `due Thu · last reviewed 9 days ago`).
- **User autonomy:** Focus card duration is editable (5–180 minutes); users may dismiss items via `Not this` without penalty.

---

## 5 · Screen specifications

### 5.1 Today
- **Header:** Time-of-day greeting, current date, week number (`week n of 16`), and weekly logged study hours.
- **Focus card:** Prominent card displaying course chip, action title (`--text-xl`), factual why-line, 180° topic coverage gauge, `Start` action, and duration capsule.
- **Daily queue:** Ordered task rows (`late → due → next in plan → stale`) with state indicators, metadata, and direct start triggers.
- **Coming up:** Chronological preview of the next 3 upcoming deadlines.
- **This week:** 7-day study bar chart plotted against a fixed 90-minute ceiling.

### 5.2 Plan
- **Header:** Summary of pending deadlines and exams within selected horizon.
- **Horizon filter:** Segmented switch (`1 week` / `2 weeks` / `A month`).
- **Week overview:** 7-day calendar strip showing daily deadline counts.
- **Exam deck:** Countdown tiles displaying days remaining, course code, exam title, and syllabus coverage.
- **Deadlines list:** Chronological list of upcoming deliverables with overdue items highlighted.

### 5.3 Courses
- **Catalog view:** Course cards displayed in distinct category washes with course code, topic progress fraction, and instructor info.
- **Course detail (`#/course/:id`):**
  - Sequential syllabus topics showing mastery ladder: `Not started → Learning → Proof → Solid`.
  - Topics advance to `Proof` via practice and to `Solid` only via successful recalls.
  - Topic rows expand inline to show recent test history and duration logging.

### 5.4 Review
- **Queue header:** Pending recall count and estimated completion time (`12 waiting · about 18 minutes`).
- **Recall card:** Single card displaying question text (`--text-xl`); `Space` toggles answer reveal.
- **Rating controls:** 4 consequence-labeled grades:
  - `1` (Forgot): Rescheduled for tomorrow.
  - `2` (Hard): Retains learning status.
  - `3` (Good): Advances toward proof.
  - `4` (Solid): Retains interval for 2 weeks.
- **Keyboard navigation:** `Space` (reveal) → `1`–`4` (grade) → automatic advance.

### 5.5 Progress
- **Key metrics:** Minutes logged, completed sessions, active study days (`10/14`), and recalls completed.
- **Daily activity chart:** 16px vertical ink stems representing daily study minutes against a 90-minute ceiling.
- **Course allocation:** Proportional time distribution per course.
- **Mastery ledger:** Breakdown of syllabus topics across mastery stages. Zero gamified streak scores.

### 5.6 System
- Live token specimens, color swatches, typographic hierarchy, and interactive component test states rendered from production CSS.

---

## 6 · Primary interaction flows

- **First run:** The room — *what it is* (SAM, one plan of plain files on this machine) · *how it works* (the same plan two ways in — pressing in the app, and editing the plan's own files, each drawn as a specimen) · *get started* — three screens of the app's own material on one ground of the room's own pastel colours — one wash with a hairline weave drawn across it — no mark finer than the type — with the last screen carrying the press that writes the plan (`plan.new`: the blank plan, or a preset the bundle ships, one text link away) and the plans to open instead — what the plans root holds, and any plan this machine remembers opening outside it — and a machine whose plans already exist but none is open draws the same room on that last screen, with the two movements one press back. The same three screens are the app's own document, re-readable over a working plan as the Getting-started sheet (`app.gettingStarted`), where the form gives way to the hand-over: every deed after the press belongs to the screen the student is standing on — Today names what to add first, the queue row starts the session under the app's own timer, and the first grade is a real review whose date the plan answers. Nothing is configured, and no screen repeats another's teaching.
- **Focus session:** Click `Start` on focus card or queue row → timer pill mounts in titlebar → `Stop` persists study session log (minimum 1 minute threshold).
- **Quick capture:** Press `a` or `⌘K` → type task name → `Enter` commits to Today's queue.
- **Spaced review:** Review → `Start reviewing` → reveal answer (`Space`) → assign grade (`1`–`4`) → session completion ledger.
- **Hiatus recovery:** Launching after ≥ 3 days inactive displays objective backlog recap and optional multi-day catch-up schedule.

---

## 7 · Product voice & copy standards

- **Direct and objective:** State facts, numbers, and dates plainly; avoid patronizing motivation or streak-loss shaming.
- **Constructive error handling:** State what failed, what data was preserved, and provide an immediate retry action.
- **Both halves of a capability, both stated:** anything the app can do is reachable by a control *and* by editing the plan's own files. Say so in that order, and say plainly that nothing requires hand-editing a file — a student who reads the files as the app's manual rather than as their own copy of the plan has been told the wrong half first.

---

## 8 · State matrix

| State | Today | Plan | Courses | Review | Progress |
| --- | --- | --- | --- | --- | --- |
| **Empty** | Onboarding CTA | "Add a deadline" | "Add your first course" | "Nothing due" | "No sessions yet" |
| **Loading** | Focus skeleton + rows | Deck skeleton | Grid skeleton | Card skeleton | Chart skeleton |
| **Partial** | Card + sync status | Unscheduled list | Unscheduled badge | Active queue items | Chart with missing days |
| **Error** | Inline card retry | Toast notification | Inline alert | Card retry button | Retains cached chart |
| **Overflow** | Capped at 5 rows (`+n more`) | Paginated deck | Scrolling grid | Capped session | Scrolling 14-day window |

---

## 9 · Accessibility & input mapping

- Full keyboard-pointer parity across all workflows.
- OS-adaptive shortcut labels (`⌘K` on macOS; `Ctrl+K` on Windows/Linux).
- Explicit visible focus rings via `design/base.css`.
- Touch/click targets ≥ 32px; core buttons 40px height.
- Zero reliance on color alone to convey semantic status.

---

## 10 · Excluded capabilities

- GPA and formal grade calculation.
- Native rich-text note taking (links to external URLs/files instead).
- Multi-user collaboration and public leaderboards.
- Complex nested scheduling configurations.

---

## 11 · Core quality metrics

- **Time to first action:** < 15 seconds from launch to active study session.
- **Review throughput:** Uninterrupted card rating execution via single keystrokes.
- **Hiatus resilience:** Seamless return to study routine without backlog overload.

---

## 12 · Open design validations

1. Optimal task queue length cap (3 vs. 5 items).
2. Cognitive clarity of 7-day activity stems without textual legends.
3. Deferral interaction ergonomics: single-click skip vs. structured reason selection.
4. Information clarity of course wash indicators in high-density listings.
