/**
 * Record door sheet variant registration (Variant A).
 */
import type { VariantComponents } from '../registry';
import A from './A.svelte';

export type {
  DoorColumn,
  DoorField,
  DoorKind,
  DoorReadRow,
  DoorReceipt,
  DoorRefusal,
  RecordDoorProps,
} from './props';

export default { a: A } satisfies VariantComponents;
