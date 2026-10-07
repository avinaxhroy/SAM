<!--
  PROGRESS · B · THE VESSELS.
  Vessel-style progress metrics where fill height corresponds to the metric's
  target capacity, with support for comparing current vs previous period.
-->
<script lang="ts">
  import type { Figure, ProgressStatsProps } from './props';

  let { figures }: ProgressStatsProps = $props();

  /** Which reading the four vessels hold. The lab's own two, nothing else. */
  let view = $state<'this' | 'prev'>('this');

  /** The numeral's measured place inside each vessel: the reference's liquid
      has a surface for the chip to break, and the level is read there. */
  let place = $state<Record<string, { bottom: number; on: 'well' | 'ink' }>>({});

  let vessels: Record<string, HTMLElement | undefined> = {};
  let nums: Record<string, HTMLElement | undefined> = {};

  /** The reading the segment has selected for one figure. A figure with no
      predecessor — the two plan totals, and any figure on a plan younger than
      the window — has nothing to print for the week before, so it prints the
      empty read and the panel's own sentence rather than this week's number
      under a "week before" heading. */
  function reading(figure: Figure) {
    if (view === 'this') return figure;
    return (
      figure.prev ?? {
        value: '—',
        num: 0,
        of: figure.of,
        ofText: figure.ofText,
        delta: figure.delta,
      }
    );
  }

  /** `share` is the vessel's own arithmetic: the value over its own capacity,
      clamped. A figure the plan declares no denominator for stays empty. */
  function share(figure: Figure): number {
    const read = reading(figure);
    if (!read.of || read.of <= 0) return 0;
    return Math.max(0, Math.min(100, (read.num / read.of) * 100));
  }

  /** Placement runs once per state change **and** once the display face has
      landed: a late font swap changes the glyph's height under a position
      computed for the fallback (the lab's own note). */
  function placeAll(): void {
    const next: Record<string, { bottom: number; on: 'well' | 'ink' }> = {};
    for (const figure of figures) {
      const vessel = vessels[figure.key];
      const num = nums[figure.key];
      if (!vessel || !num) continue;
      const height = vessel.clientHeight || 148;
      const fillPx = (share(figure) / 100) * height;
      const glyph = num.offsetHeight || 46;
      next[figure.key] =
        fillPx + glyph + 10 <= height
          ? { bottom: fillPx + 10, on: 'well' }
          : { bottom: Math.max(8, fillPx - glyph - 10), on: 'ink' };
    }
    place = next;
  }

  $effect(() => {
    // Read the state the placement depends on, so a change re-places.
    void view;
    void figures;
    placeAll();
    if (typeof document !== 'undefined' && document.fonts) {
      void document.fonts.ready.then(placeAll);
    }
  });
</script>

<div class="v-fit vb">
  <section class="cd-card vb-card">
    <header class="cd-card__head">
      <h2 class="cd-card__title">The four figures</h2>
      <span class="cd-card__spacer"></span>
      <div class="cd-seg" role="group" aria-label="Which week the vessels hold">
        <button
          class="cd-seg__pill"
          type="button"
          aria-pressed={view === 'this'}
          onclick={() => (view = 'this')}
        >
          This week
        </button>
        <button
          class="cd-seg__pill"
          type="button"
          aria-pressed={view === 'prev'}
          onclick={() => (view = 'prev')}
        >
          The week before
        </button>
      </div>
    </header>

    <div class="vb-row">
      {#each figures as figure, index (figure.key)}
        {@const read = reading(figure)}
        <div class="vb-fig" data-fig={figure.key} style={`--rise: ${index * 60}ms`}>
          <span
            class="vb-vessel"
            style={`--v: ${share(figure).toFixed(2)}%`}
            bind:this={vessels[figure.key]}
          >
            <span class="vb-rim" aria-hidden="true"></span>
            <span class="vb-fill" aria-hidden="true"></span>
            <b
              class="vb-num"
              data-on={place[figure.key]?.on ?? 'well'}
              style={place[figure.key] ? `bottom: ${place[figure.key].bottom}px` : undefined}
              bind:this={nums[figure.key]}
            >{read.value}</b>
          </span>
          <p class="vb-label">{figure.label}</p>
          <p class="vb-den">{read.ofText}</p>
          <p class="vb-delta">{read.delta}</p>
        </div>
      {/each}
    </div>
  </section>
</div>

<style>
  /* ══ B · THE VESSELS ════════════════════════════════════════════════════ */

  .vb-card { padding: calc(var(--space-xl) * var(--ui-s)); }
  .vb-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: calc(var(--space-xl) * var(--ui-s));
  }

  /* One vessel. The body is the figure's own denominator and the ink is its
     value, so a fill is a length of ink read against a capacity that is printed
     in words beneath it — the four fills compare only because each vessel says
     what it is full of. */
  .vb-vessel {
    --v: 0%;
    position: relative;
    display: block;
    height: calc(148px * var(--ui-s));
    background: var(--well);
    border-radius: var(--r-tile);
    overflow: clip;
  }
  /* the rim: the reference's mouth, 6px of a deeper well across the top */
  .vb-rim {
    position: absolute;
    inset: 0 0 auto 0;
    height: 6px;
    background: var(--well-2);
  }
  .vb-fill {
    position: absolute;
    inset: auto 0 0 0;
    height: var(--v);
    background: var(--ink);
    transform-origin: 50% 100%;
    transition: height var(--dur-3) var(--ease);
    /* the reference's chip drops in; a level can only rise, so the arrival is
       the fill coming up from the floor, one vessel 60ms behind the last */
    animation: vb-rise var(--dur-3) var(--ease) both;
    animation-delay: var(--rise, 0ms);
  }
  @keyframes vb-rise {
    from { transform: scaleY(0); }
    to { transform: none; }
  }
  /* The numeral rides the fill's own surface, 10px clear of it. The bottom is
     set from measured pixels on render (see the component); this is the shape
     it holds before the first measurement lands. */
  .vb-num {
    position: absolute;
    left: 50%;
    bottom: calc(clamp(8px, var(--v), calc(100% - 58px)) + 10px);
    transform: translateX(-50%);
    font-size: calc(var(--text-2xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    white-space: nowrap;
    transition: bottom var(--dur-3) var(--ease), color var(--dur-2) var(--ease);
    animation: vb-fade var(--dur-2) var(--ease) both;
    animation-delay: var(--rise, 0ms);
  }
  /* on the liquid: the same numeral, read against the ink instead of the well */
  .vb-num[data-on='ink'] { color: var(--ink-inv); }
  @keyframes vb-fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .vb-label {
    margin-top: calc(var(--space-md) * var(--ui-s));
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .vb-den,
  .vb-delta {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    letter-spacing: var(--track-title);
  }
  .vb-den { color: var(--ink-3); }
  .vb-delta { color: var(--ink-2); }
  /* the mark's own hover: the two facts it is read against come forward */
  .vb-fig:hover .vb-den,
  .vb-fig:hover .vb-delta { color: var(--ink); }

  /* Four vessels need four columns; a 360px record panel has one. */
  @container (max-width: 720px) {
    .vb-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @container (max-width: 420px) {
    .vb-row { grid-template-columns: minmax(0, 1fr); }
    .vb-vessel { height: calc(120px * var(--ui-s)); }
  }
</style>
