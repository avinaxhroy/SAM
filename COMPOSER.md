# COMPOSER — Screen Composition Specification

Technical contract for custom screen composition in SAM: adding, reordering, removing, and styling widgets on user-defined screens.

**Status:** Built and verified (2026-09-30).  
**Cross-references:** `UI_SPEC.md` (§2.2, R3, R4, R19), `UX_FLOWS.md` (F9), `SAM_PLAN.md` (§0 Rule 3), `BUILDLOG.md`.

---

## 0 · The bet sheet

- **Context:** SAM provides 24 designed components (with 3 visual designs each) composed into preset screens via the `panel` field. Composer allows users to customize screens by adding, removing, resizing, and reordering widgets without modifying underlying schemas or engine state.
- **Design principles:**
  - Presets remain defaults, recoverable in one click.
  - Bounded mechanism: closed catalog of 12 widgets, two widths (half/full), no arbitrary nesting or free-canvas placement.
  - Every layout edit is a transactional, validated plan write supporting undo.
  - View mode renders locked component designs; edit mode adds frames, handles, and drop indicators.

| Reference | Mechanics Adopted |
| --- | --- |
| WordPress Gutenberg | Block controls (drag handle, name/switcher, movers), inserter sheet, in-place style switching |
| Home Assistant | Global Edit toggle, per-item menus, explicit "Done" exit, add-card picker |
| iOS Widgets | Edit mode makes content inert; deletions use menu + undo receipt |
| Notion | Drag handle menu for moving/deleting, inline variant switching |

---

## 1 · The ask, in one flow

1. **Empty screen:** Displays title, description, and primary action: *Add component*.
2. **Edit mode:** Toggled via titlebar pencil or empty state button. Shows sticky edit bar: `Editing <screen>` · *Add component* · *Done*. Each widget gains a frame with drag handle, name chip, width toggle, variant arrows, and remove action.
3. **Add:** Inserter sheet displays searchable 12-widget catalog grouped by Today/Study. Appends selection to the screen.
4. **Drag:** Pointer drag or keyboard (`ArrowUp`/`ArrowDown`) reorders items; drop line indicates target slot.
5. **Switch design:** Chevron controls (`‹`/`›`) cycle component variants in place.
6. **Remove:** Frame removal button triggers deletion with undo receipt toast.
7. **Done:** Exits edit mode. Each modification has already persisted via transactional writes.
8. **Preset customization:** Editing a default preset screen materializes a custom composition on first modification. *Use SAM's design again* reverts to preset defaults.
9. **New screen:** Rail Add action (`list.new` without type) initializes an empty screen directly in edit mode.

---

## 2 · Engine contract (`crates/sam-core`)

### 2.1 Schema (`content/views.json`)

`ViewDef` adds an optional `components` array. Fields `type` and `layout` become optional for composed screens:

```json
"morning.screen": {
  "components": [
    { "surface": "today-focus" },
    { "surface": "week-chart", "span": 1 }
  ]
}
```

- `components: Option<Vec<ComponentDef>>`
- `ComponentDef { surface: String, span: Option<u8> }`:
  - `surface`: UI component identifier (open string).
  - `span`: Column width in two-column grid. `1` = half-width, `2` = full-width (default `2` is omitted from serialized JSON).
- **Validation rules:**
  - Neither `type` nor `components` present: error `view.empty`.
  - `type` present without `layout`: error `view.layout-missing`.
  - `layout` present without `type`: error `view.type-missing`.
  - Component missing `surface`: error `component.surface-missing`.
  - Component `span < 1`: error `component.span-invalid`.
  - Component `span > 2`: warning `component.span-wide` (rendered full width).
  - Duplicate surfaces on a screen: warning `view.duplicate-component`.
- **Render precedence:** `components` (including empty `[]`) → `panel` → `blocks`/records.

### 2.2 Commands

| Command | Parameters | Behavior |
| --- | --- | --- |
| `view.setComponents` | `name`, `surfaces?` (list), `spans?` (list of `1\|2`), `clear?` (bool) | Transactionally sets components. `spans` must match `surfaces` length with values `1` or `2` (`2` omits the field). `clear: true` deletes `components`, falling back to panel/blocks. Returns `{ view, surfaces, spans, cleared }`. |
| `list.new` | `name`, `type?`, `id?`, `layout?`, `icon?` | Without `type`: creates `{ "components": [] }` and adds rail entry with icon in one transaction. Returns `{ "view": <id> }`. |
| `view.delete` / `list.set` / `list.delete` | Standard parameters | Standard CRUD. `view.delete` enforces type-less view safety checks. |

