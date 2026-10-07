/**
 * REFERENCE · THE LIBRARY — the three designs, as the registry reads them
 * (2026-09-29).
 *
 * A · The Shelf (records stand on a plank as spines; pulling one unfolds its
 * page in place) · B · The Palette (one field whose deck of result cards unfolds
 * from its own trigger) · C · The Folders (every course is a sleeve with the
 * paper standing in it and its own count on the pocket). One screen — Library
 * and Notes are the same panel with a filter — so all three read the same
 * contract (`props.ts`). The student's choice lives in
 * `variants/styles.svelte.ts`; this file is only the map from the letter to the
 * drawing.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
