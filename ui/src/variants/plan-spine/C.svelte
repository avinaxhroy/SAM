<!--
  PLAN SPINE · C · THE CAROUSEL.
  Carousel variant featuring a week chip ladder navigation rail alongside a
  staged week card with peek transitions between adjacent weeks.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '../../shell/Icon.svelte';
  import { readOf, type PlanProps } from './props';

  let {
    title,
    weeks,
    summary,
    selectedId,
    newTopicCommand,
    onSelect,
    onTopic,
    onNewTopic,
  }: PlanProps = $props();

  // Narrow container breakpoint.
  const NARROW_AT = 760;

  /**
   * Shortest signed offset around the cyclic week sequence.
   */
  function shortestOffset(index: number, current: number, len: number): number {
    if (len === 0) return 0;
    let offset = (((index - current) % len) + len) % len;
    if (offset > len / 2) offset -= len;
    return offset;
  }

  /** The week on the stage, and where it stands in the plan's order. */
  const openPosition = $derived(Math.max(0, weeks.findIndex((week) => week.id === selectedId)));
  const open = $derived(weeks[openPosition] ?? null);
  /** The one week the plan resolves today into, and its seat on the wheel. */
  const nowPosition = $derived(weeks.findIndex((week) => week.now));
  const now = $derived(nowPosition >= 0 ? weeks[nowPosition] : null);
  const hasPrev = $derived(open !== null && openPosition > 0);
  const hasNext = $derived(open !== null && openPosition < weeks.length - 1);

  /** The stage's own name: the week and its range, never an invented date. */
  const stageLabel = $derived(
    open === null ? '' : open.dates === null ? open.name : `${open.name} · ${open.dates}`,
  );

  /** The card's own label, and the ladder's fallback line — both written as one
   *  string, because the compiler trims a space that opens an `{#if}` block and
   *  “Week 2· now” is not a label. */
  const eyebrow = $derived(open === null ? '' : open.now ? `${open.name} · now` : open.name);
  const nowLine = $derived(
    now === null
      ? ''
      : now.dates === null
        ? `Now · W${now.index}`
        : `Now · W${now.index} · ${now.dates}`,
  );

  /** The container's own width, and the register's multiplier, both read off
   *  the container: the wheel is only in play on the wide layout, and the seat
   *  the card arrives from is written in px by the JS below and drawn in px by
   *  the CSS, so both read `--ui-s` rather than one guessing at the other. */
  let width = $state(0);
  let scale = $state(1);
  const wide = $derived(width > NARROW_AT);
  let root = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const element = root;
    if (element === null) return;
    const pass = () => {
      const next = element.clientWidth;
      if (next !== width) width = next;
      const declared = parseFloat(getComputedStyle(element).getPropertyValue('--ui-s').trim());
      const step = Number.isFinite(declared) && declared > 0 ? declared : 1;
      if (step !== scale) scale = step;
    };
    pass();
    const observer = new ResizeObserver(pass);
    observer.observe(element);
    return () => observer.disconnect();
  });

  // Hidden when current week is within the visible chip window.
  const hereHidden = $derived(
    !wide ||
      nowPosition < 0 ||
      Math.abs(shortestOffset(nowPosition, openPosition, weeks.length)) <= 3,
  );

  // Direction of the last navigation step (+1 forward, -1 backward).
  let seat = 1;
  let stepping = false;
  let booted = false;

  const chips: (HTMLButtonElement | null)[] = [];

  let settled: string | null = null;
  $effect(() => {
    if (selectedId === settled) return;
    settled = selectedId;
    booted = true;
    stepping = false;
  });

  let easing: ((progress: number) => number) | null = null;
  function easeOf(element: HTMLElement): (progress: number) => number {
    if (easing) return easing;
    const declared = getComputedStyle(element).getPropertyValue('--ease').trim();
    const parts = /cubic-bezier\(\s*([\d.]+)\s*,\s*(-?[\d.]+)\s*,\s*([\d.]+)\s*,\s*(-?[\d.]+)\s*\)/.exec(
      declared,
    );
    if (!parts) return (easing = (progress) => 1 - (1 - progress) ** 3);
    const [, x1, y1, x2, y2] = parts.map(Number);
    const at = (a: number, b: number, t: number) => ((1 - t) ** 2 * t * a + (1 - t) * t * t * b) * 3 + t ** 3;
    return (easing = (x) => {
      let low = 0;
      let high = 1;
      let t = x;
      for (let step = 0; step < 24; step += 1) {
        if (at(x1, x2, t) < x) low = t;
        else high = t;
        t = (low + high) / 2;
      }
      return at(y1, y2, t);
    });
  }

  // Duration in milliseconds from --dur-2, instant when reduced motion is preferred.
  function cardDuration(element: HTMLElement): number {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return 0;
    const declared = parseFloat(getComputedStyle(element).getPropertyValue('--dur-2').trim());
    return Number.isFinite(declared) ? declared : 220;
  }

  // Compute transform matrix for staged and ghosted cards.
  function seatTransform(direction: number, away: number): string {
    const x = (direction * 104 * scale * away).toFixed(2);
    const rotate = (direction * 3 * away).toFixed(3);
    const shrink = (1 - 0.15 * away).toFixed(4);
    return `translateX(${x}px) rotate(${rotate}deg) scale(${shrink})`;
  }

  /**
   * THE ARRIVING CARD stands where its neighbour's ghost stands and settles
   * into the frame — at FULL opacity throughout, because the card being left is
   * the one that fades: the stage is never empty for a frame and nothing pops in
   * at zero. Transform only; this transition never touches opacity.
   */
  function arrive(node: HTMLElement) {
    if (!booted || !stepping) return { duration: 0 };
    const direction = seat;
    return {
      duration: cardDuration(node),
      easing: easeOf(node),
      css: (t: number) => `transform: ${seatTransform(direction, 1 - t)}; z-index: 2;`,
    };
  }

  /**
   * THE LEAVING CARD travels to the seat on the other side of the stage and
   * dissolves into the ghost that now holds it, held above the arriving card for
   * the whole of that one movement. From its first frame it is the frame the
   * student was looking at and not a second reading of the week: `inert` and
   * `aria-hidden`, and untouchable while it travels.
   */
  function depart(node: HTMLElement) {
    const direction = -seat;
    node.toggleAttribute('inert', true);
    node.setAttribute('aria-hidden', 'true');
    return {
      duration: booted && stepping ? cardDuration(node) : 0,
      easing: easeOf(node),
      css: (t: number) =>
        `transform: ${seatTransform(direction, 1 - t)}; opacity: ${t.toFixed(3)}; z-index: 3; pointer-events: none;`,
    };
  }

  /**
   * Ask the panel for a week. `onSelect` is the only writer of the selection —
   * the ladder, the two nav buttons and the arrow keys all come through here, so
   * there is one selection and not four. `focusChip` carries focus to the week
   * that moved, which is where a keyboard user's next step starts.
   */
  async function select(id: string, focusChip: boolean): Promise<void> {
    const to = weeks.findIndex((week) => week.id === id);
    if (to < 0) return;
    if (to === openPosition) {
      if (focusChip) chips[to]?.focus({ preventScroll: true });
      return;
    }
    seat = shortestOffset(to, openPosition, weeks.length) < 0 ? -1 : 1;
    stepping = true;
    onSelect(id);
    if (focusChip) {
      await tick();
      chips[to]?.focus({ preventScroll: true });
    }
  }

  /** One week along the plan's order. */
  function step(delta: number): void {
    const next = weeks[openPosition + delta];
    if (next) void select(next.id, true);
  }

  /** ←/→ walk the term wherever focus is in the design; ↑/↓ are the ladder's
   *  own axis. Inside the card's scroller they are the scroller's keys, and the
   *  lab leaves them alone too. */
  function onKey(event: KeyboardEvent): void {
    const target = event.target;
    if (!(target instanceof Element) || target.closest('input, select, textarea, [contenteditable]') !== null) return;
    const inLadder = target.closest('.psc-ladder') !== null;
    const delta =
      event.key === 'ArrowRight'
        ? 1
        : event.key === 'ArrowLeft'
          ? -1
          : inLadder && event.key === 'ArrowDown'
            ? 1
            : inLadder && event.key === 'ArrowUp'
              ? -1
              : 0;
    if (delta === 0) return;
    event.preventDefault();
    step(delta);
  }
