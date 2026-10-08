<!-- Timer chrome variant A: The Notch Slab. Expands from the titlebar tools area. -->
<script lang="ts">
  import type { TimerProps } from './props';

  let {
    label,
    course,
    clock,
    paused,
    targetMin,
    command,
    placement,
    ended,
    onPause,
    onStop,
  }: TimerProps = $props();
  let root = $state<HTMLElement | null>(null);
  let slot = $state<HTMLElement | null>(null);
  /** `mini` (the resting capsule), `open` (the slab), `confirm` (the ask). */
  let mode = $state<'mini' | 'open' | 'confirm'>('mini');
  /** The measured room decides how much of the reading survives. */
  let room = $state<'wide' | 'mid' | 'tight'>('wide');
  /** Corner fillets, shown only when clearance permits on that side. */
  let flareLeft = $state(true);
  let flareRight = $state(true);
  let expanded = $state(false);

  /** Density modes, widest first. */
  const DENSITIES = ['wide', 'mid', 'tight'] as const;
  type Density = (typeof DENSITIES)[number];

  /** Measures required content width for an open row at a given density.
      Temporarily applies data-room to use CSS visibility rules, summing
      child scrollWidths so unconstrained text width is captured. */
  function readingWidth(element: HTMLElement, density: Density): number {
    const open = element.querySelector<HTMLElement>('.vtca__layer[data-slot="open"]');
    const row = open?.querySelector<HTMLElement>('.vtca__row');
    if (!open || !row) return 0;
    const was = element.dataset.room;
    element.dataset.room = density;
    const layer = getComputedStyle(open);
    const gap = parseFloat(getComputedStyle(row).columnGap) || 0;
    let width = parseFloat(layer.paddingLeft) + parseFloat(layer.paddingRight);
    let parts = 0;
    for (const part of Array.from(row.children) as HTMLElement[]) {
      if (getComputedStyle(part).display === 'none') continue;
      if (parts > 0) width += gap;
      width += part.scrollWidth;
      parts += 1;
    }
    if (was === undefined) delete element.dataset.room;
    else element.dataset.room = was;
    return Math.ceil(width);
  }

  /** Returns the widest density that fits within available width. */
  function densityFor(element: HTMLElement, width: number): Density {
    for (const density of DENSITIES) if (readingWidth(element, density) <= width) return density;
    return 'tight';
  }

  /** The measured width of the dead space beside the slot — the notch's room. */
  function measure(): void {
    const element = root;
    if (!element || !slot) return;
    const group = element.closest('.cd-titlebar')?.querySelector('.cd-titlebar__group');
    if (!group) return;
    const space = Math.round(slot.getBoundingClientRect().right - group.getBoundingClientRect().right - 12);
    /* Clamp width between 156px and available space, expanding past 440px
       if wide content needs extra room and space permits. */
    const width = Math.min(Math.max(156, space), Math.max(440, readingWidth(element, 'wide')));
    element.style.setProperty('--a-w', `${width}px`);
    /* Select density dynamically based on content width to avoid clipping. */
    room = densityFor(element, width);
    /* Left fillet needs 20px clearance to titlebar items; right fillet needs
       8px clearance to the next control to prevent overlapping it. */
    const next = element.nextElementSibling;
    const clearanceLeft = space + 12 - width;
    const clearanceRight = next ? next.getBoundingClientRect().left - slot.getBoundingClientRect().right : 0;
    flareLeft = clearanceLeft >= 20;
    flareRight = clearanceRight >= 8;
  }

  $effect(() => {
    const element = root;
    if (!element) return;
    measure();
    const observer = new ResizeObserver(() => measure());
    observer.observe(element);
    const bar = element.closest('.cd-titlebar');
    if (bar) observer.observe(bar);
    return () => observer.disconnect();
  });

  /* Re-measure when clock string length changes (e.g. digit added)
     or a target is set, rather than on every second tick. */
  $effect(() => {
    void clock.length;
    void targetMin;
    measure();
  });

  /** The slot opens on hover, on keyboard focus, on a pin, or for the ask. */
  function refresh(): void {
    expanded = mode !== 'mini';
  }
  function onEnter(): void {
    expanded = true;
  }
  function onLeave(): void {
    refresh();
  }
  function onFocusIn(): void {
    expanded = true;
  }
  function onFocusOut(): void {
    refresh();
  }

  function toggle(): void {
    mode = mode === 'open' ? 'mini' : 'open';
    refresh();
  }
  function ask(): void {
    mode = 'confirm';
    refresh();
  }
  function keepGoing(): void {
    mode = 'open';
    refresh();
  }
  function log(): void {
    onStop();
  }

