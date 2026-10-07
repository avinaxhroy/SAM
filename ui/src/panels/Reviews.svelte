<!-- Reviews panel (S5, UI P2 · U3, F13): review queues and recall sessions. -->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import IdPair from '../shell/IdPair.svelte';
  import Menu from '../shell/Menu.svelte';
  import Variant from '../variants/Variant.svelte';
  import type { RecallProps, RecallRow } from '../variants/reviews-recall/props';
  import type { QueueGroup, QueueMoved, QueueProps, QueueRow } from '../variants/reviews-queue/props';
  import { app } from '../session.svelte';
  import { durationText, nameOf } from '../types';
  import { fieldOwnsUndo, type MenuRow } from '../commands/registry';
  import { DEFAULT_GRADES, GRADES, methodWords, STAGE_WORDS, stepWords, type Ask } from '../words';
  import { minutesOf, readPlanModel, type PlanModel } from './model';
  import '../styles/reviews.css';

  let { type, title }: { type: string; title: string } = $props();

  /** The stage words and the row's sentence come from `words.ts`: a pipeline's
      ids are the plan's, and their words are the app's, printed once. */

  type Review = {
    due?: string | null;
    last?: string | null;
    intervalDays?: number | null;
    log?: unknown[];
  };

  type Row = {
    id: string;
    type: string;
    title: string;
    context: { key: string; id: string; label: string } | null;
    pipeline: string;
    stages: string[];
    next: string | null;
    asks: Ask[];
    complete: boolean;
    review: Review | null;
    due: string | null;
    overdueDays: number;
  };

  type Queue = {
    today: string;
    timezoneMinutes: number;
    timezone: string;
    scheduler: string;
    counts: Record<string, number>;
    due: Row[];
    upcoming: Row[];
    waiting: Row[];
    pipelines: Record<string, { stages: string[]; completeWhen: string | null; scheduler: string | null }>;
    pipelineOf: Record<string, string>;
  };

  /** The student's 2-or-4 grade preference, per machine (`sam.reviews.grades`). */
  const GRADES_KEY = 'sam.reviews.grades';

  /** Maximum waiting rows displayed before paging. */
  const WAITING_PAGE = 6;

  /** Backlog age classification windows keyed by overdueDays. */
  const WINDOWS: Array<{ id: string; label: string; rule: string; within: (late: number) => boolean }> = [
    { id: 'today', label: 'Due today', rule: 'not late yet', within: (late) => late === 0 },
    { id: 'soon', label: '1–4 days late', rule: 'one to four days past its date', within: (late) => late >= 1 && late <= 4 },
    { id: 'week', label: '5–7 days late', rule: 'nearly a week behind', within: (late) => late >= 5 && late <= 7 },
    {id: 'late', label: '8 days or more', rule: 'a week or more behind', within: (late) => late >= 8 },
  ];

  const DEFER_DAYS = [1, 2, 3, 7];

  const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  let queue = $state<Queue | null>(null);
  let failure = $state<string | null>(null);
  let loadedFor: string | null = null;
  /** The plan's own records, for the one duration a design prints (B's evidence
   *  pane). The read is cached per revision, so a screen that already opened
   *  costs nothing here. */
  let model = $state<PlanModel | null>(null);

  let revealed = $state(false);
  let revealedFor: string | null = null;
  let earned = $state<Record<string, { due: string | null; intervalDays: number | null; distance: string | null; stage: string | null }>>({});
  /** True while dry-run rating queries are in flight. */
  let asking = $state(false);
  let token = 0;

  /** Student notes entered during the sitting, keyed by record ID. */
  let answers = $state<Record<string, string>>({});

  /** Grade toast message and dismiss timer. */
  let trail = $state<string | null>(null);
  let trailTimer: number | null = null;

  let deferred = $state<QueueMoved[]>([]);
  let graded = $state<QueueMoved[]>([]);

  let showAll = $state(storedShowAll());
  let pinned = $state<string | null>(null);
  let openStep = $state<string | null>(null);
  let menuOpen = $state(false);
  let allWaiting = $state(false);

  /** Session metrics for the completion summary ledger. */
  let session = $state<{
    dueIds: string[];
    graded: string[];
    soon: number;
    held: number;
  } | null>(null);

  let problems = $state<Record<string, number | undefined>>({});
  let signals = $state<Record<string, string>>({});
  let reasons = $state<Record<string, string>>({});

  function storedShowAll(): boolean {
    try {
      return localStorage.getItem(GRADES_KEY) === 'four';
    } catch {
      return false;
    }
  }

  async function refresh(): Promise<void> {
    const result = await app.run('reviews.due', {}, { tracked: false });
    if (result) {
      queue = result.data as Queue;
      failure = null;
    } else {
      failure = app.lastDiagnostic ?? 'the plan did not answer';
    }
  }

  // One read per accepted revision — the publication every write lands on, and
  // never a read this screen's own reads can retrigger.
  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void refresh();
    void readPlanModel().then((read) => {
      model = read;
    });
  });

  // A new card is a new question: the last one's reveal and its consequences do
  // not belong to it. (Reads `current` only — writing those while reading them
  // would re-run forever.)
  $effect(() => {
    const id = current?.id ?? null;
    if (id === revealedFor) return;
    revealedFor = id;
    revealed = false;
    earned = {};
    asking = false;
    token += 1;
  });

  const due = $derived(queue?.due ?? []);
  const waiting = $derived(queue?.waiting ?? []);
  const upcoming = $derived(queue?.upcoming ?? []);
  const shown = $derived(showAll ? GRADES : DEFAULT_GRADES);
  const gradedIds = $derived(session?.graded ?? []);

  /** Active card: pinned record if set, otherwise first unreviewed due record. */
  const current = $derived(
    (pinned ? [...due, ...waiting].find((row) => row.id === pinned) : undefined) ??
      due.find((row) => !gradedIds.includes(row.id)) ??
      due[0] ??
      null,
  );

  /**
   * Sitting progress metrics. Uses `session.dueIds` length rather than live `due`
   * length to maintain a stable denominator during rescheduling.
   */
  const batch = $derived.by(() => {
    const of = session ? session.dueIds.length : due.length;
    const done = session ? session.graded.length : 0;
    return { done, of, left: Math.max(0, of - done - (current ? 1 : 0)) };
  });
  /** True if the active card is part of the initial due queue. */
  const inQueue = $derived(current ? due.some((row) => row.id === current.id) : false);
  /** 1-based sitting position for the active card in the current batch. */
  const position = $derived(
    current && inQueue && batch.of > 0 ? { at: batch.done, of: batch.of, left: batch.left } : null,
  );

  const lateRows = $derived(due.filter((row) => row.overdueDays > 0));
  const staleRows = $derived(due.filter((row) => daysBack(row.review?.last ?? null) >= 7));
  /** True when all initial due records have been reviewed at least once. */
  const done = $derived(
    Boolean(session && session.graded.length > 0 && session.dueIds.every((id) => session.graded.includes(id))),
  );
  const stillLate = $derived(
    session ? due.filter((row) => session.dueIds.includes(row.id) && row.overdueDays > 0).length : 0,
  );
  /** Estimated sitting duration (~30s per card). */
  const minutes = $derived(due.length > 0 ? Math.max(1, Math.round(due.length / 2)) : 0);
  const nextBack = $derived(upcoming[0]?.due ?? null);

  const waitingShown = $derived(allWaiting ? waiting : waiting.slice(0, WAITING_PAGE));
  /** Common ladder step across waiting rows when homogeneous; null if steps diverge. */
  const sharedStep = $derived.by(() => {
    const words = new Set(waitingShown.map(stepWords));
    return waitingShown.length > 1 && words.size === 1 ? [...words][0] : null;
  });
  /** Oldest waiting record available for early practice when the queue is clear. */
  const recallable = $derived(waiting[0] ?? null);

  const currentPipeline = $derived(queue?.pipelineOf?.[type] ?? null);
  const methodNames = $derived(Object.keys(queue?.pipelines ?? {}));
  const todayView = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'today')?.[0] ?? null,
  );

  /** The plan's own civil date for an instant (the queue states its timezone). */
  function planDay(iso: string | null | undefined): string | null {
    if (!iso) return null;
    const at = Date.parse(iso);
    if (Number.isNaN(at)) return null;
    return new Date(at + (queue?.timezoneMinutes ?? 0) * 60_000).toISOString().slice(0, 10);
  }

  function daysBetween(from: string, to: string): number | null {
    const a = Date.parse(`${from}T00:00:00Z`);
    const b = Date.parse(`${to}T00:00:00Z`);
    if (Number.isNaN(a) || Number.isNaN(b)) return null;
    return Math.round((b - a) / 86_400_000);
  }

  /** Calendar days from plan today to target date. Positive is future, negative past. */
  function daysFromToday(day: string): number | null {
    const today = queue?.today;
    if (!today) return null;
    return daysBetween(today, day);
  }

  function daysBack(iso: string | null | undefined): number {
    const day = planDay(iso);
    if (!day) return 0;
    const days = daysFromToday(day);
    return days === null ? 0 : Math.max(0, -days);
  }

  /** Add whole days to civil date string (YYYY-MM-DD). */
  function addDays(day: string, count: number): string {
    const at = Date.parse(`${day}T00:00:00Z`);
    if (Number.isNaN(at)) return day;
    return new Date(at + count * 86_400_000).toISOString().slice(0, 10);
  }

  /** Format date as human-readable short string (e.g. 'Fri 2 Oct'). */
  function dayText(iso: string | null | undefined): string {
    const day = planDay(iso) ?? (iso ? iso.slice(0, 10) : null);
    if (!day) return 'no day yet';
    const [year, month, date] = day.split('-').map(Number);
    if (!year || !month || !date) return 'no day yet';
    const at = new Date(Date.UTC(year, month - 1, date));
    return `${WEEKDAYS[at.getUTCDay()]} ${date} ${MONTHS[month - 1]}`;
  }

  /** Relative time phrase from plan today (R7). */
  function distance(due: string | null | undefined, style: 'back' | 'ago' | 'coming'): string | null {
    const day = planDay(due) ?? due?.slice(0, 10) ?? null;
    if (!day) return null;
    const days = daysFromToday(day);
    if (days === null) return null;
    if (style === 'ago') {
      if (days > 0) return null;
      const past = -days;
      if (past === 0) return 'today';
      if (past === 1) return 'yesterday';
      return `${past} days ago`;
    }
    if (style === 'coming') {
      if (days <= 0) return 'today';
      if (days === 1) return 'tomorrow';
      return `in ${days} days`;
    }
    if (days < 0) return 'still overdue';
    if (days === 0) return 'back later today';
    if (days === 1) return 'back tomorrow';
    return `back in ${days} days`;
  }

  function latePhrase(days: number): string {
    return days === 1 ? '1 day late' : `${days} days late`;
  }

  function lastReviewed(row: Row): string {
    const ago = distance(row.review?.last ?? null, 'ago');
    return ago ? `last reviewed ${ago}` : 'never reviewed';
  }

  function recalls(row: Row): number {
    return row.review?.log?.length ?? 0;
  }

  /** 1-based index of stage in pipeline ladder. */
  function rungOf(row: Row, stage: string | null): number {
    const ladder = queue?.pipelines?.[row.pipeline]?.stages ?? [];
    if (stage && ladder.indexOf(stage) >= 0) return ladder.indexOf(stage) + 1;
    return Math.max(1, row.stages.length);
  }

  /** Collection urgency indicator (`collection.css` §4). */
  function markOf(row: Row): 'overdue' | 'soon' {
    return row.overdueDays > 0 ? 'overdue' : 'soon';
  }

  /** Display code or short label for course chip (D2). */
  function codeOf(context: { id: string; label: string } | null): string | null {
    if (!context) return null;
    const record = model?.byId.get(context.id);
    const value = record?.fields.code ?? record?.fields.short;
    return typeof value === 'string' && value.length > 0 ? value : context.label;
  }

  /** The ladder of a named pipeline, in the plan's own words. */
  function ladderWords(name: string | null): string | null {
    return methodWords(name ? (queue?.pipelines?.[name]?.stages ?? null) : null);
  }

  /** Primary label for method pill control, taken from initial pipeline stage. */
  function methodName(name: string | null): string {
    const stages = queue?.pipelines?.[name ?? '']?.stages ?? [];
    const first = STAGE_WORDS[stages[0] ?? ''];
    if (!first) return nameOf(name ?? '');
    return first.charAt(0).toUpperCase() + first.slice(1);
  }

  /** Control layout: segmented control for <=4 methods, dropdown menu for >4 (`controls.md` §2). */
  const methodControl = $derived<'seg' | 'menu'>(methodNames.length > 4 ? 'menu' : 'seg');

  const methodRows = $derived<MenuRow[]>(
    methodNames
      .filter((name) => name !== currentPipeline)
      .map(
        (name): MenuRow => ({
          id: 'type.setPipeline',
          title: ladderWords(name) ?? `a ${(queue?.pipelines?.[name]?.stages ?? []).length}-step method`,
          run: () => void app.run('type.setPipeline', { type, pipeline: name, fresh: true }),
        }),
      ),
  );

  function setPipeline(name: string): void {
    void app.run('type.setPipeline', { type, pipeline: name, fresh: true });
  }

  /** Pin record to active recall card. */
  function pin(id: string): void {
    pinned = id;
    allWaiting = false;
  }

  /** Dry-run ratings to preview scheduling consequences before selection. */
  async function loadConsequences(row: Row): Promise<void> {
    const mine = (token += 1);
    const found: typeof earned = {};
    asking = true;
    for (const grade of shown) {
      const result = await app.run(
        'record.logReview',
        { id: row.id, rating: grade.rating, 'dry-run': true },
        { tracked: false },
      );
      if (mine !== token) return;
      const data = result?.data as { due?: string | null; intervalDays?: number | null; stage?: string | null } | undefined;
      // On dry-run failure, omit preview consequence.
      if (!result || !data || !('due' in data)) continue;
      found[grade.rating] = {
        due: data.due ?? null,
        intervalDays: data.intervalDays ?? null,
        distance: distance(data.due ?? null, 'back') ?? 'no further recalls',
        stage: data.stage ?? null,
      };
    }
    if (mine !== token) return;
    earned = found;
    asking = false;
  }

  async function reveal(): Promise<void> {
    const row = current;
    if (!row || revealed) return;
    revealed = true;
    await loadConsequences(row);
  }

  function hide(): void {
    revealed = false;
    earned = {};
    asking = false;
    token += 1;
  }

  function setGrades(all: boolean): void {
    showAll = all;
    try {
      localStorage.setItem(GRADES_KEY, all ? 'four' : 'two');
    } catch {
      /* Ignore localStorage access restrictions in private mode */
    }
    const row = current;
    if (row && revealed) void loadConsequences(row);
  }

  /** Display 5-second transient toast after grading action. */
  function leaveTrail(label: string): void {
    trail = label;
    if (trailTimer !== null) window.clearTimeout(trailTimer);
    trailTimer = window.setTimeout(() => {
      trail = null;
      trailTimer = null;
    }, 5000);
  }

  async function grade(rating: string): Promise<void> {
    const row = current;
    const q = queue;
    if (!row || !q || !revealed) return;
    const fromQueue = q.due.some((candidate) => candidate.id === row.id);
    if (!session && fromQueue) {
      session = { dueIds: q.due.map((candidate) => candidate.id), graded: [], soon: 0, held: 0 };
    }
    const result = await app.run('record.logReview', { id: row.id, rating });
    if (!result) return;
    const answer = result.data as { due?: string | null } | undefined;
    const said = distance(answer?.due ?? null, 'back');
    const word = GRADES.find((grade_) => grade_.rating === rating)?.label ?? rating;
    graded = [...graded.filter((entry) => entry.id !== row.id), { id: row.id, title: row.title, say: said ? `${word} · ${said}` : word }];
    if (session && fromQueue && !session.graded.includes(row.id)) {
      session = {
        ...session,
        graded: [...session.graded, row.id],
        soon: session.soon + (rating === 'again' ? 1 : 0),
        held: session.held + (rating === 'good' || rating === 'easy' ? 1 : 0),
      };
    }
    if (pinned === row.id) pinned = null;
    leaveTrail(`Graded ${word}`);
    await refresh();
    loadedFor = app.revision;
  }

  async function undo(): Promise<void> {
    await app.undo();
    trail = null;
    if (trailTimer !== null) {
      window.clearTimeout(trailTimer);
      trailTimer = null;
    }
    deferred = deferred.slice(0, Math.max(0, deferred.length - 1));
    graded = graded.slice(0, Math.max(0, graded.length - 1));
    await refresh();
  }

  async function deferTo(id: string, offset: number): Promise<void> {
    const q = queue;
    if (!q) return;
    const row = q.due.find((candidate) => candidate.id === id);
    const day = addDays(q.today, offset);
    const result = await app.run('record.defer', { ids: [id], date: day, reviews: true });
    if (!result) return;
    if (row) deferred = [...deferred, { id, title: row.title, say: `due ${dayText(day)}` }];
    if (pinned === id) pinned = null;
    await refresh();
    loadedFor = app.revision;
  }

  function toggleStep(row: Row): void {
    if (openStep === row.id) {
      openStep = null;
      return;
    }
    const proof = row.asks.find((ask) => ask.key === 'problems');
    if (proof && problems[row.id] === undefined) problems[row.id] = proof.min ?? 2;
    openStep = row.id;
  }

  function solved(row: Row): number | null {
    const ask = row.asks.find((candidate) => candidate.key === 'problems');
    if (!ask) return null;
    const value = problems[row.id];
    if (typeof value !== 'number' || !Number.isFinite(value)) return null;
    return value;
  }

  function enoughSolved(row: Row): boolean {
    const ask = row.asks.find((candidate) => candidate.key === 'problems');
    if (!ask) return true;
    const value = solved(row);
    return value !== null && value >= (ask.min ?? 1);
  }

  /** Advance record to next pipeline stage with supplied evidence payload. */
  async function advance(row: Row, evidence: Record<string, unknown>): Promise<void> {
    if (!row.next) return;
    const result = await app.run('record.advanceStage', { id: row.id, stage: row.next, ...evidence });
    if (!result) return;
    openStep = null;
    if (pinned === row.id) pinned = null;
  }

  async function toToday(): Promise<void> {
    if (todayView) await app.run('rail.select', { view: todayView });
  }

  function keepGoing(): void {
    session = null;
    pinned = null;
  }

  /** Pin oldest unscheduled record for manual review when due queue is empty. */
  function recallOneNow(): void {
    const row = recallable;
    if (row) pin(row.id);
  }

  // ── variant contracts ──────────────────────────────────────────────────
  /** Backlog queue props contract across variant designs. */
  const queueRows = $derived<QueueRow[]>(
    due.map((row) => ({
      id: row.id,
      title: row.title,
      course: codeOf(row.context),
      wash: row.context ? app.washes[row.context.id] : undefined,
      late: row.overdueDays,
      state: row.overdueDays > 0 ? latePhrase(row.overdueDays) : 'due today',
      tone: row.overdueDays >= 5 ? 'overdue' : row.overdueDays >= 1 ? 'risk' : 'info',
      due: dayText(row.due),
    })),
  );

  const queueGroups = $derived<QueueGroup[]>(
    WINDOWS.map((window) => ({
      id: window.id,
      label: window.label,
      rule: window.rule,
      rows: queueRows.filter((row) => window.within(row.late)),
    })),
  );

  const backlog = $derived<QueueProps>({
    groups: queueGroups,
    rows: queueRows,
    total: queueRows.length,
    oldestLate: queueRows.reduce((most, row) => Math.max(most, row.late), 0),
    dueToday: queueRows.filter((row) => row.late === 0).length,
    weekOld: queueRows.filter((row) => row.late >= 8).length,
    moved: { deferred, graded },
    days: (queue ? DEFER_DAYS.map((offset) => ({ days: offset, label: dayText(addDays(queue.today, offset)) })) : []),
    onPut: pin,
    onDefer: deferTo,
    onUndo: undo,
  });

  /** Windowed queue of up to 5 forthcoming review records for sitting view. */
  const sitting = $derived.by<RecallRow[]>(() => {
    if (!current) return [];
    const from = Math.max(0, due.findIndex((row) => row.id === current.id));
    return due.slice(from, from + 5).map((row, index) => ({
      id: row.id,
      numeral: String(from + index + 1).padStart(2, '0'),
      title: row.title,
      state: row.overdueDays > 0 ? latePhrase(row.overdueDays) : null,
      late: row.overdueDays,
    }));
  });
  const moreWaiting = $derived.by(() => {
    if (!current) return null;
    const from = Math.max(0, due.findIndex((row) => row.id === current.id));
    const left = due.length - (from + 5);
    return left > 0 ? `${left} more waiting` : 'nothing more waiting';
  });

  /**
   * Unified consequence label when all shown grades yield identical scheduling
   * (e.g. fixed interval schedules, `scheduler.rs:305`). Returns null if grades diverge.
   */
  const returned = $derived.by(() => {
    if (asking || shown.length < 2) return null;
    const first = earned[shown[0].rating];
    if (!first) return null;
    const same = shown.every(
      (grade_) => earned[grade_.rating]?.due === first.due && earned[grade_.rating]?.intervalDays === first.intervalDays,
    );
    if (!same) return null;
    const interval =
      first.intervalDays === null ? null : `${first.intervalDays} ${first.intervalDays === 1 ? 'day' : 'days'}`;
    // Lateness relative to computed consequence target date.
    const back = first.due ? daysFromToday(planDay(first.due) ?? first.due.slice(0, 10)) : null;
    const when = back === null || back === 0 ? 'due today' : back < 0 ? latePhrase(-back) : `in ${back} days`;
    return { day: dayText(first.due), say: interval ? `${interval} · ${when}` : when };
  });

  /** Grade action options with target rungs and individual consequences when divergent. */
  const grades = $derived(
    shown.map((grade_, index) => {
      const fact = earned[grade_.rating];
      return {
        rating: grade_.rating,
        key: String(index + 1),
        label: grade_.label,
        rung: current ? rungOf(current, fact?.stage ?? null) : 1,
        rungs: current ? (queue?.pipelines?.[current.pipeline]?.stages.length ?? 0) : 0,
        consequence: returned ? null : (fact?.distance ?? null),
      };
    }),
  );

  const recall = $derived<RecallProps | null>(
    current
      ? {
          card: {
            id: current.id,
            title: current.title,
            course: codeOf(current.context),
            wash: current.context ? app.washes[current.context.id] : undefined,
            state:
              current.overdueDays > 0
                ? latePhrase(current.overdueDays)
                : current.due
                  ? 'due today'
                  : 'ready for one recall',
            tone: current.overdueDays > 0 ? 'overdue' : current.due ? 'info' : 'quiet',
            since: (() => {
              const ago = distance(current.review?.last ?? null, 'ago');
              return ago ? `${ago} since the last recall` : null;
            })(),
            recalls: recalls(current),
            minutes: (() => {
              const record = model?.byId.get(current.id);
              return record ? minutesOf(record) : null;
            })(),
          },
          position,
          sitting,
          more: moreWaiting,
          grades,
          returned,
          revealed,
          answer: answers[current.id] ?? '',
          asking,
          trail,
          onReveal: () => void reveal(),
          onHide: hide,
          onAnswer: (text: string) => {
            const id = current?.id;
            if (!id) return;
            if (text) answers[id] = text;
            else delete answers[id];
          },
          onGrade: grade,
          onSelect: pin,
          onUndo: undo,
        }
      : null,
  );

  /**
   * Keyboard shortcuts for review loop (F4): Space to reveal, 1-4 to grade.
   * Suppressed when typing in form fields or when modals/sheets are open.
   */
  function onkeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.repeat) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (fieldOwnsUndo(event.target)) return;
    if (app.sheet || app.paletteOpen || app.settingsOpen || app.detail || menuOpen) return;

    if (event.key === ' ' || event.code === 'Space') {
      if (current && !revealed) {
        event.preventDefault();
        void reveal();
      }
      return;
    }
    if (/^[1-4]$/.test(event.key)) {
      const grade_ = shown[Number(event.key) - 1];
      if (grade_ && revealed) {
        event.preventDefault();
        void grade(grade_.rating);
      }
      return;
    }
    if (event.key === 'n' || event.key === 'N') {
      event.preventDefault();
      setGrades(!showAll);
    }
  }
