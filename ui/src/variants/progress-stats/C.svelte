<!--
  PROGRESS · C · THE DOT MATRIX.
  Term activity dot matrix variant displaying daily study duration intensity
  across rows and weeks, with keyboard navigation and summary statistics.
-->
<script lang="ts">
  import type { ProgressStatsProps } from './props';

  let { figures, term, aimMin, termNote }: ProgressStatsProps = $props();

  /** The day the pointer is reading, or none. */
  let hovered = $state(-1);
  /** Where the ring cursor sits. It opens on the last day — the one just lived. */
  let cursor = $state(-1);
  let focused = $state(false);

  /** The cursor opens on the term's last day once the term lands. */
  $effect(() => {
    if (cursor < 0 && term.length > 0) cursor = term.length - 1;
  });

  const at = $derived(term[Math.max(0, cursor)] ?? null);

  /**
   * One tab stop with a ring cursor, not 112 buttons: the lab's own accepted
   * trade (112 tab stops would be a maze, and the dots' values are in the
   * caption and in the `.cd-sr` sentence). The listeners are attached at the
   * DOM — the lab's own wiring — so 112 marks cost no per-dot closure, and the
   * arrows move a week or a day, Home/End go to the term's ends.
   */
  function gridKeys(
    node: HTMLElement,
    api: {
      cursor: () => number;
      count: () => number;
      setCursor: (index: number) => void;
      setHover: (index: number) => void;
    },
  ) {
    let current = api;
    const moves: Record<string, number> = {
      ArrowLeft: -7,
      ArrowRight: 7,
      ArrowUp: -1,
      ArrowDown: 1,
    };

    function keys(event: KeyboardEvent): void {
      let delta = moves[event.key];
      const here = current.cursor();
      if (event.key === 'Home') delta = -here;
      else if (event.key === 'End') delta = current.count() - 1 - here;
      if (delta === undefined) return;
      event.preventDefault();
      current.setCursor(Math.max(0, Math.min(current.count() - 1, here + delta)));
      current.setHover(-1);
    }

    function over(event: MouseEvent): void {
      const dot = (event.target as HTMLElement).closest?.('[data-i]');
      current.setHover(dot ? Number(dot.getAttribute('data-i')) : -1);
    }

    function out(): void {
      current.setHover(-1);
    }

    node.addEventListener('keydown', keys);
    node.addEventListener('mouseover', over);
    node.addEventListener('mouseleave', out);
    return {
      update(next: typeof api) {
        current = next;
      },
      destroy() {
        node.removeEventListener('keydown', keys);
        node.removeEventListener('mouseover', over);
        node.removeEventListener('mouseleave', out);
      },
    };
  }

  /** Four steps: nothing logged, then three ink rungs against the plan's own
      daily aim (see the note at the top of this file). */
  function level(minutes: number): 0 | 1 | 2 | 3 {
    if (minutes <= 0) return 0;
    const third = aimMin && aimMin > 0 ? aimMin / 3 : 35;
    if (minutes < third) return 1;
    if (minutes < third * 2) return 2;
    return 3;
  }

  const columns = $derived(Math.ceil(term.length / 7));

  /** The month rule: a label over the first column whose Monday opens a month. */
  const monthAt = $derived.by(() => {
    const marks: Array<string | null> = [];
    let last = '';
    for (let column = 0; column < columns; column += 1) {
      const first = term[column * 7];
      const month = first ? first.month : '';
      marks.push(month !== last ? month : null);
      last = month;
    }
    return marks;
  });

  const caption = $derived(
    hovered >= 0 ? (term[hovered]?.fact ?? termNote) : focused ? (at?.fact ?? termNote) : termNote,
  );

  const sr = $derived(focused && at ? `${termNote} ${at.fact}.` : `${termNote}.`);

  const gridLabel = $derived(
    term.length > 0
      ? `A dot for each day of the term, ${term[0].when} to ${term[term.length - 1].when}; the darker the dot, the more minutes logged`
      : 'A dot for each day of the term',
  );
</script>

