<!--
  Catchup sheet variant C: The Slide-Up Panel.
  Bottom slide-up drawer with card selection and swipe-to-confirm action slider (§11).
-->
<script lang="ts">
  import { arrivalLine, plural, type CatchupProps } from './props';

  let { title, items, dates, onCommit, onUndo, onClose }: CatchupProps = $props();

  const total = $derived(items.length);
  const home = $derived(dates[0]);

  let picked = $state<Record<string, boolean>>({});
  let key = $state<string>(dates[0]?.key ?? '1');
  let phase = $state<'live' | 'receipt'>('live');
  let busy = $state(false);
  let open = $state(true);
  let fraction = $state(0);
  let dragging = $state(false);
  let snapshot: { picked: Record<string, boolean>; key: string } | null = null;
  let ring = $state<{ left: number; width: number }>({ left: 0, width: 0 });

  const chosen = $derived(dates.find((date) => date.key === key) ?? home);
  const pickedIds = $derived(items.filter((item) => picked[item.id]).map((item) => item.id));
  const count = $derived(pickedIds.length);

  const lipSub = $derived(
    phase === 'receipt' ? 'Backlog moved' : `${count} of ${total} on the move · ${chosen?.short}`,
  );
  const doorLabel = $derived(open ? 'Put it back' : 'Open the move');
  const trackLabel = $derived(
    dragging ? 'Release to confirm' : `Slide to move ${plural(count, 'recall', 'recalls')}`,
  );
  const thumbLabel = $derived(`Move ${plural(count, 'recall', 'recalls')} to ${chosen?.full ?? ''}`);
  const bandLine = $derived(arrivalLine(count, chosen?.full ?? ''));
  const disabled = $derived(count === 0 && phase !== 'receipt');

  let foldEl = $state<HTMLElement | null>(null);
  let trackEl = $state<HTMLElement | null>(null);
  let thumbEl = $state<HTMLElement | null>(null);
  const plates: Record<string, HTMLElement> = {};

  // ── the travelling ring: one mark that slides to the chosen plate ───────
  function placeRing(): void {
    if (!open || phase === 'receipt') return;
    const plate = plates[key];
    if (!plate) return;
    const left = plate.offsetLeft;
    const width = plate.offsetWidth;
    if (left !== ring.left || width !== ring.width) ring = { left, width };
  }
  $effect(() => {
    void key;
    void phase;
    void open;
    placeRing();
  });

  function toggle(id: string): void {
    if (phase === 'receipt' || !open) return;
    picked = { ...picked, [id]: !picked[id] };
  }

  function choose(region: string): void {
    if (phase === 'receipt') return;
    key = region;
  }

  function rove(event: KeyboardEvent, index: number): void {
    const delta =
      event.key === 'ArrowRight' || event.key === 'ArrowDown'
        ? 1
        : event.key === 'ArrowLeft' || event.key === 'ArrowUp'
          ? -1
          : 0;
    let to = index;
    if (delta !== 0) to = (index + delta + dates.length) % dates.length;
    else if (event.key === 'Home') to = 0;
    else if (event.key === 'End') to = dates.length - 1;
    else return;
    event.preventDefault();
    choose(dates[to].key);
    foldEl?.querySelectorAll<HTMLElement>('.xc-plate')[to]?.focus();
  }

  function reduced(): boolean {
    return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  }

  // ── the track: the thumb holds its grab point, springs home, writes ─────
  let maxX = 1;
  let grabOffset = 24;
  let trackStart = 0;
  let trackMoved = false;
  let releaseTimer: ReturnType<typeof setTimeout> | null = null;

  function setX(value: number): void {
    fraction = value;
    trackEl?.style.setProperty('--x', `${Math.round(value)}px`);
    trackEl?.style.setProperty('--fade', `${Math.max(0, 1 - value / 48).toFixed(2)}`);
  }
  function measure(): void {
    maxX = Math.max(1, (trackEl?.clientWidth ?? 0) - 8 - (thumbEl?.offsetWidth ?? 48));
  }

  function trackDown(event: PointerEvent): void {
    if (phase === 'receipt' || !open || count === 0 || !trackEl || !thumbEl) return;
    measure();
    dragging = true;
    trackMoved = false;
    trackStart = event.clientX;
    const box = trackEl.getBoundingClientRect();
    // The thumb holds the exact point it was grabbed by.
    grabOffset = Math.max(0, Math.min(thumbEl.offsetWidth, event.clientX - (box.left + 4 + fraction)));
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
    event.preventDefault();
  }

  function trackMove(event: PointerEvent): void {
    if (!dragging || !trackEl) return;
    if (Math.abs(event.clientX - trackStart) > 4) trackMoved = true;
    const box = trackEl.getBoundingClientRect();
    setX(Math.max(0, Math.min(maxX, event.clientX - box.left - 4 - grabOffset)));
  }

  async function write(): Promise<void> {
    if (count === 0 || phase === 'receipt' || busy || !chosen) return;
    const before = { picked: { ...picked }, key };
    busy = true;
    const landed = await onCommit([...pickedIds], chosen.iso);
    busy = false;
    if (!landed) return;
    snapshot = before;
    phase = 'receipt';
    setX(0);
    dragging = false;
    thumbEl?.focus({ preventScroll: true });
  }

  function trackUp(): void {
    if (!dragging) return;
    dragging = false;
    measure();
    if (!trackMoved) {
      // A tap is not a commit: the thumb nudges forward and springs home.
      setX(Math.min(maxX, Math.max(28, fraction + 28)));
      if (releaseTimer) clearTimeout(releaseTimer);
      releaseTimer = setTimeout(() => {
        if (!dragging) setX(0);
      }, 220);
      return;
    }
    if (fraction >= maxX - 6) {
      void write();
      return;
    }
    setX(0);
  }

  function trackCancel(): void {
    if (!dragging) return;
    dragging = false;
    setX(0);
  }

  // ── the lip: press to open, or pull the bar up into the panel ──────────
  type LipDrag = { y: number; height: number; moved: boolean };
  let lipDrag: LipDrag | null = null;

  function lipDown(event: PointerEvent): void {
    if (phase === 'receipt') return;
    lipDrag = { y: event.clientY, height: foldEl?.getBoundingClientRect().height ?? 320, moved: false };
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  }
  function lipMove(event: PointerEvent): void {
    if (!lipDrag || reduced() || !foldEl) return;
    const dy = lipDrag.y - event.clientY;
    if (Math.abs(dy) > 4) lipDrag.moved = true;
    if (!lipDrag.moved) return;
    const full = lipDrag.height || 320;
    const p = Math.max(0, Math.min(1, ((open ? full : 0) + dy) / full));
    foldEl.style.transition = 'none';
    foldEl.style.gridTemplateRows = p.toFixed(3) + 'fr';
  }
  function lipUp(event: PointerEvent): void {
    if (!lipDrag) return;
    const d = lipDrag;
    lipDrag = null;
    if (foldEl) {
      foldEl.style.transition = '';
      foldEl.style.gridTemplateRows = '';
    }
    if (!d.moved) {
      if (phase !== 'receipt') open = !open;
      return;
    }
    const dy = d.y - event.clientY;
    const full = d.height || 320;
    open = Math.max(0, Math.min(1, ((open ? full : 0) + dy) / full)) > 0.5;
    requestAnimationFrame(placeRing);
  }

  function undo(): void {
    onUndo();
    if (snapshot) {
      picked = { ...snapshot.picked };
      key = snapshot.key;
    }
    phase = 'live';
    open = true;
    requestAnimationFrame(placeRing);
  }

  function dismiss(): void {
    if (!open) return;
    open = false;
    globalThis.setTimeout(onClose, reduced() ? 0 : 260);
  }

  let armed = $state(false);
  $effect(() => {
    // **Armed one tick after mount**, exactly as `shell/Sheet.svelte` arms its
    // own dismissal (see A.svelte): the opening click must not take the surface
    // away in the same tick it was asked for.
    const id = setTimeout(() => (armed = true), 0);
    return () => clearTimeout(id);
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape') return;
    if (phase === 'receipt') {
      undo();
      return;
    }
    if (!open) return;
    dismiss();
  }
  function onclick(event: MouseEvent): void {
    if (!armed) return;
    const target = event.target as Element | null;
    if (!target?.closest?.('.xc-panel')) dismiss();
  }
</script>

<svelte:window {onkeydown} {onclick} />

<div class="scc-layer">
  <div
    class="xc-panel"
    data-open={open}
    data-status={phase}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
  >
    <div class="xc-liprow">
      <button
        class="xc-lip"
        type="button"
        aria-expanded={open && phase !== 'receipt'}
        aria-controls="xc-fold"
        onpointerdown={lipDown}
        onpointermove={lipMove}
        onpointerup={lipUp}
        onpointercancel={lipUp}
        onclick={(event) => {
          // A keyboard activation arrives as a click with no pointer travel.
          if (event.detail !== 0 || phase === 'receipt') return;
          open = !open;
          requestAnimationFrame(placeRing);
        }}
      >
        <span class="xc-grip" aria-hidden="true"></span>
        <span class="xc-lip__id">
          <span class="xc-lip__t">{title}</span>
          <span class="xc-lip__s num">{lipSub}</span>
        </span>
      </button>
      <button class="cd-pill cd-pill--quiet cd-pill--sm xc-door" type="button" onclick={dismiss}>
        {doorLabel}
      </button>
      <!-- The panel's own lip row: where the panel names itself as it rises, and
           the surface's design switch at the row's end. -->
    </div>

    <div class="xc-fold" id="xc-fold" inert={!open || phase === 'receipt'}>
      <div class="xc-clip">
        <div class="xc-body">
          <div class="xc-sec">
            <p class="xc-key">On the move</p>
            <div class="xc-tray" aria-label="Recalls on the move">
              {#each items as item (item.id)}
                <button
                  class="xc-card"
                  type="button"
                  aria-pressed={picked[item.id] ?? false}
                  onclick={() => toggle(item.id)}
                >
                  <span class="cd-check" aria-hidden="true">
                    {#if picked[item.id]}
                      <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.5 8.4 6.6 11.4 12.5 5"/></svg>
                    {/if}
                  </span>
                  <span class="xc-card__body">
                    <span class="xc-card__t">{item.label}</span>
                    <span class="x-facts">
                      {#if item.course}
                        <span class="cd-chip cd-chip--wash" data-w={item.wash ?? undefined}>{item.course}</span>
                      {/if}
                      {#if item.late}
                        <span class="cd-chip cd-chip--overdue">{item.late}</span>
                      {/if}
                    </span>
                  </span>
                </button>
              {/each}
            </div>
          </div>

          <div class="xc-sec">
            <p class="xc-key">They come back on</p>
            <div class="xc-plates" role="radiogroup" aria-label="They come back on">
              <span
                class="xc-ring"
                aria-hidden="true"
                style={`left: ${ring.left}px; width: ${ring.width}px`}
              ></span>
              {#each dates as date, index (date.key)}
                <button
                  class="xc-plate"
                  type="button"
                  role="radio"
                  bind:this={plates[date.key]}
                  aria-checked={date.key === key && phase !== 'receipt'}
                  aria-label={`${date.name} — ${date.full}`}
                  tabindex={date.key === key ? 0 : -1}
                  onclick={() => choose(date.key)}
                  onkeydown={(event) => rove(event, index)}
                >
                  <span class="cd-datetile num">{date.tile}</span>
                  <span class="xc-plate__txt">
                    <span class="xc-plate__t">{date.name}</span>
                    <span class="xc-plate__s">{date.full}</span>
                  </span>
                </button>
              {/each}
            </div>
          </div>

          <div class="xc-track" class:is-drag={dragging} bind:this={trackEl} aria-disabled={disabled}>
            <span class="xc-trail" aria-hidden="true"></span>
            <span class="xc-lab" aria-hidden="true">{busy ? 'Moving…' : trackLabel}</span>
            <button
              class="xc-thumb"
              type="button"
              bind:this={thumbEl}
              aria-label={thumbLabel}
              aria-disabled={disabled}
              onpointerdown={trackDown}
              onpointermove={trackMove}
              onpointerup={trackUp}
              onpointercancel={trackCancel}
              onclick={(event) => {
                // A pointer drag is decided in `trackUp`; an assistive
                // activation — and Enter or Space — arrives as a click with no
                // gesture behind it, and writes without the drag.
                if (event.detail !== 0) return;
                if (disabled) return;
                void write();
              }}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M5 12h13M13 6l6 6-6 6"/></svg>
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="xc-receipt" role="status" inert={phase !== 'receipt'}>
      <div class="xc-rrow">
        <span class="xc-seal" aria-hidden="true">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M4.5 12.6 9.4 17.5 19.5 7"/></svg>
        </span>
        <span class="xc-receipt__txt">
          <b class="x-receipt__t">{bandLine}</b>
          <span class="x-receipt__s">Review history is untouched — this writes the day they come back</span>
        </span>
        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={undo}>Undo the move</button>
      </div>
    </div>
  </div>
</div>

<style>
  .scc-layer {
    position: fixed;
    inset: 0;
    z-index: 45;
    pointer-events: none;
    container-type: inline-size;
  }

  .xc-panel {
    position: absolute;
    left: 50%;
    bottom: 0;
    transform: translateX(-50%);
    width: min(720px, calc(100cqw - 2 * var(--ui-pad)));
    pointer-events: auto;
    background: var(--card);
    border-radius: var(--r-card) var(--r-card) 0 0;
    box-shadow: var(--sh-3), var(--catch);
    overflow: clip;
  }

  /* The bar: the lip is the pull handle and the door sits beside it, so the
     line reads as one object but is two real controls. */
  .xc-liprow {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-height: calc(72px * var(--ui-s));
    padding: var(--space-sm) var(--space-lg);
  }
  .xc-lip {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex: 1;
    min-width: 0;
    min-height: var(--hit);
    padding: 0;
    text-align: left;
    cursor: grab;
    touch-action: none;
  }
  .xc-lip:active {
    cursor: grabbing;
  }
  .xc-grip {
    flex: none;
    width: var(--hit);
    height: var(--hit);
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    color: var(--ink-3);
    transition: color var(--dur-1) var(--ease);
  }
  .xc-grip::before {
    content: '';
    width: 18px;
    height: 3px;
    border-radius: var(--r-pill);
    background: currentColor;
  }
  .xc-lip:hover .xc-grip {
    color: var(--ink-2);
  }
  .xc-lip__id {
    display: grid;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .xc-lip__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink);
    letter-spacing: var(--track-title);
  }
  .xc-lip__s {
    font-size: var(--text-xs);
    color: var(--ink-2);
  }
  .num {
    font-variant-numeric: tabular-nums;
  }
  .xc-panel[data-status='receipt'] .xc-door {
    visibility: hidden;
  }

  /* Rise: the body opens on a `0fr → 1fr` row; committed, the same row folds
     and the receipt's row takes its place, so the panel shrinks rather than
     becoming a second screen. */
  .xc-fold {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-3) var(--ease);
  }
  .xc-panel[data-open='true'] .xc-fold {
    grid-template-rows: 1fr;
  }
  .xc-clip {
    min-height: 0;
    overflow: hidden;
  }
  .xc-body {
    display: grid;
    gap: var(--space-md);
    padding: var(--space-md) var(--space-lg) var(--space-lg);
    background: var(--well);
  }
  .xc-sec {
    display: grid;
    gap: var(--space-xs);
  }
  .xc-key {
    display: flex;
    align-items: baseline;
    gap: var(--space-2xs);
    margin: 0;
    font-size: var(--text-2xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .xc-tray {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-xs);
    padding: var(--space-xs);
    background: var(--well-2);
    border-radius: var(--r-tile);
  }
  .xc-card {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: var(--space-xs);
    min-height: calc(56px * var(--ui-s));
    padding: var(--space-xs) var(--space-sm);
    min-width: 0;
    text-align: left;
    border-radius: var(--r-item);
    background: transparent;
    transition: background var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .xc-card:hover {
    background: var(--rule);
  }
  .xc-card:active {
    background: var(--well);
    transform: scale(0.99);
  }
  .xc-card[aria-pressed='true'] {
    background: var(--card);
    box-shadow: var(--sh-1);
  }
  .xc-card[aria-pressed='true']:hover {
    box-shadow: var(--sh-2);
  }
  .xc-card__body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .xc-card__t {
    font-size: calc(var(--text-sm) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-2);
    letter-spacing: var(--track-title);
    min-width: 0;
  }
  .xc-card[aria-pressed='true'] .xc-card__t {
    color: var(--ink);
  }
  .xc-card .cd-check {
    width: 18px;
    height: 18px;
    background: transparent;
    color: var(--ink);
  }
  .x-facts {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    flex-wrap: wrap;
  }
  .x-facts .cd-chip {
    flex: none;
  }

  /* The day plates, and the one stroke that slides between them. */
  .xc-plates {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-xs);
  }
  .xc-ring {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: var(--r-item);
    pointer-events: none;
    box-shadow: inset 0 0 0 1.5px var(--ink);
    transition: left var(--dur-2) var(--ease), width var(--dur-2) var(--ease);
  }
  .xc-plate {
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
    min-height: calc(60px * var(--ui-s));
    padding: var(--space-xs) var(--space-sm);
    min-width: 0;
    text-align: left;
    border-radius: var(--r-item);
    transition: background var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .xc-plate:hover {
    background: var(--rule);
  }
  .xc-plate:active {
    background: var(--well);
    transform: scale(0.99);
  }
  .xc-plate__txt {
    min-width: 0;
  }
  .xc-plate__t {
    display: block;
    font-size: calc(var(--text-sm) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-2);
    letter-spacing: var(--track-title);
  }
  .xc-plate[aria-checked='true'] .xc-plate__t {
    color: var(--ink);
  }
  .xc-plate__s {
    display: block;
    margin-top: 1px;
    font-size: var(--text-2xs);
    color: var(--ink-2);
  }
  .xc-plate[aria-checked='true'] .cd-datetile {
    background-color: var(--well-2);
    color: var(--ink);
  }

  /* The commit: the screen's one ink object, with the reference's slide track. */
  .xc-track {
    position: relative;
    height: calc(56px * var(--ui-s));
    padding: 4px;
    border-radius: var(--r-pill);
    background: var(--ink);
  }
  .xc-trail {
    position: absolute;
    left: 4px;
    top: 4px;
    bottom: 4px;
    width: calc(48px + var(--x, 0px));
    border-radius: var(--r-pill);
    background: var(--fill-on-ink);
  }
  .xc-lab {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    color: var(--ink-inv);
    opacity: var(--fade, 1);
  }
  .xc-thumb {
    position: absolute;
    left: 4px;
    top: 4px;
    width: calc(48px * var(--ui-s));
    height: calc(48px * var(--ui-s));
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--sh-ink);
    transform: translateX(var(--x, 0px));
    touch-action: none;
    transition: transform var(--dur-2) var(--ease);
  }
  .xc-track.is-drag .xc-thumb {
    transition: none;
    cursor: grabbing;
  }
  .xc-track.is-drag {
    cursor: grabbing;
  }
  .xc-track[aria-disabled='true'] {
    opacity: 0.55;
  }
  .xc-track[aria-disabled='true'] .xc-thumb {
    cursor: default;
  }

  .xc-receipt {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-3) var(--ease);
  }
  .xc-panel[data-status='receipt'] .xc-receipt {
    grid-template-rows: 1fr;
  }
  .xc-panel[data-status='receipt'] .xc-fold {
    grid-template-rows: 0fr;
  }
  .xc-rrow {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    min-width: 0;
    padding: var(--space-md) var(--space-lg);
    overflow: hidden;
  }
  .xc-seal {
    flex: none;
    width: var(--hit);
    height: var(--hit);
    border-radius: var(--r-pill);
    display: grid;
    place-items: center;
    background: var(--ink);
    color: var(--ink-inv);
  }
  .xc-receipt__txt {
    display: grid;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .x-receipt__t {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-display);
    color: var(--ink);
    letter-spacing: var(--track-title);
  }
  .x-receipt__s {
    font-size: var(--text-2xs);
    color: var(--ink-2);
  }
  .xc-panel[data-status='receipt'] .xc-seal {
    animation: xc-seal var(--dur-2) var(--ease-pop) both;
  }
  @keyframes xc-seal {
    from {
      transform: scale(0.4);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }

  @container (max-width: 560px) {
    .xc-tray,
    .xc-plates {
      grid-template-columns: minmax(0, 1fr);
    }
    .xc-ring {
      display: none;
    }
    .xc-lip__t {
      font-size: var(--text-sm);
    }
  }
</style>
