<!--
  The `conditional` block: the branch the engine already chose.

  Which branch is *not* decided here — `node.blocks` holds the children of the
  branch whose `when` the engine matched, so this component has no expression to
  evaluate and must not grow one (§4.4: a second evaluator is the thing that
  drifts). What it owns is the one case where drawing nothing is correct: a
  conditional that did not match and has no children is not an empty state, it
  is a branch that resolved to silence.
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
  /** True when there is genuinely nothing to draw, and nothing to explain. */
  const silent = $derived(node.matched === false && children.length === 0);
</script>

{#if !silent}
  <!-- The state is on the wrapper so the resolution is inspectable: a screen
       that draws fewer blocks than the file declares is otherwise unreviewable. -->
  <section
    class="blk"
    data-block="conditional"
    data-matched={node.matched ?? false}
    data-when={node.when ?? ''}
  >
    {#if children.length === 0}
      <p class="blk__empty" data-block="empty">
        this condition matched, but its branch has no blocks yet
      </p>
    {:else}
      {#each children as child, position (position)}
        <Block node={child} depth={depth + 1} {targetsFor} />
      {/each}
    {/if}
  </section>
{/if}
