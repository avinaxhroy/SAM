/**
 * Component props and date formatting helpers for Problemsets Due / List Card
 * variants (A · The Day Tape, B · The Spike, C · The Tray).
 */
import type { MenuRow } from '../../commands/registry';
import type { RecordDoc } from '../../types';

/** One record of the list card, as a design draws it. */
export type ListCardRow = {
  id: string;
  /** The record's own kind — what the row's door (`record.panel`) needs. */
  type: string;
  /** `recordLabel` — the words the row is called by. */
  label: string;
  /** The row's own accessible sentence: the name and the facts it prints. */
  sentence: string;
  /** The course it belongs to: its own code and identity wash, or null. */
  course: { code: string; wash: string | null } | null;
  /** The record's own planned day, `YYYY-MM-DD`, or null when it holds none. */
  day: string | null;
  /** The day in the app's own words (`26 Sep`), or null — never an ISO string. */
  dayWords: string | null;
  /** The whole day spelled (`Sat 26 Sep 2026`), or null. */
  longDay: string | null;
  /** Whole days from the plan's today: negative is late, null when no day. */
  off: number | null;
  /** The one statement of timing: `3 days late` · `today` · `in 4 days`. */
  due: string | null;
  /** The record's own instant has passed — the late reading, in one flag. */
  late: boolean;
  /** The record, as the engine gave it — what a row menu's own rows are built from. */
  record: RecordDoc;
};

/** What a screen hands `rowsOf`: the least one record has to know. */
export type ListInput = {
  id: string;
  type: string;
  label: string;
  course?: { code: string; wash?: string | null } | null;
  /** The record's date field, exactly as it is stored. */
  day?: string | null;
  record: RecordDoc;
};

/** What every design receives. One contract, three drawers. */
export type ListCardProps = {
  /** The card's own title: the block's title, else `nameOf(view)`. */
  title: string;
  /** The view's id — the card's own `data-view`, which a design never invents. */
  view: string;
  /** The view's kind (`problemset`), which the head's doors are named after. */
  type: string;
  /** The records the read returned, in the read's own order. */
  rows: ListCardRow[];
  /** How many records the view considered — the count line's own denominator. */
  candidates: number;
  /** The plan's own today (`YYYY-MM-DD`), for a distance; null before the read lands. */
  today: string | null;
  /** The record the app has marked (`app.selection`) — one id for the whole app. */
  selected: string | null;
  /** The row whose own menu is open, or null. */
  menuFor: string | null;
  /** The row menu's rows, built by the screen (copy-json … record.delete). */
  menuRows: (row: ListCardRow) => MenuRow[];

  /** A press on a record: the app's own selection, which sets and never toggles. */
  onSelect: (id: string) => void;
  /** The record's own door — the detail panel (`record.panel`). */
  onPanel: (row: ListCardRow) => void;
  /** Open or close a row's own menu; null closes it. */
  onMenu: (id: string | null) => void;
  /** The head's three doors, which the shipped card rendered. */
  onNew: () => void;
  onPaste: () => void;
  /** Not drawn by this screen: the shipped control's field derivation is broken
   *  on a kind whose first number field is a quantity. */
  onRenumber?: () => void;
  /** C's two spills: the pipeline's last stage, and the planned day moved on. */
  onDone: (row: ListCardRow) => void;
  onLater: (row: ListCardRow) => void;
};

const WEEKDAY_SHORT = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** An ISO day as a local Date at midnight — the plan's days are the student's own. */
function localDay(iso: string): Date {
  const parts = iso.slice(0, 10).split('-').map(Number);
  return new Date(parts[0], parts[1] - 1, parts[2]);
}

/** A day shifted by `days` — C's `Later`, which moves the record's planned day. */
export function shiftDay(iso: string, days: number): string {
  const date = localDay(iso);
  date.setDate(date.getDate() + days);
  // From local parts, never `toISOString`, which shifts the day.
  const parts = [date.getFullYear(), date.getMonth() + 1, date.getDate()].map((part) =>
    String(part).padStart(2, '0'),
  );
  return `${parts[0]}-${parts[1]}-${parts[2]}`;
}

/**
 * The whole days between two ISO days, local and DST-corrected — `timeDay`'s
 * own `count` (`refs/d3-time`): the ends' offset difference is removed, which
 * is the whole reason a day *count* is not a subtraction.
 */
