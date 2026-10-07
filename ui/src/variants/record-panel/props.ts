/**
 * Record detail panel surface variant component props.
 * Displays record fields, relations, ladder rungs, and review metrics.
 */
import type { FieldRead, RecordDoc } from '../../types';

/** Which rung of the ladder a stage is on — the app's own three words. */
export type RungState = 'done' | 'now' | 'todo';

/**
 * One column the panel carries: a quantity or a date, with the control its
 * declared type opens. What the panel does NOT carry is the kind's text, select,
 * bool and url columns — the table and the file door read those, and a column
 * that listed them all would be the table the panel already sits beside.
 */
export type FieldFact = {
  key: string;
  /** The app's own word for the column (`labelOf`). */
  label: string;
  /** The declared type — which control the family opens at this value. */
  type: string;
  /** The value in the app's own words; `—` when the plan holds nothing. */
  reading: string;
  /** The numeric reading, when the column is a quantity (else null). */
  amount: number | null;
  /** The value an editor starts from: digits for a number or a duration, the
   *  engine's own day for a date. Never printed as prose. */
  raw: string;
  options: string[];
  /** Computed by the kind: read, marked `ƒ`, never written (§3.4). */
  derived: boolean;
};

export type LinkTarget = {
  /** The target record's id — a value a design passes to `onOpen`, never prints. */
  id: string;
  /** The target's own label (`recordLabel`). */
  label: string;
  /** Its `code`, when the kind declares one (a course says `CS201`). */
  code: string | null;
};

/** One relation the record holds: a declared relation field, or its parent edge. */
export type LinkFact = {
  /** The link key — declared, or the parent type's own name (§3.1). */
  key: string;
  /** The app's word for it: the field's label, or the parent kind's name. */
  label: string;
  /** The kind the links point at. */
  to: string;
  targets: LinkTarget[];
};

export type RungFact = { name: string; state: RungState; word: string };

/** The record's own entry in the review projection (`reviews.due`), in words. */
export type HistoryFact = {
  /** The earliest instant the record's review log holds, or null. */
  first: string | null;
  /** `review.last` — when the record was last reviewed. */
  reviewed: string | null;
  /** The last log entry's rating, in the app's own word (`Good`). */
  rating: string | null;
  /** `review.due` — the next review. */
  due: string | null;
  /** How late that next review is, in days (`overdueDays`). */
  late: number;
  complete: boolean;
  /** What the next rung waits on, in the app's own words. */
  asks: string[];
  /** The scheduler's own interval, in days — the cadence the plate measures. */
  intervalDays: number | null;
  rungs: RungFact[];
};

/**
 * The minutes the plan logged for the record: a **derived** sum of the
 * `session` records that link the record's course inside its own interval.
 * Printed with its last session's date beside it, and never written back.
 */
export type LoggedFact = { minutes: number; on: string | null };

export type RecordPanelProps = {
  /** The kind it is (`nameOf(type)`). The record's own name is the head's, which
   *  the pane draws — so the designs are handed the half they draw. */
  kind: string;
  /** Its line in the plan's own files — the file door's address, `data-developer`. */
  address: string;
  /** The plan's today, as the engine dates it. */
  today: string | null;
  /** The kind's quantities: number, duration, formula, progress — in its order. */
  quantities: FieldFact[];
  /** Its date columns, in the kind's order. */
  dates: FieldFact[];
  links: LinkFact[];
  history: HistoryFact | null;
  logged: LoggedFact;
  /** The day the record is planned for (`focus`), when its kind declares one —
   *  an interval's end, and C's day axis reads it. */
  planned: string | null;
  /** One `record.setField`, exactly as the table's cells dispatch it; a refusal
   *  is the engine's own sentence, returned rather than thrown. */
  onSet: (key: string, value: string | string[] | null) => Promise<string | null>;
  /** A relation's door — the app's own `openPanel`: it moves the detail pane. */
  onOpen: (id: string, type: string) => void;
  /** The file door (`record.reveal`) and the record's own delete sheet. */
  onReveal: () => void;
  onDelete: () => void;
};

/* ── ONE FORMATTER ────────────────────────────────────────────────────────
   Every function below exists in the app already; this is the one copy the
   ported designs read, so the three of them can never disagree about a word. */

