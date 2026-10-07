/**
 * REVIEWS · THE BACKLOG — the three designs, as the registry reads them
 * (2026-09-29).
 *
 * A · The Shelves (four folding age windows) · B · The Board (four trays of
 * washed cards, re-laid on every act) · C · The Inline Defer (the age capsule
 * becomes its own day picker in place). The student's choice lives in
 * `variants/styles.svelte.ts`; this file is only the map from the letter to the
 * drawing, and the contract all three read is beside it in `./props.ts`.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
