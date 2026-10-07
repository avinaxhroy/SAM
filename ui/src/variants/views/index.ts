/**
 * Record layout variant registration (Variant A: eight layouts).
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';

export type { RecordFact, ShapeProps, ViewsProps } from './props';

export default { a: A } satisfies VariantComponents;
