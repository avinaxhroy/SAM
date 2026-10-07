/**
 * PLAN SPINE · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Deck (a scroll-stack of week sheets) · B · The Workspace (a phase
 * rail with the open week on the canvas beside it) · C · The Carousel (one
 * week on the stage, a ladder of week chips wheeling past it). The student's
 * choice lives in `variants/styles.svelte.ts`; this file is only the map from
 * the letter to the drawing, and `props.ts` is the one contract all three read.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
