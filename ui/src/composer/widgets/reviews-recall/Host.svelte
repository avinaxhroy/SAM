<!--
  REVIEWS-RECALL · THE HOST — the recall card as a widget the composer mounts
  (2026-09-29).

  This is the surface's own derivation moved out of `panels/Reviews.svelte`, not
  re-written: the same `reviews.due` read per accepted revision, the same plan
  model read for the card's own `minutes`, the same card chosen (a pinned record,
  else the oldest not graded this sitting), the same `session` batch and its
  `position`, the same per-grade dry-runs (`record.logReview` with `dry-run`),
  the same `returned`/`grades` shaping, and the same writes (`record.logReview`
  for a grade, `app.undo` for the trail). It takes no props, owns its ephemeral
  state (the card's reveal, the per-record answers, the earned consequences, the
  trail), and hands `<Variant surface="reviews-recall">` exactly the object the
  panel used to spread.

  THE PIN IS SHARED. The record a queue widget puts on this card lives in
  `composer/shared.svelte.ts`, so the two widgets agree when both are on one
  screen; the card's own row selection writes the same place.

  THE KEYBOARD IS NOT PORTED. The panel's `Space` / `1–4` / `n` handler is a
  window listener — screen-level chrome owned by the screen, not this widget.
  Mounting it per host would let a recall widget on any screen swallow the
  student's keys, so it stays with the panel (reported, not improvised around).

  THE EMPTY CASE. When no record is current (nothing due, nothing waiting),
  the widget renders an empty state showing when the next item is due.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import type { RecallGrade, RecallProps, RecallRow } from '../../../variants/reviews-recall/props';
  import { app } from '../../../session.svelte';
  import { DEFAULT_GRADES, GRADES } from '../../../words';
  import { shared } from '../../shared.svelte';
  import Icon from '../../../shell/Icon.svelte';
  import { minutesOf, readPlanModel, type PlanModel } from '../../../panels/model';
  import '../../../styles/reviews.css';

  type Ask = {
    key: string;
    label: string;
    kind: string;
    required: boolean;
    min: number | null;
    stage: string;
  };

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

  /**
   * The student's 2-or-4 grade preference, per machine. The grades themselves
   * — the engine's ratings and the words the system's recall card wears — are
   * shared (`ui/src/words.ts`); the consequence on each tile is the engine's
   * own, asked of it with a `dry-run` before it is offered.
   */
  const GRADES_KEY = 'sam.reviews.grades';

  const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  let queue = $state<Queue | null>(null);
  let failure = $state<string | null>(null);
  let loadedFor: string | null = null;
  /** The plan's own records, for the one duration the card prints. The read is
   *  cached per revision, so a screen that already opened costs nothing here. */
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

  let showAll = $state(storedShowAll());

  /** Session metrics for the completion summary ledger. */
  let session = $state<{
    dueIds: string[];
    graded: string[];
    soon: number;
    held: number;
  } | null>(null);

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

  /** What the card is holding: a record the student pinned, else the oldest
   *  one they have not graded in this sitting. */
  const current = $derived(
    (shared.pinnedReview
      ? [...due, ...waiting].find((row) => row.id === shared.pinnedReview)
      : undefined) ??
      due.find((row) => !gradedIds.includes(row.id)) ??
      due[0] ??
      null,
  );

  /** The empty case's two facts, verbatim from the panel: the next record the
   *  engine will bring back, and the oldest one already waiting. */
  const nextBack = $derived(upcoming[0]?.due ?? null);
  const recallable = $derived(waiting[0] ?? null);

  /**
   * The BATCH, frozen at the moment the student answered their first card. It
   * is `session.dueIds.length` and not the live `due.length`, because grading a
   * `Forgot` sends a record away and the next `Forgot` brings one back — a live
   * denominator makes the position counter lie.
   */
  const batch = $derived.by(() => {
    const of = session ? session.dueIds.length : due.length;
    const done = session ? session.graded.length : 0;
    return { done, of, left: Math.max(0, of - done - (current ? 1 : 0)) };
  });
  /** Whether the card is one of the queue's own, or one the student reached
   *  another way (a pinned row). */
  const inQueue = $derived(current ? due.some((row) => row.id === current.id) : false);
  /** The card's place in that batch — what every design's head prints as
   *  `2 of 7`, and null when the card is not one of the queue's. */
  const position = $derived(
    current && inQueue && batch.of > 0 ? { at: batch.done, of: batch.of, left: batch.left } : null,
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

  /** Whole days from the plan's today to a date: positive is ahead, negative is
   *  past. One direction, so no caller can read a past date as a future one. */
  function daysFromToday(day: string): number | null {
    const today = queue?.today;
    if (!today) return null;
    return daysBetween(today, day);
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

  /** A distance from today, in words — never a raw date (R7). */
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

  function recalls(row: Row): number {
    return row.review?.log?.length ?? 0;
  }

  /** How many steps of a record's own ladder it has climbed. */
  function rungOf(row: Row, stage: string | null): number {
    const ladder = queue?.pipelines?.[row.pipeline]?.stages ?? [];
    if (stage && ladder.indexOf(stage) >= 0) return ladder.indexOf(stage) + 1;
    return Math.max(1, row.stages.length);
  }

  /** A course's own code, which is what a student says out loud (D2). */
  function codeOf(context: { id: string; label: string } | null): string | null {
    if (!context) return null;
    const record = model?.byId.get(context.id);
    const value = record?.fields.code ?? record?.fields.short;
    return typeof value === 'string' && value.length > 0 ? value : context.label;
  }

  /** What each shown grade would do, asked of the engine before it is offered. */
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
      // A dry-run that failed answers nothing: the grade is offered without a
      // consequence rather than with a guessed one.
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

  /** The 5s trail a grade leaves: the way back, floating at the design's foot. */
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
    if (session && fromQueue && !session.graded.includes(row.id)) {
      // The ledger's other two figures are sums of these two counters, so they
      // cannot disagree with the grades that produced them.
      session = {
        ...session,
        graded: [...session.graded, row.id],
        soon: session.soon + (rating === 'again' ? 1 : 0),
        held: session.held + (rating === 'good' || rating === 'easy' ? 1 : 0),
      };
    }
    if (shared.pinnedReview === row.id) shared.pinnedReview = null;
    leaveTrail(`Graded ${word}`);
    await refresh();
    loadedFor = app.revision;
  }

  /** Reverse the last write — the trail's own door. */
  async function undo(): Promise<void> {
    await app.undo();
    trail = null;
    if (trailTimer !== null) {
      window.clearTimeout(trailTimer);
      trailTimer = null;
    }
    // The app's own undo is untracked, so the revision never moves: this widget
    // reads its queue again rather than waiting for a publication that is not
    // coming.
    await refresh();
  }

  /** Put a record on the card — the student's choice of what to work on now. */
  function pin(id: string): void {
    shared.pinnedReview = id;
  }

  // ── the surface, as its own contract ──────────────────────────────────
  /** The sitting's own window: the card the student is on and the next four —
   *  a cap, so the design that draws the queue of questions has a bounded list
   *  at any backlog. */
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
   * The one consequence every SHOWN grade earned, when they all earned the same
   * one. `null` while the dry-runs are out, and `null` when the grades differ —
   * which is the case a plan on `sm2` or `fsrs` is always in. A `fixed` plan
   * answers all four ratings with the same date, which is why the date is hoisted
   * to ONE object per card instead of printed on four tiles.
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
    // The line is about the date the plan hands back, so the lateness beside it
    // is THAT date's.
    const back = first.due ? daysFromToday(planDay(first.due) ?? first.due.slice(0, 10)) : null;
    const when = back === null || back === 0 ? 'due today' : back < 0 ? latePhrase(-back) : `in ${back} days`;
    return { day: dayText(first.due), say: interval ? `${interval} · ${when}` : when };
  });

  /** The four ratings, with the rung each leaves the topic on and — only when
   *  the grades disagree — the consequence each one alone earned. */
  const grades = $derived<RecallGrade[]>(
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
</script>

{#if recall}
  <Variant surface="reviews-recall" {...recall} />
{:else if failure}
  <!-- A read that failed is a stated state, never "Nothing is due" (R19, D8):
       what failed, and one control that tries it again — the same words the
       panel's own failure card carries. -->
  <section class="cd-card" data-part="error">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="refresh" size={24} /></span>
      <div class="cd-empty__t">The queue could not be read</div>
      <div class="cd-empty__s">{failure}</div>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => void refresh()}>
        <Icon name="refresh" size={13} />
        Try again
      </button>
    </div>
  </section>
{:else}
  <!-- Empty state: displays when nothing is due or next scheduled item. -->
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
        <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => pin(recallable.id)}>
          Recall {recallable.title}
        </button>
      {/if}
    </div>
  </section>
{/if}
