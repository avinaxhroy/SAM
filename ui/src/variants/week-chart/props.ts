/**
 * Week chart surface variant component props.
 * Displays daily study duration bars, dynamic ceiling scales, and target lines.
 */
import { durationText } from '../../types';

/** One day of the last seven, oldest first — the panel's own read order.
 *  `date` is the key and is never printed. */
export type WeekDay = {
  date: string;
  /** The app's own short weekday: `Mon` · `Tue` · … */
  weekday: string;
  /** The day of the month, 1–31. */
  day: number;
  /** Minutes logged that day; 0 is a day with nothing, not a missing day. */
  loggedMin: number;
  isToday: boolean;
};

export type WeekChartProps = {
  /** The card's own heading, chosen by the screen (`This week` · `Work per day`). */
  title: string;
  /** The one line under the heading, in the screen's own words
   *  (`5 h across the last seven days`). */
  caption: string;
  /** The last seven days, oldest first (`todayFacts.days`). */
  days: WeekDay[];
  /** The plan's daily target in minutes; `null` (or 0) when it declares none —
   *  a legal state, and one every design draws without an aim rule or a
   *  distance line rather than inventing a target. */
  targetMin: number | null;
};

/** The round stops a ceiling may take — quarter-hours, then hours. */
const STOPS = [30, 60, 90, 120, 150, 180, 240, 300, 360, 480, 600, 720, 900, 1200, 1500];

/** The ceiling every mark is drawn against: the next round stop above the
 *  tallest day and the target, floored at an hour so a week with almost
 *  nothing logged still has a readable plot. */
export function ceilingOf(days: WeekDay[], targetMin: number | null): number {
  const need = Math.max(60, targetMin ?? 0, ...days.map((day) => day.loggedMin));
  return STOPS.find((stop) => stop >= need) ?? Math.ceil(need / 60) * 60;
}

/** The one mid ruling a variant may name: the largest round stop at or below
 *  45% of the ceiling — the lab's `1 h` at 0.4 of a 150-minute ceiling. 0 when
 *  the ceiling is too short to carry a second named line. */
export function midStopOf(ceiling: number): number {
  return [...STOPS].reverse().find((stop) => stop <= ceiling * 0.45) ?? 0;
}

/** The target's own fraction of the ceiling, or null when the plan names no
 *  target — the aim rule and every distance sentence hang on this. */
export function aimOf(ceiling: number, targetMin: number | null): number | null {
  if (!targetMin || targetMin <= 0) return null;
  return Math.min(1, targetMin / ceiling);
}

/** The date of the day with the most logged minutes, or null for a week with
 *  nothing in it. Ties go to the oldest day, which is the order the panel
 *  reads them in. */
export function peakOf(days: WeekDay[]): string | null {
  let peak: WeekDay | null = null;
  for (const day of days) if (day.loggedMin > 0 && (!peak || day.loggedMin > peak.loggedMin)) peak = day;
  return peak?.date ?? null;
}

/** A day's minutes on screen: a word for nothing, the app's own duration text
 *  otherwise. */
export function valueText(minutes: number): string {
  return minutes === 0 ? 'nothing' : durationText(minutes);
}

/** Minutes with the trailing `min` dropped where an hour leads — the compact
 *  form a day card has room for (`1 h 25`, `50 min`, `2 h`). */
function shortDuration(minutes: number): string {
  const total = Math.round(minutes);
  if (total < 60) return `${total} min`;
  const hours = Math.floor(total / 60);
  const rest = total % 60;
  return rest === 0 ? `${hours} h` : `${hours} h ${rest}`;
}

/** A day's distance from the aim in the lab's compact form (`1 h 25 under`,
 *  `5 min over`), or null when the plan names no target. */
export function readShort(loggedMin: number, targetMin: number | null): string | null {
  if (!targetMin || targetMin <= 0) return null;
  const gap = targetMin - loggedMin;
  if (gap === 0) return 'on target';
  return gap > 0 ? `${shortDuration(gap)} under` : `${shortDuration(-gap)} over`;
}

/** The same distance as the full sentence a tag, an accessibile name or an
 *  open tile states (`1 h 25 min under the target`). */
export function readLong(loggedMin: number, targetMin: number | null): string | null {
  if (!targetMin || targetMin <= 0) return null;
  const gap = targetMin - loggedMin;
  if (gap === 0) return 'on the target';
  return gap > 0 ? `${durationText(gap)} under the target` : `${durationText(-gap)} over the target`;
}

/** A day, spelled for an accessible name (`Today, Sun 27` · `Mon 21`). */
export function dayName(day: WeekDay): string {
  return day.isToday ? `Today, ${day.weekday} ${day.day}` : `${day.weekday} ${day.day}`;
}

/** The week, spelled for the chart group's accessible name. */
export function rangeLabel(days: WeekDay[]): string {
  if (days.length === 0) return '';
  const first = days[0];
  const last = days[days.length - 1];
  return `${first.weekday} ${first.day} to ${last.weekday} ${last.day}`;
}

/** One day's whole sentence: the name, what was logged, the distance and (for
 *  the peak) the fact that it is the week's most. */
export function dayLabel(day: WeekDay, targetMin: number | null, peak: string | null): string {
  const logged = day.loggedMin === 0 ? 'nothing logged' : `${durationText(day.loggedMin)} logged`;
  const parts = [dayName(day), logged];
  if (day.date === peak) parts.push('most logged');
  const distance = readLong(day.loggedMin, targetMin);
  if (distance) parts.push(distance);
  return parts.join(', ');
}
