# UI SPEC — Presentation Census, Scales, and Contracts

Core UI specifications: presentation census, scales, skeleton architecture, engine contracts, and test instruments.

**Status:** Stable (2026-09-26).  
**Cross-references:** `UI_PLAN.md` (§5–6), `UX_FLOWS.md` (§3–5), `design/controls.md` (§2–6), `COMPOSER.md`.

---

## 1 · The presentation census (stage 3)

| # | Region | Forms Considered | Committed Spec | Tie / Rationale |
| --- | --- | --- | --- | --- |
| R1 | Window | Sheet · Framed room (24px) · Chromeless | **Edge-to-edge desktop window** | Native window behavior (`decorations: true`, `data-window="flush"`) |
| R2 | Titlebar | Path/rev · Greeting/week · Blank | **Identity left, context center-left, tools right** | Displays `week n of 16`; no raw paths/hashes |
| R3 | Rail | Discs only · Labeled rail · Hybrid rail | **Date disc · Destination icons with unfolding labels · Review count · Capture (+) · System at bottom** | Glass discs; unfolds on hover/focus (`design/components.css`) |
| R4 | Screen head | Dev toolbar · Title only · Title + action | **Title + max 1 primary action + view controls** | User-facing title, clean layout controls |
| R5 | Today · Focus card | Plain row · Action card · Card + gauge | **Course chip · Action (`--text-xl`) · Factual why-line · Start + duration · Not this** | Objective facts over artificial priority scores |
| R6 | Today · Day's queue | Table · List rows · Timeline · Board | **Rows (`.cd-task`): State dot · Title · Meta line (course chip, due, est) · Start** | Direct action rows |
| R7 | Today · Coming up | Table · Dated rows · Calendar strip | **Dated rows with relative distance (`in 2 days`, `1d late`)** | Human-readable relative time |
| R8 | Record table | Pill-chip table · Dense hairline · Card grid | **Dense table: 32px rows, 13px cells, hairlines, sticky header & 1st col, full-value reveal on hover/focus** | 40-column data density |
| R9 | Board | Equal columns · Content-sized · Swimlane | **Equal-width columns (12px gutter), header (label + count), stacked row-species cards** | Columnar progression |
| R10 | Calendar | Month grid · Week strip · Agenda list | **Month grid with 76px day cells, undated top strip for dateless records** | Grid calendar with backlog buffer |
| R11 | Timeline | Spine with ticks · Gantt · Unit list | **Spine + tick rows, grouped and ordered by plan keys** | Objective chronological ordering |
| R12 | Chart | Bars · Ceiling stems · Sparkline | **Stems against labeled ceiling with factual caption; no animated bars** | Clean, non-distracting metrics |
| R13 | Stat | Big number · Number + denominator · None | **Number + denominator + label (max 4 per row)** | Contextual denominators |
| R14 | Review · Recall | Queue rows · One card · Two-pane | **Full-width card, Space to reveal, grade buttons below** | Focused recall session |
| R15 | Review · Queue | Rows · Table · Calendar | **Rows: Title · Course chip · Elapsed time (`9 days ago`). Header: count + estimated minutes** | Oldest-first ordering |
| R16 | Reference lists | Table · Card grid · Two-pane preview | **List rows with title, metadata, and 1-line preview** | Reference-grade information density |
| R17 | Sheets | Single 460px sheet · Job sheets · Inline | **460px sheets reserved for creation and destructive actions; other edits inline or in detail panel** | Minimized modal interruption |
| R18 | System | Settings dump · Grouped list · Object table | **Searchable list grouped by object type (kinds, columns, views, blocks, rules, schedulers, pipelines)** | Inspectable model layer |
| R19 | States | Text · Skeletons · Drawn empty | **Four explicit states: Loading (skeleton), Empty (title + 1-sentence + 1-action), Error (cause + recovery), All-clear** | Deterministic state UI |
| R20 | Palette | Command list · Capture + commands + objects | **Unified input: Quick capture → Commands → Domain objects** | Global action dispatcher (⌘K) |
| R21 | First run | Room · Empty Today · Working plan | **Full-window room: what it is · how it works · get started — the three movements drawn at rest, and each performed on the screen the student stands on: the room's own press writes the plan (bundled schema) and opens it, Today teaches the student's own course and first topic, the app's own timer runs the session, and the first grade is a real review whose consequence the plan answered first; then the hand-off to Today carrying both; a machine whose plans already exist but none is open draws the same room on its last movement, rows and form together, with the two movements one press back** | Zero-friction start; no rigid wizard |

