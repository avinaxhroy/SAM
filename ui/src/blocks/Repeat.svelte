<!--
  The `repeat` block: the same blocks once per record the view returned.

  The instances are the engine's — one per record of the source view, each
  already carrying its own resolved children — so this component repeats a
  drawing, never a query. Instances are keyed by position rather than by record
  id because a view may legitimately return the same record twice, and a
  duplicate key would take the whole screen down with it.
-->
<script lang="ts">
  import Block from './Block.svelte';
  import { recordLabel, type BlockNode, type FieldRead } from '../types';

  let {
    node,
    depth = 0,
    targetsFor = () => [],
  }: {
    node: BlockNode;
    depth?: number;
    targetsFor?: (field: FieldRead) => Array<{ id: string; label: string }>;
  } = $props();

  const instances = $derived(node.instances ?? []);
</script>

<section class="blk" data-block="repeat">
  {#if node.title}
    <h2 class="cd-card__title">{node.title}</h2>
  {/if}

  {#if instances.length === 0}
    <p class="blk__empty" data-block="empty">
      nothing to repeat over — this view returned no records
    </p>
  {:else}
    <div class="blk__insts">
      {#each instances as instance, position (position)}
        <div class="blk__inst" data-record-id={instance.record.id}>
          <div class="blk__insthead">
            <span>{recordLabel(instance.record)}</span>
            <span class="blk__instsub">{instance.record.id}</span>
          </div>
          {#each instance.blocks as child, slot (slot)}
            <Block node={child} depth={depth + 1} {targetsFor} />
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</section>
