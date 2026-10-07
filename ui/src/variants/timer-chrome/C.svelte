<!-- Timer chrome variant C: The Floor Bar. Docked bottom toolbar. -->
<script module lang="ts">
  /** One id per mounted dock: the Facts object names the panel it controls. */
  let panels = 0;
</script>

<script lang="ts">
  import type { TimerProps } from './props';

  let {
    label,
    clock,
    started,
    paused,
    targetMin,
    elapsedMin,
    command,
    placement,
    ended,
    onPause,
    onStop,
  }: TimerProps = $props();

  let root = $state<HTMLElement | null>(null);
  let dock = $state<HTMLElement | null>(null);
  let twin = $state<HTMLElement | null>(null);
  let bar = $state<HTMLElement | null>(null);
  let facts = $state(false);
  const panelId = `vtcc-facts-${(panels += 1)}`;

  const left = $derived(Math.max(0, targetMin - Math.floor(elapsedMin)));
  const percent = $derived(Math.max(0, Math.min(100, (elapsedMin / Math.max(1, targetMin)) * 100)));

  /** The floor the layout gives up: the dock's own measured height. */
  function measureDock(): void {
    const window_ = dock?.closest('.cd-window') as HTMLElement | null;
    if (!window_ || !dock) return;
    const height = ended ? 0 : Math.ceil(dock.getBoundingClientRect().height);
    window_.style.setProperty('--dock-h', `${height}px`);
  }

  /** The panel's open height, read from the twin the reference keeps for it. */
  function measurePanel(): void {
    if (!twin || !root) return;
    root.style.setProperty('--c-facts-h', `${Math.ceil(twin.getBoundingClientRect().height)}px`);
  }

  $effect(() => {
    const node = dock;
    const mirror = twin;
    if (!node || !mirror) return;
    measureDock();
    measurePanel();
    const observer = new ResizeObserver(() => {
      measureDock();
      measurePanel();
    });
    observer.observe(node);
    observer.observe(mirror);
    return () => observer.disconnect();
  });

  // the dock's height changes with the fold, so the reservation follows it
  $effect(() => {
    void facts;
    void ended;
    measureDock();
  });

  /** The reference's own click-outside: anything that is not the bar closes it. */
  function onOutside(event: MouseEvent): void {
    if (!facts) return;
    const target = event.target as HTMLElement;
    if (target.closest('.vtcc__bar') || target.closest('.vtcc__panel')) return;
    facts = false;
  }
  $effect(() => {
    if (!facts) return;
    document.addEventListener('mousedown', onOutside);
    return () => document.removeEventListener('mousedown', onOutside);
  });

  const chipLabel = $derived(`Studying ${label}, ${clock} elapsed${paused ? ', paused' : ''}`);
</script>

<div
  class="vtcc"
  bind:this={root}
  data-mode={ended ? 'ended' : paused ? 'paused' : 'run'}
  data-facts={facts ? '1' : '0'}
