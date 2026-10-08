<!-- TODAY · The Day's Queue · C · The State Pills. Segmented status tabs (Late, Due, Planned). -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    courseOf,
    minutesText,
    minutesWords,
    plural,
    queueSize,
    sliceSize,
    stateOf,
    titleOf,
    workLine,
    type Props,
    type QueueRow,
    type QueueState,
  } from './props';

  let {
    rows,
    groups,
    sessionCommand,
    runningId,
    runningSince,
    onStart,
    onStop,
    onAdd,
    onRemove,
    onAddToToday,
  }: Props = $props();

  const TABS: Array<{ state: QueueState; label: string }> = [
    { state: 'late', label: 'Late' },
    { state: 'due', label: 'Due' },
    { state: 'planned', label: 'Planned' },
  ];

  const sliceFor = (state: QueueState): QueueRow[] =>
    rows.filter((row) => stateOf(row.group) === state);

  let tab = $state(0);
  const current = $derived(TABS[tab]);
  const slice = $derived(sliceFor(current.state));

  /** The running session's clock. Ticks only while a session runs. */
  let now = $state(Date.now());
  $effect(() => {
    if (runningSince === null) return;
    const ticker = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(ticker);
  });
  const elapsed = $derived(
    runningSince === null
      ? null
      : (() => {
          const seconds = Math.max(0, Math.floor((now - runningSince) / 1000));
          return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
        })(),
  );

  /* The thumb: one element, moved from the active tab's own box. */
  let rails: HTMLDivElement | undefined = $state();
  let tabset: HTMLDivElement | undefined = $state();
  let tabEls: HTMLButtonElement[] = $state([]);
  let thumb = $state({ x: 0, w: 0, ready: false });

  function measure(): void {
    const rail = rails;
    const active = tabEls[tab];
    if (!rail || !active) return;
    const railBox = rail.getBoundingClientRect();
    const tabBox = active.getBoundingClientRect();
    thumb = { x: tabBox.left - railBox.left, w: tabBox.width, ready: true };
  }

  $effect(() => {
    const host = tabset;
    if (!host) return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(host);
    return () => observer.disconnect();
  });

  // Re-measure after the tabs and the selection have been painted.
  $effect(() => {
    void tab;
    void rows.length;
    measure();
  });

  /** Total queue count and estimated duration string for the header. */
  const dayLine = $derived(`${queueSize(groups)} · ${workLine(rows)}`);

  const answer = $derived(
    slice.length === 0
      ? 'nothing left'
      : `${sliceSize(slice, groups) ?? plural(slice.length, 'thing', 'things')} · ${workLine(slice)}`,
  );

  function select(index: number, focus: boolean): void {
    tab = index;
    if (focus) tabEls[index]?.focus();
  }

  function walk(event: KeyboardEvent): void {
    const last = TABS.length - 1;
    let next: number | null = null;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = tab === last ? 0 : tab + 1;
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = tab === 0 ? last : tab - 1;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = last;
    if (next === null) return;
    event.preventDefault();
    select(next, true);
  }
</script>

