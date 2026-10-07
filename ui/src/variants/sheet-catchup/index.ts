/**
 * SHEET + CATCHUP · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Tray (cards carried into three date holes) · B · The Docked Drawer (a
 * right-edge slab behind a curve that settles straight) · C · The Slide-Up Panel
 * (a bar on the window's floor that rises into the work). The student's choice
 * lives in `variants/styles.svelte.ts`; this file is only the map from the
 * letter to the drawing, and the contract all three receive is `./props.ts`.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export type { CatchupDate, CatchupItem, CatchupProps } from './props';

export default { a: A, b: B, c: C } satisfies VariantComponents;
