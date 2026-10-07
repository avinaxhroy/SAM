# Cadence — Design System

**Name:** Cadence · **System:** "Warm Workbench" · **Version:** 4.7  
**Platform:** Cross-platform desktop (macOS · Windows · Linux)  
**Files:** `tokens.css` · `base.css` · `tokens.json` · `tailwind.css` · `components.css` · `fonts.css` · `controls.md` · `tools/tokens.mjs` · `ux.md` · `demo/`

This document defines the visual system and interaction mechanics. `tokens.css` and `components.css` define the binding implementation. Where documentation and CSS disagree, the CSS takes precedence.

```html
<link rel="stylesheet" href="fonts.css" />    <!-- Bundled typeface -->
<link rel="stylesheet" href="tokens.css" />   <!-- CSS custom properties only -->
<link rel="stylesheet" href="base.css" />     <!-- Window shell defaults -->
<link rel="stylesheet" href="components.css" /><!-- Reusable components -->
```

---

## 1 · Purpose & boundaries

Cadence is a focused desktop study manager for university students balancing multiple courses and exam horizons.

| Prohibited Paradigm | Reason |
| --- | --- |
| Freeform canvas / dashboard builder | Cognitive overhead; students need immediate guidance, not layout authoring. |
| Gamified streaks | Streak loss creates shame and abandonment; progress must reflect actual work done. |
| Isolated timer | Time tracking is an output, not a scheduling plan. |
| Pure flashcard trainer | Spaced repetition is an internal engine, not the entire interface. |
| Institutional LMS mirror | LMS manages official submissions; Cadence manages personal study routines. |
| Generic web dashboard | Desktop-native application optimized for keyboard navigation and local focus. |

---

## 2 · Situation & requirements

1. **Immediate action:** Recommend one concrete action with a clear reason and an explicit dismiss/defer option.
2. **Objective progress:** Display work completed and factual intervals; eliminate streak counters and guilt-inducing alerts.
3. **Progressive disclosure:** Reduce cognitive load by presenting today's tasks first, keeping deeper configurations one door away.
4. **Active retrieval:** Topic mastery advances only on verified retrieval, never on passive checkmarks.
5. **Deterministic queues:** Spaced repetition schedules appear as clear work queues rather than opaque formulas.
6. **Keyboard-first access:** Complete functional parity across keyboard shortcuts (`⌘K`, `a`, `1`–`5`, `Space`).

---

## 3 · Core design principles

- **One decision per screen:** The interface highlights the single recommended next action.
- **One dark object per screen:** High-contrast solid ink is reserved strictly for the primary action.
- **Retrieval-gated progress:** Advancement across mastery stages requires active recall ratings.
- **No decorative brand colors:** The interface uses neutral surfaces and interaction ink; color is strictly reserved for identity (courses) and state (urgency).
- **Contextual metrics:** Gauges and counters always include explicit denominators.

---

## 4 · Provenance & adopted patterns

- **Atmosphere & Depth (Reference S2):** Cool sheet floating in a neutral room; multi-layered soft drop shadows; solid ink pill states.
- **Course Identity (Reference S1):** 4 locked pastel category washes paired with high-contrast foreground inks.
- **Chassis & Structure (Reference S3):** 20–32px corner radii, glass navigation rail, edge light highlights.
- **Discipline & Metrics (Reference S4):** Tabular numerals on all counters and timers; explicit contrast floors; informative empty states.
- **Component Mechanics:** Duration capsule with in-place editable stepper; tabular clock with fixed character width pins (`6ch`) to prevent reflow.

---

## 5 · Color model

**Core rule:** Color represents surface fill, never thin hairline outlines.

| Category | Token Family | Application | Prohibitions |
| --- | --- | --- | --- |
| **1 · Interaction** | `--ink`, `--ink-inv` | Primary action buttons, active navigation indicator, focus ring, timer pill | Decorative accents, full-screen backgrounds |
| **2 · Identity** | `--w-mint`, `--w-lilac`, `--w-butter`, `--w-sky` + `--fg-*` | Course cards, topic chips, active focus card background, course chart stems | Semantic status indicators, borders, >4 distinct washes |
| **3 · State** | `--chip-overdue`, `--chip-risk`, `--chip-ok`, `--chip-info` + `--on-*` | Status chips (`Overdue`, `Solid`), overdue row fills | Isolated red/green status dots, page backgrounds |
| **4 · Material** | `--glass`, `--glass-blur`, `--room-lit`, edge rungs | Left navigation rail tube and ambient lighting gradient | Content card surfaces (cards remain fully opaque) |

