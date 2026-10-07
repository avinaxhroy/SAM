<!--
  The `callout` block: a sentence the view carries on purpose.

  It is a wash card, not an alert: §11 refuses "you're behind" chrome, and this
  is where a student writes down the rule their own term runs on — *"the queue
  is ordered by week, then by the shortest session first"* — which is data in
  `views.json`, and therefore editable as text like everything else.
-->
<script lang="ts">
  import type { BlockNode } from '../types';

  let { node }: { node: BlockNode } = $props();
</script>

<section class="blk" data-block="callout">
  <div class="blk__callout" data-w="mint">
    {#if node.title}
      <p class="blk__callouttitle">{node.title}</p>
    {/if}
    <p class="blk__callouttext">
      {node.text && node.text.length > 0 ? node.text : 'this callout has no text yet'}
    </p>
  </div>
</section>
