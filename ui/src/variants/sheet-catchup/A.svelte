<!--
  Catchup sheet variant A: The Tray.
  Floor-docked tray allowing students to drag and file overdue cards into target date slots
  to batch-defer reviews (§11).
-->
<script lang="ts">
  import { arrivalLine, filedLine, plural, type CatchupProps } from './props';

  let { title, note, items, dates, onCommit, onUndo, onClose }: CatchupProps = $props();

  /** A card that stays put lies splayed: the angle is readable at rest. */
  const SPLAY = [-1.6, 1.2, -0.6, 1.6];

  const total = $derived(items.length);
  const home = $derived(dates[0]);
  /** The one date the batch is filed on — the region the group stands in. */
  let key = $state<string>(dates[0]?.key ?? '1');
  /** record → the region it stands in: a date's key, or `out` (staying put). */
  let at = $state<Record<string, string>>(
    Object.fromEntries(items.map((item) => [item.id, dates[0]?.key ?? '1'])),
  );
  let phase = $state<'live' | 'receipt'>('live');
  let busy = $state(false);
  let open = $state(true);
  let snapshot: { at: Record<string, string>; key: string } | null = null;
  /** How many cards each region can lay, measured off its own box. */
  let caps = $state<Record<string, number>>({});

  const chosen = $derived(dates.find((date) => date.key === key) ?? home);
  const movingIds = $derived(items.filter((item) => at[item.id] !== 'out').map((item) => item.id));
  const staying = $derived(total - movingIds.length);

  /** Where each card stands in its own region, and how many fit there — the
      mark a card carries when it is past its region's capacity is read off this
      and `caps`, never off a class the layout wrote by hand. */
  const placed = $derived.by(() => {
    const map: Record<string, { region: string; index: number }> = {};
    const counts: Record<string, number> = {};
    for (const item of items) {
      const region = at[item.id] ?? 'out';
      const index = counts[region] ?? 0;
      counts[region] = index + 1;
      map[item.id] = { region, index };
    }
    return map;
  });

  function shown(id: string): boolean {
    const spot = placed[id];
    if (!spot) return false;
    if (phase === 'receipt') return false;
    return spot.index < (caps[spot.region] ?? Number.POSITIVE_INFINITY);
  }

  const footState = $derived(
    phase === 'receipt'
      ? filedLine(movingIds.length, staying)
      : `${movingIds.length} of ${total} in the move${staying > 0 ? ` · ${staying} staying put` : ''}`,
  );
  const footPill = $derived(
    phase === 'receipt'
      ? 'Undo the move'
      : movingIds.length > 0
        ? `Move ${plural(movingIds.length, 'recall', 'recalls')} · ${chosen?.short}`
        : 'Nothing in the move',
  );
  const bandLine = $derived(arrivalLine(movingIds.length, chosen?.full ?? ''));

  let trayEl = $state<HTMLElement | null>(null);
  let holdEl = $state<HTMLElement | null>(null);
  let floorEl = $state<HTMLElement | null>(null);
  let over = $state<string | null>(null);
  const mouths: Record<string, HTMLElement> = {};
  const cards: Record<string, HTMLElement> = {};

  /** The run's own step: the card's shipped height plus its gap, so the size
      register moves it with everything else. */
  function step(): number {
    return (cards[items[0]?.id ?? '']?.offsetHeight ?? 56) + 4;
  }

  function place(el: HTMLElement, left: number, top: number, rot: number): void {
    el.style.left = `${Math.round(left)}px`;
    el.style.top = `${Math.round(top)}px`;
    el.style.setProperty('--rot', `${rot}deg`);
  }

  /** One drop is one left/top change: a card's home is a rect read off the hole
      or the floor it stands in, and the transition rides it home. */
  function layout(): void {
    if (!holdEl) return;
    const hr = holdEl.getBoundingClientRect();
    const run = step();
    const next: Record<string, number> = {};
    for (const date of dates) {
      const mouth = mouths[date.key];
      if (!mouth) continue;
      const rr = mouth.getBoundingClientRect();
      // The box decides its own capacity (the lab's revision 3): one head's
      // worth of room is kept for the label line above the run.
      const cap = Math.max(1, Math.floor((rr.height - (run + 34)) / run) + 1);
      next[date.key] = cap;
      items
        .filter((item) => at[item.id] === date.key)
        .forEach((item, index) => {
          const el = cards[item.id];
          if (!el) return;
          const width = el.offsetWidth || 240;
          place(
            el,
            rr.left - hr.left + Math.max(0, (rr.width - width) / 2),
            rr.top - hr.top + 8 + index * run,
            0,
          );
        });
    }
    if (floorEl) {
      const fr = floorEl.getBoundingClientRect();
      const stay = items.filter((item) => at[item.id] === 'out');
      const width = (stay[0] && cards[stay[0].id]?.offsetWidth) || 240;
      const cap = Math.max(1, Math.floor((fr.width - 16) / (width + 12)));
      next.out = cap;
      stay.forEach((item, index) => {
        const el = cards[item.id];
        if (!el) return;
        place(
          el,
          fr.left - hr.left + 8 + index * (width + 12),
          fr.top - hr.top + Math.max(28, (fr.height - (el.offsetHeight || 56)) / 2),
          SPLAY[index % SPLAY.length],
        );
      });
    }
    if (dates.some((date) => next[date.key] !== caps[date.key]) || next.out !== caps.out) caps = next;
  }

  $effect(() => {
    // One pass per settled state: the DOM is the geometry, so reading the boxes
    // must happen after Svelte has written them — which is when an effect runs.
    void at;
    void key;
    void items;
    void phase;
    layout();
  });

  /** One date for the batch: filing carries the whole move with it. */
  function fileTo(id: string, region: string): void {
    const next = { ...at, [id]: region };
    for (const other of movingIds) next[other] = region;
    at = next;
    key = region;
  }

  function fileAll(region: string): void {
    if (phase === 'receipt') return;
    const anchor = movingIds[0] ?? items[0]?.id;
    if (anchor) fileTo(anchor, region);
  }

  function toggle(id: string): void {
    if (phase === 'receipt' || !open) return;
    at = { ...at, [id]: at[id] === 'out' ? key : 'out' };
  }

  // ── the drag ───────────────────────────────────────────────────────────
  type Drag = {
    el: HTMLElement;
    id: string;
    grabX: number;
    grabY: number;
    home: number;
    tilt: number;
    sx: number;
    sy: number;
    px: number;
    py: number;
    pt: number;
    moved: boolean;
  };
  let drag: Drag | null = null;
  /** The card being carried, as reactive state: the mark lives in the markup. */
  let carrying = $state<string | null>(null);
  let zTop = 10;

  /** vaul's damping, floored at 0: past the tray's own edge the card is
      rubber-banded rather than clamped, so the edge is a fact the hand feels. */
  function dampen(v: number): number {
    const d = 8 * (Math.log(Math.abs(v) + 1) - 2);
    return (d > 0 ? d : 0) * (v < 0 ? -1 : 1);
  }
  function rubber(v: number, lo: number, hi: number): number {
    if (v < lo) return lo - dampen(lo - v);
    if (v > hi) return hi + dampen(v - hi);
    return v;
  }

  function zoneAt(x: number, y: number): string | null {
    let hit: string | null = null;
    for (const date of dates) {
      const mouth = mouths[date.key];
      if (!mouth) continue;
      const r = mouth.getBoundingClientRect();
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) hit = date.key;
    }
    const fr = floorEl?.getBoundingClientRect();
    if (fr && x >= fr.left && x <= fr.right && y >= fr.top && y <= fr.bottom) hit = 'out';
    return hit;
  }

  function down(event: PointerEvent, id: string): void {
    if (phase === 'receipt' || !open) return;
    const el = event.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const stay = items.filter((item) => at[item.id] === 'out');
    drag = {
      el,
      id,
      grabX: event.clientX - rect.left,
      grabY: event.clientY - rect.top,
      home: at[id] === 'out' ? SPLAY[stay.findIndex((item) => item.id === id) % SPLAY.length] : 0,
      tilt: 0,
      sx: event.clientX,
      sy: event.clientY,
      px: event.clientX,
      py: event.clientY,
      pt: performance.now(),
      moved: false,
    };
    carrying = id;
    zTop += 1;
    el.style.zIndex = String(zTop);
    el.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  }

  function move(event: PointerEvent): void {
    if (!drag || !holdEl) return;
    const now = performance.now();
    const vx = (event.clientX - drag.px) / Math.max(1, now - drag.pt);
    drag.px = event.clientX;
    drag.py = event.clientY;
    drag.pt = now;
    if (Math.abs(event.clientX - drag.sx) + Math.abs(event.clientY - drag.sy) > 4) drag.moved = true;
    if (!drag.moved) return;
    const hr = holdEl.getBoundingClientRect();
    const width = drag.el.offsetWidth || 240;
    const x = rubber(event.clientX - hr.left - drag.grabX, 0, hr.width - width);
    const y = rubber(event.clientY - hr.top - drag.grabY, 0, hr.height - (drag.el.offsetHeight || 56));
    drag.el.style.left = `${Math.round(x)}px`;
    drag.el.style.top = `${Math.round(y)}px`;
    // The tilt: velocity → angle, quartered over a hole so the card squares up
    // as it comes over the place it would land.
    const zone = zoneAt(event.clientX, event.clientY);
    const target = Math.max(-10, Math.min(10, vx * 6)) * (zone && zone !== 'out' ? 0.25 : 1);
    drag.tilt += (target - drag.tilt) * 0.3;
    drag.el.style.setProperty('--rot', `${(drag.home + drag.tilt).toFixed(2)}deg`);
    over = zone;
  }

  function up(): void {
    if (!drag) return;
    const d = drag;
    drag = null;
    carrying = null;
    over = null;
    if (d.moved) {
      const zone = zoneAt(d.px, d.py);
      if (zone === 'out') at = { ...at, [d.id]: 'out' };
      else if (zone) fileTo(d.id, zone);
      else d.el.style.setProperty('--rot', `${d.home}deg`);
    } else if (at[d.id] !== 'out') {
      d.el.style.setProperty('--rot', '0deg');
    }
  }

  // ── the keyboard's own path: a roving radio group of holes ─────────────
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
    fileAll(dates[to].key);
    trayEl?.querySelectorAll<HTMLElement>('.xa-slot__head')[to]?.focus();
  }

  // ── the write ──────────────────────────────────────────────────────────
  function reduced(): boolean {
    return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  }

  async function commit(): Promise<void> {
    if (busy) return;
    if (phase === 'receipt') {
      undo();
      return;
    }
    if (movingIds.length === 0 || !chosen) return;
    const before = { at: { ...at }, key };
    busy = true;
    const landed = await onCommit([...movingIds], chosen.iso);
    busy = false;
    if (!landed) return;
    snapshot = before;
    phase = 'receipt';
  }

  function undo(): void {
    onUndo();
    if (snapshot) {
      at = { ...snapshot.at };
      key = snapshot.key;
    }
    phase = 'live';
  }

  /** Lower the tray, then let the panel unmount it: the exit is the design's. */
  function dismiss(): void {
    if (!open) return;
    open = false;
    globalThis.setTimeout(onClose, reduced() ? 0 : 260);
  }

  let armed = $state(false);
  $effect(() => {
    // **Armed one tick after mount**, exactly as `shell/Sheet.svelte` arms its
    // own dismissal: the click that opened this surface mounts it inside that
    // click's dispatch, and a listener registered now would take the same event
    // and lower the tray the student just raised.
    const id = setTimeout(() => (armed = true), 0);
    return () => clearTimeout(id);
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') dismiss();
  }
  function onclick(event: MouseEvent): void {
    if (!armed) return;
    const target = event.target as Element | null;
    if (!target?.closest?.('.xa-tray')) dismiss();
  }
</script>

<svelte:window {onkeydown} {onclick} />

<div class="sca-layer">
  <div
    class="xa-tray"
    data-open={open}
    data-status={phase}
    bind:this={trayEl}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
  >
    <header class="xa-head">
      <button class="xa-grip" type="button" aria-label="Put the tray away" onclick={dismiss}></button>
      <span class="xa-title">{title}</span>
      <span class="xa-sp"></span>
      <span class="xa-count num">{note}</span>
      <button class="cd-iconbtn" type="button" aria-label="Close" onclick={dismiss}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6L6 18"/></svg>
      </button>
      <!-- The surface's own design switch, last in the head row: the tray is one
           thing in three designs, and its head is where the student can say so. -->
    </header>

    <div class="xa-work">
      <div class="xa-wall" role="radiogroup" aria-label="They come back on">
        {#each dates as date, index (date.key)}
          {@const held = items.filter((item) => at[item.id] === date.key).length}
          {@const room = caps[date.key] ?? held}
          <div class="xa-slot" data-on={phase !== 'receipt' && held > 0 && date.key === key} data-over={over === date.key}>
            <button
              class="xa-slot__head"
              type="button"
              role="radio"
              aria-checked={phase !== 'receipt' && held > 0 && date.key === key}
              tabindex={date.key === key ? 0 : -1}
              onclick={() => fileAll(date.key)}
              onkeydown={(event) => rove(event, index)}
            >
              <span class="cd-datetile num">{date.tile}</span>
              <span class="xa-slot__id"><span class="xa-slot__name">{date.name}</span></span>
              <span class="xa-slot__cap num">{held} of {total}</span>
            </button>
            {#if held > room}
              <p class="xa-slot__more num">+{held - room} more</p>
            {/if}
            <div class="xa-mouth" bind:this={mouths[date.key]}></div>
          </div>
        {/each}
      </div>

      <div class="xa-bed">
        <div class="xa-floor" data-on={staying > 0} data-over={over === 'out'} bind:this={floorEl}>
          <span class="xa-floor__lab">Staying put <b>{staying}</b></span>
        </div>
      </div>

      <div class="xa-hold" bind:this={holdEl}>
        {#each items as item (item.id)}
          <button
            class="xa-card"
            class:is-drag={carrying === item.id}
            class:is-off={!shown(item.id)}
            type="button"
            bind:this={cards[item.id]}
            aria-pressed={at[item.id] !== 'out'}
            onpointerdown={(event) => down(event, item.id)}
            onpointermove={move}
            onpointerup={up}
            onpointercancel={up}
            onclick={(event) => {
              // A pointer click does nothing — the card is carried, not pressed.
              // The keyboard's activation (detail 0) takes it out of the move.
              if (event.detail === 0) toggle(item.id);
            }}
          >
            <span class="xa-card__grip" aria-hidden="true">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false">
                <circle cx="9" cy="6" r="1.5"/><circle cx="15" cy="6" r="1.5"/>
                <circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/>
                <circle cx="9" cy="18" r="1.5"/><circle cx="15" cy="18" r="1.5"/>
              </svg>
            </span>
            <span class="xa-card__body">
              <span class="xa-card__t">{item.label}</span>
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

      <div class="xa-band" role="status" inert={phase !== 'receipt'}>
        <div class="xa-band__row">
          <span class="cd-datetile num">{chosen?.tile}</span>
          <span class="xa-band__txt">
            <b class="x-receipt__t">{bandLine}</b>
            <span class="x-receipt__s">Review history is untouched — this writes the day they come back</span>
          </span>
        </div>
      </div>
    </div>

    <footer class="xa-foot">
      <span class="xa-state num">{footState}</span>
      <button
        class="cd-pill xa-commit"
        type="button"
        data-command="record.defer"
        data-placement="reviews.panel"
        aria-disabled={movingIds.length === 0 && phase !== 'receipt'}
        aria-busy={busy}
        onclick={() => void commit()}
      >
        {busy ? 'Moving…' : footPill}
      </button>
    </footer>
  </div>
</div>

<style>
  /* The layer is the window's; the surface owns the floor. It holds no fixed
     child, so the container query can live here and the surface reflows with
     the room it is in rather than with the window. */
  .sca-layer {
    position: fixed;
    inset: 0;
    z-index: 45;
    pointer-events: none;
    container-type: inline-size;
  }

  .xa-tray {
    /* The wall's own geometry, declared once — a head, the run's room, and the
       padding — so the fold to the receipt is a change of one number. */
    --xa-wall: calc(44px * var(--ui-s) + var(--space-xs) + 252px * var(--ui-s) + 3 * var(--space-xs));
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    pointer-events: auto;
    display: grid;
    grid-template-rows: calc(56px * var(--ui-s)) auto calc(56px * var(--ui-s));
    background: var(--card);
    border-radius: var(--r-card) var(--r-card) 0 0;
    box-shadow: var(--sh-3), var(--catch);
    transition: transform var(--dur-3) var(--ease);
  }
  .xa-tray[data-open='false'] {
    transform: translateY(100%);
  }

  .xa-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 0 var(--space-xl);
  }
  /* The grip is a plain control: a press lowers the tray. (A tray that slid
     under the finger read as noise — the lab measured that and dropped it.) */
  .xa-grip {
    flex: none;
    width: var(--hit);
    height: var(--hit);
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    color: var(--ink-3);
    cursor: pointer;
    transition: color var(--dur-1) var(--ease);
  }
  .xa-grip::before {
    content: '';
    width: 22px;
    height: 3px;
    border-radius: var(--r-pill);
    background: currentColor;
  }
  .xa-grip:hover {
    color: var(--ink-2);
  }
  .xa-title {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink);
    letter-spacing: var(--track-title);
  }
  .xa-sp {
    flex: 1;
  }
  .xa-count,
  .xa-state {
    font-size: var(--text-2xs);
    color: var(--ink-2);
  }
  .num {
    font-variant-numeric: tabular-nums;
  }

  .xa-work {
    position: relative;
    display: grid;
    grid-template-rows: var(--xa-wall) auto;
    transition: grid-template-rows var(--dur-3) var(--ease);
  }
  .xa-tray[data-status='receipt'] .xa-work {
    grid-template-rows: calc(92px * var(--ui-s)) auto;
  }

  .xa-wall {
    grid-area: 1 / 1;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-md);
    padding: var(--space-xs) var(--space-xl);
    background: var(--well);
    transition: opacity var(--dur-2) var(--ease), visibility var(--dur-2) var(--ease);
  }
  .xa-slot {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    gap: var(--space-xs);
    min-width: 0;
  }
  .xa-slot__head {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    min-height: calc(44px * var(--ui-s));
    width: 100%;
    padding: 0;
    border-radius: var(--r-item);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .xa-slot__head:hover {
    background: var(--card);
  }
  .xa-slot__head:active {
    background: var(--well-2);
  }
  .xa-slot__id {
    display: grid;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .xa-slot__name {
    font-size: calc(var(--text-sm) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-2);
    letter-spacing: var(--track-title);
  }
  .xa-slot__cap,
  .xa-slot__more {
    font-size: var(--text-2xs);
    color: var(--ink-2);
    flex: none;
  }
  .xa-slot__more {
    margin: 0;
  }
  /* The chosen hole is the app's own selection fill plus the ink ring the
     system rings chosen checks with — never a solid plate. */
  .xa-slot[data-on='true'] .xa-slot__head {
    background: var(--well-2);
  }
  .xa-slot[data-on='true'] .xa-slot__name {
    color: var(--ink);
  }
  .xa-slot[data-on='true'] .cd-datetile {
    background-color: var(--well-2);
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }
  .xa-mouth {
    position: relative;
    min-height: calc(252px * var(--ui-s));
    border: 1.5px dashed var(--rule-strong);
    border-radius: var(--r-tile);
    background: var(--well);
    transition: border-color var(--dur-1) var(--ease), background var(--dur-1) var(--ease);
  }
  .xa-slot[data-over='true'] .xa-mouth {
    border-color: var(--ink);
    background: var(--card);
  }
  .xa-slot[data-on='true'] .xa-mouth {
    border-color: transparent;
    background: var(--well-2);
  }
  .xa-tray[data-status='receipt'] .xa-mouth {
    min-height: 0;
  }

  .xa-bed {
    grid-area: 2 / 1;
    display: grid;
    padding: var(--space-xs) var(--space-xl) var(--space-md);
    background: var(--well);
  }
  .xa-floor {
    position: relative;
    min-height: calc(100px * var(--ui-s));
    border: 1.5px dashed var(--rule-strong);
    border-radius: var(--r-tile);
    background: var(--well);
    transition: border-color var(--dur-1) var(--ease), background var(--dur-1) var(--ease),
      min-height var(--dur-3) var(--ease);
  }
  .xa-floor[data-over='true'] {
    border-color: var(--ink);
    background: var(--card);
  }
  .xa-floor[data-on='true'] {
    border-color: transparent;
    background: var(--well-2);
  }
  .xa-tray[data-status='receipt'] .xa-floor {
    min-height: 0;
    padding: var(--space-xs);
  }
  .xa-floor__lab {
    position: absolute;
    left: var(--space-xs);
    top: var(--space-xs);
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2xs);
    /* The floor's caption sits on `--well-2`, where the muted inks miss 4.5:1:
       it takes `--ink`. */
    color: var(--ink);
    font-size: var(--text-xs);
  }
  .xa-floor__lab b {
    font-family: var(--font-display);
    font-weight: var(--weight-display);
  }

  /* The cards. The layer is the tray's work area, so a home is a rect read off
     the hole or the floor, and the drop animates because the transition rides
     left/top/transform. */
  .xa-hold {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
  }
  .xa-card {
    position: absolute;
    width: min(240px, calc((100cqw - 128px) / 3));
    height: calc(56px * var(--ui-s));
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-xs);
    padding: 0 var(--space-sm);
    min-width: 0;
    pointer-events: auto;
    background: var(--card);
    border-radius: var(--r-tile);
    box-shadow: var(--sh-1), var(--catch);
    text-align: left;
    cursor: grab;
    touch-action: none;
    transform: rotate(var(--rot, 0deg));
    transform-origin: 50% 62%;
    transition: left var(--dur-2) var(--ease), top var(--dur-2) var(--ease),
      transform var(--dur-2) var(--ease), box-shadow var(--dur-1) var(--ease),
      opacity var(--dur-2) var(--ease);
  }
  .xa-card:hover {
    box-shadow: var(--sh-2), var(--catch);
  }
  .xa-card.is-drag {
    z-index: 90;
    cursor: grabbing;
    box-shadow: var(--sh-pop), var(--catch);
    transition: box-shadow var(--dur-1) var(--ease), opacity var(--dur-2) var(--ease);
    transform: rotate(var(--rot, 0deg)) scale(1.02);
  }
  /* A card past its region's capacity, and every card once the receipt lands:
     out of the drawing and out of the tab order. */
  .xa-card.is-off {
    visibility: hidden;
    pointer-events: none;
  }
  .xa-card__grip {
    width: 16px;
    height: 24px;
    display: grid;
    place-items: center;
    color: var(--ink-3);
    transition: color var(--dur-1) var(--ease);
  }
  .xa-card:hover .xa-card__grip {
    color: var(--ink-2);
  }
  .xa-card__body {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .xa-card__t {
    font-size: calc(var(--text-sm) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink);
    letter-spacing: var(--track-title);
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  /* The commit is the tray's one ink object, in the fixed foot; on the receipt
     it is the same pill in the same seat wearing Undo. */
  .xa-foot {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 0 var(--space-xl);
    border-top: 1px solid var(--rule);
  }
  .xa-state {
    flex: 1;
    min-width: 0;
  }
  .xa-commit[aria-disabled='true'] {
    opacity: 0.45;
    pointer-events: none;
  }

  /* The receipt: the back wall's own cell, with the wall fading under it. */
  .xa-band {
    grid-area: 1 / 1;
    z-index: 4;
    display: grid;
    align-items: center;
    min-width: 0;
    padding: var(--space-md) var(--space-xl);
    background: var(--well);
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition: opacity var(--dur-2) var(--ease), visibility var(--dur-2) var(--ease);
  }
  .xa-band__row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    min-width: 0;
  }
  .xa-band__txt {
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
  .xa-tray[data-status='receipt'] .xa-band {
    opacity: 1;
    visibility: visible;
    pointer-events: auto;
  }
  .xa-tray[data-status='receipt'] .xa-wall {
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
  }
  @keyframes xa-receipt-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .xa-tray[data-status='receipt'] .xa-band__row {
    animation: xa-receipt-in var(--dur-2) var(--ease-pop) both;
  }

  /* A narrow room: three holes keep their row, but each mouth gives up its
     depth (and says so, with `+n more`) rather than the wall stacking into a
     scroll region the absolutely-positioned run could not follow. */
  @container (max-width: 620px) {
    .xa-tray {
      --xa-wall: calc(44px * var(--ui-s) + var(--space-xs) + 140px * var(--ui-s) + 3 * var(--space-xs));
    }
    .xa-mouth {
      min-height: calc(140px * var(--ui-s));
    }
    .xa-floor {
      min-height: calc(72px * var(--ui-s));
    }
    .xa-slot__head {
      flex-wrap: wrap;
    }
    .xa-title {
      font-size: var(--text-2xs);
    }
  }
</style>
