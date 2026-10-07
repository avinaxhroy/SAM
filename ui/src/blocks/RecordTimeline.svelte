<!--
  Timeline record block: renders records sequentially along a dashed spine,
  displaying chronological order and date markers (§3.6).
-->
<script lang="ts">
  import { blockRows, type RecordDoc } from '../types';
  import type { ShapeProps } from '../variants/views/props';

  let { node, facts, selected, onSelect }: ShapeProps = $props();

  const byId = $derived(new Map(facts.map((fact) => [fact.id, fact])));

  type Group = { key: string | null; records: RecordDoc[] };

  const groups = $derived<Group[]>(
    node.grouped && node.groups ? node.groups : [{ key: null, records: blockRows(node) }],
  );
</script>

{#each groups as group, index (group.key ?? `group-${index}`)}
  {#if group.key !== null}
    <p class="vw-group">{group.key}</p>
  {/if}
  <ul class="vw-spine">
    {#each group.records as record (record.id)}
      {@const fact = byId.get(record.id)}
      <li class="vw-tick-li">
        <button
          class="vw-tick"
          type="button"
          data-record-id={record.id}
          data-late={fact?.late ? 'true' : undefined}
          aria-pressed={selected === record.id}
          aria-label={fact?.sentence ?? ''}
          onclick={() => onSelect(record.id)}
        >
          {#if fact?.dayWords}
            <span class="vw-tick__d">{fact.dayWords}</span>
          {/if}
          <span class="vw-tick__t">{fact?.label}</span>
        </button>
      </li>
    {/each}
  </ul>
{/each}

<style>
  .vw-group {
    margin: var(--ui-gap) 0 var(--space-2xs);
    font-size: var(--text-2xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .vw-spine {
    list-style: none;
    margin: 0;
    padding: 0 0 0 var(--space-lg);
    border-left: 1px dashed var(--rule-strong);
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-2xs);
    min-width: 0;
  }
  .vw-tick-li {
    min-width: 0;
  }
  .vw-tick {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: 100%;
    min-width: 0;
    min-height: calc(44px * var(--ui-s));
    padding: 0 var(--space-xs);
    border-radius: var(--r-item);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  /* The spine's own mark: on the rule, at the tick's centre line. */
  .vw-tick::before {
    content: '';
    position: absolute;
    left: calc(-1 * var(--space-lg) - 3px);
    top: 50%;
    margin-top: -3px;
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--ink-3);
  }
  .vw-tick:hover {
    background: var(--well);
  }
  .vw-tick:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .vw-tick[aria-pressed='true'] {
    background: var(--well-2);
  }
  /* The record you are on is the one mark in ink — the token model's own
     "you are here". */
  .vw-tick[aria-pressed='true']::before {
    background: var(--ink);
  }
  .vw-tick__d {
    flex: none;
    width: calc(104px * var(--ui-s));
    font-size: var(--text-2xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .vw-tick__t {
    flex: 1 1 auto;
    min-width: 0;
    font-size: var(--ui-text);
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vw-tick[data-late='true'] .vw-tick__d {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* Narrow: the date keeps its line, the name gets the rest. */
  @container (max-width: 520px) {
    .vw-tick__d {
      width: calc(84px * var(--ui-s));
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .vw-tick {
      transition: none;
    }
  }
</style>
