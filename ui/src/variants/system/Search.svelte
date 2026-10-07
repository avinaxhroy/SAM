<!--
  SYSTEM · THE ONE SEARCH (2026-09-29).

  One field for the whole machine, and one result list: a kind, a column, a
  screen, a rail place, a figure, a ladder, a schedule or a check, each carrying
  the mark and the words that say which kind of thing it is. The shipped screen
  drew this as a card of `.cd-coll__row--btn`s (`System.svelte:hits()`); the lab
  keeps the card and the rows and this is that card, shared by all three designs
  (the lab shares `searchCard()` between A, B and C).

  Every hit is a real button, so `Enter` opens it; opening a hit goes to the
  place that owns it and opens that row there, which is the design's own answer
  to "find it, then show me where it lives".
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import { PLACE_LABEL } from './places';
  import { MARKS, type Hit, type SystemActions } from './props';

  let { hits, acts }: { hits: Hit[]; acts: SystemActions } = $props();
</script>

<section class="cd-card">
  <div class="cd-card__head">
    <span class="cd-ictile"><Icon name="search" /></span>
    <div>
      <h2 class="cd-card__title">Everything that matches</h2>
      <p class="cd-card__sub">
        {hits.length === 0 ? 'nothing in the machine' : hits.length === 1 ? 'one thing' : `${hits.length} things`}
      </p>
    </div>
    <span class="cd-card__spacer"></span>
    <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => acts.onClear()}>Clear</button>
  </div>
  <p class="cd-sr" role="status">
    {hits.length === 0 ? 'Nothing in the machine matches.' : `${hits.length} results.`}
  </p>

  {#if hits.length === 0}
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="search" size={24} /></span>
      <p class="cd-empty__t">Nothing in the machine matches that</p>
      <p class="cd-empty__s">
        Kinds, columns, screens, figures, ladders, schedules and the checks the app runs are all searched.
      </p>
      <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => acts.onClear()}>
        Clear the search
      </button>
    </div>
  {:else}
    <div class="cd-coll">
      {#each hits as hit (`${hit.place}/${hit.what}/${hit.key}`)}
        <button class="cd-coll__row cd-coll__row--btn" type="button" onclick={() => acts.onGoHit(hit)}>
          <span class="sys-mark" aria-hidden="true"><Icon name={MARKS[hit.mark] ? hit.mark : 'dot'} size={12} /></span>
          <span class="cd-coll__body">
            <span class="cd-coll__title">{hit.title}</span>
            <span class="cd-rowmeta">
              <IdPair value={hit.key} title={`Copy “${hit.key}”`} />
              <span>{hit.meta}</span>
              <span class="cd-chip cd-chip--outline">{hit.what} · {PLACE_LABEL[hit.place]}</span>
            </span>
          </span>
          <span class="cd-coll__right">
            <span class="cd-chip cd-chip--outline">Open</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .cd-coll__row { min-height: calc(48px * var(--ui-s)); }
  .cd-rowmeta { flex-wrap: wrap; row-gap: var(--space-3xs); }
</style>
