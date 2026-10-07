# UI PLAN — Interface Architecture & Implementation Plan

Strategic and technical plan for SAM's user interface rebuild: architecture diagnosis, structural decisions (D1–D13), phase milestones (P1–P3), and quality gates.

**Status:** P1 and P3 completed (2026-09-26).  
**Cross-references:** `SAM_PLAN.md`, `UI_SPEC.md`, `UX_FLOWS.md`, `design/design.md`.

---

## 1 · Architecture diagnosis

The initial UI exposed internal schema representations directly in student task views:
1. **No direct decision surface:** The engine lacked a dedicated presentation read for "what to do next".
2. **Schema leakage:** Subtitles and table headers displayed raw pointers (`views.json#/views/<id>`) and camelCase keys (111 forbidden strings detected on baseline).
3. **Redundant creation paths:** Duplicate action buttons rendered in close proximity with overlapping responsibilities.
4. **Opaque navigation:** Rail rendered single-letter abbreviations without context.
5. **Incomplete lifecycle exits:** In-app settings lacked exit pathways; kind/view deletions had no UI exposure.

**Preserved invariants:** Zero raw color literals; full token compliance (`design/tokens.css`); unified command registry over 4 equal doors (click, ⌘K, JSON file, CLI); transactional undo; accessibility harness.

---

## 2 · Technical baseline & audit

| Feature | Design System Specification (`design/`) | Initial Implementation |
| --- | --- | --- |
| Focus card | Engine presentation read, why-line, `Not this` exit | Missing (`.cd-focus` unmounted) |
| Navigation | Unfolding labels, date disc for Today, count on Review | Letter discs without count badges |
| Header context | Date and week index (`week n of 16`) | Raw JSON pointer and revision hash |
| Theming | Course washes and clean themes | Single static theme attribute |
| Recall session | Card-at-a-time, `Space` reveal, consequence hints | Table rows with raw engine stage pills |
| Keyboard access | Single-key accelerators (`a`, `1`–`5`, `/`) | Command-only palette |
| UI states | Skeleton loading and actionable empty states | Generic `working…` text string |

---

## 3 · Category standards & design patterns

- **Skeleton:** 3-pane workbench (Sidebar → List/Table → Detail inspector), unified ⌘K palette, centered empty states.
- **Density:** 12–13px typography for data grids; 28–34px table row heights; restrained neutral palettes; 16px stroke icons.
- **System boundary:** Avoid excessive manual customization overhead; provide deterministic, transparent scheduling proposals rather than opaque calendar rewrites.

---

## 4 · Core product strategy

- **Surface classification:** Study management workflow (Altitude 1 & 2) coupled with an inspectable model management layer (Altitude 3).
- **Core value proposition:** Leverage SAM's comprehensive structured plan data (entities, relations, rules, schedulers) to provide a friction-free daily decision surface.
- **Authority split:**
  - **Craft (`design/`):** Tokens, component styles, controls, typography, and motion.
  - **Product (`SAM_PLAN.md`, `UX_FLOWS.md`, `UI_PLAN.md`): Information architecture, workflows, data schemas, and domain copy.

---

## 5 · Structural decisions

- **D1 — Screens present decisions rather than enforcing them:** Today displays objective facts (`late`, `due`, `next in plan`, `stale`) with consequences. Priority selection remains with the student; auto-scheduling rewrites are prohibited.
- **D2 — Name-first with identity pairs on demand:** Primary labels use student domain language. Schema identifiers (keys, IDs) appear exclusively in identity pairs (Label + Monospace Key) beside controls or in developer tools.
- **D3 — Single door per action:** Each command maps to exactly one visible primary control per surface. ⌘K and hotkeys serve as non-duplicative shortcuts.
- **D4 — Explicit navigation labels:** Navigation rail provides legible names and badges on hover/focus; single-letter discs are prohibited.
- **D5 — Direct adoption of design tokens and components:** Imports `design/components.css` directly. Application-specific overrides are restricted to domain-specific grids (RecordTable, ColumnMenu, Sheet wrappers).
- **D6 — Dedicated preset layouts:** Presets provide custom designed layouts rather than uniform tabular lists.
- **D7 — Dedicated Settings window:** Preferences maintains clean modal and native window dismissals.
- **D8 — Explicit state specifications:** Loading (skeletons), empty (actionable prompts), error (diagnostics), saving, and undo states are designed per surface.
- **D9 — Motion budgeted by frequency:** High-frequency interactions execute at 0ms; transitions capped at `dur-1` (140ms).
- **D10 — Sacred list (architectural invariants):** Four-door parity (UI, ⌘K, file, CLI); unified command registry and validator; transaction/undo engine; accessibility gates; existing plan schemas.
- **D11 — Full bilateral UI completeness:** Every command supported by the engine has a direct UI affordance. Neither JSON editing nor CLI usage is required for any operation.
- **D12 — Dedicated System surface for data model management:** Types, columns, views, blocks, rules, and schedulers are managed in a centralized, searchable System view.
- **D13 — Three altitude tiers with progressive disclosure:**
  - **Altitude 1 (Daily execution):** Today, Capture, Timer, Reviews. Uses student domain vocabulary.
  - **Altitude 2 (Local structuring):** Plan, Subjects, Practice, Mocks, Column headers. Includes identity pairs for active elements.
  - **Altitude 3 (System engine):** System view. Exposes full technical schemas, relations, and expressions with dependency trees.

