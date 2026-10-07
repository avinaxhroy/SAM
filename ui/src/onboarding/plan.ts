/**
 * The room's own words for a plan before one is open (R11).
 *
 * Screen III makes the plan and opens one that exists, and both presses need the
 * same three sentences: the name the press starts from, the folder that tells two
 * plans apart, and what a shipped start holds. They live here rather than beside a
 * component because both screens' rows and the form above them say them, and the
 * empty-plan screens (Today, Plan) say the same name.
 */
import { durationText } from '../types';

/**
 * The name the create press starts from. It is a sentence case noun like the rest
 * of the app's own words, and it is *editable*: this is a default, not a policy.
 */
export const BLANK_NAME = 'My plan';

/**
 * The plan's own folder, which is what tells two plans apart before either is
 * opened: the room above the plan's name (`Documents/SAM/` for a plan at
 * `Documents/SAM/My plan`).
 */
export function planFolder(path: string): string {
  const parts = path.split('/').filter((part) => part.length > 0);
  return parts.length > 1 ? `${parts[parts.length - 2]}/` : '';
}

/**
 * `17 topics · 13 h 10 min` — a start's own work kind, counted, and the time that
 * work plans. The kind is the number's noun, so no count is ever bare, and a plan
 * that plans no minutes (the language deck, which counts words) simply has nothing
 * after the dot.
 */
export function planFacts(work: number, workLabel: string, minutes: number): string {
  const counted = work > 0 ? `${work} ${workLabel}` : '';
  const planned = minutes > 0 ? durationText(minutes) : '';
  return [counted, planned].filter((part) => part !== '').join(' · ');
}
