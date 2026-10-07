/**
 * Progress stats surface variant component props.
 * Displays metric summaries, weekly progress, and period deltas.
 */

/** The lab's four figures, in its own order. */
export type FigureKey = 'time' | 'days' | 'solved' | 'tested';

/** One reading of one figure — the figure itself, and the window before it. */
export type FigureRead = {
  /** The numeral as printed. A duration goes through the app's `durationText`. */
  value: string;
  /** The same reading as a number, for a design that draws a length (B's fill). */
  num: number;
  /** The denominator, or null when the plan declares none. */
  of: number | null;
  /** The denominator in words — `of the 28 h aimed`. */
  ofText: string;
  /** The change against the period named in the same sentence, or the sentence
      that says the plan keeps no predecessor. Never a bare `+0`. */
  delta: string;
};

export type Figure = FigureRead & {
  key: FigureKey;
  /** The figure's own name — `Time studied`, `Days with work`. */
  label: string;
  /** The same figure one window back, or null when there is no predecessor —
      the plan totals, and a plan younger than the window it is read over. */
  prev: FigureRead | null;
};

/** One day, as every design draws it. */
export type ProgressDay = {
  /** The day's key. Kept for Svelte's keyed `{#each}` — **never printed**; the
      words are `when` and `fact`, composed by the panel. */
  date: string;
  /** `Mon` — the rod's letter, the row a dot sits in (index 0 = Mon). */
  weekday: string;
  /** The day of the month, as the axis numbers it. */
  day: number;
  /** `Sep` — the dot grid's month rule. */
  month: string;
  /** `Mon 21 Sep` — the day read out in full. */
  when: string;
  minutes: number;
  isToday: boolean;
  /** The day as one sentence — `Mon 21 Sep · 45 min of the 4 h aim, 3 h 15 min
      short`, or `Wed 23 Sep · nothing logged`. Composed here so all three
      designs say a day the same way. */
  fact: string;
};

export type ProgressStatsProps = {
  /** The week's four figures, each with its own denominator and its delta. */
  figures: Figure[];
  /** The seven days ending on the plan's today, oldest first — the rods, the
      vessels' window and (through the week chart) the same week. */
  week: ProgressDay[];
  /** The term so far: whole weeks from the Monday fifteen weeks back through
      the plan's today, oldest first. Index `i` is column `⌊i / 7⌋` and row
      `i % 7` of the dot grid — which is why the run starts on a Monday. */
  term: ProgressDay[];
  /** The plan's own daily aim in minutes, or null when it declares none. A's
      rod is drawn against it; null draws the rods against the tallest day. */
  aimMin: number | null;
  /** The week's own sentence, shown at rest under A's rods. */
  weekNote: string;
  /** The term's own sentence — the dot grid's caption when nothing is read. */
  termNote: string;
};
