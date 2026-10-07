<!--
  Mocks & Assessments · Variant A (The Month That Opens).
  Seven-column monthly calendar grid with expandable assessment detail cards below
  each week row. Day cells display date numeral, scheduled markers, and overflow count.
-->
<script lang="ts">
  import {
    byDayOrder,
    capsDay,
    headLineOf,
    dayCount,
    distanceText,
    levelOf,
    monthOf,
    monthsBetween,
    recordsByDay,
    seatOf,
    spokenDay,
    weeksOf,
    type MocksProps,
    type MockRecord,
  } from './props';

  let {
    title,
    loading,
    today,
    anchor,
    dated,
    waiting,
    quiet,
    newCommand,
    newLabel,
    onNew,
    onRead,
    onSetDay,
    onUndo,
  }: MocksProps = $props();

  const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  let root = $state<HTMLElement | null>(null);

  const byDay = $derived(recordsByDay(dated));
  /**
   * Month offset relative to today. Initialized once to the next scheduled
   * assessment so subsequent updates don't shift the current view unexpectedly.
   */
  let off = $state<number | null>(null);
  $effect(() => {
    if (off === null) off = monthsBetween(today.key, anchor);
  });
  const offset = $derived(off ?? monthsBetween(today.key, anchor));
  const month = $derived(monthOf(today.key, offset, byDay));
  const weeks = $derived(weeksOf(month));

  /** The slot: a day's card, or one record's card, or nothing. */
  let open = $state<{ day: string | null; rec: string | null }>({ day: null, rec: null });
  let pick = $state<string | null>(null);
  /** A choice that is no longer offered falls back to the first row. */
  const cand = $derived(pick !== null && quiet.includes(pick) ? pick : (quiet[0] ?? null));
  let placed = $state<{ id: string; from: string | null; to: string } | null>(null);
  let busy = $state(false);
  let focusOn = $state<string | null>(null);

  $effect(() => {
    const selector = focusOn;
    if (selector === null) return;
    focusOn = null;
    document.querySelector<HTMLElement>(selector)?.focus({ preventScroll: true });
  });

  const records = $derived([...dated, ...waiting]);
  const opened = $derived(open.rec === null ? null : (records.find((r) => r.id === open.rec) ?? null));
  const seat = $derived(open.day === null ? null : seatOf(weeks, open.day));

  /** The card's left edge sits on the day's own column: its left edge for
      Monday–Friday, its right edge for the last two columns, so a card under a
      Sunday never starts under a Thursday. */
  const cs = $derived(seat === null ? 1 : seat.col <= 5 ? seat.col : seat.col - 2);

  const headLine = $derived(headLineOf(records, dated, today.key));

  function openDay(record: MockRecord | null, day: string | null): void {
    open = { day, rec: record?.id ?? null };
    placed = null;
    focusOn = '.xa-card';
  }

  function close(): void {
    const back = open.day !== null ? `[data-day="${open.day}"]` : open.rec !== null ? `[data-row="${open.rec}"]` : null;
    open = { day: null, rec: null };
    placed = null;
    focusOn = back ?? '.xa-wait [data-row]';
  }

  async function commit(record: MockRecord): Promise<void> {
    if (cand === null || busy) return;
    busy = true;
    try {
      const accepted = await onSetDay(record.id, cand);
      if (!accepted) return;
      placed = { id: record.id, from: record.day, to: cand };
      focusOn = '[data-undo]';
    } finally {
      busy = false;
    }
  }

  function undo(): void {
    placed = null;
    onUndo();
    focusOn = '[data-commit]';
  }
</script>

<!-- Close the open card on Escape when focus is inside this container. -->
<svelte:window
  onkeydown={(event) => {
    if (event.key !== 'Escape') return;
    if (!root?.contains(event.target as Node)) return;
    if (open.day !== null || open.rec !== null) close();
  }}
/>

