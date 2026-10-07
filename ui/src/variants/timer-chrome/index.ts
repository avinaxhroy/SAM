/**
 * TIMER CHROME · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The notch slab (a reserved slug in the row; the session's ink grows along
 * the window's top edge) · B · The morphing pill (the clock and the length are
 * one capsule that splits in place) · C · The floor bar (separate objects docked
 * to the window's floor). The student's choice lives in
 * `variants/styles.svelte.ts`; this file is only the map from the letter to the
 * drawing.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
