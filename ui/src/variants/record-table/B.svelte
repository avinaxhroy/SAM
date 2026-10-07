<!--
  RECORD TABLE · B · THE PROFILE.
  Expandable bar variant where each record renders as a card that unfolds its
  field profile in place using CSS grid row transitions (0fr -> 1fr). Supports
  animated column sorting and dedicated action buttons for detail panels and menus.
-->
<script lang="ts">
  import { flip } from 'svelte/animate';
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

  /** Which column the bars are ordered by — null is the read's own order. */
  let picked = $state<string | null>(null);

  /**
   * A per-instance prefix for the ids this component owns (the sort label and
   * each fold). Two record tables can stand on one screen — a view may hold two
   * table blocks — and a fixed id would then name two elements at once.
   */
  const uid = $props.id();

  /**
   * The segments are the view's own rankable columns, in the view's order and
   * capped at three: a switcher with eleven segments is a second schema editor,
   * and a switcher with one segment is a control that cannot switch anything —
   * so with fewer than two, the control is omitted.
   */
  const RANKABLE = ['date', 'number', 'duration', 'formula', 'progress', 'rating'];
  const segments = $derived.by(() => {
    const list: Array<{ key: string; label: string }> = [];
    for (const cell of rows[0]?.cells ?? []) {
      if (list.length === 3) break;
      if (!RANKABLE.includes(cell.type) || list.some((held) => held.key === cell.key)) continue;
      list.push({ key: cell.key, label: cell.label });
    }
    return list.length > 1 ? list : [];
  });

  const sortKey = $derived(
    picked && segments.some((segment) => segment.key === picked) ? picked : (segments[0]?.key ?? null),
  );

  /** One reading, as something a list can be ordered by; null reads last. */
  function orderKey(row: RowFact, key: string): string | number | null {
    const type = row.cells.find((cell) => cell.key === key)?.type ?? '';
    const raw = row.record.derived?.[key] ?? row.record.fields[key];
    if (raw === undefined || raw === null || raw === '') return null;
    if (type === 'date' || type === 'text' || type === 'select') return String(raw);
    const number = Number(raw);
    return Number.isNaN(number) ? String(raw) : number;
  }

  const ordered = $derived.by(() => {
    const key = sortKey;
    if (!key) return rows;
    return [...rows].sort((one, two) => {
      const at = orderKey(one, key);
      const to = orderKey(two, key);
      if (at === null || to === null) return at === null ? (to === null ? 0 : 1) : -1;
      if (typeof at === 'number' && typeof to === 'number') return at - to;
      return String(at).localeCompare(String(to));
    });
  });

  /** Motion: the register owns the number, and reduced motion turns it off. */
  let flipMs = $state(220);
  let reduce = $state(false);
  $effect(() => {
    const query = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    if (!query) return;
    reduce = query.matches;
    const heard = (event: MediaQueryListEvent) => (reduce = event.matches);
    query.addEventListener('change', heard);
    const raw = getComputedStyle(document.documentElement).getPropertyValue('--dur-2').trim();
    const value = Number.parseFloat(raw);
    if (!Number.isNaN(value)) flipMs = raw.endsWith('ms') ? value : value * 1000;
    return () => query.removeEventListener('change', heard);
  });
</script>

