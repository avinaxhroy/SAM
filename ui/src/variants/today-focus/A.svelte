<!-- Today focus variant A: Slide to Begin. Interactive draggable start knob. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import Duration from './Duration.svelte';
  import type { TodayFocusProps } from './props';

  let {
    greeting,
    dateLine,
    mode,
    title,
    course,
    tag,
    why,
    studiedToday,
    minutes,
    onMinutes,
    sessionCommand,
    onStart,
    onPlan,
  }: TodayFocusProps = $props();

  const reduced = typeof window !== 'undefined' ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;

  const ARM = 0.92;
  const GAP = 12; // the gap the knob stops short of the capsule by

  let band = $state<HTMLDivElement | null>(null);
  let pull = $state<HTMLButtonElement | null>(null);

  /** The travel in px, its normalized value, and the pointer inside the knob. */
  let tx = $state(0);
  let p = $state(0);
  let gx = $state<number | null>(null);
  /** Whether the inline values are written (the stylesheet owns `--tx` at rest). */
  let inline = $state(false);
  let armed = $state(false);
  let dragging = $state(false);
  let started = $state(false);

  let drag: { id: number; x: number; tx: number } | null = null;
  let justDragged = false;
  let settle = 0;

  const bandStyle = $derived(
    [
      inline ? `--tx:${tx.toFixed(1)}px` : '',
      inline ? `--p:${p.toFixed(3)}` : '',
      gx !== null ? `--gx:${gx.toFixed(1)}px` : '',
    ]
      .filter(Boolean)
      .join(';'),
  );

  function pillH(): number {
    if (!band) return 46;
    const value = parseFloat(getComputedStyle(band).getPropertyValue('--pill-h-lg'));
    return Number.isFinite(value) && value > 0 ? value : 46;
  }

  /** The band's own width minus the capsule's real width and the knob's rest. */
  function maxTx(): number {
    if (!band) return 0;
    const capsule = band.querySelector<HTMLElement>('.cd-dur');
    const capW = capsule ? capsule.getBoundingClientRect().width : 0;
    return Math.max(0, band.clientWidth - capW - GAP - pillH());
  }

  /** `--tx` is read from the COMPUTED style: the stylesheet sets it too (the
      hover lean), and a drag must start where the knob already is. */
  function txNow(): number {
    if (!band) return 0;
    return parseFloat(getComputedStyle(band).getPropertyValue('--tx')) || 0;
  }

  function paint(value: number): void {
    const max = maxTx();
    tx = Math.max(0, Math.min(max, value));
    p = max > 0 ? Math.min(1, tx / max) : 0;
    armed = p >= ARM;
    inline = true;
  }

  function clearInline(): void {
    inline = false;
    tx = 0;
    p = 0;
    armed = false;
    gx = null;
  }

  function reset(): void {
    window.clearTimeout(settle);
    dragging = false;
    started = false;
    drag = null;
    clearInline();
  }

  function walkBack(): void {
    window.clearTimeout(settle);
    paint(0);
    settle = window.setTimeout(clearInline, 280);
  }

  /** The commit is a state, not a tween: the knob sweeps to the end, holds,
      and settles back — and the session starts. */
  function commit(): void {
    started = true;
    tx = maxTx();
    p = 1;
    armed = true;
    inline = true;
    window.clearTimeout(settle);
    settle = window.setTimeout(reset, 450);
    onStart();
  }

  /** A tap that is not a slide is not an activation: it nudges the fill so the
      control says what it wants, and starts nothing. */
  function nudge(): void {
    if (reduced?.matches) return;
    window.clearTimeout(settle);
    paint(16);
    settle = window.setTimeout(walkBack, 150);
  }

  function mark(clientX: number): void {
    if (!band || !pull) return;
    const rect = band.getBoundingClientRect();
    gx = Math.max(2, Math.min(pull.offsetWidth - 2, clientX - rect.left));
  }

  function onPointerDown(event: PointerEvent): void {
    if (!sessionCommand) return;
    if (event.pointerType === 'mouse' && event.button !== 0) return;
    window.clearTimeout(settle);
    started = false;
    drag = { id: event.pointerId, x: event.clientX, tx: txNow() };
    justDragged = false;
    dragging = true;
    try {
      pull?.setPointerCapture(event.pointerId);
    } catch {
      /* a pointer that cannot be captured still drags in bounds */
    }
    paint(drag.tx);
    mark(event.clientX);
  }

  function onPointerMove(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.id) return;
    const dx = event.clientX - drag.x;
    if (Math.abs(dx) > 3) justDragged = true;
    paint(drag.tx + dx);
    mark(event.clientX);
  }

  function endDrag(): void {
    if (!drag) return;
    drag = null;
    dragging = false;
    if (armed) {
      justDragged = true;
      commit();
      return;
    }
    walkBack();
  }

  function onPullClick(event: MouseEvent): void {
    dragging = false;
    if (justDragged) {
      justDragged = false;
      return;
    }
    if (started) return;
    if (event.detail === 0) {
      commit();
      return;
    }
    nudge();
  }

  function onBandClick(event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    if (target?.closest('.xa-pull') || target?.closest('.cd-dur')) return;
    if (justDragged) {
      justDragged = false;
      return;
    }
    nudge();
  }

  function onKeyDown(event: KeyboardEvent): void {
    const step = maxTx() / 5;
    const current = txNow();
    if (event.key === 'ArrowRight' || event.key === 'ArrowUp') {
      event.preventDefault();
      paint(current + step);
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') {
      event.preventDefault();
      paint(current - step);
    } else if (event.key === 'End') {
      event.preventDefault();
      paint(maxTx());
    } else if (event.key === 'Home') {
      event.preventDefault();
      paint(0);
    }
  }

  // A new state is a new object: the knob goes back to rest.
  $effect(() => {
    void mode;
    reset();
  });

  /* The band is a track, not a control: the pointer the knob follows, the
     gesture a tap teaches and the specular are tracked with real listeners so
     the track itself stays a plain box in the accessibility tree (a static
     element with pointer handlers needs a role, and the only actable thing in
     the band is the knob). */
  $effect(() => {
    const el = band;
    if (!el) return;
    const move = (event: PointerEvent) => {
      if (dragging) return;
      if (reduced?.matches) return;
      mark(event.clientX);
    };
    const leave = () => {
      if (!dragging) gx = null;
    };
    const click = (event: MouseEvent) => onBandClick(event);
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerleave', leave);
    el.addEventListener('click', click);
    return () => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerleave', leave);
      el.removeEventListener('click', click);
    };
  });
