/**
 * MOCKS & ASSESSMENTS · the three designs, as the registry reads them
 * (2026-09-29).
 *
 * A · The Month That Opens (the calm month, a card in its own flow) · B · The
 * Day Picker Rail (the month as one 31-key tube that is also the date control)
 * · C · Guided Steps (the waiting record as two quiet steps in one card). The
 * student's choice lives in `variants/styles.svelte.ts`; this file is only the
 * map from the letter to the drawing.
 *
 * `Props` lives in `./props` rather than here, because the three components
 * and the screen all import it and a cycle through `index.ts` would make the
 * contract depend on the drawings it is a contract for.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;

export type { MockRecord, MocksProps } from './props';
