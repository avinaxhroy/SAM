<!-- TODAY · The Day's Queue · A · The Day's Weight.
     Queue items sized proportionally to duration estimates, with active task featured above. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    courseOf,
    isDated,
    plural,
    queueSize,
    stateOf,
    stateWord,
    titleOf,
    workLine,
    type Props,
    type QueueRow,
  } from './props';

  let {
    rows,
    groups,
    sessionCommand,
    targetMin,
    runningId,
    runningSince,
    onStart,
    onStop,
    onLog,
    onAdd,
    onRemove,
    onAddToToday,
  }: Props = $props();

  /** The thing being worked on — the student's own press, never a re-order. */
  let liftedId = $state<string | null>(null);
  const lift = $derived(rows.find((row) => row.item.id === liftedId) ?? rows[0] ?? null);
  const rest = $derived(rows.filter((row) => row !== lift));

  /* Groups for remaining items; empty groups are omitted. */
  const bands = $derived(
    groups
      .map((group) => ({ group, rows: rest.filter((row) => row.group === group.id) }))
      .filter((band) => band.rows.length > 0),
  );

  /* Fold overflow items for later groups ('next', 'stale') after 4 items. */
  const FOLD_AFTER = 4;
  const LATER = new Set(['next', 'stale']);
  let opened = $state<Record<string, boolean>>({});
  const folds = (band: { group: { id: string }; rows: QueueRow[] }): boolean =>
    LATER.has(band.group.id) && band.rows.length > FOLD_AFTER && !opened[band.group.id];
  const shown = (band: { group: { id: string }; rows: QueueRow[] }): QueueRow[] =>
    folds(band) ? band.rows.slice(0, FOLD_AFTER) : band.rows;
  const behind = (band: { rows: QueueRow[] }): QueueRow[] => band.rows.slice(FOLD_AFTER);

  /* Scale row height by estimated duration (40px base + 0.8px/min), clamped
     between 56px and 120px. Defaults to 56px when unestimated. */
  const blockH = (row: QueueRow): number => {
    const est = row.item.est;
    if (est === null) return 56;
    return Math.round(Math.min(120, Math.max(56, 40 + est * 0.8)));
  };

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

  /** Course and due date metadata line. */
  const metaOf = (row: QueueRow): string => {
    const course = courseOf(row.item);
    const word = isDated(stateOf(row.group)) ? stateWord(row.item) : null;
    return [course?.label, word].filter(Boolean).join(' · ');
  };

  const reading = (row: QueueRow): string => {
    const place = rows.indexOf(row) + 1;
    return (
      `${titleOf(row.item)} — ${place} of ${rows.length} in the day’s queue, ` +
      `${row.item.est === null ? 'no estimate' : `${row.item.est} minutes`}` +
      `${stateWord(row.item) ? `, ${stateWord(row.item)}` : ''}`
    );
  };
</script>

