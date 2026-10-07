<!-- TODAY (S1, UI P2 / U1): daily study focus and queue derived from todayFacts. -->
<script lang="ts">
  import Variant from '../variants/Variant.svelte';
  import Button from '../records/Button.svelte';
  import type { Coverage, TodayFocusProps, WhyFact } from '../variants/today-focus/props';
  import { coverageOf, openRecord, setAside } from '../variants/today-focus/host';
  import { rowsOf } from '../variants/coming-up/props';
  import { app } from '../session.svelte';
  import { durationText, nameOf, recordLabel, todayGroup, type TodayGroupId, type TodayItem } from '../types';
  import { readPlanModel, workKindOf, type PlanModel } from './model';

  /** Queue groups and labels in engine resolution order. */
  const QUEUE: Array<{ id: TodayGroupId; label: string }> = [
    { id: 'committed', label: 'Planned for today' },
    { id: 'late', label: 'Late for review' },
    { id: 'due', label: 'Due today' },
    { id: 'next', label: 'Next in the plan' },
    { id: 'stale', label: 'Not touched in a while' },
  ];

  const rows = $derived(
    QUEUE.flatMap((group) =>
      (todayGroup(app.todayFacts, group.id)?.items ?? []).map((item) => ({
        item,
        group: group.id,
        label: group.label,
      })),
    ),
  );
  const counts = $derived(
    QUEUE.map((group) => {
      const fact = todayGroup(app.todayFacts, group.id);
      return { ...group, count: fact?.count ?? 0, sent: fact?.items.length ?? 0 };
    }),
  );
  const upcoming = $derived(todayGroup(app.todayFacts, 'upcoming')?.items ?? []);
  const totals = $derived(app.todayFacts?.totals ?? null);
  const days = $derived(app.todayFacts?.days ?? []);
  const weekTotal = $derived(days.reduce((sum, day) => sum + day.loggedMin, 0));

  const focus = $derived(rows[0] ?? null);
  let targetMin = $state(25);

  /** Session creation command for configured time-logging kind, or null if unconfigured. */
  const sessionCommand = $derived(app.sessionKind() ? `${app.sessionKind()}.new` : null);

  const planView = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'plan')?.[0] ?? null,
  );

  function greeting(hour: number): string {
    if (hour < 5) return 'Still up';
    if (hour < 12) return 'Good morning';
    if (hour < 17) return 'Good afternoon';
    return 'Good evening';
  }

  function dueWords(dueDate: string): string | null {
    const at = new Date(dueDate);
    if (Number.isNaN(at.getTime())) return null;
    const weekday = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][at.getDay()];
    const clock = `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}`;
    return `${weekday} ${clock}`;
  }

  function whyFactOf(item: TodayItem): WhyFact | null {
    if (item.lateDays > 0)
      return { key: 'Late', value: item.lateDays === 1 ? '1 day late' : `${item.lateDays} days late` };
    if (item.dueDate) {
      const words = dueWords(item.dueDate);
      if (words) return { key: 'Due', value: words };
    }
    if (item.reviewedDaysAgo !== null)
      return {
        key: 'Last session',
        value:
          item.reviewedDaysAgo === 0
            ? 'today'
            : `${item.reviewedDaysAgo} ${item.reviewedDaysAgo === 1 ? 'day' : 'days'} ago`,
      };
    if (item.daysUntil !== null)
      return {
        key: 'Next assessment',
        value:
          item.daysUntil === 0
            ? 'today'
            : item.daysUntil === 1
              ? 'tomorrow'
              : `in ${item.daysUntil} days`,
      };
    if (item.est !== null) return { key: 'Estimated', value: `${item.est} min` };
    return null;
  }

  /** Masthead status mode derived from active focus group. */
  const focusMode = $derived<'decision' | 'clear' | 'gap'>(
    !focus ? 'clear' : focus.group === 'stale' ? 'gap' : 'decision',
  );

  const dateLine = $derived(
    app.todayFacts ? `${app.todayFacts.weekday} ${app.todayFacts.day} ${app.todayFacts.month}` : '',
  );

  /**
   * The focus card's arc — the course's coverage, read through the shared host
   * helper so this screen and the composer's widget print the same number for
   * the same course. The key carries the plan's revision AND the focused
   * course: a publication re-reads, and so does the day ranking the next thing
   * — never a state change that leaves the last course's arc on the card.
   */
  let coverage = $state<Coverage | null>(null);
  let coverageFor: string | null = null;
  $effect(() => {
    const key = `${app.revision ?? ''}|${focus?.item.course?.id ?? ''}`;
    if (coverageFor === key) return;
    coverageFor = key;
    void coverageOf(focus?.item.course ?? null).then((read) => {
      if (coverageFor === key) coverage = read;
    });
  });

  const focusProps = $derived<TodayFocusProps>({
    greeting: greeting(new Date().getHours()),
    dateLine,
    mode: focusMode,
    title: focus?.item.label ?? null,
    course: focus?.item.course ? { label: focus.item.course.label, wash: focus.item.course.wash } : null,
    tag: focus?.label ?? null,
    why: focus ? whyFactOf(focus.item) : null,
    studiedToday: durationText(totals?.loggedMin ?? 0),
    minutes: targetMin,
    onMinutes: (value) => (targetMin = value),
    sessionCommand,
    onStart: () => {
      if (focus) void app.startSession(focus.item, targetMin);
    },
    onPlan: () => void app.openSheet({ kind: 'today.add' }),
    coverage,
    kind: focus?.item.kind ?? null,
    onOpen: () => {
      if (focus) openRecord(focus.item);
    },
    onDefer: () => {
      if (focus) void setAside(focus.item, app.todayFacts?.date ?? null);
    },
  });

  /** Normalized upcoming rows via `rowsOf`. */
  const comingUpRows = $derived(rowsOf(upcoming));

  // Empty plan state (Appendix C.5): plan exists on disk but contains zero records.
  const emptyPlan = $derived(app.todayFacts !== null && app.todayFacts.records === 0);

  /**
   * The plan's own records, for the one state `todayFacts` cannot show: a plan
   * that has a course and no work under it. The read is cached per revision
   * (`model.ts:45`), so a screen that already opened costs nothing here.
   */
  let model = $state<PlanModel | null>(null);
  let loadedFor: string | null = null;
  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void readPlanModel().then((read) => {
      model = read;
    });
  });

  /**
   * A course in the plan and nothing under it yet: the focus slot has no work
   * to carry, so it teaches the one thing to add instead of drawing an empty
   * day (`Plan.svelte:206` draws the same door once work exists).
   */
  const needsWork = $derived(Boolean(model && model.identities.length > 0 && model.items.length === 0));
  /** The kind a first piece of work goes in — named and timed, per `workKindOf`. */
  const workKind = $derived(model ? workKindOf(model) : null);
  const workWord = $derived(workKind ? nameOf(workKind).toLowerCase() : null);
  /** The plan's own courses, as the sentence's subject. */
  const courses = $derived.by(() => {
    const names = (model?.identities ?? []).map((record) => recordLabel(record));
    if (names.length === 0) return null;
    if (names.length === 1) return { names: names[0], plural: false };
    if (names.length === 2) return { names: `${names[0]} and ${names[1]}`, plural: true };
    return { names: `${names[0]} and ${names.length - 1} more`, plural: true };
  });

  /** Initial record type name for empty plan onboarding prompt. */
  const identityKind = $derived(app.identityKind());
  const firstKind = $derived(identityKind ? nameOf(identityKind).toLowerCase() : null);

  async function openPlan(): Promise<void> {
    if (planView) await app.run('rail.select', { view: planView });
  }

  /** Open detail panel for target dated item (`record.panel`). */
  function openDated(id: string): void {
    const item = upcoming.find((row) => row.id === id);
    if (item) void app.run('record.panel', { id, type: item.kind });
  }
