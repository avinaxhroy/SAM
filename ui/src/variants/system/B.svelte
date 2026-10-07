<!--
  SYSTEM · B · THE WORKBENCH.
  Inline workbench variant where system configuration places expand directly
  within the view, supporting inline block editing and standalone page navigation.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import Cells from './Cells.svelte';
  import Editor from './Editor.svelte';
  import Head from './Head.svelte';
  import Place from './Place.svelte';
  import Receipt from './Receipt.svelte';
  import Recovery from './Recovery.svelte';
  import Rail from './Rail.svelte';
  import Search from './Search.svelte';
  import type { SystemProps } from './props';

  let {
    places,
    index,
    place,
    strip,
    query,
    searching,
    openKey,
    editor,
    page,
    projection,
    receipt,
    recovering,
    backups,
    restoring,
    hits,
    rail,
    iconNames,
    busy,
    acts,
  }: SystemProps = $props();

  const where = $derived(places.find((entry) => entry.id === place) ?? places[0]);
  const here = $derived(index[place] ?? { rows: [], groups: [] });

  /** Which block cell the student picked — the grid *is* the layout, so this is
   *  the workbench's own working state, not something the plan holds. */
  let cell = $state<number | null>(null);
</script>

<div class="v-fit sys-b">
  <Head {strip} {query} {places} {place} {recovering} {acts} />
  {#if receipt}<Receipt {receipt} {acts} />{/if}

  {#if searching}
    <Search {hits} {acts} />
  {:else if recovering}
    <Recovery {backups} {restoring} {acts} />
  {:else if page}
    <!-- The row opened as its own page: the same body, at the width of the
         canvas, with the way back at the top. -->
    <section class="cd-card" data-page={page.place}>
      <div class="cd-card__head">
        <span class="cd-ictile"><Icon name={where.mark} /></span>
        <div>
          <h2 class="cd-card__title">{page.title}</h2>
          <p class="cd-card__sub">{page.sub}</p>
        </div>
        <span class="cd-card__spacer"></span>
        <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => acts.onBack()}>Close</button>
      </div>
      <Editor editor={page} {projection} {busy} frame="page" {acts} />
    </section>
  {:else}
    {#snippet under(rowKey: string)}
      {@const open = editor}
      <div class="cd-detail sys-detail">
        {#if open}
          {#if open.place === 'screens' && open.blocks}
            <h3 class="cd-sec">
              <span class="cd-sec__t">Its blocks, as the cells they are</span>
              <span class="cd-sec__act cd-hint">the layout this screen draws</span>
            </h3>
            <Cells
              blocks={open.blocks.nodes}
              selected={cell}
              onSelect={(index) => (cell = cell === index ? null : index)}
            />
          {/if}
          <Editor editor={open} {projection} {busy} frame="index" {acts} />
          <div class="sys-acts">
            <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => acts.onPage(rowKey)}>
              Open it as its own page
            </button>
          </div>
        {/if}
      </div>
    {/snippet}

    <Place place={where} rows={here.rows} groups={here.groups} {openKey} panel={under} {acts} />
    {#if place === 'screens'}<Rail {rail} {iconNames} {acts} />{/if}
  {/if}
</div>

<style>
  .sys-b { display: grid; gap: var(--ui-gap); }
  .sys-acts { margin-top: var(--ui-gap-sm); }
  .cd-detail { animation: sys-arrive var(--dur-2) var(--ease) both; }
  @keyframes sys-arrive {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .cd-detail { animation: none; }
  }
</style>
