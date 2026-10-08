<!-- TODAY · The Day's Queue · B · The Filter Strip. Filters queue by course or status. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    courseOf,
    coursesOf,
    isDated,
    minutesText,
    minutesWords,
    plural,
    queueSize,
    sliceSize,
    stateOf,
    stateTone,
    stateWord,
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

  const STATE_PILLS: Array<{ key: QueueState; label: string }> = [
    { key: 'late', label: 'Late' },
    { key: 'due', label: 'Due' },
    { key: 'planned', label: 'Planned' },
  ];

  const courses = $derived(coursesOf(rows));

  /** One selection: `all`, one state, or one course. Replaced, never added to. */
  let filter = $state('all');
  /** Bumped on every selection so the rows' arrival can replay. */
  let swap = $state(0);

  const slice = $derived(
    filter === 'all'
      ? rows
      : filter.startsWith('course:')
        ? rows.filter((row) => courseOf(row.item)?.label === filter.slice('course:'.length))
        : rows.filter((row) => `state:${stateOf(row.group)}` === filter),
  );

  const sliceFor = (key: string): QueueRow[] =>
    key === 'all'
      ? rows
      : key.startsWith('course:')
        ? rows.filter((row) => courseOf(row.item)?.label === key.slice('course:'.length))
        : rows.filter((row) => `state:${stateOf(row.group)}` === key);

  const filterLabel = (key: string): string =>
    key === 'all' ? 'All' : key.startsWith('course:') ? key.slice('course:'.length)
      : (STATE_PILLS.find((pill) => `state:${pill.key}` === key)?.label ?? '');

  /** The answer's own arithmetic, written from the slice it describes. */
  function answerLine(): string {
    if (slice.length === 0) return `nothing here for ${filterLabel(filter).toLowerCase()}`;
    const size = sliceSize(slice, groups);
    return `${size ?? plural(slice.length, 'thing', 'things')} · ${workLine(slice)}`;
  }

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

  function pick(key: string): void {
    if (key === filter) return;
    filter = key;
    swap += 1;
  }
</script>

