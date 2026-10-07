<!--
  Board record block: renders records grouped into columns based on the
  engine's resolved view groupings (§3.6).
-->
<script lang="ts">
  import { blockRows, type RecordDoc } from '../types';
  import type { ShapeProps } from '../variants/views/props';

  let { node, facts, selected, onSelect }: ShapeProps = $props();

  const byId = $derived(new Map(facts.map((fact) => [fact.id, fact])));

  type Column = { key: string | null; records: RecordDoc[] };

  /** The engine's groups, or the one column an ungrouped board is. */
  const columns = $derived<Column[]>(
    node.grouped && node.groups ? node.groups : [{ key: null, records: blockRows(node) }],
  );
</script>

<div class="vw-board">
  {#each columns as column, index (column.key ?? `column-${index}`)}
    <section class="vw-col">
      <p class="vw-colhead">
        <span>{column.key ?? (node.type ?? 'Records')}</span>
        <span class="vw-colcount">{column.records.length}</span>
      </p>
      {#each column.records as record (record.id)}
        {@const fact = byId.get(record.id)}
        <button
          class="vw-mini"
          type="button"
          data-record-id={record.id}
          data-late={fact?.late ? 'true' : undefined}
          aria-pressed={selected === record.id}
          aria-label={fact?.sentence ?? ''}
          onclick={() => onSelect(record.id)}
        >
          <span class="vw-mini__t">{fact?.label}</span>
          {#if fact?.due}
            <span class="vw-mini__f">{fact.due}</span>
          {/if}
        </button>
      {/each}
    </section>
  {/each}
</div>

<style>
  /* A track that can never outgrow its column, so a long name cannot carry the
     page with it (the same rule `ui/src/styles/components.css:397` keeps). */
  .vw-board {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--ui-gap);
    align-items: start;
  }
  .vw-col {
    min-width: 0;
    display: grid;
    gap: var(--ui-gap-sm);
    padding: var(--ui-pad-sm);
    background: var(--well);
    border-radius: var(--r-tile);
  }
  .vw-colhead {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
    font-size: var(--text-2xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
    overflow-wrap: anywhere;
  }
  .vw-colcount {
    margin-left: auto;
    font-variant-numeric: tabular-nums;
  }
  .vw-mini {
    min-width: 0;
    display: grid;
    gap: var(--space-3xs);
    width: 100%;
    min-height: calc(56px * var(--ui-s));
    padding: var(--ui-pad-sm);
    text-align: left;
    background: var(--card);
    border-radius: var(--r-item);
    box-shadow: var(--sh-1);
    transition: box-shadow var(--dur-2) var(--ease), transform var(--dur-2) var(--ease);
  }
  .vw-mini:hover {
    box-shadow: var(--sh-2);
    transform: translateY(-1px);
  }
  .vw-mini:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .vw-mini[aria-pressed='true'] {
    box-shadow: inset 0 0 0 2px var(--ink), var(--sh-1);
  }
  .vw-mini__t {
    font-size: var(--ui-text);
    font-weight: var(--weight-label);
    color: var(--ink);
    overflow-wrap: anywhere;
  }
  .vw-mini__f {
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .vw-mini[data-late='true'] .vw-mini__f {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  @media (prefers-reduced-motion: reduce) {
    .vw-mini {
      transition: none;
    }
    .vw-mini:hover {
      transform: none;
    }
  }
</style>
