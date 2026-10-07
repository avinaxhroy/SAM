<!-- THE PLAN (S2, UI P2 / U2): term timeline and topic schedule. -->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import Variant from '../variants/Variant.svelte';
  import { rangeText, type PlanTopic, type PlanWeek } from '../variants/plan-spine/props';
  import { app } from '../session.svelte';
  import { nameOf, recordLabel, type RecordDoc } from '../types';
  import {
    chapterOf,
    ladderOf,
    minutesOf,
    readPlanModel,
    stageWord,
    targets,
    weekIndex,
    weekName,
    workKindOf,
    type PlanModel,
  } from './model';

  let { title }: { title: string } = $props();

  let model = $state<PlanModel | null>(null);
  let loadedFor: string | null = null;

  /** The week the student is reading; null until they touch it, which opens
   *  the term on the week the engine resolves today into. */
  let chosen = $state<string | null>(null);

  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void readPlanModel().then((read) => {
      model = read;
    });
  });

  /** The plan's work kind: the one that carries a name and a time estimate. */
  const workKind = $derived(model ? workKindOf(model) : null);
  /** The plan's week kind — the ruler every week record extends. */
  const weekKind = $derived(Object.keys(app.types).includes('week') ? 'week' : null);

  /** The two kinds the empty card can add, in the plan's own words. */
  const workWord = $derived(workKind ? nameOf(workKind).toLowerCase() : null);
  const weekWord = $derived(weekKind ? nameOf(weekKind).toLowerCase() : null);

  /** The empty card's sentence: what the plan reads from, named the plan's way. */
  const emptySay = $derived.by(() => {
    if (weekWord && workWord) {
      return `A ${weekWord} is the ruler the plan is measured against, and a ${workWord} is what its work hangs from. Add either and the plan starts to read.`;
    }
    if (weekWord) return `A ${weekWord} is the ruler the plan is measured against. Add the first and the plan starts to read.`;
    if (workWord) return `A ${workWord} is what the plan's work hangs from. Add the first and the plan starts to read.`;
    return 'The plan declares no kind for this screen to write into — its own schema says what belongs here.';
  });

  /** One work record as a design draws it: state, words, minutes, course. */
  function topicOf(item: RecordDoc, plan: PlanModel): PlanTopic {
    const fact = plan.progress.get(item.id);
    const started = (fact?.stages.length ?? 0) > 0;
    const course = targets(plan, item, 'course')[0];
    const kind = item.fields.kind;
    return {
      id: item.id,
      type: item.type,
      label: recordLabel(item),
      course: course ? { label: recordLabel(course), wash: app.washes[course.id] } : null,
      state: fact?.complete ? 'done' : started ? 'step' : 'none',
      stage: stageWord(fact, ladderOf(plan, item.type)),
      kind: typeof kind === 'string' && kind.length > 0 ? kind : null,
      minutes: minutesOf(item),
    };
  }

  /**
   * Where you are, and where that answer came from. Two provenances, in the
   * engine's order of authority:
   *
   *   · `views.today.week` — the week record that covers today, resolved by the
   *     plan, which owns the timezone.
   *   · the week index `today.view` publishes on its own first recommendation.
   *     The engine ordered that queue; this reads its position back rather than
   *     presenting it as fact.
   *
   * A third possibility — the first week with work not done — is refused: it is
   * this screen inventing a priority, which is D1.
   */
  const nowIndex = $derived.by<number | null>(() => {
    if (!model) return null;
    const resolved = app.today?.week?.index ?? null;
    if (resolved !== null && model.weeks.some((week) => weekIndex(model.weeks, week.id) === resolved)) {
      return resolved;
    }
    const next = app.todayFacts?.groups.find((group) => group.id === 'next')?.items ?? [];
    const first = next.find((item) => typeof item.weekIndex === 'number');
    if (first?.weekIndex != null && model.weeks.some((week) => weekIndex(model.weeks, week.id) === first.weekIndex)) {
      return first.weekIndex;
    }
    return null;
  });

  /**
   * The weeks in the plan's order, each with its work and its own totals. The
   * numbers are all counts of records the read returned: how many things sit in
   * the week, how many minutes their own `est` fields add up to, and how many
   * the engine reports complete. Nothing here is a ranking and nothing is a
   * second evaluator — progress is `reviews.due`'s answer, read straight
   * through.
   */
  const weeks = $derived.by<PlanWeek[]>(() => {
    if (!model) return [];
    const ordered = [...model.weeks].sort(
      (left, right) =>
        (weekIndex(model.weeks, left.id) ?? 9999) - (weekIndex(model.weeks, right.id) ?? 9999),
    );
    const plan = model;
    return ordered.map((week, position) => {
      const index = weekIndex(plan.weeks, week.id) ?? position + 1;
      const topics = plan.items
        .filter((item) => (item.links.week ?? []).includes(week.id))
        .map((item) => topicOf(item, plan));
      const minutes = topics.reduce((sum, topic) => sum + (topic.minutes ?? 0), 0);
      const start = typeof week.fields.start === 'string' ? week.fields.start : null;
      const end = typeof week.fields.end === 'string' ? week.fields.end : null;
      const phase = week.fields.phase;
      return {
        id: week.id,
        index,
        name: weekName(week, position + 1),
        dates: rangeText(start, end),
        phase: typeof phase === 'string' && phase.length > 0 ? phase : null,
        topics,
        minutes,
        done: topics.filter((topic) => topic.state === 'done').length,
        now: nowIndex === index,
      };
    });
  });

  /** Work that names no week: stated as a fact, never silently dropped. */
  const unplaced = $derived.by<PlanTopic[]>(() =>
    model
      ? model.items
          .filter((item) => (item.links.week ?? []).length === 0)
          .map((item) => topicOf(item, model))
      : [],
  );

  const summary = $derived({
    weeks: weeks.length,
    things: model?.items.length ?? 0,
    done: weeks.reduce((sum, week) => sum + week.done, 0),
    minutes: weeks.reduce((sum, week) => sum + week.minutes, 0),
    unplaced: unplaced.length,
  });

  /** The week the designs open on: the student's choice, else today's week. */
  const selectedId = $derived(chosen ?? weeks.find((week) => week.now)?.id ?? weeks[0]?.id ?? null);

  /**
   * A plan with no weeks is not a plan with nothing in it. When the ruler is
   * absent the work is gathered under the chapter each record names — the
   * plan's own structure, one level down — so the screen still reads as a plan.
   * No design covers this case (the lab only draws a term), so the shell draws
   * it itself.
   */
  const byChapter = $derived.by(() => {
    if (!model || model.weeks.length > 0) return [];
    const groups = new Map<string, RecordDoc[]>();
    for (const item of model.items) {
      const label = chapterOf(model, item, '');
      const bucket = groups.get(label);
      if (bucket) bucket.push(item);
      else groups.set(label, [item]);
    }
    return [...groups.entries()].map(([label, work]) => ({ label, work }));
  });

  /** The record door: the app's own `record.panel` for one row. */
  function openTopic(topic: PlanTopic): void {
    void app.run('record.panel', { id: topic.id, type: topic.type });
  }

  /** The same door for a record the shell has not shaped into a topic yet
   *  (the chapter fallback, where the screen is not drawing a design). */
  function openRecord(item: RecordDoc): void {
    void app.run('record.panel', { id: item.id, type: item.type });
  }

  function newTopic(): void {
    if (workKind) void app.openSheet({ kind: 'record.new', type: workKind });
  }

  function newWeek(): void {
    if (weekKind) void app.openSheet({ kind: 'record.new', type: weekKind });
  }
