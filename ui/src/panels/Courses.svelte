<!-- SUBJECTS / COURSES (S2): course progress, topic breakdown, and upcoming work. -->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import Variant from '../variants/Variant.svelte';
  import type { CourseCard, CourseTopic } from '../variants/courses/props';
  import { app } from '../session.svelte';
  import { openExternal } from '../ipc';
  import { nameOf, recordLabel, type RecordDoc } from '../types';
  import {
    chapterOf,
    ladderOf,
    launchLinkOf,
    minutesOf,
    readPlanModel,
    stageWord,
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
    void app.reviewsDue().then((result) => {
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
  function topicsOf(subjectId: string, work: RecordDoc[]): CourseTopic[] {
    if (!model) return [];
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
        link: launchLinkOf(model, item),
      };
    });
  }

  /** Every record that links to a record, keyed by target id — one pass for
   *  every course, not one full scan per card per field. */
  function workByTarget(): Map<string, RecordDoc[]> {
    const byTarget = new Map<string, RecordDoc[]>();
    if (!model) return byTarget;
    for (const item of model.items) {
      const seen = new Set<string>();
      for (const ids of Object.values(item.links ?? {})) {
        for (const id of ids) {
          if (seen.has(id)) continue;
          seen.add(id);
          const bucket = byTarget.get(id);
          if (bucket) bucket.push(item);
          else byTarget.set(id, [item]);
        }
      }
    }
    return byTarget;
  }

  /** The first topic that is not done — the row every design marks as next. */
  function nextTopicOf(topics: CourseTopic[]): string | null {
    return topics.find((topic) => topic.state !== 'done')?.id ?? null;
  }

  const courses = $derived.by<CourseCard[]>(() => {
    if (!model) return [];
    const under = workByTarget();
    return (model.identities ?? []).map((record) => {
      const work = under.get(record.id) ?? [];
      const topics = topicsOf(record.id, work);
      let minutes = 0;
      let started = 0;
      for (const topic of topics) {
        minutes += topic.minutes ?? 0;
        if (topic.state !== 'none') started += 1;
      }
      return {
        id: record.id,
        code: codeOf(record),
        name: recordLabel(record),
        wash: app.washes[record.id],
        started,
        total: topics.length,
        minutes,
        days: nextDays(work),
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
    onWatch={(id) => {
      for (const course of courses) {
        const topic = course.topics.find((entry) => entry.id === id);
        if (topic?.link) {
          void openExternal(topic.link);
          return;
        }
      }
    }}
    onNewCourse={() => {
      if (subjectKind) void app.openSheet({ kind: 'record.new', type: subjectKind });
    }}
    onNewTopic={() => {
      if (chapterKind) void app.openSheet({ kind: 'record.new', type: chapterKind });
    }}
  />
{/if}
