<!--
  Record layout container: renders card and header species across 8 layout definitions (§3.6).
  Delegates body presentation to specialized block views.
-->
<script lang="ts">
  import BlockHead from '../../blocks/BlockHead.svelte';
  import RecordBoard from '../../blocks/RecordBoard.svelte';
  import RecordCalendar from '../../blocks/RecordCalendar.svelte';
  import RecordGraph from '../../blocks/RecordGraph.svelte';
  import RecordList from '../../blocks/RecordList.svelte';
  import RecordTable from '../../blocks/RecordTable.svelte';
  import RecordTimeline from '../../blocks/RecordTimeline.svelte';
  import RecordTree from '../../blocks/RecordTree.svelte';
  import { blockRows, nameOf } from '../../types';
  import { emptyWords, factsOf, layoutWord, type ShapeProps, type ViewsProps } from './props';

  let { node, type, targetsFor, today, selected, onSelect, onNew }: ViewsProps = $props();

  const rows = $derived(blockRows(node));
  /** Every fact the five drawings read, derived once. */
  const shape = $derived<ShapeProps>({ node, type, facts: factsOf(rows, type, today), selected, onSelect });
  const word = $derived(layoutWord(node.layout ?? node.kind));
  const title = $derived(node.title ?? nameOf(node.view ?? ''));
</script>

{#if node.kind === 'table'}
  <!-- The dense species has its own surface and its own head. -->
  <RecordTable {node} {targetsFor} />
{:else if node.kind === 'list' || node.kind === 'cardGrid'}
  <!-- The two cell layouts own their cards (`list` through the list card's own
       design, `cardGrid` through the tiles below). -->
  <RecordList {node} {targetsFor} />
{:else}
  <section
    class="cd-card vw-card"
    data-block={node.kind}
    data-view={node.view}
    data-type={node.type}
    data-shape={node.kind}
  >
    <BlockHead
      {title}
      type={node.type ?? ''}
      {word}
      returned={rows.length}
      candidates={node.candidates ?? rows.length}
      {onNew}
    />

    {#if rows.length === 0}
      <!-- The read with nothing in it: the app's own sentence, once per block
           rather than seven times in seven drawings. -->
      <p class="cd-empty"><span class="cd-empty__s">
        {emptyWords(node.candidates ?? 0, node.type ?? '')}
      </span></p>
    {:else if node.kind === 'board'}
      <RecordBoard {...shape} />
    {:else if node.kind === 'timeline'}
      <RecordTimeline {...shape} />
    {:else if node.kind === 'calendar'}
      <RecordCalendar {...shape} />
    {:else if node.kind === 'tree'}
      <RecordTree {...shape} />
    {:else if node.kind === 'graph'}
      <RecordGraph {...shape} />
    {/if}
  </section>
{/if}

<style>
  /* Inline-size container context for responsive layout of child record shapes. */
  .vw-card {
    container-type: inline-size;
  }
</style>
