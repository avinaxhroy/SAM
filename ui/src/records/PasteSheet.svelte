<!--
  Delimited record import sheet (Appendix C.9, §3.1, §4.7, §4.8).

  Parses TSV/CSV tabular input and maps columns to type schema fields:
    - Header auto-mapping: Maps header labels matching schema keys; unmapped columns default to skip.
    - Atomicity: Submits as a single batch via `record.paste` within one transaction.
    - Relations: Splits comma-separated entries in relation fields into string ID arrays.
    - Verification: Displays imported record receipts upon successful write.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import { labelOf, nameOf, recordLabel, type FieldRead } from '../types';
  import Variant from '../variants/Variant.svelte';
  import {
    addedIds,
    count,
    refusalOf,
    type DoorColumn,
    type DoorReadRow,
    type DoorReceipt,
    type DoorRefusal,
  } from '../variants/record-doors/props';

  let { type, onclose }: { type: string; onclose: () => void } = $props();

  /** The mapper's two non-field targets: skip this column, or it IS the id. */
  const SKIP = '';
  const ID = '(id)';

  let text = $state('');
  let header = $state(true);
  let delimiter = $state<'tab' | 'comma'>('tab');
  let busy = $state(false);
  let error = $state<DoorRefusal | null>(null);
  let receipt = $state<DoorReceipt | null>(null);
  /**
   * A correction, keyed by the column it was made on — the header's name when
   * there is one, else the position. Keyed this way the mapping survives an
   * edit to the buffer (the common case: paste, fix a cell, commit) while a
   * column that actually changed identity gets re-derived.
   */
  let overrides = $state<Record<string, string>>({});

  const fields = $derived<FieldRead[]>(app.types[type]?.fields ?? []);
  const rows = $derived(grid());
  const head = $derived(rows[0] ?? []);
  /** The rows that will become records: the header is a name, not a record. */
  const body = $derived(header ? rows.slice(1) : rows);
  const width = $derived(body.reduce((max, row) => Math.max(max, row.length), head.length));
  const columns = $derived(Array.from({ length: width }, (_, index) => index));
  const mapped = $derived(columns.filter((index) => chosen(index) !== SKIP).length);

  /** The buffer as cells. A blank line is a blank line, not a one-cell row. */
  function grid(): string[][] {
    const separator = delimiter === 'tab' ? '\t' : ',';
    return text
      .split('\n')
      .filter((line) => line.trim() !== '')
      .map((line) => line.split(separator));
  }

  function slot(index: number): string {
    const name = (head[index] ?? '').trim().toLowerCase();
    return header && name !== '' ? `h:${name}` : `c:${index}`;
  }

  /** The header's own advice: a field key, `id`, or nothing this type knows. */
  function auto(index: number): string {
    if (!header) return SKIP;
    const name = (head[index] ?? '').trim();
    if (name === '') return SKIP;
    if (name === ID || name.toLowerCase() === 'id') return ID;
    const field = fields.find((candidate) => candidate.key.toLowerCase() === name.toLowerCase());
    return field ? field.key : SKIP;
  }

  function chosen(index: number): string {
    return overrides[slot(index)] ?? auto(index);
  }

  /** What a column is called: its header, else the position it sits in. */
  function column(index: number): string {
    const name = header ? (head[index] ?? '').trim() : '';
    return name === '' ? `Column ${index + 1}` : name;
  }

  /** One row object, exactly as `record.paste` reads it (§C.9's mapping). */
  function record(cells: string[]): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    columns.forEach((index) => {
      const target = chosen(index);
      const value = (cells[index] ?? '').trim();
      // An empty cell means "not supplied", and an absent value is not an
      // empty one (§3.7) — so it is dropped rather than written as "".
      if (target === SKIP || value === '') return;
      if (target === ID) {
        out.id = value;
        return;
      }
      const field = fields.find((candidate) => candidate.key === target);
      if (!field) return;
      out[target] =
        field.type === 'relation'
          ? value.split(',').map((id) => id.trim()).filter((id) => id !== '')
          : value;
    });
    return out;
  }

  // ── what the drawing is handed ─────────────────────────────────────────
  const doorColumns = $derived<DoorColumn[]>(
    columns.map((index) => ({ index, name: column(index), target: chosen(index) })),
  );

  const mapperTargets = $derived([
    { value: SKIP, label: '— skip' },
    { value: ID, label: ID },
    ...fields.map((field) => ({ value: field.key, label: `${field.key} · ${labelOf(field, field.key)}` })),
  ]);

  const lineCount = $derived(count(body.length, 'line'));
  const countLine = $derived(`${lineCount} · ${count(width, 'column')}`);

  /** The reason this door cannot leave — the sentence the commit itself writes,
      printed where it is about instead of hidden behind an inert button. */
  const why = $derived(
    body.length === 0
      ? 'nothing to paste yet — every mapped cell is empty'
      : mapped === 0
        ? 'map at least one column — an unmapped paste would write nothing'
        : null,
  );

  /** What the batch actually produced, read back: the engine's own `diff` names
      the records it added, and the app's `records` read + `recordLabel` name
      them the way every other screen does. */
  async function receiptOf(added: string[]): Promise<DoorReceipt> {
    const written = added.length > 0 ? added : [];
    const seen = written.length > 0 ? await app.recordsOf(type) : [];
    const byId = new Map(seen.map((record_) => [record_.id, record_]));
    const rowsOut: DoorReadRow[] = written.map((id) => {
      const record_ = byId.get(id);
      return {
        label: nameOf(type),
        key: id,
        value: record_ ? recordLabel(record_) : id,
      };
    });
    return {
      title: `Pasted ${count(rowsOut.length, 'record')}`,
      rows: rowsOut,
      note: written.join(' · '),
    };
  }

  async function commit(): Promise<void> {
    error = null;
    if (mapped === 0 || body.length === 0) return;
    const records = body
      .filter((cells) => cells.some((cell, index) => chosen(index) !== SKIP && cell.trim() !== ''))
      .map(record);
    if (records.length === 0) {
      error = { message: 'nothing to paste — every mapped cell is empty' };
      return;
    }

    busy = true;
    try {
      // `records` travels as a JSON string: the engine parses the batch itself
      // (`paste_records` reads the same `records` array the CLI's flag carries).
      // The per-kind spelling of the same handler (`topic.paste`): one
      // operation, and the id on screen is the id dispatched.
      const result = await app.run(`${type}.paste`, { type, records: JSON.stringify(records) });
      if (!result) {
        // The session's dispatch path owns the toast; while a door is open the
        // door IS the app's one floating surface (§11), so the finding moves
        // here rather than being shown twice — with the address and the code the
        // engine carried, and nothing invented to fill a gap it left.
        error = refusalOf(app.lastDiagnostic ?? app.toast ?? 'the paste was refused');
        app.toast = null;
        return;
      }
      receipt = await receiptOf(addedIds(result.data));
    } finally {
      busy = false;
    }
  }
</script>

<Variant
  surface="record-doors"
  mode="paste"
  title="Paste"
  subtitle="One batch, one validation, one transaction — nothing is written unless all of it is"
  kind={type}
  kindOptions={[]}
  fixedKind={true}
  trailing="records"
  rows={[]}
  {busy}
  {error}
  {receipt}
  hint={`${lineCount} → ${nameOf(type)}`}
  buffer={text}
  hasHeader={header}
  {delimiter}
  columns={doorColumns}
  targets={mapperTargets}
  count={countLine}
  lines={lineCount}
  {why}
  onInput={() => {}}
  onPick={() => {}}
  onKind={() => {}}
  onRelation={() => {}}
  onLink={() => {}}
  onBuffer={(value: string) => {
    text = value;
    error = null;
  }}
  onHeader={(on: boolean) => {
    header = on;
  }}
  onDelimiter={(next: 'tab' | 'comma') => {
    delimiter = next;
  }}
  onMap={(index: number, target: string) => {
    overrides = { ...overrides, [slot(index)]: target };
    error = null;
  }}
  onCommit={() => void commit()}
  onCancel={onclose}
/>
