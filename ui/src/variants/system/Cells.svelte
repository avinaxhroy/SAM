<!--
  SYSTEM · B'S TRADING CELLS (2026-09-29).

  B's own mechanic, taken from `refs/uselayouts/registry/default/example/
  fluid-expanding-grid.tsx`: the items are held in **rows that are arrays**, the
  selected item takes `grid-column: 1 / span 2`, and its row-mate is pushed into
  the row below. Re-expressed here as the layout a view *is*: a screen's blocks
  are the cells of a two-column grid, picking one makes it the wide cell, and
  the block it sat beside moves down.

  What the reference's spring is refused: a grid cannot tween a `grid-column`
  span, so the shape change is **instant** and only the arrival is filled on
  `--dur-2`. Nothing here is a second dark object — a chosen cell is `--well-2`
  with a ring, never ink.
-->
<script lang="ts">
  import { BLOCKS, plural } from './props';

  let {
    blocks,
    selected = null,
    onSelect,
  }: {
    blocks: ReadonlyArray<Record<string, unknown>>;
    selected?: number | null;
    onSelect: (index: number) => void;
  } = $props();

  function said(block: Record<string, unknown>, key: string): string {
    const value = block[key];
    return typeof value === 'string' && value.trim().length > 0 ? value : '';
  }

  function kindWord(kind: unknown): string {
    const name = typeof kind === 'string' ? kind : '';
    return BLOCKS[name] ?? 'Block';
  }

  /** The cells, in the order the grid draws them: the chosen one takes its row,
   *  and whatever shared that row is pushed below it. */
  const ordered = $derived.by<Array<{ i: number; block: Record<string, unknown> }>>(() => {
    const items = blocks.map((block, i) => ({ i, block }));
    if (selected === null || !items[selected]) return items;
    const out: Array<{ i: number; block: Record<string, unknown> }> = [];
    for (let at = 0; at < items.length; at += 2) {
      const row = items.slice(at, at + 2);
      if (row.some((cell) => cell.i === selected)) {
        out.push(...row.filter((cell) => cell.i === selected));
        out.push(...row.filter((cell) => cell.i !== selected));
      } else {
        out.push(...row);
      }
    }
    return out;
  });
</script>

<div class="yb-cells">
  {#each ordered as cell (cell.i)}
    {@const first = said(cell.block, 'title') || said(cell.block, 'label') || said(cell.block, 'text')}
    {@const second = said(cell.block, 'view') || said(cell.block, 'expr')}
    {@const children = Array.isArray(cell.block.blocks) ? (cell.block.blocks as unknown[]).length : 0}
    <button
      class="yb-cell"
      type="button"
      data-sel={selected === cell.i ? '1' : '0'}
      aria-pressed={selected === cell.i}
      aria-label={`${kindWord(cell.block.kind)} — cell ${cell.i + 1} of ${blocks.length}`}
      onclick={() => onSelect(cell.i)}
    >
      <span class="yb-cell__k">{kindWord(cell.block.kind)} · {cell.i}</span>
      <span class="yb-cell__t">{first || kindWord(cell.block.kind)}</span>
      {#if second}<span class="yb-cell__s">{second}</span>{/if}
      {#if selected === cell.i}
        <span class="yb-cell__s">
          {plural(blocks.length, 'block')} · it holds {children === 0 ? 'no children' : plural(children, 'child', 'children')}
        </span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .yb-cells {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--ui-gap-sm);
    margin-top: var(--ui-gap);
  }
  .yb-cell {
    display: grid;
    align-content: start;
    gap: var(--space-3xs);
    min-height: calc(92px * var(--ui-s));
    padding: var(--ui-pad-sm) var(--ui-pad);
    border-radius: var(--r-tile);
    background: var(--card);
    box-shadow: inset 0 0 0 1.5px var(--rule);
    text-align: left;
    color: var(--ink);
    transition: background var(--dur-2) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .yb-cell:hover { background: var(--well); }
  .yb-cell[data-sel='1'] {
    grid-column: 1 / -1;
    background: var(--well-2);
    box-shadow: inset 0 0 0 1.5px var(--ink-4);
    min-height: calc(132px * var(--ui-s));
  }
  .yb-cell__k { font-size: var(--ui-meta); letter-spacing: var(--track-caps); text-transform: uppercase; color: var(--ink-3); }
  .yb-cell__t { font-size: var(--ui-text); font-weight: var(--weight-label); letter-spacing: var(--track-title); }
  .yb-cell__s { font-size: var(--ui-text-sm); color: var(--ink-2); }

  @container (max-width: 520px) {
    .yb-cells { grid-template-columns: minmax(0, 1fr); }
  }
  @media (prefers-reduced-motion: reduce) {
    .yb-cell { transition: none; }
  }
</style>
