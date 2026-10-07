<!--
  PROBLEMSETS DUE · C · THE TRAY.
  In-tray variant where records sit as individual cards. Includes action bays
  for marking items completed or advancing planned dates, with inline state
  transitions.
-->
<script lang="ts">
  import BlockHead from '../../blocks/BlockHead.svelte';
  import Doors from './Doors.svelte';
  import { dayMonth, emptyWords, shiftDay, type ListCardProps, type ListCardRow } from './props';

  let {
    title,
    view,
    type,
    rows,
    candidates,
    selected,
    menuFor,
    menuRows,
    onSelect,
    onPanel,
    onMenu,
    onNew,
    onPaste,
    onRenumber,
    onDone,
    onLater,
  }: ListCardProps = $props();

  // Transient state for the currently lifted ticket and mid-spill animation.
  let lifted = $state<string | null>(null);
  let spilled = $state<{ id: string; kind: 'done' | 'later' } | null>(null);

  const liftedRow = $derived(rows.find((row) => row.id === lifted) ?? null);

  // Text description of the +7 days reschedule target.
  const laterWords = $derived(
    liftedRow && liftedRow.day
      ? `Planned for ${dayMonth(shiftDay(liftedRow.day, 7))}`
      : 'a later day',
  );

  function lift(row: ListCardRow) {
    if (spilled) return;
    // Take out ⇄ Put back is one control. Putting back is the reference's
    // `Revert` — a lift is never a write, so nothing is dispatched here.
    lifted = lifted === row.id ? null : row.id;
  }

  function spill(kind: 'done' | 'later') {
    const row = liftedRow;
    if (!row || spilled) return;
    spilled = { id: row.id, kind };
    lifted = null;
    const reduce =
      typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true;
    const wait = reduce ? 0 : 140; // --dur-1: the exit, shorter than the lift
    const land = () => {
      spilled = null;
      if (kind === 'done') onDone(row);
      else onLater(row);
    };
    if (wait === 0) land();
    else window.setTimeout(land, wait);
  }
</script>

