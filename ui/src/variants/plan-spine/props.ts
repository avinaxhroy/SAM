/**
 * Plan spine surface variant component props.
 * Renders weekly schedule cards, term milestones, and topic sequences.
 */
import { durationText } from '../../types';

/** One piece of work, as a design draws it. */
export type PlanTopic = {
  id: string;
  /** The record's own kind — what the row's door (`record.panel`) needs. */
  type: string;
  label: string;
  /** The course the work belongs to, with its identity wash, when it names one. */
  course: { label: string; wash: string | undefined } | null;
  /** done = the ladder's top rung · step = started · none = untouched. */
  state: 'done' | 'step' | 'none';
  /** How far along, in the plan's own words (`stageWord`). */
  stage: string;
  /** The plan's own word for the kind of work (`watch`, `read`, `practice`). */
  kind: string | null;
  /** Estimated minutes, when the kind declares one. */
  minutes: number | null;
};

/** One week of the term and the work the plan placed in it. */
export type PlanWeek = {
  id: string;
  /** The week's own index, as the plan numbers it (1, 2, 3, 4, 8, 26 …). */
  index: number;
  /** A week spoken as the plan names it — its label, else `Week N`. */
  name: string;
  /** The week's range as the plan dates it (`25 Sep – 1 Oct`), else null. */
  dates: string | null;
  /** The phase the week declares (`Foundation`), else null. */
  phase: string | null;
  topics: PlanTopic[];
  /** The minutes this week's work adds up to. */
  minutes: number;
  /** How many of its topics are done. */
  done: number;
  /** True for the one week the plan resolves today into. */
  now: boolean;
};

export type PlanProps = {
  /** The screen's own title (`Plan`, from the navigation entry). */
  title: string;
  /** The term's weeks in the plan's own order. */
  weeks: PlanWeek[];
  /** Work the plan holds but has not placed in a week. */
  unplaced: PlanTopic[];
  /** The term's own figures, derived on the frame they print. */
  summary: { weeks: number; things: number; done: number; minutes: number; unplaced: number };
  /**
   * The week the student is reading. The screen owns it (a design asks for a
   * week, it does not keep a second selection of its own) and it opens on the
   * week the plan resolves today into.
   */
  selectedId: string | null;
  /** The command ids the designs' write rows dispatch, or null when the plan
   *  declares no such kind. */
  newWeekCommand: string | null;
  newTopicCommand: string | null;
  onSelect: (id: string) => void;
  /** The record door: the app's own `record.panel` for one topic row. */
  onTopic: (topic: PlanTopic) => void;
  onNewWeek: () => void;
  onNewTopic: () => void;
};

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/**
 * A week's range as a student reads a date: `25 Sep – 1 Oct`, and `25–31 Sep`
 * when both ends share a month. The plan's own dates, never an invented one —
 * an undated week returns null and the design prints no date block at all.
 *
 * Both ends are read as UTC calendar days, so the string the plan stored is the
 * string a student sees in every timezone.
 */
export function rangeText(start: string | null, end: string | null): string | null {
  const dayOf = (iso: string | null): { day: number; month: number } | null => {
    if (!iso || !/^\d{4}-\d{2}-\d{2}$/.test(iso.slice(0, 10))) return null;
    const at = Date.parse(`${iso.slice(0, 10)}T00:00:00Z`);
    if (Number.isNaN(at)) return null;
    const date = new Date(at);
    return { day: date.getUTCDate(), month: date.getUTCMonth() };
  };
  const from = dayOf(start);
  const to = dayOf(end);
  if (!from) return to ? `${to.day} ${MONTHS[to.month]}` : null;
  if (!to) return `${from.day} ${MONTHS[from.month]}`;
  if (from.month === to.month) return `${from.day}–${to.day} ${MONTHS[from.month]}`;
  return `${from.day} ${MONTHS[from.month]} – ${to.day} ${MONTHS[to.month]}`;
}

/** The one figure every design prints for a week: what is done, and how much
 *  work that is. `no topics` is a fact, not an empty string. */
export function readOf(week: PlanWeek): string {
  if (week.topics.length === 0) return 'no topics';
  return `${week.done} of ${week.topics.length} done · ${durationText(week.minutes)}`;
}

/** The fraction a meter or a run draws, 0–100. */
export function percentOf(part: number, whole: number): number {
  return whole === 0 ? 0 : Math.round((part / whole) * 100);
}
