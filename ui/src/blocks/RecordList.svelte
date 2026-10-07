<!--
  `list` and `cardGrid` view block renderers (§3.6).

  Renders record collections in card list or grid tile layouts:
    - `list`: Card list presentation delegating to variants (`ui/src/variants/problemsets-due/`).
    - `cardGrid`: Compact grid tiles with editable field controls.
    - Actions: Exposes standard row context menus (`rowMenuIds()`: JSON export, path copying, deletion).
-->
  <script lang="ts">
    import FieldControl from '../records/FieldControl.svelte';
    import Menu from '../shell/Menu.svelte';
    import Variant from '../variants/Variant.svelte';
    import BlockHead from './BlockHead.svelte';
    import { COPY_JSON, COPY_PATH, rowMenuIds, type MenuRow } from '../commands/registry';
    import { app } from '../session.svelte';
    import {
      dayMonth,
      rowsOf,
      shiftDay,
      type ListCardRow,
    } from '../variants/problemsets-due/props';
    import {
      blockRows,
      labelOf,
      nameOf,
      recordLabel,
      type BlockNode,
      type FieldRead,
      type RecordDoc,
    } from '../types';

    let {
      node,
      targetsFor,
    }: {
      node: BlockNode;
      targetsFor: (field: FieldRead) => Array<{ id: string; label: string }>;
    } = $props();

    const type = $derived(app.types[node.type ?? '']);
    const rows = $derived(blockRows(node));
    const cards = $derived(node.kind === 'cardGrid');
    const today = $derived(app.today?.date ?? null);

    /**
     * The record's own day: its first `date`/`daterange` field, found through the
     * schema and never guessed from a value's shape.
     */
    const dateKey = $derived(
      (type?.fields ?? []).find(
        (field) => field.type === 'date' || field.type === 'daterange',
      )?.key ?? null,
    );

    /** The relation a record's course is named by — the plan's own, never a guess. */
    const courseField = $derived(
      (type?.fields ?? []).find(
        (field) => field.type === 'relation' && (field.to ?? '') === 'course',
      ) ??
        (type?.fields ?? []).find((field) => field.type === 'relation') ??
        null,
    );

    /** The course labels the app already resolves for that relation (C.1). */
    const courseTargets = $derived(courseField ? targetsFor(courseField) : []);

    /** A record's own planned day, as the store holds it. */
    function dayStore(record: RecordDoc): string | null {
      const value = dateKey === null ? null : (record.fields[dateKey] ?? record.derived?.[dateKey]);
      if (typeof value === 'string' && value.length >= 10) return value.slice(0, 10);
      return null;
    }

    /** The course a record points at: the app's own label and the plan's own wash. */
    function courseOf(record: RecordDoc): { code: string; wash: string | null } | null {
      const id = courseField === null ? null : (record.links?.[courseField.key]?.[0] ?? null);
      if (id === null) return null;
      const named = courseTargets.find((target) => target.id === id);
      return { code: named?.label ?? id, wash: app.washes[id] ?? null };
    }

    /** What the list card draws: one row per record, with the app's own words. */
    const listRows = $derived(
      rowsOf(
        rows.map((record) => ({
          id: record.id,
          type: record.type,
          label: recordLabel(record),
          course: courseOf(record),
          day: dayStore(record),
          record,
        })),
        today,
      ),
    );

    /**
     * The facts a tile shows: the type's own field order, and only the kinds that
     * read as one short fact. A tile is a keyhole onto the record — the table is
     * where every column lives.
     */
    const FACTS = ['select', 'duration', 'number', 'date', 'rating', 'formula'];
    const facts = $derived((type?.fields ?? []).filter((field) => FACTS.includes(field.type)).slice(0, 3));

    const sections = $derived(
      node.grouped && node.groups
        ? node.groups.map((group) => ({ key: group.key, records: group.records }))
        : [{ key: null as string | null, records: rows }],
    );

    let menuFor = $state<string | null>(null);
    let moveFor = $state<string | null>(null);

    async function copy(text: string, what: string): Promise<void> {
      try {
        await navigator.clipboard.writeText(text);
        app.notice(`copied ${what}`);
      } catch {
        app.notice(`cannot copy ${what} — the clipboard is unavailable here`);
      }
    }

    /** One write from this card, and the receipt it leaves: the student's words,
     *  with the action that reverses it (F14). */
    async function written(id: string, params: Record<string, unknown>, line: string): Promise<void> {
      const result = await app.run(id, params);
      if (result) app.notice(line, { label: 'Undo', run: () => void app.undo() });
    }

    function rowRows(record: RecordDoc): MenuRow[] {
      return rowMenuIds().map((id): MenuRow => {
        switch (id) {
          case COPY_JSON:
            return {
              id,
              title: 'Copy as JSON',
              run: () => copy(JSON.stringify(record), `the ${record.id} line`),
            };
          case COPY_PATH:
            return {
              id,
              title: 'Copy path',
              run: () => copy(`content/records/${record.type}.jsonl#${record.id}`, 'the path'),
            };
          case 'record.reveal':
            return { id, title: 'Reveal in source pane', run: () => void app.reveal(record) };
          case 'record.move':
            return { id, title: 'Move to…', run: () => (moveFor = record.id) };
          default:
            return {
              id,
              title: 'Delete…',
              danger: true,
              run: () => void app.openSheet({ kind: 'record.delete', record }),
            };
        }
      });
    }

    /** The types this record can move to — the plan's own, never a fixed list. */
    function moveRows(record: RecordDoc): MenuRow[] {
      return Object.keys(app.types)
        .filter((name) => name !== record.type)
        .map(
          (name): MenuRow => ({
            id: 'record.move',
            title: nameOf(name),
            run: () => void app.run('record.move', { id: record.id, to: name }),
          }),
        );
    }

    /** The record's own door: the app's detail panel, which is a view state. */
    function openPanel(row: ListCardRow): void {
      app.detail = { id: row.id, type: row.type };
      app.selection = row.id;
    }

    /** C's `Later`: the record's own planned day moved on by a week. */
    function defer(row: ListCardRow): void {
      const from = row.day ?? today;
      if (from === null) return;
      const date = shiftDay(from, 7);
      void written('record.defer', { ids: [row.id], date, field: 'focus' }, `Planned for ${dayMonth(date)}`);
    }
  </script>

  {#if !cards}
    <!-- The list card: the design owns the card, the head and the drawing; this
         file owns the facts and the app's own writes. The block carries its own
         identity (`data-block`), which is what the UI check walks. -->
    <div class="blk" data-block={node.kind}>
      <Variant
        surface="problemsets-due"
        title={node.title ?? nameOf(node.view ?? '')}
        view={node.view ?? ''}
        type={node.type ?? ''}
        rows={listRows}
        candidates={node.candidates ?? rows.length}
        {today}
        selected={app.selection}
        {menuFor}
        menuRows={rowRows}
        onSelect={(id) => (app.selection = id)}
        onPanel={openPanel}
        onMenu={(id) => (menuFor = id)}
        onNew={() => void app.openSheet({ kind: 'record.new', type: node.type ?? '' })}
        onPaste={() => void app.openSheet({ kind: 'paste', type: node.type ?? '' })}
        onDone={(row) =>
          void written('record.advanceStage', { id: row.id, stage: 'done' }, `Finished ${row.label}`)}
        onLater={defer}
      />
    </div>
{:else}
  <section class="cd-card blk" data-block="cardGrid" data-view={node.view} data-type={node.type}>
    <BlockHead
      title={node.title ?? nameOf(node.view ?? '')}
      type={node.type ?? ''}
      word="Cards"
      returned={rows.length}
      candidates={node.candidates ?? rows.length}
      onEdit={() => void app.run('view.edit', { name: node.view ?? app.selected ?? '' })}
      onNew={() => void app.openSheet({ kind: 'record.new', type: node.type ?? '' })}
    />

    {#if rows.length === 0}
      <p class="cd-empty"><span class="cd-empty__s">
        {#if (node.candidates ?? 0) > 0}
          Nothing matches this view — {node.candidates} {node.type} exist.
        {:else}
          No records yet — nothing has been added to this kind.
        {/if}
      </span></p>
    {:else}
      {#each sections as section (section.key ?? '—')}
        {#if section.key !== null}
          <p class="blk__group">{section.key}</p>
        {/if}
        <div class="cd-tiles">
          {#each section.records as record (record.id)}
            <div class="cd-tile" data-record-id={record.id} data-placement="recordTable.cell">
              <span class="blk__minititle">{recordLabel(record)}</span>
              {#each facts as field (field.key)}
                <span class="blk__minisub">
                  {labelOf(field, field.key)}
                  <FieldControl
                    {field}
                    {record}
                    targets={field.type === 'relation' ? targetsFor(field) : []}
                    oncommit={(fieldKey, value) => app.setField(record, fieldKey, value)}
                  />
                </span>
              {/each}
              <span class="cd-tile__foot">
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  data-command="record.reveal"
                  onclick={() => void app.reveal(record)}
                >
                  Reveal
                </button>
                <span class="cd-card__spacer"></span>
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  aria-haspopup="menu"
                  aria-expanded={menuFor === record.id}
                  data-command="record.delete"
                  title="More for this record"
                  aria-label={`More for ${recordLabel(record)}`}
                  onclick={() => (menuFor = menuFor === record.id ? null : record.id)}
                >
                  ⋯
                </button>
              </span>
              {#if menuFor === record.id}
                <Menu rows={rowRows(record)} native={false} onclose={() => (menuFor = null)} />
              {/if}
              {#if moveFor === record.id}
                <Menu
                  rows={moveRows(record)}
                  native={false}
                  onclose={() => {
                    moveFor = null;
                    menuFor = null;
                  }}
                />
              {/if}
            </div>
          {/each}
        </div>
      {/each}
      <!--
        What this list is, in words, for someone who cannot see it: it says the
        one thing the rows do not — how many there are. The identity of each row
        is the row's own name, which is where it belongs.
      -->
      <p class="cd-sr" role="status">
        {rows.length} {rows.length === 1 ? 'entry' : 'entries'} in this list.
      </p>
    {/if}
  </section>
{/if}
