/** Props for MocksCalendar surface variants. */
export type MockCourse = {
  id: string;
  /** The course's own code (`CS201`), which is what a student says out loud. */
  code: string | null;
  label: string;
  /** The identity wash the engine assigned the course, from `app.washes`. */
  wash: string | undefined;
};

/** One assessment, as the screen hands it to every design. */
export type MockRecord = {
  id: string;
  type: string;
  /** The record's own name, through the app's `recordLabel`. */
  label: string;
  /** The plan's own word for the test (`weekly test`, `full mock`), or null. */
  kind: string | null;
  /** Its day, as a plan key — null when nothing has been set yet. */
  day: string | null;
  /** The week the plan links it to, named the way the plan names weeks. */
  week: string | null;
  /** The score the plan holds, when it holds one. */
  score: number | null;
  courses: MockCourse[];
};

/** The plan's own today, as the key it reasons with and in words. */
export type MockToday = { key: string; spoken: string };

export type MocksProps = {
  /** The screen's own title (the navigation entry's name). */
  title: string;
  /** True while the plan is being read: each design draws its own loading shape. */
  loading: boolean;
  today: MockToday;
  /** The day the calendar should open on — the plan's next assessment, or today. */
  anchor: string;
  /** Every assessment that carries a day, in the plan's own order. */
  dated: MockRecord[];
  /** Every assessment still waiting for one. */
  waiting: MockRecord[];
  /** The quiet weekdays, earliest first — one to three plan keys. */
  quiet: string[];
  /** The record door this screen creates (`assessment.new`), or null. */
  newCommand: string | null;
  /** The door's own words, through the app's `nameOf` (`New assessment`). */
  newLabel: string;
  onNew: () => void;
  /** Replay the read — the head's own ⟳ control. */
  onRead: () => void;
  /** The one write: put a record on a day. Resolves false when the engine refused. */
  onSetDay: (id: string, day: string) => Promise<boolean>;
  /** The app's own undo, for a receipt's Undo. */
  onUndo: () => void;
};

/* ── days, said the way the plan says them ─────────────────────────────── */

const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
];
const MONTHS_SHORT = [
  'JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC',
];
const WEEKDAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
const WEEKDAYS_SHORT = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'];

/** `YYYY-MM-DD` read as a local calendar day — never through UTC, which shifts it. */
export function dateOf(key: string): Date {
  const [year, month, day] = key.split('-').map(Number);
  return new Date(year, month - 1, day);
}

/** `YYYY-MM-DD` from local parts. */
export function dayKeyOf(date: Date): string {
  const parts = [date.getFullYear(), date.getMonth() + 1, date.getDate()].map((part) =>
    String(part).padStart(2, '0'),
  );
  return `${parts[0]}-${parts[1]}-${parts[2]}`;
}

/** Whole days from one plan key to another. */
export function daysBetween(from: string, to: string): number {
  return Math.round((dateOf(to).getTime() - dateOf(from).getTime()) / 86_400_000);
}

/** A day a student says out loud: `Sunday 4 October`. */
export function spokenDay(key: string): string {
  const date = dateOf(key);
  return `${WEEKDAYS[date.getDay()]} ${date.getDate()} ${MONTHS[date.getMonth()]}`;
}

/** A day in the rail's own caps: `SUN 4 OCT`. */
export function capsDay(key: string): string {
  const date = dateOf(key);
  return `${WEEKDAYS_SHORT[date.getDay()]} ${date.getDate()} ${MONTHS_SHORT[date.getMonth()]}`;
}

/** The distance to a day, in the words §7.3 uses. */
export function distanceText(todayKey: string, day: string): string {
  const n = daysBetween(todayKey, day);
  if (n === 0) return 'today';
  if (n === 1) return 'tomorrow';
  if (n === -1) return 'yesterday';
  return n > 0 ? `in ${n} days` : `${Math.abs(n)} days ago`;
}

/** The countdown chip: past → **overdue**, today and tomorrow → **risk**, else **ok**. */
export function countdownOf(
  todayKey: string,
  day: string | null,
): { tone: 'overdue' | 'risk' | 'ok' | 'info'; word: string } {
  if (day === null) return { tone: 'info', word: 'no day yet' };
  const n = daysBetween(todayKey, day);
  if (n < 0) {
    const late = Math.abs(n);
    return { tone: 'overdue', word: late === 1 ? '1 day late' : `${late} days late` };
  }
  if (n === 0) return { tone: 'risk', word: 'today' };
  if (n === 1) return { tone: 'risk', word: 'tomorrow' };
  return { tone: 'ok', word: `in ${n} days` };
}

/**
 * The date's own level mark, and it answers ONE question: where the date is.
 * `overdue` only when the day has gone, `soon` for today, tomorrow and the day
 * after, `next` for everything else — including a record with no day at all,
 * because nothing about an undated assessment is pressing.
 */
export function levelOf(todayKey: string, day: string | null): 'overdue' | 'soon' | 'next' {
  if (day === null) return 'next';
  const n = daysBetween(todayKey, day);
  if (n < 0) return 'overdue';
  if (n <= 2) return 'soon';
  return 'next';
}

/** `4 assessments` — the count a day or a group states. */
export function dayCount(n: number): string {
  return `${n} ${n === 1 ? 'assessment' : 'assessments'}`;
}