- **One dark object:** Only one solid `--ink` element per view (the primary action). Secondary controls use `--quiet` (`--well-2`) or `--ghost`.
- **Surface hierarchy:** `--backdrop` (room) → `--sheet` (window) → `--card` (content panels) → `--well` / `--well-2` (inputs and recessed rows).
- **Depth ladder:** Four distinct levels (`--sh-1` to `--sh-3`, plus `--sh-pop` and `--sh-ink`). Shadows combine a tight contact shadow and a wide ambient shadow in the room's neutral hue.
- **Dividers:** Dashed rules (`--rule`) separate rows within cards; cards never contain nested cards.

---

## 6 · Composition law

1. **Window as object:** Rounded sheet with subtle depth floating in a room; `[data-window="flush"]` is available for OS-native frame integration.
2. **Soft borderless cards:** 24px radius, soft drop shadow, zero borders.
3. **Surface-driven color:** Category washes fill containers; borders are never used for color coding.
4. **Single dark object:** Exactly one solid ink action element per view.
5. **Fixed navigation column:** Glass rail occupies a constant 88px column; disc labels expand outward without shifting the canvas.
6. **Single loud block:** Focus card is the only oversized highlighted panel on a screen.
7. **Unified radius scale:** Window/Card: 24px · Tile: 18px · Disc: 16px · Row item: 14px · Mini tile: 12px · Mark: 9px · Keycap: 6px · Pill: 999px.
8. **Explicit state coverage:** Designed empty, loading (skeleton), and error states for every surface.
9. **Keyboard parity:** All features accessible via keyboard with visible focus rings.
10. **Restrained motion:** 140–350ms duration using a standard ease-out curve; motion signifies feedback, not decorative transitions.

**The four base primitives:** Every layout is constructed from **Page Head**, **Card**, **Row**, and **Tile**.

---

## 7 · Tokens

Maintained in `tokens.css` (source of truth), mirrored in `tokens.json`, and checked via `tools/tokens.mjs`.

### 7.1 Surfaces & typography tokens

| Token | Light Theme | Dark Theme | Purpose |
| --- | --- | --- | --- |
| `--backdrop` | `oklch(91.0% .005 260)` | `oklch(23.0% .010 260)` | Surrounding desk background |
| `--sheet` | `oklch(96.2% .006 260)` | `oklch(25.0% .010 260)` | Main application canvas |
| `--card` | `oklch(98.6% .002 260)` | `oklch(30.0% .010 260)` | Content cards and containers |
| `--well` | `oklch(93.4% .007 260)` | `oklch(28.0% .010 260)` | Recessed tracks, inputs |
| `--well-2` | `oklch(90.6% .009 260)` | `oklch(32.0% .010 260)` | Quiet buttons, pressed fills |
| `--ink` | `oklch(24.0% .010 260)` | `oklch(93.0% .006 260)` | Primary text and dark action controls |
| `--ink-2` | `oklch(45.0% .014 260)` | `oklch(80.0% .012 260)` | Secondary prose |
| `--ink-3` | `oklch(51.0% .014 260)` | `oklch(72.0% .012 260)` | Metadata and captions (text floor) |
| `--ink-4` | `oklch(66.0% .012 260)` | `oklch(58.0% .010 260)` | Non-text icons, rules, empty art |
| `--rule` | `oklch(91.0% .006 260)` | `oklch(34.0% .010 260)` | Dashed collection dividers |

### 7.2 Identity washes

| Wash | Light Fill / Ink | Dark Fill / Ink | Min Contrast |
| --- | --- | --- | --- |
| **mint** | `#ceebde` / `#09281f` | `#193a2e` / `#c2e8d8` | 11.4:1 / 9.9:1 |
| **lilac** | `#dbcded` / `#241930` | `#392c47` / `#e4d8f4` | 11.1:1 / 9.4:1 |
| **butter** | `#ffe3a5` / `#3b2301` | `#46350d` / `#fee5b3` | 12.2:1 / 10.6:1 |
| **sky** | `#bce1f8` / `#0e212e` | `#1a3647` / `#c9e2f2` | 11.5:1 / 9.9:1 |

Applied via `data-w="mint|lilac|butter|sky"`. White text on pastel fills is strictly prohibited.

