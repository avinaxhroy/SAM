<!--
  COMING UP · C · THE SPLIT.
  Split timeline variant. Separates date and content into dual cards that
  part slightly as items scroll toward the vertical centre of the viewport.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { leadOf, type ComingUpProps } from './props';

  let { rows, planReady, onOpenPlan, onOpen }: ComingUpProps = $props();

  const N = $derived(rows.length);
  const REDUCE = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  let port = $state<HTMLElement | null>(null);
  let frame = 0;

  // Calculate separation offset based on distance from the vertical centre.
  function measure(): void {
    frame = 0;
    const box = port;
    if (!box || !box.clientHeight) return;
    const top = box.getBoundingClientRect().top;
    const height = box.clientHeight;
    for (const row of box.querySelectorAll<HTMLElement>('.cu-split')) {
      const rect = row.getBoundingClientRect();
      const away = rect.top + rect.height / 2 - top - height / 2;
      let sep = 1 - Math.abs(away) / (height * 0.3);
      sep = Math.min(1, Math.max(0, sep));
      const parted = sep * sep * (3 - 2 * sep);
      row.style.setProperty('--sep', parted.toFixed(3));
    }
  }

  function request(): void {
    if (!frame) frame = requestAnimationFrame(measure);
  }

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

<section class="v-fit cu-c">
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
    <!-- The row at zero: the date panel drawn empty, and the window's own slot
         where the body's line would go. -->
    <div class="cu-split cu-split--void">
      <span class="cu-split__date cu-split__date--void" aria-hidden="true"></span>
      <span class="cu-split__body cu-split__body--void">
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
            class="cu-split"
            type="button"
            data-command="record.panel"
            data-placement="recordTable.rowContext"
            aria-label={`${row.label} — open the record`}
            onclick={() => onOpen(row.id)}
          >
            <span
              class="cu-split__date"
              class:cu-split__date--void={!row.date}
              aria-hidden="true"
            >
              {#if row.date}
                <span class="cu-split__wd">{row.date.weekday}</span>
                <span class="cu-split__n num">{row.date.day}</span>
                <span class="cu-split__m">{row.date.month}</span>
              {/if}
            </span>
            <span class="cu-split__body">
              <span class="cu-split__kick">
                {#if row.course}
                  <span class="cd-chip cd-chip--code" data-w={row.course.wash}>{row.course.label}</span>
                {:else}
                  <span></span>
                {/if}
                {#if row.state}
                  <span class="cu-state" data-tone={row.state.tone}>{row.state.word}</span>
                {/if}
              </span>
              <span class="cu-split__t">{row.label}</span>
              {#if leadOf(row)}
                <span class="cu-when num">{leadOf(row)}</span>
              {/if}
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
    max-height: calc(420px * var(--ui-s));
    max-width: min(880px, 100%);
    margin: 0 auto;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: var(--ui-pad) 0;
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

  /* Dual panel row with scroll-based separation gap. */
  .cu-split {
    display: flex;
    align-items: stretch;
    gap: calc(16px + var(--sep, 0) * 28px);
    width: 100%;
    padding: 0 calc(14px * var(--ui-s));
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .cu-split__date,
  .cu-split__body {
    background: var(--card);
    border-radius: var(--r-tile);
    box-shadow: var(--sh-1);
    transition: box-shadow var(--dur-1) var(--ease);
  }
  .cu-split__date {
    flex: none;
    display: grid;
    align-content: center;
    justify-items: start;
    gap: 2px;
    width: calc(112px * var(--ui-s));
    padding: var(--ui-pad-sm) var(--ui-pad);
    background: var(--well-2);
    transform: translateX(calc(var(--sep, 0) * -14px));
  }
  .cu-split__wd,
  .cu-split__m {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    color: var(--ink-2);
  }
  .cu-split__n {
    font-size: var(--text-2xl);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    line-height: 1;
  }
  .cu-split__body {
    flex: 1;
    display: grid;
    align-content: center;
    gap: var(--ui-gap-sm);
    min-width: 0;
    padding: var(--ui-pad-sm) var(--ui-pad);
    transform: translateX(calc(var(--sep, 0) * 14px));
  }
  .cu-split__kick {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ui-gap);
  }
  .cu-split__t {
    font-size: var(--text-md);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .cu-split:hover .cu-split__date,
  .cu-split:hover .cu-split__body { box-shadow: var(--sh-2); }
  /* Highlight both panels on focus. */
  .cu-split:active { transform: scale(0.996); }
  .cu-split:focus-visible { outline: none; }
  .cu-split:focus-visible .cu-split__date,
  .cu-split:focus-visible .cu-split__body {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .cu-when {
    font-variant-numeric: tabular-nums;
    font-size: var(--text-xs);
    color: var(--ink-3);
  }
  .cu-state {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink-2);
  }
  .cu-state[data-tone='overdue'] { color: var(--on-overdue); }
  .cu-state[data-tone='risk'] { color: var(--on-risk); }

  /* Empty date panel styling. */
  .cu-split__date--void {
    background: none;
    background-image: none;
    border: 1.5px dashed var(--ink-4);
    box-shadow: none;
  }
  .cu-split--void {
    gap: 16px;
    max-width: min(880px, 100%);
    margin: 0 auto;
  }
  .cu-split--void .cu-split__date--void {
    align-self: flex-start;
    min-height: calc(88px * var(--ui-s));
  }
  .cu-split__body--void {
    display: grid;
    justify-items: start;
    gap: var(--ui-gap-sm);
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

  /* Responsive layout on narrow containers. */
  @container (max-width: 520px) {
    .cu-split__date { width: calc(84px * var(--ui-s)); }
    .cu-split__n { font-size: var(--text-xl); }
    .cu-split__kick { flex-wrap: wrap; }
    .cu-split__t { font-size: var(--text-base); }
    .cu-port { max-height: calc(360px * var(--ui-s)); }
  }

  @media (prefers-reduced-motion: reduce) {
    .cu-split { --sep: 0 !important; }
  }
</style>
