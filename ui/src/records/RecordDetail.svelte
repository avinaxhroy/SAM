<!--
  Record detail side pane (P2 · U5, UI_SPEC §2.2).

  Displays record attributes, relationships, and history in a 360px side pane:
    - Data resolution: Loads record data via `app.recordsOf`, resolves relation labels and parent edges (`model.rs`),
      and computes review state from `review.last` and `review.log`.
    - Presentation: Delegates rendering to variants (`ui/src/variants/record-panel/`).
    - Standard header: Displays record label, type title, and close control (`styles/detail-pane.css`).
-->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import { app } from '../session.svelte';
  import { labelOf, nameOf, recordLabel, type FieldRead, type RecordDoc } from '../types';
  import Variant from '../variants/Variant.svelte';
  import {
    QUANTITY_TYPES,
    amountOf,
    dayDistance,
    isDerived,
    ratingWord,
    readingOf,
    stageWord,
    type FieldFact,
    type HistoryFact,
    type LinkFact,
    type LoggedFact,
    type RungFact,
    type RungState,
  } from '../variants/record-panel/props';
  // The panel's own sheet: the species the design system does not ship
  // (a third column with its own scroll, and the three designs' objects).
  import '../styles/detail-pane.css';

  /** One review-queue entry, as `reviews.due` returns it for a record. */
  type ReviewEntry = {
    id: string;
    pipeline?: string;
    stages?: string[];
    next?: string | null;
    due?: string | null;
    overdueDays?: number;
    complete?: boolean;
    asks?: Array<{ key: string; label?: string | null; stage?: string | null }>;
    /** The review's own object (`Review::to_json`) — `last`, `due`,
     *  `intervalDays` and the log of ratings are the facts “last reviewed” and
     *  “next review” are made of. */
    review?: {
      last?: string | null;
      anchor?: string | null;
      due?: string | null;
      intervalDays?: number;
      reps?: number;
      log?: Array<{ at?: string; rating?: string }>;
    } | null;
  };

  type QueueRead = {
    due?: ReviewEntry[];
    waiting?: ReviewEntry[];
    upcoming?: ReviewEntry[];
    pipelines?: Record<string, { stages?: string[] }>;
  };

  /** What every design is handed. One read, one contract. */
  type Facts = {
    kind: string;
    address: string;
    quantities: FieldFact[];
    dates: FieldFact[];
    links: LinkFact[];
    history: HistoryFact | null;
    logged: LoggedFact;
    /** The ISO day the record is planned for, when a date column says so. */
    planned: string | null;
  };

  let record = $state<RecordDoc | null>(null);
  let facts = $state<Facts | null>(null);
  /** The read's own guard — deliberately not `$state`: it decides whether a
   *  read runs, and a reactive guard re-enters the effect that writes it. */
  const loaded = { key: null as string | null };

  const fields = $derived(
    (app.detail ? (app.types[app.detail.type]?.fields ?? []) : []) as FieldRead[],
  );

  /** The column the kind calls its plan date, in the app's own vocabulary; a
   *  kind that names it otherwise still gets its first date column. */
  function plannedKeyOf(declared: FieldRead[]): string | null {
    const named = declared.find((field) => field.type === 'date' && field.key === 'focus');
    return (named ?? declared.find((field) => field.type === 'date'))?.key ?? null;
  }

  function factOf(field: FieldRead, found: RecordDoc, today: string | null): FieldFact {
    const raw = found.fields[field.key];
    return {
      key: field.key,
      label: labelOf(field, field.key),
      type: field.type,
      reading: readingOf(field, found, today),
      amount: amountOf(field, found),
      // The editor's own starting value: a number as its digits, a day as the
      // engine's date. Neither reaches the page as prose.
      raw: raw === null || raw === undefined ? '' : String(raw),
      options: field.options ?? [],
      derived: isDerived(field),
    };
  }

  /** The record's relations: every declared relation field, **plus its parent
   *  edge** (which `fields` cannot carry — `model.rs:195-208`). */
  async function linksOf(found: RecordDoc, declared: FieldRead[]): Promise<LinkFact[]> {
    const parent = app.detail ? (app.types[app.detail.type]?.parent ?? null) : null;
    const keys: Array<{ key: string; label: string; to: string }> = declared
      .filter((field) => field.type === 'relation' && field.to)
      .map((field) => ({ key: field.key, label: labelOf(field, field.key), to: field.to as string }));
    if (parent && !keys.some((entry) => entry.key === parent)) {
      keys.push({ key: parent, label: nameOf(parent), to: parent });
    }
    const out: LinkFact[] = [];
    for (const entry of keys) {
      const ids = found.links?.[entry.key] ?? [];
      if (ids.length === 0) continue;
      const targets = await app.recordsOf(entry.to);
      out.push({
        ...entry,
        targets: ids.map((id) => {
          const match = targets.find((candidate) => candidate.id === id);
          const code = match?.fields?.code;
          return {
            id,
            label: match ? recordLabel(match) : '',
            code: typeof code === 'string' && code.length > 0 ? code : null,
          };
        }),
      });
    }
    return out;
  }

  /** The ladder, its states and the review's own dates — never a raw key. */
  function historyOf(entry: ReviewEntry | null, ladder: string[]): HistoryFact | null {
    if (!entry) return null;
    const review = entry.review ?? null;
    const log = review?.log ?? [];
    const passed = entry.stages ?? [];
    const rungs: RungFact[] = ladder.map((name) => {
      const state: RungState = passed.includes(name) ? 'done' : entry.next === name ? 'now' : 'todo';
      return { name, state, word: stageWord(state) };
    });
    const first = log.reduce<string | null>(
      (earliest, item) => (item.at && (earliest === null || item.at < earliest) ? item.at : earliest),
      null,
    );
    const last = log.length > 0 ? log[log.length - 1] : null;
    return {
      first,
      reviewed: review?.last ?? null,
      rating: ratingWord(last?.rating ?? null),
      due: review?.due ?? entry.due ?? null,
      late: entry.overdueDays ?? 0,
      complete: entry.complete === true,
      asks: (entry.asks ?? []).map((ask) => (ask.label && ask.label.length > 0 ? ask.label : ask.key)),
      intervalDays: review?.intervalDays ?? null,
      rungs,
    };
  }

  /** The minutes the plan logged for this record: the `session` records that
   *  link its course inside its own interval — a derived sum, never written. */
  async function loggedOf(
    links: LinkFact[],
    history: HistoryFact | null,
    planned: string | null,
  ): Promise<LoggedFact> {
    const courses = links
      .filter((link) => link.key === 'course' || link.to === 'course')
      .flatMap((link) => link.targets.map((target) => target.id));
    if (courses.length === 0) return { minutes: 0, on: null };
    const sessions = await app.recordsOf('session');
    const from = history?.first?.slice(0, 10) ?? null;
    const head = from && planned ? dayDistance(from, planned) : null;
    let total = 0;
    let on: string | null = null;
    for (const session of sessions) {
      if (!(session.links?.course ?? []).some((id) => courses.includes(id))) continue;
      const when = typeof session.fields?.date === 'string' ? session.fields.date : null;
      // Only an interval the record actually states narrows the sum: a record
      // with no logged history and no plan date counts every session it links.
      if (from && when && head !== null) {
        const at = dayDistance(from, when);
        if (at === null || at < 0 || at > head) continue;
      }
      const value = Number(session.fields?.min);
      if (Number.isFinite(value)) total += value;
      if (when) on = when;
    }
    return { minutes: total, on };
  }

  /** One `record.setField`, exactly as the table's cells dispatch it; a refusal
   *  is the engine's own sentence, handed back to the design that asked. */
  async function setField(key: string, value: string | string[] | null): Promise<string | null> {
    const target = record;
    if (!target) return 'no record is open';
    const result = await app.run('record.setField', { id: target.id, field: key, value });
    if (result) return null;
    const reason = app.lastDiagnostic ?? 'the engine refused the change';
    // The panel states it where the edit happened, so the toast is cleared —
    // the same rule the table's own cells follow (F14).
    app.toast = null;
    return reason;
  }

  $effect(() => {
    const target = app.detail;
    const revision = app.revision;
    const key = target ? `${target.type}:${target.id}:${revision}` : null;
    if (!target || key === loaded.key) return;
    loaded.key = key;
    void (async () => {
      const declared = (app.types[target.type]?.fields ?? []) as FieldRead[];
      const today = app.today?.date ?? null;
      const found =
        (await app.recordsOf(target.type)).find((candidate) => candidate.id === target.id) ?? null;
      if (!found) {
        record = null;
        facts = null;
        return;
      }
      const links = await linksOf(found, declared);
      const data = (await app.run('reviews.due', {}, { tracked: false }))?.data as QueueRead | undefined;
      const entry =
        [...(data?.due ?? []), ...(data?.waiting ?? []), ...(data?.upcoming ?? [])].find(
          (candidate) => candidate.id === target.id,
        ) ?? null;
      const ladder = entry?.pipeline ? (data?.pipelines?.[entry.pipeline]?.stages ?? []) : [];
      const history = historyOf(entry, ladder);
      const plannedKey = plannedKeyOf(declared);
      const plannedValue = plannedKey ? found.fields[plannedKey] : null;
      const planned = typeof plannedValue === 'string' && plannedValue.length > 0 ? plannedValue : null;
      const logged = await loggedOf(links, history, planned);
      if (loaded.key !== key) return;
      record = found;
      facts = {
        kind: nameOf(found.type),
        address: `content/records/${found.type}.jsonl#${found.id}`,
        quantities: declared
          .filter((field) => QUANTITY_TYPES[field.type] === true)
          .map((field) => factOf(field, found, today)),
        dates: declared.filter((field) => field.type === 'date').map((field) => factOf(field, found, today)),
        links,
        history,
        logged,
        planned,
      };
    })();
  });

  /** A relation's door, exactly as the app's own `openPanel` moves the panel. */
  function openPanel(id: string, type: string): void {
    app.detail = { id, type };
    app.selection = id;
  }
