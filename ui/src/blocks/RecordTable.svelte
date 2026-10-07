<!--
  The `table` block: the engine's projection, drawn by the app's record table.

  A block node is byte-compatible with a plain `SAM view` result — same
  `view`/`type`/`columns`/`records`/`groups`/`warnings` keys — so the table the
  editor already has is the table a screen draws. This wrapper exists to do two
  things and nothing else: hand the table a real `ViewRead`, and let a block's
  own `title` head the card.
-->
<script lang="ts">
  import RecordTable from '../records/RecordTable.svelte';
  import { app } from '../session.svelte';
  import type { BlockNode, FieldRead, ViewRead } from '../types';

  let {
    node,
    targetsFor,
    variant = 'table',
  }: {
    node: BlockNode;
    targetsFor: (field: FieldRead) => Array<{ id: string; label: string }>;
    /** `list` draws the same cells with ruled rows (§3.6's two cell layouts). */
    variant?: 'table' | 'list';
  } = $props();

  const type = $derived(app.types[node.type ?? '']);
  const outcome = $derived<ViewRead>({
    view: node.view ?? '',
    type: node.type ?? '',
    layout: node.layout ?? 'table',
    columns: node.columns ?? [],
    candidates: node.candidates ?? 0,
    returned: node.returned ?? 0,
    records: node.records,
    grouped: node.grouped,
    groups: node.groups,
    warnings: node.warnings,
  });
</script>

<!-- The wrapper carries the block's own identity (`data-block`), which is what
     the UI check walks: every block kind reports itself, and the table's rows
     stay the table's own component. -->
<div class="blk" data-block={node.kind}>
  <RecordTable {outcome} {type} columns={outcome.columns} {targetsFor} {variant} title={node.title ?? null} />
</div>
