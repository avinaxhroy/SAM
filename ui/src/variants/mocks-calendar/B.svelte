<!--
  Mocks & Assessments · Variant B (The Day Picker Rail).
  Horizontal day-picker rail showing daily assessment density across the month.
  Selecting a day filters the list below; in assign mode, rail acts as a date picker.
-->
<script lang="ts">
  import {
    byDayOrder,
    busiestDay,
    capsDay,
    countdownOf,
    dayCount,
    distanceText,
    headLineOf,
    levelOf,
    monthOf,
    monthsBetween,
    nextBusy,
    quietInMonth,
    recordsByDay,
    spokenDay,
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
    newCommand,
    newLabel,
    onNew,
    onRead,
    onSetDay,
    onUndo,
  }: MocksProps = $props();

  let root = $state<HTMLElement | null>(null);
  const byDay = $derived(recordsByDay(dated));
  /**
   * Month offset relative to today. Initialized to the next scheduled assessment
   * month and held stable across updates until user navigation or assignment commit.
   */
  let off = $state<number | null>(null);
  $effect(() => {
    if (off === null) off = monthsBetween(today.key, anchor);
  });
  const offset = $derived(off ?? monthsBetween(today.key, anchor));
  const month = $derived(monthOf(today.key, offset, byDay));
  let chosen = $state<string | null>(null);
  let assign = $state<string | null>(null);
  let placed = $state<{ id: string; from: string | null; to: string } | null>(null);
  let busy = $state(false);
  let focusOn = $state<string | null>(null);

  $effect(() => {
    const selector = focusOn;
    if (selector === null) return;
    focusOn = null;
    document.querySelector<HTMLElement>(selector)?.focus({ preventScroll: true });
  });

  const picking = $derived(assign === null ? null : (waiting.find((r) => r.id === assign) ?? null));
  const records = $derived([...dated, ...waiting]);
  /** Selected day, falling back to the month's busiest day if unset or out of range. */
  const pick = $derived(
    chosen !== null && month.days.some((day) => day.key === chosen) ? chosen : busiestDay(month).key,
  );
  const picked = $derived(month.days.find((day) => day.key === pick) ?? null);
  const busyDays = $derived(month.days.filter((day) => day.records.length > 0).length);

  function stepMonth(by: number): void {
    off = offset + by;
    const next = monthOf(today.key, offset + by, byDay);
    /* In assign mode the month steps to the day it would propose there; in
       browse it rests on that month's busiest day. */
    chosen = assign !== null ? quietInMonth(today.key, next) : busiestDay(next).key;
    placed = null;
    focusOn = `[data-month="${by}"]`;
  }

  function startAssign(record: MockRecord): void {
    assign = record.id;
    /* Default to an available weekday in the month. */
    chosen = quietInMonth(today.key, month);
    placed = null;
    focusOn = '[data-key][aria-checked="true"]';
  }

  async function commit(): Promise<void> {
    /* Capture record and target day before the mutation re-reads the plan. */
    const record = picking;
    const day = pick;
    if (record === null || busy) return;
    busy = true;
    try {
      const accepted = await onSetDay(record.id, day);
      if (!accepted) return;
      placed = { id: record.id, from: record.day, to: day };
      assign = null;
      chosen = day;
      /* Pan the rail to the month containing the assigned date. */
      off = monthsBetween(today.key, day);
      focusOn = '[data-undo]';
    } finally {
      busy = false;
    }
  }

  function undo(): void {
    /* Reverts assignment and restores assign mode on the target date. */
    const back = placed;
    placed = null;
    onUndo();
    if (back !== null) {
      assign = back.id;
      chosen = back.to;
    }
  }

  /** Escape leaves assign mode when focus is inside this component. */
  function escape(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || assign === null) return;
    if (!root?.contains(event.target as Node)) return;
    assign = null;
    placed = null;
  }

  /** ← / → walk the rail, Home / End go to the month's ends. */
  function rails(event: KeyboardEvent): void {
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      walk(-1);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      walk(1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      walk('home');
    } else if (event.key === 'End') {
      event.preventDefault();
      walk('end');
    }
  }

  /** ← / → walk the rail, Home / End go to the month's ends. */
  function walk(to: number | 'home' | 'end'): void {
    if (to === 'home') chosen = month.days[0].key;
    else if (to === 'end') chosen = month.days[month.days.length - 1].key;
    else {
      const at = month.days.findIndex((day) => day.key === pick);
      chosen = month.days[Math.max(0, Math.min(month.days.length - 1, at + to))].key;
    }
    focusOn = `[data-key="${chosen}"]`;
  }