### 7.3 Status chips

| Token | Light Fill / Ink | Dark Fill / Ink | Meaning |
| --- | --- | --- | --- |
| `--chip-overdue` | `#ffcfcf` / `#7f0322` | `#572125` / `#ffc0c1` | Past due, late submissions |
| `--chip-risk` | `#ffe5af` / `#663d00` | `#4a3400` / `#f7dba1` | Due within 48h, upcoming exam |
| `--chip-ok` | `#c9efdf` / `#0f4232` | `#143b2e` / `#b2e5d0` | Solid mastery, on pace |
| `--chip-info` | `#e2e7fd` / `#383f67` | `oklch(32% .04 275)` / `oklch(88%)` | Neutral tags (`Recall`, `Proof`) |

### 7.4 Typography

Font: **Plus Jakarta Sans** (variable weight WOFF2 bundled in `fonts.css`).

| Scale Step | Size | Tracking | Application |
| --- | --- | --- | --- |
| `--text-display` | 52px | `-0.030em` | Primary page title (once per screen) |
| `--text-3xl` | 40px | `-0.030em` | Course detail title |
| `--text-2xl` | 32px | `-0.030em` | Numerical metrics, empty-state titles |
| `--text-xl` | 26px | `-0.018em` | Card titles, focus card action |
| `--text-lg` | 20px | `-0.018em` | Section headers, statistics |
| `--text-md` | 17px | `-0.018em` | Row titles, tile headers |
| `--text-base` | 15px | `-0.006em` | Body copy, standard buttons (prose floor) |
| `--text-sm` / `--text-xs` | 13px | `-0.006em` | Captions, chips, quiet buttons |
| `--text-2xs` | 11px | `0.070em` (`caps`) | Micro-caps, weekday labels, kbd badges |

**Rules:** Tabular numerals (`font-variant-numeric: tabular-nums`) are mandatory for countdowns and aligned numbers.

### 7.5 Spacing, geometry, & motion scales

- **Space (4pt grid):** `2px`, `4px`, `8px`, `12px`, `16px`, `20px`, `24px`, `32px`, `48px`, `64px`.
- **Dimensions:** Titlebar: 44px; Rail width: 88px; Rail disc: 44px; Standard pill: 40px; Max canvas width: 1120px.
- **Motion:** `--dur-1` (140ms, state transitions), `--dur-2` (220ms, layout updates), `--dur-3` (350ms, overlay arrivals), `--dur-loop` (1400ms, skeleton loading sweep). Curve: `--ease: cubic-bezier(.16, 1, .3, 1)`.

---

## 8 · Components & layout patterns

### 8.1 Frame & navigation
- **`.cd-window`:** Floating rounded window sheet with subtle depth; uses `[data-window="flush"]` when embedded in native desktop shells.
- **`.cd-titlebar`:** 44px top header with OS control insets (`--titlebar-inset-mac: 78px`, `--titlebar-inset-win: 138px`), the plan-folder door (`app.changePlan`, a sheet over the working plan), the screen-editor pencil (`screen.edit`, present only where a student screen is showing), the search trigger (`⌘K`), the source-pane toggle, Settings, and the session timer.
- **A field on a wash** wears the surface's own material, not the sheet's: the titlebar search pill fills with `--glass` — ink at 5% (7% dark) over `--wash-rose` — because `--well` is a 260-hue neutral and, measured on the running app, read as `oklch(93.4% .007 260)` on the pink bar: cooler *and* darker than the surface it sat on. A field is its surface one step deeper; `--well` is for wells in the sheet's own family.
- **`.cd-nav`:** Glass navigation rail fixed at 88px width. Discs expand labels outward to 70px on hover/focus without shifting adjacent content. Review disc displays pending queue badge.

### 8.2 Work primitives
- **`.cd-card`:** 24px rounded card with icon tile, title, and dashed collection divider rows.
- **`.cd-task`:** 3-column row (mastery indicator, title + metadata, action button). Late rows use `--wash-rose` backgrounds.
- **`.cd-tile`:** Gradient-washed course card showing syllabus fractions (`7/12 topics`).

### 8.3 Daily decision components
- **`.cd-focus`:** Single recommended next task on course wash, with why-line, 180° coverage arc, duration capsule, and `Start` action.
- **`.cd-dur`:** In-place editable duration capsule (5–180 min clamp).
- **`.cd-recall`:** Active recall surface with `Space` reveal and 4 consequence-tagged rating buttons.
- **`.cd-chart`:** 16px daily ink stems plotted against a dashed 90-minute ceiling. Zero days render subtle quiet stems.

