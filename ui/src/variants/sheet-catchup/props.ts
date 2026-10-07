/** Props for Catchup sheet variants. */

/** One overdue record, as a card states it: a name and at most two facts. */
export type CatchupItem = {
  id: string;
  /** The record's own name (`recordLabel`), never its id. */
  label: string;
  /** The course's code, when the plan names one — the identity chip's label. */
  course: string | null;
  /** The identity wash the engine assigns (`mint` · `lilac` · `butter` · `sky`). */
  wash: string | null;
  /** `2 days late`, in the app's own words. */
  late: string | null;
};

/** One of the three dates the backlog may move to, already spelled. */
export type CatchupDate = {
  /** The lab's own key for the date, and the variant's state key. */
  key: string;
  /** The offset the key stands for, in days from today. */
  days: number;
  /** The app's own name for the offset: `Tomorrow`, `In 3 days`, `Next week`. */
  name: string;
  /** The day numeral the date plate wears (`28`). */
  tile: string;
  /** `28 Sep` — the short form, for a foot or a line. */
  short: string;
  /** `Mon 28 Sep` — the full form, for a card's aria and the receipt. */
  full: string;
  /** The date the write carries (`2026-09-28`). Never rendered. */
  iso: string;
};

export type CatchupProps = {
  /** The surface's own title, as the panel states it (`Move a backlog`). */
  title: string;
  /** The panel's one-line fact under it (`4 overdue recalls`). */
  note: string;
  items: CatchupItem[];
  dates: CatchupDate[];
  /**
   * The write: one `record.defer` for the batch on one date. Answers true when
   * the batch landed, false when the engine refused it (the panel has already
   * said why) — a design shows its receipt only for a write that happened.
   */
  onCommit: (ids: string[], iso: string) => Promise<boolean>;
  /** Take the write back (`app.undo()`), the receipt's own door. */
  onUndo: () => void;
  /** Put the surface away. */
  onClose: () => void;
};

/** The three offsets the shipped panel has always offered, in its order. */
export const OFFSETS: Array<{ key: string; days: number; name: string }> = [
  { key: '1', days: 1, name: 'Tomorrow' },
  { key: '3', days: 3, name: 'In 3 days' },
  { key: '7', days: 7, name: 'Next week' },
];

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** A day, in days from an ISO day — the panel's own arithmetic, moved here so
 *  the date a design draws and the date the write carries cannot disagree. */
export function addDays(iso: string, days: number): string {
  const [year, month, day] = iso.split('-').map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

/** The three dates the panel hands every design, from the plan's own today. */
export function datesFrom(today: string): CatchupDate[] {
  return OFFSETS.map((offset) => {
    const iso = addDays(today, offset.days);
    const [year, month, day] = iso.split('-').map(Number);
    const at = new Date(Date.UTC(year, month - 1, day));
    return {
      key: offset.key,
      days: offset.days,
      name: offset.name,
      tile: String(day),
      short: `${day} ${MONTHS[month - 1]}`,
      full: `${WEEKDAYS[at.getUTCDay()]} ${day} ${MONTHS[month - 1]}`,
      iso,
    };
  });
}

/** `1 day late` · `3 days late` — the same sentence Reviews prints (its `latePhrase`). */
export function lateness(days: number): string {
  return days === 1 ? '1 day late' : `${days} days late`;
}

/** `1 recall` · `4 recalls`. */
export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** `4 recalls come back Mon 28 Sep` — the receipt's own sentence. */
export function arrivalLine(n: number, when: string): string {
  return `${n === 1 ? '1 recall comes' : `${n} recalls come`} back ${when}`;
}

/** `4 recalls filed` · `3 recalls filed · 1 staying put`. */
export function filedLine(n: number, staying: number): string {
  const head = `${plural(n, 'recall', 'recalls')} filed`;
  return staying === 0 ? head : `${head} · ${staying} staying put`;
}
