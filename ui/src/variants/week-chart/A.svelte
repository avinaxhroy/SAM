<!--
  Week chart variant A: The Grid.
  Seven day cards rendered side-by-side with vertical duration bars, daily target line,
  and keyboard navigation across days.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    aimOf,
    ceilingOf,
    dayLabel,
    peakOf,
    rangeLabel,
    readShort,
    valueText,
    type WeekChartProps,
  } from './props';

  let { title, caption, days, targetMin }: WeekChartProps = $props();

  const ceiling = $derived(ceilingOf(days, targetMin));
  const aim = $derived(aimOf(ceiling, targetMin));
  const peak = $derived(peakOf(days));
  const shares = $derived(days.map((day) => Math.min(1, day.loggedMin / ceiling)));

  let pinned = $state<string | null>(null);
  let chart = $state<HTMLElement | null>(null);

  /* One read at a time. Escape belongs to the chart, so the pin clears wherever
     the focus happens to stand; the arrows walk the week. */
  function onKey(event: KeyboardEvent, index: number) {
    if (event.key === 'Escape') {
      pinned = null;
      return;
    }
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    const nodes = chart?.querySelectorAll<HTMLElement>('.wca-day');
    const next = nodes?.[index + (event.key === 'ArrowRight' ? 1 : -1)];
    if (next) {
      event.preventDefault();
      next.focus();
    }
  }
</script>

