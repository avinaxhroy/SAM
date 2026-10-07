/**
 * THE APP'S WORDS FOR THE ENGINE'S FACTS — the plan's method, its steps, and
 * the grades a recall is answered with.
 *
 * A plan declares its own method (`content/rules.json`): the blank preset's
 * `flip` pipeline is `learned → proved → anchored`, and a plan may declare
 * another one entirely. So every surface that prints a step reads the stage
 * ids *from the plan* and its words *from here* — the Reviews panel's rows,
 * Today's queue, the recall card and the getting-started document draw the
 * same sentences, and none of them can drift from the pipeline it describes.
 *
 * Each of these lists shipped duplicated inside the surface that first needed
 * it (the step sentences in the Reviews panel, the grades in the panel *and*
 * the recall widget). This module is the move that keeps every surface from
 * carrying a third copy.
 */

/** The stage's own verb, at the length a chip or a press can carry. */
export const STAGE_WORDS: Record<string, string> = {
  learned: 'learn it',
  proved: 'prove it',
  anchored: 'lock it in',
  started: 'start it',
  midway: 'get through it',
  done: 'finish it',
};

/** The row's own sentence: what the plan is waiting for, said plainly. */
export const STEP_WORDS: Record<string, string> = {
  learned: 'Ready to learn it — work through the material',
  proved: 'Ready to prove it — the plan asks for no evidence here',
  anchored: 'Ready to lock it in — one clean recall',
  started: 'Ready to start it — record the first sitting',
  midway: 'Ready to go on — this one is part-way through',
  done: 'Ready to finish it — record the last sitting',
};

/**
 * The grades a recall is answered with — the engine's `record.logReview
 * --rating` values and the words a student sees for them, in ascending grade
 * order (`design/demo/demo.js:957-960`). `Solid` rather than `Easy`: the label
 * says what the student knows, not how the engine scores it. Their
 * consequences are always read, never assumed: every surface that shows a
 * grade dry-runs `record.logReview` first.
 */
export const GRADES: Array<{ rating: string; label: string }> = [
  { rating: 'again', label: 'Forgot' },
  { rating: 'hard', label: 'Hard' },
  { rating: 'good', label: 'Good' },
  { rating: 'easy', label: 'Solid' },
];

/** The card's default ratings: miss and pass (`sam.reviews.grades`). */
export const DEFAULT_GRADES = [GRADES[0], GRADES[2]];

/** What the plan asks beside a step (`reviews.due`'s `asks`). */
export type Ask = {
  key: string;
  kind: string;
  label: string;
  min?: number | null;
  note?: string;
  required: boolean;
  stage: string | null;
};

/**
 * The row's sentence. A plan can put its own gate in front of a stage — the
 * proof pipeline asks for a problem count — so the asks are read before the
 * map: what the plan demands today is what the row says.
 */
export function stepWords(row: { asks?: Ask[]; next?: string | null }): string {
  const asks = row.asks ?? [];
  const proof = asks.find((ask) => ask.key === 'problems');
  if (proof) return `Ready to prove it — solve ${proof.min ?? 2} problems`;
  if (asks.some((ask) => ask.key === 'reason')) return 'Ready to lock it in — one clean recall';
  if (row.next && STEP_WORDS[row.next]) return STEP_WORDS[row.next];
  return 'Ready for the next step this plan declares';
}

/**
 * The whole ladder in the plan's own order, in the plan's own words —
 * "Learn it, prove it, lock it in". Null when the plan names a stage this
 * app has no word for: an unknown word is better absent than guessed.
 */
export function methodWords(stages: string[] | null | undefined): string | null {
  if (!stages || stages.length === 0) return null;
  const words = stages.map((stage) => STAGE_WORDS[stage]);
  if (words.some((word) => !word)) return null;
  return words
    .map((word, index) => (index === 0 ? word.charAt(0).toUpperCase() + word.slice(1) : word))
    .join(', ');
}

/**
 * The app's three answers, named once: what it is · how it works · get started.
 * The room (`onboarding/Onboarding.svelte`) is their one carrier — its step
 * dots, its rail and its screen reader labels all read this list, so a fourth
 * movement would be a fourth screen and nothing else to change. SAM ships no
 * walkthrough that *performs* them: the room makes the plan and the
 * app's own screens are where the deeds live (BUILDLOG #38).
 */
export const MOVEMENTS = ['What it is', 'How it works', 'Get started'];
