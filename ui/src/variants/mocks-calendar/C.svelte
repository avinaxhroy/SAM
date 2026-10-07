<!--
  Mocks & Assessments · Variant C (Guided Steps).
  Step-by-step workflow card for scheduling undated assessments: pick an available
  weekday, preview scheduling impact, and confirm. Completed assessments list below.
-->
<script lang="ts">
  import {
    byDayOrder,
    capsDay,
    countdownOf,
    dateOf,
    dayCompany,
    dayCount,
    distanceText,
    levelOf,
    recordsByDay,
    spokenDay,
    type MocksProps,
  } from './props';

  let {
    title,
    loading,
    today,
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

  const WEEKDAYS = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'];

  const byDay = $derived(recordsByDay(dated));
  /** The record the card holds; null means the head of the waiting queue. */
  let held = $state<string | null>(null);
  let step = $state<1 | 2>(1);
  let dir = $state(1);
  let pick = $state<string | null>(null);
  let placed = $state<{ id: string; from: string | null; to: string } | null>(null);
  let busy = $state(false);
  let focusOn = $state<string | null>(null);

  $effect(() => {
    const selector = focusOn;
    if (selector === null) return;
    focusOn = null;
    document.querySelector<HTMLElement>(selector)?.focus({ preventScroll: true });
  });

  const subject = $derived(
    (held === null ? waiting[0] : (waiting.find((r) => r.id === held) ?? waiting[0])) ?? null,
  );
  const sub = $derived(
    waiting.length === 0
      ? 'Every assessment has a day'
      : waiting.length === 1
        ? '1 assessment has no day'
        : `${waiting.length} assessments have no day`,
  );

  async function commit(): Promise<void> {
    /* Cache current record and target day before the mutation re-reads the plan. */
    const record = subject;
    const day = pick;
    if (record === null || day === null || busy) return;
    busy = true;
    try {
      const accepted = await onSetDay(record.id, day);
      if (!accepted) return;
      placed = { id: record.id, from: record.day, to: day };
      /* Advance to the next undated assessment at step 1. */
      held = null;
      step = 1;
      dir = 1;
      pick = null;
      focusOn = '[data-undo]';
    } finally {
      busy = false;
    }
  }

  function undo(): void {
    const back = placed;
    placed = null;
    onUndo();
    if (back !== null) {
      held = back.id;
      step = 1;
      dir = 1;
      pick = null;
      focusOn = '[data-opt]';
    }
  }
</script>

<div class="v-fit xc">
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{sub}</p>
    </div>
    <span class="cd-pagehead__aside">
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

  {#if loading}
    <section class="cd-card xc-skelstep">
      <span class="cd-skel xc-skelline" aria-hidden="true"></span>
      <span class="cd-skel xc-skeltile" aria-hidden="true"></span>
      <span class="cd-skel xc-skelpill" aria-hidden="true"></span>
    </section>
    <section class="cd-card xc-rows xc-skelrows">
      {#each [0, 1, 2] as row (row)}
        <span class="cd-skel xc-skelrow" aria-hidden="true"></span>
      {/each}
    </section>
    <p class="cd-sr" role="status">Reading the plan’s assessments</p>
  {:else}
    {#if subject === null}
      <section class="cd-card xc-done">
        <div class="cd-empty">
          <span class="cd-empty__art">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>
          </span>
          <div class="cd-empty__t">Every assessment has a day</div>
          <p class="cd-empty__s">{dayCount(dated.length)} on the plan</p>
        </div>
      </section>
    {:else}
      <section class="cd-card xc-steps" aria-label={subject.label}>
        <div class="xc-steps__head">
          <div class="xc-steps__id">
            <div class="cd-rowflex">
              {#each subject.courses as course (course.id)}
                {#if course.code}
                  <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
                {/if}
              {/each}
              <span class="cd-urgency" data-lvl={levelOf(today.key, subject.day)} aria-hidden="true"></span>
            </div>
            <h2 class="xc-steps__t">{subject.label}</h2>
            <span class="xc-steps__s">
              {[subject.kind, subject.week].filter(Boolean).join(' · ')}
            </span>
          </div>
          <div class="xc-dots" aria-hidden="true">
            <i class="xc-dot" data-on={step === 1 ? '1' : '0'}></i>
            <i class="xc-dot" data-on={step === 2 ? '1' : '0'}></i>
          </div>
        </div>

        <div class="xc-body">
          {#if step === 1}
            <div class="xc-step" style={`--dir:${dir}`} role="group" aria-label={`A day for ${subject.label}`}>
              <div class="xc-opts">
                {#each quiet as day (day)}
                  <button class="xc-opt" type="button" data-opt={day} aria-pressed={day === pick} onclick={() => (pick = day)}>
                    <span class="xc-opt__w">{WEEKDAYS[dateOf(day).getDay()]}</span>
                    <span class="xc-opt__d num">{dateOf(day).getDate()}</span>
                    <span class="xc-opt__x num">{distanceText(today.key, day)}</span>
                  </button>
                {/each}
              </div>
            </div>
          {:else if pick !== null}
            <div class="xc-step" style={`--dir:${dir}`}>
              <div class="xc-conf">
                <span class="cd-datetile num">{dateOf(pick).getDate()}</span>
                <div class="xc-conf__id">
                  <span class="xc-conf__d">{spokenDay(pick)}</span>
                  <span class="xc-conf__s num">
                    {distanceText(today.key, pick)} · {dayCompany(byDay, pick, subject.id)}
                  </span>
                </div>
              </div>
            </div>
          {/if}
        </div>

        <div class="xc-steps__foot">
          <button
            class="cd-pill cd-pill--quiet"
            type="button"
            disabled={step === 1}
            onclick={() => {
              step = 1;
              dir = -1;
              focusOn = pick === null ? '[data-opt]' : `[data-opt="${pick}"]`;
            }}
          >
            Back
          </button>
          <div class="xc-acts">
            {#if step === 1}
              <button
                class="cd-pill"
                type="button"
                data-next
                disabled={pick === null}
                onclick={() => {
                  dir = 1;
                  step = 2;
                  focusOn = '[data-commit]';
                }}
              >
                Continue
              </button>
            {:else}
              <button
                class="cd-pill"
                type="button"
                data-command="record.setField"
                data-placement="today.screen"
                data-commit
                disabled={busy || pick === null}
                onclick={() => void commit()}
              >
                Put it on {pick === null ? 'a day' : capsDay(pick)}
              </button>
            {/if}
          </div>
        </div>
      </section>
    {/if}

    {#if placed !== null}
      <div class="x-receipt">
        <span class="cd-chip cd-chip--ok">placed</span>
        <span>
          {spokenDay(placed.to)} · {placed.from === null ? 'was no day' : `was ${spokenDay(placed.from)}`}
        </span>
        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" data-undo onclick={undo}>Undo</button>
      </div>
    {/if}

    <section class="xc-list">
      <div class="xc-listhead">
        <h2>On the plan</h2>
        <span class="cd-chip">{dated.length}</span>
      </div>
      <div class="cd-card xc-rows">
        {#each [...dated].sort(byDayOrder) as record (record.id)}
          <div class="xc-row">
            <span class="cd-urgency" data-lvl={levelOf(today.key, record.day)} aria-hidden="true"></span>
            <span class="cd-rowflex">
              {#each record.courses as course (course.id)}
                {#if course.code}
                  <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
                {/if}
              {/each}
            </span>
            <span class="xc-row__t">{record.label}</span>
            <span class="xc-row__d num">{record.day === null ? '' : capsDay(record.day)}</span>
            <span class="cd-chip cd-chip--{countdownOf(today.key, record.day).tone}">
              {countdownOf(today.key, record.day).word}
            </span>
          </div>
        {/each}
      </div>
    </section>
  {/if}
</div>
