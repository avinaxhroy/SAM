<!--
  Standard header for record blocks: displays block title, record count,
  layout label, and action doors (new record, paste, renumber, edit view).
-->
<script lang="ts">
  let {
    title,
    type,
    word,
    returned,
    candidates,
    onNew,
    onPaste,
    onRenumber,
    onEdit,
  }: {
    /** The card's title: the block's own, else `nameOf(view)`. */
    title: string;
    /** The view's kind — what the two per-kind doors are named after. */
    type: string;
    /** The layout's own word (`List` · `Board` · `Outline` · …), a label. */
    word: string;
    /** How many records the read returned, and how many it considered. */
    returned: number;
    candidates: number;
    onNew?: () => void;
    onPaste?: () => void;
    onRenumber?: () => void;
    onEdit?: () => void;
  } = $props();
</script>

<header class="cd-card__head blk-head">
  <div class="blk-head__id">
    <h2 class="cd-card__title">{title}</h2>
    <p class="cd-card__sub">
      {returned} of {candidates} {type}s
    </p>
  </div>
  <span class="cd-card__spacer"></span>
  <span class="blk-head__word">{word}</span>
  {#if onPaste}
    <button
      class="cd-pill cd-pill--ghost cd-pill--sm"
      type="button"
      data-command={`${type}.paste`}
      data-placement="recordTable.paste"
      title={`Paste TSV/CSV as ${type} records — commits as one apply batch`}
      onclick={() => onPaste?.()}
    >
      Paste
    </button>
  {/if}
  {#if onNew}
    <button
      class="cd-pill"
      type="button"
      data-command={`${type}.new`}
      data-placement="recordTable.toolbar"
      onclick={() => onNew?.()}
    >
      New {type}
    </button>
  {/if}
  {#if onRenumber}
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm"
      type="button"
      data-command="records.renumber"
      data-placement="recordTable.toolbar"
      title={`Number every ${type} — undoable straight after`}
      onclick={() => onRenumber?.()}
    >
      Renumber
    </button>
  {/if}
</header>

<style>
  /* The head wraps instead of squeezing: at a narrow container the identity
     keeps its line and the doors drop under it, rather than shrinking the title
     to an ellipsis beside four pills. */
  .blk-head {
    flex-wrap: wrap;
    row-gap: var(--space-xs);
    align-items: flex-start;
  }
  .blk-head__id {
    min-width: 0;
  }
  /* The layout's own word: a label in the app's own vocabulary, at the caps
     step the system keeps for a label (`--text-2xs` + `--track-caps`). */
  .blk-head__word {
    flex: none;
    align-self: center;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
</style>
