<!--
  Week chart variant B: The Fan.
  Radial fan layout of leaning duration bars with dual horizontal reference rules
  and weekday axis markers.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    aimOf,
    ceilingOf,
    dayLabel,
    midStopOf,
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
  const mid = $derived(midStopOf(ceiling));
  const peak = $derived(peakOf(days));
  const shares = $derived(days.map((day) => Math.min(1, day.loggedMin / ceiling)));

  /** The tag's second line: the distance, with today naming itself first. */
  function tagNote(day: WeekDay): string {
    const distance = readLong(day.loggedMin, targetMin);
    if (!distance) return '';
    return day.isToday ? `today · ${distance}` : distance;
  }

  /** The axis's own value cell: the peak and the zeros state their word. */
  function axisValue(day: WeekDay): string {
    if (day.date === peak) return 'most logged';
    return valueText(day.loggedMin);
  }

  const words = $derived(new Set(days.filter((day) => day.loggedMin === 0 || day.date === peak).map((d) => d.date)));

  let pinned = $state<string | null>(null);
  let chart = $state<HTMLElement | null>(null);

  function onKey(event: KeyboardEvent, index: number) {
    if (event.key === 'Escape') {
      pinned = null;
      return;
    }
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    const nodes = chart?.querySelectorAll<HTMLElement>('.wcb-cell');
    const next = nodes?.[index + (event.key === 'ArrowRight' ? 1 : -1)];
    if (next) {
      event.preventDefault();
      next.focus();
    }
  }
</script>

