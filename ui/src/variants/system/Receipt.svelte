<!-- Status text and undo action for committed writes. -->
<script lang="ts">
  import type { Receipt as ReadBack, SystemActions } from './props';

  let { receipt, acts }: { receipt: ReadBack; acts: SystemActions } = $props();
</script>

<p class="sys-receipt" role="status">
  <b>{receipt.text}</b>
  {#if receipt.undo}
    <span class="sys-groups__spacer"></span>
    <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => acts.onUndo()}>Undo</button>
  {/if}
</p>

<style>
  .sys-receipt {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
    margin: 0 0 var(--ui-gap);
    padding: var(--ui-pad-sm);
    border-radius: var(--r-item);
    background: var(--well);
    font-size: var(--ui-text-sm);
    color: var(--ink-2);
  }
  .sys-receipt b { color: var(--ink); font-weight: var(--weight-label); }
</style>
