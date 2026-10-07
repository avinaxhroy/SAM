/**
 * THE LIST CARD · the three designs, as the registry reads them (2026-09-29).
 *
 * A · The Day Tape (a paper tape ruled by day; a bar per record whose length IS
 * its whole-day count) · B · The Spike (the records threaded on one needle, in
 * the view's own order) · C · The Tray (an in-tray whose lip is the boundary;
 * exactly two spills, each printing the write it would make).
 *
 * The screen is `ui/src/blocks/RecordList.svelte`; the contract is `./props.ts`.
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';
import B from './B.svelte';
import C from './C.svelte';

export type { ListCardProps, ListCardRow, ListInput, TapeCell } from './props';

export default { a: A, b: B, c: C } satisfies VariantComponents;