<section class="cd-card wcb" style:--aim={aim ?? 0}>
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="chart" /></span>
    <div>
      <h2 class="cd-card__title">{title}</h2>
      <p class="cd-card__sub">{caption}</p>
    </div>
  </header>

  {#if days.length > 0}
    <div class="wcb-chart" role="group" aria-label={`${title}, ${rangeLabel(days)}`} bind:this={chart}>
      <div class="wcb-scroll">
        <div class="wcb-plot">
          <div class="wcb-labels" aria-hidden="true">
            <span class="wcb-lab" style:--lvl="1">{valueText(ceiling)}</span>
            {#if mid > 0}
              <span class="wcb-lab" style:--lvl={mid / ceiling}>{valueText(mid)}</span>
            {/if}
          </div>

          <div class="wcb-stage">
            <div class="wcb-fan">
              {#each days as day, index (day.date)}
                <button
                  class="wcb-cell"
                  class:is-peak={day.date === peak}
                  class:is-pinned={pinned === day.date}
                  type="button"
                  aria-pressed={pinned === day.date}
                  aria-label={dayLabel(day, targetMin, peak)}
                  style:--f={shares[index]}
                  style:--k={index - 3}
                  onclick={() => (pinned = pinned === day.date ? null : day.date)}
                  onkeydown={(event) => onKey(event, index)}
                >
                  {#if day.date === peak}
                    <span class="wcb-say" aria-hidden="true">
                      <b class="num">{valueText(day.loggedMin)}</b>
                      <i>{readShort(day.loggedMin, targetMin)}</i>
                    </span>
                  {/if}
                  <span class="wcb-tag" aria-hidden="true">
                    <b class="num">{valueText(day.loggedMin)}</b>
                    <i>{tagNote(day)}</i>
                  </span>
                  {#if day.loggedMin === 0}
                    <span class="wcb-nil"></span>
                  {:else}
                    <span class="wcb-rod"></span>
                  {/if}
                </button>
              {/each}

              {#if aim !== null}
                <span class="wcb-aim" aria-hidden="true">
                  <span class="cd-chip">{valueText(targetMin ?? 0)} · target</span>
                </span>
              {/if}
            </div>
          </div>

          <div class="wcb-axis" aria-hidden="true">
            {#each days as day (day.date)}
              <span class="wcb-axis__d" data-today={day.isToday ? '1' : undefined}>
                <b>{day.weekday} <i>{day.isToday ? 'today' : day.day}</i></b>
                <u class="num" class:is-word={words.has(day.date)}>{axisValue(day)}</u>
              </span>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .wcb {
    container-type: inline-size;
  }

  .wcb-chart {
    --wcb-band: calc(200px * var(--ui-s));
    --wcb-gutter: calc(64px * var(--ui-s));
    --wcb-pitch: calc(62px * var(--ui-s));
    --wcb-stick: calc(74px * var(--ui-s));
    --wcb-tag-h: calc(46px * var(--ui-s));
  }

  .wcb-plot {
    display: grid;
    grid-template-columns: var(--wcb-gutter) minmax(0, 1fr);
    grid-template-rows: auto auto;
    column-gap: calc(6px * var(--ui-s));
  }
  .wcb-labels {
    grid-column: 1;
    grid-row: 1;
    position: relative;
  }
  /* The two level names hang at their own fraction of the band, in the plane
     the fan leaves free. */
  .wcb-lab {
    position: absolute;
    right: 0;
    bottom: calc(var(--lvl) * var(--wcb-band));
    transform: translateY(50%);
    font-size: var(--ui-meta);
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    z-index: 6;
  }
  .wcb-stage {
    grid-column: 2;
    grid-row: 1;
    position: relative;
    height: calc(var(--wcb-band) + calc(4px * var(--ui-s)));
  }
  /* The fan is its own grid, centred: seven pitches, so a stick's foot and the
     day's name under it are one x. The sticks overflow their cells by 6px on
     each side — that overflow *is* the overlap, and it is why the visible part
     of a stick is exactly its own cell: what you see is what you hit. */
  .wcb-fan {
    position: absolute;
    left: 50%;
    bottom: 0;
    transform: translateX(-50%);
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: var(--wcb-pitch);
    width: calc(var(--wcb-pitch) * 7);
  }
  /* the ceiling: a solid hairline at the top of the band */
  .wcb-fan::before {
    content: '';
    position: absolute;
    left: -6px;
    right: -6px;
    bottom: var(--wcb-band);
    border-top: 1px solid var(--rule);
  }
  /* the floor: the zero every stick stands on, the app's own dashed plot floor */
  .wcb-fan::after {
    content: '';
    position: absolute;
    left: -6px;
    right: -6px;
    bottom: 0;
    border-top: 1px dashed var(--rule-strong);
  }

  .wcb-cell {
    position: relative;
    display: grid;
    align-items: end;
    justify-items: center;
    height: var(--wcb-band);
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }
  .wcb-cell:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* the stick: 74px of ink at the day's own height, leaning about its foot.
     The 2px halo is the card's own colour, so a deck of overlapping sticks
     reads as separate objects instead of one stepped silhouette. */
  .wcb-rod {
    width: var(--wcb-stick);
    height: calc(var(--f) * var(--wcb-band));
    transform-origin: 50% 100%;
    transform: rotate(calc(var(--k) * 2.6deg));
    border-radius: var(--r-mini);
    background: var(--ink);
    box-shadow: 0 0 0 2px var(--card);
    transition: transform var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease);
  }
  .wcb-cell:hover .wcb-rod {
    box-shadow: 0 0 0 2px var(--card), var(--sh-1);
  }
  .wcb-cell:active .wcb-rod {
    transform: translateY(1px) rotate(calc(var(--k) * 2.6deg));
  }
  .wcb-cell.is-pinned .wcb-rod {
    box-shadow: 0 0 0 2px var(--card), var(--sh-1);
  }
  /* a day with nothing: the baseline tick and the word on the axis */
  .wcb-nil {
    width: 18px;
    height: 3px;
    border-radius: var(--r-pill);
    background: var(--ink-3);
  }
  /* the peak's standing label, above its own stick */
  .wcb-say {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    bottom: calc(var(--f) * var(--wcb-band) + calc(4px * var(--ui-s)));
    display: flex;
    align-items: baseline;
    gap: calc(4px * var(--ui-s));
    padding: 1px calc(4px * var(--ui-s));
    border-radius: var(--r-mini);
    background: var(--card);
    font-size: var(--ui-meta);
    white-space: nowrap;
    z-index: 6;
    transition: opacity var(--dur-1) var(--ease);
  }
  .wcb-say b {
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }
  .wcb-say i {
    font-style: normal;
    color: var(--ink-3);
  }
  /* the read: the day's value and its distance, clamped into the plot so a
     stick at the top of the scale still gets its label */
  .wcb-tag {
    position: absolute;
    left: 50%;
    z-index: 7;
    bottom: min(
      calc(var(--f) * var(--wcb-band) + calc(12px * var(--ui-s))),
      calc(var(--wcb-band) - var(--wcb-tag-h))
    );
    transform: translate(-50%, 4px);
    display: grid;
    justify-items: center;
    gap: 1px;
    padding: calc(4px * var(--ui-s)) calc(8px * var(--ui-s));
    border-radius: var(--r-mini);
    background: var(--card);
    box-shadow: var(--sh-2);
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .wcb-cell:hover .wcb-tag,
  .wcb-cell:focus-visible .wcb-tag,
  .wcb-cell.is-pinned .wcb-tag {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  /* The first cell's tag is held inside the fan: the scale's names hang in the
     plane just left of Monday's foot, and a tag centred on a 62px cell reaches
     further left than the plane it is allowed to cross. */
  .wcb-cell:first-child .wcb-tag {
    left: 0;
    transform: translate(0, 4px);
  }
  .wcb-cell:first-child:hover .wcb-tag,
  .wcb-cell:first-child:focus-visible .wcb-tag,
  .wcb-cell:first-child.is-pinned .wcb-tag {
    transform: translate(0, 0);
  }
  .wcb-tag b {
    font-size: var(--ui-text);
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
  }
  .wcb-tag i {
    font-style: normal;
    font-size: var(--ui-meta);
    color: var(--ink-3);
  }
  .wcb-cell.is-pinned .wcb-tag {
    box-shadow: var(--sh-2), inset 0 0 0 1.5px var(--rule-strong);
  }
  /* One read at a time: the standing label steps aside for the tag that repeats
     what it says. */
  .wcb-cell.is-peak:hover .wcb-say,
  .wcb-cell.is-peak:focus-visible .wcb-say,
  .wcb-cell.is-peak.is-pinned .wcb-say {
    opacity: 0;
  }
  .wcb-chart:has(.wcb-cell:hover) .wcb-cell.is-peak:not(:hover) .wcb-say {
    opacity: 0;
  }
  .wcb-chart:has(.wcb-cell:focus-visible) .wcb-cell.is-peak:not(:focus-visible) .wcb-say {
    opacity: 0;
  }

  /* the aim, spanning the fan's own extent, chip on its left end */
  .wcb-aim {
    position: absolute;
    left: -6px;
    right: -6px;
    bottom: calc(var(--aim) * var(--wcb-band));
    border-top: 1px dashed var(--rule-strong);
    z-index: 5;
  }
  .wcb-aim .cd-chip {
    position: absolute;
    left: 0;
    top: 0;
    transform: translateY(-50%);
  }

  /* the axis: one line of names over one line of values, its cells on the same
     pitch as the sticks, so a name is under its own stick's foot */
  .wcb-axis {
    grid-column: 2;
    grid-row: 2;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: var(--wcb-pitch);
    width: calc(var(--wcb-pitch) * 7);
    margin: calc(8px * var(--ui-s)) auto 0;
  }
  .wcb-axis__d {
    display: grid;
    justify-items: center;
    gap: 1px;
    text-align: center;
  }
  .wcb-axis__d b {
    display: flex;
    align-items: baseline;
    gap: 3px;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .wcb-axis__d b i {
    font-style: normal;
    font-weight: var(--weight-body);
    letter-spacing: 0;
    color: var(--ink-3);
    text-transform: none;
  }
  .wcb-axis__d u {
    font-size: var(--ui-meta);
    text-decoration: none;
    color: var(--ink);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .wcb-axis__d u.is-word {
    color: var(--ink-3);
  }
  .wcb-axis__d[data-today='1'] b {
    color: var(--ink);
    font-weight: var(--weight-display);
  }
  .wcb-axis__d[data-today='1'] b i {
    color: var(--ink-2);
  }

  /* Narrow: the fan is a fixed 448px hand, so below the room it needs the plot
     becomes a scroll port that keeps the lab's pitch rather than squeezing
     seven sticks (and their seven names) into a column narrower than a stick.
     The gutter labels and the aim chip travel with it — they belong to the
     plane they name. */
  @container (max-width: 540px) {
    .wcb-chart {
      --wcb-gutter: calc(52px * var(--ui-s));
    }
    .wcb-scroll {
      overflow-x: auto;
      overflow-y: hidden;
    }
    .wcb-plot {
      min-width: calc(492px * var(--ui-s));
    }
    /* In a scroll port the last cell's tag would be cut by the port's edge, so
       it hangs from its own cell's right edge the way the first one hangs from
       its left. */
    .wcb-cell:last-child .wcb-tag {
      left: auto;
      right: 0;
      transform: translate(0, 4px);
    }
    .wcb-cell:last-child:hover .wcb-tag,
    .wcb-cell:last-child:focus-visible .wcb-tag,
    .wcb-cell:last-child.is-pinned .wcb-tag {
      transform: translate(0, 0);
    }
  }
  @container (max-width: 370px) {
    .wcb-chart {
      --wcb-band: calc(170px * var(--ui-s));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wcb-rod,
    .wcb-tag,
    .wcb-say {
      transition: none;
    }
  }
</style>
