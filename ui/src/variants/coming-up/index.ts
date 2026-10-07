/**
 * COMING UP · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Deck (one dated card centre-stage, its peers as edges) · B · The
 * Reveal (a stack that assembles as it is scrolled) · C · The Split (a date
 * panel and a body panel that part as a row crosses the port). The student's
 * choice lives in `variants/styles.svelte.ts`; this file is only the map from
 * the letter to the drawing, and `props.ts` is the contract all three share.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
