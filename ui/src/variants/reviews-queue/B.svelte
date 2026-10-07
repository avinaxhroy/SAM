<!-- Reviews queue variant B: The Board. Four-column kanban layout by overdue window. -->
<script lang="ts">
  import { tick } from 'svelte';
  import type { QueueProps } from './props';

  let { groups, rows, moved, onDefer, onUndo }: QueueProps = $props();

  let board = $state<HTMLElement | null>(null);
  let busy = $state(false);

  const reduced = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /** The reference's spring, sampled into a CSS `linear()` — a consequence of
   *  its own three numbers (ζ 0.80, 1.5% overshoot, settled in 360ms). */
  function springCurve(): string {
    const stiffness = 350;
    const damping = 30;
    const mass = 1;
    const w0 = Math.sqrt(stiffness / mass);
    const zeta = damping / (2 * Math.sqrt(stiffness * mass));
    const damped = w0 * Math.sqrt(1 - zeta * zeta);
    const settle = 0.36;
    const stops: string[] = [];
    for (let step = 0; step <= 24; step += 1) {
      const progress = step / 24;
      const at = progress * settle;
      const value =
        1 - Math.exp(-zeta * w0 * at) * (Math.cos(damped * at) + ((zeta * w0) / damped) * Math.sin(damped * at));
      stops.push(`${Math.max(0, Math.round(value * 1000) / 1000)}${step ? ` ${Math.round(progress * 10000) / 100}` : ''}`);
    }
    return `linear(${stops.join(', ')})`;
  }

  const SPRING = springCurve();

  function cards(): HTMLElement[] {
    return board ? [...board.querySelectorAll<HTMLElement>('[data-id]')] : [];
  }

  function rects(): Map<string, DOMRect> {
    const out = new Map<string, DOMRect>();
    for (const el of cards()) out.set(el.dataset.id ?? '', el.getBoundingClientRect());
    return out;
  }

  /** Only a tray that receives a record moves, and only enough to show it land:
   *  the tray the record came from keeps the reader's scroll, however far the
   *  re-sort carried the card inside it. */
  function revealReceiver(id: string, source: string | null): void {
    // Record ids are plan text: escape them or a crafted id rewrites the
    // selector (SourcePane's `quote()` exists for exactly this pattern).
    const card = board?.querySelector<HTMLElement>(
      `.rqb-card[data-id="${CSS.escape(id)}"]`,
    );
    const tray = card?.closest<HTMLElement>('.rqb-tray');
    const here = card.getBoundingClientRect();
    const box = tray.getBoundingClientRect();
    if (here.top < box.top) tray.scrollTop -= box.top - here.top + 8;
    else if (here.bottom > box.bottom) tray.scrollTop += here.bottom - box.bottom + 8;
  }

  /** The flight: a fixed-position copy of the record at its old rect, released
   *  onto the new one, while the live node waits under it. */
  function fly(id: string, from: DOMRect, stand: HTMLElement): void {
    const to = stand.getBoundingClientRect();
    stand.style.opacity = '0';
    const ghost = stand.cloneNode(true) as HTMLElement;
    ghost.classList.add('rqb-ghost');
    ghost.removeAttribute('data-id');
    ghost.style.left = `${to.left}px`;
    ghost.style.top = `${to.top}px`;
    ghost.style.width = `${to.width}px`;
    ghost.style.height = `${to.height}px`;
    ghost.style.transform = `translate(${from.left - to.left}px,${from.top - to.top}px) rotate(2.5deg) scale(1.04)`;
    document.body.appendChild(ghost);
    void ghost.offsetWidth;
    ghost.style.transform = 'translate(0,0) rotate(0) scale(1)';
    window.setTimeout(() => {
      ghost.remove();
      if (!board?.contains(stand)) return;
      stand.style.opacity = '';
      stand.setAttribute('data-land', '');
      window.setTimeout(() => stand.removeAttribute('data-land'), 280);
    }, 380);
  }

  /** One action, then the whole board re-laid. */
  async function defer(id: string): Promise<void> {
    if (busy) return;
    busy = true;
    const before = rects();
    const from = before.get(id) ?? null;
    const source = board?.querySelector<HTMLElement>(`.rqb-card[data-id="${CSS.escape(id)}"]`)?.closest<HTMLElement>('.rqb-tray');
    await onDefer(id, 1);
    await tick();
    revealReceiver(id, source?.dataset.tray ?? null);
    if (board && !reduced) {
      const after = new Map<string, DOMRect>();
      const shifted: Array<{ el: HTMLElement; from: DOMRect }> = [];
      for (const el of cards()) {
        const key = el.dataset.id ?? '';
        const box = el.getBoundingClientRect();
        after.set(key, box);
        const was = before.get(key);
        if (was && (Math.abs(was.left - box.left) > 1 || Math.abs(was.top - box.top) > 1)) {
          shifted.push({ el, from: was });
        }
      }
      for (const { el, from: was } of shifted) {
        const box = el.getBoundingClientRect();
        el.style.transform = `translate(${was.left - box.left}px,${was.top - box.top}px)`;
      }
      void board.offsetWidth; // commit the inverted frame before releasing it
      for (const { el } of shifted) {
        el.setAttribute('data-flip', '');
        el.classList.add('rqb-move');
        el.style.transform = '';
      }
      window.setTimeout(() => {
        for (const el of board.querySelectorAll<HTMLElement>('.rqb-card[data-flip]')) {
          el.classList.remove('rqb-move');
          el.removeAttribute('data-flip');
        }
      }, 420);
      const escaped = CSS.escape(id);
      const stand = board.querySelector<HTMLElement>(`.rqb-card[data-id="${escaped}"], .rqb-prow[data-id="${escaped}"]`);
      if (from && stand) fly(id, from, stand);
    }
    busy = false;
  }

  async function undo(): Promise<void> {
    if (busy) return;
    await onUndo();
  }
