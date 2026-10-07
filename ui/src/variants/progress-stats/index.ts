/**
 * PROGRESS · THE FIGURES — the three designs, and the props contract they share.
 *
 * `props.ts` states what each design is handed; the components draw it. The
 * surface's entry in the catalog is `progress-stats` (`catalog.ts`), and the
 * panel mounts it as `<Variant surface="progress-stats" … />`.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export type {
  Figure,
  FigureKey,
  FigureRead,
  ProgressDay,
  ProgressStatsProps,
} from './props';

export default { a: A, b: B, c: C } satisfies VariantComponents;
