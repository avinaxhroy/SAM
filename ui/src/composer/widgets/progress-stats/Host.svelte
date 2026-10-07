<!--
  PROGRESS-STATS · THE HOST — the week's figures as a widget the composer mounts
  (2026-09-29).

  This is the surface's own derivation moved out of `panels/Progress.svelte`, not
  re-written: the same two reads (`today.view`, `reviews.due`), the same
  `readWindow(14)` union of the week ending on the plan's own today and the seven
  days before it, the same `mark`/`weekDays`/`term` day objects, and the same
  four `figures` with their own denominators and deltas. It takes no props, owns
  its own read state, and hands `<Variant surface="progress-stats">` exactly the
  object the panel used to spread.

  THE PAIRING. `today.view` and `reviews.due` are issued together (and the two
  weeks are issued as two pages of the same read), exactly as the panel does; the
  read is guarded on `app.revision` so the widget reloads on the same publication
  every screen does and on nothing else.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import Icon from '../../../shell/Icon.svelte';
  import type { Figure, ProgressDay } from '../../../variants/progress-stats/props';
  import { app } from '../../../session.svelte';
  import { durationText, type RecordDoc } from '../../../types';

  /** One row of the engine's review queue — the stage names it holds per record. */
  type ReviewRow = { id: string; stages?: string[] };
  type ReviewRead = {
    counts?: Record<string, number>;
    due?: ReviewRow[];
    upcoming?: ReviewRow[];
    waiting?: ReviewRow[];
  };
  type DayRead = { date: string; weekday: string; day: number; loggedMin: number; isToday: boolean };
  type TodayRead = {
    date: string;
    days?: DayRead[];
    totals?: { loggedMin: number; targetMin: number | null };
  };

  /** The two weeks the figures are read over: the seven days ending on the
      plan's own today, and the seven before them. `today.view` reports seven
      days ending on the day it is asked about, so the second week is the same
      read stepped back seven days — one read pair, both from the plan's own
      today, exactly as the lab's `WEEK`/`PREV` pair (NOTES, *The one data
      decision*). */
  let history = $state<DayRead[]>([]);

  let loadedFor: string | null = null;
  let reading = $state(true);
  /** A read that did not come back is a state the widget may draw quietly; the
   *  panel's stale-date line spends it. Kept so the read path is verbatim. */
  let failed = $state(false);

  let today = $state<TodayRead | null>(null);
  let review = $state<ReviewRead | null>(null);
  let kinds = $state<{ study: string | null; sets: string | null; subjects: string | null }>({
    study: null,
    sets: null,
    subjects: null,
  });
  let studyRecords = $state<RecordDoc[]>([]);
  let setRecords = $state<RecordDoc[]>([]);
  let subjectRecords = $state<RecordDoc[]>([]);
  let sessionRecords = $state<RecordDoc[]>([]);
  /** What the engine knows of each record's ladder, by id (`reviews.due`). */
  let progress = $state<Map<string, string[]>>(new Map());

  /**
   * One window of days, oldest first.
   *
   * `today.view` reports the seven days ending on the day it is asked about and
   * takes a `date`, so the second week the figures compare against is the same
   * read stepped back seven days. Two things make the union exact rather than
   * merely intended:
   *
   *  · the pages are keyed on the plan's own today (`app.today`, R2), stepped by
   *    whole days, so no page overlaps another and none depends on the browser's
   *    idea of which day it is;
   *  · the days are then **de-duplicated and sorted by the date the engine
   *    itself reported**, so even a read that answered with a day already seen
   *    cannot double-count it.
   *
   * The pages are issued together rather than in a chain: a chain would need the
   * previous page's answer before it could ask the next question.
   */
  async function readWindow(days: number): Promise<{ days: DayRead[]; total: number }> {
    const first = await app.run('today.view', {}, { tracked: false });
    if (!first) return { days: [], total: 0 };
    const read = first.data as TodayRead;
    const page = read.days ?? [];
    if (days <= 7) {
      return { days: page, total: page.reduce((sum, day) => sum + day.loggedMin, 0) };
    }

    const anchor = app.today?.date ?? read.date;
    const pages = await Promise.all(
      Array.from({ length: Math.ceil(days / 7) - 1 }, (_, index) => {
        const [year, month, day] = anchor.split('-').map(Number);
        // UTC arithmetic on a date-only key: a local `Date` would cross a DST
        // boundary and land a page an hour off, which is a day off at midnight.
        const shifted = new Date(Date.UTC(year, month - 1, day - 7 * (index + 1)));
        return app.run('today.view', { date: isoOf(shifted) }, { tracked: false });
      }),
    );

    const byDate = new Map<string, DayRead>();
    for (const day of page) byDate.set(day.date, day);
    for (const extra of pages) {
      if (!extra) continue;
      for (const day of ((extra.data as TodayRead).days ?? []) as DayRead[]) byDate.set(day.date, day);
    }
    const window = [...byDate.values()]
      .sort((left, right) => left.date.localeCompare(right.date))
      .slice(-days);
    return { days: window, total: window.reduce((sum, day) => sum + day.loggedMin, 0) };
  }

  async function read(): Promise<void> {
    reading = true;
    failed = false;

    const declared = Object.entries(app.types ?? {});
    const trackable = declared.filter(([, def]) => def.trackable === true).map(([name]) => name);
    const subjects = declared.find(([, def]) => def.colorRole === 'identity')?.[0] ?? null;
    const panels = Object.values(app.views?.views ?? {});
    // The kind the study screens are about: the one the reviews — then today —
    // destination lists. A plan that declares neither falls back to its first
    // trackable kind, and the sets are whatever else is trackable.
    const study =
      (panels.find((def) => def.panel === 'reviews')?.type ??
        panels.find((def) => def.panel === 'today')?.type ??
        trackable[0]) ||
      null;
    const sets = trackable.find((name) => name !== study) ?? null;

    const [facts, queue] = await Promise.all([
      app.run('today.view', {}, { tracked: false }),
      app.run('reviews.due', {}, { tracked: false }),
    ]);

    if (!facts || !queue) {
      failed = true;
      reading = false;
      return;
    }

    kinds = { study, sets, subjects };
    today = facts.data as TodayRead;
    review = queue.data as ReviewRead;

    const rows = new Map<string, string[]>();
    for (const key of ['due', 'upcoming', 'waiting'] as const) {
      for (const row of (review[key] ?? []) as ReviewRow[]) rows.set(row.id, row.stages ?? []);
    }
    progress = rows;

    // The two weeks the figures are read over: the week ending on the plan's
    // own today, and the week before it. One read pair, both anchored on the
    // plan's own today (R2) — nothing here depends on the browser's clock.
    history = (await readWindow(14)).days;

    studyRecords = study ? ((await app.recordsOf(study)) ?? []) : [];
    setRecords = sets ? ((await app.recordsOf(sets)) ?? []) : [];
    subjectRecords = subjects ? ((await app.recordsOf(subjects)) ?? []) : [];
    sessionRecords = await app.recordsOf('session');

    reading = false;
  }

  // One read per accepted revision — the same publication the rest of the app
  // reloads on, and never a read this effect's own writes can retrigger.
  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void read();
  });

  // ── the week, and the term ──────────────────────────────────────────────

  /** `YYYY-MM-DD` from a UTC date — the shape the read reports, and never
      `toISOString` on a local Date, which can shift the day. */
  function isoOf(date: Date): string {
    const year = date.getUTCFullYear();
    const month = String(date.getUTCMonth() + 1).padStart(2, '0');
    const day = String(date.getUTCDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  const targetMin = $derived(today?.totals?.targetMin ?? null);
  /** The plan's own line for the week: its daily aim × the seven days read.
      Null when the plan declares no `dailyTargetMin` — and then nothing on this
      screen prints a denominator for time, because there is none to print. */
  const weekAim = $derived(targetMin === null ? null : targetMin * 7);

  const week = $derived(history.slice(-7));
  const before = $derived(history.length >= 14 ? history.slice(-14, -7) : []);

  /** The minutes each day of the term holds, from the sessions the widget
      already reads — the same records `today.view` sums its day rows from, so
      the grid's last column and the figures cannot disagree. */
  const sessionMinutes = $derived.by(() => {
    const byDate = new Map<string, number>();
    for (const record of sessionRecords) {
      const day = typeof record.fields.date === 'string' ? record.fields.date.slice(0, 10) : null;
      const min = typeof record.fields.min === 'number' ? record.fields.min : 0;
      if (!day || min <= 0) continue;
      byDate.set(day, (byDate.get(day) ?? 0) + min);
    }
    return byDate;
  });

  /** One day, as every design draws it: the words are composed here so all
      three designs say a day the same way. */
  function mark(
    date: string,
    weekday: string,
    dayOfMonth: number,
    month: string,
    minutes: number,
    isToday: boolean,
    full: boolean,
  ): ProgressDay {
    const when = full ? `${weekday} ${dayOfMonth} ${month}` : `${dayOfMonth} ${month}`;
    let fact = `${when} · nothing logged`;
    if (minutes > 0) {
      fact =
        targetMin !== null && full
          ? minutes > targetMin
            ? `${when} · ${durationText(minutes)} of the ${durationText(targetMin)} aim, ${durationText(minutes - targetMin)} over`
            : `${when} · ${durationText(minutes)} of the ${durationText(targetMin)} aim`
          : `${when} · ${durationText(minutes)}`;
    }
    return { date, weekday, day: dayOfMonth, month, when, minutes, isToday, fact };
  }

  /** The week the figures and the rods read, oldest first. */
  const weekDays = $derived.by<ProgressDay[]>(() =>
    week.map((day) =>
      mark(
        day.date,
        day.weekday,
        day.day,
        MONTHS[Number(day.date.slice(5, 7)) - 1] ?? '',
        day.loggedMin,
        day.isToday,
        true,
      ),
    ),
  );

  /** The term so far, oldest first: whole weeks from the Monday fifteen weeks
      back through the plan's own today. Index `i` is column `⌊i / 7⌋` and row
      `i % 7` of the dot grid — which is why the run starts on a Monday, and why
      the grid's Mon/Sun gutter cannot drift from the days it names. */
  const term = $derived.by<ProgressDay[]>(() => {
    const anchor = today?.date;
    if (!anchor) return [];
    const last = new Date(Date.parse(`${anchor}T00:00:00Z`));
    if (Number.isNaN(last.getTime())) return [];
    const row = (last.getUTCDay() + 6) % 7; // Mon = 0
    const weeks = 15; // the fifteen whole weeks before the week today is in
    const first = new Date(last.getTime() - (row + weeks * 7) * 86_400_000);
    const days: ProgressDay[] = [];
    for (let index = 0; index < row + 1 + weeks * 7; index += 1) {
      const date = new Date(first.getTime() + index * 86_400_000);
      const key = isoOf(date);
      days.push(
        mark(
          key,
          WEEKDAYS[index % 7],
          date.getUTCDate(),
          MONTHS[date.getUTCMonth()],
          sessionMinutes.get(key) ?? 0,
          key === anchor,
          false,
        ),
      );
    }
    return days;
  });

  const weekTotal = $derived(weekDays.reduce((sum, day) => sum + day.minutes, 0));
  const daysWithWork = $derived(weekDays.filter((day) => day.minutes > 0).length);
  const beforeTotal = $derived(before.reduce((sum, day) => sum + day.loggedMin, 0));
  const beforeDaysWithWork = $derived(before.filter((day) => day.loggedMin > 0).length);
  /** A predecessor exists only when the read actually answered with a whole
      week before this one; a younger plan prints no arithmetic, never a
      fabricated one (NOTES, revision 3). */
  const hasPrev = $derived(before.length === 7);

  /** A duration's change, against the period named in the same sentence. */
  function durationDelta(now: number, then: number, period: string): string {
    const change = now - then;
    if (change === 0) return `level with ${period}`;
    return `${durationText(Math.abs(change))} ${change > 0 ? 'more' : 'less'} than ${period}`;
  }

  /** A count's change, in whole days, against the named period. */
  function daysDelta(now: number, then: number, period: string): string {
    const change = now - then;
    if (change === 0) return `level with ${period}`;
    const many = Math.abs(change) === 1 ? 'one day' : `${Math.abs(change)} days`;
    return `${many} ${change > 0 ? 'more' : 'fewer'} than ${period}`;
  }

  /** The week's own sentence — A's caption at rest, and the fact the four
      figures are a reading of. */
  const weekNote = $derived.by(() => {
    const worked = `${daysWithWork} ${daysWithWork === 1 ? 'day' : 'days'}`;
    if (weekAim === null) {
      return `${durationText(weekTotal)} over ${worked} · the plan declares no daily aim to measure it against`;
    }
    const short = weekAim - weekTotal;
    const tail =
      short > 0
        ? `${durationText(short)} short of the ${durationText(weekAim)} aim`
        : short === 0
          ? `level with the ${durationText(weekAim)} aim`
          : `${durationText(-short)} over the ${durationText(weekAim)} aim`;
    return `${durationText(weekTotal)} over ${worked} · ${tail}`;
  });

  /** The term's own sentence — the dot grid's caption when nothing is read. */
  const termNote = $derived.by(() => {
    const worked = term.filter((day) => day.minutes > 0).length;
    const total = term.reduce((sum, day) => sum + day.minutes, 0);
    return `${worked} of ${term.length} days worked · ${durationText(total)} logged`;
  });

  // ── the figures ─────────────────────────────────────────────────────────

  /** A record counts as self-tested when the engine has a stage on record for
      it. The ladder moves only through a recall (§8.3), so this is the count of
      things the student has actually been asked about — which is not the same
      as "revised", the word this screen used and the plan does not support. */
  function selfTested(id: string): boolean {
    return (progress.get(id)?.length ?? 0) > 0;
  }

  const studyTotal = $derived(studyRecords.length);
  const studyTested = $derived(studyRecords.filter((record) => selfTested(record.id)).length);
  /** The student's own word for the kind, pluralised the plain way. */
  const studyWord = $derived(
    kinds.study ? (kinds.study.endsWith('s') ? kinds.study : `${kinds.study}s`) : 'records',
  );

  function number(record: RecordDoc, key: string): number | null {
    const value = record.fields[key];
    return typeof value === 'number' ? value : null;
  }

  const questions = $derived.by(() => {
    let total = 0;
    let solved = 0;
    for (const set of setRecords) {
      total += number(set, 'total') ?? 0;
      solved += number(set, 'solved') ?? 0;
    }
    return { total, solved };
  });

  /** The sentence a figure with no predecessor carries in its delta slot: no
      arithmetic, because the plan holds no earlier count to subtract. */
  const NO_PREV = 'no earlier count in this plan';

  /**
   * The four figures, as the contract the three designs read
   * (`ui/src/variants/progress-stats/props.ts`). Each is printed once, each
   * states its denominator — in the fraction where the fraction *is* the
   * figure, in the words where it is not — and each states its delta against a
   * **named period**, or says in words that the plan keeps no predecessor.
   */
  const figures = $derived.by<Figure[]>(() => {
    const solvedOf = questions.total > 0 ? questions.total : null;
    const testedOf = studyTotal > 0 ? studyTotal : null;
    return [
      {
        key: 'time',
        label: 'Time studied',
        value: durationText(weekTotal),
        num: weekTotal,
        of: weekAim,
        ofText: weekAim === null ? 'the plan declares no daily aim' : `of the ${durationText(weekAim)} aimed`,
        delta: hasPrev
          ? durationDelta(weekTotal, beforeTotal, 'the 7 days before')
          : NO_PREV,
        prev: hasPrev
          ? {
              value: durationText(beforeTotal),
              num: beforeTotal,
              of: weekAim,
              ofText: weekAim === null ? 'the plan declares no daily aim' : `of the ${durationText(weekAim)} aimed`,
              delta: durationDelta(beforeTotal, weekTotal, 'this week'),
            }
          : null,
      },
      {
        key: 'days',
        label: 'Days with work',
        value: String(daysWithWork),
        num: daysWithWork,
        of: 7,
        ofText: 'of the 7 days in the week',
        delta: hasPrev ? daysDelta(daysWithWork, beforeDaysWithWork, 'the 7 days before') : NO_PREV,
        prev: hasPrev
          ? {
              value: String(beforeDaysWithWork),
              num: beforeDaysWithWork,
              of: 7,
              ofText: 'of the 7 days before',
              delta: daysDelta(beforeDaysWithWork, daysWithWork, 'this week'),
            }
          : null,
      },
      {
        key: 'solved',
        label: 'Questions solved',
        value: String(questions.solved),
        num: questions.solved,
        of: solvedOf,
        ofText:
          solvedOf === null
            ? 'the plan declares no question sets'
            : `of the ${solvedOf} the plan declares`,
        // A `problemset` holds one `solved` count and the plan keeps no earlier
        // one, so there is no predecessor for this figure to subtract — and a
        // delta printed here would be an invention, not a measurement.
        delta: NO_PREV,
        prev: null,
      },
      {
        key: 'tested',
        label: `${studyWord} you have self-tested`,
        value: String(studyTested),
        num: studyTested,
        of: testedOf,
        ofText: testedOf === null ? 'the plan keeps no records of that kind' : `of the ${testedOf} ${studyWord}`,
        delta: NO_PREV,
        prev: null,
      },
    ];
  });

  const statsProps = $derived({
    figures,
    week: weekDays,
    term,
    aimMin: targetMin,
    weekNote,
    termNote,
  });
</script>

{#if failed}
  <!-- A read that failed states the fact (R19, D8) instead of drawing a zeroed
       week as if the student had done nothing. -->
  <section class="cd-card" data-part="error">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="refresh" size={24} /></span>
      <div class="cd-empty__t">The figures could not be read</div>
      <div class="cd-empty__s">The plan did not answer. Try again in a moment, or run SAM --configcheck.</div>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => void read()}>
        <Icon name="refresh" size={13} />
        Try again
      </button>
    </div>
  </section>
{:else}
  <Variant surface="progress-stats" {...statsProps} />
{/if}