Reads: `views` bootstrap serializes full `ViewDef` including `components`. `view <name>` returns `components: [{ surface, span? }]` with optional/null `type`.

### 2.3 Evidence

- Cargo workspace builds and tests pass.
- `sam --selfcheck` exercises `list.new` without type, `view.setComponents`, `--clear`, and round-trip reloads.

---

## 3 · UI contract (`ui/src`)

### 3.1 New layer: `ui/src/composer/`

```
composer/
  defaults.ts                 PANEL_DEFAULTS mapping panel names to default surfaces
  shared.svelte.ts            Cross-widget shared state (session duration, pinned review)
  widgets/registry.ts         WIDGETS catalog and hostFor(surface) mapping
  widgets/<surface>/Host.svelte   Self-sufficient host per widget (12 total)
  Screen.svelte               Composed screen container (view + edit mode, empty state, edit bar)
  Frame.svelte                Component edit chrome (handle, name chip, width toggle, delete)
  VariantMenu.svelte          Design switcher popover
  Inserter.svelte             Add-component modal sheet
  reorder.svelte.ts           Pointer drag and drop state engine
```

**Host contract:** `Host.svelte` takes no props. Derives all data from `app` store and local reads. Mounts `<Variant surface="<surface>" {...props} />` matching existing panel prop contracts. Empty widget surfaces render an explicit empty state card.

**Widget catalog (12 widgets):**

| Group | Surface ID | Host Label |
| --- | --- | --- |
| day | `today-focus` | Session card |
| day | `today-queue` | Today's queue |
| day | `coming-up` | Coming up |
| day | `week-chart` | Week chart |
| study | `plan-spine` | Plan spine |
| study | `courses` | Courses |
| study | `practice-banks` | Practice banks |
| study | `mocks-calendar` | Mocks calendar |
| study | `reviews-recall` | Recall card |
| study | `reviews-queue` | Review queue |
| study | `progress-stats` | Progress figures |
| study | `reference-library` | Library |

`PANEL_DEFAULTS`:
- `today`: `today-focus`, `today-queue`, `coming-up`, `week-chart`
- `plan`: `plan-spine`
- `subjects`: `courses`
- `practice`: `practice-banks`
- `mocks`: `mocks-calendar`
- `library` / `notes`: `reference-library`
- `reviews`: `reviews-recall`, `reviews-queue`
- `progress`: `progress-stats`, `week-chart`

### 3.2 Session store (`session.svelte.ts`)

```typescript
screenEditing: boolean;
screenSurfaces: string[];
screenIsCustom: boolean;
screenIsEmpty: boolean;
screenHasDefault: boolean;
beginScreenEdit(): void;
endScreenEdit(): void;
setScreenSurfaces(surfaces: string[]): Promise<void>;
resetScreen(): Promise<void>;
```

- Entering edit mode does not trigger writes.
- First structural change persists full component array.
- `screenEditing` state survives view refreshes.
- Unknown surfaces render fallback placeholder cards.

### 3.3 Mount points

- **`App.svelte`:** Renders `<Screen />` when `outcome.components !== undefined` or `app.screenEditing` is active; falls back to standard panel routing.
- **Titlebar:** Pencil icon dispatches `screen.edit` (`aria-pressed`). Hidden on Settings and System views.
- **Rail Add:** Sheet adds third mode: *A screen* (`list.new` without type).
- **Sheets:** `component.insert` mounts `<Inserter />`.

---

## 4 · Flows in full

### 4.1 View mode
Renders widget stack full-width at `--canvas-max` (1120px). Empty composed screens show R19 empty state: *Nothing on this screen yet* (`--text-2xl`) with *Add component* button.

### 4.2 Entering and leaving edit mode
- Pencil button dispatches `screen.edit` → `beginScreenEdit()`.
- Sticky edit bar mounts at canvas top: `Editing <screen>` label, *Add component*, and *Done* button. Uncustomized preset screens show prompt noting first edit customizes the screen; customized screens show *Use SAM's design again*.
- `Escape` or *Done* exits without saving if unmodified.

### 4.3 Component frame (edit mode)
- Outlined with 1px dashed `--line`, radius `--r-card`, `--s-xs` padding.
- Frame header (left to right): drag handle (⠿), component name label, width toggle (`Half width` / `Full width`), and remove button (✕).
- Design switcher chevrons (`‹`/`›`) flank the component body on both sides, inside the frame and centered on it.
- Frame content is `inert` during edit mode.
- Keyboard: frame is focusable (`tabindex="0"`); `ArrowUp`/`ArrowDown` reorders slots with live region announcements (*"Week chart, position 2 of 4"*). `ArrowLeft`/`ArrowRight` cycles variants.

