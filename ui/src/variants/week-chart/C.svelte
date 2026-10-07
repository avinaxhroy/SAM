<!--
  Week chart variant C: The Shelf.
  Stacked horizontal progress tracks with notch indicators for daily goals and
  expandable detail disclosure per day.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    aimOf,
    ceilingOf,
    dayLabel,
    peakOf,
    rangeLabel,
    readLong,
    readShort,
    valueText,
    type WeekChartProps,
    type WeekDay,
  } from './props';

  let { title, caption, days, targetMin }: WeekChartProps = $props();

  const ceiling = $derived(ceilingOf(days, targetMin));
  const aim = $derived(aimOf(ceiling, targetMin));
  const peak = $derived(peakOf(days));
  const percents = $derived(days.map((day) => Math.min(1, day.loggedMin / ceiling) * 100));

  /* Today is open on arrival — the screen's default state is the day you are in
     and what it owes the target. A week with no today (a read that has not
     landed) opens with every tile closed, which is a legal state. The initial
     value is deliberately read once: a read that lands later must not reopen a
     tile the student has closed. */
  // svelte-ignore state_referenced_locally
  let open = $state<string | null>(days.find((day) => day.isToday)?.date ?? null);
  let chart = $state<HTMLElement | null>(null);

  /** The open tile's own sentence: the distance, and the peak's own fact. */
  function openRead(day: WeekDay): string {
    const distance = readLong(day.loggedMin, targetMin);
    if (!distance) return '';
    return day.date === peak ? `${distance} · most logged` : distance;
  }

  function onKey(event: KeyboardEvent, index: number) {
    if (event.key === 'Escape') {
      open = null;
      return;
    }
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    const nodes = chart?.querySelectorAll<HTMLElement>('.wcc-tile');
    const next = nodes?.[index + (event.key === 'ArrowDown' ? 1 : -1)];
    if (next) {
      event.preventDefault();
      next.focus();
    }
  }
</script>