<section class="cd-card tqa v-fit" id="today-queue">
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="checklist" /></span>
    <div>
      <h2 class="cd-card__title">The day’s queue</h2>
      {#if rows.length > 0}
        <p class="cd-card__sub">{queueSize(groups)} · {workLine(rows)}</p>
      {/if}
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
      <b>Nothing in the queue</b>
      <span>Nothing is due, late or next in this plan.</span>
    </div>
  {:else if lift}
    {@const liftMeta = metaOf({ item: lift.item, group: lift.group, label: lift.label })}
    <div class="tqa-body">
      <!-- THE THING BEING WORKED. The day's one block that carries controls,
           so it is a `div` and not a button, in the app's own selected fill
           (`collection.css` §3) rather than a fill of this file's. -->
      <div
        class="tqa-hero"
        data-state={stateOf(lift.group)}
        style="--h: {blockH(lift)}px"
        aria-current="true"
      >
        <span class="tqa-hero__t">{titleOf(lift.item)}</span>
        <span class="tqa-hero__facts">
          {#if liftMeta}<span class="tqa-meta">{liftMeta}</span>{/if}
          {#if runningId === lift.item.id}
            <span class="cd-chip cd-chip--info">
              <Icon name="clock" size={11} />
              <span class="num">{elapsed ?? 'running'}</span>
            </span>
          {/if}
        </span>
        <span class="tqa-hero__min num">
          {#if lift.item.est === null}
            <span class="tqa-none">no estimate</span>
          {:else}
            {lift.item.est}<i>min</i>
          {/if}
        </span>
        <span class="tqa-acts">
          <button
            class="cd-iconbtn"
            type="button"
            aria-label={lift.group === 'committed'
              ? `Take ${titleOf(lift.item)} off today`
              : `Plan ${titleOf(lift.item)} for today`}
            onclick={() => (lift.group === 'committed' ? onRemove(lift.item) : onAdd(lift.item))}
          >
            <Icon name={lift.group === 'committed' ? 'close' : 'plus'} size={14} />
          </button>
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            data-command={sessionCommand ?? undefined}
            data-placement="today.screen"
            disabled={!sessionCommand}
            aria-label={`Log ${targetMin} min on ${titleOf(lift.item)} without starting a timer`}
            onclick={() => onLog(lift.item)}
          >
            <Icon name="clock" size={13} />
            Log {targetMin} min
          </button>
          {#if runningId === lift.item.id}
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              aria-label={`Stop the session on ${titleOf(lift.item)}`}
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
              aria-label={`Start a session on ${titleOf(lift.item)}`}
              onclick={() => onStart(lift.item)}
            >
              <Icon name="play" size={13} />
              Start
            </button>
          {/if}
        </span>
      </div>

      <!-- THE SHELVES. One sentence a shelf, no rule and no caps: the name on
           the left in the label face, the shelf's own arithmetic on the right. -->
      {#each bands as band (band.group.id)}
        {@const held = band.rows}
        <p class="tqa-shelf">
          <span class="tqa-shelf__t">{band.group.label}</span>
          <span class="tqa-shelf__n num">{plural(held.length, 'thing', 'things')} · {workLine(held)}</span>
        </p>
        {#each shown(band) as row (row.item.id)}
          {@const rowMeta = metaOf(row)}
          <button
            class="tqa-block"
            type="button"
            data-state={stateOf(row.group)}
            style="--h: {blockH(row)}px"
            aria-label="Bring up {reading(row)}{runningId === row.item.id ? ' — a session is running on it' : ''}"
            onclick={() => (liftedId = row.item.id)}
          >
            <span class="tqa-block__body">
              <span class="tqa-block__t">{titleOf(row.item)}</span>
              {#if rowMeta}<span class="tqa-meta">{rowMeta}</span>{/if}
            </span>
            <span class="tqa-block__min num">
              {#if row.item.est === null}
                <span class="tqa-none">no estimate</span>
              {:else}
                {row.item.est}<i>min</i>
              {/if}
            </span>
          </button>
        {/each}
        {#if folds(band)}
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm tqa-more"
            type="button"
            aria-expanded="false"
            onclick={() => (opened = { ...opened, [band.group.id]: true })}
          >
            Show all {plural(held.length, 'thing', 'things')} · {workLine(behind(band))}
          </button>
        {/if}
      {/each}
    </div>
  {/if}
</section>

<style>
  .tqa {
    --tqa-gap: var(--space-xs);
    --tqa-num-w: 58px;
  }

  .tqa-body {
    display: flex;
    flex-direction: column;
    gap: var(--tqa-gap);
  }

  /* ── Queue row block ──────────────────────────────────────────────────────
     Row height set dynamically via --h based on estimate.
     Background tint indicates state (late, due, planned). */
  .tqa-block {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    min-height: var(--h, 56px);
    padding: var(--space-sm) var(--space-md);
    border: 0;
    border-radius: var(--r-tile);
    background: var(--well);
    color: var(--ink);
    text-align: start;
    cursor: pointer;
    transition:
      box-shadow var(--dur-1) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .tqa-block[data-state='late'] { background: var(--chip-overdue); }
  .tqa-block[data-state='due'] { background: var(--chip-risk); }
  .tqa-block:hover { box-shadow: var(--sh-1); }
  .tqa-block:focus-visible { box-shadow: inset 0 0 0 2px var(--ink); }

  .tqa-block__body {
    display: grid;
    gap: var(--space-2xs);
    min-width: 0;
  }
  .tqa-block__t {
    font-size: var(--text-base);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Course and due date metadata line. */
  .tqa-meta {
    font-size: var(--text-xs);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Duration numeral ─────────────────────────────────────────────────────
     Right-aligned duration numbers aligned across rows using --tqa-num-w. */
  .tqa-block__min,
  .tqa-hero__min {
    flex: none;
    min-width: var(--tqa-num-w);
    text-align: end;
    white-space: nowrap;
    font-size: var(--text-lg);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    line-height: 1;
  }
  .tqa-block__min i,
  .tqa-hero__min i {
    font-style: normal;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    color: var(--ink-3);
    margin-inline-start: 3px;
  }
  .tqa-none {
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
    letter-spacing: var(--track-body);
    color: var(--ink-3);
  }

  /* ── Active hero block ───────────────────────────────────────────────────
     Selected active item with inline action buttons. Min-height 88px to fit
     controls, or scaled higher if duration estimate exceeds 88px. */
  .tqa-hero {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-xs) var(--space-md);
    min-height: max(88px, var(--h, 88px));
    padding: var(--space-md) var(--space-md) var(--space-md) var(--space-lg);
    border-radius: var(--r-tile);
    background: var(--well-2);
  }
  /* Status accent line on left edge for late/due active items. */
  .tqa-hero::before {
    content: '';
    position: absolute;
    inset-block: var(--space-sm);
    inset-inline-start: var(--space-xs);
    width: 3px;
    border-radius: var(--r-pill);
    background: transparent;
  }
  .tqa-hero[data-state='late']::before { background: var(--on-overdue); }
  .tqa-hero[data-state='due']::before { background: var(--on-risk); }
  .tqa-hero__t {
    font-size: var(--text-lg);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
  }
  .tqa-hero__facts {
    grid-column: 1;
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    min-width: 0;
    flex-wrap: wrap;
  }
  .tqa-hero__min {
    grid-column: 2;
    grid-row: 1;
  }
  .tqa-acts {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  /* ── Group shelf header ────────────────────────────────────────────────── */
  .tqa-shelf {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-sm);
    margin: var(--space-lg) 0 0;
    font-size: var(--text-xs);
  }
  .tqa-body .tqa-shelf { margin-block-start: var(--space-lg); }
  .tqa-shelf__t {
    font-weight: var(--weight-label);
    color: var(--ink-2);
  }
  .tqa-shelf__n { color: var(--ink-3); }

  /* 'Show all' toggle for folded overflow items. */
  .tqa-more {
    align-self: flex-start;
    margin-block-start: var(--space-2xs);
  }

  @media (prefers-reduced-motion: reduce) {
    .tqa-block { transition: none; }
    .tqa-block:hover { transform: none; }
  }
</style>
