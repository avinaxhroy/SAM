<!--
  PROBLEMSETS DUE · A · THE DAY TAPE.
  Timeline tape variant where due dates are plotted along a scrollable daily
  ruler starting from today's marker. Due items render as horizontal span bars
  with course chips and distance labels.
-->
<script lang="ts">
  import BlockHead from '../../blocks/BlockHead.svelte';
  import Doors from './Doors.svelte';
  import { WASHES } from '../../types';
  import { barOf, emptyWords, tapeOf, type ListCardProps, type ListCardRow } from './props';

  let {
    title,
    view,
    type,
    rows,
    candidates,
    today,
    selected,
    menuFor,
    menuRows,
    onSelect,
    onPanel,
    onMenu,
    onNew,
    onPaste,
    onRenumber,
  }: ListCardProps = $props();

  /** The rows the tape may draw: a record with no day has no bar, and a slip
   *  standing on the tape without one would say the record is due on a day it
   *  does not hold — the one thing this design must never say. So the tape
   *  draws the dated rows, and the dateless ones are listed under its port. The
   *  read's own order is kept on both sides. */
  const dated = $derived(rows.filter((row) => row.day !== null));
  const undated = $derived(rows.filter((row) => row.day === null));

  /** The tape, ruled once by the contract's own arithmetic (no sorting here). */
  const cells = $derived(tapeOf(dated, today));
  /** The cell today's printed rule stands at — the tape's one anchor. A tape
   *  whose rule is not among its cells has nothing to run bars from, so it is
   *  drawn as the plain stack below instead. */
  const ruleIndex = $derived(cells.findIndex((cell) => cell.today));
  const ruled = $derived(cells.length > 0 && ruleIndex >= 0);

  /** The tape's bands, in the read's own order: each dated row with the bar it
   *  runs. A row the tape cannot run a bar to never becomes a band: the
   *  contract's capped window excludes that day. */
  const bands = $derived(
    dated.flatMap((row) => {
      const bar = barOf(cells, row);
      return bar === null ? [] : [{ row, bar }];
    }),
  );

  /** Every row the tape does not draw, in the read's own order: a record with
   *  no day has no bar, and neither has one whose day the capped window never
   *  rules — and a bar-less slip standing on the tape would say the record is
   *  due on whichever day it happened to stand at, start or end. */
  const offTape = $derived(rows.filter((row) => !bands.some((band) => band.row.id === row.id)));

  /** Why those rows are off the tape, each reason only when it counts: the
   *  dateless rows, and the dated rows the ruled window does not reach (the
   *  second is zero when no tape can be ruled at all — there the whole body is
   *  the stack, and no window has been drawn to fall outside of). */
  const noDate = $derived(undated.length);
  const outside = $derived(ruled ? dated.length - bands.length : 0);

  /** The port's own name: the window it holds and where today is on it. */
  const tapeName = $derived(
    !ruled
      ? `The day tape — ${title}`
      : `The day tape — ${title}. ${cells.length} days, ${cells[0].long} to ` +
          `${cells[cells.length - 1].long}; today is ${cells[ruleIndex]?.long ?? cells[0].long}.`,
  );

  // Wash color variables mapped from WASHES.
  const WASH_ROOM: Record<string, readonly [string, string]> = Object.fromEntries(
    WASHES.map((room) => [room, [`var(--w-${room})`, `var(--fg-${room})`]]),
  );

  // Position slip immediately adjacent to the due date endpoint.
  function slipFrom(bar: { index: number } | null): number {
    const last = cells.length - 4;
    if (bar === null) return Math.max(0, last);
    const from = bar.index < ruleIndex ? bar.index - 4 : bar.index;
    return Math.max(0, Math.min(from, last));
  }

  // Display month header only at month transitions.
  function monthAt(index: number): string | null {
    const cell = cells[index];
    if (index > 0 && cells[index - 1].day.slice(0, 7) === cell.day.slice(0, 7)) return null;
    return cell.long.split(' ')[2] ?? null;
  }

  let port = $state<HTMLDivElement | null>(null);

  // Scroll viewport to today's rule or to beginning if any items are late.
  function anchor(node: HTMLDivElement): void {
    if (node.scrollWidth <= node.clientWidth) return;
    const rule = node.querySelector<HTMLElement>('.pd-rule');
    if (rule === null) return;
    node.scrollLeft = dated.some((row) => row.late)
      ? 0
      : Math.max(0, rule.offsetLeft - node.clientWidth / 3);
  }

  // Re-anchor on tape rebuild or container width change.
  $effect(() => {
    const node = port;
    const ruledOn = ruled;
    if (node === null || !ruledOn) return;
    let width = node.clientWidth;
    const settle = requestAnimationFrame(() => anchor(node));
    const observer = new ResizeObserver(() => {
      if (node.clientWidth === width) return;
      width = node.clientWidth;
      anchor(node);
    });
    observer.observe(node);
    return () => {
      cancelAnimationFrame(settle);
      observer.disconnect();
    };
  });