---

## 2 · The declared scales (stage 4)

### 2.1 Inherited (`design/tokens.css` / `tokens.json`)

| Token Family | Declared Values |
| --- | --- |
| Type Size | `2xs (11px)`, `xs (13px)`, `base (15px)`, `md (17px)`, `lg (20px)`, `xl (26px)`, `2xl (32px)` (empty/error titles only) |
| Tracking | `display (-0.030em)`, `title (-0.018em)`, `body (-0.006em)`, `code (0.020em)`, `caps (0.070em)` |
| Spacing | `3xs (2px)`, `2xs (4px)`, `xs (8px)`, `sm (12px)`, `md (16px)`, `lg (20px)`, `xl (24px)`, `2xl (32px)`, `3xl (48px)` |
| Radius | `card (24px)`, `tile (18px)`, `disc (16px)`, `item (14px)`, `mini (12px)`, `mark (9px)`, `key (6px)`, `pill (999px)` |
| Shadow | `1`, `2`, `3`, `pop`, `ink` (plus dark ladder) |
| Motion | `dur-1 (140ms)`, `dur-2 (220ms)`, `dur-3 (350ms)`, `loop (1400ms)`; easing `out [0.16,1,0.3,1]`, `pop [0.22,1,0.36,1]` |
| Geometry | Titlebar: 44px; Rail: 88px (disc 44px, label 70px); Gutter: 24px; Canvas max: 1120px |
| Typography | Plus Jakarta Sans; weights: 400 (regular), 600 (semibold), 700 (bold) |

### 2.2 App-layer delta

| Object | Metric / Rule | Context |
| --- | --- | --- |
| **Row Species A (Work row)** | `padding: 16px 24px`, grid `auto / 1fr / auto`, gap 16px, title `15/600`, meta `11`, dashed divider; hover/focus actions | Adopted from `.cd-task` |
| **Row Species B (Table row)** | Height 32px, cell padding `0 10px`, font 13px, numbers tabular right-aligned, hairline border | Data table density |
| Table head | Height 34px, 11px uppercase with `tracking.caps`, sticky | Schema controls live in column menus |
| Column widths | Min 96px, max 320px, text columns `1fr` with truncation, first column sticky | Fluid table scaling |
| Field controls | Inline height 28px; chips 22px; fixed layout | Prevents grid shift on cell edits |
| Sheet | Width 460px, max-height 80vh, header 44px, footer 56px, scrolling body | Standard modal dimensions |
| Detail panel | Width 360px, full height, independent scroll | Side panel inspector |
| Canvas | Max width 1120px centered; side padding decreases below 1024px before columns hide | Main content viewport |
| **z-order ladder** | Canvas: `0` · Card: `10` · Sticky header: `20` · Menu: `30` · Scrim: `40` · Sheet / Palette: `50` · Toast: `60` · Drag ghost: `70` | Prevents overlay collisions |
| Motion rules | State changes / commits: **0ms**; Hover / press: `dur-1`; Sheets / menus enter: `dur-2`; Ring animations only: `dur-3`/`loop` | High responsiveness floor |
| Icons | Single family, 16px bounding box, 1.5px stroke, monochrome except state indicators | Accessible labels on all controls |
| Themes | Four shipped themes: Cadence Light, Cadence Dark, Daylight Console, High Contrast | Applied before first paint |

---

## 3 · The skeleton round (stage 5)

| Candidate | Structure | Status | Usage |
| --- | --- | --- | --- |
| **S-A · Ledger** | Rail → single canvas (1120px) → Today (focus card over rows); tables; sheets | **Adopted** | Primary application shell |
| **S-B · Workbench** | Rail → list pane → detail pane | Adapted | Detail pane adopted for records and machine inspector |
| **S-C · Day-shell** | Rail as time lenses; persistent right strip with timer/current item | Adapted | Titlebar timer slot adopted |

