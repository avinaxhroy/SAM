/**
 * WEEK CHART · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Grid (seven day cards on one shared floor) · B · The Fan (seven
 * leaning sticks, height is minutes) · C · The Shelf (seven stacked tiles,
 * today's open on arrival). The screens' own contract is in `./props`; the
 * student's choice lives in `variants/styles.svelte.ts`. This file is only the
 * map from the letter to the drawing.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export type { WeekChartProps, WeekDay } from './props';

export default { a: A, b: B, c: C } satisfies VariantComponents;
