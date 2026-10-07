<!--
  Tree / Outline record block: renders parent-child hierarchy based on declared
  type relations with recursion depth caps and loop prevention (§3.6).
-->
<script lang="ts">
  import { blockRows, recordLabel, type RecordDoc, type TypeRead } from '../types';
  import type { ShapeProps } from '../variants/views/props';

  let { node, type, facts, selected, onSelect }: ShapeProps = $props();

  const byId = $derived(new Map(facts.map((fact) => [fact.id, fact])));

  /**
   * Nesting stops at twelve levels: the same number the block walk uses,
   * restated here because a tree has a second way to go deep — the *data*.
   */
  const MAX_TREE_DEPTH = 12;

  const records = $derived(blockRows(node));
  const parent = $derived(type?.parent ?? null);

  type Branch = { record: RecordDoc; children: Branch[] };

  /**
   * The forest, in the engine's order. Children are bucketed by parent id once,
   * then every record is placed exactly once: the roots first, then anything the
   * walk did not reach — which is how a record inside a parent loop (by
   * definition no root) is still drawn instead of disappearing.
   */
  function forest(rows: RecordDoc[], edge: string | null): Branch[] {
    const ids = new Set(rows.map((record) => record.id));
    const children = new Map<string | null, RecordDoc[]>();
    for (const record of rows) {
      const linked = edge === null ? null : (record.links?.[edge]?.[0] ?? null);
      const key = linked !== null && ids.has(linked) ? linked : null;
      const bucket = children.get(key);
      if (bucket) bucket.push(record);
      else children.set(key, [record]);
    }

    const visited = new Set<string>();
    const place = (record: RecordDoc, depth: number): Branch => {
      visited.add(record.id);
      const kids =
        depth >= MAX_TREE_DEPTH
          ? []
          : (children.get(record.id) ?? [])
              .filter((child) => !visited.has(child.id))
              .map((child) => place(child, depth + 1));
      return { record, children: kids };
    };

    const roots = (children.get(null) ?? []).map((record) => place(record, 0));
    for (const record of rows) {
      if (!visited.has(record.id)) roots.push(place(record, 0));
    }
    return roots;
  }

  const branches = $derived(parent === null ? [] : forest(records, parent));

  /** The edge as the type read declares it — read off the node's own type key. */
  const typeKey = $derived(node.type ?? '');
</script>

{#if parent === null}
  <p class="vw-note">
    {typeKey === '' ? 'this type' : typeKey} declares no parent type, so there is no edge to nest
    by — the outline is a flat list with a mark.
  </p>
{/if}

{#snippet level(nodes: Branch[])}
  <ul class="vw-rows">
    {#each nodes as branch (branch.record.id)}
      {@const fact = byId.get(branch.record.id)}
      <li class="vw-branch">
        <button
          class="vw-row"
          type="button"
          data-record-id={branch.record.id}
          aria-pressed={selected === branch.record.id}
          aria-label={fact?.sentence ?? recordLabel(branch.record)}
          onclick={() => onSelect(branch.record.id)}
        >
          <!-- The mark is a mark: a leaf is a dot, a branch a chevron, and the
               row's own accessible name does not repeat it. -->
          <span class="vw-mark" aria-hidden="true">
            {branch.children.length > 0 ? '▸' : '·'}
          </span>
          <span class="vw-row__t">{fact?.label ?? recordLabel(branch.record)}</span>
          {#if fact?.dayWords}
            <span class="vw-row__f">{fact.dayWords}</span>
          {/if}
        </button>
        {#if branch.children.length > 0}
          <div class="vw-children">
            {@render level(branch.children)}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

{#if branches.length > 0}
  {@render level(branches)}
{:else}
  <ul class="vw-rows">
    {#each records as record (record.id)}
      {@const fact = byId.get(record.id)}
      <li class="vw-branch">
        <button
          class="vw-row"
          type="button"
          data-record-id={record.id}
          aria-pressed={selected === record.id}
          aria-label={fact?.sentence ?? recordLabel(record)}
          onclick={() => onSelect(record.id)}
        >
          <span class="vw-mark" aria-hidden="true">·</span>
          <span class="vw-row__t">{fact?.label ?? recordLabel(record)}</span>
          {#if fact?.dayWords}
            <span class="vw-row__f">{fact.dayWords}</span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .vw-rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
  }
  .vw-branch {
    min-width: 0;
  }
  .vw-children {
    padding-left: var(--ui-pad);
  }
  .vw-row {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: 100%;
    min-width: 0;
    min-height: var(--ui-row-h);
    padding: 0 var(--space-xs);
    border-radius: var(--r-item);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .vw-row:hover {
    background: var(--well);
  }
  .vw-row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .vw-row[aria-pressed='true'] {
    background: var(--well-2);
  }
  /* `--ink-3`, never the app's `--ink-4`: a bullet beside a record's own name
     is read as text. */
  .vw-mark {
    flex: none;
    width: 14px;
    text-align: center;
    color: var(--ink-3);
    font-size: var(--ui-text-sm);
  }
  .vw-row__t {
    flex: 1 1 auto;
    min-width: 0;
    font-size: var(--ui-text);
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vw-row__f {
    flex: none;
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .vw-note {
    margin-bottom: var(--ui-gap-sm);
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }

  @media (prefers-reduced-motion: reduce) {
    .vw-row {
      transition: none;
    }
  }
</style>
