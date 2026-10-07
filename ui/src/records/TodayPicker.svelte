<!--
  "Add something to today" (P2 · U1): the one-gesture door for the thin
  intention the read calls `committed`.

  It lists the plan's own work — the kinds the plan marks `trackable` — in plan
  order, with a filter over their names. Choosing one sets its `focus` date to
  the plan's today through the ordinary `record.setField` door, so the intention
  is a fact on the record, never a parallel list.

  Nothing here invents work: a thing that should exist and does not yet is the
  capture flow's job (F3), and this sheet says so with a link to that door.
-->
<script lang="ts">
  import Sheet from '../shell/Sheet.svelte';
  import Icon from '../shell/Icon.svelte';
  import { app } from '../session.svelte';
  import { nameOf, recordLabel, type RecordDoc } from '../types';

  let { onclose }: { onclose: () => void } = $props();

  /** The kinds a plan can hold intentions for: the ones it marks trackable. */
  const kinds = Object.entries(app.types)
    .filter(([, def]) => def.trackable === true)
    .map(([name]) => name);

  let query = $state('');
  let records = $state<Array<RecordDoc & { order: number }>>([]);
  let loaded = $state(false);
  let busy = $state(false);

  $effect(() => {
    // One read per kind, then one list. The engine's `records` read is the same
    // one the relation pickers use; the plan's weeks are read too, because plan
    // order is a week's own `index` — a record with no week has no plan order
    // and goes last rather than being given an invented one.
    let cancelled = false;
    void (async () => {
      const weeks = Object.keys(app.types).includes('week') ? ((await app.recordsOf('week')) ?? []) : [];
      const indexOfWeek = new Map<string, number>();
      for (const week of weeks) {
        const index = week.fields.index;
        if (typeof index === 'number') indexOfWeek.set(week.id, index);
      }
      const all: Array<RecordDoc & { order: number }> = [];
      for (const kind of kinds) {
        for (const record of (await app.recordsOf(kind)) ?? []) {
          const week = record.links.week?.[0];
          all.push({ ...record, order: week ? (indexOfWeek.get(week) ?? 9999) : 9999 });
        }
      }
      if (cancelled) return;
      all.sort(
        (left, right) =>
          left.order - right.order ||
          left.type.localeCompare(right.type) ||
          recordLabel(left).localeCompare(recordLabel(right)),
      );
      records = all;
      loaded = true;
    })();
    return () => {
      cancelled = true;
    };
  });

  const matches = $derived(
    query.trim().length === 0
      ? records
      : records.filter((record) => recordLabel(record).toLowerCase().includes(query.trim().toLowerCase())),
  );

  async function add(record: RecordDoc): Promise<void> {
    busy = true;
    try {
      await app.addToToday(record.id);
      app.notice(`planned ${recordLabel(record)} for today`);
      onclose();
    } finally {
      busy = false;
    }
  }

  async function capture(): Promise<void> {
    const kind = kinds[0];
    if (!kind) return;
    onclose();
    await app.openSheet({ kind: 'record.new', type: kind });
  }
</script>

<Sheet title="Plan something for today" subtitle="It will read under “planned for today” on the Today screen" {onclose}>
  <div class="cd-sheet__field">
    <label class="cd-sheet__label" for="today-filter">Find</label>
    <input
      id="today-filter"
      class="cd-sheet__well"
      type="text"
      placeholder="Start typing a name…"
      bind:value={query}
    />
  </div>

  {#if !loaded}
    <div class="cd-skel__rows" aria-busy="true">
      <p class="cd-sr" role="status">Reading the plan…</p>
      {#each [0, 1, 2] as row (row)}
        <div class="cd-skel__row">
          <span class="cd-skel"></span>
          <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 60 : 40}%`}></span>
          <span class="cd-skel"></span>
        </div>
      {/each}
    </div>
  {:else if matches.length === 0}
    <div class="cd-empty">
      <div class="cd-empty__t">Nothing matches “{query}”</div>
      <div class="cd-empty__s">Try another word — or add it as something new.</div>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => void capture()}>
        <Icon name="plus" size={13} />
        Add something new
      </button>
    </div>
  {:else}
    <div class="cd-tasks">
      {#each matches.slice(0, 60) as record (record.id)}
        <div class="cd-task" data-record-id={record.id}>
          <span class="cd-state" data-state="next" aria-hidden="true"></span>
          <div>
            <div class="cd-task__title">{recordLabel(record)}</div>
            <div class="cd-task__meta">
              <span class="cd-chip">{nameOf(record.type)}</span>
              {#if record.fields.kind}<span>{record.fields.kind}</span>{/if}
              {#if record.fields.est}<span>{record.fields.est} min</span>{/if}
              {#if record.order < 9999}<span>week {record.order}</span>{/if}
            </div>
          </div>
          <div class="cd-task__right">
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              data-command="record.setField"
              data-placement="today.screen"
              disabled={busy}
              aria-label={`Plan ${recordLabel(record)} for today`}
              onclick={() => void add(record)}
            >
              Add
            </button>
          </div>
        </div>
      {/each}
    </div>
    {#if matches.length > 60}
      <p class="cd-sheet__hint">Showing the first 60 of {matches.length} — type to narrow it down.</p>
    {/if}
  {/if}

  {#snippet footer()}
    <span class="cd-sheet__hint">Adding sets the record's “Planned for” date to today. Nothing is moved or deleted.</span>
    <span class="cd-sheet__spacer"></span>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Close</button>
  {/snippet}
</Sheet>
