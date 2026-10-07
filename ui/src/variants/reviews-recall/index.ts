/**
 * REVIEWS · THE RECALL LOOP — the three designs, as the registry reads them
 * (2026-09-29).
 *
 * A · The Top Card (the deck you toss) · B · Three Stages (prompt · evidence ·
 * grade) · C · The Split (question left, answer right). The student's choice
 * lives in `variants/styles.svelte.ts`; this file is only the map from the
 * letter to the drawing, and the contract all three read is beside it in
 * `./props.ts`.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
