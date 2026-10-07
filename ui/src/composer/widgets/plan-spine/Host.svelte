<!-- Plan spine widget host (COMPOSER §3.1). -->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import { rangeText, type PlanTopic, type PlanWeek } from '../../../variants/plan-spine/props';
  import { app } from '../../../session.svelte';
  import { nameOf, recordLabel, type RecordDoc } from '../../../types';
  import {
    ladderOf,
    minutesOf,
    readPlanModel,
    stageWord,
    targets,
    weekIndex,
    weekName,
    workKindOf,
    type PlanModel,
  } from '../../../panels/model';

  /** This surface's own view, and the title the shell names it by. */
  const home = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'plan')?.[0] ?? null,
  );
  const title = $derived(
    app.navigation.find((entry) => entry.view === home)?.title ?? nameOf(home ?? ''),
  );

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

  /** The record door: the app's own `record.panel` for one row. */
  function openTopic(topic: PlanTopic): void {
    void app.run('record.panel', { id: topic.id, type: topic.type });
  }

  function newTopic(): void {
    if (workKind) void app.openSheet({ kind: 'record.new', type: workKind });
  }

  function newWeek(): void {
    if (weekKind) void app.openSheet({ kind: 'record.new', type: weekKind });
  }
</script>

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