### 4.4 Pointer drag and drop
- Entire frame is draggable after 4px pointer movement with pointer capture. Controls within frame retain normal click handlers.
- Dragged frame lifts to z-index `70` (`--sh-pop`, scale 1.01, `cursor: grabbing`, `will-change: transform`).
- 2px `--ink` drop line indicates target slot based on pointer x/y coordinates.
- Drop triggers `setScreenSurfaces` (`view.setComponents`), followed by smooth settle animation over `--dur-1` (instant with `prefers-reduced-motion`). `Escape` cancels drag.
- Implemented with Pointer Events and `setPointerCapture` (no HTML5 Drag & Drop).

### 4.5 Design switcher
- Flanking 32px chevron buttons (`chevronleft`/`chevronright`) vertically centered outside the component body.
- **The switcher lives in the frame, so it exists only in edit mode**: view mode draws no design chrome at all — a screen in use is the student's, not the designer's. The second home for the same choice is *Settings › Component styles*, which lists every surface and its three designs with *Reset all*.
- Swapping calls `styles.setVariant(surface, id)`, updating the stored preference (`cadence.component-styles`, beside the theme) immediately without altering plan data or undo stack.

### 4.6 Inserter sheet
- Search input (auto-focused) with widget gallery grouped into Today and Study.
- Wireframe icon preview, widget title, description, and *Add* button per card. Widgets already present are disabled (*On this screen*).
- Adding appends widget to bottom of screen and opens edit mode if not already active.

### 4.7 Screen management
- **Create:** Rail Add sheet → *A screen* (name + icon) → dispatches `list.new`, opens new screen in edit mode with inserter active.
- **Manage:** Edit bar menu provides *Rename screen* (`list.set --title`), *Change icon* (`list.set --icon`), and *Remove screen* (`list.delete`/`view.delete`) with confirmation.

---

## 5 · Visual & interaction floors

- **Tokens only:** Follows `design/tokens.css` and `UI_SPEC.md` §2 scales. Edit bar: z-index 20; menus: 30; sheets: 50; toast: 60; drag ghost: 70.
- **Component integrity:** Variants remain unchanged; edit chrome wraps external boundaries.
- **One dark object:** Edit frames, chips, and drop indicators use subtle lines/neutral chrome rather than heavy dark fills.
- **Accessibility:** Touch targets ≥ 32px; visible focus rings (`design/base.css`); full keyboard equivalents for drag and variant switching; live region announcements for mode and slot updates.
- **Census & parity:** Zero raw keys or schema tokens displayed (`tools/uicensus.mjs`); all commands have rendered UI controls (`tools/uicheck-diff.mjs`).

---

## 6 · What this does not do

- **No arbitrary grid canvas or nesting:** Uses a 2-column auto-dense grid (`grid-auto-flow: row dense`). Widgets occupy half (span 1) or full (span 2) width.
- **No per-instance widget configurations:** A widget surface appears at most once per screen.
- **No new component surfaces:** Restricted to the 12 catalog widgets derived from the 24 base designs.
- **No freeform CLI layout:** Layout state is restricted to ordered surface IDs and spans in `view.setComponents`.

---

## 7 · Verification plan

| # | Invariant | Verification Gate |
| --- | --- | --- |
| 1 | Fresh plan screens render identically to presets | Visual captures + `tools/gates.mjs` |
| 2 | Component add, drag, and remove round-trip to disk | Driven browser test + `SAM view` JSON inspection |
| 3 | Initial edit on preset materializes custom view | Plan revision unchanged on enter/exit; increments on edit |
| 4 | *Use SAM's design again* restores preset view | Visual capture + plan inspection |
| 5 | Design switcher persists across reloads | `localStorage` inspection + reload verification |
| 6 | Keyboard reorder works with screen reader announcements | A11y audit + keyboard navigation test |
| 7 | Screen creation via rail Add opens edit mode | Driven browser session |
| 8 | Undo toast reverts layout modifications | Driven browser session |
| 9 | Gates suite passes cleanly | `node tools/gates.mjs` (tokenlint, census, a11y, parity) |
| 10 | Rust engine tests pass | `cargo test --workspace` |

---

## 8 · Sacred list

- Existing command registry signatures and parameters remain strictly backward-compatible.
- `rail.select` and navigation behaviors preserved.
- Toast and undo transaction lifecycles preserved.
- Sheet mount and click-outside dismissal timing preserved.
- Component variant contracts and props signatures in `variants/**` preserved.
- Feature rollback safety: deleting `components` restores default panel behavior without migrations.