<section class="cd-card wcc" style:--aim={aim ?? 0}>
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="chart" /></span>
    <div>
      <h2 class="cd-card__title">{title}</h2>
      <p class="cd-card__sub">{caption}</p>
    </div>
  </header>

  {#if days.length > 0}
    <div class="wcc-chart" bind:this={chart}>
      <div class="wcc-shelf" role="group" aria-label={`${title}, ${rangeLabel(days)}`}>
        {#each days as day, index (day.date)}
          <button
            class="wcc-tile"
            type="button"
            aria-expanded={open === day.date}
            aria-label={dayLabel(day, targetMin, peak)}
            onclick={() => (open = open === day.date ? null : day.date)}
            onkeydown={(event) => onKey(event, index)}
          >
            <span class="wcc-name">
              <b>{day.weekday}</b>
              <i>{day.isToday ? 'today' : day.day}</i>
            </span>
            <span class="cd-meter__track">
              <span class="cd-meter__fill" style:--v={`${percents[index]}%`}></span>
              {#if aim !== null}
                <span class="wcc-aim" aria-hidden="true"></span>
              {/if}
            </span>
            <span class="wcc-val">
              <b class="num">{valueText(day.loggedMin)}</b>
              <i>{day.date === peak ? readShort(day.loggedMin, targetMin) : ''}</i>
            </span>
            <span class="wcc-fold">
              <span class="wcc-read"><i>{openRead(day)}</i></span>
            </span>
          </button>
        {/each}
      </div>

      <div class="wcc-axis" aria-hidden="true">
        <!-- When the target IS the ceiling (a plan whose daily target is the
             round stop above every day) the notch lands on the track's own end,
             and the two names would sit on one another at one x. The aim's name
             already carries the number, so the ceiling's own name stands down. -->
        <span class="wcc-axis__rail">
          <span class="wcc-axis__zero">0</span>
          {#if aim !== null}
            <span class="wcc-axis__aim">{valueText(targetMin ?? 0)} · target</span>
          {/if}
          {#if aim === null || aim < 0.98}
            <span class="wcc-axis__cap">{valueText(ceiling)}</span>
          {/if}
        </span>
      </div>
    </div>
  {/if}
</section>

<style>
  .wcc {
    container-type: inline-size;
  }

  .wcc-chart {
    --wcc-col: calc(104px * var(--ui-s));
    --wcc-pad: calc(12px * var(--ui-s));
    --wcc-gap: calc(12px * var(--ui-s));
  }

  .wcc-shelf {
    display: grid;
    gap: calc(4px * var(--ui-s));
  }
  .wcc-tile {
    display: grid;
    grid-template-columns: var(--wcc-col) minmax(0, 1fr) var(--wcc-col);
    align-items: center;
    column-gap: var(--wcc-gap);
    padding: var(--wcc-pad);
    border: 0;
    border-radius: var(--r-tile);
    background: var(--well);
    cursor: pointer;
    text-align: left;
    transition: background var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease),
      transform var(--dur-2) var(--ease);
  }
  .wcc-tile:hover {
    box-shadow: var(--sh-1);
  }
  .wcc-tile:active {
    transform: translateY(1px);
  }
  .wcc-tile:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* the open tile: the raised one, and the only one whose reading is open */
  .wcc-tile[aria-expanded='true'] {
    background: var(--card);
    box-shadow: var(--sh-1);
  }

  .wcc-name {
    display: grid;
    gap: 1px;
  }
  .wcc-name b {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    line-height: 1.2;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .wcc-name i {
    font-style: normal;
    font-size: var(--ui-meta);
    line-height: 1.2;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .wcc-tile[aria-expanded='true'] .wcc-name b {
    color: var(--ink);
    font-weight: var(--weight-display);
  }
  .wcc-tile[aria-expanded='true'] .wcc-name i {
    color: var(--ink-2);
  }

  /* The meter is the system's; the notch is ours. The notch is the tile's *own*
     surface colour, so the aim reads as a cut through the bar in both themes:
     `--well` on a quiet tile, `--card` on the open one. */
  .wcc-tile .cd-meter__track {
    position: relative;
    height: 8px;
    transition: height var(--dur-2) var(--ease);
  }
  .wcc-aim {
    position: absolute;
    top: 0;
    bottom: 0;
    left: calc(var(--aim) * 100%);
    width: 2px;
    border-radius: var(--r-pill);
    background: var(--well);
  }
  .wcc-tile[aria-expanded='true'] .wcc-aim {
    background: var(--card);
  }
  .wcc-tile[aria-expanded='true'] .cd-meter__track {
    height: 12px;
  }

  .wcc-val {
    display: grid;
    justify-items: end;
    gap: 1px;
    text-align: right;
  }
  .wcc-val b {
    font-size: var(--ui-text);
    font-weight: var(--weight-display);
    line-height: 1.2;
    letter-spacing: var(--track-title);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    white-space: nowrap;
    transition: font-size var(--dur-2) var(--ease);
  }
  /* the state line — *most logged*, *5 min over*, *nothing* — reserved in every
     tile so the stack's rhythm never jumps when one tile opens */
  .wcc-val i {
    display: block;
    min-height: calc(13px * var(--ui-s));
    font-style: normal;
    font-size: var(--ui-meta);
    line-height: 1.2;
    color: var(--ink-3);
    white-space: nowrap;
  }
  /* The open value is one step below the display sizes on purpose: a long
     duration at `--text-xl` would wrap in its own column, and a value that
     wraps when you open its tile is a value you have to re-read. */
  .wcc-tile[aria-expanded='true'] .wcc-val b {
    font-size: calc(var(--text-lg) * var(--ui-s));
  }

  /* the reading: `grid-template-rows` 0fr → 1fr opens a row to its own height
     without scripting it */
  .wcc-fold {
    grid-column: 2 / 4;
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-2) var(--ease);
  }
  .wcc-tile[aria-expanded='true'] .wcc-fold {
    grid-template-rows: 1fr;
  }
  .wcc-read {
    overflow: hidden;
    min-height: 0;
    font-size: var(--ui-meta);
    color: var(--ink-3);
  }
  .wcc-read i {
    display: block;
    padding-top: calc(4px * var(--ui-s));
    font-style: normal;
  }

  /* the axis under the stack: the two ends of the scale and the aim named where
     the notch is cut, on the tracks' own x */
  .wcc-axis {
    display: grid;
    grid-template-columns: var(--wcc-col) minmax(0, 1fr) var(--wcc-col);
    column-gap: var(--wcc-gap);
    padding: 0 var(--wcc-pad);
    margin-top: calc(4px * var(--ui-s));
  }
  .wcc-axis__rail {
    position: relative;
    grid-column: 2;
    height: calc(32px * var(--ui-s));
    border-top: 1px solid var(--rule);
  }
  .wcc-axis__rail span {
    position: absolute;
    top: calc(4px * var(--ui-s));
    font-size: var(--ui-meta);
    line-height: 1.2;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .wcc-axis__zero {
    left: 0;
  }
  .wcc-axis__cap {
    right: 0;
  }
  /* The ends of the scale name themselves on the rail's own line; the aim names
     itself on a line under it, its right edge exactly on the notch it labels.
     The lab draws all three on one line because its rail is 456px; a card of
     Today's width makes 80% and 100% close neighbours, and a name that touched
     the ceiling's own name would read as one label. */
  .wcc-axis__rail .wcc-axis__aim {
    top: calc(18px * var(--ui-s));
    right: calc(100% - var(--aim) * 100%);
  }

  /* Narrow: the two fixed columns size to their own content, so the long open
     value never wraps and the bar keeps the middle. */
  @container (max-width: 520px) {
    .wcc-chart {
      --wcc-col: minmax(0, auto);
      --wcc-gap: calc(8px * var(--ui-s));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wcc-tile,
    .wcc-fold,
    .wcc-read,
    .wcc-val b,
    .wcc-tile .cd-meter__track {
      transition: none;
    }
  }
</style>
