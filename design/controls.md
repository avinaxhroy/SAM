# CONTROLS — Control Chooser & Interaction Standards

Systematic standards for selecting form inputs, hint ladders, motion budgets, and state handling across SAM.

**Status:** Stable (2026-09-26).  
**Cross-references:** `UI_SPEC.md`, `UI_PLAN.md`, `design/tokens.css`.

---

## 1 · Selection gates

Every input selection must satisfy three evaluations:

| Gate | Assessment | Impact |
| --- | --- | --- |
| **Frequency** | How often is this value modified? | Governs motion and interaction budget (§6). High-frequency controls must be instantaneous and minimal. |
| **Consequence** | What downstream state depends on this value? | Determines hint depth (§3). Values altering schedules or data must display impact previews. |
| **Reversibility** | What is the cost of reversing this action? | Dictates confirmation requirements: lightweight undo (`⌘Z`) vs. impact preview dialogs. |

---

## 2 · Control chooser matrix

| Answer Space | Recommended Control | Prohibited Form | Hint Level (§3) |
| --- | --- | --- | --- |
| Boolean (`true`/`false`) | **Switch** | 2-option radio group | L0 |
| 2–5 discrete options | **Segmented control** (default selected) | Dropdown select | L1 / L2 |
| 6–20 known vocabulary | **Searchable picker** (labels first) | Native `<select>` | L1 |
| 6–20 unfamiliar items | **Browsable list** with descriptions | Raw identifier `<select>` | L2 |
| Ordered stages / ladder | **Stepped ladder** with milestone gates | Continuous slider | L2 |
| Numeric range (magnitude-first) | **Slider + numeric input + presets** | Slider alone | L2 |
| Exact numeric value | **Stepper or numeric input** | Freehand slider | L2 |
| Cyclic / angular quantity | **Dial / radial ring** (read-only, §5) | Dial for linear budgets | L2 |
| Date | **Date picker with offset chips** (`+7d`, `+1m`) | Raw ISO text input | L1 / L2 |
| Linked entity / relation | **Searchable picker** (recent-first) | Comma-separated ID inputs | L1 |
| Unconstrained text | **Inline input with suggestions** | Unassisted blank input | L1 |

**Core rules:**
- Dropdowns are restricted to large, universally standardized vocabularies (>20 items).
- Precision numeric inputs must always provide direct keyboard entry.

---

## 3 · Hint ladder

| Level | Content | Requirement |
| --- | --- | --- |
| **L0** | Primary label | Mandatory for all controls |
| **L1** | Units and formatting (`45m`, `in 23 days`, `3 of 9`) | Mandatory whenever units apply |
| **L2 · Consequence** | Downstream impact (resulting interval, new daily load, record count) | Mandatory whenever a value mutates downstream state |
| **L3 · Preview** | Pre-commit projection with accept/decline actions | Mandatory for structural and bulk schema writes |

**Rule:** Any control whose value modifies future schedules or model state must display that consequence directly at the control before commit.

---

## 4 · Field-type control mappings

| Field / Type | Assigned Control | Hint Specification |
| --- | --- | --- |
| `text`, `longtext` | Inline input (longtext expands to panel) | L0 |
| `number` | Numeric stepper + input | L2 (aggregate target impact) |
| `duration` (`est`, `min`) | Duration chips (`15 · 25 · 45 · 60`) + stepper | L2 (calibrated estimate feedback) |
| `date` | Date control + relative offset chips | L1 / L2 (relative distance: `in 23 days`) |
| `select` (≤ 5 items) | Segmented control | L2 (downstream requirement rules) |
| `select` (> 5 items) | Searchable picker with descriptions | L1 |
| `multiSelect` | Searchable picker with selected chip bar | L1 (count of selected items) |
| `bool` | Toggle switch | L2 (explicit behavior outcome) |
| `relation` | Searchable label-first picker | L1 (resolved target title, never ID) |
| `url` | Text input + direct launch action | L1 (target domain) |
| `formula` | Read-only computed field + formula explanation | L2 (expression summary in plain language) |
| Stage ladder | Stepped control | L2 (prerequisite stage rules) |
| Review ratings | 4 explicit buttons with interval labels | L2 (interval consequence as button label) |
| Scheduler select | Radio cards with workload simulation | L2 (projected daily reviews curve) |
| `dailyTargetMin` | Slider + direct numeric field + preset buttons | L2 (estimated session count) |
| Retention rate | Slider + numeric field | L2 (projected workload curve) |
| Token overrides | Token selector + typed value input | L1 (resolved source value) |
| Timer | Read-only radial progress ring | L1 (elapsed time vs. target) |

---

## 5 · Radial vs. linear controls

- **Radial for reading; linear for writing.**
- Radial rings are reserved for read-only cyclical progress tracking (e.g. session countdown timer).
- Linear sliders and numeric steppers are mandatory for setting discrete numeric values and budgets.

---

## 6 · Motion budgets

Excessive animation degrades utility. Transitions are governed by interaction frequency:

| Frequency | Motion Allowance | Implementation Target |
| --- | --- | --- |
| High (multiple times/session) | **0–100ms** (Instantaneous) | Cell editing, review rating, row toggling |
| Medium (daily operations) | **140ms** (`--dur-1`) | Route navigation, sheet opening |
| Low (weekly / milestones) | **220ms** (`--dur-2`) | Session completion, milestone toasts |
| Unique | **350ms** (`--dur-3`) | Single signature onboarding transition |

---

## 7 · Control states & accessibility

Every interactive control must support all standard states:
`default` · `hover` · `focus-visible` · `active` · `disabled` · `loading` · `error` · `success`

**Accessibility requirements:**
- Explicit keyboard focus ring via `design/base.css` — 2px of `--ink` with a 2px offset around a control that stands on a surface, and the same ring drawn *inside* the edge (`outline-offset: -2px`) for a field (`input`, `textarea`, `select`) and for any control its container clips. A field is a recessed well, and a ring painted outside it is squared at its corners (an offset outline runs on the element's own radius) and cut off by the first ancestor that clips. And because that outline runs on the *element's* radius: a field whose box reaches its container's rounded corner wears that corner's inner radius — the corner less the field's own inset — so the ring runs concentric with the clip instead of squaring off under it (`ui/src/variants/palette/C.svelte`).
- Descriptive `aria-label` or visible text on every control.
- Information conveyed independently of color alone.
- `loading` is **stated, never `disabled`**: a control whose own write is in flight carries `aria-busy` and keeps its focus and its place in the tab order — `disabled` is reserved for what cannot be chosen (a bound reached, an action that does not apply). Disabling a control that is merely working drops the keyboard's focus to the body, and the write outlives the press by microseconds.
- Compliance with global `prefers-reduced-motion` settings.

---

## 8 · Negative constraints

- No dropdown menus for sets of 5 or fewer choices.
- No freehand sliders without adjacent numeric input boxes.
- No rotary dials for setting linear budgets.
- No non-standard inputs where native HTML equivalents suffice.
- No mouse-only tooltips (hints must be accessible to keyboards and screen readers).
- No animated transitions on high-frequency daily controls.

---

## 9 · Verification protocol

1. **Control audit:** Verify all preset schema fields resolve to controls specified in §4.
2. **Accessibility testing:** Full gate verification via `tools/a11ycheck.mjs`.
3. **Vocabulary validation:** Enforce zero internal keys or identifiers via `tools/uicensus.mjs`.
4. **Motion audit:** Confirm zero transition delays on high-frequency daily controls.