---

## 4 · Engine surfaces the UI depends on

### E1 · `today.view` (read) — U1

```typescript
today.view({ date?: string, window?: 'day' | 'week', limit?: number })
```

Returns objective facts, without arbitrary rankings:

```json
{
  "date": "2026-10-04",
  "weekIndex": 6,
  "weekOf": 16,
  "groups": [
    { "id": "late", "items": [], "count": 0 },
    { "id": "due", "items": [], "count": 0 },
    { "id": "next", "items": [], "count": 0 },
    { "id": "stale", "items": [], "count": 0 }
  ],
  "totals": { "plannedMin": 0, "loggedMin": 0, "targetMin": 0 },
  "allClear": true
}
```

Item schema: `{ id, kind, label, course?, dueDate?, lateDays?, est?, reason }`. Items within groups follow plan ordering; group ordering is fixed (`late → due → next → stale`).

### E2 · Core delete commands — U6

| Command | Parameters | Cascade Behavior | Confirmation |
| --- | --- | --- | --- |
| `type.delete` | `{ name, mode: 'records' \| 'schema' }` | `records`: deletes kind, all records, and referencing relations. `schema`: deletes TypeDef only (refuses if records exist). | Typed name confirmation required if records exist |
| `view.delete` | `{ name }` | Deletes view, blocks, and rail entry; recreates default type view. | Standard dialog |
| `list.delete` | `{ name, keepView?: boolean }` | Deletes rail entry; with `keepView: true` preserves view as unlisted. | Standard dialog |

All delete operations are transactional, undoable via `⌘Z` / `Ctrl+Z`, and tracked in Recently Deleted (F14).

---

## 5 · Test instruments

| # | Instrument | Verification Target | Status |
| --- | --- | --- | --- |
| I1 | **Reverse parity** (`tools/uicheck-diff.mjs`) | Every declared write command has an accessible UI control or documented door | Active |
| I2 | **Label completeness** | Every field in bundled presets carries a user-facing label (`field.no-label`) | Active |
| I3 | **No-model workflow** | Full study loop (capture, log session, grade, review) executable without opening System | Active |
| I4 | **5-second test** | Core purpose of each screen immediately identifiable on display | Active |
| I5 | **UI census** (`tools/uicensus.mjs`) | Zero forbidden schema keys, raw IDs, or camelCase strings rendered in student surfaces | Active |
| I6 | **A11y & token lint** | Keyboard navigation, focus rings, contrast ratios, and strict token adherence | Active (`tools/gates.mjs`) |

---

## 6 · Open items ledger

| # | Topic | Resolution / Recommendation | Target |
| --- | --- | --- | --- |
| Q4 | Intention tracking | `commitment` kind with inline record field shorthand | `today.commitments` in E1 |
| Q5 | Commitment duration unit | Per-commitment, defaulting to last selected unit | Focus session card |
| Q6 | Estimate calibration | Advisory proposals in hint slot (non-judgmental) | Review interface |
| Q7 | Learning gap tracking | Route through problem set records before adding standalone kind | Study practice views |
| Q8 | Focus card contents | Course chip, action title, factual reason, Start button + duration, "Not this" | R5 & E1 |
| Q9 | Rail icon family | Unified 16px stroke icon set (§2.2) | Navigation rail |
| Q10 | System entry placement | Dual access via rail footer and global ⌘K palette | Settings / Machine |

---

## 7 · Metric derivations

- **Tokens:** Derived from `Sources/SAMCore/Resources/tokens.json` (v4.7) and `design/tokens.css`.
- **Component dimensions:** Work rows (16/24px padding), calendar cells (76px min-height), chart containers (120px height), rail (88px width), canvas (1120px max-width) match `design/components.css`.
- **Table density:** 32px row height and 34px header height enforce high information density without sacrificing touch targets.
- **z-index hierarchy:** Prevents rendering collisions across canvas (0), cards (10), sticky heads (20), menus (30), scrims (40), sheets (50), toasts (60), and drag ghosts (70).
