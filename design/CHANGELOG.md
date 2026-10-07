# Cadence — Design System Changelog

Chronological log of changes and structural fixes in Cadence design system.

---

### 4.10 (2026-10) — Native Window Frame Alignment
- **Flush window restored:** Enabled `.cd-window[data-window="flush"]` (`border-radius: 0`, full-bleed titlebar) when running in OS window frames (`UI_SPEC.md` R1 / S1).
- **Metric alignment:** Eliminated double-nested container borders; window body expanded to full native height (`winScroll = docScroll = 0`).

---

### 4.9 (2026-10) — Rail Geometry & Viewport Budgeting
- **Vertical rail containment:** Pinned `.cd-body` grid row to window height (`minmax(0, 1fr)`), preventing rail height overflow from stretching content canvas.
- **Rail spacing budget:** Reduced padding and margins (`sm` / `xs`) to fit full 685px nav column within 780px default window height.
- **Gear icon fix:** Regenerated 8-tooth cog SVG path geometry to prevent rendering artifacts at small resolutions.

---

### 4.8 (2026-10) — Neutral Backdrop & Header Accent Band
- **Backdrop neutralization:** Updated `--backdrop` to neutral 260 hue (`oklch(91.0% 0.005 260)` light, `oklch(23.0% 0.010 260)` dark), removing unintended warm tint from desk surface.
- **Titlebar warmth:** Applied `--wash-rose` accent directly to `.cd-titlebar` with inset light-catch edge.
- **Spacing rhythm harmonization:** Replaced non-standard gaps with standard `--s-xl` (24px) scale tokens.

---

### 4.7 (2026-09) — Token Multi-file Synchronization
- **Token consistency script:** Added `tools/tokens.mjs` to enforce strict parity across `tokens.css`, `tokens.json`, and `tailwind.css`.
- **Shadow scale unification:** Updated dark theme shadows in `tokens.json` to two-part contact/ambient definitions matching CSS.
- **CSS modularization:** Separated window frame assumptions into `base.css`, keeping `tokens.css` strictly scoped to custom property declarations.

---

### 4.6 (2026-09) — Timer Width Pinning & Theme Persistence
- **Timer text reflow guard:** Pinned `.cd-timer__time` to `min-width: 6ch` with `tabular-nums` to prevent layout shifts as minute counts grow.
- **Theme boot resolution:** Inline script in `<head>` resolves stored theme from `localStorage` before initial paint.

---

### 4.5 (2026-09) — Glass Material & Non-Reflowing Rail
- **Fixed rail width:** Replaced `width: max-content` with static 88px column (`--rail-w`); disc labels unfold outward without reflowing adjacent canvas.
- **Theme-agnostic glass recipe:** Formulated `--glass` from 5% veil of `--ink`, rendering translucent tint automatically in both light and dark modes.

---

### 4.4 (2026-09) — Unfolding Rail Labels
- **Contextual disc expansion:** Individual nav discs unfold labels to fixed `--rail-label` (70px) on hover/focus while rail tube remains static.
- **Badge anchoring:** Anchored count badge directly to inner glyph box.

---

### 4.3 (2026-09) — Light Stack Luminance Tuning
- **Contrast ceiling relief:** Reduced `--card` from pure white to `oklch(98.6% .002 260)` and eased `--ink`, reducing extreme contrast glare while retaining AAA/AA compliance.
- **Metadata contrast floor:** Darkened `--ink-3` to guarantee ≥ 5.15:1 contrast against sheet and card backgrounds.

---

### 3.1 (2026-09) — Scale Audit & Literal Elimination
- **Token enforcement:** Replaced 93 arbitrary px and weight literals with system tokens (`tokens.css`).
- **Tabular figures:** Standardized tabular numerals (`.num`) across all counters, timers, and data columns.
- **Structural skeleton contract:** Skeletons borrow exact layout dimensions of target components (e.g. 76px task row) to prevent post-load layout shifts.
