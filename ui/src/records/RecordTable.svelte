<!-- Table view for record inspection and editing across variants A, B, and C (R8, Phase 2, U5). -->
  <script lang="ts">
    import Variant from '../variants/Variant.svelte';
    import { COPY_JSON, COPY_PATH, columnPlusIds, rowMenuIds, type MenuRow } from '../commands/registry';
    import {
      durationText,
      labelOf,
      nameOf,
      recordLabel,
      rowsOf,
      type FieldRead,
      type RecordDoc,
      type TypeRead,
      type ViewRead,
    } from '../types';
    import { app } from '../session.svelte';
    import {
      dayWords,
      daysBetween,
      flexKeyOf,
      type CellFact,
      type CellWrite,
      type ColumnFact,
      type InstantFact,
      type RecordTableProps,
      type RowFact,
    } from '../variants/record-table/props';

    let {
      outcome,
      type,
      columns,
      targetsFor,
      title = null,
      /** Block layout hint (`table` | `list`). */
      variant = 'table',
    }: {
      outcome: ViewRead;
      type: TypeRead | undefined;
      columns: string[];
      /** The records a relation field points at, resolved per field (C.1). */
      targetsFor: (field: FieldRead) => Array<{ id: string; label: string }>;
      /** A block's own heading, when the table is one block of a screen. */
      title?: string | null;
      variant?: 'table' | 'list';
    } = $props();

    const rows = $derived(rowsOf(outcome));

    /** Group advisory warnings by diagnostic code for summary presentation (D2). */
    const findings = $derived.by(() => {
      const byCode = new Map<string, { code: string; count: number; detail: string }>();
      for (const warning of outcome.warnings ?? []) {
        const seen = byCode.get(warning.code);
        if (seen) {
          seen.count += 1;
          continue;
        }
        byCode.set(warning.code, {
          code: warning.code,
          count: 1,
          detail: `${warning.path}: ${warning.message}`,
        });
      }
      return [...byCode.values()];
    });

    let dragging = $state<string | null>(null);
    let dropTarget = $state<string | null>(null);
    let plusOpen = $state(false);
    let rowMenuFor = $state<string | null>(null);
    let moveFor = $state<string | null>(null);
    let cells = $state<Record<string, CellWrite>>({});
    let inFlight = $state(0);

    function fieldOf(key: string): FieldRead | undefined {
      return type?.fields.find((field) => field.key === key);
    }

    const hidden = $derived(
      (type?.fields ?? []).map((field) => field.key).filter((key) => !columns.includes(key)),
    );

    const today = $derived(app.today?.date ?? null);

    /**
     * The records a cell can point at, read once per relation column rather than
     * once per cell: eight rows of one relation is one read of the same list.
     */
    const relationTargets = $derived.by(() => {
      const found = new Map<string, Array<{ id: string; label: string }>>();
      for (const key of columns) {
        const field = fieldOf(key);
        if (field?.type === 'relation') found.set(key, targetsFor(field));
      }
      return found;
    });

    /** Resolves active date field: checks visible date columns first, then fallback list. */
    const INSTANT_KEYS = ['focus', 'due', 'dueAt', 'deadline', 'date', 'start', 'lastAttempt'];
    const LATE_KEYS = ['focus', 'due', 'dueAt', 'deadline'];
    const instantField = $derived.by<FieldRead | null>(() => {
      const shown = columns.map((key) => fieldOf(key)).find((field) => field?.type === 'date');
      if (shown) return shown;
      const dates = (type?.fields ?? []).filter((field) => field.type === 'date');
      return (
        INSTANT_KEYS.map((key) => dates.find((field) => field.key === key)).find(
          (field) => field !== undefined,
        ) ??
        dates[0] ??
        null
      );
    });

    /**
     * One write from a control in this card, and the receipt it leaves: what
     * changed, in the student's words, with the action that reverses it (F14).
     * The sentence is per command, so a receipt never prints a registry id.
     */
    async function written(id: string, params: Record<string, unknown>, line: string): Promise<void> {
      const result = await app.run(id, params);
      if (result) app.notice(line, { label: 'Undo', run: () => void app.undo() });
    }

    /**
     * The detail panel's switch — the one presentation surface this file opens.
     *
     * It goes through the session's own field rather than `app.run('record.panel',
     * …)` because the panel is a *view state* the frame renders (`App.svelte`), and
     * the row's click must not also become a palette/menu entry: the command id
     * exists in the registry (`record.panel`, presentation) for the other doors,
     * and every door reaches the same panel.
     */
    function openPanel(id: string, kind: string): void {
      app.detail = { id, type: kind };
      app.selection = id;
    }

    /**
     * The commit a drop performs: one dispatch that writes this view's column
     * order (`view.setColumns`), which is the list the header draws — dropping a
     * column onto another puts it in front of that column, and the end zone puts
     * it last. A block whose columns come from the block itself has no view to
     * write, so its heads do not drag.
     */
    function dropOn(before: string): void {
      const dragged = dragging;
      dragging = null;
      dropTarget = null;
      if (!dragged || dragged === before || !outcome.view) return;
      const order = columns.filter((key) => key !== dragged);
      const at = before === '' ? order.length : order.indexOf(before);
      order.splice(at < 0 ? order.length : at, 0, dragged);
      if (order.join(',') === columns.join(',')) return;
      void written('view.setColumns', { name: outcome.view, columns: order.join(',') }, 'Moved the column');
    }

    /**
     * The one write a cell performs (§4.8's one dispatch path), with its states
     * kept at the cell: `saving` while the engine has it, `saved` for a moment
     * after, and on refusal the engine's own reason — rendered under the row, not
     * floated in a toast nobody can act on.
     */
    async function commitCell(record: RecordDoc, key: string, value: string | string[] | null): Promise<void> {
      const at = `${record.id}:${key}`;
      cells = { ...cells, [at]: { state: 'saving' } };
      inFlight += 1;
      try {
        const result = await app.run('record.setField', { id: record.id, field: key, value });
        if (result) {
          cells = { ...cells, [at]: { state: 'saved' } };
          setTimeout(() => {
            if (cells[at]?.state === 'saved') {
              const next = { ...cells };
              delete next[at];
              cells = next;
            }
          }, 1400);
          return;
        }
        // The reason lives in `lastDiagnostic`; the toast is cleared because this
        // row already says the same thing, where the edit happened (F14).
        const reason = app.lastDiagnostic ?? 'the engine refused the change';
        app.toast = null;
        cells = { ...cells, [at]: { state: 'failed', reason } };
      } finally {
        inFlight -= 1;
      }
    }

    /** The failures of one row, in the engine's words, under that row. */
    function failures(record: RecordDoc): string[] {
      return Object.entries(cells)
        .filter(([at, wrote]) => wrote.state === 'failed' && at.startsWith(`${record.id}:`))
        .map(([, wrote]) => wrote.reason ?? 'the engine refused the change');
    }

    /** A cell's state chip: the write's own progress, at the cell it lands in. */
    function stateOf(record: RecordDoc, key: string): CellWrite | null {
      return cells[`${record.id}:${key}`] ?? null;
    }

    /** Formats a cell value for display and accessibility sentences (mirrors `FieldControl`). */
    function readingOf(record: RecordDoc, field: FieldRead): string {
      const derived = field.type === 'formula' || field.type === 'progress';
      if (field.type === 'relation') {
        const links = record.links?.[field.key] ?? [];
        if (links.length === 0) return '—';
        const known = relationTargets.get(field.key) ?? [];
        return links.map((id) => known.find((target) => target.id === id)?.label ?? id).join(', ');
      }
      const value = derived ? record.derived?.[field.key] : record.fields[field.key];
      if (value === undefined || value === null || value === '') return derived ? '—' : '';
      if (field.type === 'duration') {
        const minutes = Number(value);
        if (!Number.isFinite(minutes) || minutes < 0) return String(value);
        if (minutes % 60 === 0) return `${minutes / 60}h`;
        return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
      }
      if (field.type === 'date') {
        const days = today ? daysBetween(String(value), today) : null;
        return days === null ? String(value) : dayWords(days);
      }
      if (field.type === 'bool') return value ? 'set' : 'not set';
      if (field.type === 'multiSelect') {
        return (Array.isArray(value) ? value.map(String) : [String(value)]).join(', ');
      }
      if (field.type === 'rating') return '★'.repeat(Number(value));
      if (typeof value === 'number') return String(Math.round(value * 100) / 100);
      return String(value);
    }

    /**
     * The record's own instant, in the plan's words — and whether it is set, which
     * is what makes a clear live and a clock able to shift it.
     */
    function instantOf(record: RecordDoc): InstantFact | null {
      const field = instantField;
      if (!field) return null;
      const label = labelOf(field, field.key);
      const held = record.fields[field.key];
      const day = typeof held === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(held) ? held : '';
      if (!day) return { key: field.key, label, words: '', set: false, late: false };
      const days = today ? daysBetween(day, today) : null;
      return {
        key: field.key,
        label,
        words: days === null ? '' : dayWords(days),
        set: true,
        late: days !== null && days < 0 && LATE_KEYS.includes(field.key),
      };
    }

    /** Shift record date relative to existing date or today (`controls.md` §2). */
    function shiftInstant(record: RecordDoc, days: number): void {
      const field = instantField;
      if (!field) return;
      const held = record.fields[field.key];
      const base = typeof held === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(held) ? held : today;
      if (!base) return;
      const next = new Date(Date.parse(`${base}T00:00:00Z`) + days * 86_400_000).toISOString().slice(0, 10);
      void commitCell(record, field.key, next);
    }

    /**
     * The row's identity mark: the record's own shortest categorical reading (its
     * kind, its difficulty, its phase) on the wash the plan assigns it. The lab's
     * tile is a *course code*, which is a plan-specific word this app cannot
     * assume — but the job is the same one: say what this record is, at a glance,
     * on the identity colour. A record with no such reading gets no mark rather
     * than a made-up monogram.
     */
    function markOf(record: RecordDoc, read: CellFact[]): RowFact['mark'] {
      const short = read.find(
        (cell) => cell.field?.type === 'select' && cell.text.length > 0 && cell.text.length <= 12,
      );
      return short ? { text: short.text, wash: app.washes[record.id] } : null;
    }

    /**
     * The row's own sentence: what a screen reader hears for this row, built from
     * the readings the row actually shows — never from a joined list of raw
     * values (the census gate caught exactly that mistake on a `session` list,
     * where a record's label is a date and a screen reader spells it out).
     */
    function sentenceOf(record: RecordDoc, read: CellFact[]): string {
      const said = read
        .filter((cell) => cell.field)
        .map((cell) => `${cell.label} ${cell.text || 'empty'}`)
        .join(' · ');
      return said.length > 0 ? `${recordLabel(record)} · ${said}` : recordLabel(record);
    }

    /**
     * The view's columns as the strip draws them: label, kind, and the menu wired
     * to this column. A column the kind does not declare has no menu — a header
     * the schema cannot act on would be a control that lies about what it edits.
     */
    const heads = $derived.by<ColumnFact[]>(() =>
      columns.flatMap((key, at) => {
        const field = fieldOf(key);
        if (!field) return [];
        return [
          {
            key,
            label: labelOf(field, key),
            type: field.type,
            menu: {
              field,
              type: outcome.type,
              view: outcome.view,
              previous: at > 0 ? (fieldOf(columns[at - 1]) ?? null) : null,
              next: at + 1 < columns.length ? (fieldOf(columns[at + 1]) ?? null) : null,
              columns,
              ondrag: (key_: string) => (dragging = key_ === '' ? null : key_),
              oncommand: (id: string, params: Record<string, unknown>) => {
                const lines: Record<string, string> = {
                  'column.duplicate': 'Duplicated the column',
                  'column.hide': 'Hidden in this view — undo shows it again',
                  'view.setColumns': 'Moved the column',
                };
                if (lines[id]) void written(id, params, lines[id]);
                else void app.run(id, params);
              },
              onsheet: (target: FieldRead, kind: 'rename' | 'retype' | 'choices' | 'delete') => {
                switch (kind) {
                  case 'rename':
                    return void app.openSheet({ kind: 'column.rename', type: outcome.type, field: target });
                  case 'retype':
                    return void app.openSheet({ kind: 'column.retype', type: outcome.type, field: target });
                  case 'choices':
                    return void app.openSheet({ kind: 'column.choices', type: outcome.type, field: target });
                  default:
                    return void app.openSheet({ kind: 'column.delete', type: outcome.type, field: target });
                }
              },
            },
          },
        ];
      }),
    );

    /** The one column that takes the free space — the strip and A's rows share it. */
    const flexKey = $derived(flexKeyOf(heads));

    /** Every row, as the designs draw it. */
    const facts = $derived.by<RowFact[]>(() =>
      rows.map((record) => {
        const read: CellFact[] = columns.map((key) => {
          const field = fieldOf(key);
          return {
            key,
            label: labelOf(field, key),
            type: field?.type ?? 'unknown',
            field,
            targets: relationTargets.get(key) ?? [],
            text: field ? readingOf(record, field) : '',
            write: stateOf(record, key),
          };
        });
        return {
          id: record.id,
          label: recordLabel(record),
          sentence: sentenceOf(record, read),
          record,
          kind: record.type,
          wash: app.washes[record.id],
          mark: markOf(record, read),
          cells: read,
          instant: instantOf(record),
          failures: failures(record),
        };
      }),
    );

    /** The first number field a renumber can write — `index` in the seed. */
    const numberField = $derived(
      (type?.fields ?? []).find((field) => field.type === 'number')?.key ?? 'index',
    );

    /** The header `+`: this view's hidden columns first, then *New column*. */
    const plusRows = $derived<MenuRow[]>([
      ...hidden.map((key): MenuRow => ({
        id: 'column.show',
        title: `Show ${labelOf(fieldOf(key), key)}`,
        run: () =>
          void written(
            'column.show',
            { spec: `${outcome.type}.${key}`, view: outcome.view },
            'Shown in this view again — undo takes it back out',
          ),
      })),
      ...(hidden.length > 0 ? ([{ separator: true }] as MenuRow[]) : []),
      ...columnPlusIds()
        .filter((id) => id === 'column.new')
        .map((id): MenuRow => ({
          id,
          title: 'New column…',
          run: () => void app.openSheet({ kind: 'column.new', type: outcome.type }),
        })),
    ]);

    async function copy(text: string, what: string): Promise<void> {
      try {
        await navigator.clipboard.writeText(text);
        app.notice(`copied ${what}`);
      } catch {
        app.notice(`cannot copy ${what} — the clipboard is unavailable here`);
      }
    }

    /** The row menu: the canonical line, the path, the panel, move and delete. */
    function rowRows(row: RowFact): MenuRow[] {
      const record = row.record;
      return rowMenuIds().map((id): MenuRow => {
        switch (id) {
          case COPY_JSON:
            return {
              id,
              title: 'Copy as JSON',
              run: () => copy(JSON.stringify(record), `the ${recordLabel(record)} line`),
            };
          case COPY_PATH:
            return {
              id,
              title: 'Copy the path',
              run: () => copy(`content/records/${record.type}.jsonl#${record.id}`, 'the path'),
            };
          case 'record.reveal':
            return { id, title: 'Open in the source pane', run: () => void app.reveal(record) };
          case 'record.move':
            // Structural editing (§6 Phase 6): a record can change kind, and the
            // engine reports what the target cannot hold rather than dropping it
            // silently. The target list is the plan's own types — data, not a
            // hardcoded vocabulary.
            return { id, title: 'Move to another kind…', run: () => (moveFor = row.id) };
          default:
            return {
              id,
              title: 'Delete this record…',
              danger: true,
              run: () => void app.openSheet({ kind: 'record.delete', record }),
            };
        }
      });
    }

    /** The kinds a record can move to: every declared kind but its own. */
    function moveRows(row: RowFact): MenuRow[] {
      const record = row.record;
      return Object.keys(app.types)
        .filter((name) => name !== record.type)
        .map(
          (name): MenuRow => ({
            id: 'record.move',
            title: nameOf(name),
            run: () => void written('record.move', { id: record.id, to: name }, `Moved to ${nameOf(name)}`),
          }),
        );
    }

    /**
     * Everything a design is handed, assembled from the reads above. The designs
     * take data and callbacks and never touch `app.*` themselves — and the one
     * write they can start (a clock, a clear) leaves through `onInstant`, which is
     * the same single commit path a cell uses.
     */
    const panel: RecordTableProps = $derived({
      title: title ?? nameOf(outcome.view),
      view: outcome.view,
      kind: outcome.type,
      columns: heads,
      flexKey,
      hiddenCount: hidden.length,
      rows: facts,
      candidates: outcome.candidates,
      inFlight,
      selected: app.selection,
      today,
      plusOpen,
      menuFor: rowMenuFor,
      moveFor,
      dragging,
      dropTarget,
      plusRows,
      rowRows,
      moveRows,
      instant: instantField
        ? { key: instantField.key, label: labelOf(instantField, instantField.key) }
        : null,
      onSelect: (id) => (app.selection = id),
      onCommit: (row, key, value) => void commitCell(row.record, key, value),
      onPanel: (row) => openPanel(row.id, row.kind),
      onMenu: (id) => (rowMenuFor = id),
      onMove: (id) => (moveFor = id),
      onPlus: (open) => (plusOpen = open),
      onDrag: (key) => (dragging = key === '' ? null : key),
      onOver: (key) => (dropTarget = key),
      onDrop: dropOn,
      onInstant: (row, days) => {
        const field = instantField;
        if (!field) return;
        if (days === null) void commitCell(row.record, field.key, null);
        else shiftInstant(row.record, days);
      },
    });
  </script>

  <!-- The card is one of three designs; which one the student reads is their own
       preference, set in Settings › Component styles and nowhere else. -->
  <section class="cd-card" data-view={outcome.view} data-type={outcome.type}>
    <header class="cd-card__head">
      <div>
        <h2 class="cd-card__title">{title ?? nameOf(outcome.view)}</h2>
        <p class="cd-card__sub">
          {facts.length} of {outcome.candidates} rows
          {#if inFlight > 0}<span class="cd-hint" role="status"> · saving…</span>{/if}
        </p>
      </div>
      <span class="cd-card__spacer"></span>
      <button
        class="cd-pill"
        type="button"
        data-command={`${outcome.type}.new`}
        data-placement="recordTable.toolbar"
        onclick={() => void app.openSheet({ kind: 'record.new', type: outcome.type })}
      >
        New {outcome.type}
      </button>
      <!-- The bulk door sits beside the simple one, never instead of it (§4.8). -->
      <button
        class="cd-pill cd-pill--ghost"
        type="button"
        data-command={`${outcome.type}.paste`}
        data-placement="recordTable.paste"
        title={`Paste TSV/CSV as ${outcome.type} records — commits as one apply batch`}
        onclick={() => void app.openSheet({ kind: 'paste', type: outcome.type })}
      >
        Paste
      </button>
      <!-- One control, one command: the renumber is the toolbar's own verb, and
           the record's own verbs live in the record's own menu. -->
      <button
        class="cd-pill cd-pill--quiet cd-pill--sm"
        type="button"
        data-command="records.renumber"
        data-placement="recordTable.toolbar"
        title={`Number every ${outcome.type} by ${numberField} — undoable straight after`}
        onclick={() =>
          void written(
            'records.renumber',
            { type: outcome.type, field: numberField },
            `Numbered every ${outcome.type} by ${numberField}`,
          )}
      >
        Renumber
      </button>
    </header>

    <!-- The body is the design's; the strip, the rows and the row's own doors all
         live inside whichever of the three the student kept. -->
    <Variant surface="record-table" {...panel} />

    {#if facts.length === 0}
      <!-- "nothing matches" and "nothing exists" are different states, and the
           fix differs: one is a filter, the other is a record. -->
      <p class="cd-empty">
        {#if (outcome.candidates ?? 0) > 0}
          Nothing matches this view — {outcome.candidates} {outcome.type} exist. Clear the filter, or widen it in <em>Edit view…</em>.
        {:else}
          No records yet — use New {outcome.type}, or paste lines.
        {/if}
      </p>
    {:else}
      <!--
        A count, and nothing else.

        This announced every row's label joined by a middot, which is wrong twice
        over. It is redundant — every row is a real control with its own accessible
        name, so a screen reader reads the table once here and again row by row.
        And it is where the interface **spoke the schema**: for a `session` table
        the record's label IS its date, because the `session` type declares
        `date · min · note` and no title, so the announcement came out as
        "2026-09-18 · 2026-09-19 · 2026-09-21 …" — which the string census caught,
        and which a screen reader pronounces as "two thousand and twenty, hyphen,
        naught nine, hyphen, one eight", eight times.

        So it says the one thing the rows do not, which is how many there are. Each
        row's identity is that row's own name, which is where it belongs.
      -->
      <p class="cd-sr" role="status">
        {facts.length} {facts.length === 1 ? 'row' : 'rows'} in this table.
      </p>
    {/if}

    <!-- Keyed by code, and the engine's own words stay in the row's `title`
         (D2): the message is machine text — `record "topic.r1" sort key: path
         segment "index" continues from a non-record value` — and printing it put
         a record id and a field path in front of the student (the census caught
         it). One row per code, because the same refusal once per record is one
         fact, not nineteen; the count says how many. -->
    {#each findings as finding (finding.code)}
      <p class="cd-error" title={finding.detail}>
        A sort or filter this view asks for cannot be read for every record{#if finding.count > 1}
          ({finding.count} findings){/if} — hover this line for the engine’s own words.
      </p>
    {/each}
  </section>

<style>
  /* The count line's own `· saving…` (the head's, not the design's). */
  .cd-hint {
    color: var(--ink-3);
  }
</style>