</script>

<aside class="cd-detailpane" aria-label="Record details" data-panel="record">
  <header class="cd-detailpane__head">
    <div>
      <h2 class="cd-detailpane__title">{record ? recordLabel(record) : 'Reading…'}</h2>
      {#if app.detail}<p class="cd-detailpane__sub">{nameOf(app.detail.type)}</p>{/if}
    </div>
    <button
      class="cd-iconbtn"
      type="button"
      aria-label="Close the detail panel"
      onclick={() => (app.detail = null)}
    >
      <Icon name="close" />
    </button>
  </header>

  {#if !record}
    {#if loaded.key && app.detail && !app.busy}
      <!-- The record is not in the plan any more: a stated fact, not a skeleton
           that never arrives (D8). -->
      <p class="cd-detailpane__note">
        This record is not in the plan any more. It may have been deleted, or moved to another kind.
      </p>
    {:else}
      <div class="cd-skel__rows" aria-busy="true">
        <p class="cd-sr" role="status">Reading the record…</p>
        {#each [0, 1, 2, 3] as row (row)}
          <div class="cd-skel__row">
            <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 52 : 34}%`}></span>
            <span class="cd-skel"></span>
          </div>
        {/each}
      </div>
    {/if}
  {:else if facts}
    <Variant
      surface="record-panel"
      kind={facts.kind}
      address={facts.address}
      today={app.today?.date ?? null}
      quantities={facts.quantities}
      dates={facts.dates}
      links={facts.links}
      history={facts.history}
      logged={facts.logged}
      planned={facts.planned}
      onSet={setField}
      onOpen={openPanel}
      onReveal={() => void app.reveal(record!)}
      onDelete={() => void app.openSheet({ kind: 'record.delete', record: record! })}
    />
  {/if}
</aside>
