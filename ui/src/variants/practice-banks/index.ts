/**
 * PRACTICE · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Row That Opens (the row grows into its bench, its mark a hairline
 * track with the preview beside it) · B · Folders In A Rack (a 2×4 rack of
 * washed tabs and pocket strips, the logging card sliding out of the pocket) ·
 * C · The Tactile Commit (a quiet list where the pen docks under the picked row
 * and the ink cap squashes into its socket to commit). The student's choice
 * lives in `variants/styles.svelte.ts`; this file is only the map from the
 * letter to the drawing.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
