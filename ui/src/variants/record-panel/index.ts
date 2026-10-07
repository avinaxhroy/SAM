/**
 * RECORD PANEL · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Diagram (a drafting plate, the relations wired from the boxes' own
 * geometry) · B · The Counter Bank (quantity columns as drums that roll on a
 * write) · C · The Band (the plate inks the gap between what was planned and
 * what the plan logged). The student's choice lives in
 * `variants/styles.svelte.ts`; this file is only the map from the letter to the
 * drawing, and `props.ts` is the contract all three read.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
