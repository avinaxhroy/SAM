/** Props for Reviews recall loop variants. */
export type RecallGrade = {
  /** The engine's own word for the judgement (`again` · `hard` · `good` · `easy`). */
  rating: string;
  /** The numeral the keycap shows, and the key that grades. */
  key: string;
  /** The student's word for it (`Forgot` · `Hard` · `Good` · `Solid`). */
  label: string;
  /** The rung of the topic's ladder this grade leaves it on, and the ladder's
   *  own length — the engine's `stage` when it names one, else the rung the
   *  topic already holds, because a `fixed` plan moves the date and not the
   *  step (`crates/sam-core/src/scheduler.rs:305`). */
  rung: number;
  rungs: number;
  /** What this grade alone would do, when the shown grades disagree. `null`
   *  while the dry-runs are out, and `null` when they agree — the panel then
   *  prints the shared answer once, as `returned`. */
  consequence: string | null;
};

/** The card in the loop's hands: the record the student is answering. */
export type RecallCard = {
  id: string;
  /** The prompt — the record's own label, and the one thing the reveal never
   *  replaces. */
  title: string;
  /** The course's own code, as a student says it out loud, and its wash. */
  course: string | null;
  wash: string | undefined;
  /** The head's state chip: `2 days late` · `due today` · `ready for one recall`. */
  state: string;
  /** Which chip species carries it. */
  tone: 'overdue' | 'info' | 'quiet';
  /** `9 days since the last recall`, or null when it has never been recalled. */
  since: string | null;
  recalls: number;
  /** The plan's own estimate for this record, in minutes. */
  minutes: number | null;
};

/** One row of the sitting, for the design that draws the queue of questions. */
export type RecallRow = {
  id: string;
  /** The record's own place in the due list, zero-padded (`02`). */
  numeral: string;
  title: string;
  /** The row's state word (`5 days late` · `due today`), or null when the row
   *  says nothing the current one does not. */
  state: string | null;
  late: number;
};

export type RecallProps = {
  card: RecallCard;
  /** Where the card sits in the sitting (`2 of 7`), or null for a record the
   *  student reached another way (the empty state's hatch, a pinned row). */
  position: { at: number; of: number; left: number } | null;
  /** The window of the sitting — the cards this loop will reach from here. */
  sitting: RecallRow[];
  /** What is left after the window (`2 more waiting`), or null when the window
   *  is the whole queue. */
  more: string | null;
  grades: RecallGrade[];
  /** The date and interval every shown grade earned, when they earned one —
   *  printed once per card instead of on every tile. */
  returned: { day: string; say: string } | null;
  revealed: boolean;
  /** The student's own words about this topic, kept per record by the panel. */
  answer: string;
  /** True while rating dry-runs are in flight. */
  asking: boolean;
  /** The trail the loop leaves after a grade (`Graded Good`), or null. */
  trail: string | null;
  onReveal: () => void;
  onHide: () => void;
  onAnswer: (text: string) => void;
  onGrade: (rating: string) => void;
  /** Put another record of the sitting on the card. */
  onSelect: (id: string) => void;
  onUndo: () => void;
};

/** The chip species a state word wears (`components.css:495`). */
export function toneClass(tone: RecallCard['tone']): string {
  if (tone === 'overdue') return 'cd-chip--overdue';
  if (tone === 'info') return 'cd-chip--info';
  return '';
}

/** The rung word every design prints beside its own ladder mark. */
export function rungText(grade: RecallGrade): string {
  return `rung ${grade.rung}`;
}

/** How full the ladder mark is drawn, 0–100. The system's own mark is three
 *  bars (`components.css:574`); the plan's ladder is read across them, so a
 *  two-rung ladder and a four-rung one both read truly. */
export function rungFill(grade: RecallGrade): number {
  if (grade.rungs <= 0) return 0;
  return Math.max(0, Math.min(100, Math.round((grade.rung / grade.rungs) * 100)));
}

/** The bars the three-rung pictogram lights, 0–3 — the mark's own scale. */
export function rungBars(grade: RecallGrade): number {
  return Math.max(0, Math.min(3, Math.round((rungFill(grade) / 100) * 3)));
}

/** A grade's accessible name: the key that fires it, the word, and the rung it
 *  leaves the topic on. Never a bare numeral, never a bare word. */
export function gradeName(grade: RecallGrade): string {
  const rungs = grade.rungs > 0 ? ` of ${grade.rungs}` : '';
  const extra = grade.consequence ? ` ${grade.consequence}.` : '';
  return `${grade.key} — ${grade.label}. Moves this topic to rung ${grade.rung}${rungs}.${extra}`;
}

/** The facts line the head of every design carries, out of the card's own
 *  three facts — one string so the three cannot disagree. */
export function factsOf(card: RecallCard): string {
  const parts = [card.since ?? 'Never recalled', `${card.recalls} ${card.recalls === 1 ? 'recall' : 'recalls'} logged`];
  return parts.join(' · ');
}