<div class="v-fit xa" bind:this={root}>
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{headLine}</p>
    </div>
    <span class="cd-pagehead__aside">
      <!-- New assessment button rendered with quiet style when items await scheduling. -->
      {#if newCommand}
        <button
          class={`cd-pill${waiting.length > 0 ? ' cd-pill--quiet' : ''}`}
          type="button"
          data-command={newCommand}
          data-placement="today.screen"
          onclick={onNew}
        >
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M8 3.4v9.2M3.4 8h9.2"/></svg>
          {newLabel}
        </button>
      {/if}
      <button
        class="cd-iconbtn"
        type="button"
        title="Read the plan's assessments again"
        aria-label="Read the plan's assessments again"
        onclick={onRead}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3M19.5 4.8V9.4H15"/></svg>
      </button>
    </span>
  </header>

  <div class="xa-monthbar">
    <button
      class="cd-iconbtn"
      type="button"
      aria-label="The month before"
      onclick={() => {
        off = offset - 1;
        open = { day: null, rec: null };
        placed = null;
        focusOn = '[data-month="-1"]';
      }}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 5.5 8.5 12 15 18.5"/></svg>
    </button>
    <span class="xa-month">{month.name}</span>
    <button
      class="cd-iconbtn"
      type="button"
      aria-label="The month after"
      onclick={() => {
        off = offset + 1;
        open = { day: null, rec: null };
        placed = null;
        focusOn = '[data-month="1"]';
      }}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5"/></svg>
    </button>
    <span class="x-spacer"></span>
    <span class="xa-now">today · {today.spoken}</span>
  </div>

  <div class="xa-dow" aria-hidden="true">
    {#each WEEKDAYS as day (day)}<span>{day}</span>{/each}
  </div>

  {#snippet recordCard(record: MockRecord, backKey: string | null)}
    <section class="xa-card" tabindex="-1" aria-label={record.label}>
      <div class="xa-card__head">
        {#if backKey}
          <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => openDay(null, backKey)}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 5.5 8.5 12 15 18.5"/></svg>
            {spokenDay(backKey)}
          </button>
        {:else if record.kind}
          <span class="cd-chip cd-chip--info">{record.kind}</span>
        {/if}
        <button class="cd-iconbtn xa-card__x" type="button" aria-label="Close" onclick={close}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6 6 18"/></svg>
        </button>
      </div>

      <div class="xa-ident">
        <div class="cd-rowflex">
          {#each record.courses as course (course.id)}
            {#if course.code}
              <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
            {/if}
          {/each}
          <span class="cd-urgency" data-lvl={levelOf(today.key, record.day)} aria-hidden="true"></span>
        </div>
        <h2 class="xa-h2">{record.label}</h2>
      </div>

      <!-- Assessment metadata: scheduled date, planned week, and score. -->
      <dl class="xa-facts">
        <div>
          <dt>When</dt>
          <dd>{record.day === null ? 'No day yet' : `${spokenDay(record.day)} · ${distanceText(today.key, record.day)}`}</dd>
        </div>
        {#if record.week}
          <div><dt>Planned for</dt><dd>{record.week}</dd></div>
        {/if}
        {#if record.score !== null}
          <div><dt>Score</dt><dd class="num">{record.score}</dd></div>
        {/if}
      </dl>

      <!-- Date assignment selection, commit action, and undo confirmation receipt. -->
      <div class="xa-ctl">
        {#if placed !== null && placed.id === record.id}
          <div class="x-receipt">
            <span class="cd-chip cd-chip--ok">placed</span>
            <span>
              {spokenDay(placed.to)} ·
              {placed.from === null ? 'was no day' : `was ${spokenDay(placed.from)}`}
            </span>
            <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" data-undo onclick={undo}>Undo</button>
          </div>
        {:else if quiet.length > 0}
          <div class="xa-cands" role="group" aria-label={`A day for ${record.label}`}>
            {#each quiet as day (day)}
              <button
                class="xa-cand"
                type="button"
                aria-pressed={day === cand}
                onclick={() => (pick = day)}
              >
                {capsDay(day)} <i>{distanceText(today.key, day)}</i>
              </button>
            {/each}
          </div>
          <button
            class="cd-pill"
            type="button"
            data-command="record.setField"
            data-placement="today.screen"
            data-commit
            disabled={busy || cand === null}
            onclick={() => void commit(record)}
          >
            Put it on {cand === null ? 'a day' : capsDay(cand)}
          </button>
        {/if}
      </div>
    </section>
  {/snippet}

  {#snippet activeCard()}
    {#if opened}
      {@render recordCard(opened, open.day !== null && (byDay.get(open.day)?.length ?? 0) > 1 ? open.day : null)}
    {:else if open.day !== null}
      <section class="xa-card" tabindex="-1" aria-label={`${spokenDay(open.day)}, ${dayCount(byDay.get(open.day)?.length ?? 0)}`}>
        <div class="xa-card__head">
          <div class="xa-card__id">
            <span class="xa-card__d">{spokenDay(open.day)}</span>
            <span class="xa-card__n">{dayCount(byDay.get(open.day)?.length ?? 0)}</span>
          </div>
          <button class="cd-iconbtn xa-card__x" type="button" aria-label="Close" onclick={close}>
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6 6 18"/></svg>
          </button>
        </div>
        <div class="xa-recs">
          {#each [...(byDay.get(open.day) ?? [])].sort(byDayOrder) as record (record.id)}
            <button class="xa-rec" type="button" data-card-record={record.id} onclick={() => openDay(record, open.day)}>
              <span class="cd-rowflex">
                {#each record.courses as course (course.id)}
                  {#if course.code}
                    <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
                  {/if}
                {/each}
              </span>
              <span class="xa-rec__t">{record.label}</span>
              <span class="xa-rec__m">{record.kind ?? ''}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}
  {/snippet}

  {#snippet slot(card: string)}
    <div class="xa-slot" style={`--cs:${card}`}>
      {@render activeCard()}
    </div>
  {/snippet}

  {#if loading}
    <div class="xa-weeks">
      {#each weeks as week, index (index)}
        <div class="xa-week">
          {#each week as _cell, track (track)}
            <span class="cd-skel xa-skelcell"></span>
          {/each}
        </div>
      {/each}
    </div>
    <section class="xa-wait">
      <div class="xa-waithead">
        <h2>No day yet</h2>
        <span class="cd-skel xa-skeln" aria-hidden="true"></span>
      </div>
      <div class="cd-card xa-wrows">
        <div class="xa-recs">
          {#each [0, 1] as row (row)}
            <span class="xa-rec" aria-hidden="true">
              <span class="cd-skel xa-skelc"></span>
              <span class="cd-skel xa-skelb"></span>
            </span>
          {/each}
        </div>
      </div>
    </section>
    <p class="cd-sr" role="status">Reading the plan’s assessments</p>
  {:else}
    <!-- Re-mount calendar grid on month change to trigger step transition. -->
    {#key month.name}
      <div class="xa-weeks">
        {#each weeks as week, index (index)}
          <div class="xa-week">
            {#each week as cell, track (track)}
              {#if cell === null}
                <span class="xa-cell xa-cell--out" aria-hidden="true"></span>
              {:else if cell.records.length === 0}
                <span class={`xa-cell${cell.today ? ' xa-cell--today' : ''}`}>
                  <span class="xa-cell__n num">{cell.num}</span>
                </span>
              {:else}
                <button
                  class={`xa-cell xa-cell--day${cell.today ? ' xa-cell--today' : ''}`}
                  type="button"
                  data-day={cell.key}
                  aria-expanded={open.day === cell.key}
                  aria-label={`${spokenDay(cell.key)}, ${dayCount(cell.records.length)}`}
                  onclick={() => {
                    if (open.day === cell.key && open.rec === null) close();
                    else openDay(cell.records.length === 1 ? cell.records[0] : null, cell.key);
                  }}
                >
                  <span class="xa-cell__n num">{cell.num}</span>
                  <span class="xa-cell__marks" aria-hidden="true">
                    {#each cell.records.slice(0, 3) as _mark, index (index)}
                      <i class="xa-mark"></i>
                    {/each}
                    {#if cell.records.length > 3}
                      <span class="xa-more num">+{cell.records.length - 3}</span>
                    {/if}
                  </span>
                </button>
              {/if}
            {/each}
          </div>
          {#if seat !== null && seat.week === index}
            {@render slot(cs)}
          {/if}
        {/each}
      </div>
    {/key}

    <section class="xa-wait">
      {#if waiting.length > 0}
        <div class="xa-waithead">
          <h2>No day yet</h2>
          <span class="cd-chip">{waiting.length}</span>
        </div>
        <div class="cd-card xa-wrows">
          <div class="xa-recs">
            {#each waiting as record (record.id)}
              <!-- Select undated record to open its date assignment card. -->
              <button
                class="xa-rec"
                type="button"
                data-row={record.id}
                data-command="record.setField"
                data-placement="today.screen"
                onclick={() => openDay(record, null)}
              >
                <span class="cd-rowflex">
                  {#each record.courses as course (course.id)}
                    {#if course.code}
                      <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
                    {/if}
                  {/each}
                </span>
                <span class="xa-rec__t">{record.label}</span>
                <span class="xa-rec__m">no day</span>
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <div class="cd-dashed">
          <b>Every assessment has a day</b>
          <span>{dayCount(dated.length)} on the plan</span>
        </div>
      {/if}
      {#if open.rec !== null && open.day === null}
        {@render slot(1)}
      {/if}
    </section>

    {#if dated.length === 0 && waiting.length > 0}
      <!-- Notice prompting user to date waiting assessments. -->
      <div class="cd-detail xa-none">
        <span class="xa-none__t">Nothing is dated yet</span>
        <p class="xa-none__s">
          {waiting.length} waiting for a day — and a day is the only thing a calendar needs
        </p>
        <button
          class="cd-pill cd-pill--sm"
          type="button"
          data-command="record.setField"
          data-placement="today.screen"
          onclick={() => openDay(waiting[0], null)}
        >
          Give the first one a day
        </button>
      </div>
    {/if}
  {/if}
</div>
