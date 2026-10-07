/** Props for RecordTable surface variants. */
import type { MenuRow } from '../../commands/registry';
import type { FieldRead, RecordDoc } from '../../types';

/** One cell write, as the app remembers it while it is in flight. */
export type CellWrite = { state: 'saving' | 'saved' | 'failed'; reason?: string };

/** One column of the view, as a design draws it. */
export type CellFact = {
  key: string;
  /** Column display label (`labelOf`). */
  label: string;
  /** Field type name, or 'unknown'. */
  type: string;
  /** Field declaration definition if valid. */
  field: FieldRead | undefined;
  /** Relation target records (§C.1); empty for other field types. */
  targets: Array<{ id: string; label: string }>;
  /** Formatted cell text representation. */
  text: string;
  /** In-flight write state for this cell. */
  write: CellWrite | null;
};

/** The record's primary date instant. */
export type InstantFact = {
  key: string;
  label: string;
  /** Relative date string (e.g. 'in 3 days', 'today'). */
  words: string;
  /** True if the record has a date set. */
  set: boolean;
  /** True if the date is overdue. */
  late: boolean;
};

/** One row of the table, as a design draws it. */
export type RowFact = {
  id: string;
  /** Display label from `recordLabel`. */
  label: string;
  /** Accessible row summary sentence for screen readers. */
  sentence: string;
  record: RecordDoc;
  kind: string;
  /** Course identity wash if assigned. */
  wash: string | undefined;
  mark: { text: string; wash: string | undefined } | null;
  cells: CellFact[];
  /** Primary date instant if present. */
  instant: InstantFact | null;
  /** Error messages for rejected mutations. */
  failures: string[];
};

/** The column strip's own wiring, built once by the screen. */
export type ColumnFact = {
  key: string;
  label: string;
  type: string;
  /** Everything `ColumnMenu` takes, wired to this column. */
  menu: {
    field: FieldRead;
    type: string;
    view: string;
    previous: FieldRead | null;
    next: FieldRead | null;
    columns: string[];
    ondrag: (key: string) => void;
    oncommand: (id: string, params: Record<string, unknown>) => void;
    onsheet: (field: FieldRead, kind: 'rename' | 'retype' | 'choices' | 'delete') => void;
  };
};

/** What the screen hands every design. One contract, three drawers. */
export type RecordTableProps = {
  /** The card's own title (`Core`, `Practice Sets` …). */
  title: string;
  /** The view's id — the canvas's own `data-view`, which the designs never touch. */
  view: string;
  kind: string;
  /** The view's columns, in the view's own order. */
  columns: ColumnFact[];
  /**
   * The column that takes the free space: the first reading that is prose, or
   * the first column when the view is all numbers. Computed from the *view*, so
   * every row and the strip above them share one width table — which is how a
   * label sits over its own cells and how A's tool rail can open on a row
   * without moving a fact on any other.
   */
  flexKey: string | null;
  /** How many of the kind's own fields this view hides (`column.show` restores them). */
  hiddenCount: number;
  rows: RowFact[];
  /** How many records the view holds in total, before its own limit. */
  candidates: number;
  /** Cell writes in flight — the head's own `· saving…`. */
  inFlight: number;
  /** The row the cursor is on (`app.selection`) — A's rail and C's sheet follow it. */
  selected: string | null;
  /** The plan's own today, for a distance in days. */
  today: string | null;

  /** The open column `+` menu, the open row menu, the row whose move menu is open. */
  plusOpen: boolean;
  menuFor: string | null;
  moveFor: string | null;
  /** The column being dragged, and the column it would land in front of. */
  dragging: string | null;
  dropTarget: string | null;
  /** The rows of the column `+` menu, the row menu and the move menu, built here. */
  plusRows: MenuRow[];
  rowRows: (row: RowFact) => MenuRow[];
  moveRows: (row: RowFact) => MenuRow[];

  /** The record's own instant, as a column the clock verbs can shift. */
  instant: { key: string; label: string } | null;

  onSelect: (id: string) => void;
  /** One cell write — the app's single dispatch path (`record.setField`). */
  onCommit: (row: RowFact, key: string, value: string | string[] | null) => void;
  /** The record's own door: the detail panel (`record.panel`). */
  onPanel: (row: RowFact) => void;
  /** Open or close a row's own menu; null closes it. */
  onMenu: (id: string | null) => void;
  /** Open or close a row's "move to another kind" menu. */
  onMove: (id: string | null) => void;
  onPlus: (open: boolean) => void;
  /** A column drag: the key picked up, the drop target, and the commit. */
  onDrag: (key: string) => void;
  onOver: (key: string | null) => void;
  onDrop: (before: string) => void;
  /**
   * The rail's clock verbs and B's danger item, as one write: `days` shifts the
   * record's own instant by that many days, `null` clears it.
   */
  onInstant: (row: RowFact, days: number | null) => void;
};

/**
 * A distance in days as a student says it — the vocabulary `FieldControl`
 * already speaks for a date cell (`today` · `tomorrow` · `in 3 days`). Kept
 * here rather than re-invented per design so three drawings cannot say three
 * different things about one record's own date.
 */
export function dayWords(days: number): string {
  if (days === 0) return 'today';
  if (days === 1) return 'tomorrow';
  if (days === -1) return 'yesterday';
  return days > 0 ? `in ${days} days` : `${-days} days ago`;
}

/** The whole days between two `YYYY-MM-DD` days, or null when either is not a day. */
export function daysBetween(day: string, today: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day) || !/^\d{4}-\d{2}-\d{2}$/.test(today)) return null;
  const gap = Date.parse(`${day}T00:00:00Z`) - Date.parse(`${today}T00:00:00Z`);
  if (Number.isNaN(gap)) return null;
  return Math.round(gap / 86_400_000);
}

/**
 * The column a table can afford to stretch: the first one that holds prose (a
 * title, a name, a link, a relation), else the first column at all. Declared
 * here because three things must agree about it — the strip's labels, A's cells
 * and A's tool bay — and a table whose header and body disagreed about which
 * column flexes is exactly the drift this file exists to prevent.
 */
export function flexKeyOf(columns: Array<{ key: string; type: string }>): string | null {
  const prose = columns.find((column) => PROSE_TYPES.includes(column.type));
  return (prose ?? columns[0])?.key ?? null;
}

/** The readings that stretch; everything else asks only for its own room. */
export const PROSE_TYPES = ['text', 'longtext', 'url', 'relation'];
