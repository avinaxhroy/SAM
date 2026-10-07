/** Props for Today queue variants. */
import { durationText, type TodayGroupId, type TodayItem } from '../../types';

/** One row of the day, exactly as the panel's own `rows` read draws it. */
export type QueueRow = {
  item: TodayItem;
  /** The group the plan put it in. */
  group: TodayGroupId;
  /** That group's label, in the screen's words (`Late for review`). */
  label: string;
};

/** One group of the day: how many it holds, out of how many the read sent. */
export type QueueGroup = {
  id: TodayGroupId;
  label: string;
  count: number;
  sent: number;
};

export type Props = {
  /** The day's queue, in the plan's order, grouped as the screen groups it. */
  rows: QueueRow[];
  /** Group totals and counts returned by the query. */
  groups: QueueGroup[];
  /** The plan's own time-logging command id (`session.new`), or null. */
  sessionCommand: string | null;
  /** The length a session runs for, in minutes (the duration capsule). */
  targetMin: number;
  /** The item a session is running on, or null. */
  runningId: string | null;
  /** The epoch ms the running session started, when the panel knows it: the
   *  run chip's clock. Null draws the plain state word instead of a clock. */
  runningSince: number | null;
  onStart: (item: TodayItem) => void;
  /** Stop the session that is running. */
  onStop: () => void;
  /** Log the target length on this item without running a timer. */
  onLog: (item: TodayItem) => void;
  /** Plan this item for today — the app's own `record.setField` door. */
  onAdd: (item: TodayItem) => void;
  /** Take this item off today — the same field, cleared. */
  onRemove: (item: TodayItem) => void;
  /** The card head's Add: the `today.add` sheet. */
  onAddToToday: () => void;
};

/* ── the three states the designs say out loud ─────────────────────────────
   The plan's five buckets are three facts a student reads: late, due, and
   everything the plan has scheduled next (`committed` is the student's own
   "today" and `stale` is a lapse in review — both are work the plan already
   holds, which is what `planned` means here). */
export type QueueState = 'late' | 'due' | 'planned';

export function stateOf(group: TodayGroupId): QueueState {
  if (group === 'late') return 'late';
  if (group === 'due') return 'due';
  return 'planned';
}

/** The four things that are dated (late or due) — the ones the eye lands on. */
export function isDated(state: QueueState): boolean {
  return state !== 'planned';
}

/**
 * The row's state in the app's own words, or null when the record has no date
 * to state — and a chip with no word is worse than no chip, so the callers
 * omit it rather than print a placeholder.
 */
export function stateWord(item: TodayItem): string | null {
  if (item.lateDays > 0) return item.lateDays === 1 ? '1 day late' : `${item.lateDays} days late`;
  if (item.daysUntil === null) return null;
  if (item.daysUntil < 0) {
    const late = Math.abs(item.daysUntil);
    return late === 1 ? '1 day late' : `${late} days late`;
  }
  if (item.daysUntil === 0) return 'due today';
  if (item.daysUntil === 1) return 'tomorrow';
  if (item.daysUntil <= 7) return `in ${item.daysUntil} days`;
  return null;
}

/** The chip's tone, from the same fact the word is written from. */
export function stateTone(item: TodayItem): 'overdue' | 'risk' | 'ok' | null {
  if (item.lateDays > 0) return 'overdue';
  if (item.daysUntil === null) return null;
  if (item.daysUntil < 0) return 'overdue';
  if (item.daysUntil <= 1) return 'risk';
  return 'ok';
}

/** Formatted estimate string or 'no estimate' if unset. */
export function minutesText(item: TodayItem): string {
  return item.est === null ? 'no estimate' : `${item.est} min`;
}

/** The same fact for the ear (`45 minutes`). */
export function minutesWords(item: TodayItem): string {
  return item.est === null ? 'no estimate' : `${item.est} minutes`;
}

/** The course chip, when the record belongs to a course. */
export function courseOf(item: TodayItem): { label: string; wash?: string } | null {
  return item.course ? { label: item.course.label, wash: item.course.wash } : null;
}

