<!--
  TODAY · THE DAY'S QUEUE · A · THE WORKING STACK.
  Stacked card queue variant where the active item is lifted above the pile
  with session timer controls (Start, Log), while queued items appear as
  compressed slips below it.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    courseOf,
    isDated,
    minutesText,
    minutesWords,
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

  const reading = (row: QueueRow): string => {
    const place = rows.indexOf(row) + 1;
    return (
      `${titleOf(row.item)} — ${place} of ${rows.length} on the stack, ` +
      `${minutesWords(row.item)}${stateWord(row.item) ? `, ${stateWord(row.item)}` : ''}`
    );
  };
</script>

<section class="tqa v-fit" id="today-queue">
  <header class="cd-card__head tqa-head">
    <span class="cd-ictile"><Icon name="checklist" /></span>
    <div>
      <h2 class="cd-card__title">The day's queue</h2>
      <p class="cd-card__sub">{queueSize(groups)} on the stack · {workLine(rows)}</p>
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
    <div class="cd-dashed tqa-empty">
      <b>Nothing on the stack</b>
      <span>Nothing is due, late or next in this plan.</span>
    </div>
  {:else if lift}
    {@const state = stateOf(lift.group)}
    {@const course = courseOf(lift.item)}
    {@const word = stateWord(lift.item)}
    <div class="tqa-body">
    <div class="tqa-lift">
      <span class="tqa-tick" data-s={state} aria-hidden="true"><i></i></span>
      <div class="tqa-lift__main">
        <span class="tqa-lift__t">{titleOf(lift.item)}</span>
        <span class="tqa-lift__meta">
          {#if course}
            <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.label}</span>
          {/if}
          {#if word}
            <span class="tqa-why">{word}</span>
          {/if}
          <span class="num">{minutesText(lift.item)}</span>
        </span>
      </div>
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
          <span class="tqa-run">
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
              aria-label={`Stop the session on ${titleOf(lift.item)}`}
              onclick={onStop}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false"><rect x="7" y="7" width="10" height="10" rx="2" /></svg>
              Stop
            </button>
          </span>
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

    {#if rest.length > 0}
      <div class="tqa-tray">
        {#each rest as row (row.item.id)}
          {@const slipState = stateOf(row.group)}
          {@const slipCourse = courseOf(row.item)}
          {@const slipWord = stateWord(row.item)}
          <button
            class="tqa-slip"
            type="button"
            data-dated={isDated(slipState) ? '1' : undefined}
            aria-label="Bring up {reading(row)}{runningId === row.item.id ? ' — a session is running on it' : ''}"
            onclick={() => (liftedId = row.item.id)}
          >
            <span class="tqa-tick" data-s={slipState} aria-hidden="true"><i></i></span>
            <span class="tqa-slip__t">{titleOf(row.item)}</span>
            <!-- Every slot is rendered, empty or not, so the columns line up
                 down all the slips: a missing cell would shift the minutes and
                 the arrow one column left. -->
            {#if slipCourse}
              <span class="cd-chip cd-chip--code tqa-slip__chip" data-w={slipCourse.wash}>{slipCourse.label}</span>
            {:else}
              <span class="tqa-slip__chip" aria-hidden="true"></span>
            {/if}
            <span class="tqa-why tqa-slip__why">
              {isDated(slipState) && slipWord ? slipWord : ''}
            </span>
            <span class="tqa-slip__run">
              {#if runningId === row.item.id}
                <span class="cd-chip cd-chip--info">running</span>
              {/if}
            </span>
            <span class="tqa-slip__m num">{minutesText(row.item)}</span>
            <span class="tqa-slip__up" aria-hidden="true"><Icon name="arrowup" size={14} /></span>
          </button>
        {/each}
      </div>
    {/if}
    </div>
  {/if}
</section>

<style>
  /* ── THE SECTION HEAD IS ON THE SURFACE (owner's call, 2026-09-30) ────────
     The card was never the object: it was a box drawn around a title and the
     tray that title introduces, so the section read as a card title and then a
     card. The head now stands on the sheet like Today's own greeting, and the
     lift and the tray under it are the object — the same pixels the card
     contained, with the insets the card was paying for removed, so the title
     and the tray share one left edge. */
  .tqa-head {
    padding: 0;
  }
  .tqa-head.cd-card__head {
    margin-bottom: var(--ui-gap);
  }
  .tqa-empty {
    margin: 0;
  }

  /* ── the lifted card ─────────────────────────────────────────────────── */
  .tqa-lift {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: 2px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--ui-gap-sm);
    margin: 0 calc(-8px * var(--ui-s)) calc(-6px * var(--ui-s));
    padding: var(--ui-pad-y) var(--ui-pad);
    background: var(--card);
    border-radius: var(--r-card);
    box-shadow: var(--sh-2);
  }
  .tqa-lift__main {
    display: grid;
    gap: var(--space-3xs);
    min-width: 0;
  }
  .tqa-lift__t {
    font-size: var(--text-md);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.15;
  }
  .tqa-lift__meta {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-xs);
    color: var(--ink-3);
    flex-wrap: wrap;
  }
  .tqa-lift__meta .num {
    color: var(--ink-2);
  }
  .tqa-acts > *,
  .tqa-acts .cd-iconbtn {
    flex: none;
  }
  .tqa-acts {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .tqa-run {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2xs);
  }

  /* The state tick: a 2×16 bar whose height is the state — a bar, never a dot,
     so nine of them read as one hairline ledger down the pile. The row's own
     state word carries the reading, so the mark is `aria-hidden`. */
  .tqa-tick {
    display: grid;
    align-items: end;
    justify-items: start;
    width: 2px;
    height: 16px;
    flex: none;
  }
  .tqa-tick i {
    display: block;
    width: 2px;
    border-radius: var(--r-pill);
    height: var(--h, 16px);
    background: var(--bg, var(--rule-strong));
  }
  .tqa-tick[data-s='late'] i {
    --h: 16px;
    --bg: var(--ink);
  }
  .tqa-tick[data-s='due'] i {
    --h: 9px;
    --bg: var(--ink);
  }
  .tqa-tick[data-s='planned'] i {
    --h: 16px;
    --bg: var(--rule-strong);
  }
  .tqa-why {
    color: var(--ink-2);
    font-weight: var(--weight-label);
  }

  /* ── the tray of slips ───────────────────────────────────────────────── */
  .tqa-tray {
    padding: var(--ui-pad) var(--space-xs) var(--space-xs);
    background: var(--well);
    border-radius: var(--r-tile);
    overflow: clip;
  }
  .tqa-slip {
    display: grid;
    grid-template-columns: 2px minmax(0, 1fr) auto auto auto auto 20px;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: 100%;
    min-height: calc(48px * var(--ui-s));
    padding: var(--space-2xs) var(--ui-pad);
    background: var(--card);
    border-radius: var(--r-item);
    box-shadow: var(--sh-1);
    text-align: left;
    transition: box-shadow var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .tqa-slip + .tqa-slip {
    margin-top: calc(6px * var(--ui-s));
  }
  .tqa-slip:hover {
    box-shadow: var(--sh-2);
    transform: translateY(-1px);
  }
  .tqa-slip:active {
    box-shadow: var(--sh-1);
    transform: none;
  }
  .tqa-slip:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .tqa-slip__t {
    font-size: var(--text-base);
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The four things that are dated take the ink; the rest stay one step down,
     which is where the eye's first pass lands. */
  .tqa-slip[data-dated] .tqa-slip__t {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .tqa-slip__m {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink-2);
    text-align: right;
    white-space: nowrap;
  }
  .tqa-slip__run {
    display: flex;
    align-items: center;
  }
  .tqa-slip__up {
    color: var(--ink-3);
    display: grid;
    place-items: center;
    transition: color var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .tqa-slip:hover .tqa-slip__up {
    color: var(--ink);
    transform: translateY(-2px);
  }

  /* The card is 60% of the canvas on Today; narrower than a reading column the
     slips give up their course chip and their state word — the row's own
     accessible name still says both — and keep the tick, the title, the
     minutes and the arrow that is the slip's own affordance. */
  @container (max-width: 560px) {
    .tqa-slip {
      grid-template-columns: 2px minmax(0, 1fr) auto auto;
    }
    .tqa-slip__why,
    .tqa-slip__chip,
    .tqa-slip__run {
      display: none;
    }
    .tqa-lift {
      grid-template-columns: 2px minmax(0, 1fr);
    }
    .tqa-acts {
      grid-column: 2;
      justify-content: flex-start;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tqa-slip,
    .tqa-slip__up {
      transition: none;
    }
    .tqa-slip:hover {
      transform: none;
    }
    .tqa-slip:hover .tqa-slip__up {
      transform: none;
    }
  }
</style>
