<!--
  REVIEWS-QUEUE · THE HOST — the backlog as a widget the composer mounts
  (2026-09-29).

  This is the surface's own derivation moved out of `panels/Reviews.svelte`, not
  re-written: the same `reviews.due` read per accepted revision, the same four
  lateness windows derived from the plan's own `overdueDays`, the same rows
  (`codeOf` off the cached plan model, `app.washes`), the same defer days counted
  from the plan's own today, and the same two writes (`record.defer` for one
  record, `app.undo` for the trail). It takes no props, owns its ephemeral state
  (the receipts `deferred`/`graded`, the local `pinned`), and hands `<Variant
  surface="reviews-queue">` exactly the object the panel used to spread.

  THE ONE COUPLING THIS SURFACE HAD, NOW BRIDGED. In the panel `onPut` = `pin`,
  which set the panel's `pinned` and so put this record on the recall card
  ABOVE — one memory shared because both surfaces live in one component. As
  widgets they cannot see each other's variables, so the pin lives in
  `composer/shared.svelte.ts`, where a recall widget on the same screen reads
  it: put a record on the card from the queue, and the card holds it.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import type { QueueGroup, QueueMoved, QueueProps, QueueRow } from '../../../variants/reviews-queue/props';
  import { app } from '../../../session.svelte';
  import { shared } from '../../shared.svelte';
  import { readPlanModel, type PlanModel } from '../../../panels/model';

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
    asks: Array<{ key: string; label: string; kind: string; required: boolean; min: number | null; stage: string }>;
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

  /** The four lateness windows the backlog is shelved into. The names are the
   *  engine's own bands, and they are derived here — from `overdueDays` — and
   *  never typed into a design. */
  const WINDOWS: Array<{ id: string; label: string; rule: string; within: (late: number) => boolean }> = [
    { id: 'today', label: 'Due today', rule: 'not late yet', within: (late) => late === 0 },
    { id: 'soon', label: '1–4 days late', rule: 'one to four days past its date', within: (late) => late >= 1 && late <= 4 },
    { id: 'week', label: '5–7 days late', rule: 'nearly a week behind', within: (late) => late >= 5 && late <= 7 },
    { id: 'late', label: '8 days or more', rule: 'a week or more behind', within: (late) => late >= 8 },
  ];

  /** The days a defer can offer: the next four things a student could sit
   *  down for, counted from the plan's own today. */
  const DEFER_DAYS = [1, 2, 3, 7];

  const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  let queue = $state<Queue | null>(null);
  let loadedFor: string | null = null;
  /** The plan's own records, for the one fact a design prints (a record's own
   *  course code). The read is cached per revision. */
  let model = $state<PlanModel | null>(null);

  /** The receipts this sitting has moved, keyed by what happened. */
  let deferred = $state<QueueMoved[]>([]);
  let graded = $state<QueueMoved[]>([]);

  async function refresh(): Promise<void> {
    const result = await app.run('reviews.due', {}, { tracked: false });
    if (result) {
      queue = result.data as Queue;
    }
  }

  // One read per accepted revision — the publication every write lands on, and
  // never a read this widget's own reads can retrigger.
  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void refresh();
    void readPlanModel().then((read) => {
      model = read;
    });
  });

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

  /** Whole days from the plan's today to a date: positive is ahead, negative is
   *  past. One direction, so no caller can read a past date as a future one. */
  function daysFromToday(day: string): number | null {
    const today = queue?.today;
    if (!today) return null;
    return daysBetween(today, day);
  }

  /** A civil day, counted forward. */
  function addDays(day: string, count: number): string {
    const at = Date.parse(`${day}T00:00:00Z`);
    if (Number.isNaN(at)) return day;
    return new Date(at + count * 86_400_000).toISOString().slice(0, 10);
  }

  /** A day as a student says it (`Fri 2 Oct`) — the same shape the Mocks screen
   *  prints, and never the plan's raw date. */
  function dayText(iso: string | null | undefined): string {
    const day = planDay(iso) ?? (iso ? iso.slice(0, 10) : null);
    if (!day) return 'no day yet';
    const [year, month, date] = day.split('-').map(Number);
    if (!year || !month || !date) return 'no day yet';
    const at = new Date(Date.UTC(year, month - 1, date));
    return `${WEEKDAYS[at.getUTCDay()]} ${date} ${MONTHS[month - 1]}`;
  }

  function latePhrase(days: number): string {
    return days === 1 ? '1 day late' : `${days} days late`;
  }

  /** A course's own code, which is what a student says out loud (D2) — the
   *  record's `code`, else its label, so a chip never prints an id. */
  function codeOf(context: { id: string; label: string } | null): string | null {
    if (!context) return null;
    const record = model?.byId.get(context.id);
    const value = record?.fields.code ?? record?.fields.short;
    return typeof value === 'string' && value.length > 0 ? value : context.label;
  }

  const due = $derived(queue?.due ?? []);

  /** Put a record on the card — the student's choice of what to work on now.
   *  Verbatim from the panel; with no card above, it holds the choice locally. */
  function pin(id: string): void {
    shared.pinnedReview = id;
  }

  /** Defer one record to a day from the plan's own today — the same write the
   *  catch-up sheet makes for a whole selection, for a single row. */
  async function deferTo(id: string, offset: number): Promise<void> {
    const q = queue;
    if (!q) return;
    const row = q.due.find((candidate) => candidate.id === id);
    const day = addDays(q.today, offset);
    const result = await app.run('record.defer', { ids: [id], date: day, reviews: true });
    if (!result) return;
    if (row) deferred = [...deferred, { id, title: row.title, say: `due ${dayText(day)}` }];
    if (shared.pinnedReview === id) shared.pinnedReview = null;
    await refresh();
    loadedFor = app.revision;
  }

  /** Reverse the last write — the trail's own door. */
  async function undo(): Promise<void> {
    await app.undo();
    deferred = deferred.slice(0, Math.max(0, deferred.length - 1));
    graded = graded.slice(0, Math.max(0, graded.length - 1));
    // The app's own undo is untracked, so the revision never moves: this widget
    // reads its queue again rather than waiting for a publication that is not
    // coming.
    await refresh();
  }

  // ── the surface, as its own contract ──────────────────────────────────

  /** The backlog's records, as every design draws them. */
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
</script>

<Variant surface="reviews-queue" {...backlog} />