/** Whole months between two plan keys, measured on their first days. */
export function monthsBetween(fromKey: string, toKey: string): number {
  const from = dateOf(fromKey);
  const to = dateOf(toKey);
  return (to.getFullYear() - from.getFullYear()) * 12 + (to.getMonth() - from.getMonth());
}

/** One day of a month, as the grid and the rail read it. */
export type MonthDay = {
  key: string;
  num: number;
  today: boolean;
  records: MockRecord[];
};

export type Month = { name: string; /** Monday-first lead-in, 0–6. */ lead: number; days: MonthDay[] };

/** The month `offset` months from the plan's today, with each day's records. */
export function monthOf(
  todayKey: string,
  offset: number,
  byDay: Map<string, MockRecord[]>,
): Month {
  const base = dateOf(todayKey);
  const first = new Date(base.getFullYear(), base.getMonth() + offset, 1);
  const length = new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate();
  const days: MonthDay[] = [];
  for (let i = 0; i < length; i += 1) {
    const date = new Date(first.getFullYear(), first.getMonth(), 1 + i);
    const key = dayKeyOf(date);
    days.push({ key, num: date.getDate(), today: key === todayKey, records: byDay.get(key) ?? [] });
  }
  return {
    name: `${MONTHS[first.getMonth()]} ${first.getFullYear()}`,
    lead: (first.getDay() + 6) % 7,
    days,
  };
}

/** The month as weeks of seven tracks, Monday first — null is a day outside it. */
export function weeksOf(month: Month): (MonthDay | null)[][] {
  const cells: (MonthDay | null)[] = [
    ...Array.from({ length: month.lead }, () => null),
    ...month.days,
  ];
  while (cells.length % 7 !== 0) cells.push(null);
  const weeks: (MonthDay | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  return weeks;
}

/** Where a day sits in the month's grid: its week and its column (1–7). */
export function seatOf(weeks: (MonthDay | null)[][], key: string): { week: number; col: number } | null {
  for (let i = 0; i < weeks.length; i += 1) {
    for (let j = 0; j < 7; j += 1) {
      if (weeks[i][j]?.key === key) return { week: i, col: j + 1 };
    }
  }
  return null;
}

/** The day of the month with the most on it — the rail's own resting pick. */
export function busiestDay(month: Month): MonthDay {
  let best: MonthDay | null = null;
  for (const day of month.days) {
    if (day.records.length === 0) continue;
    if (best === null || day.records.length > best.records.length) best = day;
  }
  return best ?? month.days[0];
}

/**
 * The rail's own proposal, in assign mode: the earliest weekday in the month on
 * screen that carries nothing and has not gone — never a day the plan already
 * filled, and never a day in the past.
 */
export function quietInMonth(todayKey: string, month: Month): string {
  for (const day of month.days) {
    const weekday = dateOf(day.key).getDay();
    if (day.key < todayKey || weekday === 0 || weekday === 6 || day.records.length > 0) continue;
    return day.key;
  }
  return month.days[0].key;
}

/** The next day after `key` that carries something, or null. */
export function nextBusy(month: Month, key: string): MonthDay | null {
  for (const day of month.days) {
    if (day.key > key && day.records.length > 0) return day;
  }
  return null;
}

/** What else is already on a day — read live, never promised from load. */
export function dayCompany(
  byDay: Map<string, MockRecord[]>,
  day: string,
  exceptId: string | null,
): string {
  const others = (byDay.get(day) ?? []).filter((record) => record.id !== exceptId).length;
  if (others <= 0) return 'nothing else on it';
  return others === 1 ? '1 other assessment' : `${others} other assessments`;
}

/** The days' records, keyed by plan day — the one map all three designs read. */
export function recordsByDay(dated: MockRecord[]): Map<string, MockRecord[]> {
  const byDay = new Map<string, MockRecord[]>();
  for (const record of dated) {
    if (record.day === null) continue;
    const bucket = byDay.get(record.day);
    if (bucket) bucket.push(record);
    else byDay.set(record.day, [record]);
  }
  return byDay;
}

/** Records under a day in the plan's order: by day, then by name. */
export function byDayOrder(a: MockRecord, b: MockRecord): number {
  return String(a.day ?? '').localeCompare(String(b.day ?? '')) || a.label.localeCompare(b.label);
}

/**
 * The next assessment the plan has not passed yet — or, when every dated one
 * has gone, the most recent of them. It is the screen's own answer to "when is
 * the next one", and the head line, the calendar's opening month and the rail's
 * resting pick all read it, so no two of them can disagree.
 */
export function nextUp(dated: MockRecord[], todayKey: string): MockRecord | null {
  const ordered = [...dated].sort(byDayOrder);
  return ordered.find((record) => (record.day ?? '') >= todayKey) ?? ordered[ordered.length - 1] ?? null;
}

/** The head's one line: how many assessments, and when the nearest one falls —
 *  stated in the right tense when the plan's dated work is all behind us. */
export function headLineOf(records: MockRecord[], dated: MockRecord[], todayKey: string): string {
  const noun = records.length === 1 ? 'assessment' : 'assessments';
  if (dated.length === 0) return `${records.length} ${noun} · none of them has a day yet`;
  const next = nextUp(dated, todayKey);
  if (next?.day == null) return `${records.length} ${noun} in the plan`;
  const when =
    next.day >= todayKey
      ? `the next is ${distanceText(todayKey, next.day)}`
      : `the last one was ${distanceText(todayKey, next.day)}`;
  return `${records.length} ${noun} · ${when}, ${spokenDay(next.day)}`;
}
