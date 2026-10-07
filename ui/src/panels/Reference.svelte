<!--
  Reference library and notes panel (S7, UI P2 · U2, D1, UI_PLAN D6).

  Combines reference material (`library`, `notes`) into a unified filtered view:
    - Kind segmentation: Uses segmented controls when multiple kinds exist, omitting the control when only one kind is declared.
    - Labels: Resolves display names dynamically from plan navigation metadata.
    - Presentation: Delegates rendering to variants (`ui/src/variants/reference-library/`).
    - Reactivity: Guarded by `loadedFor` to prevent redundant fetches.
-->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import Variant from '../variants/Variant.svelte';
  import { app } from '../session.svelte';
  import { nameOf, recordLabel, type RecordDoc } from '../types';
  import { safeUrl } from '../safeUrl';
  import { readPlanModel, type PlanModel } from './model';
  import {
    kindFacts,
    matchesOf,
    type LibraryCourse,
    type LibraryRecord,
    type LibrarySegment,
  } from '../variants/reference-library/props';

  let { title }: { title: string } = $props();

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
   * The two reference destinations, resolved from the plan's own navigation
   * rather than from a list written here: for each of the two panels the product
   * names, find the view that renders it, take the kind it reads, and label it
   * with that destination's own navigation title. A kind with no records is not
   * offered — a segment option that is always empty is a dead control.
   */
  const segments = $derived.by<LibrarySegment[]>(() => {
    const held: LibrarySegment[] = [];
    for (const panel of ['library', 'notes']) {
      const entry = Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === panel);
      if (!entry) continue;
      const kind = entry[1].type;
      if (!(model?.other ?? []).some((record) => record.type === kind)) continue;
      held.push({
        type: kind,
        label: app.navigation.find((item) => item.view === entry[0])?.title ?? nameOf(kind) + 's',
      });
    }
    return held;
  });

  /** The segment the student arrived on: the route they opened, else the first
   *  kind the plan actually holds. */
  let segment = $state<string | null>(null);
  const active = $derived(
    segments.find((item) => item.type === segment) ??
      segments.find((item) => item.type === app.outcome?.type) ??
      segments[0] ??
      null,
  );

  /** A course's own code, which is what a student says out loud (D2). */
  function codeOf(record: RecordDoc): string | null {
    const value = record.fields.code ?? record.fields.short;
    return typeof value === 'string' && value.length > 0 ? value : null;
  }

  /** The plan's own courses, in its own order: C's folders are its sleeves, and
   *  a record's filing is named with its code. */
  const courses = $derived<LibraryCourse[]>(
    (model?.identities ?? []).map((record) => ({
      id: record.id,
      code: codeOf(record),
      name: recordLabel(record),
    })),
  );

  /**
   * Every record of the kind being looked at, as a design draws it. The facts a
   * design is allowed are the plan's own: the record's label, the word its kind
   * declares, its provider, the first course it is filed under, its link, and
   * the note's own text. Nothing here computes a fact the engine owns.
   */
  const records = $derived.by<LibraryRecord[]>(() => {
    if (active === null) return [];
    const held = (model?.other ?? []) as RecordDoc[];
    return held
      .filter((record) => record.type === active.type)
      .map((record) => {
        const declared = record.fields.kind;
        const provider = record.fields.provider;
        const body = record.fields.body ?? record.fields.note ?? record.fields.summary;
        const url = record.fields.url;
        const courseId = (record.links?.course ?? [])[0] ?? null;
        return {
          id: record.id,
          type: record.type,
          title: recordLabel(record),
          ...kindFacts(
            record.type,
            typeof declared === 'string' && declared.length > 0 ? declared : null,
            nameOf(record.type),
            app.types[record.type]?.icon ?? 'trayFull',
          ),
          provider: typeof provider === 'string' && provider.length > 0 ? provider : null,
          course: courses.find((entry) => entry.id === courseId) ?? null,
          url: typeof url === 'string' && safeUrl(url) ? url : null,
          body: typeof body === 'string' && body.length > 0 ? body : null,
        };
      });
  });

  let query = $state('');
  const matches = $derived(matchesOf(records, query));

  const noun = $derived(active === null ? 'record' : nameOf(active.type).toLowerCase());
  const plural = $derived(noun === 'note' ? 'notes' : `${noun}s`);
  /** The head's own title: the destination the student is reading, which follows
   *  the segment when it is switched. */
  const heading = $derived(active?.label ?? title);

  /** The record door: the app's own `record.panel` for one saved thing. */
  function open(id: string): void {
    const record = model?.byId.get(id);
    if (record) void app.run('record.panel', { id, type: record.type });
  }

  function add(): void {
    if (active) void app.openSheet({ kind: 'record.new', type: active.type });
  }
</script>

{#if !model}
  <!-- Loading borrows the arriving geometry: a reference row is the collection
       layer's 57px, not a 193px card. -->
  <section class="cd-card" aria-busy="true">
    <p class="cd-sr" role="status">Reading the list…</p>
    <div class="cd-skel__rows">
      {#each [0, 1, 2, 3] as row (row)}
        <div class="cd-skel__row cd-skel__row--coll">
          <span class="cd-skel"></span>
          <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 62 : 44}%`}></span>
          <span class="cd-skel"></span>
        </div>
      {/each}
    </div>
  </section>
{:else if active === null}
  <!-- A plan that holds neither kind. One statement, and it says what is
       missing rather than apologising for the screen. -->
  <section class="cd-card">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="trayFull" size={24} /></span>
      <div class="cd-empty__t">This plan keeps nothing to look up</div>
      <div class="cd-empty__s">
        Notes and saved links are the two kinds this screen reads, and the plan has neither.
      </div>
    </div>
  </section>
{:else if records.length === 0}
  <!-- NOTHING HERE YET. A drawn mark in the kind's own glyph, a title, one
       sentence, and ONE escape hatch that does something real — it creates the
       first one. -->
  <section class="cd-card">
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name={app.types[active.type]?.icon ?? 'trayFull'} size={24} /></span>
      <div class="cd-empty__t">No {plural} yet</div>
      <div class="cd-empty__s">
        {active.type === 'note'
          ? 'Notes you write down show up here — a thought, a formula, a mistake worth keeping.'
          : 'Links and books you collect for this plan show up here.'}
      </div>
      <button
        class="cd-pill cd-pill--ghost cd-pill--sm"
        type="button"
        data-command={`${active.type}.new`}
        data-placement="today.screen"
        onclick={add}
      >
        <Icon name="plus" size={13} />
        Add a {noun}
      </button>
    </div>
  </section>
{:else}
  <Variant
    surface="reference-library"
    {heading}
    {segments}
    segment={active.type}
    {courses}
    {records}
    {matches}
    {query}
    newCommand={`${active.type}.new`}
    onSegment={(type) => {
      segment = type;
      query = '';
    }}
    onQuery={(value) => (query = value)}
    onClear={() => (query = '')}
    onAdd={add}
    onOpen={open}
  />
{/if}
