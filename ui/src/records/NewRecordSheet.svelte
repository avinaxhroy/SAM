<!--
  New record creation sheet (Appendix C.9, §4.7 rule 2).

  Dynamically generates form inputs based on the target type schema (`[FieldDef]`):
    - Auto ID assignment: Omitting `id` lets the engine assign a sequential ID via `DocEdit.autoId`.
    - Computed exclusion: Derived fields (`formula`, `progress`) are excluded from creation drafts.
    - Diagnostics: Validation errors map to corresponding field rows based on JSON pointer paths.
    - Relation serialization: Multi-ID references serialize as arrays.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import { labelOf, nameOf, type FieldRead } from '../types';
  import Variant from '../variants/Variant.svelte';
  import {
    addedIds,
    dayText,
    refusalOf,
    type DoorField,
    type DoorKind,
    type DoorReadRow,
    type DoorReceipt,
    type DoorRefusal,
  } from '../variants/record-doors/props';

  let { type, onclose }: { type: string; onclose: () => void } = $props();

  /** The kind this door declares — re-declarable from its own head. */
  let kind = $state(type);
  let id = $state('');
  let draft = $state<Record<string, string>>({});
  /** Relation picks, kept as id lists (never comma-joined) and per field. */
  let links = $state<Record<string, string[]>>({});
  /** Multi-choice picks, for the same reason. */
  let multi = $state<Record<string, string[]>>({});
  /** Which relation field's picker is open, and what each target kind holds. */
  let openFor = $state<string | null>(null);
  let targets = $state<Record<string, Array<{ id: string; label: string }>>>({});
  let busy = $state(false);
  let error = $state<DoorRefusal | null>(null);
  let receipt = $state<DoorReceipt | null>(null);

  const fields = $derived<FieldRead[]>(
    (app.types[kind]?.fields ?? []).filter(
      (field) => field.type !== 'formula' && field.type !== 'progress',
    ),
  );

  /** The plan's own kinds, in declaration order (`types.json`'s order). */
  const kindOptions = $derived<DoorKind[]>(
    Object.keys(app.types).map((key) => ({ key, name: nameOf(key) })),
  );

  /** One row per settable field, and the control its type declares. */
  function rowOf(field: FieldRead): DoorField {
    const label = labelOf(field, field.key);
    const options = field.options ?? [];
    if (field.type === 'bool') {
      return { control: 'bool', key: field.key, label, value: draft[field.key] === 'true' };
    }
    if (field.type === 'relation') {
      const to = field.to ?? '';
      return {
        control: 'relation',
        key: field.key,
        label,
        to,
        chosen: links[field.key] ?? [],
        targets: targets[to] ?? [],
        open: openFor === field.key,
      };
    }
    if (field.type === 'multiSelect') {
      return { control: 'multi', key: field.key, label, options, values: multi[field.key] ?? [] };
    }
    if (field.type === 'select') {
      const value = draft[field.key] ?? '';
      return options.length > 0 && options.length <= 5
        ? { control: 'segment', key: field.key, label, options, value }
        : { control: 'select', key: field.key, label, options, value };
    }
    const value = draft[field.key] ?? '';
    if (field.type === 'longtext') return { control: 'longtext', key: field.key, label, value };
    if (field.type === 'number' || field.type === 'duration' || field.type === 'date') {
      return {
        control: field.type,
        key: field.key,
        label,
        value,
        placeholder: field.type === 'duration' ? '30' : undefined,
      };
    }
    return { control: 'text', key: field.key, label, value };
  }

  const rows = $derived<DoorField[]>(fields.map(rowOf));

  /** The picker opens on demand and reads the target kind once (C.1). */
  async function openPicker(field: FieldRead): Promise<void> {
    const to = field.to ?? '';
    if (to !== '' && !targets[to]) {
      const records = await app.recordsOf(to);
      targets = {
        ...targets,
        [to]: records.map((record) => ({
          id: record.id,
          label: String(record.fields.title ?? record.fields.label ?? record.fields.name ?? record.id),
        })),
      };
    }
    openFor = openFor === field.key ? null : field.key;
  }

  function onInput(key: string, value: string): void {
    if (key === 'id') {
      id = value;
    } else {
      draft = { ...draft, [key]: value };
    }
    error = null;
  }

  function onPick(key: string, value: string): void {
    error = null;
    const field = fields.find((candidate) => candidate.key === key);
    if (field?.type === 'multiSelect') {
      const current = multi[key] ?? [];
      multi = {
        ...multi,
        [key]: current.includes(value) ? current.filter((option) => option !== value) : [...current, value],
      };
      return;
    }
    // A boolean and a segment both write one value, and both clear on a second
    // press (`false` for the switch, `''` for the segment).
    draft = { ...draft, [key]: draft[key] === value ? '' : value };
  }

  function onLink(key: string, target: string): void {
    const current = links[key] ?? [];
    links = {
      ...links,
      [key]: current.includes(target) ? current.filter((id_) => id_ !== target) : [...current, target],
    };
  }

  /** Re-declare the door: a new kind is a new form, and no key of the old one
      is a key of the new one, so the draft is emptied rather than carried. */
  function onKind(next: string): void {
    kind = next;
    id = '';
    draft = {};
    links = {};
    multi = {};
    openFor = null;
    error = null;
  }

  /** The parameters the per-type verb reads: field keys, plus an optional id. */
  function params(): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    const chosen = id.trim();
    if (chosen !== '') out.id = chosen;
    for (const field of fields) {
      if (field.type === 'relation') {
        const picked = links[field.key] ?? [];
        if (picked.length > 0) out[field.key] = picked;
        continue;
      }
      if (field.type === 'multiSelect') {
        const picked = multi[field.key] ?? [];
        if (picked.length > 0) out[field.key] = picked;
        continue;
      }
      const value = (draft[field.key] ?? '').trim();
      // An empty field means "not supplied" (Appendix C.3's draft rule): the
      // key is dropped rather than sent as "", so §3.7's defaults still apply.
      if (value === '') continue;
      out[field.key] = value;
    }
    return out;
  }

  /** What the write left behind, read back: the id the engine wrote, and the
      fields the draft supplied. An omitted field is NOT shown — an empty draft
      field was never written, and `—` would claim a value. */
  function receiptOf(added: string[]): DoorReceipt {
    const today = app.todayFacts?.date ?? null;
    const written = added[0] ?? id.trim();
    const rowsOut: DoorReadRow[] = [];
    if (written !== '') rowsOut.push({ label: 'ID', key: 'id', value: written });
    for (const field of fields) {
      const label = labelOf(field, field.key);
      if (field.type === 'relation') {
        const picked = links[field.key] ?? [];
        if (picked.length === 0) continue;
        rowsOut.push({
          label,
          key: field.key,
          value: picked
            .map((id_) => (targets[field.to ?? ''] ?? []).find((target) => target.id === id_)?.label ?? id_)
            .join(' · '),
        });
        continue;
      }
      if (field.type === 'multiSelect') {
        const picked = multi[field.key] ?? [];
        if (picked.length === 0) continue;
        rowsOut.push({ label, key: field.key, value: picked.join(' · ') });
        continue;
      }
      const value = (draft[field.key] ?? '').trim();
      if (value === '') continue;
      rowsOut.push({
        label,
        key: field.key,
        value: field.type === 'date' && today ? dayText(value, today) : value,
      });
    }
    return {
      title: written === '' ? `Created a ${nameOf(kind)}` : `Created ${nameOf(kind)} ${written}`,
      rows: rowsOut,
      note: added.join(' · '),
    };
  }

  async function commit(): Promise<void> {
    error = null;
    busy = true;
    try {
      // One dispatch of the per-type verb (`topic.new`): its suffix is read and
      // the type id is passed through untouched (§4.7 rule 2).
      const result = await app.run(`${kind}.new`, params());
      if (!result) {
        // The finding belongs in the door the student is still looking at, so
        // the door stays open and the toast is cleared rather than repeated.
        // Its row: the engine's own one-line form leads with the address when it
        // has one, and a refusal with no address is the id's — the engine checks
        // the id before it parses a value (`doc_edit.rs:1017-1035`).
        const text = app.lastDiagnostic ?? app.toast ?? `the new ${kind} was refused`;
        error = refusalOf(text);
        app.toast = null;
        return;
      }
      receipt = receiptOf(addedIds(result.data));
    } finally {
      busy = false;
    }
  }
</script>

<Variant
  surface="record-doors"
  mode="new"
  title="New"
  subtitle="The engine validates the whole record before anything is written"
  {kind}
  {kindOptions}
  fixedKind={false}
  trailing=""
  {rows}
  idValue={id}
  idHint="(blank = auto)"
  hint={id.trim() === '' ? 'the id comes from the engine' : id.trim()}
  {busy}
  {error}
  {receipt}
  onInput={onInput}
  onPick={onPick}
  onKind={onKind}
  onRelation={(key: string) => {
    const field = fields.find((candidate) => candidate.key === key);
    if (field) void openPicker(field);
  }}
  onLink={onLink}
  onCommit={() => void commit()}
  onCancel={onclose}
/>