</script>

<div class="v-fit rqb" bind:this={board} style={`--rqb-spring: ${SPRING}`}>
  <div class="rqb-board">
      {#each groups as group, index (group.id)}
        <section class="rqb-lane" data-empty={group.rows.length === 0 ? '' : undefined} aria-label={group.rule}>
          <header class="rqb-lane__head">
            <span class="rqb-lane__n num">{group.rows.length}</span>
            <span class="rqb-lane__l">{group.label}</span>
          </header>
          <div class="rqb-tray" data-tray={index}>
            {#each group.rows as row (row.id)}
              <article class="rqb-card" data-id={row.id} data-w={row.wash}>
                <span class="rqb-age">
                  {#if row.late > 0}
                    <span class="rqb-age__n num">{row.late}</span>
                  {/if}
                  <span class="rqb-age__k">{row.late === 0 ? 'due today' : row.late === 1 ? 'day late' : 'days late'}</span>
                </span>
                <span class="rqb-card__t">{row.title}</span>
                <span class="rqb-foot">
                  {#if row.course}
                    <span class="cd-chip cd-chip--onwash num">{row.course}</span>
                  {/if}
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm rqb-onwash"
                    type="button"
                    aria-label={`Defer ${row.title} by one day`}
                    onclick={() => void defer(row.id)}
                  >
                    Defer 1 day
                  </button>
                </span>
              </article>
            {/each}
          </div>
      </section>
    {/each}
  </div>

  {#if moved.deferred.length > 0}
    <section class="rqb-pocket" aria-label="Deferred today">
      <p class="rqb-pocket__k">
        Deferred today · <b class="num">{moved.deferred.length}</b>
      </p>
      {#each moved.deferred as row (row.id)}
        <div class="rqb-prow" data-id={row.id}>
          <span class="rqb-prow__t">{row.title}</span>
          <span class="rqb-prow__d num">{row.say}</span>
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            aria-label={`Undo the defer on ${row.title}`}
            onclick={() => void undo()}
          >
            Undo
          </button>
        </div>
      {/each}
    </section>
  {/if}

  <p class="cd-sr">{rows.length} records are in the backlog</p>
</div>

<style>
  .rqb {
    width: min(1000px, 100%);
  }
  .rqb-board {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--ui-gap);
  }
  .rqb-lane {
    display: grid;
    gap: var(--ui-gap-sm);
    align-content: start;
    min-width: 0;
  }
  .rqb-lane__head {
    display: grid;
    gap: 1px;
    padding: 0 calc(4px * var(--ui-s));
  }
  .rqb-lane__n {
    font-size: calc(18px * var(--ui-s));
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-title);
    line-height: 1;
  }
  .rqb-lane__l {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .rqb-lane[data-empty] .rqb-lane__n {
    color: var(--ink-3);
  }

  /* The tray is a well: the cards are objects lying on it, never cards on
     cards. Its height is the design's own (two and a half cards), so a lane of
     eighty scrolls — which is what a four-column board costs and the notes say
     so. */
  .rqb-tray {
    display: grid;
    gap: var(--ui-gap-sm);
    align-content: start;
    height: calc(328px * var(--ui-s));
    padding: calc(8px * var(--ui-s));
    border-radius: var(--r-tile);
    background: var(--well);
    overflow-y: auto;
    overflow-anchor: none;
  }

  .rqb-card {
    display: grid;
    gap: calc(4px * var(--ui-s));
    padding: var(--ui-pad-sm) var(--ui-pad-sm) calc(10px * var(--ui-s));
    border-radius: var(--r-tile);
    background-color: var(--wash, var(--well));
    background-image: var(--wash-grad, none);
    color: var(--onwash, var(--ink));
    box-shadow: var(--sh-1);
    transition: box-shadow var(--dur-2) var(--ease);
  }
  .rqb-card:hover {
    box-shadow: var(--sh-2);
  }
  :global(.rqb-card[data-flip]) {
    will-change: transform;
  }
  :global(.rqb-move) {
    transition: transform var(--dur-3) var(--rqb-spring, var(--ease));
  }
  .rqb-age {
    display: flex;
    align-items: baseline;
    gap: calc(4px * var(--ui-s));
  }
  .rqb-age__n {
    font-size: calc(20px * var(--ui-s));
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-title);
    line-height: 1;
  }
  .rqb-age__k {
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    opacity: 0.72;
  }
  .rqb-card__t {
    font-size: var(--ui-text);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.2;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .rqb-foot {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    margin-top: calc(4px * var(--ui-s));
    flex-wrap: wrap;
  }
  /* A pill on a wash: the system's pane recipe, never plain card. */
  .rqb-onwash {
    background: var(--pane);
    color: var(--onwash, var(--ink));
  }
  .rqb-onwash:hover {
    background: color-mix(in oklab, var(--ink-inv) 82%, transparent);
    box-shadow: var(--sh-1);
  }
  .rqb-onwash:active {
    box-shadow: none;
  }

  /* The flight: the record leaves the tray as a fixed-position copy, so no tray
     edge can clip it. */
  :global(.rqb-ghost) {
    position: fixed;
    z-index: 60;
    margin: 0;
    pointer-events: none;
    transition: transform var(--dur-3) var(--rqb-spring, var(--ease));
  }
  @keyframes rqb-land {
    from {
      opacity: 0;
      filter: blur(4px);
    }
    to {
      opacity: 1;
      filter: blur(0);
    }
  }
  :global(.rqb-card[data-land]) .rqb-age,
  :global(.rqb-card[data-land]) .rqb-card__t,
  :global(.rqb-card[data-land]) .rqb-foot,
  :global(.rqb-prow[data-land]) {
    animation: rqb-land var(--dur-2) var(--ease) both;
  }

  .rqb-pocket {
    display: grid;
    gap: calc(4px * var(--ui-s));
    margin-top: var(--ui-gap);
    padding: var(--ui-pad-sm) var(--ui-pad);
    border-radius: var(--r-tile);
    background: var(--well);
  }
  .rqb-pocket__k {
    margin: 0;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .rqb-prow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(44px * var(--ui-s));
  }
  .rqb-prow + .rqb-prow {
    border-top: 1px dashed var(--rule-strong);
  }
  .rqb-prow__t {
    font-size: var(--ui-text);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rqb-prow__d {
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
    white-space: nowrap;
  }

  @container (max-width: 780px) {
    .rqb-board {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .rqb-tray {
      height: calc(300px * var(--ui-s));
    }
  }
  @container (max-width: 460px) {
    .rqb-board {
      grid-template-columns: minmax(0, 1fr);
    }
    .rqb-tray {
      height: auto;
      max-height: calc(300px * var(--ui-s));
    }
  }
</style>