/**
 * The four courses the day's queue actually holds, in first-appearance order —
 * the plan's own order, which is the order the strip states them in. Gathered
 * by label because that is the fact the chips print, and a plan that names two
 * courses the same way is one course to a reader.
 */
export function coursesOf(rows: QueueRow[]): Array<{ label: string; wash?: string }> {
  const seen = new Map<string, { label: string; wash?: string }>();
  for (const row of rows) {
    const course = courseOf(row.item);
    if (course && !seen.has(course.label)) seen.set(course.label, course);
  }
  return [...seen.values()];
}

/**
 * The title a row shows. `today.view` can hand back a record whose `label` **is
 * its date** (a `session` declares no title, so the read falls back to the one
 * field it has), and an ISO date on a screen is D2's exact prohibition. The
 * record is then named by what it is — the panel's own rule, kept here so all
 * three designs name it the same way.
 */
const ISO_TITLE = /^\d{4}-\d{2}-\d{2}$/;

export function titleOf(item: TodayItem): string {
  if (item.label && !ISO_TITLE.test(item.label)) return item.label;
  return kindWords(item.kind);
}

function kindWords(kind: string): string {
  switch (kind) {
    case 'session':
      return 'Study session';
    case 'assessment':
      return 'Assessment';
    case 'problemset':
      return 'Question set';
    case 'resource':
      return 'Resource';
    case 'note':
      return 'Note';
    default:
      return 'Something to do';
  }
}

/** The minutes the rows' own estimates add up to (`no estimate` adds nothing). */
export function minutesSum(rows: QueueRow[]): number {
  return rows.reduce((total, row) => total + (row.item.est ?? 0), 0);
}

/** Whether any row of the slice carries no estimate — the `about` hedge. */
export function hedged(rows: QueueRow[]): boolean {
  return rows.some((row) => row.item.est === null);
}

/**
 * The work the slice holds, in words. A total over a day hedges with `about`
 * whenever one thing of it has no estimate, and a slice with nothing estimated
 * at all says so rather than printing `0 min`.
 */
export function workLine(rows: QueueRow[]): string {
  const minutes = minutesSum(rows);
  if (rows.length === 0 || minutes === 0) return 'no estimate';
  return `${hedged(rows) ? 'about ' : ''}${durationText(minutes)} of work`;
}

/** `2 things` · `1 thing`. */
export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** The slice's size and its work: `9 things · about 4 h 00 min of work`. */
export function sizeLine(rows: QueueRow[]): string {
  return `${plural(rows.length, 'thing', 'things')} · ${workLine(rows)}`;
}

/**
 * The day's whole size, from the groups' own counts — never from the rows on
 * screen. `19 of 97 things` when the read capped the day, `19 things` when it
 * sent all of it. This is the shipped card's own pair (`12 of 25`) kept as the
 * fact it is.
 */
export function queueSize(groups: QueueGroup[]): string {
  const count = groups.reduce((total, group) => total + group.count, 0);
  const sent = groups.reduce((total, group) => total + group.sent, 0);
  return sent < count ? `${sent} of ${count} things` : plural(sent, 'thing', 'things');
}

/**
 * The same pair for a slice that is a whole set of groups (a state filter, or
 * the whole queue) — `null` when there is nothing to state, which is the case
 * for a slice of one course: the read sends the capped 12 of `next`, so the
 * slice's own total is not knowable and the design states what it shows instead
 * of a number it cannot check.
 */
export function sliceSize(rows: QueueRow[], groups: QueueGroup[]): string | null {
  const states = new Set(rows.map((row) => stateOf(row.group)));
  const whole = groups.every((group) => {
    const inSlice = rows.some((row) => row.group === group.id);
    const inState = states.has(stateOf(group.id));
    if (inState) return true;
    return !inSlice && group.sent === group.count;
  });
  if (!whole) return null;
  const parts = groups.filter((group) => states.has(stateOf(group.id)));
  const count = parts.reduce((total, group) => total + group.count, 0);
  const sent = parts.reduce((total, group) => total + group.sent, 0);
  return sent < count ? `${sent} of ${count} things` : plural(sent, 'thing', 'things');
}
