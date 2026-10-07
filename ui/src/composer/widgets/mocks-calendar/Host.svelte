<!-- Mocks calendar widget host (COMPOSER §3.1). -->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import Icon from '../../../shell/Icon.svelte';
  import {
    byDayOrder,
    dateOf,
    dayKeyOf,
    spokenDay,
    type MockRecord,
  } from '../../../variants/mocks-calendar/props';
  import { app } from '../../../session.svelte';
  import { nameOf, recordLabel, type RecordDoc, type ViewRead } from '../../../types';
  import { readPlanModel, weekIndex, weekName, type PlanModel } from '../../../panels/model';

  /** This surface's own view, and the title the shell names it by. */
  const home = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'mocks')?.[0] ?? null,
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

  const rows = $derived((destination?.records ?? []) as RecordDoc[]);

  let model = $state<PlanModel | null>(null);
  let loading = $state(true);
  let failed = $state(false);
  let loadedFor: string | null = null;

  async function read(): Promise<void> {
    loading = true;
    failed = false;
    try {
      model = await readPlanModel();
    } catch {
      model = null;
      failed = true;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const revision = app.revision;
    if (loadedFor === revision) return;
    loadedFor = revision;
    void read();
  });

  /** The key the plan keeps its date in, found on the kind's own declaration. */
  const dateKey = $derived(
    (app.types[destination?.type ?? '']?.fields ?? []).find((field) => field.type === 'date')?.key ?? 'date',
  );

  function dayOf(record: RecordDoc): string | null {
    const value = record.fields[dateKey];
    return typeof value === 'string' && value.length >= 10 ? value.slice(0, 10) : null;
  }

  /** The plan's own today (`views.today`), never the machine's clock. */
  const todayKey = $derived(app.today?.date ?? dayKeyOf(new Date()));

  /** A day offset from the plan's today, as a plan key. */
  function offsetKey(days: number): string {
    const at = dateOf(todayKey);
    at.setDate(at.getDate() + days);
    return dayKeyOf(at);
  }

  /** The plan's own week for an assessment, when it declares one. */
  function weekText(record: RecordDoc): string | null {
    const weekId = (record.links.week ?? [])[0];
    if (!weekId || !model) return null;
    const week = model.byId.get(weekId);
    if (!week) return null;
    const index = weekIndex(model.weeks, weekId);
    const name = weekName(week, index ?? 0);
    return index !== null && !name.toLowerCase().includes(String(index)) ? `${name}, week ${index}` : name;
  }

  /** The assessment's own courses, in the plan's order. */
  function coursesOf(record: RecordDoc): RecordDoc[] {
    return (record.links.course ?? [])
      .map((id) => model?.byId.get(id))
      .filter((found): found is RecordDoc => Boolean(found));
  }

  function toMock(record: RecordDoc): MockRecord {
    return {
      id: record.id,
      type: record.type,
      label: recordLabel(record),
      kind: typeof record.fields.kind === 'string' ? record.fields.kind : null,
      day: dayOf(record),
      week: weekText(record),
      score: typeof record.fields.score === 'number' ? record.fields.score : null,
      courses: coursesOf(record).map((course) => {
        const code = course.fields.code ?? course.fields.short;
        return {
          id: course.id,
          code: typeof code === 'string' && code.length > 0 ? code : null,
          label: recordLabel(course),
          wash: app.washes[course.id],
        };
      }),
    };
  }

  /** The assessments, split once: the ones with a day, and the ones waiting. */
  const dated = $derived(rows.filter((record) => dayOf(record) !== null).map(toMock).sort(byDayOrder));
  const waiting = $derived(rows.filter((record) => dayOf(record) === null).map(toMock));

  /**
   * The day the calendar opens on: the plan's next assessment, so the screen
   * answers its own question the moment it draws — the lab opens on the month
   * that carries the work, not on the month of today. When nothing is dated it
   * opens on today.
   */
  const anchor = $derived.by(() => {
    const days = dated.map((record) => record.day ?? '').sort();
    return days.find((day) => day >= todayKey) ?? days[days.length - 1] ?? todayKey;
  });

  /**
   * The quiet weekdays the date controls offer: the earliest weekday with
   * nothing on it in each of the next three weeks, computed ONCE from the plan
   * so a commit cannot change the alternatives the control is offering
   * (`design/controls.md` §2's offsets, drawn as real days rather than
   * `+7d`). A plan that filled every weekday still gets the next three
   * weekdays, because a control with nothing to choose is a dead end.
   */
  const quiet = $derived.by(() => {
    const busy = new Set(dated.map((record) => record.day));
    const out: string[] = [];
    const weeks = new Set<number>();
    for (let ahead = 7; ahead <= 27 && out.length < 3; ahead += 1) {
      const key = offsetKey(ahead);
      const weekday = dateOf(key).getDay();
      if (weekday === 0 || weekday === 6 || busy.has(key)) continue;
      const week = Math.floor((ahead - 1) / 7);
      if (weeks.has(week)) continue;
      weeks.add(week);
      out.push(key);
    }
    for (let ahead = 1; ahead <= 14 && out.length < 3; ahead += 1) {
      const key = offsetKey(ahead);
      const weekday = dateOf(key).getDay();
      if (weekday === 0 || weekday === 6 || out.includes(key)) continue;
      out.push(key);
    }
    return out;
  });

  /** The record door: the plan's own `<type>.new`, exactly as the shell names it. */
  const newCommand = $derived(destination ? `${destination.type}.new` : null);

  function openNew(): void {
    void app.openSheet({ kind: 'record.new', type: destination?.type ?? '' });
  }

  async function setDay(id: string, day: string): Promise<boolean> {
    if (day.length !== 10) return false;
    return Boolean(await app.run('record.setField', { id, field: dateKey, value: day }));
  }

  function undo(): void {
    void app.undo();
  }
</script>

{#if failed}
  <!-- A read that failed states the fact (R19, D8) instead of drawing an empty
       month as if the plan had no assessments. -->
  <section class="cd-card" data-part="error">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="refresh" size={24} /></span>
      <div class="cd-empty__t">The month could not be read</div>
      <div class="cd-empty__s">The plan did not answer. Try again in a moment, or run SAM --configcheck.</div>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => void read()}>
        <Icon name="refresh" size={13} />
        Try again
      </button>
    </div>
  </section>
{:else}
  <Variant
  surface="mocks-calendar"
  {title}
  {loading}
  today={{ key: todayKey, spoken: spokenDay(todayKey) }}
  {anchor}
  {dated}
  {waiting}
  {quiet}
  {newCommand}
  newLabel={`New ${destination ? nameOf(destination.type).toLowerCase() : 'record'}`}
  onNew={openNew}
  onRead={() => void read()}
  onSetDay={setDay}
  onUndo={undo}
/>
{/if}
