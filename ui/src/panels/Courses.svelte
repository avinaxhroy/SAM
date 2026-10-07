<!--
  SUBJECTS / COURSES (S2).
  Identity layer displaying course progress, topic breakdown, and upcoming work.
  Reads the plan model and `reviews.due` schedule to supply props for the
  selected courses variant (`variants/courses/props.ts`).
-->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import Variant from '../variants/Variant.svelte';
  import type { CourseCard, CourseTopic } from '../variants/courses/props';
  import { app } from '../session.svelte';
  import { nameOf, recordLabel, type RecordDoc } from '../types';
  import {
    chapterOf,
    ladderOf,
    minutesOf,
    readPlanModel,
    stageWord,
    workUnder,
    type PlanModel,
  } from './model';

  let { title }: { title: string } = $props();

  let model = $state<PlanModel | null>(null);
  let loadedFor: string | null = null;
  /** The course whose detail is open; each design draws it its own way. */
  let opened = $state<string | null>(null);

  /**
   * The plan's own calendar, for the one date the designs rank by. The read is
   * the same `reviews.due` the collection layer already makes and caches
   * (`model.ts:64`), so this costs no second read of anything.
   */
  let clock = $state<{ today: string; timezoneMinutes: number } | null>(null);

  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void app.run('reviews.due', {}, { tracked: false }).then((result) => {
      const data = result?.data as { today?: string; timezoneMinutes?: number } | undefined;
      clock =
        data && typeof data.today === 'string'
          ? { today: data.today, timezoneMinutes: data.timezoneMinutes ?? 0 }
          : null;
    });
    void readPlanModel().then((read) => {
      model = read;
    });
  });

  /** The plan's own civil date for an instant, in the plan's own timezone. */
  function planDay(iso: string | null | undefined): string | null {
    if (!iso || !clock) return null;
    const at = Date.parse(iso);
    if (Number.isNaN(at)) return null;
    return new Date(at + clock.timezoneMinutes * 60_000).toISOString().slice(0, 10);
  }

  function daysFromToday(day: string): number | null {
    if (!clock) return null;
    const a = Date.parse(`${clock.today}T00:00:00Z`);
    const b = Date.parse(`${day}T00:00:00Z`);
    if (Number.isNaN(a) || Number.isNaN(b)) return null;
    return Math.round((b - a) / 86_400_000);
  }

  /**
   * A course's nearest dated thing, as a **distance in days** (what C's
   * featured card ranks by), or null when the course has no dated work — in
   * which case the design never invents a deadline the plan has not set.
   */
  function nextDays(work: RecordDoc[]): number | null {
    let best: number | null = null;
    for (const item of work) {
      const value = item.fields.focus;
      if (typeof value !== 'string' || value.length === 0) continue;
      const day = planDay(value) ?? value.slice(0, 10);
      const days = daysFromToday(day);
      if (days === null) continue;
      if (best === null || days < best) best = days;
    }
    return best;
  }

  /** A course's own code, which is what a student says out loud (D2). */
  function codeOf(record: RecordDoc): string | null {
    const value = record.fields.code ?? record.fields.short;
    return typeof value === 'string' && value.length > 0 ? value : null;
  }

  /** One work record as the designs draw it: state, rung, minutes, words. */
  function topicsOf(subjectId: string): CourseTopic[] {
    if (!model) return [];
    const work = workUnder(model, subjectId);
    const ladder = ladderOf(model, work[0]?.type ?? '');
    return work.map((item) => {
      const fact = model?.progress.get(item.id);
      const started = (fact?.stages.length ?? 0) > 0;
      const done = fact?.complete === true;
      return {
        id: item.id,
        label: recordLabel(item),
        type: item.type,
        chapter: chapterOf(model, item, subjectId),
        state: done ? ('done' as const) : started ? ('step' as const) : ('none' as const),
        rung: fact?.stages.length ?? 0,
        rungs: ladder.length,
        minutes: minutesOf(item),
        stage: stageWord(fact, ladder),
      };
    });
  }

  /** The first topic that is not done — the row every design marks as next. */
  function nextTopicOf(topics: CourseTopic[]): string | null {
    return topics.find((topic) => topic.state !== 'done')?.id ?? null;
  }

  const courses = $derived.by<CourseCard[]>(() => {
    if (!model) return [];
    return (model.identities ?? []).map((record) => {
      const topics = topicsOf(record.id);
      const minutes = workUnder(model, record.id).reduce((sum, item) => sum + (minutesOf(item) ?? 0), 0);
      return {
        id: record.id,
        code: codeOf(record),
        name: recordLabel(record),
        wash: app.washes[record.id],
        started: topics.filter((topic) => topic.state !== 'none').length,
        total: topics.length,
        minutes,
        days: nextDays(workUnder(model, record.id)),
        nextId: nextTopicOf(topics),
        topics,
      };
    });
  });

  const summary = $derived({
    courses: courses.length,
    topics: courses.reduce((sum, course) => sum + course.total, 0),
    started: courses.reduce((sum, course) => sum + course.started, 0),
  });

  /**
   * The kinds this screen creates: a subject (the plan's identity kind) and a
   * chapter (the kind a plan nests under one). Both are the plan's own
   * declarations — the screen never invents a kind to write into.
   */
  const subjectKind = $derived(model?.identityKinds[0] ?? null);
  /** The plan's own word for that kind (`course`, `track`) — never a fixed noun. */
  const courseWord = $derived(subjectKind ? nameOf(subjectKind).toLowerCase() : null);
  const chapterKind = $derived(
    model
      ? (Object.keys(app.types).find(
          (name) =>
            !model?.workKinds.includes(name) &&
            !model?.identityKinds.includes(name) &&
            name !== 'week' &&
            Boolean(app.types[name]?.parent && model?.identityKinds.includes(app.types[name].parent ?? '')),
        ) ?? null)
      : null,
  );

  /** The record door: the app's own `record.panel` for one topic row. */
  function openTopic(id: string): void {
    for (const course of courses) {
      const topic = course.topics.find((entry) => entry.id === id);
      if (topic) {
        void app.run('record.panel', { id, type: topic.type });
        return;
      }
    }
  }