---

## 9 · Dark mode physics

Dark mode preserves optical balance rather than performing a simple color inversion:
1. **Shadows:** Ambient shadows shift to pure black with increased opacity (`--sh-3` alpha 0.14 → 0.55).
2. **Typography:** Font weight for body copy drops from 400 to 350 to compensate for light-on-dark optical spread.
3. **Recessed fills:** `--well-2` rises above card lightness to maintain contrast for quiet pills.
4. **Identity chroma:** Course pastels increase chroma slightly to prevent muddy gray appearance at lower lightness.

---

## 10 · Accessibility verification

- Contrast ratios strictly meet WCAG 2.1 AAA/AA standards (e.g. `--ink` on card: 15.8:1 light, 11.1:1 dark; text floor `--ink-3`: 5.5:1).
- Non-text `--ink-4` (2.8:1) is restricted to decorative art, glyph strokes, and subtle rules.
- State is never conveyed by color alone (all chips include text; ladders use structural bars).
- Interactive controls use native `<button>` or `<a>` elements with explicit accessible names.
- Focus indicator: 2px solid ink ring with 2px offset around a control that stands on a surface; **the ring goes inside the edge** (`outline-offset: -2px`) for a field — `input`, `textarea`, `select` — and for any fused control whose container clips (a table row, a sheet well, a scroller): an offset outline is painted outside the border box on the element's own radius, so outside a field it is both clipped by its frame and squared at the corners. `design/base.css` carries both, keyed on the element, and `controls.md` §7 requires the state on every control. One corollary of the same fact: a field whose **box** reaches its container's rounded corner wears that corner's inner radius — the corner less the field's own inset — so its inside ring runs concentric with the clip rather than squared off under it (the palette's head field, `ui/src/variants/palette/C.svelte`).
- `prefers-reduced-motion` collapses transitions to 1ms.

---

## 11 · What this system refuses

Every constraint below is binding:

| Prohibited Pattern | Technical Rationale |
| --- | --- |
| **Solid hairline row separators & table grids** | Replaced by single dashed rules between collection rows; no alternating stripes. |
| **Wide labeled sidebar** | Replaced by 88px compact rail with labels that unfold on hover/focus. |
| **Top toolbar clusters (filter/sort toolbars)** | Controls belong directly on the actionable items; no extraneous sorting bars. |
| **Colored left border accents (3px stripe)** | Outdated visual trope; color must fill surfaces, not outlines. |
| **Bordered status chips** | Chips use solid pastel washes with matching dark text; borders are prohibited. |
| **Saturated red/green/amber status dots** | Ineffective for color-blind users; status must include textual labels. |
| **Gamified streaks, XP, and shaming copy** | Decreases retention and promotes counter optimization over actual learning. |
| **Full donut progress rings** | Closed circles falsely imply completeness before final exams. |
| **White text on pastel backgrounds** | Fails accessibility contrast floors. |
| **Multiple overlapping floating layers** | Limited to one floating rail and one modal console (⌘K). |
| **Simulated OS window controls** | OS handles native window controls; application reserves insets via `[data-os]`. |
| **Chromatic gradient page backgrounds** | Content sits on neutral sheets; gradients are restricted to small identity tiles. |
| **Embossed / claymorphic shadows** | Inset shadows falsely imply pressability on non-interactive surfaces. |

---

## 12 · Desktop platform integration

- Responds natively to OS platform (`[data-os="mac|win|linux"]`), automatically applying control insets and shortcut labels (`⌘K` vs. `Ctrl+K`).
- Responsive rail collapses to a top horizontal navigation strip below 760px viewport width.
- Keyboard navigation provides full operational parity without mouse requirements.

---

## 13 · Verification & test gates

- Visual verification via clean `demo/` implementation across all routes.
- Scripted token integrity audit: `node tools/tokens.mjs` verifies alignment across `tokens.css`, `tokens.json`, and `tailwind.css`.
- Layout stability: Unfolding rail labels preserve exact canvas geometry without reflow.

---

## 14 · Open design items

1. Visual treatment for 5th concurrent course (reuse wash with distinct code chip vs. expanding palette).
2. Syllabus weightage indicator design for recommendation engine.
3. Scaling 14-day activity stem graphs across full 16-week term views.
