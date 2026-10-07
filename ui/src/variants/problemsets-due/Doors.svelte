<!--
  THE LIST CARD'S ROW ACTS — the record's door and its own verbs (2026-09-29).

  One record in the list card is one object and nothing else: the object is its
  own control (the press that marks it), and the two things that act on the
  *record* stand beside it, never inside it — a button inside a button is not
  markup a browser will parse. So every one of the three designs renders this
  same pair, at the row's own end:

    · the door — `record.panel`, the app's own detail panel, at a full `--hit`
      square, with the record's own name in its accessible label;
    · the record's own verbs — the app's row menu (`copy-json` · `copy-path` ·
      `record.reveal` · `record.move` · `record.delete`), in the app's own
      `Menu`, opened from here and closed by the screen's own state.

  It is here, once, for the reason `PORTING.md` gives: the three designs must
  not drift apart on a control the parity gate diffs against the engine's
  declared writes (`tools/uicheck-diff.mjs` reads `data-command` off the page).
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import Menu from '../../shell/Menu.svelte';
  import type { MenuRow } from '../../commands/registry';
  import type { ListCardRow } from './props';

  let {
    row,
    open,
    menuRows,
    onPanel,
    onMenu,
  }: {
    row: ListCardRow;
    /** Is this row's own menu the open one? */
    open: boolean;
    menuRows: (row: ListCardRow) => MenuRow[];
    onPanel: (row: ListCardRow) => void;
    onMenu: (id: string | null) => void;
  } = $props();
</script>

<span class="pdl-acts">
  <button
    class="cd-iconbtn"
    type="button"
    data-command="record.panel"
    data-placement="recordTable.row"
    title="Details"
    aria-label={`Details for ${row.label}`}
    onclick={() => onPanel(row)}
  >
    <Icon name="textQuote" size={14} />
  </button>
  <button
    class="cd-iconbtn"
    type="button"
    aria-haspopup="menu"
    aria-expanded={open}
    data-command="record.delete"
    title="More for this record"
    aria-label={`More for ${row.label}`}
    onclick={() => onMenu(open ? null : row.id)}
  >
    ⋯
  </button>
  {#if open}
    <Menu rows={menuRows(row)} native={false} onclose={() => onMenu(null)} />
  {/if}
</span>

<style>
  /* The acts are one cluster at the row's own end, each a `--hit` square. The
     menu is anchored to the cluster (the app's own `.cd-menu` is absolute), so
     it opens under the control that asked for it wherever the row's own
     geometry puts it — and right-aligned, because the cluster is at the row's
     end and a menu opening leftwards would leave the card. */
  .pdl-acts {
    position: relative;
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
  }
  .pdl-acts :global(.cd-menu) {
    left: auto;
    right: 0;
  }
</style>
