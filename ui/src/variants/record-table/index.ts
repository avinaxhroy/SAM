/**
 * RECORD TABLE · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Rail (a quiet table, and a tool rail that rises in the focused row) ·
 * B · The Profile (bars that unfold one record's profile in place) · C · The
 * Morph (a run of tiles over the record's own sheet, where a cell becomes its
 * editor where it stands). The student's choice lives in
 * `variants/styles.svelte.ts`; this file is only the map from the letter to the
 * drawing, and `props.ts` is what all three are handed.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
