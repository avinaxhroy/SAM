<!--
  Catchup sheet variant B: The Docked Drawer.
  Right-docked sliding drawer displaying overdue cards on a date offset timeline scale (§11).
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
  let open = $state(false);
  let entering = $state(false);
  let snapshot: { picked: Record<string, boolean>; key: string } | null = null;

  const chosen = $derived(dates.find((date) => date.key === key) ?? home);
  const pickedIds = $derived(items.filter((item) => picked[item.id]).map((item) => item.id));
  const count = $derived(pickedIds.length);
  const behind = $derived(total - count);

  const headSub = $derived(
    phase === 'receipt' ? plural(count, 'recall', 'recalls') + ' filed' : `${total} overdue ${total === 1 ? 'recall' : 'recalls'}`,
  );
  const footNote = $derived(
    phase === 'receipt'
      ? `${chosen?.full}${behind > 0 ? ` · ${behind} still behind` : ''}`
      : `${count} of ${total} selected · ${chosen?.short}`,
  );
  const pill = $derived(
    phase === 'receipt'
      ? 'Undo the move'
      : count > 0
        ? `Move ${plural(count, 'recall', 'recalls')}`
        : 'Pick a record first',
  );
  const bandLine = $derived(arrivalLine(count, chosen?.full ?? ''));

  /** The tube's fill: one unit per day, so 1/3/7 days is 14/43/100 percent. */
  const fill = $derived(chosen ? `${((chosen.days / 7) * 100).toFixed(1)}%` : '0%');
  /** Where a stop sits on the rail, as a fraction of the run (0–1). */
  function at(days: number): string {
    return String(days / 7);
  }

  function toggle(id: string): void {
    if (phase === 'receipt' || !open) return;
    picked = { ...picked, [id]: !picked[id] };
  }

  function choose(region: string): void {
    if (phase === 'receipt' || !open) return;
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
    stopsEl?.querySelectorAll<HTMLElement>('.xb-stop')[to]?.focus();
  }
  let stopsEl = $state<HTMLElement | null>(null);

  function reduced(): boolean {
    return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  }

  async function commit(): Promise<void> {
    if (busy) return;
    if (phase === 'receipt') {
      onUndo();
      if (snapshot) {
        picked = { ...snapshot.picked };
        key = snapshot.key;
      }
      phase = 'live';
      return;
    }
    if (count === 0 || !chosen) return;
    const before = { picked: { ...picked }, key };
    busy = true;
    const landed = await onCommit([...pickedIds], chosen.iso);
    busy = false;
    if (!landed) return;
    snapshot = before;
    phase = 'receipt';
  }

  function dismiss(): void {
    if (!open) return;
    open = false;
    globalThis.setTimeout(onClose, reduced() ? 0 : 340);
  }

  let armed = $state(false);
  $effect(() => {
    // The first arrival: the drawer travels in from the window's edge behind a
    // bulged leading edge, and the bulge collapses to nothing when `entering`
    // clears — arrives curved, settles straight. Never re-run afterwards.
    const raf = requestAnimationFrame(() => {
      open = true;
      entering = true;
    });
    const settle = setTimeout(() => (entering = false), reduced() ? 0 : 460);
    const arm = setTimeout(() => (armed = true), 0);
    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(settle);
      clearTimeout(arm);
    };
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') dismiss();
  }
  function onclick(event: MouseEvent): void {
    if (!armed) return;
    const target = event.target as Element | null;
    if (!target?.closest?.('.xb-drawer')) dismiss();
  }
</script>

<svelte:window {onkeydown} {onclick} />

