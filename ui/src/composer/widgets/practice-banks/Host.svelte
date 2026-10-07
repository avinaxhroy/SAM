<!--
  PRACTICE BANKS widget host (`COMPOSER.md` §3.1).
  Mounts the active practice banks variant within composed screens,
  deriving problem set totals, solved percentages, and attempt logs.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import type { PracticeBank, PracticeSummary } from '../../../variants/practice-banks/props';
  import { app } from '../../../session.svelte';
  import { nameOf, recordLabel, type RecordDoc, type ViewRead } from '../../../types';
  import { readPlanModel, type PlanModel } from '../../../panels/model';

  /** This surface's own view, and the title the shell names it by. */
  const home = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'practice')?.[0] ?? null,
  );
  const title = $derived(
    app.navigation.find((entry) => entry.view === home)?.title ?? nameOf(home ?? ''),
  );

  /**
   * The destination read, when the app is not already showing this surface's
   * own view. `app.outcome` is used as the panel used it while it is.
   */
  let elsewhere = $state<ViewRead | null>(null);
  let viewLoadedFor: string | null = null;

  $effect(() => {
    const revision = app.revision;
    const shown = app.outcome?.view ?? null;
    const key = `${revision}|${home ?? ''}|${shown ?? ''}`;
    if (viewLoadedFor === key) return;
    viewLoadedFor = key;
    if (!home || shown === home) {
      elsewhere = null;
      return;
    }
    void app.run('view', { name: home }, { tracked: false }).then((result) => {
      elsewhere = (result?.data as ViewRead) ?? null;
    });
  });

  const destination = $derived(app.outcome?.view === home ? app.outcome : elsewhere);

  /** The destination's own view already resolved this kind's records (with the
      plan's formula values in `derived`), so the screen reads them there. */
  const sets = $derived((destination?.records ?? []) as RecordDoc[]);

  /** The model carries the names of the things these banks point at. The banks,
      their counts and their bars are on screen the moment the destination
      resolves; only the course names arrive one read later. */
  let model = $state<PlanModel | null>(null);
  let loadingNames = $state(true);
  let failedNames = $state(false);
  let loadedFor: string | null = null;

  async function readNames(): Promise<void> {
    loadingNames = true;
    failedNames = false;
    try {
      model = await readPlanModel();
    } catch {
      model = null;
      failedNames = true;
    } finally {
      loadingNames = false;
    }
  }

  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void readNames();
  });

  /** The bank whose entry is open. The screen owns this one fact and hands it
      down, so all three designs read the same selection. */
  let openedId = $state<string | null>(null);
  let busy = $state(false);

  function num(record: RecordDoc, key: string): number | null {
    const value = record.fields[key];
    return typeof value === 'number' ? value : null;
  }

  /** The bank's own whole, or null when the plan declares no size for it. */
  function totalOf(record: RecordDoc): number | null {
    const total = num(record, 'total');
    return total !== null && total > 0 ? total : null;
  }

  /**
   * The bank's solved share, rounded, or null when it declares no size. The
   * plan's own formula (`pct(solved, total)`, `types.json#/types/problemset`)
   * already returns a **percentage** — 26.666… for a 12-of-45 bank — so it is
   * used as it stands; dividing it by the total again is what produced bars at
   * 59% and 73% for two banks that are 27% and 26% solved. A kind that declares
   * no formula gets the same arithmetic here.
   */
  function shareOf(record: RecordDoc): number | null {
    const total = totalOf(record);
    if (total === null) return null;
    const derived = record.derived?.pct;
    const solved = num(record, 'solved');
    const exact = typeof derived === 'number' ? derived : solved === null ? null : (solved / total) * 100;
    return exact === null ? null : Math.max(0, Math.min(100, Math.round(exact)));
  }

  /**
   * The age of a bank's last attempt, in the app's own words — and the one place
   * the screen is allowed to be silent about nothing: a bank whose questions
   * carry no date has **no date**, which is not the same fact as "no attempt
   * recorded", and the screen never infers one.
   */
  function sinceText(record: RecordDoc): string {
    const value = record.fields.lastAttempt;
    const tried = num(record, 'attempted') ?? 0;
    const blank = tried > 0 ? 'no date recorded' : 'no attempt yet';
    if (typeof value !== 'string' || value.length < 10) return blank;
    const then = Date.parse(`${value.slice(0, 10)}T00:00:00`);
    if (Number.isNaN(then)) return blank;
    const days = Math.max(0, Math.round((Date.now() - then) / 86_400_000));
    if (days === 0) return 'tried today';
    if (days === 1) return 'tried yesterday';
    if (days < 21) return `tried ${days} days ago`;
    if (days < 60) return `tried ${Math.round(days / 7)} weeks ago`;
    return `tried ${Math.round(days / 30)} months ago`;
  }

  /** A course's own code, which is what a student says out loud. */
  function codeOf(record: RecordDoc | null): string | null {
    if (record === null) return null;
    const value = record.fields.code ?? record.fields.short;
    return typeof value === 'string' && value.length > 0 ? value : null;
  }

  const banks = $derived.by<PracticeBank[]>(() => {
    return sets.map((record) => {
      const courseId = (record.links.course ?? [])[0];
      const course = courseId ? (model?.byId.get(courseId) ?? null) : null;
      return {
        id: record.id,
        type: record.type,
        label: recordLabel(record),
        course: course === null ? null : recordLabel(course),
        code: codeOf(course),
        wash: courseId ? app.washes[courseId] : undefined,
        total: totalOf(record),
        attempted: num(record, 'attempted') ?? 0,
        solved: num(record, 'solved') ?? 0,
        share: shareOf(record),
        since: sinceText(record),
      };
    });
  });

  const summary = $derived.by<PracticeSummary>(() => {
    let questions = 0;
    let attempted = 0;
    let solved = 0;
    let thin = 0;
    for (const bank of banks) {
      questions += bank.total ?? 0;
      attempted += bank.attempted;
      solved += bank.solved;
      if (bank.share !== null && bank.share < 60) thin += 1;
    }
    return { banks: banks.length, questions, attempted, solved, thin };
  });

  const namesState = $derived(failedNames ? 'failed' : loadingNames ? 'loading' : 'ready');

  /**
   * The day a save stamps, in the plan's own words — the same `new Date()` the
   * write below stamps from, so the sentence at the control and the value on the
   * disk cannot disagree about what "today" is.
   */
  const stamp = (() => {
    const now = new Date();
    const weekday = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][now.getDay()];
    const month = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][now.getMonth()];
    return `${weekday} ${now.getDate()} ${month}`;
  })();

  /** Commits updated question bank attempt and solved counts if changed. */
  async function logAttempt(id: string, attempted: number, solved: number): Promise<boolean> {
    const record = sets.find((item) => item.id === id);
    if (!record || busy || solved > attempted) return false;
    const writes: Array<[string, string]> = [];
    if (attempted !== num(record, 'attempted')) writes.push(['attempted', String(attempted)]);
    if (solved !== num(record, 'solved')) writes.push(['solved', String(solved)]);
    // Skip timestamp update if counts did not change.
    if (writes.length === 0) return true;
    busy = true;
    try {
      writes.push(['lastAttempt', new Date().toISOString().slice(0, 10)]);
      // Every write that lands is announced; the first refusal stops the run and
      // leaves the engine's own reason in the diagnostics rather than a false
      // receipt on screen.
      for (const [field, value] of writes) {
        const result = await app.run('record.setField', { id, field, value });
        if (!result) return false;
      }
      app.notice(`logged ${recordLabel(record)}`, { label: 'Undo', run: () => void app.undo() });
      return true;
    } finally {
      busy = false;
    }
  }

  /** The record door: the app's own `record.panel` for one bank. */
  function openRecord(id: string): void {
    const record = sets.find((item) => item.id === id);
    if (record) void app.run('record.panel', { id, type: record.type });
  }

  const bankType = $derived(destination?.type ?? null);
</script>

<Variant
  surface="practice-banks"
  {title}
  {banks}
  {summary}
  {openedId}
  {namesState}
  newBankCommand={bankType ? `${bankType}.new` : null}
  {stamp}
  {busy}
  onOpen={(id) => (openedId = id)}
  onClose={() => (openedId = null)}
  onRecord={openRecord}
  onNewBank={() => {
    if (bankType) void app.openSheet({ kind: 'record.new', type: bankType });
  }}
  onRetryNames={() => void readNames()}
  onLog={logAttempt}
/>
