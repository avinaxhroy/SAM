<!--
  SYSTEM · RECOVERY (2026-09-29).

  Hide-first's other half: a delete is undoable at once, and restorable later,
  because every write left the engine a backup. Recovery is a **control**, not a
  sixth place — you visit it once — which is why the shipped screen reaches it
  from the group row's trailing button and all three designs keep that.

  The list is `tx.list`'s own read (an action, so this component does not fetch —
  the panel holds it and hands it down), and a restore is a transaction like any
  other (`tx.restore`), previewable, undoable and refused on a stale revision.
  The transaction's own summary is the engine's sentence, marked `data-developer`
  — the same string the terminal prints, never this screen's prose (D2).
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import type { SystemActions } from './props';

  let {
    backups,
    restoring,
    acts,
  }: {
    backups: Array<{ txid: string; when: string; documents: number; summary: string }> | null;
    restoring: string | null;
    acts: SystemActions;
  } = $props();
</script>

<section class="cd-card">
  <div class="cd-card__head">
    <span class="cd-ictile"><Icon name="undo" /></span>
    <div>
      <h2 class="cd-card__title">Recently deleted</h2>
      <p class="cd-card__sub">the engine’s own backups — a restore is a transaction, previewed and undoable</p>
    </div>
    <span class="cd-card__spacer"></span>
    <!-- The read's own control, and a real one: a list loaded when this was
         opened can be stale, and a student who deleted something in another
         window wants it re-read. -->
    <button
      class="cd-pill cd-pill--ghost cd-pill--sm"
      type="button"
      data-command="tx.list"
      data-placement="system"
      onclick={() => acts.onReloadBackups()}
    >
      {backups === null ? 'Read them' : 'Check again'}
    </button>
  </div>

  {#if backups === null}
    <p class="cd-hint">
      The engine keeps a backup of every write this plan has taken. Read them to see what can come
      back, and bring any one of them back as a transaction of its own.
    </p>
  {:else if backups.length === 0}
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="check" size={24} /></span>
      <p class="cd-empty__t">Nothing to bring back</p>
      <p class="cd-empty__s">
        Every write this plan has taken is kept, and a deletion is undoable from its own receipt for a
        while after it happens. Nothing has been deleted here.
      </p>
    </div>
  {:else}
    <div class="cd-coll">
      {#each backups as backup (backup.txid)}
        <div class="cd-coll__row" data-txid={backup.txid}>
          <span class="sys-mark" aria-hidden="true"><Icon name="undo" size={12} /></span>
          <span class="cd-coll__body">
            <span class="cd-coll__title">A deletion, {backup.when}</span>
            <span class="cd-rowmeta">
              <IdPair value={backup.txid} title="Copy the backup id" />
              <span>{backup.documents} documents changed</span>
              <span data-developer>{backup.summary}</span>
            </span>
          </span>
          <span class="cd-coll__right">
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              data-command="tx.restore"
              data-placement="system"
              disabled={restoring === backup.txid}
              onclick={() => acts.onRestore(backup.txid)}
            >
              {restoring === backup.txid ? 'Bringing back…' : 'Bring it back'}
            </button>
          </span>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .cd-coll__row { min-height: calc(48px * var(--ui-s)); }
  .cd-rowmeta { flex-wrap: wrap; row-gap: var(--space-3xs); }
</style>
