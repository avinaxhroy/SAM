/** Props for RecordView layout blocks (`Block.svelte`). */
import { recordLabel, type BlockNode, type FieldRead, type RecordDoc, type TypeRead } from '../../types';

/**
 * The layout's own word, in the app's own vocabulary (`App.svelte:117-146`).
 * A label, never the `view.setLayout` control: a block cannot write the view's
 * layout from here, and the campaign refuses a control that only pretends to
 * dispatch.
 */
export const LAYOUT_WORDS: Record<string, string> = {
  list: 'List',
  table: 'Table',
  board: 'Board',
  timeline: 'Timeline',
  calendar: 'Calendar',
  tree: 'Outline',
  cardGrid: 'Cards',
  graph: 'Map',
};

export function layoutWord(layout: string | null | undefined): string {
  if (!layout) return 'Layout';
  return LAYOUT_WORDS[layout] ?? layout;
}

/** One record as a drawing reads it: its name, its day, and the day in words. */
export type RecordFact = {
  id: string;
  label: string;
  /** The record's own day (`YYYY-MM-DD`), or null when the kind holds none. */
  day: string | null;
  /** The day as the app prints it (`Wed 30 Sep`), or null. */
  dayWords: string | null;
  /** The distance from the plan's today (`3 days late` · `today` · `in 4 days`). */
  due: string | null;
  /** The record's own instant has passed. */
  late: boolean;
  /** The record's own spoken line: the object's accessible name. */
  sentence: string;
  record: RecordDoc;
};

/** The type's own date field, found through the schema and never guessed from
 *  a value's shape — the rule `RecordTimeline`/`RecordCalendar` already keep. */
export function dateKeyOf(type: TypeRead | undefined): string | null {
  return (
    (type?.fields ?? []).find((field) => field.type === 'date' || field.type === 'daterange')?.key ??
    null
  );
}

const WEEKDAY_SHORT = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

function localDay(iso: string): Date {
  const parts = iso.slice(0, 10).split('-').map(Number);
  return new Date(parts[0], parts[1] - 1, parts[2]);
}

/** A record's day as `YYYY-MM-DD`, or null — *no value*, never a stand-in. */
export function dayOf(record: RecordDoc, key: string | null): string | null {
  const value = key === null ? null : (record.fields[key] ?? record.derived?.[key]);
  if (typeof value === 'string' && value.length >= 10) return value.slice(0, 10);
  // A range sits on the day it starts: a tick, a card and a calendar cell are
  // each one day, not a span band.
  if (typeof value === 'object' && value !== null && 'start' in value) {
    const start = value.start;
    if (typeof start === 'string') return start.slice(0, 10);
  }
  return null;
}

/** The day as a column prints it: `Wed 30 Sep` — never an ISO string. */
export function dayWords(iso: string): string {
  const date = localDay(iso);
  return `${WEEKDAY_SHORT[date.getDay()]} ${date.getDate()} ${MONTHS[date.getMonth()]}`;
}

/** The whole day spelled: `30 Sep 2026`, for a month grid's own heading. */
export function dayMonth(iso: string): string {
  const date = localDay(iso);
  return `${date.getDate()} ${MONTHS[date.getMonth()]}`;
}

/** Whole days between two days, local and DST-corrected — `timeDay`'s `count`. */
export function wholeDays(from: string, to: string): number {
  const a = localDay(from);
  const b = localDay(to);
  const minutes =
    b.getTime() - a.getTime() - (b.getTimezoneOffset() - a.getTimezoneOffset()) * 60000;
  return Math.floor(minutes / 86400000);
}

/** A distance as a student says it — the app's own form (`RecordDetail`). */
export function dueWords(off: number | null): string | null {
  if (off === null) return null;
  if (off === 0) return 'today';
  if (off === 1) return 'tomorrow';
  if (off === -1) return 'yesterday';
  if (off < 0) return `${-off} days late`;
  return `in ${off} days`;
}

/** The sentence the app itself prints for a record block with nothing in it. */
export function factsOf(
  records: RecordDoc[],
  type: TypeRead | undefined,
  today: string | null,
): RecordFact[] {
  const key = dateKeyOf(type);
  return records.map((record) => {
    const day = dayOf(record, key);
    const off = day !== null && today !== null ? wholeDays(today, day) : null;
    const due = dueWords(off);
    const words = day === null ? null : dayWords(day);
    const bits = [recordLabel(record)];
    if (words) bits.push(words);
    if (due) bits.push(due);
    return {
      id: record.id,
      label: recordLabel(record),
      day,
      dayWords: words,
      due,
      late: off !== null && off < 0,
      sentence: bits.join(' · '),
      record,
    };
  });
}

/** The sentence the app itself prints for a record block with nothing in it. */
export function emptyWords(candidates: number, type: string): string {
  return candidates > 0
    ? `Nothing matches this view — ${candidates} ${type} exist.`
    : `No records yet — nothing has been added to this kind.`;
}

/** The fields a relation cell needs resolved, which only the heads of the two
 *  delegated shapes ask for. */
export type TargetsFor = (field: FieldRead) => Array<{ id: string; label: string }>;

/** What every drawing below receives. One contract, one design. */
export type ShapeProps = {
  node: BlockNode;
  /** The type's own schema — the parent edge and the date field live here. */
  type: TypeRead | undefined;
  facts: RecordFact[];
  selected: string | null;
  onSelect: (id: string) => void;
};

/** What `ui/src/blocks/Block.svelte` hands the views design. */
export type ViewsProps = {
  node: BlockNode;
  /** The type's own schema — the app's read, handed in so a design never reads `app`. */
  type: TypeRead | undefined;
  targetsFor: TargetsFor;
  /** The plan's own today (`YYYY-MM-DD`), for a distance in days. */
  today: string | null;
  /** The one id the app has marked (`app.selection`). */
  selected: string | null;
  onSelect: (id: string) => void;
  /** The head's create door: `{type}.new`, the one control the lab draws. */
  onNew: () => void;
};