</script>

{#if !app.todayFacts}
  <!-- Loading skeleton matching focus card and queue geometry. -->
  <div class="cd-stack" aria-busy="true">
    <p class="cd-sr" role="status">Reading today…</p>
    <div class="tw-focus-skel" aria-hidden="true">
      <div class="tw-focus-skel__row">
        <span class="cd-skel" style="--w: 92px"></span>
        <span class="cd-skel" style="--w: 132px"></span>
      </div>
      <span class="cd-skel" style="--w: 62%"></span>
      <span class="cd-skel" style="--w: 38%"></span>
      <div class="tw-focus-skel__row">
        <span class="cd-skel" style="--w: 148px"></span>
        <span class="cd-skel" style="--w: 128px"></span>
      </div>
    </div>
    <section class="cd-card">
      <div class="cd-skel__rows">
        {#each [0, 1, 2, 3, 4, 5] as row (row)}
          <div class="cd-skel__row cd-skel__row--coll tw-skel-tall">
            <span class="cd-skel"></span>
            <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 58 : 41}%`}></span>
            <span class="cd-skel"></span>
          </div>
        {/each}
      </div>
    </section>
  </div>
{:else if emptyPlan}
  <!-- Empty plan (Appendix C.5, R11): the screen the student lands on right
       after a plan is made. It names the one thing to add, says the loop in its
       own words so the cards that arrive later can be read, and hands over to
       the document for the whole story. The press is the identity kind's own
       door; a plan whose schema has no identity kind falls back to its outline
       rather than to nothing, so no empty plan is a dead end. -->
  <div class="cd-stack">
    <header class="cd-pagehead">
      <h1 class="cd-pagehead__title">{greeting(new Date().getHours())}</h1>
      <p class="cd-pagehead__sub">{dateLine}</p>
    </header>
    <section class="cd-card tw-first">
      <h2 class="cd-card__title">Your plan is empty</h2>
      <p class="cd-card__sub">
        It is on disk and open, and there is nothing in it yet — nothing due, nothing late.
        {firstKind
          ? `The first ${firstKind} is yours to add; the rest of the plan grows from it.`
          : 'Add a record and the rest of the plan grows from it.'}
      </p>
      <p class="cd-card__sub">
        A course holds its topics, and a session is where a topic is studied. What a session
        leaves behind — your grade, and how long you worked — is what the plan reads to say
        what comes next.
      </p>
      <div class="tw-first__acts">
        {#if identityKind}
          <Button
            id={`${identityKind}.new`}
            placement="today.screen"
            label={firstKind ? `Add your first ${firstKind}` : 'Add a record'}
            size="lg"
            hint={false}
            onClick={() => void app.openSheet({ kind: 'record.new', type: identityKind })}
          />
        {:else if planView}
          <button
            class="cd-pill cd-pill--lg"
            type="button"
            data-command="rail.select"
            data-placement="today.screen"
            onclick={() => void openPlan()}
          >
            Open the plan
          </button>
        {/if}
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          data-command="app.gettingStarted"
          data-placement="today.screen"
          onclick={() => void app.run('app.gettingStarted')}
        >
          How SAM works
        </button>
      </div>
    </section>
  </div>
{:else}
  <div class="cd-stack">
  {#if needsWork}
    <!-- Work's door: a course is in the plan and nothing hangs under it, so
         the focus slot carries the one addition that starts the loop instead
         of drawing a day with nothing in it. The press is the work kind's own
         door; a plan that declares no trackable kind falls back to its outline
         rather than to nothing, so the state is never a dead end. -->
    <section class="cd-card tw-first">
      <h2 class="cd-card__title">Nothing to study yet</h2>
      <p class="cd-card__sub">
        {courses?.names}{courses?.plural ? ' are' : ' is'} in the plan, and nothing hangs under
        {courses?.plural ? 'them' : 'it'} yet. {workWord
          ? `A ${workWord} is what a session runs on — add the first one and Today starts carrying the work.`
          : 'Work is what a session runs on; the plan declares no kind of it yet.'}
      </p>
      <p class="cd-card__sub">
        What a session leaves behind — your grade, and how long you worked — is what the plan
        reads to say what comes next.
      </p>
      <div class="tw-first__acts">
        {#if workKind && workWord}
          <Button
            id={`${workKind}.new`}
            placement="today.screen"
            label={`Add the first ${workWord}`}
            size="lg"
            hint={false}
            onClick={() => void app.openSheet({ kind: 'record.new', type: workKind })}
          />
        {:else if planView}
          <button
            class="cd-pill cd-pill--lg"
            type="button"
            data-command="rail.select"
            data-placement="today.screen"
            onclick={() => void openPlan()}
          >
            Open the plan
          </button>
        {/if}
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          data-command="app.gettingStarted"
          data-placement="today.screen"
          onclick={() => void app.run('app.gettingStarted')}
        >
          How SAM works
        </button>
      </div>
    </section>
  {:else}
    <Variant surface="today-focus" {...focusProps} />
  {/if}

    <div class="cd-grid-2">
      <div class="cd-stack">
        <Variant
          surface="today-queue"
          {rows}
          groups={counts}
          {sessionCommand}
          {targetMin}
          runningId={app.timer?.item.id ?? null}
          runningSince={app.timer?.startedAt ?? null}
          onStart={(item) => void app.startSession(item, targetMin)}
          onStop={() => app.stopSession()}
          onLog={(item) => void app.logTime(item, targetMin)}
          onAdd={(item) => void app.addToToday(item.id)}
          onRemove={(item) => void app.removeFromToday(item.id)}
          onAddToToday={() => void app.openSheet({ kind: 'today.add' })}
        />
        <Variant
          surface="week-chart"
          title="This week"
          caption={`${durationText(weekTotal)} across the last seven days`}
          {days}
          targetMin={totals?.targetMin ?? null}
        />
      </div>
      <div class="cd-stack">
        <Variant
          surface="coming-up"
          rows={comingUpRows}
          planReady={Boolean(planView)}
          onOpenPlan={() => void openPlan()}
          onOpen={openDated}
        />
      </div>
    </div>
  </div>
{/if}
