<!-- SYSTEM · C · The Ladder. Continuous document layout with stacked configuration places. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
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
    backups,
    restoring,
    hits,
    rail,
    iconNames,
    busy,
    acts,
  }: SystemProps = $props();

  const where = $derived(places.find((entry) => entry.id === page?.place) ?? places[0]);
</script>

<div class="v-fit sys-c">
  {#if page}
    <!-- The full page: a breadcrumb in the app's own words, the pagehead, and
         the same body the row drew — at the width of the canvas. -->
    <nav class="yc-crumb" aria-label="Where this page sits">
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => acts.onBack()}>
        The machine
      </button>
      <span aria-hidden="true"><Icon name="arrowup" size={11} /></span>
      <span>{where.label}</span>
      <span aria-hidden="true"><Icon name="arrowup" size={11} /></span>
      <b>{page.title}</b>
    </nav>

    <section class="cd-pagehead">
      <div>
        <h1 class="cd-pagehead__title">{page.title}</h1>
        <p class="cd-pagehead__sub">{page.sub}</p>
      </div>
      <div class="cd-pagehead__aside">
        <span class="cd-strip"><span class="cd-strip__v">{where.label}</span></span>
        <!-- The full page's own head: the same place the shared pagehead puts
             the switch, so the surface reads the same on the page and in the
             index. -->
      </div>
    </section>

    {#if receipt}<Receipt {receipt} {acts} />{/if}

    <section class="cd-card">
      <Editor editor={page} {projection} {busy} frame="page" {acts} />
    </section>
  {:else}
    <Head {strip} {query} {acts} />
    {#if receipt}<Receipt {receipt} {acts} />{/if}

    {#if searching}
      <Search {hits} {acts} />
    {:else}
      <div class="yc-doc">
        {#each places as section (section.id)}
          {@const part = index[section.id] ?? { rows: [], groups: [] }}
          {#snippet inRow(rowKey: string)}
            {#if editor && place === section.id}
              <div class="cd-detail sys-detail">
                <Editor editor={editor} {projection} {busy} frame="index" {acts} />
                <div class="sys-acts">
                  <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => acts.onPage(rowKey)}>
                    Open its own page
                  </button>
                </div>
              </div>
            {/if}
          {/snippet}
          <Place
            place={section}
            rows={part.rows}
            groups={part.groups}
            {openKey}
            frame="doc"
            panel={inRow}
            {acts}
          />
          {#if section.id === 'screens'}<Rail {rail} {iconNames} {acts} />{/if}
        {/each}
        <Recovery {backups} {restoring} {acts} />
      </div>
    {/if}
  {/if}
</div>

<style>
  .yc-doc { display: grid; grid-template-columns: minmax(0, 1fr); gap: var(--ui-gap-lg); }
  .yc-crumb {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    flex-wrap: wrap;
    margin-bottom: var(--ui-gap);
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .yc-crumb b { color: var(--ink); font-weight: var(--weight-label); }

  /* A chosen option is a RINGED tick carrying an ink check — not the shipped ink
     dot — so C keeps exactly one dark object per screen: the pill that writes. */
  :global(.sys-c .sys-pick[data-sel='1'] .sys-pick__tick) {
    background: transparent;
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }
  .sys-acts { margin-top: var(--ui-gap-sm); }
</style>
