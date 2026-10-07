<!--
  REFERENCE-LIBRARY widget host (`COMPOSER.md` §3.1).
  Mounts the active reference library variant within composed screens,
  resolving library and note records with filter query state.
-->
<script lang="ts">
  import Variant from '../../../variants/Variant.svelte';
  import { app } from '../../../session.svelte';
  import { nameOf, recordLabel, type RecordDoc } from '../../../types';
  import { readPlanModel, type PlanModel } from '../../../panels/model';
  import { safeUrl } from '../../../safeUrl';
  import {
    kindFacts,
    matchesOf,
    type LibraryCourse,
    type LibraryRecord,
    type LibrarySegment,
  } from '../../../variants/reference-library/props';

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

  /** This widget's home view, found by panel name — the plan's own view whose
   *  navigation title names the destination. */
  const homeView = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'library')?.[0] ??
      Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'notes')?.[0] ??
      null,
  );
  const title = $derived(
    app.navigation.find((entry) => entry.view === homeView)?.title ?? nameOf(homeView ?? ''),
  );
  /** The head's own title: the destination being read, which follows the
   *  segment when it is switched. */
  const heading = $derived(active?.label ?? title);

  /** The record door: the app's own `record.panel` for one saved thing. */
  function open(id: string): void {
    const record = model?.byId.get(id);
    if (record) void app.run('record.panel', { id, type: record.type });
  }

  function add(): void {
    if (active) void app.openSheet({ kind: 'record.new', type: active.type });
  }

  const libraryProps = $derived({
    heading,
    segments,
    segment: active?.type ?? null,
    courses,
    records,
    matches,
    query,
    newCommand: active ? `${active.type}.new` : null,
    onSegment: (type: string) => {
      segment = type;
      query = '';
    },
    onQuery: (value: string) => (query = value),
    onClear: () => (query = ''),
    onAdd: add,
    onOpen: open,
  });
</script>

<Variant surface="reference-library" {...libraryProps} />
