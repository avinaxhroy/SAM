<!--
  SYSTEM · THE PROJECTION (2026-09-29).

  A structural write is *choose → project → accept*: the choice is a radio card
  that costs nothing, the projection is the engine's own dry run rendered as
  **sentences**, and only the accept button carries the command id. That is the
  shipped screen's own discipline (`System.svelte`'s header, `controls.md` §3)
  and all three designs draw the same object — the lab shares `projection()`
  between A, B and C for exactly this reason.

  The object itself is the system's own `.cd-detail` panel, so a projection is
  visibly the same kind of thing as every other disclosure in the app; this
  component adds no surface of its own. Three parts, always in this order: what
  changes (a sentence with the count), what does not, and the row of doors —
  never a number without a noun, never the engine's payload.

  The terminal twin and the file door are `data-developer`, the census's own
  exemption for the four doors; the accept is the one control here that writes.
-->
<script lang="ts">
  import IdPair from '../../shell/IdPair.svelte';
  import type { Projection, SystemActions } from './props';

  let { projection, acts }: { projection: Projection; acts: SystemActions } = $props();
</script>

<div class="cd-detail sys-detail sys-proj" data-pane="projection">
  <p class="cd-detail__k">What this would change</p>

  {#if projection.pending}
    <p class="cd-sr" role="status">Asking the engine what this would change.</p>
    <div class="cd-skel" style="--w: 26ch" aria-hidden="true"></div>
  {:else if projection.wait}
    <p class="cd-hint">{projection.wait}</p>
  {:else if projection.error}
    <p class="cd-error" role="alert">{projection.error}</p>
  {:else}
    <ul class="sys-proj__lines">
      {#each projection.lines as line, index (index)}
        <li class="sys-proj__line"><b>{line.lead}</b>{line.rest}</li>
      {/each}
    </ul>
  {/if}

  <div class="sys-proj__acts">
    <button
      class="cd-pill"
      class:cd-danger={projection.danger}
      type="button"
      data-command={projection.command}
      data-placement="system"
      disabled={projection.disabled || projection.busy}
      onclick={() => acts.onAccept()}
    >
      {projection.accept}
    </button>
    <button class="cd-pill cd-pill--ghost" type="button" onclick={() => acts.onCancel()}>
      Keep it as it is
    </button>
    <IdPair label="Terminal" value={projection.terminal} />
  </div>
</div>

<style>
  .sys-proj__acts { margin-top: var(--ui-gap-sm); }
  .sys-proj { gap: var(--ui-gap-sm); }
</style>
