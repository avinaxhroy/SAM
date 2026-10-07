<!-- RECORD TABLE · C · The Morph. Tablist of records above an active record sheet. -->
<script lang="ts">
  import FieldControl from '../../records/FieldControl.svelte';
  import Menu from '../../shell/Menu.svelte';
  import Icon from '../../shell/Icon.svelte';
  import ColumnStrip from './ColumnStrip.svelte';
  import type { RecordTableProps, RowFact } from './props';

  let {
    columns,
    hiddenCount,
    rows,
    selected,
    plusOpen,
    menuFor,
    moveFor,
    dragging,
    dropTarget,
    plusRows,
    rowRows,
    moveRows,
    onSelect,
    onCommit,
    onPanel,
    onMenu,
    onMove,
    onPlus,
    onDrag,
    onOver,
    onDrop,
  }: RecordTableProps = $props();

  /** The record the sheet is showing: the cursor's, else the read's own first. */
  const drawn = $derived(rows.find((row) => row.id === selected) ?? rows[0] ?? null);

  /**
   * The cell that holds an editor, and the width it is to go back to: a cell is
   * exactly as wide as its reading, and a value that did not change must return
   * to exactly the width it left.
   */
  let editor = $state<{ key: string; rest: number } | null>(null);
  /** The letters that are leaving the cell the editor opened in. */
  let ghost = $state<{ key: string; text: string; step: number; delay: number } | null>(null);

  /** The cells' own boxes, so a morph can pin the width it is leaving. */
  const boxes = new Map<string, HTMLElement>();
  function cellBox(node: HTMLElement, key: string) {
    boxes.set(key, node);
    return {
      update(next: string) {
        if (next !== key) {
          boxes.delete(key);
          boxes.set(next, node);
        }
      },
      destroy() {
        boxes.delete(key);
      },
    };
  }

  const reduce = $derived(
    typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true,
  );
  /** The one width a cell widens to: a declaration, not a measurement. */
  const EDIT = 'var(--vc-edit)';

  /**
   * The readings whose editor opens *inside* the cell — the only ones a capsule
   * can morph. A date, a duration, a picker or a switch opens the app's own
   * control instead, and the design does not pretend otherwise.
   */
  const INLINE = ['text', 'longtext', 'number', 'url'];

  /**
   * The editor opened inside this cell: pin the width the box is leaving, then
   * release the editor's declared width so the morph has two numbers to travel
   * between — and start the value's own letters leaving.
   */
  function opened(event: FocusEvent, key: string, text: string): void {
    const target = event.target as HTMLElement | null;
    // A popover (a date, a duration, a relation picker) is not a morph: the
    // capsule belongs to an editor that lives *inside* the cell, and the design
    // stays out of the way of the app's own control.
    if (!target || target.tagName !== 'INPUT') return;
    const box = boxes.get(key);
    if (!box) return;
    const rest = box.getBoundingClientRect().width;
    editor = { key, rest };
    if (!reduce) {
      const room = box.parentElement ? box.parentElement.clientWidth : rest;
      box.style.width = `${rest}px`;
      void box.offsetWidth;
      box.style.width = room > 0 ? `min(${EDIT}, ${room}px)` : EDIT;
    }
    const letters = text.length;
    if (reduce || letters === 0) {
      ghost = null;
      return;
    }
    // One envelope for the whole dissolve: 9ms a letter while the value is
    // short, closer together as it grows.
    const step = letters > 1 ? Math.min(9, 140 / (letters - 1)) : 0;
    ghost = { key, text, step, delay: Math.max(0, 140 + step * (letters - 1) - 70) };
  }

  /** The editor closed: the cell travels back to the width it left, and drops
   *  the inline width once it has landed, so it is content-sized again. */
  function closed(event: FocusEvent, key: string, box: HTMLElement | undefined): void {
    const next = event.relatedTarget as Node | null;
    if (next && box?.contains(next)) return;
    if (editor?.key !== key) return;
    const rest = editor.rest;
    editor = null;
    ghost = null;
    if (!box || reduce) return;
    box.style.width = `${rest}px`;
    window.setTimeout(() => {
      if (editor?.key !== key) box.style.removeProperty('width');
    }, 240);
  }

  /** The editor inside this cell, if it is one. */
  function editorOf(box: HTMLElement | undefined): HTMLInputElement | null {
    const found = box?.querySelector('input');
    return found instanceof HTMLInputElement ? found : null;
  }

  /** The cancel disc: the Escape the app's own editor already answers. */
  function cancel(key: string): void {
    editorOf(boxes.get(key))?.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
    );
  }

  /** The commit disc: the blur the app's own editor already commits on. */
  function commit(key: string): void {
    editorOf(boxes.get(key))?.blur();
  }

  /** ↑↓←→ walk the run; ↓ hands the keyboard to the sheet. */
  function tileKey(event: KeyboardEvent, at: number): void {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (event.key === 'ArrowDown') {
      const first = document.getElementById(SHEET)?.querySelector<HTMLElement>('input, button.cd-cell');
      if (first) {
        event.preventDefault();
        first.focus();
      }
      return;
    }
    const to =
      step !== 0
        ? Math.max(0, Math.min(rows.length - 1, at + step))
        : event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? rows.length - 1
            : -1;
    if (to < 0 || rows[to] === undefined) return;
    event.preventDefault();
    onSelect(rows[to].id);
    const tile = document.getElementById(`${SHEET}-tab-${rows[to].id}`);
    tile?.focus();
  }

  /**
   * A per-instance prefix for the sheet's id and its tabs'. Two record tables
   * can stand on one screen — a view may hold two table blocks — and a fixed id
   * would then name two elements at once, breaking `aria-controls` for both.
   */
  const uid = $props.id();
  const SHEET = `${uid}-sheet`;
  const tileId = (row: RowFact) => `${SHEET}-tab-${row.id}`;
