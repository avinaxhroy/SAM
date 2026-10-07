/**
 * Reviews queue surface variant component props.
 * Organizes overdue review cards by lateness windows and supports inline deferrals.
 */
export type QueueRow = {
  id: string;
  title: string;
  /** The course's own code, and its wash — what a chip says out loud. */
  course: string | null;
  wash: string | undefined;
  /** Whole days past the record's own date; 0 means it came due today. */
  late: number;
  /** The state word: `due today` · `4 days late`. */
  state: string;
  /** Which chip species carries the state (the system's four form-per-state
   *  marks, `desktop` §7.3): overdue at a week, risk under it, info at today. */
  tone: 'overdue' | 'risk' | 'info';
  /** The day its lateness counts back to (`Mon 5 Oct`) — the pocket's return line. */
  due: string;
};

/** One age window, with its records. The window's name and its rule are one
 *  fact, so both are handed over rather than recomputed by a drawing. */
export type QueueGroup = {
  id: string;
  label: string;
  rule: string;
  rows: QueueRow[];
};

/** A record this sitting took out of the backlog, with what became of it. */
export type QueueMoved = { id: string; title: string; say: string };

export type QueueProps = {
  /** The four lateness windows, in the order the plan ages in. */
  groups: QueueGroup[];
  /** Every record of the backlog, oldest first — the design whose axis is the
   *  row rather than the window. */
  rows: QueueRow[];
  total: number;
  /** How late the oldest record is, in days. */
  oldestLate: number;
  dueToday: number;
  /** How many are a week old or more — A's own head says it. */
  weekOld: number;
  /** What the sitting moved: the defers (every design that can defer) and the
   *  grades (the record of what the card above has answered). */
  moved: { deferred: QueueMoved[]; graded: QueueMoved[] };
  /** The days a defer can offer, derived from the plan's own today — the next
   *  four things a student could actually sit down for. */
  days: { days: number; label: string }[];
  /** Put this record on the recall card above. */
  onPut: (id: string) => void;
  /** Push one record to a day that many days from the plan's today. */
  onDefer: (id: string, days: number) => Promise<void>;
  /** Reverse the write that took a record out of the backlog. */
  onUndo: () => Promise<void>;
};

/** The chip species a row's state wears. */
export function stateClass(tone: QueueRow['tone']): string {
  if (tone === 'overdue') return 'cd-chip--overdue';
  if (tone === 'risk') return 'cd-chip--risk';
  return 'cd-chip--info';
}