---

## 6 · Implementation roadmap

### 6.1 Surface map

| ID | Surface | Initial State | Rebuilt Specification |
| --- | --- | --- | --- |
| S0 | Frame & Rail | Letter discs, path titles | Week indicator, date disc, unfolding labels |
| S1 | Today | Generic stat blocks | Decision deck backed by `today.view` (E1) |
| S2 | Plan & Courses | Repetitive tables | Hierarchical syllabus breakdown (Units → Courses → Weeks) |
| S3 | Practice | Raw float scores | Attempted/solved summary with fast entry |
| S4 | Mocks | Empty calendar | Calendar grid with integrated date assignment |
| S5 | Reviews | Raw table rows | Focused card recall flow with consequence tags |
| S6 | Progress | Duplicated metrics | 4 key denominator metrics + ceiling chart |
| S7 | Reference | Raw tables | Reference layout with inline previews |
| S8 | Work Grid | Schema header | 32px dense rows with contextual column menus |
| S9 | Sheets | Generic 460px dialogs | Split: quick entry sheets + System panel inspectors |
| S10 | Source Pane | Titlebar button | Contextual JSON inspector with live synchronization |
| S11 | Palette | Internal verb dump | Searchable capture, actions, and objects (⌘K) |
| S12 | Settings | Flat property list | Grouped preferences with direct UI controls |
| S13 | State Handlers | Text fallbacks | Structural skeletons and informative empty states |
| S14 | System | Unhandled / scattered | Centralized inspector for kinds, columns, views, blocks, rules |

### 6.2 Phased delivery

#### Phase 1 · Foundation (P1 — Session U0) — Completed
- Imported `design/components.css` and removed duplicate styles.
- Applied `UI_SPEC.md` §2.2 metrics (32px rows, z-ladder, icon tokens).
- Integrated `IdPair.svelte` across sheets and tables.
- Added user-facing labels to all preset fields.
- Implemented `tools/uicensus.mjs` verification gate.

#### Phase 2 · Daily Product (P2 — Sessions U1–U5)
- **U1 (Today):** Presentation read `today.view` (E1: late, due, next, stale). Focus card with why-line.
- **U2 (Work Surfaces):** Rebuilt Plan, Practice, and Mocks views with dedicated layouts.
- **U3 (Reviews & Timer):** Recall card interaction (`Space` + `1`/`2`) and titlebar timer.
- **U4 (Progress):** 4 denominator-backed metrics and ceiling stem charts.
- **U5 (Capture & Editing):** Inline column manager, fast capture palette, undo receipts.

#### Phase 3 · System & Release (P3 — Sessions U6–U7) — Completed
- **U6 (Machine & Doors):** Built `System.svelte` (tree block editor, dependency graph, rule simulation). Implemented cascading deletes (`type.delete`, `view.delete`, `list.delete`).
- **U7 (Verification):** Full gate validation across tokenlint, a11y, census, and parity checks.

### 6.3 Definition of done

1. **Identifier census = 0:** Zero technical keys, JSON pointers, or raw paths visible in student views (`tools/uicensus.mjs`).
2. **Single door per action:** No duplicate buttons for identical command IDs on any screen.
3. **Universal navigation:** All destinations reachable in 1 click; no terminal exit traps.
4. **Time to value:** Core study action executable in ≤ 3 gestures from launch.
5. **Complete state lifecycle:** Verified loading, empty, saving, error, and undo states.
6. **Token & a11y compliance:** Passes `tokenlint.mjs` and `a11ycheck.mjs`.
7. **Bilateral UI parity:** Every engine write command exposed via UI controls.
8. **No-model test:** Complete study week executable without accessing System.
9. **Hop budget:** Max 1 hop for Altitude 1; max 2 hops for Altitude 2.
10. **Labeled presets:** All bundled schema fields include user-facing labels.

### 6.4 Census baseline & verification

```bash
node tools/uicensus.mjs --report census-*.json
```

- **Baseline (2026-09-26):** 111 forbidden strings detected across 10 views (Exit 1).
- **Control target (`design/demo/`):** 0 forbidden strings across 523 parsed tokens (Exit 0).
- **Post-P3 status:** 0 forbidden strings across all views and sheets.

---

## 7 · Risk mitigations

| Risk | Impact | Strategy |
| --- | --- | --- |
| **New engine presentation read** | Requires backend Rust modifications | Scoped strictly to deterministic queries (`today.view` E1); no complex heuristic ranking |
| **Authority conflicts** | Drift between design system and app code | Strict separation: `design/` governs tokens and component styling; specs govern data and architecture |
| **Feature prioritization drift** | Machine customization prioritized over study loop | Gate enforcement: P2 daily flows must pass before P3 machine features unlock |
| **Plan migration burden** | User plans broken by schema changes | Purely additive changes; backward compatibility guaranteed by transaction engine |

---

## 8 · Architectural alignments

- **Schema exposure:** Name-first by default; schema keys exposed exclusively on demand via identity pairs and developer doors.
- **Design system benchmark:** `design/` defines the craft quality floor; application screens derive custom layouts matching that quality.
- **Workflow emphasis:** Maximize clarity of the daily decision loop without taking agency away from the student.
- **Rebuild scope:** Complete presentation-layer overhaul above the existing Rust engine and transaction core.