<div class="scb-layer">
  <div
    class="xb-drawer"
    data-open={open}
    data-enter={entering}
    data-status={phase}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
  >
    <span class="xb-lens" aria-hidden="true"></span>

    <div class="xb-panel">
      <header class="xb-head">
        <div class="xb-head__text">
          <span class="xb-head__t">{title}</span>
          <span class="xb-head__s num">{headSub}</span>
        </div>
        <button class="cd-iconbtn" type="button" aria-label="Close" onclick={dismiss}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6L6 18"/></svg>
        </button>
        <!-- The drawer's own head, and the surface's design switch at its end. -->
      </header>

      <div class="xb-body" inert={phase === 'receipt'}>
        <div class="xb-deck" aria-label="Recalls on the move">
          {#each items as item (item.id)}
            <button
              class="xb-tile"
              type="button"
              aria-pressed={picked[item.id] ?? false}
              onclick={() => toggle(item.id)}
            >
              <span class="xb-tile__top">
                <span class="xb-tile__t">{item.label}</span>
                <span class="cd-check" aria-hidden="true">
                  {#if picked[item.id]}
                    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.5 8.4 6.6 11.4 12.5 5"/></svg>
                  {/if}
                </span>
              </span>
              <span class="x-facts">
                {#if item.course}
                  <span class="cd-chip cd-chip--wash" data-w={item.wash ?? undefined}>{item.course}</span>
                {/if}
                {#if item.late}
                  <span class="cd-chip cd-chip--overdue">{item.late}</span>
                {/if}
              </span>
            </button>
          {/each}
        </div>

        <div class="xb-rail" bind:this={stopsEl} role="radiogroup" aria-label="They come back on">
          <span class="xb-tube"><span class="xb-tube__fill" style={`height: ${fill}`}></span></span>
          {#each dates as date, index (date.key)}
            {#if index > 0}
              <span class="xb-gap num" style={`--at: ${(dates[index - 1].days + date.days) / 2 / 7}`}>
                +{date.days - dates[index - 1].days} days
              </span>
            {/if}
            <span class="xb-tick" data-on={date.key === key && phase !== 'receipt'} style={`--at: ${at(date.days)}`}></span>
          {/each}
          {#each dates as date, index (date.key)}
            <button
              class="xb-stop"
              type="button"
              role="radio"
              aria-checked={date.key === key && phase !== 'receipt'}
              aria-label={`${date.name} — ${date.full}`}
              tabindex={date.key === key ? 0 : -1}
              style={`--at: ${at(date.days)}`}
              onclick={() => choose(date.key)}
              onkeydown={(event) => rove(event, index)}
            >
              <span class="cd-datetile num">{date.tile}</span>
              <span class="xb-stop__txt">
                <span class="xb-stop__t">{date.name}</span>
                <span class="xb-stop__s">{date.full}</span>
              </span>
            </button>
          {/each}
        </div>
      </div>

      <div class="xb-receipt" role="status" inert={phase !== 'receipt'}>
        <div class="xb-receipt__row">
          <span class="cd-datetile num">{chosen?.tile}</span>
          <span class="xb-receipt__txt">
            <b class="x-receipt__t">{bandLine}</b>
            <span class="x-receipt__s">Review history is untouched — this writes the day they come back</span>
          </span>
        </div>
      </div>

      <footer class="xb-foot">
        <span class="xb-foot__note num">{footNote}</span>
        <button
          class="cd-pill xb-commit"
          type="button"
          data-command="record.defer"
          data-placement="reviews.panel"
          aria-disabled={count === 0 && phase !== 'receipt'}
          aria-busy={busy}
          onclick={() => void commit()}
        >
          {busy ? 'Moving…' : pill}
        </button>
      </footer>
    </div>
  </div>
</div>

<style>
  .scb-layer {
    position: fixed;
    inset: 0;
    z-index: 50;
    pointer-events: none;
    container-type: inline-size;
  }

  /* The drawer's visible width IS its content: the panel fills the slab and the
     lens bulges outside its leading edge. */
  .xb-drawer {
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    width: min(384px, 100cqw);
    pointer-events: auto;
    box-shadow: var(--sh-3), var(--catch);
    transition: transform var(--dur-3) var(--ease);
  }
  .xb-drawer[data-open='false'] {
    transform: translateX(calc(100% + 12px));
  }

  /* The lens: it bulges left of the panel while the drawer travels, then
     collapses to nothing when `data-enter` clears. */
  .xb-lens {
    position: absolute;
    right: 100%;
    top: 0;
    bottom: 0;
    width: 0;
    pointer-events: none;
    background: var(--card);
    border-radius: calc(168px * var(--ui-s)) 0 0 calc(168px * var(--ui-s)) / 50% 0 0 50%;
    box-shadow: inherit;
    transition: width var(--dur-3) var(--ease);
  }
  .xb-drawer[data-open='true'][data-enter='true'] .xb-lens {
    width: calc(168px * var(--ui-s));
    transition: none;
  }

  .xb-panel {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: var(--card);
  }
  /* The head is a band of the drawer closed by a hairline, as the reference
     closes its own header. */
  .xb-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-lg) var(--space-lg) var(--space-md);
    border-bottom: 1px solid var(--rule);
  }
  .xb-head__text {
    display: grid;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .xb-head__t {
    font-size: calc(var(--text-lg) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink);
    letter-spacing: var(--track-title);
  }
  .xb-head__s {
    font-size: var(--text-xs);
    color: var(--ink-2);
  }
  .num {
    font-variant-numeric: tabular-nums;
  }

  .xb-body {
    grid-area: 2 / 1;
    min-height: 0;
    overflow: auto;
    padding: var(--space-md) var(--space-lg);
    display: grid;
    gap: var(--space-md);
    align-content: start;
  }
  /* A 2×2 deck of tiles in a recessed tray: the batch is legible as objects. */
  .xb-deck {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-xs);
    padding: var(--space-xs);
    background: var(--well);
    border-radius: var(--r-tile);
  }
  .xb-tile {
    display: grid;
    gap: var(--space-2xs);
    min-height: calc(80px * var(--ui-s));
    padding: var(--space-xs) var(--space-sm);
    min-width: 0;
    text-align: left;
    border-radius: var(--r-item);
    background: transparent;
    transition: background var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .xb-tile:hover {
    background: var(--well-2);
  }
  .xb-tile:active {
    background: var(--well-2);
    transform: scale(0.99);
  }
  /* A chosen tile stands proud of the tray; an unchosen one lies flat in it. */
  .xb-tile[aria-pressed='true'] {
    background: var(--card);
    box-shadow: var(--sh-1);
  }
  .xb-tile[aria-pressed='true']:hover {
    box-shadow: var(--sh-2);
  }
  .xb-tile__top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--space-xs);
    align-items: start;
  }
  .xb-tile__t {
    font-size: calc(var(--text-sm) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-2);
    letter-spacing: var(--track-title);
    min-width: 0;
  }
  .xb-tile[aria-pressed='true'] .xb-tile__t {
    color: var(--ink);
  }
  /* The check only confirms the tile's own state: a modest 18px ink ring. */
  .xb-tile .cd-check {
    width: 18px;
    height: 18px;
    margin-top: -1px;
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

  /* The rail: distance is the layout. One unit per day, a tick at every stop,
     and the chosen stop's own tile wears the ink ring. */
  .xb-rail {
    --rail-top: calc(24px * var(--ui-s));
    --rail-span: calc(232px * var(--ui-s));
    position: relative;
    height: calc(280px * var(--ui-s));
  }
  .xb-tube {
    position: absolute;
    left: 0;
    top: var(--rail-top);
    bottom: var(--rail-top);
    width: 6px;
    border-radius: var(--r-pill);
    background: var(--well-2);
    overflow: hidden;
  }
  .xb-tube__fill {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 0;
    border-radius: var(--r-pill);
    background: var(--ink);
    transition: height var(--dur-3) var(--ease);
  }
  /* The tick is centred on the tube (6px at left 0 → centre 3). */
  .xb-tick {
    position: absolute;
    left: -1px;
    width: 8px;
    height: 2px;
    border-radius: var(--r-pill);
    background: var(--rule-strong);
    pointer-events: none;
    transition: background var(--dur-1) var(--ease);
    top: calc(var(--rail-top) + var(--at) * var(--rail-span) - 1px);
  }
  .xb-tick[data-on='true'] {
    background: var(--ink);
  }
  .xb-stop {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(var(--rail-top) + var(--at) * var(--rail-span) - calc(24px * var(--ui-s)));
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
    min-height: calc(48px * var(--ui-s));
    padding: 0 var(--space-2xs) 0 var(--space-lg);
    border-radius: var(--r-item);
    text-align: left;
    transition: background var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .xb-stop:hover {
    background: var(--well);
  }
  .xb-stop:active {
    background: var(--well-2);
    transform: scale(0.99);
  }
  .xb-stop[aria-checked='true'] {
    background: var(--well-2);
  }
  .xb-stop[aria-checked='true'] .cd-datetile {
    background-color: var(--well-2);
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }
  .xb-stop__txt {
    min-width: 0;
  }
  .xb-stop__t {
    display: block;
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-2);
    letter-spacing: var(--track-title);
  }
  .xb-stop[aria-checked='true'] .xb-stop__t {
    color: var(--ink);
  }
  .xb-stop__s {
    display: block;
    margin-top: 1px;
    font-size: var(--text-2xs);
    color: var(--ink-2);
  }
  .xb-gap {
    position: absolute;
    left: var(--space-lg);
    top: calc(var(--rail-top) + var(--at) * var(--rail-span) - 8px);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2xs);
    color: var(--ink-2);
    font-size: var(--text-2xs);
    pointer-events: none;
  }
  .xb-gap::before {
    content: '';
    width: 10px;
    height: 1.5px;
    border-radius: var(--r-pill);
    background: var(--rule-strong);
  }

  .xb-foot {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-height: calc(64px * var(--ui-s));
    padding: var(--space-xs) var(--space-lg);
    border-top: 1px solid var(--rule);
  }
  .xb-foot__note {
    flex: 1;
    min-width: 0;
    font-size: var(--text-xs);
    color: var(--ink-2);
  }
  .xb-commit[aria-disabled='true'] {
    opacity: 0.45;
    pointer-events: none;
  }

  .xb-drawer[data-status='receipt'] .xb-body {
    display: none;
  }
  .xb-receipt {
    grid-area: 2 / 1;
    display: none;
    align-content: center;
    padding: var(--space-lg);
  }
  .xb-drawer[data-status='receipt'] .xb-receipt {
    display: grid;
  }
  .xb-receipt__row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    min-width: 0;
  }
  .xb-receipt__txt {
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
  @keyframes xb-receipt-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .xb-drawer[data-status='receipt'] .xb-receipt__row {
    animation: xb-receipt-in var(--dur-2) var(--ease-pop) both;
  }

  /* A narrow room: the deck keeps two columns (each tile is small), and at a
     phone-width window the drawer becomes the window rather than a 384px slab
     with a bulge that has no edge to bulge off. */
  @container (max-width: 560px) {
    .xb-lens {
      display: none;
    }
    .xb-deck {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