<section class="cd-card wca" style:--aim={aim ?? 0}>
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="chart" /></span>
    <div>
      <h2 class="cd-card__title">{title}</h2>
      <p class="cd-card__sub">{caption}</p>
    </div>
  </header>

  {#if days.length > 0}
    <div class="wca-chart" role="group" aria-label={`${title}, ${rangeLabel(days)}`} bind:this={chart}>
      <div class="wca-gutter" aria-hidden="true">
        <span class="wca-glab">{valueText(ceiling)}</span>
      </div>

      <div class="wca-scroll">
        <div class="wca-week">
          <span class="wca-cap" aria-hidden="true"></span>
          <span class="wca-floor" aria-hidden="true"></span>
          {#if aim !== null}
            <!-- A target at the top of the scale would lift the chip out of the
                 band and onto the first day's own date, so `is-high` hangs it
                 under the rule instead of straddling it. -->
            <span class="wca-aim" class:is-high={aim > 0.85} aria-hidden="true">
              <span class="cd-chip">{valueText(targetMin ?? 0)} · target</span>
            </span>
          {/if}

          {#each days as day, index (day.date)}
            <button
              class="wca-day"
              class:is-nil={day.loggedMin === 0}
              class:is-peak={day.date === peak}
              class:is-today={day.isToday}
              class:is-pinned={pinned === day.date}
              type="button"
              aria-pressed={pinned === day.date}
              aria-label={dayLabel(day, targetMin, peak)}
              style:--f={shares[index]}
              style:--i={index}
              onclick={() => (pinned = pinned === day.date ? null : day.date)}
              onkeydown={(event) => onKey(event, index)}
            >
              <span class="wca-name">
                <b>{day.weekday}</b>
                <i>{day.isToday ? 'today' : day.day}</i>
              </span>
              <span class="wca-band">
                {#if day.date === peak}
                  <span class="wca-say">most logged</span>
                {/if}
                {#if day.loggedMin === 0}
                  <i class="wca-nil"></i>
                {:else}
                  <i class="wca-mark"></i>
                {/if}
              </span>
              <span class="wca-foot">
                <b class="num">{valueText(day.loggedMin)}</b>
                <span class="wca-read">{readShort(day.loggedMin, targetMin)}</span>
              </span>
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  /* The card is the container: everything inside measures from the size
     register, and a narrow column reflows without touching the window. */
  .wca {
    container-type: inline-size;
    --wca-shelf: calc(8px * var(--ui-s));
    --wca-pad: calc(10px * var(--ui-s));
    --wca-band: calc(124px * var(--ui-s));
    --wca-foot: calc(34px * var(--ui-s));
    --wca-base: calc(var(--wca-shelf) + var(--wca-pad) + var(--wca-foot));
    --wca-top: calc(var(--wca-base) + var(--wca-band));
    --wca-aim: calc(var(--wca-base) + var(--aim) * var(--wca-band));
  }

  .wca-chart {
    display: grid;
    grid-template-columns: calc(40px * var(--ui-s)) minmax(0, 1fr);
    column-gap: calc(6px * var(--ui-s));
  }
  /* The ceiling, named where it is drawn; the label may overflow its column
     leftward into the card's own padding, which is what that padding is for. */
  .wca-gutter {
    position: relative;
  }
  .wca-glab {
    position: absolute;
    right: 0;
    bottom: var(--wca-top);
    transform: translateY(50%);
    font-size: var(--ui-meta);
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .wca-week {
    position: relative;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    gap: calc(4px * var(--ui-s));
    padding: var(--wca-shelf);
    border-radius: var(--r-tile);
    background: var(--well);
  }
  .wca-cap {
    position: absolute;
    left: var(--wca-shelf);
    right: var(--wca-shelf);
    bottom: var(--wca-top);
    border-top: 1px solid var(--rule);
    z-index: 3;
  }
  .wca-floor {
    position: absolute;
    left: var(--wca-shelf);
    right: var(--wca-shelf);
    bottom: var(--wca-base);
    border-top: 1px dashed var(--rule-strong);
    z-index: 3;
  }
  .wca-aim {
    position: absolute;
    left: var(--wca-shelf);
    right: var(--wca-shelf);
    bottom: var(--wca-aim);
    border-top: 1px dashed var(--rule-strong);
    z-index: 3;
  }
  .wca-aim .cd-chip {
    position: absolute;
    left: 0;
    top: 0;
    transform: translateY(-50%);
  }
  /* a target at the ceiling: the chip hangs under the rule, inside the band */
  .wca-aim.is-high .cd-chip {
    transform: none;
  }

  /* ── a day card ─────────────────────────────────────────────────────── */
  .wca-day {
    --f: 0;
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: auto var(--wca-band) var(--wca-foot);
    justify-items: center;
    padding: var(--wca-pad) 0;
    border: 0;
    border-radius: var(--r-tile);
    background: var(--card);
    box-shadow: var(--sh-1);
    cursor: pointer;
    text-align: center;
    transition: box-shadow var(--dur-2) var(--ease);
  }
  /* The staged pair rises one rung. That is the whole staging: no bed behind a
     column, no second colour, nothing to decode. */
  .wca-day.is-peak,
  .wca-day.is-today {
    box-shadow: var(--sh-2);
  }
  .wca-day:hover {
    box-shadow: var(--sh-2);
  }
  .wca-day:active {
    box-shadow: var(--sh-1), inset 0 0 0 1.5px var(--rule-strong);
  }
  .wca-day.is-pinned {
    box-shadow: var(--sh-2), inset 0 0 0 1.5px var(--rule-strong);
  }
  .wca-day:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  .wca-name {
    display: grid;
    justify-items: center;
    gap: 1px;
    padding-bottom: calc(4px * var(--ui-s));
  }
  .wca-name b {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    line-height: 1.2;
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .wca-name i {
    font-style: normal;
    font-size: var(--ui-meta);
    line-height: 1.2;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .wca-day.is-today .wca-name b {
    color: var(--ink);
    font-weight: var(--weight-display);
  }
  .wca-day.is-today .wca-name i {
    color: var(--ink-2);
  }

  .wca-band {
    position: relative;
    width: 100%;
    display: grid;
    align-items: end;
    justify-items: center;
  }
  /* the mark: the height is always the data, the width never is */
  .wca-mark {
    width: 8px;
    height: calc(var(--f) * 100%);
    border-radius: var(--r-pill);
    background: var(--ink);
  }
  /* A day with nothing is a mark, not an absence: a baseline tick with the word
     *nothing* on the foot, both in `--ink-3` (5.5:1 on the card). */
  .wca-nil {
    width: 18px;
    height: 3px;
    border-radius: var(--r-pill);
    background: var(--ink-3);
  }
  /* the peak's own label, standing above its mark */
  .wca-say {
    position: absolute;
    left: -8px;
    right: -8px;
    bottom: calc(var(--f) * 100% + calc(4px * var(--ui-s)));
    font-size: var(--ui-meta);
    color: var(--ink-2);
    white-space: nowrap;
  }
  .wca-foot {
    display: grid;
    justify-items: center;
    align-content: start;
    gap: 1px;
    width: 100%;
  }
  .wca-foot b {
    font-size: var(--ui-text);
    font-weight: var(--weight-display);
    line-height: 1.2;
    letter-spacing: var(--track-title);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    white-space: nowrap;
  }
  .wca-day.is-nil .wca-foot b {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    color: var(--ink-3);
  }
  /* The read's line, reserved in every card so a read never moves a line of the
     week. The peak and today print it at rest; the other five answer a read
     with it. `opacity` only. */
  .wca-read {
    font-size: var(--ui-meta);
    line-height: 1.2;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    opacity: 0;
    transition: opacity var(--dur-1) var(--ease);
  }
  .wca-day.is-peak .wca-read,
  .wca-day.is-today .wca-read,
  .wca-day:hover .wca-read,
  .wca-day:focus-visible .wca-read,
  .wca-day.is-pinned .wca-read {
    opacity: 1;
  }

  /* one arrival, staggered left to right — stopped dead by the motion gate */
  @keyframes wca-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .wca-day {
    animation: wca-in var(--dur-3) var(--ease) both;
    animation-delay: calc(var(--i, 0) * 35ms);
  }

  /* Narrow: the gutter thins and the foot steps down a size; the seven columns
     keep one shared floor, which is the point of the design. */
  @container (max-width: 560px) {
    .wca-chart {
      grid-template-columns: calc(32px * var(--ui-s)) minmax(0, 1fr);
    }
    .wca-foot b {
      font-size: var(--ui-meta);
    }
    .wca-read {
      font-size: calc(10px * var(--ui-s));
    }
  }
  /* Narrower still: seven columns of a duration cannot be squeezed further
     without one day's number touching the next, so the shelf keeps its column
     width and becomes a scroll port — the lab's own answer for a port narrower
     than the drawing. The gutter stays outside it, so the ceiling it names is
     always in view. */
  @container (max-width: 460px) {
    .wca-scroll {
      overflow-x: auto;
      overflow-y: hidden;
    }
    .wca-week {
      min-width: calc(520px * var(--ui-s));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .wca-day,
    .wca-read {
      animation: none;
      transition: none;
    }
  }
</style>