</script>

{#snippet slip(row: ListCardRow, place: string)}
  <div
    class="pd-slip"
    data-record-id={row.id}
    data-held={selected === row.id ? 'true' : undefined}
    style={place === '' ? undefined : place}
  >
    <!-- The object is its own control; its acts stand beside it, below. -->
    <button
      class="pd-slip__press"
      type="button"
      aria-pressed={selected === row.id}
      aria-label={row.sentence}
      onclick={() => onSelect(row.id)}
    >
      <span class="pd-nm">{row.label}</span>
      <span class="pd-fact">
        {#if row.course}
          <span class="cd-chip cd-chip--code" data-w={row.course.wash}>{row.course.code}</span>
        {/if}
        {#if row.due}
          <span class="pd-when num" data-late={row.late}>{row.due}</span>
        {/if}
      </span>
    </button>
    <div class="pd-slip__acts">
      <Doors {row} open={menuFor === row.id} {menuRows} {onPanel} {onMenu} />
    </div>
  </div>
{/snippet}

<section class="cd-card pd-card" data-view={view} data-type={type}>
  <BlockHead {title} {type} word="List" returned={rows.length} {candidates} {onNew} {onPaste} {onRenumber} />

  {#if rows.length === 0}
    <p class="cd-empty"><span class="cd-empty__s">{emptyWords(candidates, type)}</span></p>
  {:else}
    {#if ruled}
      <div
        class="pd-port cd-scroll"
        role="region"
        aria-label={tapeName}
        tabindex="0"
        bind:this={port}
      >
        <div class="pd-tape" style={`--pd-cols: ${cells.length}`}>
          <!-- The ruler: one cell per day, placed on its own grid line. It is
               the tape's printing, not its content — every date a record
               carries in words it carries on its own slip — so it is decoration
               to a reader, and the tape's window and today are stated once, in
               the region's own name above. -->
          {#each cells as cell, index (cell.day)}
            {@const month = monthAt(index)}
            <div
              class="pd-cell pd-ruler"
              data-today={cell.today ? '1' : undefined}
              style={`grid-row: 1; grid-column: ${index + 1} / ${index + 2}`}
              aria-hidden="true"
            >
              <span class="pd-ruler__w">{cell.today ? 'Today' : cell.weekday}</span>
              <span class="pd-ruler__d num">{cell.num}{#if month !== null}{' '}<span class="pd-ruler__m">{month}</span>{/if}</span>
            </div>
          {/each}

          <!-- The printed rule: today, down the tape's whole height. -->
          <span
            class="pd-rule"
            style={`grid-row: 1 / span ${dated.length + 1}; grid-column: ${ruleIndex + 1} / ${ruleIndex + 2}`}
            aria-hidden="true"
          ></span>

          {#each dated as row, band (row.id)}
            {@const bar = barOf(cells, row)}
            {@const place = `grid-row: ${band + 2}; grid-column: ${slipFrom(bar) + 1} / span 4`}
            {@const wash = row.course?.wash ? WASH_ROOM[row.course.wash] : undefined}
            <div class="pd-band" style={`grid-row: ${band + 2}`} aria-hidden="true"></div>
            <!-- The band's cells: the day columns run the tape's full height. -->
            {#each cells as cell, index (cell.day)}
              <div
                class="pd-cell"
                style={`grid-row: ${band + 2}; grid-column: ${index + 1} / ${index + 2}`}
                aria-hidden="true"
              ></div>
            {/each}
            {#if bar !== null}
              <div
                class="pd-bar"
                data-dir={bar.index < ruleIndex ? 'late' : 'ahead'}
                style={`grid-row: ${band + 2}; grid-column: ${Math.min(bar.index, ruleIndex) + 1} / ${Math.max(bar.index, ruleIndex) + 1}${wash ? `; --wash: ${wash[0]}; --onwash: ${wash[1]}` : ''}`}
                aria-hidden="true"
              ></div>
            {/if}
            {@render slip(row, place)}
          {/each}
        </div>
      </div>
    {/if}

    <!-- No rule to print means no tape at all (today has not landed yet, or no
         row holds a day), and a record with no day could never join one: the
         tape rules days, so a bar-less slip on it would read as due on the
         first day the tape holds. Those rows stand under the port instead,
         introduced by the app's own sentence (`RecordCalendar`), in the read's
         own order — and a record the read returned is never dropped. -->
    {#if !ruled}
      <div class="pd-plain">
        {#if undated.length > 0}
          <p class="pd-note">
            {undated.length} {undated.length === 1 ? 'record has' : 'records have'} no date
          </p>
        {/if}
        {#each rows as row (row.id)}
          {@render slip(row, '')}
        {/each}
      </div>
    {:else if undated.length > 0}
      <div class="pd-plain">
        <p class="pd-note">
          {undated.length} {undated.length === 1 ? 'record has' : 'records have'} no date
        </p>
        {#each undated as row (row.id)}
          {@render slip(row, '')}
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .pd-card {
    container-type: inline-size;
  }

  /* ── A · THE DAY TAPE ─────────────────────────────────────────────────── */

  /* The port: the tape's own scroll (`.cd-scroll` is the app's scrollbar; the
     overflow is this card's answer to a container narrower than the cells). */
  .pd-port {
    overflow-x: auto;
    overflow-y: hidden;
    padding-bottom: var(--space-2xs);
  }

  /* The tape is a strip of paper: a warm sheet with punched edges, ruled once
     per day. Its width is its cells' own — it never squeezes to the port. */
  .pd-tape {
    /* The cell is the drawing's own proportion: one custom property, so the
       narrow container below can shrink the whole tape with one value. */
    --pd-cell: 64px;
    display: grid;
    grid-template-columns: repeat(var(--pd-cols, 14), var(--pd-cell));
    grid-auto-rows: min-content;
    width: max-content;
    position: relative;
    /* the lead and tail keep a slip off the tape's own rounded ends */
    padding: var(--space-sm) var(--space-sm) var(--space-md);
    background-color: color-mix(in oklab, var(--wash-peach) 62%, var(--card));
    border-radius: var(--r-tile);
    /* The perforation: one punched hole per cell along the tape's two edges. */
    background-image:
      radial-gradient(circle at 5px 3px, var(--rule-strong) 0 1.6px, transparent 1.7px),
      radial-gradient(circle at 5px calc(100% - 3px), var(--rule-strong) 0 1.6px, transparent 1.7px);
    background-size: var(--pd-cell) 100%, var(--pd-cell) 100%;
    background-position: 0 0, 0 0;
    background-repeat: repeat-x, repeat-x;
  }

  /* One cell: the ruler's cell carries a weekday and a day number, a band's cell
     carries nothing but its own rule — the day columns run the tape's height. */
  .pd-cell {
    min-width: 0;
    border-right: 1px solid var(--rule);
  }
  .pd-tape > .pd-cell:first-child {
    border-left: 1px solid var(--rule);
  }

  .pd-ruler {
    display: grid;
    align-content: start;
    justify-items: center;
    gap: 1px;
    padding: var(--space-2xs) 0 var(--space-xs);
  }
  .pd-ruler__w {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .pd-ruler__d {
    font-size: calc(var(--text-xs) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }
  .pd-ruler__m {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .pd-cell[data-today] .pd-ruler__w,
  .pd-cell[data-today] .pd-ruler__d {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* The printed rule: today is the tape's one strong vertical mark. It is
     `--ink-4`, the system's non-text grey — a printed rule, never an ink object,
     so the screen keeps exactly one of those (the head's own door). A rule is
     printed, so it ends square: no radius anywhere on this tape. */
  .pd-rule {
    justify-self: start;
    width: 2px;
    background: var(--ink-4);
  }

  .pd-band {
    grid-column: 1 / -1;
    align-self: start;
    height: 0;
    border-top: 1px solid var(--rule);
  }

  /* The bar: from the rule to the record's own due end, inked flat across the
     day cells it spans — its length is the count and its ends are printed, so
     it carries no rounding. The tick at the due end is that day's boundary. */
  .pd-bar {
    align-self: end;
    height: 12px;
    margin-bottom: calc(14px * var(--ui-s));
    background-color: var(--wash, var(--well-2));
    position: relative;
  }
  .pd-bar::after {
    content: '';
    position: absolute;
    right: 0;
    top: -4px;
    bottom: -4px;
    width: 2px;
    background: var(--onwash, var(--ink-4));
  }
  .pd-bar[data-dir='late']::after {
    right: auto;
    left: 0;
  }

  /* The slip: the record's own name and facts, standing outside the bar's due
     end. The bar's span is the day count; the slip never covers it. */
  .pd-slip {
    align-self: start;
    justify-self: start;
    min-width: 0;
    display: grid;
    background: var(--card);
    border-radius: var(--r-item);
    box-shadow: var(--sh-1);
  }
  .pd-slip[data-held='true'] {
    box-shadow: var(--sh-2), inset 0 0 0 1.5px var(--rule-strong);
  }

  /* The press is the record's own control: the name leads, the facts follow. */
  .pd-slip__press {
    display: grid;
    gap: calc(4px * var(--ui-s));
    min-width: 0;
    min-height: var(--hit);
    padding: var(--ui-pad-sm) var(--ui-pad-sm) calc(6px * var(--ui-s));
    text-align: left;
    border-radius: var(--r-item) var(--r-item) 0 0;
    transition: background var(--dur-1) var(--ease);
  }
  .pd-slip__press:hover {
    background: var(--well);
  }
  .pd-slip__press:active {
    background: var(--well-2);
  }
  .pd-slip__press:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .pd-nm {
    font-size: calc(var(--text-base) * var(--ui-s));
    line-height: 1.3;
    color: var(--ink);
    overflow-wrap: anywhere;
    text-wrap: pretty;
  }
  .pd-fact {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: calc(8px * var(--ui-s));
    min-width: 0;
  }
  .pd-when {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
  }
  .pd-when[data-late='true'] {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* The record's acts, below the record: the door and the record's own verbs,
     each a `--hit` square (the app's own controls). */
  .pd-slip__acts {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    padding: 0 var(--ui-pad-sm) var(--ui-pad-sm);
  }
  /* The slip's own row menu opens across the tape, never off its end. The app's
     own rule right-aligns a row menu to its acts cluster — right for the shipped
     list card, whose acts stand at the row's end — but the tape's slips carry
     their acts at the slip's start, where a right-aligned 200px menu would hang
     off the paper's left end (a late record, which is exactly the record this
     design leads with) and be cut by the port. Anchored to the cluster's left it
     opens over the day cells instead, which is the room the tape has. */
  .pd-slip :global(.cd-menu) {
    left: 0;
    right: auto;
  }

  /* The slips the tape cannot carry (a record with no day, or the whole card
     when no tape can be ruled), listed under the port in the read's own order,
     under the app's own sentence for them (`RecordCalendar`'s note). */
  .pd-plain {
    display: grid;
    gap: var(--ui-gap-sm);
  }
  .pd-port + .pd-plain {
    margin-top: var(--ui-gap);
  }
  .pd-note {
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .pd-plain .pd-slip {
    max-width: 100%;
  }

  /* ── the container's own answer: a card narrower than the tape ────────── */
  @container (max-width: 520px) {
    .pd-tape {
      --pd-cell: 46px;
    }
  }

  /* ── reduced motion: the one transition on this tape is the press's own ── */
  @media (prefers-reduced-motion: reduce) {
    .pd-slip__press {
      transition: none;
    }
  }
</style>