export function wholeDays(from: string, to: string): number {
  const a = localDay(from);
  const b = localDay(to);
  const minutes = (b.getTime() - a.getTime()) - (b.getTimezoneOffset() - a.getTimezoneOffset()) * 60000;
  return Math.floor(minutes / 86400000);
}

/**
 * A distance as a student says it — the app's own form (`RecordDetail`), and
 * the one statement of timing a row carries. `null` when the record holds no
 * day at all: "no date" is not "today".
 */
export function dueWords(off: number | null): string | null {
  if (off === null) return null;
  if (off === 0) return 'today';
  if (off === 1) return 'tomorrow';
  if (off === -1) return 'yesterday';
  if (off < 0) return `${-off} days late`;
  return `in ${off} days`;
}

/** The day as the app prints it in a column: `26 Sep`. */
export function dayMonth(iso: string): string {
  const date = localDay(iso);
  return `${date.getDate()} ${MONTHS[date.getMonth()]}`;
}

/** The whole day spelled: `Sat 26 Sep 2026`. */
export function longDay(iso: string): string {
  const date = localDay(iso);
  return `${WEEKDAY_SHORT[date.getDay()]} ${date.getDate()} ${MONTHS[date.getMonth()]} ${date.getFullYear()}`;
}

/**
 * One cell of A's tape: a day, its numeral, its weekday, and whether it is the
 * plan's today — the day the tape's rule is printed at.
 */
export type TapeCell = {
  day: string;
  num: number;
  weekday: string;
  long: string;
  today: boolean;
  /** Days from today: negative is behind the rule, positive ahead of it. */
  off: number;
};

/**
 * Generates tape calendar cells spanning from earliest due date to latest,
 * including 4 padding cells on each side, capped at 62 cells.
 */
export function tapeOf(rows: ListCardRow[], today: string | null): TapeCell[] {
  const days = rows.map((row) => row.day).filter((day): day is string => day !== null);
  if (days.length === 0 || today === null) return [];
  const sorted = [...days].sort();
  const first = sorted[0] < today ? sorted[0] : today;
  const last = sorted[sorted.length - 1] > today ? sorted[sorted.length - 1] : today;
  const from = shiftDay(first, -4);
  const to = shiftDay(last, 4);
  const cells: TapeCell[] = [];
  let day = from;
  while (day <= to && cells.length < 62) {
    cells.push({
      day,
      num: localDay(day).getDate(),
      weekday: WEEKDAY_SHORT[localDay(day).getDay()],
      long: longDay(day),
      today: day === today,
      off: wholeDays(today, day),
    });
    day = shiftDay(day, 1);
  }
  return cells;
}

/** The day of one row as tape coordinates: the cell index and how many cells
 *  its bar spans from the rule. Null when the row holds no day, or the day is
 *  outside the cells the tape could draw. */
export function barOf(cells: TapeCell[], row: ListCardRow): { index: number; span: number } | null {
  if (row.day === null) return null;
  const index = cells.findIndex((cell) => cell.day === row.day);
  const rule = cells.findIndex((cell) => cell.today);
  if (index < 0 || rule < 0) return null;
  return { index, span: Math.abs(index - rule) };
}

/** The sentence the app itself prints for a record block with nothing in it. */
export function emptyWords(candidates: number, type: string): string {
  return candidates > 0
    ? `Nothing matches this view — ${candidates} ${type} exist.`
    : `No records yet — nothing has been added to this kind.`;
}

/** The rows a design draws, derived once for all three. */
export function rowsOf(inputs: ListInput[], today: string | null): ListCardRow[] {
  return inputs.map((input) => {
    const day = typeof input.day === 'string' && input.day.length >= 10 ? input.day.slice(0, 10) : null;
    const off = day !== null && today !== null ? wholeDays(today, day) : null;
    const due = dueWords(off);
    const course = input.course ? { code: input.course.code, wash: input.course.wash ?? null } : null;
    const bits = [input.label];
    if (course) bits.push(course.code);
    if (due !== null) bits.push(due);
    return {
      id: input.id,
      type: input.type,
      label: input.label,
      sentence: bits.join(' · '),
      course,
      day,
      dayWords: day === null ? null : dayMonth(day),
      longDay: day === null ? null : longDay(day),
      off,
      due,
      late: off !== null && off < 0,
      record: input.record,
    };
  });
}
