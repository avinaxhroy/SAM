/** Props for ComingUp variants. */
import { durationText } from '../../types';

/** Formatted display date broken into parts. */
export type ComingUpDate = {
  weekday: string;
  day: string;
  month: string;
  long: string;
  time: string | null;
};

/** The one statement of timing a row carries. */
export type ComingUpState = { tone: 'overdue' | 'risk' | 'ok'; word: string };

/** One dated thing, as a design draws it. */
export type ComingUpRow = {
  id: string;
  /** The record's own kind, for a design that wants to speak of it. */
  kind: string;
  /** The title, named the way the screen names it (never an ISO date). */
  label: string;
  /** The course it belongs to, with the identity wash, or null. */
  course: { label: string; wash: string | undefined } | null;
  /** The date as words and parts, or null when the record carries no date. */
  date: ComingUpDate | null;
  /** How many days late — 0 when it is not late. */
  lateDays: number;
  /** Days from today, or null when the record carries no date. */
  daysUntil: number | null;
  /** The record's own estimate in minutes, when the kind declares one. */
  est: number | null;
  /** The one statement of timing, or null when the record has no date. */
  state: ComingUpState | null;
  /** A session is a block of minutes already spent, and says so. */
  logged: boolean;
};

/**
 * What a screen hands `rowsOf`: the least a dated thing has to know. `TodayItem`
 * satisfies it as it stands, and a record read elsewhere maps into it without
 * this file learning about that read.
 */
export type DatedInput = {
  id: string;
  kind: string;
  label: string;
  course?: { label: string; wash?: string } | null;
  dueDate?: string | null;
  lateDays?: number;
  daysUntil?: number | null;
  est?: number | null;
};

/** What every design receives. */
export type ComingUpProps = {
  /** The dated rows, in the screen's own order (the read already sorts them). */
  rows: ComingUpRow[];
  /** False when the plan is not open — the empty state's own door says so. */
  planReady: boolean;
  /** The empty state's door: the plan, which is where a date is put. */
  onOpenPlan: () => void;
  /** The record door a row's press takes (B · C — A's cards are not doors). */
  onOpen: (id: string) => void;
};

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** A record whose `label` IS its date (`2026-09-27`) — `types.ts`'s warning. */
const ISO_TITLE = /^\d{4}-\d{2}-\d{2}$/;

/** The record kind as a student would say it, for a record that has no name. */
function kindWords(kind: string): string {
  switch (kind) {
    case 'session': return 'Study session';
    case 'assessment': return 'Assessment';
    case 'problemset': return 'Question set';
    case 'resource': return 'Resource';
    case 'note': return 'Note';
    default: return 'Something to do';
  }
}

/** The title a row shows: the record's own name, or what the record *is*. */
function titleOf(input: DatedInput): string {
  if (input.label && !ISO_TITLE.test(input.label)) return input.label;
  return kindWords(input.kind);
}

/** The date as parts and words, or null when there is no usable date.
 *
 *  `dueDate` is an ISO string and the type doc is explicit that no screen parses
 *  one, so it is handed to `Date` once here and every word below is taken off
 *  the parts. A clock is printed only when the string itself names one: a
 *  day-only record is a whole day, and midnight UTC is not its time. */
function dateOf(input: DatedInput): ComingUpDate | null {
  const dueDate = input.dueDate ?? null;
  if (!dueDate) return null;
  const at = new Date(dueDate);
  if (Number.isNaN(at.getTime())) return null;
  const weekday = WEEKDAYS[at.getDay()];
  const day = String(at.getDate());
  const month = MONTHS[at.getMonth()];
  const timed = /T\d{2}:\d{2}/.test(dueDate) && input.kind !== 'session';
  const time = timed
    ? `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}`
    : null;
  return { weekday, day, month, long: `${weekday} ${day} ${month}`, time };
}

/** The one statement of timing: lateness, today, tomorrow, or a countdown. */
function stateOf(input: DatedInput): ComingUpState | null {
  const late = input.lateDays ?? 0;
  if (late > 0) return { tone: 'overdue', word: late === 1 ? '1 day late' : `${late} days late` };
  const off = input.daysUntil ?? null;
  if (off === null) return null;
  if (off < 0) return { tone: 'overdue', word: off === -1 ? '1 day late' : `${-off} days late` };
  if (off === 0) return { tone: 'risk', word: 'today' };
  if (off === 1) return { tone: 'risk', word: 'tomorrow' };
  return { tone: 'ok', word: `in ${off} days` };
}

/** The rows a design draws, derived once for all three. */
export function rowsOf(items: DatedInput[]): ComingUpRow[] {
  return items.map((item) => ({
    id: item.id,
    kind: item.kind,
    label: titleOf(item),
    course: item.course ? { label: item.course.label, wash: item.course.wash } : null,
    date: dateOf(item),
    lateDays: item.lateDays ?? 0,
    daysUntil: item.daysUntil ?? null,
    est: item.est ?? null,
    state: stateOf(item),
    logged: item.kind === 'session',
  }));
}

/** Primary timing indicator: clock time if present, session logged status, or duration estimate. */
export function leadOf(row: ComingUpRow): string | null {
  if (row.date?.time) return row.date.time;
  if (row.logged) return 'already logged';
  if (row.est !== null) return durationText(row.est);
  return null;
}

