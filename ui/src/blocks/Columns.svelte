<!--
  The `columns` block: a row of side-by-side slots.

  The slots are not this component's business — the engine resolved
  `node.blocks`, and each child draws itself through the dispatcher, which is
  also where the recursion boundary lives. So this adds geometry and nothing
  else: columns are *layout*, not a card, and they stay uncarded so the cards
  inside them are the only cards on the screen.
-->
<script lang="ts">
  import Block from './Block.svelte';
  import type { BlockNode, FieldRead } from '../types';

  let {
    node,
    depth = 0,
    targetsFor = () => [],
  }: {
    node: BlockNode;
    depth?: number;
    targetsFor?: (field: FieldRead) => Array<{ id: string; label: string }>;
  } = $props();

  const children = $derived(node.blocks ?? []);
</script>

<section class="blk" data-block="columns">
  {#if node.title}
    <h2 class="cd-card__title">{node.title}</h2>
  {/if}

  {#if children.length === 0}
    <!-- An empty row is a half-finished view, not a rendering failure: the fix
         is in the editor, so the line says where to go. -->
    <p class="blk__empty" data-block="empty">
      this row has no blocks yet — add one in the view editor
    </p>
  {:else}
    <div class="blk__cols">
      {#each children as child, position (position)}
        <Block node={child} depth={depth + 1} {targetsFor} />
      {/each}
    </div>
  {/if}
</section>
