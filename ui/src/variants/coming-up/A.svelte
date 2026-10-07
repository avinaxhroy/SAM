<!-- Coming up variant A: The Deck. Layered carousel with swipe and arrow navigation. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { leadOf, type ComingUpProps } from './props';

  let { rows, planReady, onOpenPlan }: ComingUpProps = $props();

  const N = $derived(rows.length);
  const REDUCE = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /** Transition timing in ms for card exit. */
  const EXIT = 180;
  /** Gesture velocity/distance thresholds for swipe navigation. */
  const SWIPE = 90;
  const FLING = 450;
  /** Card container width for drag resistance calculation. */
  const W = 620;

  /** The committed slot (what the layout draws) and the card the chrome speaks
      about. They differ only during beat one. */
  let at = $state(0);
  let front = $state(0);
  /** The card in beat one and how it was sent there, or null. */
  let exiting = $state<{ index: number; dir: number; carry: number } | null>(null);
  let flying = $state(false);
  /** The peel: the front card's own two numbers while a finger is down or a
      spring is still returning it. */
  let peel = $state<{ index: number; dx: number; shrink: number } | null>(null);

  let beat = 0;
  let springFrame = 0;

  // A new set of rows is a new list: the deck goes back to its first card.
  // (This effect reads only the rows — writing `at`/`front` while reading them
  // would re-enter itself.)
  $effect(() => {
    void rows;
    cancelAll();
    at = 0;
    front = 0;
    exiting = null;
  });

  // The one arrow that is never disabled is the one that can still move.
  const back = $derived(at === 0 && !flying);
  const forth = $derived(at === N - 1 && !flying);
  const first = $derived(Math.min(front, Math.max(0, N - 1)));

  /** What the live region reads out: the card's own words and its position. */
  const spoken = $derived.by(() => {
    const row = rows[first];
    if (!row) return '';
    const when = row.date
      ? row.date.time
        ? `${row.date.long}, ${row.date.time}`
        : row.date.long
      : leadOf(row);
    const words = [row.label, when, row.state?.word].filter((word): word is string => Boolean(word));
    return words.length ? `${words.join(', ')} — ${first + 1} of ${N}` : `${first + 1} of ${N}`;
  });

  /** How far a flung card keeps travelling, in the direction it was sent. */
  function throwOf(dir: number, carry: number): number {
    const travel = Math.abs(carry) + SWIPE + Math.abs(carry) * 0.12;
    return (dir > 0 ? -1 : 1) * Math.min(travel, dir > 0 ? 520 : 300);
  }

  /** Past an end the card still answers the pointer, with a resistance that
      grows the further it is pulled and can never reach the deck's width. */
  function rubber(over: number): number {
    return (over * W * 0.55) / (W + 0.55 * over);
  }

  function cancelAll(): void {
    clearTimeout(beat);
    cancelAnimationFrame(springFrame);
    springFrame = 0;
    peel = null;
  }

  /** The one transport: the arrows, the arrow keys and a committed flick all
      take this step. */
  function step(dir: number, carry = 0): void {
    const to = front + dir;
    if (to < 0 || to >= N || flying) return;
    if (REDUCE) {
      cancelAll();
      at = to;
      front = to;
      return;
    }
    cancelAll();
    flying = true;
    exiting = { index: at, dir, carry };
    front = to;
    clearTimeout(beat);
    beat = setTimeout(() => {
      at = to;
      exiting = null;
      flying = false;
    }, EXIT);
  }

  function onKey(event: KeyboardEvent): void {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();
    step(event.key === 'ArrowRight' ? 1 : -1);
  }

  /** The deck's two listeners, attached to the frame rather than written as
      template attributes: the drag is imperative pointer machinery (the lab
      attaches it the same way), and the arrow keys are the same transport the
      arrows take, reachable wherever focus already is inside the deck. */
  let wrapEl = $state<HTMLElement | null>(null);
  let deckEl = $state<HTMLElement | null>(null);

  $effect(() => {
    const wrap = wrapEl;
    const deck = deckEl;
    if (!wrap || !deck) return;
    wrap.addEventListener('keydown', onKey);
    deck.addEventListener('pointerdown', onPointerDown);
    return () => {
      wrap.removeEventListener('keydown', onKey);
      deck.removeEventListener('pointerdown', onPointerDown);
    };
  });

  function depthOf(index: number): number {
    return index - at;
  }

  /** Every card's whole placement, as the stack's own custom properties. */
  function cardStyle(index: number): string {
    const d = depthOf(index);
    const leaves = exiting !== null && exiting.index === index;
    const vis = leaves ? (exiting.dir > 0 ? -2 : 1) : d < 0 ? -2 : Math.min(d, 4);
    const o = leaves ? (exiting.dir > 0 ? 0 : 1) : d < 0 ? 0 : d > 3 ? 0 : 1;
    const z = leaves ? 30 : flying && index === front ? 21 : d < 0 ? 12 : 20 - d;
    const peels = peel !== null && peel.index === index && index === at;
    const dx = leaves && exiting.carry ? exiting.carry : peels ? peel.dx : 0;
    const shrink = peels ? peel.shrink : 0;
    return `--vis: ${vis}; --o: ${o}; z-index: ${z}; --dx: ${dx.toFixed(2)}px; --shrink: ${shrink.toFixed(4)}`;
  }

  /* ── the peel ────────────────────────────────────────────────────────────
     The drag answers the pointer frame by frame, so it is not transitioned
     while a finger is down; the moment it lifts, the card is either thrown (a
     flick, which takes the same step the arrows take) or sprung back. */
  let drag: { id: number; x0: number; vx: number; samples: { x: number; t: number }[]; el: HTMLElement } | null =
    null;

  function onPointerDown(event: PointerEvent): void {
    if (REDUCE || event.button !== 0 || flying) return;
    const card = (event.target as Element).closest('.cu-card');
    if (!card || card.getAttribute('data-front') !== '1' || exiting) return;
    const el = card as HTMLElement;
    cancelAll();
    drag = { id: event.pointerId, x0: event.clientX, vx: 0, samples: [{ x: event.clientX, t: performance.now() }], el };
    peel = { index: at, dx: 0, shrink: 0 };
    el.style.transition = 'none';
    el.setPointerCapture(event.pointerId);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);
  }

  function onPointerMove(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.id) return;
    const now = performance.now();
    let dx = event.clientX - drag.x0;
    // The release velocity comes off the last ~90ms of pointer samples, so one
    // slow frame cannot decide the throw.
    drag.samples.push({ x: event.clientX, t: now });
    while (drag.samples.length > 2 && now - drag.samples[0].t > 90) drag.samples.shift();
    const from = drag.samples[0];
    if (drag.samples.length > 1 && now - from.t >= 8) drag.vx = ((event.clientX - from.x) / (now - from.t)) * 1000;
    if (dx < 0 ? at === N - 1 : at === 0) dx = (dx < 0 ? -1 : 1) * rubber(Math.abs(dx));
    peel = { index: at, dx, shrink: Math.min(Math.abs(dx) / 280, 0.04) };
  }

  function onPointerUp(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.id) return;
    const { el, x0, vx } = drag;
    const dx = event.clientX - x0;
    el.removeEventListener('pointermove', onPointerMove);
    el.removeEventListener('pointerup', onPointerUp);
    el.removeEventListener('pointercancel', onPointerUp);
    drag = null;
    el.style.transition = '';
    const dir = (Math.abs(dx) > SWIPE ? dx : vx) < 0 ? 1 : -1;
    const to = at + dir;
    if ((Math.abs(dx) > SWIPE || Math.abs(vx) > FLING) && to >= 0 && to < N) {
      peel = null;
      step(dir, dx); // a flick: the deck's own step, carrying the pointer's throw
      return;
    }
    settle(dx, vx);
  }

  /** A card let go short of a flick: a spring carries the pointer's own velocity
      into it — it keeps travelling the way it was sent, then comes home. There
      is no duration here; the spring is stepped until it has stopped. */
  function settle(x: number, v: number): void {
    const index = at;
    let last = performance.now();
    let dx = x;
    cancelAnimationFrame(springFrame);
    const frame = (now: number) => {
      const dt = Math.min(0.032, Math.max(0.001, (now - last) / 1000));
      last = now;
      v += (-150 * dx - 16 * v) * dt;
      dx += v * dt;
      if (Math.abs(dx) < 0.4 && Math.abs(v) < 12) {
        peel = null;
        springFrame = 0;
        return;
      }
      peel = { index, dx, shrink: Math.min(Math.abs(dx) / 280, 0.04) };
      springFrame = requestAnimationFrame(frame);
    };
    springFrame = requestAnimationFrame(frame);
  }