<div class="v-fit dm">
  <section class="cd-card dm-term">
    <header class="cd-card__head">
      <div>
        <h2 class="cd-card__title">The term</h2>
        <p class="cd-card__sub dm-cap">{caption}</p>
      </div>
    </header>

    <!-- One tab stop with a ring cursor, not 112 buttons: the lab's own trade,
         and the dots' values are reachable in the caption and in the sentence
         below. The grid's own label carries the reading for a screen reader. -->
    <div
      class="dm-grid"
      tabindex="0"
      role="group"
      aria-label={gridLabel}
      use:gridKeys={{
        cursor: () => cursor,
        count: () => term.length,
        setCursor: (index) => (cursor = index),
        setHover: (index) => (hovered = index),
      }}
      onfocus={() => (focused = true)}
      onblur={() => {
        focused = false;
        hovered = -1;
      }}
    >
      <!-- the day gutter and the month row sit in the same tracks as the dots,
           so the labels cannot drift from the marks they name; both decorative -->
      <span class="dm-days" style="grid-column: 1; grid-row: 2" aria-hidden="true">Mon</span>
      <span class="dm-days" style="grid-column: 1; grid-row: 8" aria-hidden="true">Sun</span>
      {#each monthAt as month, column (column)}
        {#if month}
          <span
            class="dm-months"
            style={`grid-column: ${column + 2}; grid-row: 1`}
            aria-hidden="true"
          >{month}</span>
        {/if}
      {/each}
      {#each term as day, index (day.date)}
        <span
          class="dm-dot"
          data-i={index}
          data-lvl={level(day.minutes)}
          data-cur={index === cursor && focused ? '' : undefined}
          style={`--d: ${Math.abs(3 - (index % 7)) * 26 + Math.floor(index / 7) * 10}ms; grid-column: ${Math.floor(index / 7) + 2}; grid-row: ${(index % 7) + 2}`}
        ></span>
      {/each}
    </div>

    <p class="cd-sr">{sr}</p>
  </section>

  <section class="cd-card dm-week">
    <header class="cd-card__head">
      <div>
        <h2 class="cd-card__title">This week</h2>
      </div>
    </header>
    <div class="dm-rows">
      {#each figures as figure (figure.key)}
        <div class="dm-row" data-fig={figure.key}>
          <span class="dm-row__l">{figure.label}</span>
          <span class="dm-row__v num">{figure.value}</span>
          <span class="dm-row__of">{figure.ofText}</span>
          <span class="dm-row__d">{figure.delta}</span>
        </div>
      {/each}
    </div>
  </section>
</div>

<style>
  /* ══ C · THE DOT MATRIX ═════════════════════════════════════════════════ */

  /* The design switch takes the head's right end; this head has no spacer. */

  .dm {
    display: grid;
    gap: var(--ui-gap-lg);
  }
  .dm-term { padding: calc(var(--space-xl) * var(--ui-s)); }

  /* Sixteen columns of seven. The track sizes are the drawing; they scale with
     the size register and, below the width the grid needs, with the container —
     a fixed 24px dot set is 674px wide and would overflow the record panel. */
  .dm-grid {
    --dm-dot: min(calc(24px * var(--ui-s)), 3.5cqw);
    --dm-gap: min(calc(16px * var(--ui-s)), 2.3cqw);
    display: grid;
    grid-template-columns: min(calc(34px * var(--ui-s)), 4.6cqw) repeat(16, var(--dm-dot));
    grid-template-rows: min(calc(16px * var(--ui-s)), 2.2cqw) repeat(7, var(--dm-dot));
    gap: var(--dm-gap);
    justify-content: center;
    margin-top: calc(var(--space-md) * var(--ui-s));
    outline-offset: var(--space-xs);
  }
  .dm-months,
  .dm-days {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .dm-months { align-self: start; white-space: nowrap; }
  .dm-days { align-self: center; }
  .dm-dot {
    display: block;
    border-radius: var(--r-pill);
    background: var(--ink);
    transition: background var(--dur-2) var(--ease);
    animation: dm-in var(--dur-3) var(--ease) both;
    animation-delay: var(--d, 0ms);
  }
  @keyframes dm-in {
    from { transform: scale(0.45); opacity: 0; }
    to { transform: none; opacity: 1; }
  }
  /* the four steps of the scale: nothing logged, then three ink rungs. The
     empty day is a hollow ring in `--ink-4` (the token's own job: a glyph or a
     dashed outline, never a word). */
  .dm-dot[data-lvl='0'] { background: transparent; box-shadow: inset 0 0 0 1px var(--ink-4); }
  .dm-dot[data-lvl='1'] { background: var(--ink-3); }
  .dm-dot[data-lvl='2'] { background: var(--ink-2); }
  .dm-dot[data-lvl='3'] { background: var(--ink); }
  /* the cursor: one ring, moved by hover or by the arrow keys — the keyboard
     ring only exists while the grid actually holds focus */
  .dm-dot:hover { outline: 2px solid var(--focus); outline-offset: 3px; }
  .dm-grid:focus-visible .dm-dot[data-cur] { outline: 2px solid var(--focus); outline-offset: 3px; }

  .dm-week { padding: 0; overflow: clip; }
  .dm-week .cd-card__head { padding: calc(var(--space-xl) * var(--ui-s)) calc(var(--space-lg) * var(--ui-s)) 0; }
  .dm-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 212px 264px;
    align-items: center;
    gap: calc(var(--space-md) * var(--ui-s));
    min-height: calc(52px * var(--ui-s));
    padding: 0 calc(var(--space-lg) * var(--ui-s));
  }
  .dm-row + .dm-row { border-top: 1px dashed var(--rule-strong); }
  .dm-row__l {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
  }
  .dm-row__v {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    text-align: right;
  }
  .dm-row__of {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .dm-row__d {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    text-align: right;
  }

  /* The four-row register folds in two, then stacks: the words are the content
     and the row is a sentence, so it reflows rather than clips. */
  @container (max-width: 720px) {
    .dm-row { grid-template-columns: minmax(0, 1fr) auto; row-gap: calc(var(--space-2xs) * var(--ui-s)); }
    .dm-row__l { grid-column: 1; }
    .dm-row__v { grid-column: 2; }
    .dm-row__of { grid-column: 1; }
    .dm-row__d { grid-column: 2; }
  }

  /* base.css already collapses every animation and transition to 1ms here;
     said once more for this component so the refusal is explicit in the file
     that adds the motion. */
  @media (prefers-reduced-motion: reduce) {
    .dm-dot { animation: none; }
  }
</style>