</script>

<svelte:window onkeydown={onkeydown} />

<div data-panel="reviews" data-type={type}>
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <!-- Queue size context shown when due records exist. -->
      {#if !queue}
        <p class="cd-pagehead__sub">Reading the queue…</p>
      {:else if due.length > 0}
        <p class="cd-pagehead__sub">
          {due.length} waiting · about {minutes} {minutes === 1 ? 'minute' : 'minutes'}
          {#if staleRows.length > 0}
            · {staleRows.length} {staleRows.length === 1 ? 'is' : 'are'} over a week old
          {/if}
        </p>
      {/if}
    </div>

    <!-- Study method pipeline selector (D2). -->
    <span class="cd-pagehead__aside">
      <span class="rv__aside">
        {#if currentPipeline && methodNames.length > 0}
          <IdPair label={ladderWords(currentPipeline) ?? 'How this work moves'} value={currentPipeline} />
          {#if methodControl === 'seg'}
            <span class="cd-seg rv__method" role="group" aria-label="How this kind of work moves, in order">
              {#each methodNames as name (name)}
                <button
                  class="cd-seg__pill"
                  type="button"
                  aria-pressed={name === currentPipeline}
                  aria-label={`${ladderWords(name) ?? name} — switch the study method`}
                  data-command="type.setPipeline"
                  data-placement="reviews.panel"
                  title={ladderWords(name) ?? name}
                  onclick={() => setPipeline(name)}
                >
                  {methodName(name)}
                </button>
              {/each}
            </span>
          {:else}
            <span class="rv__method">
              <button
                class="cd-pill cd-pill--quiet cd-pill--sm"
                type="button"
                aria-haspopup="menu"
                aria-expanded={menuOpen}
                disabled={!type || methodNames.length < 2}
                data-command="type.setPipeline"
                data-placement="reviews.panel"
                title="How this kind of work moves, in order"
                onclick={() => (menuOpen = !menuOpen)}
              >
                {ladderWords(currentPipeline) ?? 'The plan’s own method'} ▾
              </button>
              {#if menuOpen}
                <Menu rows={methodRows} native={false} onclose={() => (menuOpen = false)} />
              {/if}
            </span>
          {/if}
        {/if}
      </span>    </span>
  </header>

  {#if !queue && failure}
    <!-- Error retry state (R19, D8). -->
    <section class="cd-card" data-part="error">
      <div class="cd-empty cd-empty--tight">
        <span class="cd-empty__art"><Icon name="refresh" size={24} /></span>
        <div class="cd-empty__t">The queue could not be read</div>
        <div class="cd-empty__s">{failure}</div>
        <button
          class="cd-pill cd-pill--ghost cd-pill--sm"
          type="button"
          data-command="reviews.due"
          data-placement="reviews.panel"
          onclick={() => void refresh()}
        >
          <Icon name="refresh" size={13} />
          Try again
        </button>
      </div>
    </section>
  {:else if !queue}
    <!-- Skeleton placeholder matching card geometry (R19). -->
    <div class="cd-stack" aria-busy="true">
      <p class="cd-sr" role="status">Reading the queue…</p>
      <div class="cd-skel cd-skel--card"></div>
      <section class="cd-card">
        <div class="cd-skel__rows">
          {#each [0, 1, 2] as row (row)}
            <div class="cd-skel__row cd-skel__row--coll">
              <span class="cd-skel"></span>
              <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 58 : 41}%`}></span>
              <span class="cd-skel"></span>
            </div>
          {/each}
        </div>
      </section>
    </div>
  {:else}
    <div class="cd-stack">
      {#if done}
        <!-- Session completion summary (F4). -->
        <section class="cd-complete" data-part="ledger">
          <div class="cd-complete__spark">
            <span class="cd-sparkle"><Icon name="check" size={24} /></span>
            <span class="cd-sparkle"><Icon name="check" size={30} /></span>
            <span class="cd-sparkle"><Icon name="check" size={24} /></span>
          </div>
          <div class="cd-complete__t">The queue you came for is clear</div>
          <div class="cd-complete__s">Here is what just happened, counted.</div>
          <div class="cd-complete__ledger">
            <div class="cd-complete__cell">
              <div class="cd-complete__v">{session?.graded.length ?? 0}</div>
              <div class="cd-complete__k">recalls answered</div>
            </div>
            <div class="cd-complete__cell">
              <div class="cd-complete__v">{session?.soon ?? 0}</div>
              <div class="cd-complete__k">come back tomorrow</div>
            </div>
            <div class="cd-complete__cell">
              <div class="cd-complete__v">{session?.held ?? 0}</div>
              <div class="cd-complete__k">held longer</div>
            </div>
            <div class="cd-complete__cell">
              <div class="cd-complete__v">{stillLate}</div>
              <div class="cd-complete__k">still late</div>
            </div>
          </div>
          <p class="rv__acts">
            {#if due.length > 0}
              <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={keepGoing}>Keep going</button>
            {/if}
            <button
              class="cd-pill cd-pill--lg"
              type="button"
              data-command="rail.select"
              data-placement="reviews.panel"
              disabled={!todayView}
              onclick={() => void toToday()}
            >
              Back to today
            </button>
          </p>
        </section>
      {:else}
        {#if lateRows.length > 0}
          <!-- Overdue catch-up banner (F13): prompt to start oldest or batch defer. -->
          <section class="cd-dashed" data-part="overdue">
            <b>
              {lateRows.length} {lateRows.length === 1 ? 'recall is' : 'recalls are'} waiting
              {lateRows[0].overdueDays === 1 ? 'from yesterday' : `from ${lateRows[0].overdueDays} days ago`}
            </b>
            <span>They come back oldest first, one at a time.</span>
            <span class="rv__row">
              <button
                class="cd-pill cd-pill--ghost cd-pill--sm"
                type="button"
                data-command="reviews.due"
                data-placement="reviews.panel"
                onclick={() => pin(lateRows[0].id)}
              >
                Start with the oldest
              </button>
              <button
                class="cd-pill cd-pill--ghost cd-pill--sm"
                type="button"
                data-command="record.defer"
                data-placement="reviews.panel"
                onclick={() =>
                  app.openSheet({
                    kind: 'catchup',
                    items: lateRows.map((row) => ({ id: row.id, label: row.title })),
                    label: `${lateRows.length} overdue ${lateRows.length === 1 ? 'recall' : 'recalls'}`,
                  })}
              >
                Move the backlog…
              </button>
            </span>
          </section>
        {/if}

        {#if recall}
          <!-- Review loop card surface. -->
          <Variant surface="reviews-recall" {...recall} />
        {:else if !queue.scheduler}
          <!-- Empty state when no schedule is defined. -->
          <section class="cd-card" data-part="empty">
            <div class="cd-empty cd-empty--tight">
              <span class="cd-empty__art"><Icon name="check" size={24} /></span>
              <div class="cd-empty__t">This plan declares no review schedule</div>
              <div class="cd-empty__s">
                Its rules do not say when a recall comes back, so nothing here will ever come due.
              </div>
              <button
                class="cd-pill cd-pill--ghost cd-pill--sm"
                type="button"
                data-command="rail.select"
                data-placement="reviews.panel"
                disabled={!todayView}
                onclick={() => void toToday()}
              >
                Back to today
              </button>
            </div>
          </section>
        {:else}
          <section class="cd-card" data-part="empty">
            <div class="cd-empty cd-empty--tight">
              <span class="cd-empty__art"><Icon name="check" size={24} /></span>
              <div class="cd-empty__t">Nothing is due</div>
              <div class="cd-empty__s">
                {#if nextBack}
                  The next one comes back {distance(nextBack, 'coming')}.
                {:else if recallable}
                  Nothing is on a review date yet — {recallable.title} is ready for one.
                {:else}
                  Nothing is on a review date yet, and nothing is ready to lock in.
                {/if}
              </div>
              {#if recallable}
                <button
                  class="cd-pill cd-pill--ghost cd-pill--sm"
                  type="button"
                  data-command="record.logReview"
                  data-placement="reviews.panel"
                  onclick={recallOneNow}
                >
                  Recall {recallable.title}
                </button>
              {:else}
                <button
                  class="cd-pill cd-pill--ghost cd-pill--sm"
                  type="button"
                  data-command="rail.select"
                  data-placement="reviews.panel"
                  disabled={!todayView}
                  onclick={() => void toToday()}
                >
                  Back to today
                </button>
              {/if}
            </div>
          </section>
        {/if}
      {/if}

      {#if due.length > 0 && !done}
        <!-- Backlog queue grouped by lateness windows. -->
        <Variant surface="reviews-queue" {...backlog} />
      {/if}

      {#if waiting.length > 0}
        <!-- Records awaiting next pipeline step (`record.advanceStage`). -->
        <section class="cd-card" data-part="next-steps">
          <header class="cd-card__head">
            <span class="cd-ictile"><Icon name="route" /></span>
            <div>
              <h2 class="cd-card__title">Ready for the next step</h2>
              <p class="cd-card__sub">
                {#if sharedStep}
                  {waiting.length} {waiting.length === 1 ? 'record is' : 'records are'} waiting on a step the plan
                  asks for — {sharedStep.charAt(0).toLowerCase() + sharedStep.slice(1)}
                {:else}
                  {waiting.length} {waiting.length === 1 ? 'record is' : 'records are'} waiting on a step the plan asks
                  for
                {/if}
              </p>
            </div>
            <span class="cd-card__spacer"></span>
            {#if waiting.length > WAITING_PAGE}
              <button
                class="cd-pill cd-pill--quiet cd-pill--sm"
                type="button"
                aria-expanded={allWaiting}
                onclick={() => (allWaiting = !allWaiting)}
              >
                {allWaiting ? 'Show fewer' : `Show all ${waiting.length}`}
              </button>
            {/if}
          </header>
          <div class="cd-coll">
            {#each waitingShown as row (row.id)}
              {@const proof = row.asks.find((ask) => ask.key === 'problems')}
              {@const reason = row.asks.find((ask) => ask.key === 'reason')}
              {@const signal = row.asks.find((ask) => ask.key === 'signal')}
              <div class="cd-coll__row" data-record-id={row.id} data-part="waiting">
                <span class="cd-urgency" data-lvl="next" aria-hidden="true"></span>
                <span class="cd-coll__body">
                  <span class="cd-coll__title">{row.title}</span>
                  <span class="cd-rowmeta">
                    {#if row.context}
                      <span class="cd-chip cd-chip--wash" data-w={app.washes[row.context.id]}>
                        {row.context.label}
                      </span>
                    {/if}
                    {#if !sharedStep}<span>{stepWords(row)}</span>{/if}
                  </span>
                </span>
                <span class="cd-coll__right">
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm"
                    type="button"
                    aria-expanded={openStep === row.id}
                    onclick={() => toggleStep(row)}
                  >
                    Record the step
                    <span class="cd-sr"> for {row.title}</span>
                  </button>
                </span>

                {#if openStep === row.id}
                  <div class="cd-detail rv__step" data-part="step">
                    <div class="cd-detail__grid">
                      {#if proof}
                        <label class="rv__field">
                          <span class="cd-detail__k">{proof.label}</span>
                          <input
                            class="rv__input"
                            type="number"
                            min={proof.min ?? 0}
                            bind:value={problems[row.id]}
                            aria-label={`${proof.label} for ${row.title}`}
                          />
                        </label>
                      {/if}
                      {#if reason}
                        <label class="rv__field">
                          <span class="cd-detail__k">Reason</span>
                          <input
                            class="rv__input rv__input--wide"
                            type="text"
                            bind:value={reasons[row.id]}
                            aria-label={`Why the step is skipped, for ${row.title}`}
                          />
                        </label>
                      {/if}
                      {#if signal}
                        <label class="rv__field">
                          <span class="cd-detail__k">{signal.label}</span>
                          <input
                            class="rv__input rv__input--wide"
                            type="text"
                            bind:value={signals[row.id]}
                            aria-label={`${signal.label} for ${row.title}`}
                          />
                        </label>
                      {/if}
                    </div>
                    <p class="cd-detail__note">
                      {#if reason}
                        A recall locks it in. A reason records the skip instead — the plan accepts either.
                      {:else if proof}
                        Only what you actually solved is recorded; a prompt is not evidence.
                      {:else}
                        Recording the step is the only thing this one asks for.
                      {/if}
                    </p>
                    <div class="cd-detail__acts">
                      {#if proof}
                        <button
                          class="cd-pill cd-pill--sm"
                          type="button"
                          data-command="record.advanceStage"
                          data-placement="reviews.panel"
                          disabled={!enoughSolved(row)}
                          onclick={() => void advance(row, { problems: solved(row) })}
                        >
                          Record {solved(row) ?? proof.min ?? 2} solved
                        </button>
                      {:else if !reason}
                        <button
                          class="cd-pill cd-pill--sm"
                          type="button"
                          data-command="record.advanceStage"
                          data-placement="reviews.panel"
                          onclick={() =>
                            void advance(row, signals[row.id] ? { signal: signals[row.id] } : {})}
                        >
                          Record the step
                        </button>
                      {/if}
                      {#if reason}
                        <button
                          class="cd-pill cd-pill--sm"
                          type="button"
                          data-command="record.advanceStage"
                          data-placement="reviews.panel"
                          disabled={!reasons[row.id]}
                          onclick={() => void advance(row, { reason: reasons[row.id] })}
                        >
                          Skip the step
                        </button>
                        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => pin(row.id)}>
                          Recall it now
                        </button>
                      {/if}
                      {#if row.next}
                        <IdPair label="Stage" value={row.next} />
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>