<section class="cd-card pdt-card" data-view={view} data-type={type}>
  <BlockHead {title} {type} word="List" returned={rows.length} {candidates} {onNew} {onPaste} {onRenumber} />

  {#if rows.length === 0}
    <p class="cd-empty"><span class="cd-empty__s">{emptyWords(candidates, type)}</span></p>
  {:else}
    <div class="pdt-tray">
      <div class="pdt-box">
        <ul class="pdt-tickets">
          {#each rows as row (row.id)}
            <li
              class="pdt-ticket"
              data-lifted={lifted === row.id}
              data-spill={spilled?.id === row.id ? spilled.kind : undefined}
            >
              <button
                class="pdt-press"
                type="button"
                aria-pressed={selected === row.id}
                aria-label={row.sentence}
                onclick={() => onSelect(row.id)}
              >
                <span class="pdt-nm">{row.label}</span>
                <span class="pdt-fact">
                  {#if row.course}
                    <span class="cd-chip cd-chip--code" data-w={row.course.wash}>{row.course.code}</span>
                  {/if}
                  {#if row.due}
                    <span class="pdt-when num" data-tone={row.late ? 'late' : undefined}>{row.due}</span>
                  {/if}
                </span>
              </button>

              <div class="pdt-ticket__acts">
                <Doors {row} open={menuFor === row.id} {menuRows} {onPanel} {onMenu} />
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm pdt-lift"
                  type="button"
                  aria-pressed={lifted === row.id}
                  aria-label={`${lifted === row.id ? 'Put back' : 'Take out'} ${row.label}`}
                  onclick={() => lift(row)}
                >
                  <span data-face="out">Take out</span>
                  <span data-face="back">Put back</span>
                </button>
              </div>
            </li>
          {/each}
        </ul>

        <div class="pdt-lip" aria-hidden="true"></div>
      </div>

      <div class="pdt-bays">
        <button
          class="pdt-bay"
          type="button"
          data-command="record.advanceStage"
          data-placement="recordTable.row"
          disabled={!liftedRow}
          data-armed={liftedRow ? 'true' : 'false'}
          onclick={() => spill('done')}
        >
          <span class="pdt-bay__k">Done</span>
          <span class="pdt-bay__s num">finished</span>
        </button>
        <button
          class="pdt-bay"
          type="button"
          data-command="record.defer"
          data-placement="recordTable.row"
          disabled={!liftedRow}
          data-armed={liftedRow ? 'true' : 'false'}
          onclick={() => spill('later')}
        >
          <span class="pdt-bay__k">Later</span>
          <span class="pdt-bay__s num">{laterWords}</span>
        </button>
      </div>
    </div>
  {/if}
</section>

<style>
  /* The card is the container: the tray reflows with the room it is in, which
     a media query cannot see in a 360px record panel. */
  .pdt-card {
    container-type: inline-size;
  }

  .pdt-tray {
    /* The system ships an ease-out and no ease-in; an exit is not an entrance,
       so the spill eases in and runs shorter than the lift that preceded it. */
    --pdt-ease-in: cubic-bezier(0.4, 0, 1, 1);
    display: grid;
    gap: var(--ui-gap-sm);
  }

  /* The base: one recessed surface, mixed off `--well` rather than `--card`,
     so it stays deeper than the card in both themes. No back board, no walls. */
  .pdt-box {
    position: relative;
    overflow: hidden;
    padding: var(--ui-pad) var(--ui-pad) 0;
    background-color: color-mix(in oklab, var(--wash-peach) 62%, var(--well));
    border-radius: var(--r-tile);
    box-shadow: inset 0 0 0 1px var(--rule), var(--catch);
  }

  /* The base holds a wrapping grid of tracks, not one row: a view with twelve
     records is twelve tickets, and a single flex row squeezed them to ~65px
     each, which broke every name mid-word. `minmax(calc(150px * var(--ui-s)),
     1fr)` gives every ticket its own track (never a `flex-basis` in one row),
     and `align-items: end` stands each one on the base at its own content
     height rather than stretching it to the row's tallest. */
  .pdt-tickets {
    position: relative;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(150px * var(--ui-s)), 1fr));
    align-items: end;
    gap: var(--ui-gap-sm);
    margin: 0;
    padding: 0 var(--space-md);
    list-style: none;
    min-height: calc(150px * var(--ui-s));
  }

  /* A ticket: paper standing on the base, its width its own grid track and its
     height its content. Its position and its leaving fade are the only things
     that move; shadows stay state-explicit. */
  .pdt-ticket {
    min-width: 0;
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    padding: var(--ui-pad-sm) var(--space-sm) var(--ui-pad);
    min-height: calc(138px * var(--ui-s));
    background: var(--card);
    border-radius: var(--r-item) var(--r-item) var(--r-mini) var(--r-mini);
    box-shadow: var(--sh-1), inset 0 0 0 1px var(--rule);
    translate: 0 0;
    transition: translate var(--dur-2) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  /* Lifted: the ticket rises off the base and its edge firms up. */
  .pdt-ticket[data-lifted='true'] {
    z-index: 3;
    box-shadow: var(--sh-2), inset 0 0 0 1.5px var(--rule-strong);
    translate: 0 calc(-14px * var(--ui-s));
  }
  /* Spilled: an exit, so shorter and easing in — the reader already knows the
     ticket is leaving. It travels only as far as the tray's own clip, so the
     sink behind the lip and the fade are both seen. */
  .pdt-ticket[data-spill] {
    transition: translate var(--dur-1) var(--pdt-ease-in), opacity var(--dur-1) var(--pdt-ease-in);
  }
  .pdt-ticket[data-spill='done'] {
    translate: 0 calc(30px * var(--ui-s));
    opacity: 0;
  }
  .pdt-ticket[data-spill='later'] {
    translate: calc(20px * var(--ui-s)) calc(22px * var(--ui-s));
    opacity: 0;
  }

  /* The object's own control: the press that marks the record. */
  .pdt-press {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2xs);
    min-width: 0;
    padding: 0;
    text-align: left;
  }
  .pdt-press:hover .pdt-nm {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .pdt-nm {
    font-size: calc(var(--text-base) * var(--ui-s));
    line-height: 1.3;
    color: var(--ink);
    overflow-wrap: anywhere;
    text-wrap: pretty;
  }
  .pdt-fact {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    min-width: 0;
    flex-wrap: wrap;
  }
  .pdt-when {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
  }
  .pdt-when[data-tone='late'] {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* The ticket's foot wraps rather than overflowing: at a 150px track the door
     cluster and the lift pill cannot share one line, so `Doors` stands above
     `Take out` instead of being clipped past the ticket's edge. */
  .pdt-ticket__acts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2xs);
    margin-top: auto;
    min-width: 0;
  }
  /* Take out ⇄ Put back is one control standing in one place: its two faces
     are stacked in the same cell and crossfade, so its width never moves. */
  /* `.cd-pill.pdt-lift`, not `.pdt-lift`: the app's own `.cd-pill` sets
     `display: inline-flex`, and the two rule sets tie on specificity — the
     stacked faces need the grid whichever stylesheet lands last. */
  .cd-pill.pdt-lift {
    display: inline-grid;
    align-items: center;
    justify-items: start;
  }
  .cd-pill.pdt-lift > * {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
    transition: opacity var(--dur-1) var(--ease);
  }
  .cd-pill.pdt-lift[aria-pressed='true'] [data-face='out'],
  .cd-pill.pdt-lift[aria-pressed='false'] [data-face='back'] {
    opacity: 0;
  }

  /* The lip: the boundary, drawn. One rule across the tray's near edge with
     its own shallow face under it, flush with the base's bottom corners so the
     two curves are concentric. `--ink-4`: a drawn boundary, never ink. */
  .pdt-lip {
    position: relative;
    z-index: 2;
    height: calc(6px * var(--ui-s));
    margin: 0 calc(var(--ui-pad) * -1);
    background: var(--well-2);
    border-top: 2px solid var(--ink-4);
    border-radius: 0 0 var(--r-tile) var(--r-tile);
  }

  /* The two outcomes, past the lip. Their ladder is legibility, not dimming:
     armed = a raised plate, available = a ringed plate, waiting = the plate
     alone with its words at `--ink-3` (never `--ink-4`, which is non-text, and
     never an opacity that measures under 3:1). */
  .pdt-bays {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--ui-gap-sm);
  }
  .pdt-bay {
    display: grid;
    gap: 2px;
    justify-items: start;
    min-height: calc(62px * var(--ui-s));
    padding: var(--ui-pad-sm) var(--space-md);
    border-radius: var(--r-tile);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--rule-strong);
    transition: background var(--dur-1) var(--ease);
  }
  .pdt-bay:hover {
    background: var(--well-2);
  }
  .pdt-bay:active {
    transform: scale(0.99);
  }
  .pdt-bay[disabled] {
    pointer-events: none;
    box-shadow: none;
  }
  .pdt-bay[disabled] .pdt-bay__k,
  .pdt-bay[disabled] .pdt-bay__s {
    color: var(--ink-3);
  }
  .pdt-bay[data-armed='true'] {
    background: var(--well-2);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong);
  }
  .pdt-bay__k {
    font-size: calc(var(--text-xs) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink);
  }
  .pdt-bay__s {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* Narrow: the base's grid stands one ticket per row — each keeps the full
     width it has — and the two bays stack, so nothing overflows the page. */
  @container (max-width: 520px) {
    .pdt-tickets {
      grid-template-columns: minmax(0, 1fr);
      padding: 0 var(--space-sm);
    }
    .pdt-bays {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* Reduced motion: nothing travels or lifts; the write lands in the press's
     own task (the script reads the same query for its wait). */
  @media (prefers-reduced-motion: reduce) {
    .pdt-ticket,
    .pdt-ticket[data-spill] {
      transition: none;
      translate: 0 0;
    }
  }
</style>
