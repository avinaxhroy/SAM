<!--
  Progress (S6, UI P2 · U4) — study metrics and set review status (F8).

  THE FIGURES SURFACE
  Derives week totals, activity metrics, and set recommendations for the
  progress-stats surface (`ui/src/variants/progress-stats/props.ts`).

  THE WEEK WINDOW
  Renders the seven days ending on the plan's current date. Figures compare
  against the preceding 7-day window when available.

  THE CHART SURFACE
  Mounts the week-chart surface using the same seven-day data window, ensuring
  figure and chart consistency.
-->
  <script lang="ts">
    import Icon from '../shell/Icon.svelte';
    import Variant from '../variants/Variant.svelte';
    import type { Figure, ProgressDay } from '../variants/progress-stats/props';
    import { app } from '../session.svelte';
    import { durationText, recordLabel, type RecordDoc } from '../types';

    let { title }: { type: string; title: string } = $props();

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

    /** 14-day history window for progress calculations (current week and preceding week). */
    let history = $state<DayRead[]>([]);

    let loadedFor: string | null = null;
    let reading = $state(true);
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
    /** Review ladder stages by record ID from `reviews.due`. */
    let progress = $state<Map<string, string[]>>(new Map());

    /**
     * Fetch two consecutive 7-day windows ending on plan today (`app.today`, R2).
     * Deduplicates and sorts days by reported date across parallel queries.
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
          // UTC arithmetic on date-only key to avoid DST boundary shifts.
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
      // Resolve trackable subject kinds from reviews or today view destinations.
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

      // Two 7-day query windows anchored to plan today (R2): current week and previous week.
      history = (await readWindow(14)).days;

      studyRecords = study ? ((await app.recordsOf(study)) ?? []) : [];
      setRecords = sets ? ((await app.recordsOf(sets)) ?? []) : [];
      subjectRecords = subjects ? ((await app.recordsOf(subjects)) ?? []) : [];
      sessionRecords = await app.recordsOf('session');

      reading = false;
    }

    // Re-query progress on accepted revision publications.
    $effect(() => {
      const revision = app.revision;
      if (loadedFor === revision) return;
      loadedFor = revision;
      void read();
    });

    // ── the week, and the term ──────────────────────────────────────────────

    /** Format UTC date as YYYY-MM-DD string. */
    function isoOf(date: Date): string {
      const year = date.getUTCFullYear();
      const month = String(date.getUTCMonth() + 1).padStart(2, '0');
      const day = String(date.getUTCDate()).padStart(2, '0');
      return `${year}-${month}-${day}`;
    }

    const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
    const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

    const targetMin = $derived(today?.totals?.targetMin ?? null);
    /** Target weekly study minutes: daily aim multiplied by 7 days. */
    const weekAim = $derived(targetMin === null ? null : targetMin * 7);

    const week = $derived(history.slice(-7));
    const before = $derived(history.length >= 14 ? history.slice(-14, -7) : []);

    /** Daily logged minutes map across term history. */
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

    /** Formatted day representation for chart components. */
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

    /** 15-week term history starting on Monday, oldest first. */
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
    const emptyDays = $derived(weekDays.length - daysWithWork);
    const beforeTotal = $derived(before.reduce((sum, day) => sum + day.loggedMin, 0));
    const beforeDaysWithWork = $derived(before.filter((day) => day.loggedMin > 0).length);
    /** Preceding week exists only when query returned a full 7-day prior window. */
    const hasPrev = $derived(before.length === 7);

    /** Duration delta against comparison period. */
    function durationDelta(now: number, then: number, period: string): string {
      const change = now - then;
      if (change === 0) return `level with ${period}`;
      return `${durationText(Math.abs(change))} ${change > 0 ? 'more' : 'less'} than ${period}`;
    }

    /** Day count delta against comparison period. */
    function daysDelta(now: number, then: number, period: string): string {
      const change = now - then;
      if (change === 0) return `level with ${period}`;
      const many = Math.abs(change) === 1 ? 'one day' : `${Math.abs(change)} days`;
      return `${many} ${change > 0 ? 'more' : 'fewer'} than ${period}`;
    }

    /** Summary phrase for current week performance. */
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

    /** Summary phrase for term history. */
    const termNote = $derived.by(() => {
      const worked = term.filter((day) => day.minutes > 0).length;
      const total = term.reduce((sum, day) => sum + day.minutes, 0);
      return `${worked} of ${term.length} days worked · ${durationText(total)} logged`;
    });

    /** Unmeasured session minutes lacking explicit date fields. */
    const undatedSessions = $derived(
      sessionRecords.filter(
        (record) => typeof record.fields.min === 'number' && typeof record.fields.date !== 'string',
      ),
    );
    const undatedSessionMin = $derived(
      undatedSessions.reduce((sum, record) => sum + (typeof record.fields.min === 'number' ? record.fields.min : 0), 0),
    );

    /** Summary caption for week chart (R9). */
    const chartNote = $derived.by(() => {
      if (undatedSessions.length > 0) {
        const noun = undatedSessions.length === 1 ? 'session carries' : 'sessions carry';
        return `Your ${undatedSessions.length} logged ${noun} no day, so the ${durationText(undatedSessionMin)} in them cannot be counted here. Give a session a day and it appears on this chart.`;
      }
      if (emptyDays === 0) return 'All seven days have work on them.';
      return emptyDays === 1
        ? '1 of these 7 days has nothing logged.'
        : `${emptyDays} of these 7 days have nothing logged — a fact, not a warning.`;
    });

    // ── the figures ─────────────────────────────────────────────────────────

    /** True if record has recorded stage in review pipeline (§8.3). */
    function selfTested(id: string): boolean {
      return (progress.get(id)?.length ?? 0) > 0;
    }

    const studyTotal = $derived(studyRecords.length);
    const studyTested = $derived(studyRecords.filter((record) => selfTested(record.id)).length);
    /** Pluralized label for kind. */
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

    /** Fallback label when plan contains no predecessor period. */
    const NO_PREV = 'no earlier count in this plan';

    /** Four key progress metric props for variant contracts (`props.ts`). */
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
          // Single problemset solved metric has no predecessor in history.
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

    const headLine = $derived(
      `The last 7 days, read from your own records — nothing here is a score`,
    );

    const coverageLine = $derived(
      coverage.length === 0
        ? 'This plan keeps no subjects'
        : partial
          ? 'Listed in parts, so these counts are a floor'
          : 'Counted once you have been asked the question',
    );

    // ── coverage by subject ─────────────────────────────────────────────────

    /** Resolve subject record ID for work record link. */
    const subjectsById = $derived(new Map(subjectRecords.map((record) => [record.id, record])));

    function subjectIdOf(record: RecordDoc): string | null {
      for (const ids of Object.values(record.links ?? {})) {
        for (const id of ids) if (subjectsById.has(id)) return id;
      }
      return null;
    }

    const coverage = $derived.by(() => {
      const rows = subjectRecords.map((record) => ({
        id: record.id,
        label: recordLabel(record),
        total: 0,
        tested: 0,
      }));
      const index = new Map(rows.map((row) => [row.id, row]));
      for (const record of studyRecords) {
        const id = subjectIdOf(record);
        const bucket = id ? index.get(id) : undefined;
        if (!bucket) continue;
        bucket.total += 1;
        if (selfTested(record.id)) bucket.tested += 1;
      }
      return rows.filter((row) => row.total > 0);
    });

    /** Review queue capacity cap indicator for unscheduled records. */
    const partial = $derived(
      (review?.counts?.waiting ?? 0) >= 50 && (review?.waiting?.length ?? 0) >= 50,
    );

    /** Days elapsed since recorded date relative to plan today. */
    function daysSince(value: unknown): number | null {
      const iso = typeof value === 'string' ? value.slice(0, 10) : null;
      const now = today?.date;
      if (!iso || !now) return null;
      const then = Date.parse(`${iso}T00:00:00Z`);
      const at = Date.parse(`${now}T00:00:00Z`);
      if (Number.isNaN(then) || Number.isNaN(at)) return null;
      return Math.max(0, Math.round((at - then) / 86_400_000));
    }

    /** Relative elapsed time description; null if record lacks date. */
    function sinceText(value: unknown): string | null {
      const ago = daysSince(value);
      if (ago === null) return null;
      if (ago === 0) return 'tried today';
      if (ago === 1) return 'tried yesterday';
      if (ago < 21) return `tried ${ago} days ago`;
      if (ago < 60) return `tried ${Math.round(ago / 7)} weeks ago`;
      return `tried ${Math.round(ago / 30)} months ago`;
    }

    /**
     * Problem set recommendations: ascending completion share, breaking ties
     * by plan record order. Capped at 3 items.
     */
    const weak = $derived.by(() => {
      const anyDay = setRecords.some((record) => daysSince(record.fields.lastAttempt) !== null);
      const rows = setRecords
        .map((record, index) => {
          const total = number(record, 'total');
          const solved = number(record, 'solved');
          const share = total && total > 0 && solved !== null ? solved / total : null;
          return { record, index, total, solved, share, ago: daysSince(record.fields.lastAttempt) };
        })
        .filter((row) => row.total !== null && row.total > 0)
        .filter((row) => (row.share !== null && row.share < 0.6) || (row.ago !== null && row.ago >= 21))
        .sort((left, right) => (left.share ?? 1) - (right.share ?? 1) || (right.ago ?? 0) - (left.ago ?? 0) || left.index - right.index);
      return { rows: rows.slice(0, 3), anyDay };
    });

    /** Human-readable description of current recommendation filter. */
    const weakSub = $derived.by(() => {
      if (setRecords.length === 0) return 'This plan holds no question sets yet';
      if (weak.rows.length === 0) {
        return weak.anyDay
          ? 'Every set is at least three fifths solved, and none is a month old'
          : 'Every set is at least three fifths solved';
      }
      return weak.anyDay
        ? 'Least solved first, and sets left alone for a month'
        : 'Least solved first — no set in this plan records a day, so none is ranked by age';
    });

    /** Problem sets with recorded attempts but no timestamp. */
    const undatedSets = $derived(
      setRecords.filter(
        (record) => (number(record, 'attempted') ?? 0) > 0 && daysSince(record.fields.lastAttempt) === null,
      ).length,
    );

    const practice = $derived(
      app.navigation.find((entry) => {
        const def = app.views?.views?.[entry.view];
        return kinds.sets !== null && def?.type === kinds.sets && !def.panel;
      }) ?? null,
    );
  </script>

  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{headLine}</p>
    </div>
    <!-- Single week view with contextual notes. -->
  </header>

  {#if reading}
    <!-- Loading skeleton placeholder (R19). -->
    <div class="cd-stack" aria-busy="true">
      <p class="cd-sr" role="status">Reading your records…</p>
      <div class="cd-stats">
        {#each [0, 1, 2, 3] as cell (cell)}
          <div class="pr-fig--skel" aria-hidden="true">
            <span class="cd-skel" style="--w: 58%; height: var(--text-2xl)"></span>
            <span class="cd-skel" style="--w: 82%"></span>
          </div>
        {/each}
      </div>
      <section class="cd-card">
        <div class="cd-skel__plot">
          {#each [42, 66, 24, 78, 50, 30, 60] as stem (stem)}
            <span class="cd-skel" style={`--v: ${stem}%`}></span>
          {/each}
        </div>
      </section>
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
  {:else if failed}
    <section class="cd-card">
      <!-- Error retry state (R19, D8). -->
      <div class="cd-empty cd-empty--tight">
        <span class="cd-empty__art"><Icon name="refresh" size={24} /></span>
      <div class="cd-empty__t">Your progress did not come back</div>
      <div class="cd-empty__s">
        The plan itself is untouched — the counts could not be read this time.
      </div>
      <button
        class="cd-pill cd-pill--ghost cd-pill--sm"
        type="button"
        title={app.lastDiagnostic ?? ''}
        onclick={() => void read()}
      >
        Read it again
      </button>
    </div>
  </section>
{:else}
  <div class="cd-stack">
    <!-- Four key progress figures across variant designs. -->
    <Variant
      surface="progress-stats"
      {figures}
      week={weekDays}
      {term}
      aimMin={targetMin}
      {weekNote}
      {termNote}
    />

    <!-- Weekly study activity chart. -->
    <Variant
      surface="week-chart"
      title="Work per day"
      caption={`${durationText(weekTotal)} across the last seven days`}
      days={weekDays.map((day) => ({
        date: day.date,
        weekday: day.weekday,
        day: day.day,
        loggedMin: day.minutes,
        isToday: day.isToday,
      }))}
      {targetMin}
    />
    <p class="cd-hint">{chartNote}</p>

    <div class="cd-grid-2">
      <!-- Subject coverage meters. -->
      <section class="cd-card" data-part="subjects">
        <header class="cd-card__head">
          <span class="cd-ictile"><Icon name="book" /></span>
          <div>
            <h2 class="cd-card__title">Coverage by subject</h2>
            <p class="cd-card__sub">{coverageLine}</p>
          </div>
        </header>
        {#if coverage.length === 0}
          <div class="cd-empty cd-empty--tight">
            <span class="cd-empty__art"><Icon name="book" size={24} /></span>
            <div class="cd-empty__t">Nothing groups work by subject here</div>
            <div class="cd-empty__s">
              So the first figure above is the whole coverage, and this card has nothing to add.
            </div>
          </div>
        {:else}
          <div aria-label="Coverage by subject">
            {#each coverage as row (row.id)}
              {@const left = row.total - row.tested}
              <div class="cd-meterrow" data-subject={row.id} data-w={app.washes[row.id]}>
                <span class="cd-chip cd-chip--wash" data-w={app.washes[row.id]}>
                  {row.label}
                </span>
                <span class="cd-meter cd-meter--head cd-meter--wash">
                  <span class="cd-meter__track">
                    <span
                      class="cd-meter__fill"
                      style={`--v: ${row.total > 0 ? Math.round((row.tested / row.total) * 100) : 0}%`}
                    ></span>
                  </span>
                </span>
                <span class="cd-meterrow__say">
                  {row.tested} of {row.total} self-tested
                  <span class="cd-meterrow__left">· {left} not yet</span>
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <section class="cd-card" data-part="weak">
        <header class="cd-card__head">
          <span class="cd-ictile"><Icon name="flag" /></span>
          <div>
            <h2 class="cd-card__title">Where to look next</h2>
            <p class="cd-card__sub">{weakSub}</p>
          </div>
          {#if practice}
            <span class="cd-card__spacer"></span>
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              data-command="rail.select"
              data-placement="progress.screen"
              aria-label={`Open ${practice.title} to work on a set`}
              onclick={() => void app.run('rail.select', { view: practice.view })}
            >
              <Icon name="bolt" size={13} />
              {practice.title}
            </button>
          {/if}
        </header>

        {#if setRecords.length === 0}
          <div class="cd-empty cd-empty--tight">
            <span class="cd-empty__art"><Icon name="checklist" size={24} /></span>
            <div class="cd-empty__t">This plan holds no question sets</div>
            <div class="cd-empty__s">
              A set is where a chapter's questions live, and coverage follows it.
            </div>
          </div>
        {:else if weak.rows.length === 0}
          <div class="cd-empty cd-empty--tight">
            <span class="cd-empty__art"><Icon name="check" size={24} /></span>
            <div class="cd-empty__t">Nothing looks thin</div>
            <div class="cd-empty__s">
              {#if weak.anyDay}
                Every set is at least half solved, and none has been left alone for a month.
              {:else}
                Every set is at least half solved, so there is nothing here to flag.
              {/if}
            </div>
          </div>
        {:else}
          <div class="cd-coll">
            {#each weak.rows as row (row.record.id)}
              {@const courseId = (row.record.links.course ?? [])[0]}
              {@const course = courseId ? subjectsById.get(courseId) : undefined}
              {@const since = sinceText(row.record.fields.lastAttempt)}
              {@const share = row.share === null ? 0 : Math.round(row.share * 100)}
              <div class="cd-coll__row pr-set" data-part="set" data-record-id={row.record.id}>
                <span class="cd-coll__body">
                  <span class="cd-coll__title">{recordLabel(row.record)}</span>
                  <span class="cd-rowmeta">
                    {#if course}
                      <span class="cd-chip cd-chip--wash" data-w={app.washes[course.id]}>
                        {recordLabel(course)}
                      </span>
                    {/if}
                    <!-- Solved count with denominator. -->
                    <span class="cd-metric">
                      <span class="cd-metric__n">{row.solved ?? 0}</span>
                      <span class="cd-metric__d">of {row.total} solved</span>
                    </span>
                    {#if since}<span>{since}</span>{/if}
                  </span>
                </span>
                <span class="cd-coll__right">
                  <span
                    class="cd-meter cd-meter--head"
                    role="img"
                    aria-label={`${share}% of ${recordLabel(row.record)} solved`}
                  >
                    <span class="cd-meter__track">
                      <span class="cd-meter__fill" style={`--v: ${share}%`}></span>
                    </span>
                  </span>
                </span>
              </div>
            {/each}
          </div>
          {#if undatedSets > 0}
            <p class="cd-hint" data-part="set-note">
              {undatedSets} of your {setRecords.length} sets record questions tried but keep no day
              for it, so none of them can be ranked by how long ago you worked it.
            </p>
          {/if}
        {/if}
      </section>
    </div>
  </div>
{/if}