/** The review's four ratings, in the app's own words (`Reviews.svelte`). */
export const RATING_WORDS: Record<string, string> = {
  again: 'Forgot',
  hard: 'Hard',
  good: 'Good',
  easy: 'Solid',
};

export function ratingWord(rating: string | null | undefined): string | null {
  if (!rating) return null;
  return RATING_WORDS[rating] ?? null;
}

/** The ladder's three state words (`RecordDetail.svelte`). */
export function stageWord(state: RungState): string {
  return state === 'done' ? 'passed' : state === 'now' ? 'next up' : 'not yet';
}

/** §3.4: a derived field is computed, never stored, so it is never edited. */
export function isDerived(field: { type: string }): boolean {
  return field.type === 'formula' || field.type === 'progress';
}

function dayOf(iso: string): number {
  return Date.parse(`${iso.slice(0, 10)}T00:00:00Z`);
}

/** Whole days from one date to another, or null when either is not a date. */
export function dayDistance(from: string | null | undefined, to: string | null | undefined): number | null {
  if (!from || !to) return null;
  const days = Math.round((dayOf(to) - dayOf(from)) / 86_400_000);
  return Number.isNaN(days) ? null : days;
}

export function plusDays(iso: string, count: number): string {
  return new Date(dayOf(iso) + count * 86_400_000).toISOString().slice(0, 10);
}

/** A distance from the plan's today, never an ISO string (D2, R9). */
export function relative(iso: string | null | undefined, today: string | null): string {
  const days = dayDistance(today, iso);
  if (days === null) return '';
  if (days === 0) return 'today';
  if (days === 1) return 'tomorrow';
  if (days === -1) return 'yesterday';
  return days > 0 ? `in ${days} days` : `${-days} days ago`;
}

/** Days of study time, as a student says them (`RecordDetail.minutes`). */
export function minutes(value: number): string {
  if (!Number.isFinite(value)) return '';
  if (value < 60) return `${value} minutes`;
  const hours = Math.floor(value / 60);
  const rest = value % 60;
  return rest === 0 ? `${hours}h` : `${hours}h ${rest}m`;
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

/** `3 Oct` — a day on an axis, and the short form of a stored day. */
export function dayMonth(iso: string | null | undefined): string {
  if (!iso) return '';
  const date = new Date(dayOf(iso));
  return Number.isNaN(date.getTime()) ? '' : `${date.getUTCDate()} ${MONTHS[date.getUTCMonth()]}`;
}

/** `Sat 3 Oct` — the app's own stamped date (`Titlebar`), for an aria sentence. */
export function stamp(iso: string | null | undefined): string {
  if (!iso) return '';
  const date = new Date(dayOf(iso));
  return Number.isNaN(date.getTime())
    ? ''
    : `${WEEKDAYS[date.getUTCDay()]} ${date.getUTCDate()} ${MONTHS[date.getUTCMonth()]}`;
}

/** One value, in the words the rest of the app uses for it (`RecordDetail.cell`). */
export function readingOf(field: FieldRead, record: RecordDoc, today: string | null): string {
  const value = record.fields[field.key];
  if (isDerived(field)) {
    const computed = record.derived?.[field.key];
    if (computed === null || computed === undefined) return 'not worked out yet';
    return typeof computed === 'number' ? String(Math.round(computed * 100) / 100) : String(computed);
  }
  if (value === null || value === undefined || value === '') return '—';
  if (field.type === 'date') return relative(String(value), today) || '—';
  if (field.type === 'duration') return minutes(Number(value)) || '—';
  if (field.type === 'bool') return value ? 'yes' : 'no';
  if (field.type === 'multiSelect') return Array.isArray(value) ? value.join(' · ') : String(value);
  if (Array.isArray(value)) return value.join(' · ');
  if (typeof value === 'object') return '—';
  return String(value);
}

/** A quantity's own number, when the column is one (for a wheel or a rule). */
export function amountOf(field: FieldRead, record: RecordDoc): number | null {
  if (isDerived(field)) {
    const computed = record.derived?.[field.key];
    return typeof computed === 'number' && Number.isFinite(computed) ? computed : null;
  }
  const value = record.fields[field.key];
  if (value === null || value === undefined || value === '') return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

/** The columns the panel carries as quantities: numbers, durations, computed. */
export const QUANTITY_TYPES: Record<string, true> = {
  number: true,
  duration: true,
  formula: true,
  progress: true,
};
