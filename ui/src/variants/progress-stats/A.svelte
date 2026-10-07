<!--
  PROGRESS · A · THE OUTLINE NUMERALS.
  Displays the week's key metrics using large outline numerals with interactive
  drag echoes, paired with a 7-day target/floor rod chart.
-->
<script lang="ts">
  import type { ProgressStatsProps } from './props';

  let { figures, week, aimMin, weekNote }: ProgressStatsProps = $props();

  /** The day the student pinned, or none — the pin outlives the pointer. */
  let pinned = $state(-1);
  /** The day being read: the rod under the pointer, or the one with focus. */
  let hovered = $state(-1);
  /** The numeral being dragged, or null. Only the drag's two ends change it. */
  let dragging = $state<string | null>(null);

  /**
   * The reference's drag: movement lights the stack, release puts it away.
   *
   * Pointer-only — the numeral is data, not a control, and its value is
   * printed in the column and pushed to the reader with no pointer at all. The
   * listeners are attached at the DOM (the lab's own `wireDrag`), so a
   * `pointermove` writes two custom properties and re-renders nothing; only the
   * two ends of a drag touch state, because only they change what is drawn.
   * Under reduced motion the drag is refused outright — the reference's own
   * case, kept: the stack does not exist, so there is nothing to light.
   */
  function dragStack(node: HTMLElement, api: { set: (dragging: boolean) => void }) {
    let id: number | null = null;
    let fromX = 0;
    let fromY = 0;

    function down(event: PointerEvent): void {
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
      id = event.pointerId;
      fromX = event.clientX;
      fromY = event.clientY;
      node.setPointerCapture(id);
      api.set(true);
    }

    function move(event: PointerEvent): void {
      if (id === null || event.pointerId !== id) return;
      // The reference's `clamp(delta, containerRect…)` at figure scale: a
      // numeral that wandered into its neighbour's column would be a layout bug
      // wearing a costume. `--tx`/`--ty` are the negated delta, as the lab sets
      // them.
      const x = -Math.max(-14, Math.min(14, event.clientX - fromX));
      const y = -Math.max(-12, Math.min(12, event.clientY - fromY));
      node.style.setProperty('--tx', `${x}px`);
      node.style.setProperty('--ty', `${y}px`);
    }

    function end(): void {
      id = null;
      api.set(false);
      node.style.removeProperty('--tx');
      node.style.removeProperty('--ty');
    }

    node.addEventListener('pointerdown', down);
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerup', end);
    node.addEventListener('pointercancel', end);
    return {
      destroy() {
        node.removeEventListener('pointerdown', down);
        node.removeEventListener('pointermove', move);
        node.removeEventListener('pointerup', end);
        node.removeEventListener('pointercancel', end);
      },
    };
  }

  /** The rod's own scale: minutes against the plan's daily aim, in percent. The
      aim is the plan's line; with no aim declared the tallest day is the scale,
      and the caption says which of the two it is reading. */
  const scale = $derived(
    aimMin ?? Math.max(60, ...week.map((day) => day.minutes)),
  );

  const caption = $derived(
    hovered >= 0 ? week[hovered].fact : pinned >= 0 ? week[pinned].fact : weekNote,
  );

  const chartLabel = $derived(
    week.length > 0
      ? `Minutes studied each day, ${week[0].when} to ${week[week.length - 1].when}`
      : 'Minutes studied each day',
  );
</script>

