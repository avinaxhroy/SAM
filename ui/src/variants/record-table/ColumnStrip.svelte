<!--
  COLUMN STRIP.
  Shared schema editor column header strip across table variants. Hosts column
  headers, context menus (rename, retype, hide, move, delete), column reordering
  drag handles, and the trailing '+' action for creating/unhiding columns.
-->
<script lang="ts">
  import ColumnMenu from '../../records/ColumnMenu.svelte';
  import Menu from '../../shell/Menu.svelte';
  import type { MenuRow } from '../../commands/registry';
  import './table.css';
  import type { ColumnFact } from './props';

  let {
    columns,
    hidden,
    plusRows,
    plusOpen,
    dragging,
    dropTarget,
    flexKey = null,
    tone = 'a',
    onPlus,
    onOver,
    onDrop,
  }: {
    columns: ColumnFact[];
    /** How many of the kind's own fields this view hides (`column.show` restores them). */
    hidden: number;
    plusRows: MenuRow[];
    plusOpen: boolean;
    dragging: string | null;
    dropTarget: string | null;
    /** The one column that takes the free space, so a label sits over its cells. */
    flexKey?: string | null;
    /** The design's own material: the strip re-dresses per variant. */
    tone?: 'a' | 'b' | 'c';
    onPlus: (open: boolean) => void;
    onOver: (key: string | null) => void;
    onDrop: (before: string) => void;
  } = $props();
</script>

<div class="cd-labels vt-strip" data-tone={tone}>
  {#each columns as column (column.key)}
    <div
      class="cd-label vt-label"
      role="presentation"
      data-field-type={column.type}
      data-flex={column.key === flexKey ? '1' : '0'}
      class:cd-label--drop={dropTarget === column.key && dragging !== null}
      onclick={(event) => event.stopPropagation()}
      ondragover={(event) => {
        // A preview only: nothing writes while the pointer moves (C.2).
        event.preventDefault();
        onOver(column.key);
      }}
      ondragleave={() => onOver(null)}
      ondrop={(event) => {
        event.preventDefault();
        onDrop(column.key);
      }}
    >
      <ColumnMenu {...column.menu} />
    </div>
  {/each}
  <div
    class="cd-label cd-label--plus"
    role="presentation"
    class:cd-label--drop={dropTarget === '__end__' && dragging !== null}
    onclick={(event) => event.stopPropagation()}
    ondragover={(event) => {
      event.preventDefault();
      onOver('__end__');
    }}
    ondragleave={() => onOver(null)}
    ondrop={(event) => {
      // The end zone: the end of this view's own list.
      event.preventDefault();
      onDrop('');
    }}
  >
    <!-- No command id of its own: this control opens a menu, and the menu's
         items are the doors (D3). The hidden columns are named in the menu that
         restores them, and only counted here. -->
    <button
      class="cd-iconbtn vt-plus"
      type="button"
      title={hidden > 0 ? `${hidden} hidden here, and New column…` : 'New column…'}
      aria-label={hidden > 0 ? `${hidden} hidden columns here, and New column…` : 'New column…'}
      aria-haspopup="menu"
      aria-expanded={plusOpen}
      onclick={() => onPlus(!plusOpen)}
    >
      +
    </button>
    {#if plusOpen}
      <Menu rows={plusRows} native={false} onclose={() => onPlus(false)} />
    {/if}
  </div>
</div>

<style>
  .vt-strip {
    flex: none;
  }
  .vt-strip[data-tone='b'] {
    margin-bottom: var(--ui-gap);
    border-bottom: 0;
    height: calc(28px * var(--ui-s));
  }
  .vt-strip[data-tone='b'] :global(.cd-collabel) {
    color: var(--ink-4);
    letter-spacing: var(--track-caps);
  }
  .vt-strip[data-tone='c'] {
    border-bottom: 1px solid var(--rule);
    height: calc(28px * var(--ui-s));
    margin-bottom: var(--ui-gap-sm);
  }
  .vt-strip[data-tone='c'] :global(.cd-collabel) {
    color: var(--ink-4);
  }
  /* The `+` door keeps the app's 32px floor whatever the strip's own height is. */
  .vt-plus {
    width: var(--hit);
    height: var(--hit);
    flex: none;
    border-radius: var(--r-pill);
  }
</style>