</script>

<div class="v-fit" bind:this={root}>
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">
        {summary.weeks} {summary.weeks === 1 ? 'week' : 'weeks'} · {summary.things}
        {summary.things === 1 ? 'thing' : 'things'} to get through · {summary.done} of {summary.things} done
        · {summary.unplaced} not placed in a week
      </p>
    </div>
  </header>

  <div class="psc-car" onkeydown={onKey}>
    <!-- THE LADDER. Every week is a chip, offset by its own shortest distance
         from the week being read: the list is keyed by the week's id and never
         rebuilt, so a step moves the chips instead of redrawing them. -->
    <div class="psc-ladder">
      <div class="psc-ladder__in" role="tablist" aria-orientation="vertical" aria-label="The term's weeks">
        {#each weeks as week, position (week.id)}
          {@const offset = shortestOffset(position, openPosition, weeks.length)}
          {@const far = wide && Math.abs(offset) > 3}
          <button
            class="psc-chip"
            type="button"
            role="tab"
            id={`psc-chip-${week.id}`}
            aria-selected={position === openPosition}
            tabindex={position === openPosition ? 0 : -1}
            data-on={position === openPosition ? '' : undefined}
            data-near={Math.abs(offset) === 1 ? '' : undefined}
            data-now={week.now ? '' : undefined}
            inert={far}
            style={`--off: ${offset}${far ? '; visibility: hidden' : ''}`}
            bind:this={chips[position]}
            onclick={() => void select(week.id, true)}
          >
            <span class="psc-chip__n num">W{week.index}</span>
            {#if week.dates}<span class="psc-chip__d num">{week.dates}</span>{/if}
            {#if week.now}<span class="psc-chip__now">now</span>{/if}
          </button>
        {/each}
      </div>

      <!-- “You are here”, kept on the ladder even when the wheel has carried
           the current week's own chip out of the box: never two of it, and
           never in the narrow layout, where the chip is in the list. -->
      <p class="psc-here" hidden={hereHidden}>
        <span class="psc-dot" aria-hidden="true"></span>
        {nowLine}
      </p>
    </div>

    <div
      class="psc-stage"
      role="tabpanel"
      aria-labelledby={open === null ? undefined : `psc-chip-${open.id}`}
      aria-label={stageLabel}
    >
      <div class="psc-stage__in">
        {#if hasPrev}
          <!-- A neighbour is a POSITION, not a reading: a blank card at the
               seat the card being left travels into. No text is ever drawn on
               one, at any distance, in either direction. -->
          <div class="psc-ghost" data-side="prev" aria-hidden="true"></div>
        {/if}

        {#if open !== null}
          {#key open.id}
            <article class="psc-card" tabindex="-1" in:arrive out:depart>
              <header class="psc-card__head">
                <p class="psc-card__eyebrow">{eyebrow}</p>
                <h2 class="psc-card__t">{open.dates ?? open.name}</h2>
                <p class="psc-card__fig num">{readOf(open)}</p>
              </header>

              <div class="psc-card__body" tabindex="0" role="region" aria-label={`Work in ${open.name}`}>
                {#if open.topics.length === 0}
                  <p class="psc-void">No topics in this week.</p>
                {:else}
                  {#each open.topics as topic (topic.id)}
                    <button
                      class="psc-row"
                      type="button"
                      data-command="record.panel"
                      data-placement="recordTable.rowContext"
                      data-done={topic.state === 'done' ? '1' : undefined}
                      onclick={() => onTopic(topic)}
                    >
                      <span class="psc-row__t">{topic.label}</span>
                      <span class="psc-row__s">{topic.kind ?? topic.stage}</span>
                      <span class="cd-sr">{topic.stage}</span>
                      <span class="psc-row__m num">{topic.minutes === null ? '' : `${topic.minutes} min`}</span>
                    </button>
                  {/each}
                {/if}
              </div>

              {#if newTopicCommand}
                <div class="psc-card__foot">
                  <button
                    class="psc-write"
                    type="button"
                    data-command={newTopicCommand}
                    data-placement="today.screen"
                    onclick={onNewTopic}
                  >
                    <Icon name="plus" size={13} />
                    New topic
                  </button>
                </div>
              {/if}
            </article>
          {/key}
        {/if}

        {#if hasNext}
          <div class="psc-ghost" data-side="next" aria-hidden="true"></div>
        {/if}
      </div>

      <div class="psc-nav">
        <button
          class="psc-nav__b"
          type="button"
          aria-label="The week before"
          disabled={!hasPrev}
          onclick={() => step(-1)}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"><path d="M14.5 5.5 8 12l6.5 6.5" /></svg
          >
        </button>
        <button
          class="psc-nav__b"
          type="button"
          aria-label="The week after"
          disabled={!hasNext}
          onclick={() => step(1)}
        >
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"><path d="M9.5 5.5 16 12l-6.5 6.5" /></svg
          >
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  /* ══ THE CAROUSEL ═══════════════════════════════════════════════════════
     The lab's geometry, drawn from the size register: the ladder is 34% of the
     row on `--well`, its chips are 40px pills on a 44px step, the stage centres
     one card capped at 470 (112 of the width and 72 of the height stay for the
     ghosts), and the two step buttons are the register's own 32px.
     The lab's window replica (`.x-win`) is here the row's own height: the app's
     canvas is a page, and a percentage height over an auto parent is exactly the
     320px collapse the lab found and fixed by naming the host. */
  .psc-car {
    display: flex;
    height: calc(660px * var(--ui-s));
    overflow: clip;
    overscroll-behavior: contain;
  }

  /* ── the ladder ───────────────────────────────────────────────────────── */
  .psc-ladder {
    position: relative;
    flex: 0 0 34%;
    min-width: 0;
    display: grid;
    place-items: center;
    overflow: clip;
    background: var(--well);
    box-shadow: inset -1px 0 0 var(--rule);
  }
  /* The wheel's own box: the ±3 chips around the week being read, at 44px a
     step. The step is the chip's 40px height plus its 4px of air, so it moves
     with the register exactly as the chip does. */
  .psc-ladder__in {
    position: relative;
    width: 100%;
    height: calc(320px * var(--ui-s));
  }
  /* The lab's “you are here” line: `--ink-2` because it sits on `--well`, and
     it is the ladder's floor rather than a second object on it. */
  .psc-here {
    position: absolute;
    inset-block-end: var(--space-md);
    inset-inline: var(--space-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-xs);
    margin: 0;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .psc-here[hidden] {
    display: none;
  }
  .psc-dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--ink);
  }

  /* A chip: `W6` and the week's range, positioned by its own offset from the
     week being read. That transform IS the wheel — one `--dur-2` per step, and
     the ladder itself is never rebuilt, so the chips keep their motion. */
  .psc-chip {
    --psc-step: calc(44px * var(--ui-s));
    position: absolute;
    left: 50%;
    top: 50%;
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    height: calc(40px * var(--ui-s));
    min-height: var(--hit);
    padding: 0 var(--space-lg);
    border: 0;
    border-radius: var(--r-pill);
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--rule-strong);
    color: var(--ink-3);
    font: inherit;
    font-size: calc(var(--text-xs) * var(--ui-s));
    white-space: nowrap;
    cursor: pointer;
    transform: translate(-50%, calc(-50% + var(--off, 0) * var(--psc-step)));
    transition: transform var(--dur-2) var(--ease), color var(--dur-1) var(--ease),
      background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .psc-chip:hover {
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1.5px var(--ink-3);
  }
  .psc-chip:active {
    background: var(--well-2);
  }
  .psc-chip:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  /* The reference's opacity ladder cannot be spent here — a faded chip cannot
     hold 4.5:1 — so distance reads as an ink step: the weeks either side of you
     are `--ink-2` behind a stronger ring, the ones beyond are `--ink-3`, and
     past ±3 the chip is out of the box entirely (the far flag in the markup). */
  .psc-chip[data-near] {
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1.5px var(--ink-3);
  }
  .psc-chip[data-near]:hover {
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink-2);
  }
  /* The week being read: the raised chip on the card. */
  .psc-chip[data-on] {
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--sh-1);
  }
  /* The week you are IN: the screen's ONE dark object, wherever the ladder has
     wheeled it. */
  .psc-chip[data-now] {
    background: var(--ink);
    color: var(--ink-inv);
    box-shadow: var(--sh-ink);
  }
  .psc-chip[data-now]:hover {
    background-image: linear-gradient(var(--fill-on-ink), var(--fill-on-ink));
    color: var(--ink-inv);
  }
  .psc-chip__n {
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .psc-chip__d {
    letter-spacing: var(--track-title);
  }
  .psc-chip__now {
    padding: 1px var(--space-xs);
    border-radius: var(--r-pill);
    background: var(--fill-on-ink);
    color: var(--ink-inv);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
  }

  /* ── the stage ────────────────────────────────────────────────────────── */
  .psc-stage {
    position: relative;
    flex: 1;
    min-width: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .psc-stage__in {
    position: relative;
    width: min(100% - calc(112px * var(--ui-s)), calc(470px * var(--ui-s)));
    height: min(100% - calc(72px * var(--ui-s)), calc(470px * var(--ui-s)));
  }
  /* The neighbours: blank cards, because a rotated sheet of text behind the
     active one is noise and its words could not hold contrast. They are
     positions — the seat the leaving card travels into and the seat the
     arriving card grows out of — and the seats are the card's own. */
  .psc-ghost {
    position: absolute;
    inset: 0;
    z-index: 1;
    border-radius: var(--r-card);
    background: var(--well-2);
    box-shadow: var(--sh-1);
  }
  .psc-ghost[data-side='prev'] {
    transform: translateX(calc(-104px * var(--ui-s))) rotate(-3deg) scale(0.85);
  }
  .psc-ghost[data-side='next'] {
    transform: translateX(calc(104px * var(--ui-s))) rotate(3deg) scale(0.85);
  }

  /* The card itself. NO transition on transform or opacity here: the movement
     is the `in:`/`out:` pair above, which writes the transform on every frame —
     a CSS transition over those writes would lag behind them and trail on the
     frame the card settles. */
  .psc-card {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    background: var(--card);
    border-radius: var(--r-card);
    box-shadow: var(--sh-2);
    overflow: clip;
  }
  .psc-card:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .psc-card__head {
    padding: var(--space-xl) var(--space-xl) var(--space-md);
  }
  .psc-card__eyebrow {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .psc-card__t {
    margin-top: var(--space-2xs);
    font-size: calc(var(--text-xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
  }
  .psc-card__fig {
    margin-top: var(--space-3xs);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* The card's line is 24 (the head sits on it); the rows and the write keep a
     declared 8 of recess, so their own fill has an edge to breathe on and the
     text still lands on the card's line (8 + 16 = 24). */
  .psc-card__body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 var(--space-xs) var(--space-xs);
  }
  .psc-card__body:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .psc-card__foot {
    margin-inline: calc(var(--space-xs) * -1);
    padding: var(--space-3xs) var(--space-xs) var(--space-xs);
    box-shadow: inset 0 1px 0 var(--rule-strong);
  }

  .psc-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(44px * var(--ui-s));
    padding: 0 var(--space-md);
    border: 0;
    border-radius: var(--r-item);
    background: none;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background var(--dur-1) var(--ease);
  }
  .psc-row + .psc-row {
    box-shadow: inset 0 1px 0 var(--rule-strong);
  }
  .psc-row:hover {
    background: var(--well);
  }
  .psc-row:active {
    background: var(--well-2);
  }
  .psc-row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* The name WRAPS rather than clipping — a long topic takes a second line and
     its facts follow it — and it says so itself: the app's collection layer
     clips a `__t` in other surfaces, so the wrap is declared here and not left
     to the absence of a rule. Only the two meta runs are kept whole. */
  .psc-row__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    white-space: normal;
  }
  .psc-row[data-done] .psc-row__t {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .psc-row__s {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }
  .psc-row__m {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: right;
    white-space: nowrap;
  }

  /* The card's own scroller holds a full-bleed line at its edges, so the empty
     week's line takes the card's line as its own padding instead. */
  .psc-void {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin: 0;
    padding: var(--space-md) var(--space-md);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* The write, at the card's foot: one ink step quieter than the rows above it,
     so it reads as the card's door rather than as a second action. */
  .psc-write {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    width: 100%;
    min-height: var(--hit);
    padding: var(--space-2xs) var(--space-xl);
    border: 0;
    border-radius: var(--r-item);
    background: none;
    font: inherit;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: left;
    cursor: pointer;
    transition: color var(--dur-1) var(--ease), background var(--dur-1) var(--ease);
  }
  .psc-write:hover {
    color: var(--ink);
    background: var(--well);
  }
  .psc-write:active {
    background: var(--well-2);
  }
  .psc-write:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  /* The stage's own two steps, in its lower corner — disabled at the term's two
     ends, where there is no week to step to. */
  .psc-nav {
    position: absolute;
    inset-block-end: var(--space-sm);
    inset-inline-end: var(--space-sm);
    display: flex;
    gap: var(--space-2xs);
    z-index: 3;
  }
  .psc-nav__b {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border: 0;
    border-radius: var(--r-mini);
    background: var(--card);
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong), var(--sh-1);
    cursor: pointer;
    transition: color var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .psc-nav__b:hover {
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink-3), var(--sh-1);
  }
  .psc-nav__b:active {
    background: var(--well);
  }
  .psc-nav__b:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .psc-nav__b:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .psc-nav__b:disabled:hover {
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong), var(--sh-1);
  }

  /* ── NARROW (the lab's own 760px probe, as a container query) ────────────
     The ladder becomes the term's list at the top of the panel: every chip
     stands still, full width and on the collection's own 32px, the wheel is out
     of play (the `far` flag is off in the markup too) and the ghosts are put
     away, because the card is the only thing on the stage. */
  @container (max-width: 760px) {
    .psc-car {
      flex-direction: column;
      height: calc(560px * var(--ui-s));
    }
    .psc-ladder {
      flex: none;
      display: block;
      height: calc(208px * var(--ui-s));
      overflow-x: hidden;
      overflow-y: auto;
      box-shadow: inset 0 -1px 0 var(--rule);
    }
    .psc-ladder__in {
      height: auto;
      width: 100%;
      display: grid;
      gap: 2px;
      padding: var(--space-xs) var(--space-sm);
    }
    .psc-chip {
      position: static;
      transform: none;
      width: 100%;
      height: var(--hit);
      justify-content: flex-start;
    }
    .psc-stage__in {
      width: min(100% - calc(32px * var(--ui-s)), calc(470px * var(--ui-s)));
      height: min(100% - calc(32px * var(--ui-s)), calc(470px * var(--ui-s)));
    }
    .psc-ghost {
      display: none;
    }
  }

  /* The chips are POSITIONED by their own offset, so nothing may move one of
     them on hover: the transform is pinned. Wide only — the narrow ladder
     stands its chips still, and a pin there would snap one back onto the wheel. */
  @media (prefers-reduced-motion: reduce) {
    @container (min-width: 761px) {
      .psc-chip:hover {
        transform: translate(-50%, calc(-50% + var(--off, 0) * var(--psc-step)));
      }
    }
  }
</style>
