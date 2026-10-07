<!--
  COMING UP · B · THE REVEAL.
  Scroll reveal timeline showing dated items in chronological order. Uses
  scroll-position-based entrance offsets for incoming rows.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { leadOf, type ComingUpProps } from './props';

  let { rows, planReady, onOpenPlan, onOpen }: ComingUpProps = $props();

  const N = $derived(rows.length);
  const REDUCE = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  let port = $state<HTMLElement | null>(null);
  let frame = 0;

  // Calculate visibility ratio and smoothstep progress for each visible row.
  function measure(): void {
    frame = 0;
    const box = port;
    if (!box || !box.clientHeight) return;
    const top = box.getBoundingClientRect().top;
    const height = box.clientHeight;
    for (const item of box.querySelectorAll<HTMLElement>('.cu-item')) {
      const rect = item.getBoundingClientRect();
      let p = (height - (rect.top - top)) / (rect.height * 0.9);
      p = Math.min(1, Math.max(0, p));
      const arrived = p * p * (3 - 2 * p);
      item.style.setProperty('--p', arrived.toFixed(3));
    }
  }

  function request(): void {
    if (!frame) frame = requestAnimationFrame(measure);
  }

  // Re-measure row visibility after DOM mutations or row changes.
  $effect(() => {
    void rows;
    void port;
    if (REDUCE) return;
    request();
  });

  $effect(() => {
    const onResize = () => request();
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('resize', onResize);
      if (frame) cancelAnimationFrame(frame);
    };
  });
</script>

<section class="v-fit cu-b">
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
    <!-- The card at zero, holding the list's own language drawn empty: a dashed
         rule, the app's own words, and the one way out. -->
    <div class="cu-empty">
      <span class="cu-run" aria-hidden="true"><i class="cu-run__line"></i></span>
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
    </div>
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="cu-port cd-scroll"
      role="region"
      aria-label="Coming up"
      tabindex="0"
      bind:this={port}
      onscroll={request}
    >
      <div class="cu-list">
        {#each rows as row, index (row.id)}
          <button
            class="cu-item"
            type="button"
            data-command="record.panel"
            data-placement="recordTable.rowContext"
            aria-label={`${row.label} — open the record`}
            onclick={() => onOpen(row.id)}
          >
            <span class="cu-item__rank num" aria-hidden="true">{String(index + 1).padStart(2, '0')}</span>
            <span class="cu-item__body">
              <span class="cu-item__t">{row.label}</span>
              <span class="cu-item__meta">
                {#if row.course}
                  <span class="cd-chip cd-chip--code" data-w={row.course.wash}>{row.course.label}</span>
                {/if}
                {#if row.date}
                  <span class="cu-when num">{row.date.time ? `${row.date.long}, ${row.date.time}` : row.date.long}</span>
                {/if}
                {#if leadOf(row) && !row.date?.time}
                  <span class="cu-when num">{leadOf(row)}</span>
                {/if}
                {#if row.state}
                  <span class="cu-state" data-tone={row.state.tone}>{row.state.word}</span>
                {/if}
              </span>
            </span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  .cu-port {
    position: relative;
    max-height: calc(400px * var(--ui-s));
    max-width: min(760px, 100%);
    margin: 0 auto;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--ui-pad);
    border-radius: var(--r-card);
    background: var(--well);
  }
  .cu-port:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .cu-list {
    display: grid;
    gap: var(--ui-gap);
  }

  /* Item with scroll progress driven entrance. */
  .cu-item {
    display: grid;
    grid-template-columns: calc(40px * var(--ui-s)) minmax(0, 1fr);
    gap: var(--ui-gap-lg);
    align-items: center;
    width: 100%;
    min-height: calc(84px * var(--ui-s));
    padding: var(--ui-pad-sm) var(--ui-pad);
    text-align: left;
    background: var(--card);
    border-radius: var(--r-tile);
    box-shadow: var(--sh-1);
    transform: translateY(calc((1 - var(--p, 1)) * 24px));
    opacity: calc(0.35 + var(--p, 1) * 0.65);
    transition:
      box-shadow var(--dur-1) var(--ease),
      background var(--dur-1) var(--ease);
  }
  .cu-item__rank {
    font-size: var(--text-xl);
    font-weight: var(--weight-body);
    letter-spacing: var(--track-title);
    color: var(--ink-3);
  }
  .cu-item__body {
    display: grid;
    gap: var(--space-2xs);
    min-width: 0;
  }
  .cu-item__t {
    font-size: var(--text-md);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cu-item__meta {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-xs);
    min-width: 0;
  }
  .cu-item:hover { box-shadow: var(--sh-2); }
  .cu-item:active {
    background: var(--well);
    box-shadow: var(--sh-1);
  }
  .cu-item:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .cu-when { font-variant-numeric: tabular-nums; color: var(--ink-3); }
  .cu-state {
    font-weight: var(--weight-label);
    color: var(--ink-2);
  }
  .cu-state[data-tone='overdue'] { color: var(--on-overdue); }
  .cu-state[data-tone='risk'] { color: var(--on-risk); }

  /* The empty state: the port's own card at zero, holding the rule the list
     would have drawn — with no ends to name, because a plan with no dates has
     no window to draw. */
  .cu-empty {
    display: grid;
    justify-items: center;
    gap: var(--ui-gap-sm);
    max-width: min(760px, 100%);
    margin: 0 auto;
    padding: var(--ui-gap-lg) var(--ui-pad);
    background: var(--card);
    border-radius: var(--r-tile);
    box-shadow: var(--sh-1);
  }
  .cu-run {
    display: flex;
    align-items: center;
    width: 100%;
  }
  .cu-run__line {
    flex: 1;
    height: 0;
    border-top: 1px dashed var(--ink-4);
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
    text-align: center;
  }

  /* Responsive wrap for narrow widths. */
  @container (max-width: 520px) {
    .cu-item {
      grid-template-columns: calc(32px * var(--ui-s)) minmax(0, 1fr);
      gap: var(--ui-gap-sm);
      align-items: start;
    }
    .cu-item__rank { font-size: var(--text-md); }
    .cu-item__t { white-space: normal; }
    .cu-item__meta { flex-wrap: wrap; }
    .cu-port { max-height: calc(340px * var(--ui-s)); }
  }

  @media (prefers-reduced-motion: reduce) {
    .cu-item { --p: 1 !important; }
  }
</style>