</script>

{#if !model}
  <!-- Loading carries the screen's head — the design owns the head once data
       arrives, so without this the title would vanish for a frame and the
       skeleton would float. -->
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">Reading the plan…</p>
    </div>
  </header>
  <!-- Loading borrows the arriving geometry: a course tile is 193px
       (`.cd-skel--card`), and three is the number the system itself draws for a
       three-course plan — the identity system is capped at four washes
       (`design.md` §7.2), so a plan cannot be waiting on a fifth. -->
  <div class="cd-tiles" aria-busy="true">
    <p class="cd-sr" role="status">Reading the subjects…</p>
    {#each [0, 1, 2] as slot (slot)}
      <div class="cd-skel cd-skel--card" aria-hidden="true"></div>
    {/each}
  </div>
{:else if courses.length === 0}
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{courseWord ? `No ${courseWord}s yet` : 'Nothing here yet'}</p>
    </div>
  </header>
  <section class="cd-card">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="book" size={24} /></span>
      <div class="cd-empty__t">
        {courseWord ? `This plan has no ${courseWord}s yet` : 'This plan has nothing here yet'}
      </div>
      <div class="cd-empty__s">
        {courseWord
          ? `A ${courseWord} is where its work lives — the plan's own records decide what belongs under it. Add one and the work gathers there.`
          : 'The plan declares no kind for this screen to write into — its own schema says what belongs here.'}
      </div>
      {#if subjectKind}
        <!-- The door the rest of the screen uses (`variants/courses/A.svelte`):
             it adds the plan's own identity kind, never a list of the app's. -->
        <button
          class="cd-pill cd-pill--ghost cd-pill--sm"
          type="button"
          data-command={`${subjectKind}.new`}
          data-placement="today.screen"
          onclick={() => void app.openSheet({ kind: 'record.new', type: subjectKind })}
        >
          Add your first {courseWord}
        </button>
      {:else}
        <button
          class="cd-pill cd-pill--ghost cd-pill--sm"
          type="button"
          data-command="list.new"
          data-placement="today.screen"
          onclick={() => void app.openSheet({ kind: 'list.new' })}
        >
          Add a list
        </button>
      {/if}
    </div>
  </section>
{:else}
  <Variant
    surface="courses"
    {title}
    {courses}
    {summary}
    openedId={opened}
    newCourseCommand={subjectKind ? `${subjectKind}.new` : null}
    newTopicCommand={chapterKind ? `${chapterKind}.new` : null}
    onOpen={(id) => (opened = id)}
    onClose={() => (opened = null)}
    onTopic={openTopic}
    onNewCourse={() => {
      if (subjectKind) void app.openSheet({ kind: 'record.new', type: subjectKind });
    }}
    onNewTopic={() => {
      if (chapterKind) void app.openSheet({ kind: 'record.new', type: chapterKind });
    }}
  />
{/if}
