<!-- RECORD TABLE · A · The Rail. Slide-out tool rail for quick date adjustments. -->
<script lang="ts">
  import FieldControl from '../../records/FieldControl.svelte';
  import Menu from '../../shell/Menu.svelte';
  import Icon from '../../shell/Icon.svelte';
  import ColumnStrip from './ColumnStrip.svelte';
  import './table.css';
  import type { RecordTableProps, RowFact } from './props';

  let {
    columns,
    flexKey,
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
    instant,
    onSelect,
    onCommit,
    onPanel,
    onMenu,
    onMove,
    onPlus,
    onDrag,
    onOver,
    onDrop,
    onInstant,
  }: RecordTableProps = $props();

  // Active tool rail panel for the currently open row.
  let panelFor = $state<string | null>(null);

  function rowKey(event: KeyboardEvent, row: RowFact): void {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onSelect(row.id);
    } else if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      onSelect(row.id);
      onMenu(row.id);
    }
  }
</script>

<div class="v-fit va">
  {#if columns.length > 0}
    <ColumnStrip
      {columns}
      {flexKey}
      hidden={hiddenCount}
      {plusRows}
      {plusOpen}
      {dragging}
      {dropTarget}
      tone="a"
      {onPlus}
      {onOver}
      {onDrop}
    />
  {/if}

  {#if rows.length > 0}
    <!-- Accessible row items with field controls and interactive tool rail. -->
    <ul class="va-rows">
      {#each rows as row (row.id)}
        {@const open = selected === row.id}
        {@const panel = panelFor === row.id ? 2 : 1}
        <li
          class="cd-row va-row"
          role="row"
          tabindex="0"
          aria-selected={open}
          aria-label={row.sentence}
          data-record-id={row.id}
          data-placement="recordTable.cell"
          data-cur={open ? '1' : '0'}
          data-tools={open ? '1' : '0'}
          data-panel={panel}
          data-late={row.instant?.late ? '1' : '0'}
          onclick={() => onSelect(row.id)}
          onkeydown={(event) => {
            // Only when the row *itself* has focus: Enter on one of its own
            // controls is that control's own key (a cell's editor, the rail's
            // clock), and swallowing it here would break the very keyboard the
            // row's tab stop exists for.
            if (event.target !== event.currentTarget) return;
            rowKey(event, row);
          }}
          oncontextmenu={(event) => {
            event.preventDefault();
            onSelect(row.id);
            onMenu(row.id);
          }}
        >
          {#each row.cells as cell, at (cell.key)}
            {#if at === 0 && columns.length > 1}
              <!-- THE BAY: zero wide until its row's rail rises, then exactly the
                   rail's own width plus its air — taken out of the flexible
                   column, so every fact keeps its x whether a rail is open or
                   not. -->
              <div class="va-bay">
                <div class="va-rail" inert={!open}>
                  <div class="va-track">
                    <div class="va-panel va-panel--1">
                      <button
                        class="va-tool"
                        type="button"
                        disabled={!instant}
                        aria-label={instant ? `Push ${instant.label} one day later` : 'No day to shift here'}
                        onclick={() => onInstant(row, 1)}
                      >
                        +1d
                      </button>
                      <button
                        class="va-tool"
                        type="button"
                        disabled={!instant}
                        aria-label={instant ? `Pull ${instant.label} one day earlier` : 'No day to shift here'}
                        onclick={() => onInstant(row, -1)}
                      >
                        −1d
                      </button>
                      <button
                        class="va-tool"
                        type="button"
                        aria-label="More tools"
                        aria-expanded={panel === 2}
                        onclick={() => (panelFor = row.id)}
                      >
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5" /></svg>
                      </button>
                    </div>
                    <div class="va-panel va-panel--2">
                      <button
                        class="va-tool"
                        type="button"
                        aria-label="Back to the first tools"
                        onclick={() => (panelFor = null)}
                      >
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 5.5 8.5 12 15 18.5" /></svg>
                      </button>
                      <button
                        class="va-tool"
                        type="button"
                        disabled={!instant}
                        aria-label={instant ? `Push ${instant.label} one week later` : 'No day to shift here'}
                        onclick={() => onInstant(row, 7)}
                      >
                        +1w
                      </button>
                      <button
                        class="va-tool"
                        type="button"
                        disabled={!instant}
                        aria-label={instant ? `Push ${instant.label} one month later` : 'No day to shift here'}
                        onclick={() => onInstant(row, 30)}
                      >
                        +1m
                      </button>
                      <button
                        class="va-tool va-tool--clear"
                        type="button"
                        disabled={!row.instant?.set}
                        aria-label={instant ? `Clear ${instant.label}` : 'Nothing to clear here'}
                        onclick={() => onInstant(row, null)}
                      >
                        Clear
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
            <div
              class="va-cell vt-cell"
              data-field-type={cell.field?.type ?? 'unknown'}
              data-flex={cell.key === flexKey ? '1' : '0'}
            >
              {#if cell.field}
                <FieldControl
                  field={cell.field}
                  record={row.record}
                  targets={cell.targets}
                  oncommit={(key, value) => onCommit(row, key, value)}
                />
              {:else}
                <span class="va-unknown" title="the view names a column the type does not declare">?</span>
              {/if}
              {#if cell.write}
                <span
                  class="cd-chip va-state"
                  class:cd-chip--ok={cell.write.state === 'saved'}
                  class:cd-chip--risk={cell.write.state === 'failed'}
                  role="status"
                >
                  {cell.write.state === 'saving' ? 'Saving…' : cell.write.state === 'saved' ? 'Saved' : 'Not saved'}
                </span>
              {/if}
            </div>
          {/each}

          <!-- The row's own control: the record's detail panel, and the row's
               context menu. Both are the app's, and both appear on hover and on
               focus-within so a keyboard that reaches the row can see them. -->
          <div class="va-rest">
            <button
              class="cd-iconbtn va-rest__btn"
              type="button"
              data-command="record.panel"
              data-placement="recordTable.row"
              aria-label={`Details for ${row.label}`}
              onclick={() => onPanel(row)}
            >
              <Icon name="textQuote" size={14} />
            </button>
            <button
              class="cd-iconbtn va-rest__btn"
              type="button"
              data-rowmenu-trigger
              aria-haspopup="menu"
              aria-expanded={menuFor === row.id}
              aria-label={`Actions for ${row.label}`}
              onclick={(event) => {
                // The menu mounts *during* this click, and `Menu`'s own window
                // handler dismisses on any click outside `.cd-menu` — including this
                // one, which would close the menu in the same frame it opened.
                event.stopPropagation();
                onMenu(menuFor === row.id ? null : row.id);
              }}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M2.6 5.4h6.2M12.4 5.4h1M2.6 10.6h1.6M7.8 10.6h5.6" /><circle cx="10.6" cy="5.4" r="1.5" /><circle cx="6" cy="10.6" r="1.5" /></svg>
            </button>
          </div>
        </li>

        {#if menuFor === row.id}
          <li class="va-menurow">
            <Menu rows={rowRows(row)} onclose={() => onMenu(null)} />
          </li>
        {/if}
        {#if moveFor === row.id}
          <li class="va-menurow">
            <Menu
              rows={moveRows(row)}
              native={false}
              onclose={() => {
                onMove(null);
                onMenu(null);
              }}
            />
          </li>
        {/if}
        {#if row.failures.length > 0}
          <!-- A refused write says what failed, in the engine's own words, at the
               row it was about (D8, F14). -->
          {#each row.failures as reason, position (position)}
            <li class="va-rowfail">
              <p role="alert">{reason}</p>
            </li>
          {/each}
        {/if}
      {/each}
    </ul>
  {/if}
</div>

<style>
  /* Row list styling based on hit token height. */
  .va-rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .va-row {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: calc(var(--hit) + 5px * var(--ui-s));
    padding: 0 var(--ui-pad-sm);
    transition: background var(--dur-1) var(--ease);
  }
  .va-row + .va-row {
    border-top: 1px solid var(--rule);
  }
  .va-row:hover {
    background: var(--well);
  }
  .va-row[data-cur='1'] {
    background: var(--well-2);
  }

  /* ── THE CELL ─────────────────────────────────────────────────────────────
     The value's own box: a 32px hit box inside the row, and the app's own
     control inside that. **The width belongs to the field's type and lives in
     `table.css`**, shared with the column strip above, so a label sits over its
     own cells. */
  .va-cell {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    height: var(--hit);
    padding: 0 var(--ui-gap-sm);
    border-radius: var(--r-mini);
    font-size: var(--ui-text-sm);
    color: var(--ink-2);
  }
  .va-cell[data-field-type='number'],
  .va-cell[data-field-type='duration'],
  .va-cell[data-field-type='formula'],
  .va-cell[data-field-type='progress'] {
    justify-content: flex-end;
    font-variant-numeric: tabular-nums;
  }
  .va-cell[data-field-type='number'] :global(button.cd-cell),
  .va-cell[data-field-type='duration'] :global(button.cd-cell),
  .va-cell[data-field-type='number'] :global(.cd-cellinput),
  .va-cell[data-field-type='duration'] :global(.cd-cellinput) {
    justify-content: flex-end;
    text-align: right;
  }
  /* The app's control fills its own box, so the box is the target — including
     the url cell, whose own wrapper would otherwise hug an empty value and
     leave an 8px-wide click target (FieldControl is not ours to edit). */
  .va-cell :global(button.cd-cell) {
    width: 100%;
    height: var(--hit);
    box-sizing: border-box;
  }
  .va-cell :global(.cd-url) {
    display: flex;
    width: 100%;
    min-width: 0;
  }
  .va-cell :global(.cd-cellinput) {
    height: var(--hit);
    box-sizing: border-box;
  }
  .va-unknown {
    color: var(--ink-4);
  }

  /* ── THE BAY AND THE RAIL ─────────────────────────────────────────────────
     The two panel widths are computed from the fixed 32px the tools are drawn
     on, so they cannot disagree with their own contents. */
  .va-bay {
    --va-p1: calc(var(--hit) * 3 + var(--ui-gap-sm) * 2 + 4px);
    --va-p2: calc(var(--hit) * 3 + var(--hit) * 1.5 + var(--ui-gap-sm) * 3 + 4px);
    flex: none;
    position: relative;
    width: 0;
    height: var(--hit);
    transition: width var(--dur-2) var(--ease);
  }
  .va-row[data-tools='1'] .va-bay {
    width: calc(var(--va-p1) + var(--ui-gap-sm));
  }
  .va-row[data-tools='1'][data-panel='2'] .va-bay {
    width: calc(var(--va-p2) + var(--ui-gap-sm));
  }
  .va-rail {
    position: absolute;
    right: 0;
    top: 50%;
    width: var(--va-p1);
    height: var(--hit);
    transform: translateY(-50%) translateX(6px);
    background: var(--card);
    border-radius: var(--r-pill);
    box-shadow: var(--sh-1), var(--catch);
    overflow: clip;
    opacity: 0;
    pointer-events: none;
    transition:
      width var(--dur-2) var(--ease),
      opacity var(--dur-1) var(--ease),
      transform var(--dur-2) var(--ease);
  }
  .va-row[data-tools='1'] .va-rail {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(-50%);
  }
  .va-row[data-panel='2'] .va-rail {
    width: var(--va-p2);
  }
  .va-track {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    display: flex;
    align-items: center;
    transition: transform var(--dur-2) var(--ease);
  }
  .va-row[data-panel='2'] .va-track {
    transform: translateX(calc(-1 * var(--va-p1)));
  }
  .va-panel {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    padding: 2px;
  }
  .va-panel--1 {
    width: var(--va-p1);
  }
  .va-panel--2 {
    width: var(--va-p2);
    padding-left: 4px;
  }
  /* The clock verbs carry their own words: a glyph for "a day later" is a glyph
     that needs explaining. 32px square is the system's floor, and it is what
     keeps the two panel widths declared numbers. */
  .va-tool {
    flex: none;
    width: var(--hit);
    height: var(--hit);
    padding: 0;
    border-radius: var(--r-pill);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    white-space: nowrap;
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .va-tool:hover {
    background: var(--well);
    color: var(--ink);
  }
  .va-tool:active {
    transform: scale(0.94);
  }
  .va-tool--clear {
    width: calc(var(--hit) * 1.5);
  }
  .va-tool[disabled] {
    opacity: 0.42;
    pointer-events: none;
  }

  /* ── THE ROW'S OWN DOORS ──────────────────────────────────────────────────
     Revealed on hover and on focus-within, never on hover alone (§2.2): a
     keyboard reaching the row must be able to see and take them. */
  .va-rest {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    margin-left: auto;
  }
  .va-rest__btn {
    opacity: 0;
    width: var(--hit);
    height: var(--hit);
    transition: opacity var(--dur-2) var(--ease);
  }
  .va-row:hover .va-rest__btn,
  .va-row:focus-within .va-rest__btn,
  .va-row[data-cur='1'] .va-rest__btn {
    opacity: 1;
  }

  /* The row menu and the move menu, under the row they belong to. */
  .va-menurow {
    list-style: none;
    position: relative;
    z-index: 30;
  }

  /* ── THE WRITE'S OWN STATE ────────────────────────────────────────────────
     At the cell the write landed in, absolutely placed, so a commit never
     re-lays-out the row it landed on. A right-aligned reading keeps the left of
     its box free, so the chip stands there instead and cannot cover the number
     it is about. */
  .va-state {
    position: absolute;
    inset-block: 0;
    inset-inline-end: var(--space-2xs);
    margin: auto 0;
    pointer-events: none;
  }
  .va-cell[data-field-type='number'] .va-state,
  .va-cell[data-field-type='duration'] .va-state,
  .va-cell[data-field-type='date'] .va-state {
    inset-inline-end: auto;
    inset-inline-start: var(--space-2xs);
  }

  /* A refused write: the reason in words, under the row it belongs to, on the
     risk pair the system already pairs for exactly this. */
  .va-rowfail {
    list-style: none;
    margin: var(--space-2xs) 0;
    padding: var(--space-2xs) var(--ui-pad-sm);
    border-radius: var(--r-mini);
    background: var(--chip-risk);
    color: var(--on-risk);
    font-size: var(--ui-meta);
    overflow-wrap: anywhere;
  }
  .va-rowfail p {
    margin: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .va-bay,
    .va-rail,
    .va-track,
    .va-row,
    .va-tool,
    .va-rest__btn {
      transition: none;
    }
  }
</style>
