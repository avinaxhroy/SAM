<!-- Floating undo notification bar displayed after grading a review card. -->
<script lang="ts">
  let { label, onUndo }: { label: string; onUndo: () => void } = $props();
</script>

<div class="trl" role="status">
  <span class="trl__t">{label}</span>
  <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={onUndo}>Undo</button>
</div>

<style>
  .trl {
    position: absolute;
    left: 50%;
    bottom: 0;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    padding: calc(4px * var(--ui-s)) calc(4px * var(--ui-s)) calc(4px * var(--ui-s)) var(--ui-pad);
    border-radius: var(--r-pill);
    background: var(--well);
    box-shadow: var(--sh-2);
    animation: trl-in var(--dur-2) var(--ease-pop) both;
  }
  .trl__t {
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-label);
    color: var(--ink-2);
    white-space: nowrap;
  }
  @keyframes trl-in {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(10px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateX(-50%);
    }
  }
</style>