>
  <div class="vtcc__dock" bind:this={dock}>
    <!-- the reference's hidden measurement node: off screen, out of the
         accessibility tree, and the only second copy of the panel anywhere -->
    <div class="vtcc__twin" aria-hidden="true">
      <div class="vtcc__panel" bind:this={twin}>
        <div class="vtcc__fact"><span class="vtcc__fk">Started</span><span class="vtcc__fv num">{started}</span></div>
        <div class="vtcc__fact"><span class="vtcc__fk">Left</span><span class="vtcc__fv num">{left} min of {targetMin}</span></div>
        <!-- The same switch row the panel carries: the twin exists to have the
             panel's open height measured off it (`--c-facts-h`), so it has to
             hold every row the panel holds. It is off screen and
             `visibility: hidden`, so this copy is neither seen nor reachable. -->
      </div>
    </div>

    <div class="vtcc__panel" id={panelId}>
      <div class="vtcc__fact"><span class="vtcc__fk">Started</span><span class="vtcc__fv num">{started}</span></div>
      <div class="vtcc__fact"><span class="vtcc__fk">Left</span><span class="vtcc__fv num">{left} min of {targetMin}</span></div>
      <!-- The panel the dock grows is the only region this design opens, so the
           surface's design switch stands in it — never in the 32px bar, where a
           capsule does not fit. -->
    </div>

    <div class="vtcc__bar" bind:this={bar}>
      <span class="vtcc__chip" role="timer" aria-label={chipLabel}>
        <span class="vtcc__live" aria-hidden="true" hidden={paused}></span>
        <span class="vtcc__ring" aria-hidden="true" hidden={!paused}></span>
        <span class="vtcc__clock num">{clock}</span>
        {#if paused}<span class="vtcc__state">Paused</span>{/if}
        {#if !paused}<span class="vtcc__name" title={label}>{label}</span>{/if}
        <span class="vtcc__meter" aria-hidden="true"><i style={`--v:${percent.toFixed(1)}%`}></i></span>
      </span>
      <button
        class="vtcc__obj vtcc__obj--pause"
        type="button"
        aria-label={paused ? 'Resume the session' : 'Pause the session'}
        title={paused ? 'Resume the session' : 'Pause the session'}
        onclick={onPause}
      >
        <svg class="vtcc__g vtcc__g--pause" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 4v8M10 4v8" /></svg>
        <svg class="vtcc__g vtcc__g--play" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M5.6 3.4 12.4 8l-6.8 4.6z" /></svg>
        <span class="vtcc__vlabel">{paused ? 'Resume' : 'Pause'}</span>
      </button>
      <button
        class="vtcc__obj"
        type="button"
        data-command={command ?? undefined}
        data-placement={command ? placement : undefined}
        aria-label="Stop and log the session"
        title="Stop and log the session"
        onclick={onStop}
      >
        <svg class="vtcc__g" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
        <span class="vtcc__vlabel">Stop</span>
      </button>
      <button
        class="vtcc__obj"
        type="button"
        aria-label={facts ? 'Hide the session facts' : 'Session facts'}
        aria-expanded={facts}
        aria-controls={panelId}
        onclick={() => (facts = !facts)}
      >
        <span class="vtcc__vlabel">Facts</span>
        <svg class="vtcc__g vtcc__chev" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M4.4 6.4 8 10l3.6-3.6" /></svg>
      </button>
    </div>
  </div>
</div>

<style>
  /* The dock is out of the row's flow and pinned to the window's own floor:
     `.cd-window` is `position: relative`, so this absolute box lands on its
     bottom edge, and the window's `overflow: hidden` keeps it inside. */
  .vtcc {
    --c-chip: calc(244px * var(--ui-s));
    --c-panel: 264px;
    display: contents;
  }
  .vtcc__dock {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 25;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0 var(--space-md) var(--space-lg);
    transition:
      transform var(--dur-2) var(--ease),
      opacity var(--dur-2) var(--ease),
      visibility 0s linear 0s;
  }
  .vtcc[data-mode='ended'] .vtcc__dock {
    transform: translateY(100%);
    opacity: 0;
    pointer-events: none;
    visibility: hidden;
    transition:
      transform var(--dur-2) var(--ease),
      opacity var(--dur-2) var(--ease),
      visibility 0s linear var(--dur-2);
  }

  /* the facts panel, above the bar and in the dock's flow */
  .vtcc__panel {
    width: var(--c-panel);
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2xs);
    padding: 0;
    height: 0;
    margin-bottom: 0;
    border-radius: var(--r-tile);
    background: var(--card);
    box-shadow: var(--sh-2);
    overflow: hidden;
    opacity: 0;
    visibility: hidden;
    transform-origin: bottom center;
    transform: scale(0.94, 0.9);
    filter: blur(6px);
    transition:
      height var(--dur-3) var(--ease),
      padding var(--dur-3) var(--ease),
      margin-bottom var(--dur-2) var(--ease),
      opacity var(--dur-2) var(--ease),
      transform var(--dur-3) var(--ease),
      filter var(--dur-2) var(--ease),
      visibility 0s linear var(--dur-3);
  }
  .vtcc[data-facts='1'] .vtcc__panel {
    height: var(--c-facts-h, 68px);
    padding: var(--space-2xs);
    margin-bottom: var(--space-2xs);
    opacity: 1;
    visibility: visible;
    transform: none;
    filter: none;
  }
  /* the twin: measured with everything open, and never seen */
  .vtcc__twin {
    position: absolute;
    left: -9999px;
    top: -9999px;
    visibility: hidden;
    pointer-events: none;
  }
  .vtcc__twin .vtcc__panel {
    height: auto;
    padding: var(--space-2xs);
    margin: 0;
    opacity: 1;
    visibility: hidden;
    transform: none;
    filter: none;
  }
  /* The panel's switch row: it spans the panel's two columns and takes the
     row's right end. The twin mirrors it, so the height measured off the twin
     is still the panel's own. */
  .vtcc__fact {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--r-item);
    background: var(--well);
  }
  .vtcc__fk {
    font-size: var(--text-2xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .vtcc__fv {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* the bar of separate objects, on the system's glass */
  .vtcc__bar {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    padding: var(--space-2xs);
    border-radius: var(--r-tile);
    background: var(--glass);
    box-shadow: var(--rim-glass), var(--sh-2);
    -webkit-backdrop-filter: blur(var(--glass-blur));
    backdrop-filter: blur(var(--glass-blur));
  }

  /* the chip: the session, and this variant's one dark object. A declared
     width, so a longer topic never pushes the objects beside it. */
  .vtcc__chip {
    flex: none;
    width: var(--c-chip);
    height: var(--hit);
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 0 var(--space-sm);
    border-radius: var(--r-pill);
    background: var(--ink);
    color: var(--ink-inv);
    box-shadow: var(--sh-ink);
  }
  .vtcc__live {
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--ink-inv);
    flex: none;
  }
  .vtcc__ring {
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    box-shadow: inset 0 0 0 1.5px var(--ink-inv);
    flex: none;
  }
  .vtcc__live[hidden],
  .vtcc__ring[hidden] {
    display: none;
  }
  .vtcc__clock {
    flex: none;
    min-width: 6ch;
    font-size: var(--text-xs);
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-title);
  }
  .vtcc[data-mode='paused'] .vtcc__clock {
    opacity: 0.62;
  }
  .vtcc__state {
    flex: none;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
  }
  .vtcc__name {
    flex: 1 1 auto;
    min-width: 0;
    font-size: var(--text-2xs);
    opacity: 0.72;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* the progress mark, inside the chip: elapsed against the length the session
     was started with */
  .vtcc__meter {
    flex: none;
    width: 40px;
    height: 4px;
    border-radius: var(--r-pill);
    background: var(--fill-on-ink);
    overflow: hidden;
  }
  .vtcc__meter i {
    display: block;
    height: 100%;
    width: var(--v, 0%);
    border-radius: var(--r-pill);
    background: var(--ink-inv);
  }

  /* the objects beside the chip: 32px targets on the bar's own glass */
  .vtcc__obj {
    flex: none;
    height: var(--hit);
    min-width: var(--hit);
    padding: 0 var(--space-sm);
    border-radius: var(--r-pill);
    background: transparent;
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2xs);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  .vtcc__obj:hover {
    background: var(--well);
  }
  .vtcc__obj:active {
    background: var(--well-2);
  }
  .vtcc__obj:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .vtcc__obj[aria-expanded='true'] {
    background: var(--well-2);
  }
  /* the bar is centred in the dock, so the pause object holds the width of its
     longer word — otherwise "Resume" would slide every object sideways. The
     selected state is the word, never a colour: the chip is the ink. */
  .vtcc__obj--pause {
    min-width: calc(88px * var(--ui-s));
  }
  .vtcc__g {
    width: 13px;
    height: 13px;
    flex: none;
  }
  /* One glyph per (state, verb), never two in the same box; the pause object
     says what pressing it will DO, so the glyph swaps with the word. */
  .vtcc__g--play {
    display: none;
  }
  .vtcc[data-mode='paused'] .vtcc__g--pause {
    display: none;
  }
  .vtcc[data-mode='paused'] .vtcc__g--play {
    display: block;
  }
  .vtcc__chev {
    transition: transform var(--dur-2) var(--ease);
  }
  .vtcc__obj[aria-expanded='true'] .vtcc__chev {
    transform: rotate(180deg);
  }

  /* the bar gives up its readings in its own order — the name first (it is
     whole on the queue row), then the meter — and never its objects */
  @media (max-width: 780px) {
    .vtcc {
      --c-chip: 148px;
      --c-panel: 240px;
    }
    .vtcc__name {
      display: none;
    }
  }
  @media (max-width: 620px) {
    .vtcc {
      --c-chip: 116px;
    }
    .vtcc__meter {
      display: none;
    }
    .vtcc__vlabel {
      display: none;
    }
    .vtcc__obj {
      padding: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .vtcc__dock,
    .vtcc__panel,
    .vtcc__obj,
    .vtcc__chev,
    .vtcc__meter i {
      transition: none;
    }
  }
</style>
