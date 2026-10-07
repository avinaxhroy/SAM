<!--
  SYSTEM · ONE PLACE, AND ITS ROWS (2026-09-29).

  The machine's five places are the same five indexes in every design: a name,
  its key in the identity pair, at most two facts, one chip that opens the row's
  own editor. This component draws that index, and nothing else — where the
  editor goes is the design's own decision (`panel` is a snippet the design
  passes in; A passes none, because its editor is a sheet).

  `room` is A's own frame: while its sheet is open, the place behind is drawn as
  the room the sheet grew out of, the row it came from keeps `aria-expanded`
  and is marked with `data-origin`, and that mark is a **fill** (`--well`), never
  ink — the sheet is the screen's one dark object while it is open.

  `frame` moves only the structural numbers (the row's own height, the card's
  padding), so the same markup reads as an index, a room or a document section.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import { MARKS, type Group, type Place, type Row, type SystemActions } from './props';

  let {
    place,
    rows,
    groups = [],
    openKey = null,
    room = false,
    frame = 'index',
    panel = null,
    acts,
    trailing = null,
  }: {
    place: Place;
    rows: Row[];
    /** The rows that follow the index — the settings, the checks, the rail. */
    groups?: Group[];
    openKey?: string | null;
    /** A's room: the open row is marked, not expanded. */
    room?: boolean;
    frame?: 'index' | 'room' | 'doc';
    /** The row's editor, drawn under the row by the design that owns it. */
    panel?: Snippet<[Row]> | null;
    acts: SystemActions;
    trailing?: Snippet | null;
  } = $props();
</script>

<section class="cd-card" class:sys-doc={frame === 'doc'} data-place={place.id}>
  <div class="cd-card__head">
    <span class="cd-ictile"><Icon name={place.mark} /></span>
    <div>
      <h2 class="cd-card__title">{place.label}</h2>
      <p class="cd-card__sub">{place.sub}</p>
    </div>
    <span class="cd-card__spacer"></span>
    {@render trailing?.()}
  </div>

  {#snippet rowLine(row: Row)}
    {@const open = openKey === row.key}
    <div
      class="cd-coll__row"
      class:sys-origin={room && open}
      data-row={row.key}
      data-origin={room && open ? '1' : undefined}
    >
      <span class="sys-mark" aria-hidden="true"><Icon name={MARKS[row.mark] ? row.mark : 'dot'} size={12} /></span>
      <span class="cd-coll__body">
        <span class="cd-coll__title">{row.title}</span>
        <span class="cd-rowmeta">
          <IdPair value={row.key} title={`Copy “${row.key}”`} />
          {#each row.facts as fact, index (fact + index)}<span>{fact}</span>{/each}
          {#each row.chips as chip, index (chip.label + index)}
            <span class="cd-chip" class:cd-chip--outline={chip.mod === 'outline'} class:cd-chip--risk={chip.mod === 'risk'} class:cd-chip--ok={chip.mod === 'ok'} class:cd-chip--info={chip.mod === 'info'}>{chip.label}</span>
          {/each}
        </span>
        {#if row.tail}
          <span class="sys-rowtail">{row.tail}</span>
        {/if}
      </span>
      <span class="cd-coll__right">
        {#if row.terminal}
          <IdPair label="Terminal" value={row.terminal} />
        {:else if row.opens}
          <button
            class="cd-chip cd-chip--outline sys-open"
            type="button"
            aria-expanded={open}
            aria-label={`${row.action} — ${row.title}`}
            onclick={() => acts.onOpen(row.key)}
          >
            {open ? 'Close' : row.action}
          </button>
        {/if}
      </span>
    </div>
  {/snippet}

  {#if rows.length === 0}
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name={place.mark} size={24} /></span>
      <p class="cd-empty__t">Nothing in this part of the machine</p>
      <p class="cd-empty__s">A plan that declares none of these is a plan that holds nothing yet.</p>
    </div>
  {:else}
    <div class="cd-coll">
      {#each rows as row (row.key)}
        {@render rowLine(row)}
        {#if panel && openKey === row.key}
          {@render panel(row)}
        {/if}
      {/each}
    </div>
  {/if}

  {#each groups as group (group.heading)}
    <h3 class="cd-sec"><span class="cd-sec__t">{group.heading}</span></h3>
    <div class="cd-coll">
      {#each group.rows as row (row.key)}
        {@render rowLine(row)}
      {/each}
    </div>
  {/each}
</section>

<style>
  /* The row's own air, from the size register: the register moves with the
     student's *Interface size*, and a machine row is denser than a work row
     (it is an index, not a record). */
  .cd-coll__row { min-height: calc(48px * var(--ui-s)); }
  .cd-coll__row--compact,
  .cd-coll :global(.cd-coll__row--compact) { min-height: var(--ui-row-h); }

  /* Where a row's editor came from. A fill, never ink: the sheet is the screen's
     one dark object while it is open, and the marked row's own meta line steps
     up one register so a marked row is never the one place under the text floor. */
  .sys-origin { background: var(--well) !important; }
  .sys-origin .cd-rowmeta { color: var(--ink-2); }

  /* A row's own hint, under its facts — a sentence, never a key. */
  .sys-rowtail { display: block; margin-top: var(--space-3xs); font-size: var(--ui-meta); color: var(--ink-3); }

  /* The compact species: the title and its facts share ONE line. */
  .cd-rowmeta { flex-wrap: wrap; row-gap: var(--space-3xs); }

  .sys-doc { padding-inline: 0; }
  .sys-doc .cd-card__head { padding-inline: var(--ui-pad); }
  @container (max-width: 520px) {
    .cd-coll__right { flex-wrap: wrap; justify-content: flex-end; }
  }
</style>
