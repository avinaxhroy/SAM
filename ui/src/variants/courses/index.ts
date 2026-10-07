/**
 * COURSES · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Index (text-forward, monochrome) · B · The Shelf (washed tiles and a
 * pip per topic) · C · The Next Step (a featured course and an ink hero). The
 * student's choice lives in `variants/styles.svelte.ts`; this file is only the
 * map from the letter to the drawing.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