</script>

<svelte:window onkeydown={escape} />

<div class="v-fit xb" bind:this={root}>
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{headLineOf(records, dated, today.key)}</p>
    </div>
    <span class="cd-pagehead__aside">
      {#if newCommand}
        <button
          class="cd-pill cd-pill--quiet"
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

  <section
    class="xb-rail"
    data-mode={picking ? 'assign' : 'browse'}
    aria-label={picking ? `A day for ${picking.label}` : `The days of ${month.name}`}
  >
    <div class="xb-rail__head">
      <button class="cd-iconbtn" type="button" data-month="-1" aria-label="The month before" onclick={() => stepMonth(-1)}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 5.5 8.5 12 15 18.5"/></svg>
      </button>
      <div class="xb-rail__id">
        {#if picking}
          <span class="xb-rail__t">Set a day for {picking.label}</span>
          <span class="xb-rail__s">{month.name}</span>
        {:else}
          <span class="xb-rail__t">{month.name}</span>
          <span class="xb-rail__s">
            {busyDays === 1 ? '1 day has assessments' : `${busyDays} days have assessments`}
          </span>
        {/if}
      </div>
      <span class="x-spacer"></span>
      {#if picking}
        <button
          class="cd-pill cd-pill--quiet cd-pill--sm"
          type="button"
          onclick={() => {
            assign = null;
            placed = null;
          }}
        >
          Cancel
        </button>
      {/if}
      <button class="cd-iconbtn" type="button" data-month="1" aria-label="The month after" onclick={() => stepMonth(1)}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5"/></svg>
      </button>
    </div>

    {#if loading}
      <div class="xb-keys" style={`--xb-keys:${month.days.length}`}>
        {#each month.days as day (day.key)}
          <span class="xb-skelkey" aria-hidden="true">
            <i class="cd-skel xb-skeld"></i>
            <i class="cd-skel xb-skeln"></i>
          </span>
        {/each}
      </div>
      <div class="xb-rail__foot"><span class="cd-skel xb-skelfoot" aria-hidden="true"></span></div>
    {:else}
      <div
        class="xb-keys"
        role="radiogroup"
        tabindex="-1"
        aria-label={`The days of ${month.name}`}
        style={`--xb-keys:${month.days.length}`}
        onkeydown={rails}
      >
        {#each month.days as day (day.key)}
          <button
            class="xb-key"
            type="button"
            role="radio"
            data-key={day.key}
            aria-checked={day.key === pick}
            aria-current={day.today ? 'date' : undefined}
            data-nothing={day.records.length === 0 ? '1' : undefined}
            aria-label={`${spokenDay(day.key)}, ${day.records.length === 0 ? 'nothing on it' : dayCount(day.records.length)}`}
            onclick={() => {
              chosen = day.key;
              focusOn = `[data-key="${day.key}"]`;
            }}
          >
            <span class="xb-key__d num">{day.num}</span>
            <span class="xb-key__n num">{day.records.length}</span>
          </button>
        {/each}
      </div>

      {#if picking}
        <div class="xb-rail__foot">
          <button
            class="cd-pill"
            type="button"
            data-command="record.setField"
            data-placement="today.screen"
            data-commit
            disabled={busy}
            onclick={() => void commit()}
          >
            Put it on {capsDay(pick)}
          </button>
          <span class="xb-rail__note num">
            {spokenDay(pick)}{pick === today.key ? '' : ` · ${distanceText(today.key, pick)}`}
          </span>
        </div>
      {/if}
    {/if}
  </section>

  {#if placed !== null}
    <div class="x-receipt">
      <span class="cd-chip cd-chip--ok">placed</span>
      <span>
        {spokenDay(placed.to)} · {placed.from === null ? 'was no day' : `was ${spokenDay(placed.from)}`}
      </span>
      <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" data-undo onclick={undo}>Undo</button>
    </div>
  {/if}

  {#snippet dayRows(day: string, rows: MockRecord[], actionable: boolean)}
    {#each [...rows].sort(byDayOrder) as record (record.id)}
      {#if actionable}
        <button
          class="xb-row xb-row--btn"
          type="button"
          data-set={record.id}
          data-command="record.setField"
          data-placement="today.screen"
          aria-label={`Set a day for ${record.label}`}
          onclick={() => startAssign(record)}
        >
          <span class="cd-urgency" data-lvl={levelOf(today.key, record.day)} aria-hidden="true"></span>
          <span class="cd-rowflex">
            {#each record.courses as course (course.id)}
              {#if course.code}
                <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
              {/if}
            {/each}
          </span>
          <span class="xb-row__t">{record.label}</span>
          <span class="xb-row__m">{record.kind ?? ''}</span>
          <span class="cd-chip cd-chip--outline">set a day</span>
        </button>
      {:else}
        <div class="xb-row">
          <span class="cd-urgency" data-lvl={levelOf(today.key, record.day)} aria-hidden="true"></span>
          <span class="cd-rowflex">
            {#each record.courses as course (course.id)}
              {#if course.code}
                <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
              {/if}
            {/each}
          </span>
          <span class="xb-row__t">{record.label}</span>
          <span class="xb-row__m">{record.kind ?? ''}</span>
          <span class="cd-chip cd-chip--{countdownOf(today.key, record.day).tone}">
            {countdownOf(today.key, record.day).word}
          </span>
        </div>
      {/if}
    {/each}
  {/snippet}

  {#if loading}
    <section class="xb-day">
      <div class="xb-skelday">
        <span class="cd-skel xb-skeldaytile" aria-hidden="true"></span>
        <span class="cd-skel xb-skeldayline" aria-hidden="true"></span>
      </div>
      <div class="cd-card xb-rows">
        {#each [0, 1] as row (row)}
          <span class="cd-skel xb-skelrow" aria-hidden="true"></span>
        {/each}
      </div>
    </section>
    <p class="cd-sr" role="status">Reading the plan’s assessments</p>
  {:else}
    <section class="xb-day">
      {#if picked}
        <header class="xb-dayhead">
          <span class="cd-datetile num">{picked.num}</span>
          <div class="xb-dayhead__id">
            <span class="xb-dayhead__t">{spokenDay(picked.key)}</span>
            {#if picked.records.length > 0}
              <span class="xb-dayhead__s">
                {dayCount(picked.records.length)} · {distanceText(today.key, picked.key)}
              </span>
            {/if}
          </div>
        </header>
        {#if picked.records.length > 0}
          <div class="cd-card xb-rows">
            {@render dayRows(picked.key, picked.records, false)}
          </div>
        {:else}
          <!-- Empty state when no assessments fall on the selected date. -->
          <div class="cd-dashed xb-emptyday">
            <b>Nothing on {spokenDay(picked.key)}</b>
            <span>
              {#if nextBusy(month, picked.key) !== null}
                The next day with an assessment is
                {spokenDay(nextBusy(month, picked.key)?.key ?? '')} ·
                {distanceText(today.key, nextBusy(month, picked.key)?.key ?? today.key)}
              {:else}
                No assessment is left in {month.name}
              {/if}
            </span>
          </div>
        {/if}
      {/if}
    </section>

    <section class="xb-wait">
      <header class="xb-dayhead">
        <span class="xb-slot" aria-hidden="true">–</span>
        <div class="xb-dayhead__id">
          <span class="xb-dayhead__t">No day yet</span>
          <span class="xb-dayhead__s">
            {waiting.length > 0 ? dayCount(waiting.length) : 'Every assessment has a day'}
          </span>
        </div>
      </header>
      {#if waiting.length > 0}
        <div class="cd-card xb-rows">
          {@render dayRows(today.key, waiting, true)}
        </div>
      {:else}
        <div class="cd-dashed">
          <b>Every assessment has a day</b>
          <span>{dayCount(dated.length)} on the plan</span>
        </div>
      {/if}
    </section>
  {/if}
</div>