<section class="cd-card tqb v-fit" id="today-queue">
  <header class="cd-card__head">
    <span class="cd-ictile"><Icon name="checklist" /></span>
    <div>
      <h2 class="cd-card__title">The day’s queue</h2>
      <!-- Subtitle omitted because the filter strip below displays counts. -->
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
    <div class="tqb-strip" role="group" aria-label="Filter the day's queue">
      <button
        class="tqb-fpill"
        type="button"
        aria-pressed={filter === 'all'}
        aria-label="Show the whole day — {queueSize(groups)}, {workLine(rows)}"
        onclick={() => pick('all')}
      >
        All
      </button>
      {#each courses as course (course.label)}
        {@const count = sliceFor(`course:${course.label}`).length}
        <button
          class="tqb-fpill tqb-fpill--code"
          type="button"
          aria-pressed={filter === `course:${course.label}`}
          aria-disabled={count === 0 ? 'true' : undefined}
          tabindex={count === 0 ? -1 : undefined}
          aria-label="Show {course.label} — {plural(count, 'thing', 'things')}, {workLine(sliceFor(`course:${course.label}`))}"
          onclick={() => pick(`course:${course.label}`)}
        >
          {course.label}
        </button>
      {/each}
      <span class="tqb-sep" aria-hidden="true"></span>
      {#each STATE_PILLS as pill (pill.key)}
        {@const count = sliceFor(`state:${pill.key}`).length}
        <button
          class="tqb-fpill"
          type="button"
          aria-pressed={filter === `state:${pill.key}`}
          aria-disabled={count === 0 ? 'true' : undefined}
          tabindex={count === 0 ? -1 : undefined}
          aria-label="Show {pill.label} — {plural(count, 'thing', 'things')}, {workLine(sliceFor(`state:${pill.key}`))}"
          onclick={() => pick(`state:${pill.key}`)}
        >
          {pill.label}
        </button>
      {/each}
    </div>

    <p class="tqb-answer num">{answerLine()}</p>

    {#if slice.length === 0}
      <div class="cd-empty">
        <p class="cd-empty__t">Nothing here</p>
        <p class="cd-empty__s">Nothing of the day answers this pill. Pick another.</p>
      </div>
    {:else}
      {#key swap}
        <div class="tqb-list">
          {#each slice as row, i (row.item.id)}
            {@const state = stateOf(row.group)}
            {@const course = courseOf(row.item)}
            {@const word = stateWord(row.item)}
            {@const tone = stateTone(row.item)}
            <div
              class="tqb-row"
              data-dated={isDated(state) ? '1' : undefined}
              data-run={runningId === row.item.id ? '1' : undefined}
              style={`--i:${Math.min(i, 6)}`}
            >
              {#if course}
                <span class="cd-chip cd-chip--code tqb-row__c" data-w={course.wash}>{course.label}</span>
              {:else}
                <span class="tqb-row__c" aria-hidden="true"></span>
              {/if}
              <span class="tqb-row__t">{titleOf(row.item)}</span>
              <span class="tqb-row__s">
                {#if isDated(state) && word && tone}
                  <span class="cd-chip cd-chip--{tone}">{word}</span>
                {/if}
              </span>
              <span class="tqb-row__m num">{minutesText(row.item)}</span>
              <span class="tqb-row__acts">
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
        </div>
      {/key}
    {/if}
  {/if}
</section>

<style>
  .tqb {
    overflow: clip;
  }
  .tqb-strip,
  .tqb-answer {
    margin: 0;
  }

  /* ── the strip: one capsule, one selection ───────────────────────────── */
  .tqb-strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2xs);
    padding: var(--space-2xs);
    background: var(--well);
    border-radius: var(--r-pill);
  }
  .tqb-fpill {
    flex: none;
    min-width: var(--hit);
    justify-content: center;
    height: var(--hit);
    padding: 0 var(--ui-gap-sm);
    border-radius: var(--r-pill);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2xs);
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    transition: background var(--dur-2) var(--ease), color var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .tqb-fpill--code {
    font-variant-numeric: tabular-nums;
  }
  .tqb-fpill:hover {
    color: var(--ink);
    background: color-mix(in oklab, var(--card) 62%, transparent);
  }
  .tqb-fpill:active {
    background: var(--well-2);
  }
  .tqb-fpill:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  /* The lab's pressed pill takes the ink (the system's segmented recipe). On
     Today the screen's one dark object is the focus card's Start, so the
     selection is dressed as a raised `--card` pill instead — the same language
     C's thumb speaks — and this card spends no ink. */
  .tqb-fpill[aria-pressed='true'] {
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--sh-1);
  }
  .tqb-fpill[aria-pressed='true']:hover,
  .tqb-fpill[aria-pressed='true']:active {
    background: var(--card);
    color: var(--ink);
  }
  .tqb-fpill[aria-disabled='true'] {
    opacity: 0.45;
    pointer-events: none;
  }
  .tqb-sep {
    flex: none;
    width: 1px;
    height: 16px;
    margin: 0 var(--space-2xs);
    background: var(--rule-strong);
  }
  .tqb-answer {
    margin-top: var(--ui-gap-sm);
    font-size: var(--text-xs);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
    min-height: var(--hit);
    display: flex;
    align-items: center;
  }

  /* ── the list: one grid per row, so the columns stand still ──────────── */
  .tqb-list {
    margin-top: var(--space-2xs);
  }
  .tqb-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto calc(76px * var(--ui-s)) auto;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: 100%;
    min-height: calc(56px * var(--ui-s));
    padding: var(--space-2xs) var(--ui-pad);
    transition: background var(--dur-1) var(--ease);
  }
  .tqb-row + .tqb-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .tqb-row:hover,
  .tqb-row:focus-within {
    background: var(--well);
  }
  .tqb-row__t {
    font-size: var(--text-base);
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tqb-row[data-dated] .tqb-row__t {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .tqb-row__s {
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }
  .tqb-row__m {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink-2);
    text-align: right;
    white-space: nowrap;
  }
  .tqb-row__acts > * {
    flex: none;
  }
  .tqb-row__acts {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-xs);
  }
  /* While a session runs its minutes cell leaves the grid (by `display`, not
     `visibility` — an auto-placed neighbour would otherwise be pushed onto a
     second row) and the clock's cluster takes exactly its width, so every
     row's right edge stays flush. */
  .tqb-row[data-run] {
    grid-template-columns: auto minmax(0, 1fr) auto auto;
  }
  .tqb-row[data-run] .tqb-row__m {
    display: none;
  }

  @keyframes tqb-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  .tqb-list .tqb-row {
    animation: tqb-in var(--dur-2) var(--ease) both;
    animation-delay: calc(var(--i, 0) * 25ms);
  }

  /* At narrow widths (< 472px container, i.e. 520px column minus 48px padding),
     stack action buttons onto a second line. */
  @container (max-width: 472px) {
    .tqb-row {
      grid-template-columns: auto minmax(0, 1fr) auto calc(76px * var(--ui-s));
      grid-template-areas:
        'c t s m'
        'a a a a';
      row-gap: var(--space-2xs);
    }
    .tqb-row__c {
      grid-area: c;
    }
    .tqb-row__t {
      grid-area: t;
    }
    .tqb-row__s {
      grid-area: s;
    }
    .tqb-row__m {
      grid-area: m;
    }
    .tqb-row__acts {
      grid-area: a;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tqb-row {
      animation: none;
    }
    .tqb-fpill,
    .tqb-row {
      transition: none;
    }
  }
</style>
