/**
 * SYSTEM · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Index (a place's rows on the system's own segment; every editor a
 * 460px sheet grown from the row that asked) · B · The Workbench (the same five
 * places, edited in place: rows open inline, the blocks as trading cells, the
 * tree still a tree) · C · The Ladder (one document of five sections, with a
 * breadcrumb out to a full-page editor). The student's choice lives in
 * `variants/styles.svelte.ts`; this file is only the map from the letter to the
 * drawing.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export default { a: A, b: B, c: C } satisfies VariantComponents;