<section class="cd-card tqc v-fit" id="today-queue">
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="checklist" /></span>
    <div>
      <h2 class="cd-card__title">The day’s queue</h2>
      <p class="cd-card__sub">{dayLine}</p>
    </div>
    <span class="cd-card__spacer"></span>
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm"
      type="button"
      data-command="record.setField"
      data-placement="today.screen"
      onclick={onAddToToday}
    >
      <Icon name="plus" size={13} />
      Add
    </button>
  </header>

  {#if rows.length === 0}
    <div class="cd-dashed">
      <b>Nothing on the stack</b>
      <span>Nothing is due, late or next in this plan.</span>
    </div>
  {:else}
    <div
      class="tqc-tabs"
      role="tablist"
      aria-label="The day's queue by state"
      tabindex="-1"
      bind:this={rails}
      onkeydown={walk}
    >
      <span
        class="tqc-thumb"
        style={`transform: translateX(${thumb.x}px); width: ${thumb.w}px;`}
        data-ready={thumb.ready ? '1' : undefined}
        aria-hidden="true"
      ></span>
      <div class="tqc-tabset" bind:this={tabset}>
        {#each TABS as pill, i (pill.state)}
          {@const count = sliceFor(pill.state).length}
          <button
            class="tqc-tab"
            type="button"
            role="tab"
            id={`tqc-tab-${pill.state}`}
            aria-selected={tab === i}
            aria-controls="tqc-panel"
            tabindex={tab === i ? 0 : -1}
            bind:this={tabEls[i]}
            onclick={() => select(i, false)}
          >
            <span>{pill.label}</span>
            <span class="tqc-tab__n num">{count}</span>
          </button>
        {/each}
      </div>
    </div>

    <p class="tqc-answer" aria-live="polite">
      <!-- Hide filter summary when all items match, avoiding duplication with header. -->
      <span class="num">{answer === dayLine ? '' : answer}</span>
    </p>

    <div
      class="tqc-list"
      id="tqc-panel"
      role="tabpanel"
      aria-labelledby={`tqc-tab-${current.state}`}
      tabindex="0"
    >
      {#if slice.length === 0}
        <div class="cd-empty">
          <p class="cd-empty__t">Nothing here</p>
          <p class="cd-empty__s">Nothing of the day is {current.label.toLowerCase()}.</p>
        </div>
      {:else}
        {#each slice as row (row.item.id)}
          {@const course = courseOf(row.item)}
          <div class="tqc-row" data-run={runningId === row.item.id ? '1' : undefined}>
            {#if course}
              <span class="cd-chip cd-chip--code tqc-row__c" data-w={course.wash}>{course.label}</span>
            {:else}
              <span class="tqc-row__c" aria-hidden="true"></span>
            {/if}
            <span class="tqc-row__t">{titleOf(row.item)}</span>
            <span class="tqc-row__m num">{minutesText(row.item)}</span>
            <span class="tqc-row__acts">
              <button
                class="cd-iconbtn"
                type="button"
                aria-label={row.group === 'committed'
                  ? `Take ${titleOf(row.item)} off today`
                  : `Plan ${titleOf(row.item)} for today`}
                onclick={() => (row.group === 'committed' ? onRemove(row.item) : onAdd(row.item))}
              >
                <Icon name={row.group === 'committed' ? 'close' : 'plus'} size={14} />
              </button>
              {#if runningId === row.item.id}
                {#if elapsed}
                  <span class="cd-chip cd-chip--info">
                    <Icon name="clock" size={11} />
                    <span class="num">{elapsed}</span>
                  </span>
                {:else}
                  <span class="cd-chip cd-chip--info">running</span>
                {/if}
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  aria-label={`Stop the session on ${titleOf(row.item)}`}
                  onclick={onStop}
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false"><rect x="7" y="7" width="10" height="10" rx="2" /></svg>
                  Stop
                </button>
              {:else}
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  data-command={sessionCommand ?? undefined}
                  data-placement="today.screen"
                  disabled={!sessionCommand}
                  aria-label={`Start a session on ${titleOf(row.item)} — ${minutesWords(row.item)}`}
                  onclick={() => onStart(row.item)}
                >
                  <Icon name="play" size={13} />
                  Start
                </button>
              {/if}
            </span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .tqc {
    overflow: clip;
  }

  /* ── the three pills and the one thumb ───────────────────────────────── */
  .tqc-tabs {
    position: relative;
    display: inline-flex;
    align-items: center;
    margin: 0;
    padding: var(--space-2xs);
    background: var(--well);
    border-radius: var(--r-pill);
  }
  .tqc-thumb {
    position: absolute;
    left: 0;
    top: var(--space-2xs);
    height: var(--pill-h);
    border-radius: var(--r-pill);
    background: var(--card);
    box-shadow: var(--sh-1);
  }
  /* Before the first measurement the pill has nothing to glide from. */
  .tqc-thumb:not([data-ready]) {
    transition: none;
    opacity: 0;
  }
  .tqc-thumb[data-ready] {
    transition: transform var(--dur-3) var(--ease), width var(--dur-3) var(--ease);
  }
  .tqc-tabset {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
  }
  .tqc-tab {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    height: var(--pill-h);
    padding: 0 var(--ui-pad);
    border-radius: var(--r-pill);
    font-size: var(--text-base);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    color: var(--ink-3);
    transition: color var(--dur-2) var(--ease);
  }
  .tqc-tab:hover {
    color: var(--ink-2);
  }
  .tqc-tab[aria-selected='true'] {
    color: var(--ink);
  }
  .tqc-tab:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .tqc-tab__n {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: var(--chip-h);
    height: var(--chip-h);
    padding: 0 6px;
    border-radius: var(--r-pill);
    background: var(--well-2);
    color: var(--ink-2);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    font-variant-numeric: tabular-nums;
  }
  .tqc-tab[aria-selected='true'] .tqc-tab__n {
    color: var(--ink);
  }

  /* The answer line is the group's own size, held at the control floor so it
     never moves as the counts change. */
  .tqc-answer {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-height: var(--hit);
    margin: var(--ui-gap-sm) 0 0;
    font-size: var(--text-xs);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
  }

  /* ── the rows of the selected group ──────────────────────────────────── */
  .tqc-list {
    margin-top: var(--space-2xs);
  }
  .tqc-list:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .tqc-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) calc(76px * var(--ui-s)) auto;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: 100%;
    min-height: calc(52px * var(--ui-s));
    padding: var(--space-2xs) var(--ui-pad);
    transition: background var(--dur-1) var(--ease);
  }
  .tqc-row + .tqc-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .tqc-row:hover,
  .tqc-row:focus-within {
    background: var(--well);
  }
  .tqc-row__t {
    font-size: var(--text-base);
    letter-spacing: var(--track-title);
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tqc-row__m {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink-2);
    text-align: right;
    white-space: nowrap;
  }
  .tqc-row__acts > * {
    flex: none;
  }
  .tqc-row__acts {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-xs);
  }
  /* While a session runs its minutes cell leaves the grid (by `display`, not
     `visibility`) and the clock's cluster takes its width, so every row's right
     edge stays flush. */
  .tqc-row[data-run] {
    grid-template-columns: auto minmax(0, 1fr) auto;
  }
  .tqc-row[data-run] .tqc-row__m {
    display: none;
  }

  /* Container width < 472px (520px column minus 48px padding). */
  @container (max-width: 472px) {
    .tqc-row {
      grid-template-columns: auto minmax(0, 1fr) calc(76px * var(--ui-s));
      grid-template-areas:
        'c t m'
        'a a a';
      row-gap: var(--space-2xs);
    }
    .tqc-row__c {
      grid-area: c;
    }
    .tqc-row__t {
      grid-area: t;
    }
    .tqc-row__m {
      grid-area: m;
    }
    .tqc-row__acts {
      grid-area: a;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tqc-thumb[data-ready] {
      transition: none;
    }
    .tqc-tab,
    .tqc-row {
      transition: none;
    }
  }
</style>