</script>

<div class="v-fit tfa">
  <!-- Card header with greeting and date, shared by active and clear card states. -->
  {#snippet masthead()}
    <header class="cd-card__head">
      <span class="cd-ictile"><Icon name="calendar" /></span>
      <div>
        <h1 class="cd-card__title">{greeting}</h1>
        <p class="cd-card__sub">{dateLine}</p>
      </div>
    </header>
  {/snippet}

  {#if mode === 'clear'}
    <div class="tf-plank xa-plank">
      {@render masthead()}
      <div class="tf-id">
        <p class="tf-eyebrow">
          <span class="tf-mark" aria-hidden="true"><Icon name="check" size={13} /></span>
        </p>
        <h2 class="tf-title tw-focus__text">Nothing waiting</h2>
        <p class="tf-why">
          <span class="tf-why__k">Studied today</span>
          <span><b>{studiedToday}</b></span>
        </p>
      </div>
      <div class="tf-acts">
        <button class="cd-pill cd-pill--lg cd-pill--quiet" type="button" onclick={onPlan}>Plan a session</button>
      </div>
    </div>
  {:else}
    <div class="cd-card xa-card">
      {@render masthead()}
      <div class="tf-id">
        <p class="tf-eyebrow">
          {#if course}
            <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.label}</span>
          {/if}
          {#if tag}
            <span class="tf-tag">{tag}</span>
          {/if}
        </p>
        <h2 class="tf-title tw-focus__text" title={title ?? undefined}>{title}</h2>
        {#if why}
          <p class="tf-why">
            <span class="tf-why__k">{why.key}</span>
            <span><b>{why.value}</b></span>
          </p>
        {/if}
      </div>

      <div
        class="xa-band"
        bind:this={band}
        style={bandStyle}
        data-drag={dragging ? '1' : undefined}
        data-arm={armed ? '1' : undefined}
        data-started={started ? '1' : undefined}
      >
        <button
          class="xa-pull"
          type="button"
          bind:this={pull}
          data-command={sessionCommand ?? undefined}
          data-placement={sessionCommand ? 'today.screen' : undefined}
          disabled={!sessionCommand}
          title={sessionCommand ? 'Start a session on this' : 'This plan has no kind that records minutes'}
          aria-label={`Start the session on ${title ?? 'the next thing'}`}
          onpointerdown={onPointerDown}
          onpointermove={onPointerMove}
          onpointerup={endDrag}
          onpointercancel={endDrag}
          onclick={onPullClick}
          onkeydown={onKeyDown}
        >
          <svg class="xa-pull__ico xa-pull__ico--go" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" aria-hidden="true" focusable="false">
            <path d="M4 12h9" stroke-width="4" stroke-dasharray="0 5.4" />
            <path d="M13.2 6.6 19 12l-5.8 5.4" stroke-width="2" />
          </svg>
          <svg class="xa-pull__ico xa-pull__ico--ok" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
            <path d="M5 12.6 9.6 17 19 7.4" />
          </svg>
        </button>
        <span class="xa-pull__t" aria-hidden="true">Start</span>
        <Duration raised {minutes} onCommit={onMinutes} />
      </div>
    </div>
  {/if}
</div>

<style>
  .tfa {
    display: grid;
    align-content: start;
  }
  /* Span full card width and let container row-gap handle bottom spacing. */
  .xa-card .cd-card__head,
  .xa-plank .cd-card__head {
    grid-column: 1 / -1;
    margin-bottom: 0;
  }

  /* ── the object · four slots ────────────────────────────────────────── */
  .tf-id {
    display: grid;
    align-content: start;
    justify-items: start;
    gap: var(--ui-gap);
  }
  .tf-eyebrow {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin: 0;
  }
  .tf-tag {
    font-size: var(--text-2xs);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .tf-title {
    margin: 0;
    font-size: calc(var(--text-3xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.08;
  }
  /* The clamp clips at its own padding box and the display leading is tighter
     than the type's ink, so the ink gets the room as padding and the layout
     pays it back as a negative margin. */
  .tf-title.tw-focus__text {
    padding-block: 0.14em;
    margin-block: -0.14em;
  }
  .tf-why {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
    margin: 0;
    font-size: calc(var(--text-sm) * var(--ui-s));
    color: var(--ink-2);
  }
  .tf-why__k {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .tf-why b {
    color: var(--ink);
    font-weight: var(--weight-label);
    font-variant-numeric: tabular-nums;
  }

  /* the action slot: one height in every state, so the fourth slot cannot
     drift apart from the other states of the same surface */
  .tf-acts {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-height: calc(var(--pill-h-lg) + var(--space-md));
  }

  /* ── the all-clear · the plank ──────────────────────────────────────── */
  .tf-plank {
    display: grid;
    align-content: start;
    gap: var(--ui-gap);
    padding: var(--space-xl) var(--space-2xl);
    background: var(--well);
    border-radius: var(--r-card);
  }
  .tf-mark {
    width: var(--chip-h);
    height: var(--chip-h);
    flex: none;
    border-radius: var(--r-pill);
    background: var(--card);
    color: var(--ink-2);
    display: grid;
    place-items: center;
  }
  /* A quiet pill on a recessed plank would be a second recess: on this surface
     the quiet recipe turns to the card. */
  .tf-plank :global(.cd-pill--quiet) {
    background: var(--card);
  }
  .tf-plank :global(.cd-pill--quiet:hover) {
    background: var(--card);
    box-shadow: var(--sh-1);
  }

  /* ── A · the slide band ─────────────────────────────────────────────── */
  .xa-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) min(420px, 42%);
    align-items: center;
    column-gap: var(--space-2xl);
    padding: var(--space-xl) var(--space-2xl);
  }
  .xa-band {
    --tx: 0px;
    --p: 0;
    position: relative;
    display: flex;
    align-items: center;
    height: var(--pill-h-lg);
    border-radius: var(--r-pill);
    background: var(--well);
  }
  /* the lean: the pointer arriving is the first half of the lesson */
  .xa-band:hover:not([data-drag='1']) {
    --tx: 12px;
  }
  .xa-pull {
    position: relative;
    z-index: 1;
    flex: none;
    width: calc(var(--pill-h-lg) + var(--tx));
    height: var(--pill-h-lg);
    border: 0;
    padding: 0;
    border-radius: var(--r-pill);
    background-color: var(--ink);
    background-image: radial-gradient(
      42px 42px at var(--gx, 50%) 46%,
      color-mix(in oklab, var(--ink-inv) 30%, transparent),
      transparent 74%
    );
    color: var(--ink-inv);
    cursor: grab;
    touch-action: none;
    -webkit-user-select: none;
    user-select: none;
    transition:
      width var(--dur-2) var(--ease),
      transform var(--dur-1) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .xa-pull:disabled {
    cursor: default;
  }
  /* while the hand leads, the width is not a transition — it is the pointer */
  .xa-band[data-drag='1'] .xa-pull {
    transition: transform var(--dur-1) var(--ease);
    cursor: grabbing;
  }
  .xa-band[data-arm='1'] .xa-pull {
    box-shadow: var(--sh-ink);
  }
  .xa-pull__ico {
    position: absolute;
    top: 0;
    bottom: 0;
    right: 14px;
    margin: auto;
    width: 18px;
    height: 18px;
    transition:
      opacity var(--dur-2) var(--ease),
      transform var(--dur-2) var(--ease);
  }
  .xa-pull__ico--ok {
    opacity: 0;
    transform: translateX(-160%);
  }
  .xa-band[data-arm='1'] .xa-pull__ico--go {
    opacity: 0;
    transform: translateX(160%);
  }
  .xa-band[data-arm='1'] .xa-pull__ico--ok {
    opacity: 1;
    transform: translateX(0);
  }
  .xa-pull__t {
    position: absolute;
    left: calc(var(--pill-h-lg) + var(--space-sm));
    top: 50%;
    transform: translateY(-50%);
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-2);
    opacity: clamp(0, calc(1 - var(--p) * 6), 1);
    transition: opacity var(--dur-2) var(--ease);
    pointer-events: none;
  }
  .xa-band :global(.cd-dur) {
    position: relative;
    z-index: 1;
    margin-left: auto;
  }

  /* the all-clear is the same two tracks, so the quiet pill stands where the
     knob stands in the other two states */
  .xa-plank {
    grid-template-columns: minmax(0, 1fr) min(420px, 42%);
    align-items: center;
    column-gap: var(--space-2xl);
  }
  .xa-plank .tf-acts {
    align-items: center;
    justify-content: flex-start;
  }

  /* base.css already collapses every transition under reduced motion; this is
     the page's own statement of the same intent. */
  @media (prefers-reduced-motion: reduce) {
    .xa-band[data-drag='1'] .xa-pull {
      transition: none;
    }
  }

  /* One narrow fallback: when the card cannot hold two tracks, they stack, so
     the band takes the card's width instead of squeezing the identity away. */
  @container (max-width: 640px) {
    .xa-card,
    .xa-plank {
      grid-template-columns: minmax(0, 1fr);
      row-gap: var(--ui-gap);
    }
  }
</style>