</script>

<section class="v-fit cu-a">
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="flag" /></span>
    <div>
      <h2 class="cd-card__title">Coming up</h2>
      {#if N > 0}
        <p class="cd-card__sub">Dated things, by their own dates</p>
      {/if}
    </div>
  </header>

  {#if N === 0}
    <!-- The deck's own card at zero: the date tile drawn empty, and the one way
         out — the plan, which is where a date is put. -->
    <div class="cu-deckwrap">
      <div class="cu-deck cu-deck--void">
        <article class="cu-card cu-card--void">
          <span class="cu-date cu-date--void" aria-hidden="true"></span>
          <span class="cu-card__body">
            <span class="cu-void__t">Nothing dated yet</span>
            <span class="cu-void__s">Nothing in this plan carries a date — its work is placed by week.</span>
            <button
              class="cd-pill cd-pill--ghost cd-pill--sm"
              type="button"
              data-command="rail.select"
              data-placement="today.screen"
              disabled={!planReady}
              onclick={() => onOpenPlan()}
            >
              {planReady ? 'Open the plan' : 'The plan is not open'}
            </button>
          </span>
        </article>
      </div>
    </div>
  {:else}
    <div class="cu-deckwrap" role="group" aria-label="The dated deck" bind:this={wrapEl}>
      <div class="cu-deck" bind:this={deckEl} style={`--cu-depth: ${Math.min(N - 1, 3) * 10}px`}>
        {#each rows as row, index (row.id)}
          <article
            class="cu-card"
            data-front={index === at ? '1' : '0'}
            data-phase={exiting !== null && exiting.index === index ? 'exit' : undefined}
            aria-hidden={index !== at}
            style={cardStyle(index)}
          >
            <!-- The date tile: the deck's mark, a card wide — weekday · day ·
                 month, so a glance at the card's left edge is the whole date.
                 It carries the course's own wash, the identity the rest of the
                 app speaks, and the day is stated nowhere else on the card. -->
            <span class="cu-date" data-w={row.course?.wash} aria-hidden="true">
              {#if row.date}
                <span class="cu-date__wd">{row.date.weekday}</span>
                <span class="cu-date__n num">{row.date.day}</span>
                <span class="cu-date__m">{row.date.month}</span>
              {:else}
                <span class="cu-date__wd">no</span>
                <span class="cu-date__m">date</span>
              {/if}
            </span>
            <span class="cu-card__body">
              {#if row.course}
                <span class="cd-chip cd-chip--code" data-w={row.course.wash}>{row.course.label}</span>
              {/if}
              <span class="cu-card__t">{row.label}</span>
              <span class="cu-due">
                {#if leadOf(row)}
                  <span class="cu-when num">{leadOf(row)}</span>
                  <span class="cu-due__sep" aria-hidden="true">·</span>
                {/if}
                {#if row.state}
                  <span class="cu-state" data-tone={row.state.tone}>{row.state.word}</span>
                {/if}
              </span>
            </span>
          </article>
        {/each}
      </div>

      <div class="cu-deckfoot">
        <button
          class="cu-arrow"
          type="button"
          aria-label="Previous dated thing"
          disabled={back}
          onclick={() => step(-1)}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 5.5 8.5 12 15 18.5" /></svg>
        </button>
        <p class="cu-pos num">{first + 1} of {N}</p>
        <button
          class="cu-arrow"
          type="button"
          aria-label="Next dated thing"
          disabled={forth}
          onclick={() => step(1)}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5" /></svg>
        </button>
      </div>

      <p class="cd-sr" aria-live="polite">{spoken}</p>
    </div>
  {/if}
</section>

<style>
  /* ── the deck's own motion, in one place ─────────────────────────────── */
  .cu-a {
    --cu-exit: 180ms; /* beat one: the card in the slot leaves it */
    --cu-step: 360ms; /* beat two: the deck takes the slot it left */
    --cu-throw: 360ms; /* a flicked card keeps its own direction */
    --cu-spring: linear(0 0%, 0.0352 4%, 0.1202 8%, 0.2307 12%, 0.3498 16%, 0.4664 20%, 0.5738 24%, 0.6685 28%, 0.749 32%, 0.8153 36%, 0.8685 40%, 0.91 44%, 0.9415 48%, 0.9646 52%, 0.9811 56%, 0.9923 60%, 0.9994 64%, 1.0036 68%, 1.0057 72%, 1.0063 76%, 1.006 80%, 1.005 84%, 1.0038 88%, 1.0025 92%, 1.0012 96%, 1 100%);
    --cu-out: cubic-bezier(0.85, 0, 1, 0.3); /* the leave: holds, then goes */
  }

  .cu-deckwrap {
    display: grid;
    justify-items: center;
    gap: var(--ui-gap-lg);
    /* The card that used to stand above this row is gone (owner's call,
       2026-09-30): the section head stands on the sheet and the deck below it
       is the object, so the wrap keeps only the deck's own slack and the head's
       margin above it does the spacing. */
    padding: 0 0 var(--ui-gap-sm);
  }
  .cu-deck {
    position: relative;
    width: 100%;
    max-width: min(620px, 100%);
    /* The stack reserves exactly the edges it has: three peers is the lab's
       620px deck, and a deck of two cards is 10px shorter rather than leaving a
       hole where the others would have been. */
    height: calc((232px + var(--cu-depth, 30px)) * var(--ui-s));
  }
  .cu-deck--void { height: calc(232px * var(--ui-s)); }

  .cu-card {
    position: absolute;
    top: 0;
    left: 0;
    display: grid;
    grid-template-columns: calc(104px * var(--ui-s)) minmax(0, 1fr);
    gap: var(--ui-gap-lg);
    align-items: center;
    width: 100%;
    height: calc(232px * var(--ui-s));
    padding: var(--ui-pad);
    background: var(--card);
    border-radius: var(--r-card);
    box-shadow: var(--sh-1);
    /* Depth is `transform` (the stack's own language); the peel is `translate`,
       so a drag is written frame by frame without touching the stack, and the
       two settle on their own clocks. */
    transform: translateY(calc(var(--vis, 0) * 10px)) scale(calc(1 - var(--vis, 0) * 0.02 - var(--shrink, 0)));
    translate: var(--dx, 0px);
    opacity: var(--o, 1);
    transition:
      transform var(--cu-step) var(--cu-spring),
      translate var(--cu-throw) var(--cu-spring),
      opacity var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
    will-change: transform, translate, opacity;
  }
  /* Beat one. The card in the slot leaves it alone — nothing else moves while
     it goes, so no two cards settle into the same slot at once. */
  .cu-card[data-phase='exit'] {
    transition:
      transform var(--cu-exit) var(--cu-spring),
      translate var(--cu-throw) var(--cu-spring),
      opacity var(--cu-exit) var(--cu-out),
      box-shadow var(--cu-exit) var(--cu-out);
  }
  /* The front card is the object; every other card is depth, and depth takes
     neither the pointer nor the reading order. */
  .cu-card[data-front='0'] { pointer-events: none; }
  .cu-card[data-front='1'] {
    box-shadow: var(--sh-2);
    cursor: grab;
    touch-action: pan-y;
  }
  .cu-card[data-front='1']:active { cursor: grabbing; }

  /* The date tile: the numeral of `.cd-datetile`, labelled. It is the deck's
     mark — 104 × 128, reading weekday / day / month — and the only place the
     day is stated, so the due line carries only the time. */
  .cu-date {
    display: grid;
    align-content: center;
    justify-items: center;
    gap: 2px;
    width: calc(104px * var(--ui-s));
    height: calc(128px * var(--ui-s));
    border-radius: var(--r-tile);
    background-color: var(--wash, var(--well));
    background-image: var(--wash-grad, none);
    color: var(--onwash, var(--ink));
  }
  .cu-date__wd,
  .cu-date__m {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
  }
  .cu-date__n {
    font-size: var(--text-display);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1;
  }

  .cu-card__body {
    display: grid;
    justify-items: start;
    gap: var(--ui-gap-sm);
    min-width: 0;
  }
  .cu-card__t {
    font-size: var(--text-xl);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.15;
  }

  /* The two pieces of writing every dated object carries. The due line is the
     time then the one state word; the date is on the tile, not repeated. */
  .cu-due {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-xs);
    color: var(--ink-3);
  }
  .cu-due__sep { color: var(--ink-3); }
  .cu-when { font-variant-numeric: tabular-nums; }
  .cu-state {
    font-weight: var(--weight-label);
    color: var(--ink-2);
  }
  .cu-state[data-tone='overdue'] { color: var(--on-overdue); }
  .cu-state[data-tone='risk'] { color: var(--on-risk); }

  /* The deck's own controls: two 32px arrows and the position between them —
     a deck you cannot fall off, because the ends are really disabled. */
  .cu-deckfoot {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
  }
  .cu-arrow {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border-radius: var(--r-pill);
    background: var(--well-2);
    color: var(--ink);
    transition:
      background var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .cu-arrow:hover { box-shadow: var(--sh-1); }
  .cu-arrow:active { transform: scale(0.94); }
  .cu-arrow:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .cu-arrow[disabled] {
    opacity: 0.45;
    pointer-events: none;
  }
  .cu-pos {
    min-width: 52px;
    text-align: center;
    font-size: var(--text-xs);
    color: var(--ink-3);
  }

  /* The empty state: the deck's card at zero, the tile drawn empty. */
  .cu-deck--void .cu-card,
  .cu-card--void { box-shadow: var(--sh-1); }
  .cu-date--void {
    background: none;
    background-image: none;
    border: 1.5px dashed var(--ink-4);
  }
  .cu-void__t {
    font-size: var(--text-lg);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .cu-void__s {
    font-size: var(--text-xs);
    color: var(--ink-3);
    max-width: 34ch;
    line-height: 1.4;
  }

  /* ── the component's own container ─────────────────────────────────────
     The Today column is about 360px wide, so the lab's 620px card arrives as a
     narrow one: the tile drops to a strip on the card's own top line and the
     title takes the step below it, and the deck grows by the height that costs
     it so the three edges stay visible. */
  @container (max-width: 520px) {
    .cu-card {
      grid-template-columns: minmax(0, 1fr);
      gap: var(--ui-gap-sm);
      align-content: center;
      height: calc(268px * var(--ui-s));
    }
    .cu-deck { height: calc((268px + var(--cu-depth, 30px)) * var(--ui-s)); }
    .cu-deck--void { height: calc(268px * var(--ui-s)); }
    .cu-date {
      grid-auto-flow: column;
      align-items: baseline;
      justify-content: start;
      justify-items: start;
      width: 100%;
      height: auto;
      padding: var(--ui-pad-sm) var(--ui-pad);
      border-radius: var(--r-mini);
    }
    .cu-date__n { font-size: var(--text-2xl); }
    .cu-card__t { font-size: var(--text-md); }
  }

  /* ── reduced motion: the deck changes instantly and the peel is inert ─── */
  @media (prefers-reduced-motion: reduce) {
    .cu-card,
    .cu-card[data-phase='exit'] { transition: none; }
    .cu-card { --shrink: 0 !important; }
  }
</style>