</script>

{#if !ended}
    <div
      class="vtca"
      bind:this={root}
      data-state={paused ? 'paused' : 'run'}
      data-room={room}
      data-flare-l={flareLeft ? '1' : '0'}
      data-flare-r={flareRight ? '1' : '0'}
    >
    <div
      class="vtca__slot"
      bind:this={slot}
      data-mode={mode}
      role="group"
      aria-label="Study session"
      aria-expanded={expanded}
      tabindex="0"
      onclick={(event) => {
        if ((event.target as HTMLElement).closest('button')) return;
        toggle();
      }}
      onkeydown={(event) => {
        if (event.key !== 'Enter' && event.key !== ' ') return;
        if ((event.target as HTMLElement).closest('button')) return;
        event.preventDefault();
        toggle();
      }}
      onmouseenter={onEnter}
      onmouseleave={onLeave}
      onfocusin={onFocusIn}
      onfocusout={onFocusOut}
    >
      <div class="vtca__panel">
        <div class="vtca__clip">
          <!-- the resting capsule: the live mark and the clock -->
          <div class="vtca__layer vtca__mini" data-slot="mini" data-for="run" hidden={paused}>
            <span class="vtca__live" aria-hidden="true"></span>
            <span class="vtca__clock num">{clock}</span>
            <svg class="vtca__chev" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M4.4 6.4 8 10l3.6-3.6" /></svg>
          </div>
          <!-- paused, at rest: the same 6px box, hollow — one object changing -->
          <div class="vtca__layer vtca__mini" data-slot="mini" data-for="paused" hidden={!paused}>
            <span class="vtca__ring" aria-hidden="true"></span>
            <span class="vtca__clock num">{clock}</span>
            <span class="vtca__word">Paused</span>
          </div>

          <!-- the slab: band 1 is the subject (with the state word when paused),
               band 2 is the clock, its target and the two verbs -->
          <div class="vtca__layer vtca__open" data-slot="open">
            <div class="vtca__caprow">
              <p class="vtca__subject" title={label}>{label}</p>
              {#if paused}<span class="vtca__state">Paused</span>{/if}
            </div>
            <div class="vtca__row">
              <span class="vtca__clock vtca__clock--big num">{clock}</span>
              <span class="vtca__target">of {targetMin} min</span>
              <span class="vtca__acts">
                <button
                  class="cd-pill cd-pill--sm vtca__verb vtca__verb--ico"
                  type="button"
                  aria-label={paused ? 'Resume the session' : 'Pause the session'}
                  onclick={onPause}
                >
                  <svg class="vtca__vico vtca__vico--pause" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 4v8M10 4v8" /></svg>
                  <svg class="vtca__vico vtca__vico--play" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M5.6 3.4 12.4 8l-6.8 4.6z" /></svg>
                  <span class="vtca__verblabel">{paused ? 'Resume' : 'Pause'}</span>
                </button>
                <button
                  class="cd-pill cd-pill--sm vtca__verb vtca__verb--hi"
                  type="button"
                  data-command={command ?? undefined}
                  data-placement={command ? placement : undefined}
                  aria-label="Stop and log the session"
                  onclick={ask}
                >
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
                  <span class="vtca__verblabel">Stop · log <span class="num">{clock}</span></span>
                </button>
              </span>
              <!-- The slab's own row — the one that carries Pause and Stop·log —
                   and the surface's design switch at its end. `.vtca__acts` is
                   pushed right by its own `margin-left: auto`, so the capsule
                   lands at the row's end without a second auto margin (two would
                   split the row's air and center the verbs). The resting 32px
                   capsule keeps nothing. -->
            </div>
          </div>

          <!-- the ask, inside the same measured geometry -->
          <div class="vtca__layer vtca__confirm" data-slot="confirm">
            <div class="vtca__caprow">
              <p class="vtca__ask">
                Log <b class="num">{clock}</b>{#if course} to {course}{/if} — {label}
              </p>
            </div>
            <div class="vtca__row">
              <span class="vtca__acts">
                <button
                  class="cd-pill cd-pill--sm vtca__verb vtca__verb--hi"
                  type="button"
                  data-command={command ?? undefined}
                  data-placement={command ? placement : undefined}
                  aria-label="Log the session"
                  onclick={log}
                >
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
                  <span class="vtca__verblabel">Log it</span>
                </button>
                <button class="cd-pill cd-pill--sm vtca__verb" type="button" aria-label="Keep going" onclick={keepGoing}>
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M4.6 4.6l6.8 6.8M11.4 4.6l-6.8 6.8" /></svg>
                  <span class="vtca__verblabel">Keep going</span>
                </button>
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}


<style>
  /* The reserve. A declared number in the row, never a width the contents set:
     that is what lets the search pill beside it hold still. */
  .vtca {
    --a-reserve: 156px;
    --a-w: 440px;
    --a-slab-h: calc(60px * var(--ui-s));
    /* The slot is centered in the bar: `margin-block: auto`, a `--hit` box in a
       `--titlebar-h` row, leaves 6px of air above it. So the resting capsule,
       which IS the slot, sits at `top: 0` and is centered — and the slab, which
       must meet the window's own top edge (the flares fuse to it), rises by
       exactly that air. Both are the tokens' arithmetic, never a measured
       number. */
    --a-rise: calc((var(--hit) - var(--titlebar-h)) / 2);
    position: relative;
    flex: none;
    width: var(--a-reserve);
    align-self: stretch;
    height: var(--hit);
    margin-block: auto;
    /* the titlebar is a drag region; a click on the session must not start a
       window drag (`-webkit-app-region: drag` swallows pointer events) */
    -webkit-app-region: no-drag;
  }

  .vtca__slot {
    position: relative;
    display: block;
    width: var(--a-reserve);
    height: var(--hit);
    cursor: pointer;
  }

  /* The slab. `width`/`height` rather than a transform: a notch is a shape
     whose MEASUREMENT changed, and scaling it would scale its type. It is
     absolutely positioned, so the animation never re-lays-out the row. */
  .vtca__panel {
    position: absolute;
    right: 0;
    top: 0;
    width: var(--a-reserve);
    height: var(--hit);
    background: var(--ink);
    color: var(--ink-inv);
    /* the radius travels to the clip through a custom property, so the panel's
       two shapes stay one declaration */
    --vtca-r: var(--r-pill);
    border-radius: var(--vtca-r);
    box-shadow: var(--sh-ink);
    transition:
      width var(--dur-3) var(--ease-pop),
      height var(--dur-3) var(--ease-pop),
      top var(--dur-2) var(--ease),
      border-radius var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .vtca__clip {
    position: absolute;
    inset: 0;
    border-radius: var(--vtca-r, var(--r-pill));
    overflow: hidden;
  }

  .vtca__slot:is(:hover, :focus-within, [data-mode='open'], [data-mode='confirm']) .vtca__panel {
    top: var(--a-rise);
    width: var(--a-w);
    height: var(--a-slab-h);
    --vtca-r: 0 0 var(--r-tile) var(--r-tile);
    box-shadow: var(--sh-3);
  }
  /* the ring on ink is `--ink-inv`, drawn inset: the slab meets the window's
     top edge, where an outer outline would be clipped by the window itself */
  .vtca__slot:focus-visible .vtca__panel {
    outline: 2px solid var(--ink-inv);
    outline-offset: -3px;
  }
  .vtca .cd-pill:focus-visible {
    outline: 2px solid var(--ink-inv);
    outline-offset: 2px;
  }

  /* The flanges: the ink flares outward where it meets the edge, and the flare
     is a CONCAVE fillet — a rounded box-shadow blob, not a `border-radius`. */
  .vtca__panel::before,
  .vtca__panel::after {
    content: '';
    position: absolute;
    top: -1px;
    width: 20px;
    height: 20px;
    background: transparent;
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease);
  }
  .vtca__panel::before {
    right: 100%;
    border-top-right-radius: var(--r-tile);
    box-shadow: 10px -10px 0 10px var(--ink);
  }
  .vtca__panel::after {
    left: 100%;
    /* 7px radius matches the 8px gap to the adjacent control without clipping. */
    border-top-left-radius: 7px;
    box-shadow: -10px -10px 0 10px var(--ink);
  }
  .vtca__slot:is(:hover, :focus-within, [data-mode='open'], [data-mode='confirm']) .vtca__panel::before,
  .vtca__slot:is(:hover, :focus-within, [data-mode='open'], [data-mode='confirm']) .vtca__panel::after {
    opacity: 1;
    transition-delay: var(--dur-1);
  }
  /* Hide side fillets when clearance checks fail. */
  .vtca[data-flare-l='0'] .vtca__panel::before,
  .vtca[data-flare-r='0'] .vtca__panel::after {
    display: none;
  }

  /* The three layers of one object, cross-faded: the shape first, the words
     110ms behind it. */
  .vtca__layer {
    position: absolute;
    inset: 0;
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--dur-2) var(--ease);
  }
  .vtca__slot:not(:hover):not(:focus-within):not([data-mode='open']):not([data-mode='confirm']) .vtca__layer[data-slot='mini'],
  .vtca__slot:is(:hover, :focus-within, [data-mode='open']) .vtca__layer[data-slot='open'] {
    opacity: 1;
    visibility: visible;
  }
  .vtca__slot:is(:hover, :focus-within, [data-mode='open']) .vtca__layer[data-slot='open'] {
    transition-delay: var(--dur-1);
  }
  .vtca__slot[data-mode='confirm'] .vtca__layer[data-slot='open'] {
    opacity: 0;
    visibility: hidden;
  }
  .vtca__slot[data-mode='confirm'] .vtca__layer[data-slot='confirm'] {
    opacity: 1;
    visibility: visible;
    transition-delay: var(--dur-1);
  }
  .vtca__layer[hidden] {
    display: none;
  }

  /* the mini capsule */
  .vtca__mini {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 0 var(--space-sm) 0 var(--space-md);
  }
  .vtca__clock {
    min-width: 6ch;
    font-size: var(--text-xs);
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-title);
  }
  /* the mark: STATIC, a solid dot running and a hollow ring paused. The state
     is the shape; permanent chrome is the last place for an animation. */
  .vtca__live {
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--ink-inv);
    flex: none;
  }
  .vtca__ring {
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    box-shadow: inset 0 0 0 1.5px var(--ink-inv);
    flex: none;
  }
  .vtca[data-state='paused'] .vtca__clock {
    opacity: 0.62;
  }
  .vtca__word {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    opacity: 0.72;
  }
  .vtca__chev {
    width: 13px;
    height: 13px;
    margin-left: auto;
    opacity: 0.5;
    transition: transform var(--dur-2) var(--ease);
  }
  .vtca__slot:is(:hover, [data-mode='open']) .vtca__chev {
    transform: rotate(180deg);
  }

  /* the slab's two bands */
  .vtca__open,
  .vtca__confirm {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-3xs);
    padding: 6px var(--space-md) 8px;
  }
  .vtca__caprow {
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
    min-width: 0;
  }
  .vtca__state {
    flex: none;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    opacity: 0.85;
  }
  .vtca__row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    min-width: 0;
  }
  .vtca__clock--big {
    font-size: var(--text-2xl);
    min-width: 6ch;
    line-height: 1;
  }
  .vtca__target {
    flex: none;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    opacity: 0.7;
  }
  .vtca__subject {
    flex: 1 1 auto;
    min-width: 0;
    margin: 0;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vtca__ask {
    flex: 1 1 auto;
    min-width: 0;
    margin: 0;
    font-size: var(--text-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vtca__ask b {
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
  }
  .vtca__acts {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    margin-left: auto;
    flex: none;
  }
  .vtca__vico {
    width: 13px;
    height: 13px;
    flex: none;
  }
  .vtca__vico--play {
    display: none;
  }
  .vtca[data-state='paused'] .vtca__vico--pause {
    display: none;
  }
  .vtca[data-state='paused'] .vtca__vico--play {
    display: block;
  }

  /* The measured densities. The room decides: the slab keeps the clock, the
     target and the verbs and gives up the reading the queue row already owns
     (the subject), and then the stop verb's word. Every verb stays a 32px
     target in every density. */
  .vtca__verb--pause .vtca__verblabel {
    display: none;
  }
  .vtca__verb--ico {
    padding: 0 10px;
  }
  .vtca[data-room='mid'] .vtca__target,
  .vtca[data-room='mid'] .vtca__verblabel {
    display: none;
  }
  .vtca[data-room='tight'] .vtca__subject,
  .vtca[data-room='tight'] .vtca__target,
  .vtca[data-room='tight'] .vtca__verblabel {
    display: none;
  }
  .vtca[data-room='tight'] .vtca__clock--big {
    font-size: var(--text-xs);
  }
  .vtca[data-room='tight'] .vtca__open,
  .vtca[data-room='tight'] .vtca__confirm {
    padding: 6px var(--space-sm) 8px;
  }
  .vtca[data-room='tight'] .vtca__row,
  .vtca[data-room='tight'] .vtca__acts {
    gap: var(--space-2xs);
  }

  /* the shipped on-ink recipe */
  .vtca .cd-pill {
    background: var(--fill-on-ink);
    color: var(--ink-inv);
  }
  .vtca .cd-pill:hover,
  .vtca .cd-pill.vtca__verb--hi {
    background: var(--fill-on-ink-hi);
  }

  @media (prefers-reduced-motion: reduce) {
    .vtca__panel,
    .vtca__panel::before,
    .vtca__panel::after,
    .vtca__layer,
    .vtca__chev {
      transition: none;
    }
  }
</style>
