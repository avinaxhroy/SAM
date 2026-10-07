<!-- System view header with search and place navigation. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import type { Place, PlaceId, Strip, SystemActions } from './props';

  let {
    strip,
    query,
    places = null,
    place,
    recovering = false,
    acts,
  }: {
    strip: Strip[];
    query: string;
    /** The five places, or null where the design is a document (C). */
    places?: Place[] | null;
    place?: PlaceId;
    recovering?: boolean;
    acts: SystemActions;
  } = $props();
</script>

<section class="cd-pagehead">
  <div>
    <h1 class="cd-pagehead__title">The machine</h1>
    <p class="cd-strip" aria-label="What the plan is made of">
      {#each strip as fact, index (fact.label + index)}
        <span class="cd-strip__v">{fact.value}</span>
        <span>{fact.label}</span>
      {/each}
    </p>
  </div>
  <div class="cd-pagehead__aside">
    <!-- ONE search field for the whole machine, and it is the system's own
         `.cd-searchpill`. It filters every place at once; the result rows carry
         the mark that says which kind of thing each hit is. -->
    <label class="cd-searchpill sys-q">
      <Icon name="search" size={14} />
      <span class="cd-sr">Find a kind, a column, a screen, a figure or a ladder</span>
      <input
        type="search"
        value={query}
        autocomplete="off"
        spellcheck="false"
        placeholder="Find in the machine…"
        aria-label="Find in the machine"
        oninput={(event) => acts.onQuery(event.currentTarget.value)}
      />
    </label>
    <!-- The surface's own design switch, last in the head row: System is one
         screen in three designs, and the pagehead is where it stands. -->
  </div>
</section>

{#if places}
  <!-- THE FIVE PLACES. In the flow, not sticky: a bar that had to stay within
       reach would be a bar stuck inside one of them. The lit pill is this
       screen's one dark object. -->
  <nav class="sys-groups" aria-label="Parts of the machine">
    <span class="cd-seg" role="group" aria-label="Part of the machine">
      {#each places as entry (entry.id)}
        <button
          class="cd-seg__pill"
          type="button"
          aria-pressed={!recovering && place === entry.id}
          onclick={() => acts.onPlace(entry.id)}
        >
          {entry.label}
        </button>
      {/each}
    </span>
    <span class="sys-groups__spacer"></span>
    <button
      class="cd-pill cd-pill--ghost cd-pill--sm"
      type="button"
      aria-pressed={recovering}
      onclick={() => acts.onRecover()}
    >
      {recovering ? 'Back to the machine' : 'Recently deleted'}
    </button>
  </nav>
{/if}

<style>
  /* The pill's own ring: the field inside it paints none of its own. */
  .cd-searchpill:focus-within { box-shadow: 0 0 0 2px var(--focus); }
  /* The machine's search is a field, not a chrome affordance: the UA's own
     clear button paints in the platform's accent, which is the one raw colour
     this window refuses. `Clear` beside the results is the affordance. */
  .cd-searchpill input::-webkit-search-cancel-button,
  .cd-searchpill input::-webkit-search-decoration { -webkit-appearance: none; appearance: none; }
  .cd-searchpill input { cursor: text; }
  .sys-groups { gap: var(--ui-gap-sm); }
</style>
