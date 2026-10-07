/**
 * Source pane variant registration (Variant A).
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';

export default { a: A } satisfies VariantComponents;

export type { SourceFinding, SourceIdentity, SourceOption, SourcePaneProps, SourceStatus } from './props';