<div class="v-fit vb">
  {#if segments.length > 0}
    <div class="vb-tools">
      <span class="vb-tools__label" id={`${uid}-sort`}>Sort</span>
      <div class="cd-seg" role="group" aria-labelledby={`${uid}-sort`}>
        {#each segments as segment (segment.key)}
          <button
            class="cd-seg__pill"
            type="button"
            aria-pressed={sortKey === segment.key}
            onclick={() => (picked = segment.key)}
          >
            {segment.label}
          </button>
        {/each}
      </div>
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
      tone="b"
      {onPlus}
      {onOver}
      {onDrop}
    />
  {/if}

  {#if rows.length > 0}
    <ul class="vb-list">
      {#each ordered as row (row.id)}
        {@const open = selected === row.id}
        <li
          class="vb-item"
          data-record-id={row.id}
          data-placement="recordTable.cell"
          data-open={open ? '1' : '0'}
          data-late={row.instant?.late ? '1' : '0'}
          animate:flip={{ duration: reduce ? 0 : flipMs }}
        >
          <div class="vb-line">
            <button
              class="vb-bar"
              type="button"
              aria-expanded={open}
              aria-controls={`${uid}-fold-${row.id}`}
              aria-label={row.sentence}
              onclick={() => onSelect(row.id)}
            >
              {#if row.mark}
                <span class="cd-ictile vb-tile" data-w={row.mark.wash}>{row.mark.text}</span>
              {:else}
                <span class="vb-tile vb-tile--empty" aria-hidden="true"></span>
              {/if}
              <span class="vb-title">{row.label}</span>
              {#if row.instant?.words}
                <span class="vb-due num">{row.instant.words}</span>
              {/if}
            </button>
            <!-- The row's own doors: the detail panel, and the row's context
                 menu. Beside the bar, never inside it. -->
            <div class="vb-acts">
              <button
                class="cd-iconbtn vb-act"
                type="button"
                data-command="record.panel"
                data-placement="recordTable.row"
                aria-label={`Details for ${row.label}`}
                onclick={() => onPanel(row)}
              >
                <Icon name="textQuote" size={14} />
              </button>
              <button
                class="cd-iconbtn vb-act"
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
                <span class="vb-dots" aria-hidden="true">⋯</span>
              </button>
            </div>
          </div>

          <div class="vb-fold" id={`${uid}-fold-${row.id}`} inert={!open}>
            <div class="vb-foldin">
              <div class="vb-box">
                {#each row.cells as cell (cell.key)}
                  <div class="vb-f" data-field-type={cell.field?.type ?? 'unknown'}>
                    <span class="vb-fk">{cell.label}</span>
                    <div class="vb-fv">
                      {#if cell.field}
                        <FieldControl
                          field={cell.field}
                          record={row.record}
                          targets={cell.targets}
                          oncommit={(key, value) => onCommit(row, key, value)}
                        />
                      {:else}
                        <span class="vb-unknown" title="the view names a column the type does not declare">?</span>
                      {/if}
                      {#if cell.write}
                        <span
                          class="cd-chip vb-state"
                          class:cd-chip--ok={cell.write.state === 'saved'}
                          class:cd-chip--risk={cell.write.state === 'failed'}
                          role="status"
                        >
                          {cell.write.state === 'saving' ? 'Saving…' : cell.write.state === 'saved' ? 'Saved' : 'Not saved'}
                        </span>
                      {/if}
                    </div>
                  </div>
                {/each}

                {#if instant}
                  <div class="vb-div"></div>
                  <!-- The reference's last item, in the danger ink; disabled is
                       the record that has nothing there to clear. -->
                  <button
                    class="vb-danger"
                    type="button"
                    disabled={!row.instant?.set}
                    onclick={() => onInstant(row, null)}
                  >
                    Clear {instant.label}
                  </button>
                {/if}
              </div>

              {#if row.failures.length > 0}
                {#each row.failures as reason, position (position)}
                  <p class="vb-fail" role="alert">{reason}</p>
                {/each}
              {/if}
            </div>
          </div>

          {#if menuFor === row.id}
            <div class="vb-menu">
              <Menu rows={rowRows(row)} onclose={() => onMenu(null)} />
            </div>
          {/if}
          {#if moveFor === row.id}
            <div class="vb-menu">
              <Menu
                rows={moveRows(row)}
                native={false}
                onclose={() => {
                  onMove(null);
                  onMenu(null);
                }}
              />
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  /* ── THE HEAD'S ONE CONTROL ─────────────────────────────────────────────── */
  .vb-tools {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--ui-gap);
    margin-bottom: var(--ui-gap-sm);
  }
  .vb-tools__label {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }

  /* ── THE BARS ─────────────────────────────────────────────────────────────
     Air between them and no rule anywhere: the bar is a card of its own, and
     the only thing that moves when a bar becomes the one you are reading is its
     tile's ring. */
  .vb-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .vb-item + .vb-item {
    margin-top: var(--ui-gap-sm);
  }
  .vb-line {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
  }
  .vb-bar {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: calc(38px * var(--ui-s)) minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(48px * var(--ui-s));
    padding: var(--ui-gap-sm) var(--ui-pad-sm);
    background: var(--card);
    border-radius: var(--r-item);
    box-shadow: var(--sh-1);
    text-align: left;
    transition:
      background var(--dur-1) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .vb-bar:hover {
    background: var(--well);
    box-shadow: var(--sh-2);
  }
  .vb-bar:active {
    box-shadow: var(--sh-1);
  }
  .vb-bar[aria-expanded='true'] .vb-tile {
    box-shadow:
      0 0 0 2px var(--card),
      0 0 0 4px var(--ink);
  }
  .vb-title {
    font-size: var(--ui-text);
    color: var(--ink);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vb-due {
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
    white-space: nowrap;
  }
  .vb-item[data-late='1'] .vb-due {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  /* The record's own course: 38px because a five-character code needs the room,
     and the system's icon tile is a 34–38px square — this is its top end. */
  .vb-tile {
    width: calc(38px * var(--ui-s));
    height: calc(38px * var(--ui-s));
    font-size: var(--ui-meta);
    letter-spacing: 0;
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    display: grid;
    place-items: center;
    border-radius: var(--r-mini);
    background-color: var(--wash, var(--wash-rose));
    background-image: var(--wash-grad, none);
    color: var(--onwash, var(--ink-2));
    flex: none;
    overflow: hidden;
  }
  /* A kind that points at nothing gets no monogram, only the bar's own air. */
  .vb-tile--empty {
    background: none;
    box-shadow: inset 0 0 0 1px var(--rule);
  }

  /* The row's own doors, revealed on hover and on focus-within (never on hover
     alone): a keyboard reaching the row must be able to see them. */
  .vb-acts {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
  }
  .vb-act {
    width: var(--hit);
    height: var(--hit);
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease);
  }
  .vb-line:hover .vb-act,
  .vb-line:focus-within .vb-act,
  .vb-item[data-open='1'] .vb-act {
    opacity: 1;
  }
  .vb-dots {
    font-size: var(--text-base);
    line-height: 1;
  }

  /* ── THE FOLD ─────────────────────────────────────────────────────────────
     The reference's own mechanic on the system's own curve: the row track runs
     0fr → 1fr, the inner is clipped, and the content arrives on an opacity and
     a small translate. Opening pushes the bars below; it never covers them. */
  .vb-fold {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-3) var(--ease);
  }
  .vb-item[data-open='1'] .vb-fold {
    grid-template-rows: 1fr;
  }
  .vb-fold > * {
    overflow: hidden;
  }
  .vb-foldin {
    opacity: 0;
    transform: translateY(-6px);
    transition:
      opacity var(--dur-2) var(--ease),
      transform var(--dur-3) var(--ease);
  }
  .vb-item[data-open='1'] .vb-foldin {
    opacity: 1;
    transform: none;
  }
  .vb-box {
    margin: var(--ui-gap-sm) 0 0;
    padding: var(--space-2xs) 0;
    background: var(--well);
    border-radius: var(--r-tile);
  }
  .vb-f {
    display: grid;
    grid-template-columns: calc(128px * var(--ui-s)) minmax(0, 1fr);
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(var(--hit) + 8px * var(--ui-s));
    padding: 0 var(--ui-pad-sm);
  }
  .vb-f + .vb-f {
    border-top: 1px solid var(--rule);
  }
  .vb-fk {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .vb-fv {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    height: var(--hit);
    border-radius: var(--r-mini);
  }
  /* The app's own control owns the box, so a write never re-lays-out the row. */
  .vb-fv :global(button.cd-cell),
  .vb-fv :global(.cd-cellinput) {
    width: 100%;
    height: var(--hit);
    box-sizing: border-box;
  }
  /* The url cell's own wrapper would hug an empty value and leave an 8px-wide
     click target; the cell's box is the target (FieldControl is not ours). */
  .vb-fv :global(.cd-url) {
    display: flex;
    width: 100%;
    min-width: 0;
  }
  .vb-unknown {
    color: var(--ink-4);
  }
  .vb-div {
    height: 1px;
    background: var(--rule);
    margin: var(--space-2xs) var(--ui-pad-sm);
  }
  /* The reference's last item, in the risk ink. */
  .vb-danger {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    width: calc(100% - var(--ui-pad-sm) * 2);
    margin: 0 var(--ui-pad-sm);
    min-height: var(--hit);
    padding: 0 var(--space-sm);
    border-radius: var(--r-item);
    color: var(--on-risk);
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-label);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .vb-danger:hover {
    background: var(--chip-risk);
  }
  .vb-danger[disabled] {
    opacity: 0.42;
    pointer-events: none;
  }
  .vb-fail {
    margin: var(--space-2xs) var(--ui-pad-sm) var(--ui-pad-sm);
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--r-mini);
    background: var(--chip-risk);
    color: var(--on-risk);
    font-size: var(--ui-meta);
    overflow-wrap: anywhere;
  }

  /* The write's own state, at the value it is about. */
  .vb-state {
    position: absolute;
    inset-block: 0;
    inset-inline-end: var(--space-3xs);
    margin: auto 0;
    pointer-events: none;
  }

  .vb-menu {
    position: relative;
    z-index: 30;
  }

  @media (prefers-reduced-motion: reduce) {
    .vb-bar,
    .vb-fold,
    .vb-foldin,
    .vb-act,
    .vb-danger {
      transition: none;
    }
  }
</style>
