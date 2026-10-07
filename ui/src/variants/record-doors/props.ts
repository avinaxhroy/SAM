/** Props for NewRecordSheet and PasteSheet. */

/** One kind the plan declares, as the door's own control lists it. */
export type DoorKind = { key: string; name: string };

/**
 * One row. The `control` is the one `design/controls.md` §4 declares for the
 * field's type: a well for text, a numeric well for a number or a duration, a
 * date control for a date, a segment for 2–5 choices, a select or chips beyond
 * that, a labelled check for a yes/no, and a picker for a relation.
 */
export type DoorField =
  | {
      control: 'text' | 'number' | 'duration' | 'date' | 'longtext';
      key: string;
      label: string;
      value: string;
      placeholder?: string;
    }
  | { control: 'bool'; key: string; label: string; value: boolean }
  | { control: 'segment'; key: string; label: string; options: string[]; value: string }
  | { control: 'select'; key: string; label: string; options: string[]; value: string }
  | { control: 'multi'; key: string; label: string; options: string[]; values: string[] }
  | {
      control: 'relation';
      key: string;
      label: string;
      /** The kind this relation points at, as the row states beside its key. */
      to: string;
      chosen: string[];
      /** The target kind's records, by label — read when the picker opens. */
      targets: Array<{ id: string; label: string }>;
      open: boolean;
    };

/** One line of the receipt: a field, its key and the value it holds. */
export type DoorReadRow = { label: string; key: string; value: string; derived?: boolean };

/** What the write left behind, read back — never the engine's `summary`. */
export type DoorReceipt = { title: string; rows: DoorReadRow[]; note: string };

/** A refusal, carrying only what the engine carried. */
export type DoorRefusal = {
  /** The row the finding is about (`id`, a field key), when it is about one. */
  row?: string;
  /** The address the engine named (`fields/total`), when it named one. */
  at?: string;
  /** The engine's own code (`value.invalid`), when it carried one. */
  code?: string;
  message: string;
};

/** One mapped column of the paste buffer: its header, and what it becomes. */
export type DoorColumn = { index: number; name: string; target: string };

export type RecordDoorProps = {
  mode: 'new' | 'paste';
  /** `New` / `Paste`, and the door's own subtitle (one of the two sentences
      the shipped sheets say about their own write). */
  title: string;
  subtitle: string;
  /** The kind this door currently declares. */
  kind: string;
  /** The plan's kinds, in declaration order — the door that may change kind. */
  kindOptions: DoorKind[];
  /** true = the kind is a statement, not a control (Paste's mappings are fixed
      to one kind's keys, so it does not offer what it would have to undo). */
  fixedKind: boolean;
  /** The word after the kind chip (`New [problemset] ` / `Paste [problemset] records`). */
  trailing: string;

  /** The rows. `id` is not among them: it is the door's own first row. */
  rows: DoorField[];
  busy: boolean;

  /** The last refusal, printed at the row (or the mapper) it is about. */
  error?: DoorRefusal | null;
  /** The write's read-back, or null while the door is still a draft. */
  receipt?: DoorReceipt | null;

  /** The foot's hint (the id rule on New, the batch on Paste). */
  hint?: string;

  /* ── New's own half ─────────────────────────────────────────────────── */
  /** The `/ (blank = auto)` id row. */
  idValue?: string;
  idHint?: string;

  /* ── Paste's own half ───────────────────────────────────────────────── */
  /** The buffer and its two switches. */
  buffer?: string;
  hasHeader?: boolean;
  delimiter?: 'tab' | 'comma';
  columns?: DoorColumn[];
  /** The mapper's targets: skip, the id, and the kind's own keys. */
  targets?: Array<{ value: string; label: string }>;
  /** `4 lines · 7 columns` — the count line under the buffer. */
  count?: string;
  /** `4 lines` — the batch, for the commit's own label. */
  lines?: string;
  /** The reason the commit cannot leave, printed at the place it is about. */
  why?: string | null;

  onInput: (key: string, value: string) => void;
  onPick: (key: string, value: string) => void;
  onKind: (kind: string) => void;
  onRelation: (key: string) => void;
  onLink: (key: string, id: string) => void;
  onCommit: () => void;
  onCancel: () => void;
  onBuffer?: (text: string) => void;
  onHeader?: (on: boolean) => void;
  onDelimiter?: (delimiter: 'tab' | 'comma') => void;
  onMap?: (index: number, target: string) => void;
};

/** `3 lines` · `1 column` — a count a person reads. */
export function count(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

/**
 * The engine's one-line refusal, split back into the parts it carried.
 *
 * `IpcError.text` is the only form the session keeps (`app.lastDiagnostic`), and
 * it is `message [code]` when the diagnostic has no line, or
 * `path:line — message [code]` when it has one. So the address is recovered
 * only when the engine actually named one — a `Usage` error (a taken id) never
 * does, and nothing is invented to fill the gap.
 */
export function refusalOf(text: string): DoorRefusal {
  const at = /(?:^|\s)(fields\/[A-Za-z0-9_]+(?::\d+)?)/.exec(text)?.[1] ?? null;
  const code = /\[([a-z][a-z0-9._]*)\]\s*$/.exec(text)?.[1] ?? null;
  let message = text;
  if (at) message = message.replace(at, '').replace(/^\s*—\s*/, '');
  if (code) message = message.slice(0, message.lastIndexOf('['));
  const row = at ? at.slice('fields/'.length).replace(/:.*$/, '') : undefined;
  return { row, at: at ?? undefined, code: code ?? undefined, message: message.trim() };
}

/**
 * The ids a write actually added, read off the engine's own `diff` in the
 * dispatch result (`apply::RecordDiffEntry`, `action: "added"`). This is how a
 * receipt reads back the id the ENGINE chose for a blank draft, instead of
 * guessing `<kind>.r<N>`.
 */
export function addedIds(data: unknown): string[] {
  const diff = (data as { diff?: Array<{ id?: unknown; action?: unknown }> } | undefined)?.diff;
  if (!Array.isArray(diff)) return [];
  return diff
    .filter((entry) => entry?.action === 'added' && typeof entry.id === 'string')
    .map((entry) => entry.id as string);
}

/**
 * A day said the way a person says it (`4 days from now`, `today`): the same
 * distance `RecordDetail` prints, so a receipt never shows an ISO string.
 */
export function dayText(iso: string, today: string): string {
  const days = Math.round(
    (Date.parse(`${iso.slice(0, 10)}T00:00:00Z`) - Date.parse(`${today}T00:00:00Z`)) / 86400000,
  );
  if (!Number.isFinite(days)) return '—';
  if (days === 0) return 'today';
  if (days === 1) return 'tomorrow';
  if (days === -1) return 'yesterday';
  return days > 0 ? `in ${days} days` : `${-days} days ago`;
}