</script>

<div class="v-fit vc">
  {#if rows.length > 0}
    <div
      class="vc-index"
      role="tablist"
      aria-orientation="horizontal"
      aria-label="the records in this view"
    >
      {#each rows as row, at (row.id)}
        <button
          class="vc-tile"
          type="button"
          role="tab"
          id={tileId(row)}
          aria-controls={SHEET}
          aria-selected={selected === row.id}
          aria-label={row.sentence}
          tabindex={selected === row.id || (selected === null && at === 0) ? 0 : -1}
          data-w={row.mark?.wash ?? row.wash}
          data-record-id={row.id}
          data-placement="recordTable.cell"
          data-late={row.instant?.late ? '1' : '0'}
          onclick={() => onSelect(row.id)}
          onkeydown={(event) => tileKey(event, at)}
        >
          <span class="vc-tile__t">{row.label}</span>
          {#if row.instant?.words}
            <span class="vc-tile__d num">{row.instant.words}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if columns.length > 0}
    <ColumnStrip
      {columns}
      hidden={hiddenCount}
      {plusRows}
      {plusOpen}
      {dragging}
      {dropTarget}
      tone="c"
      {onPlus}
      {onOver}
      {onDrop}
    />
  {/if}

  {#if drawn}
    <div
      class="vc-sheet"
      id={SHEET}
      role="tabpanel"
      aria-labelledby={tileId(drawn)}
      tabindex="-1"
    >
      {#each drawn.cells as cell (cell.key)}
        <div class="vc-f" data-field-type={cell.field?.type ?? 'unknown'}>
          <span class="vc-fk">{cell.label}</span>
          <div class="vc-slot">
            <div
              class="vc-cell"
              class:v-open={editor?.key === cell.key}
              data-edit={INLINE.includes(cell.type) ? '1' : undefined}
              style:--vc-step={ghost?.key === cell.key ? `${ghost.step}ms` : null}
              style:--vc-in={ghost?.key === cell.key ? `${ghost.delay}ms` : null}
              use:cellBox={cell.key}
              onfocusin={(event) => opened(event, cell.key, cell.text)}
              onfocusout={(event) =>
                closed(event, cell.key, (event.currentTarget as HTMLElement) ?? undefined)}
            >
              {#if editor?.key === cell.key}
                <button
                  class="vc-disc"
                  type="button"
                  aria-label={`Cancel ${cell.label}`}
                  onmousedown={(event) => event.preventDefault()}
                  onclick={() => cancel(cell.key)}
                >
                  <Icon name="close" size={14} />
                </button>
              {/if}

              {#if cell.field}
                <FieldControl
                  field={cell.field}
                  record={drawn.record}
                  targets={cell.targets}
                  oncommit={(key, value) => onCommit(drawn, key, value)}
                />
              {:else}
                <span class="vc-unknown" title="the view names a column the type does not declare">?</span>
              {/if}

              {#if editor?.key === cell.key}
                <button
                  class="vc-disc vc-disc--go"
                  type="button"
                  aria-label={`Save ${cell.label}`}
                  onmousedown={(event) => event.preventDefault()}
                  onclick={() => commit(cell.key)}
                >
                  <Icon name="check" size={14} />
                </button>
              {/if}

              {#if ghost?.key === cell.key}
                <!-- The letters that are leaving, over the editor that is
                     already focused underneath. They belong to a real record,
                     never to a placeholder. -->
                <span class="vc-ghost" aria-hidden="true">
                  {#each ghost.text.split('') as letter, index (index)}
                    <span class="vc-l" style={`--i: ${index}`}>{letter === ' ' ? '\u00A0' : letter}</span>
                  {/each}
                </span>
              {/if}

              {#if cell.write}
                <span
                  class="cd-chip vc-state"
                  class:cd-chip--ok={cell.write.state === 'saved'}
                  class:cd-chip--risk={cell.write.state === 'failed'}
                  role="status"
                >
                  {cell.write.state === 'saving' ? 'Saving…' : cell.write.state === 'saved' ? 'Saved' : 'Not saved'}
                </span>
              {/if}
            </div>
          </div>
        </div>
      {/each}

      <div class="vc-doors">
        <button
          class="cd-iconbtn"
          type="button"
          data-command="record.panel"
          data-placement="recordTable.row"
          aria-label={`Details for ${drawn.label}`}
          onclick={() => onPanel(drawn)}
        >
          <Icon name="textQuote" size={14} />
        </button>
        <button
          class="cd-iconbtn"
          type="button"
          data-rowmenu-trigger
          aria-haspopup="menu"
          aria-expanded={menuFor === drawn.id}
          aria-label={`Actions for ${drawn.label}`}
          onclick={(event) => {
            // The menu mounts *during* this click, and `Menu`'s own window
            // handler dismisses on any click outside `.cd-menu` — including this
            // one, which would close the menu in the same frame it opened.
            event.stopPropagation();
            onMenu(menuFor === drawn.id ? null : drawn.id);
          }}
        >
          <span class="vc-dots" aria-hidden="true">⋯</span>
        </button>
      </div>

      {#if menuFor === drawn.id}
        <div class="vc-menu">
          <Menu rows={rowRows(drawn)} onclose={() => onMenu(null)} />
        </div>
      {/if}
      {#if moveFor === drawn.id}
        <div class="vc-menu">
          <Menu
            rows={moveRows(drawn)}
            native={false}
            onclose={() => {
              onMove(null);
              onMenu(null);
            }}
          />
        </div>
      {/if}
    </div>

    {#if drawn.failures.length > 0}
      {#each drawn.failures as reason, position (position)}
        <p class="vc-fail" role="alert">{reason}</p>
      {/each}
    {/if}
  {/if}
</div>

<style>
  /* ── THE RUN ──────────────────────────────────────────────────────────────
     176px is the column floor, so a fortnight reads five across and three down.
     Two facts a tile and no mark: lateness is the due line's own ink, and
     everything else the run could say is one gesture away in the sheet. */
  .vc-index {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(176px * var(--ui-s)), 1fr));
    gap: var(--ui-gap-sm) var(--ui-gap-lg);
    margin-bottom: var(--ui-gap-lg);
  }
  .vc-tile {
    display: grid;
    align-content: center;
    gap: var(--space-3xs);
    width: 100%;
    min-height: calc(52px * var(--ui-s));
    padding: var(--ui-gap-sm) var(--ui-pad-sm);
    border-radius: var(--r-item);
    text-align: left;
    transition:
      background-color var(--dur-1) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .vc-tile:hover {
    background-color: var(--well);
  }
  .vc-tile:active {
    background-color: var(--well-2);
  }
  /* The tile the sheet is showing, in that record's own course wash — the
     identity job, spent once, and the only thing on the run that moves. */
  .vc-tile[aria-selected='true'] {
    background-color: var(--wash, var(--well-2));
    background-image: var(--wash-grad, none);
    color: var(--onwash, var(--ink));
    box-shadow: var(--sh-1);
  }
  .vc-tile__t {
    font-size: var(--ui-text-sm);
    color: var(--ink-2);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vc-tile[aria-selected='true'] .vc-tile__t {
    color: inherit;
    font-weight: var(--weight-label);
  }
  .vc-tile__d {
    font-size: var(--ui-meta);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
    white-space: nowrap;
  }
  .vc-tile[data-late='1'] .vc-tile__d {
    color: var(--ink);
  }
  .vc-tile[aria-selected='true'] .vc-tile__d {
    color: inherit;
  }

  /* ── THE SHEET ────────────────────────────────────────────────────────────
     One labelled line per reading, in the view's own order. */
  .vc-sheet {
    --vc-edit: calc(320px * var(--ui-s));
    /* the editor's own inset: the padding, a disc, and the gap */
    --vc-inset: calc(var(--hit) + var(--ui-gap-sm));
    position: relative;
    padding: var(--ui-gap-sm) var(--ui-pad);
  }
  .vc-f {
    display: grid;
    grid-template-columns: calc(128px * var(--ui-s)) minmax(0, 1fr);
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(var(--hit) + 8px * var(--ui-s));
  }
  .vc-f + .vc-f {
    border-top: 1px solid var(--rule);
  }
  .vc-fk {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  /* THE SLOT is the row's whole writable width; the cell inside it is only as
     wide as its own reading — so the pill a hover paints is the value's pill,
     and the pill a morph widens is the value's pill, not a full-width band. */
  .vc-slot {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    height: calc(var(--hit) + 8px * var(--ui-s));
  }
  /* THE CELL: the system's 32px floor, 40px at rest and open, so the editor
     never moves the row it stands in. */
  .vc-cell {
    position: relative;
    flex: 0 1 auto;
    min-width: var(--hit);
    max-width: 100%;
    height: calc(var(--hit) + 8px * var(--ui-s));
    padding: 0 var(--ui-gap-sm);
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    border-radius: var(--r-pill);
    font-size: var(--ui-text);
    color: var(--ink);
    transition:
      width var(--dur-2) var(--ease),
      padding var(--dur-2) var(--ease),
      background-color var(--dur-1) var(--ease);
  }
  .vc-cell[data-edit='1'] {
    cursor: text;
  }
  .vc-cell[data-edit='1']:hover {
    background-color: var(--well);
  }
  /* The cell only hugs its reading: `width: 100%` on the app's own control
     resolves against a content-sized box, which is what keeps a hover pill the
     value's own width and gives the morph a real width to leave. */
  .vc-cell :global(button.cd-cell) {
    width: auto;
    max-width: 100%;
    height: var(--hit);
    box-sizing: border-box;
  }
  /* The url cell's own link is the control; it hugs its value as the cell does,
     but never below the system's 32px floor — an empty link keeps a real
     target (FieldControl is not ours to edit). */
  .vc-cell :global(.cd-url) {
    display: flex;
    min-width: 0;
  }
  .vc-cell :global(.cd-url__text) {
    min-width: calc(var(--hit) + var(--space-2xs));
  }
  /* OPEN — the editor is the cell's own box. The padding drops to 4px so the
     two discs stand at the capsule's ends and the text owns the middle; the
     ring is inset, because the cell is now one group of three and an outer ring
     would make the discs look detached from it. */
  .vc-cell.v-open {
    background-color: var(--well);
    box-shadow: inset 0 0 0 1.5px var(--ink);
    padding: 0 var(--space-2xs);
  }
  .vc-cell.v-open :global(.cd-cellinput) {
    flex: 1 1 auto;
    width: auto;
    min-width: 0;
    height: var(--hit);
    box-sizing: border-box;
  }
  .vc-unknown {
    color: var(--ink-4);
  }
  /* THE DISCS — the system's 32px floor, arriving a beat after the letters are
     clear so nothing crosses the buttons taking their places. */
  .vc-disc {
    flex: none;
    width: var(--hit);
    height: var(--hit);
    border-radius: var(--r-pill);
    display: grid;
    place-items: center;
    background: var(--card);
    color: var(--ink-2);
    box-shadow: var(--sh-1);
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .vc-cell.v-open .vc-disc {
    animation: vc-disc-in var(--dur-1) var(--ease) var(--dur-1) backwards;
  }
  .vc-disc:hover {
    color: var(--ink);
    box-shadow: var(--sh-2);
  }
  .vc-disc:active {
    transform: scale(0.94);
  }
  .vc-disc--go {
    color: var(--ink);
  }

  /* THE LETTERS. The value dissolves where the editor lands: the reference's
     per-letter choreography, played on the text that is leaving instead of on a
     placeholder arriving. The strip is transparent and nowrap, so a long value
     is clipped rather than wrapped mid-dissolve, and its inset travels with the
     capsule so no letter jumps sideways. */
  @keyframes vc-lyr {
    to {
      opacity: 0;
      transform: translateY(-4px);
      filter: blur(2px);
    }
  }
  @keyframes vc-ghost-in {
    from {
      padding: 0 var(--ui-gap-sm);
    }
  }
  @keyframes vc-in {
    from {
      opacity: 0;
    }
  }
  @keyframes vc-disc-in {
    from {
      opacity: 0;
      transform: scale(0.8);
    }
  }
  .vc-ghost {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    padding: 0 var(--vc-inset);
    border-radius: var(--r-pill);
    color: var(--ink);
    font-size: var(--ui-text);
    white-space: nowrap;
    pointer-events: none;
    overflow: hidden;
    animation: vc-ghost-in var(--dur-2) var(--ease);
  }
  .vc-l {
    display: inline-block;
    animation: vc-lyr var(--dur-1) var(--ease) both;
    animation-delay: calc(var(--i, 0) * var(--vc-step, 9ms));
  }
  /* The editor's own value waits under the letters and is released a little
     before the last one is gone. */
  .vc-cell.v-open :global(.cd-cellinput) {
    animation: vc-in var(--dur-1) var(--ease) both;
    animation-delay: var(--vc-in, 0ms);
  }

  /* THE SHEET'S OWN DOORS: the record's detail panel and its context menu, on
     the sheet the record belongs to. Quiet, at the sheet's own end. */
  .vc-doors {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3xs);
    margin-top: var(--ui-gap-sm);
  }
  .vc-dots {
    font-size: var(--text-base);
    line-height: 1;
  }
  .vc-menu {
    position: absolute;
    inset-inline-end: var(--ui-pad);
    z-index: 30;
  }
  .vc-state {
    position: absolute;
    inset-block: 0;
    inset-inline-end: var(--space-2xs);
    margin: auto 0;
    pointer-events: none;
  }
  .vc-fail {
    margin: var(--ui-gap-sm) 0 0;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--r-mini);
    background: var(--chip-risk);
    color: var(--on-risk);
    font-size: var(--ui-meta);
    overflow-wrap: anywhere;
  }

  @media (prefers-reduced-motion: reduce) {
    .vc-tile,
    .vc-cell,
    .vc-disc {
      transition: none;
    }
    /* the cell expands to editor width immediately without animated transitions */
    .vc-cell.v-open {
      width: min(var(--vc-edit), 100%);
    }
    .vc-ghost,
    .vc-l,
    .vc-cell.v-open .vc-disc,
    .vc-cell.v-open :global(.cd-cellinput) {
      animation: none;
    }
  }
</style>