<div class="v-fit oa">
  <div class="cd-card oa-figs">
    {#each figures as figure (figure.key)}
      <div class="oa-fig" data-fig={figure.key}>
        <!-- The numeral and its echo stack. At rest the echoes are invisible
             (`opacity: 0`) — the reference's law is that the stack exists only
             while it is moved. -->
        <span
          class="oa-stack"
          class:oa-drag={dragging === figure.key}
          use:dragStack={{ set: (on) => (dragging = on ? figure.key : null) }}
        >
          <i class="oa-echo" data-layer="1" aria-hidden="true">{figure.value}</i>
          <i class="oa-echo" data-layer="2" aria-hidden="true">{figure.value}</i>
          <b class="oa-num">{figure.value}</b>
        </span>
        <p class="oa-label">{figure.label}</p>
        <p class="oa-den">{figure.ofText}</p>
        <p class="oa-delta">{figure.delta}</p>
      </div>
    {/each}
  </div>

  <section class="cd-card oa-week">
    <header class="cd-card__head">
      <div>
        <h2 class="cd-card__title">The week</h2>
        <p class="cd-card__sub oa-cap">{caption}</p>
      </div>
      <span class="cd-card__spacer"></span>
      <!-- A reset with nothing to reset is not a control. -->
      <button
        class="cd-pill cd-pill--quiet cd-pill--sm"
        type="button"
        disabled={pinned < 0}
        onclick={() => (pinned = -1)}
      >
        All days
      </button>
    </header>

    <div class="oa-chart" role="group" aria-label={chartLabel} data-aim={aimMin && aimMin > 0 ? '1' : '0'}>
      {#each week as day, index (day.date)}
        <button
          class="oa-day"
          type="button"
          data-empty={day.minutes === 0 ? '1' : '0'}
          aria-pressed={pinned === index}
          onclick={() => (pinned = pinned === index ? -1 : index)}
          onmouseenter={() => (hovered = index)}
          onmouseleave={() => (hovered = -1)}
          onfocus={() => (hovered = index)}
          onblur={() => (hovered = -1)}
        >
          <span class="oa-day__rod" aria-hidden="true">
            <i style={`--v: ${scale > 0 ? (day.minutes / scale) * 100 : 0}`}></i>
          </span>
          <span class="oa-day__d" aria-hidden="true">{day.weekday}</span>
          <span class="cd-sr">{day.fact}</span>
        </button>
      {/each}
    </div>
  </section>
</div>

<style>
  /* ══ A · THE OUTLINE NUMERALS ═══════════════════════════════════════════
     The lab's own geometry, with every structural number measured from the
     size register (`calc(<token> * var(--ui-s))`) and the drawing constants —
     the 8px rod, the 32px tick, the 52px numeral — left as they are drawn. */

  .oa {
    display: grid;
    gap: var(--ui-gap-lg);
  }

  /* the four figures, as four ruled columns of one card */
  .cd-card.oa-figs {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    padding: 0;
    overflow: clip;
  }
  .oa-fig {
    display: grid;
    align-content: start;
    gap: calc(var(--space-2xs) * var(--ui-s));
    padding: calc(var(--space-xl) * var(--ui-s));
    min-width: 0;
    /* The column measures itself: a numeral's own width is in its text, and the
       app's durations (`11 h 30 min`) are far wider than the lab's sample
       (`5h 0m`) — see the fit on `.oa-num` below. */
    container-type: inline-size;
  }
  .oa-fig + .oa-fig { border-left: 1px dashed var(--rule-strong); }

  /* the numeral and its echo stack */
  .oa-stack {
    --tx: 0px;
    --ty: 0px;
    position: relative;
    display: block;
    margin-bottom: calc(var(--space-md) * var(--ui-s));
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .oa-stack.oa-drag { cursor: grabbing; }
  /* The hover spread is the reference's `MIN_TRAIL_DISTANCE` at figure scale:
     small enough that the numeral stays the same object. */
  .oa-stack:hover { --tx: -6px; --ty: -6px; }
  .oa-num,
  .oa-echo {
    font-family: var(--font-display);
    /* The display numeral, capped to what the column can actually hold: the
       lab's four figures were five characters each (`5h 0m`), the app's
       duration is `11 h 30 min` — ten — and a numeral that overflowed into its
       neighbour's column would be the layout bug the drag is clamped against.
       14cqw is the width the longest value this screen prints (`123 h 45 min`)
       needs at the tabular advance. */
    font-size: min(calc(var(--text-display) * var(--ui-s)), 14cqw);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .oa-num {
    position: relative;
    z-index: 1;
    display: block;
    color: var(--ink);
    transform: translate(calc(var(--tx) * 0.3), calc(var(--ty) * 0.3));
    transition: transform var(--dur-3) var(--ease);
  }
  /* the outline itself; the rule above is the solid ink fallback */
  @supports (-webkit-text-stroke: 1px currentColor) {
    .oa-num,
    .oa-echo {
      -webkit-text-stroke: 1.5px var(--ink);
      color: transparent;
    }
  }
  .oa-echo {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 0;
    opacity: 0;
    pointer-events: none;
    transform: translate(calc(var(--tx) * var(--k)), calc(var(--ty) * var(--k)));
    transition: transform var(--dur-3) var(--ease), opacity var(--dur-2) var(--ease);
  }
  .oa-echo[data-layer='2'] { --k: 0.55; }
  .oa-echo[data-layer='1'] { --k: 1; }
  .oa-stack:hover .oa-echo,
  .oa-stack.oa-drag .oa-echo { opacity: 1; }
  .oa-stack:hover .oa-echo[data-layer='1'] { opacity: 0.34; }
  .oa-stack:hover .oa-echo[data-layer='2'] { opacity: 0.2; }
  .oa-stack.oa-drag .oa-echo[data-layer='1'] { opacity: 0.5; }
  .oa-stack.oa-drag .oa-echo[data-layer='2'] { opacity: 0.3; }

  /* the two facts under the numeral: what the figure is of, and its delta */
  .oa-label {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .oa-den,
  .oa-delta {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    letter-spacing: var(--track-title);
  }
  .oa-den { color: var(--ink-3); }
  .oa-delta { color: var(--ink-2); }

  /* the minimal week chart: seven rods, one hairline each, seven letters */
  .oa-week { margin-top: 0; }
  .oa-chart {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: calc(var(--space-sm) * var(--ui-s));
  }
  .oa-day {
    display: grid;
    gap: calc(var(--space-xs) * var(--ui-s));
    justify-items: center;
    min-height: calc(160px * var(--ui-s));
    padding: calc(var(--space-xs) * var(--ui-s)) calc(var(--space-xs) * var(--ui-s)) 0;
    border-radius: var(--r-item);
    background: none;
    border: 0;
    transition: background var(--dur-1) var(--ease);
  }
  .oa-day:hover { background: var(--well); }
  .oa-day:active { background: var(--well-2); }
  /* The column is 124px of height for a 112px aim: the hairline at 112 is the
     day's own ceiling and a day that passes it is allowed to — the rod crosses
     the line instead of being clipped by it, and the caption says by how much.
     `--v` is unitless (minutes ÷ aim × 100), so the rod is a length of ink
     against the marked aim and not a percentage of the column. */
  .oa-day__rod {
    position: relative;
    display: block;
    width: 100%;
    height: calc(124px * var(--ui-s));
  }
  /* Two 32px ticks per column and no axis: the aim at 112px — which the rod is
     allowed to cross — and the floor at 0, so a zero day is a position rather
     than an absence. */
  .oa-day__rod::before,
  .oa-day__rod::after {
    content: '';
    position: absolute;
    left: 50%;
    margin-left: -16px;
    width: 32px;
    height: 1px;
    background: var(--rule-strong);
  }
  .oa-day__rod::before { bottom: calc(112px * var(--ui-s)); }
  .oa-day__rod::after { bottom: 0; }
  /* No aim declared is no aim rule: the tick would be a line the plan never
     asked for, and the rods are then drawn against the tallest day (see
     `scale` above), which the caption says in words. */
  .oa-chart[data-aim='0'] .oa-day__rod::before { display: none; }
  .oa-day__rod i {
    position: absolute;
    left: 50%;
    bottom: 0;
    width: 8px;
    margin-left: -4px;
    height: max(2px, calc(var(--v, 0) * 1.12px * var(--ui-s)));
    border-radius: var(--r-pill);
    background: var(--ink-3);
    transition: height var(--dur-3) var(--ease), background var(--dur-1) var(--ease);
  }
  /* a day with nothing logged keeps a 2px mark: `void` is not a bar */
  .oa-day[data-empty='1'] .oa-day__rod i {
    height: 2px;
    background: var(--rule-strong);
  }
  .oa-day:hover .oa-day__rod i,
  .oa-day[aria-pressed='true'] .oa-day__rod i { background: var(--ink); }
  .oa-day__d {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .oa-day:hover .oa-day__d,
  .oa-day[aria-pressed='true'] .oa-day__d { color: var(--ink); }
  .oa-cap { color: var(--ink-2); }

  /* The container query. The lab's four columns share a card measured in
     hundreds of pixels; in the record panel's 360px, or a split window, they
     fold to two and then to one — the rows are text, so they keep their shape
     while the rod row wraps rather than clips. */
  @container (max-width: 720px) {
    .cd-card.oa-figs { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .oa-fig:nth-child(3) { border-left: 0; }
    .oa-fig:nth-child(n + 3) { border-top: 1px dashed var(--rule-strong); }
  }
  @container (max-width: 420px) {
    .cd-card.oa-figs { grid-template-columns: minmax(0, 1fr); }
    .oa-fig + .oa-fig { border-left: 0; }
    .oa-fig:nth-child(n + 2) { border-top: 1px dashed var(--rule-strong); }
    .oa-chart { gap: calc(var(--space-2xs) * var(--ui-s)); }
    .oa-day__rod i { width: 6px; margin-left: -3px; }
  }

  /* base.css already collapses every animation and transition to 1ms here;
     said once more for this component so the refusal is explicit in the file
     that adds the motion — A's echo stack is the reference's own reduced-motion
     case (no stack at all, not a short one). */
  @media (prefers-reduced-motion: reduce) {
    .oa-stack:hover { --tx: 0px; --ty: 0px; }
    .oa-echo { opacity: 0 !important; }
  }
</style>