</script>

{#if !model}
  <!-- Loading borrows the arriving geometry: a week band and its work rows are
       the collection's own two heights, so the reveal moves nothing. -->
  <div class="cd-card" aria-busy="true">
    <p class="cd-sr" role="status">Reading the plan…</p>
    <div class="cd-skel__rows">
      {#each [0, 1, 2, 3, 4] as row (row)}
        <div class="cd-skel__row" class:tw-skel-band={row % 2 === 0}>
          <span class="cd-skel"></span>
          <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 34 : 58}%`}></span>
          <span class="cd-skel"></span>
        </div>
      {/each}
    </div>
  </div>
{:else if weeks.length === 0 && byChapter.length > 0}
  <div class="cd-stack">
    {#each byChapter as group (group.label)}
      <section class="cd-card" data-chapter={group.label}>
        <header class="cd-card__head">
          <span class="cd-ictile"><Icon name="squareStack" /></span>
          <div>
            <h2 class="cd-card__title">{group.label}</h2>
            <p class="cd-card__sub">
              {group.work.length} {group.work.length === 1 ? 'thing' : 'things'} ·
              {group.work.filter((item) => (model?.progress.get(item.id)?.stages.length ?? 0) > 0).length} started
            </p>
          </div>
        </header>
        <div class="cd-coll">
          {#each group.work as item (item.id)}
            {@const fact = model?.progress.get(item.id)}
            {@const ladder = model ? ladderOf(model, item.type) : []}
            <button
              class="cd-coll__row cd-coll__row--btn"
              data-record-id={item.id}
              data-done={fact?.complete ? '1' : undefined}
              data-command="record.panel"
              data-placement="recordTable.rowContext"
              onclick={() => openRecord(item)}
            >
              <span class="cd-coll__body">
                <span class="cd-coll__title">{recordLabel(item)}</span>
                <span class="cd-rowmeta">
                  <span>{nameOf(item.type)}</span>
                  <span>{stageWord(fact, ladder)}</span>
                  {#if minutesOf(item)}<span>{minutesOf(item)} min</span>{/if}
                </span>
              </span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
  </div>
{:else if weeks.length === 0}
  <section class="cd-card">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="calendar" size={24} /></span>
      <div class="cd-empty__t">This plan has nothing in it yet</div>
      <div class="cd-empty__s">
        {emptySay}
      </div>
      {#if weekKind && weekWord}
        <button
          class="cd-pill cd-pill--ghost cd-pill--sm"
          type="button"
          data-command={`${weekKind}.new`}
          data-placement="today.screen"
          onclick={newWeek}
        >
          <Icon name="plus" size={13} />
          Add the first {weekWord}
        </button>
      {/if}
      {#if workKind && workWord}
        <button
          class="cd-pill cd-pill--ghost cd-pill--sm"
          type="button"
          data-command={`${workKind}.new`}
          data-placement="today.screen"
          onclick={newTopic}
        >
          <Icon name="plus" size={13} />
          Add the first {workWord}
        </button>
      {/if}
    </div>
  </section>
{:else}
  <!-- The design: its own head, its own body, its own interaction. -->
  <Variant
    surface="plan-spine"
    {title}
    {weeks}
    {unplaced}
    {summary}
    {selectedId}
    newWeekCommand={weekKind ? `${weekKind}.new` : null}
    newTopicCommand={workKind ? `${workKind}.new` : null}
    onSelect={(id) => (chosen = id)}
    onTopic={openTopic}
    onNewWeek={newWeek}
    onNewTopic={newTopic}
  />

  <!-- The page's own foot, drawn once for all three designs because the lab
       draws it identically under all three stages: the ruler's own door, and
       the work the plan holds but has not dated. A second door for a different
       act is not a second door for the same one, and the ruler's door belongs
       where the ruler ends. -->
  <div class="ps-tail">
    {#if weekKind}
      <button
        class="ps-write"
        type="button"
        data-command={`${weekKind}.new`}
        data-placement="today.screen"
        onclick={newWeek}
      >
        <Icon name="plus" size={13} />
        New week
      </button>
    {/if}

    {#if unplaced.length > 0}
      <section class="cd-card ps-slip">
        <header class="cd-card__head">
          <span class="cd-ictile"><Icon name="trayFull" /></span>
          <div>
            <h2 class="cd-card__title">Not placed in a week</h2>
            <p class="cd-card__sub">
              {unplaced.length} {unplaced.length === 1 ? 'thing' : 'things'} the plan holds but has not dated — they
              still count.
            </p>
          </div>
        </header>
        <div class="ps-colls">
          {#each unplaced as topic (topic.id)}
            <button
              class="ps-row"
              type="button"
              data-record-id={topic.id}
              data-done={topic.state === 'done' ? '1' : undefined}
              data-command="record.panel"
              data-placement="recordTable.rowContext"
              onclick={() => openTopic(topic)}
            >
              {#if topic.state !== 'none'}
                <span class="cd-urgency" data-lvl={topic.state === 'done' ? 'done' : 'next'} aria-hidden="true"></span>
              {/if}
              <span class="ps-row__body">
                <span class="ps-row__t">{topic.label}</span>
                <span class="ps-row__m num">
                  {#if topic.course}<span class="cd-chip cd-chip--wash" data-w={topic.course.wash}>{topic.course.label}</span>{/if}
                  <span>{topic.stage}</span>
                  {#if topic.minutes !== null}<span>{topic.minutes} min</span>{/if}
                </span>
              </span>
            </button>
          {/each}
        </div>
      </section>
    {/if}
  </div>
{/if}
