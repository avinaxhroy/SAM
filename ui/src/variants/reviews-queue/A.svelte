<!-- Reviews queue variant A: The Shelves. Grouped by lateness window. -->
<script lang="ts">
  import { stateClass, type QueueProps } from './props';

  let { groups, moved, onPut, onUndo }: QueueProps = $props();

  /** The oldest window that has records — where the work is, and so where the
   *  deck opens when the student has not chosen. */
  const oldestWithWork = $derived(groups.reduce((best, group, index) => (group.rows.length > 0 ? index : best), -1));
  /** Which shelf the student opened, or `null` for “not chosen yet”. `-1` is
   *  “everything folded”, which is a state they can choose — and it is how 204
   *  records hold one card. */
  let picked = $state<number | null>(null);

  // An open window that lost its last record closes itself, so the deck never
  // shows an empty open box.
  const shown = $derived.by(() => {
    const wanted = picked ?? oldestWithWork;
    if (wanted < 0) return -1;
    return (groups[wanted]?.rows.length ?? 0) === 0 ? -1 : wanted;
  });

  function toggle(index: number): void {
    if (groups[index].rows.length === 0) return;
    picked = shown === index ? -1 : index;
  }
</script>

<div class="v-fit rqa">
  <div class="cd-card rqa-deck">
    {#each groups as group, index (group.id)}
      {@const isOpen = shown === index}
      <div class="rqa-shelf" data-open={isOpen ? '' : undefined}>
        <button
          class="rqa-bar"
          type="button"
          aria-expanded={group.rows.length > 0 ? isOpen : undefined}
          aria-controls={isOpen ? `rqa-rows-${index}` : undefined}
          disabled={group.rows.length === 0}
          aria-label={`${group.label}, ${group.rows.length} ${group.rows.length === 1 ? 'record' : 'records'}`}
          onclick={() => toggle(index)}
        >
          <span class="rqa-bead num" class:rqa-bead--zero={group.rows.length === 0}>{group.rows.length}</span>
          <span class="rqa-bar__l">{group.label}</span>
          <span class="rqa-fold" aria-hidden="true">
            <svg width="16" height="16" viewBox="0 0 24 24"><path d="M6.5 9.5 12 15.5 17.5 9.5" /></svg>
          </span>
        </button>

        {#if isOpen}
          <div class="rqa-rows" id={`rqa-rows-${index}`}>
            {#each group.rows as row (row.id)}
              <div class="rqa-row">
                <span class="rqa-row__t">{row.title}</span>
                {#if row.course}
                  <span class="cd-chip cd-chip--code num" data-w={row.wash}>{row.course}</span>
                {/if}
                <span class="cd-chip {stateClass(row.tone)} num">{row.state}</span>
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  aria-label={`Grade ${row.title} — put it on the card`}
                  onclick={() => onPut(row.id)}
                >
                  Grade
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  {#if moved.graded.length > 0}
    <section class="rqa-pocket" aria-label="Graded today">
      <p class="rqa-pocket__k">
        Graded today · <b class="num">{moved.graded.length}</b>
      </p>
      {#each moved.graded as row (row.id)}
        <div class="rqa-prow">
          <span class="rqa-prow__t">{row.title}</span>
          <span class="rqa-prow__d num">{row.say}</span>
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            aria-label={`Undo the grade on ${row.title}`}
            onclick={() => void onUndo()}
          >
            Undo
          </button>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  /* ── the deck: an accordion of four shelves, and nothing else ─────────── */
  .rqa-deck {
    display: flex;
    flex-direction: column;
    gap: calc(4px * var(--ui-s));
    padding: var(--ui-pad-sm);
    width: min(760px, 100%);
  }
  .rqa-shelf {
    display: flex;
    flex-direction: column;
    height: calc(52px * var(--ui-s));
    min-height: 0;
    border-radius: var(--r-item);
    overflow: clip;
    transition: height var(--dur-3) var(--ease), background var(--dur-2) var(--ease);
  }
  .rqa-shelf[data-open] {
    height: var(--rqa-open, calc(324px * var(--ui-s)));
    background: var(--well);
  }
  .rqa-bar {
    display: grid;
    grid-template-columns: calc(26px * var(--ui-s)) minmax(0, 1fr) calc(24px * var(--ui-s));
    align-items: center;
    gap: var(--ui-gap);
    flex: none;
    width: 100%;
    height: calc(52px * var(--ui-s));
    padding: 0 var(--ui-pad-sm);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .rqa-bar:hover {
    background: var(--well);
  }
  .rqa-bar:active {
    background: var(--well-2);
  }
  .rqa-shelf[data-open] .rqa-bar:hover {
    background: var(--well-2);
  }
  /* The bar bleeds into the shelf's clipped corner, so the ring is inset. */
  .rqa-bar:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* A window with nothing in it cannot open: its own state, not a hidden one. */
  .rqa-bar[disabled] {
    cursor: default;
  }
  .rqa-bar[disabled]:hover,
  .rqa-bar[disabled]:active {
    background: none;
  }
  .rqa-bar[disabled] .rqa-bar__l,
  .rqa-bar[disabled] .rqa-bead {
    color: var(--ink-3);
  }
  .rqa-bar[disabled] .rqa-fold {
    opacity: 0.35;
  }

  /* The bead: the shelf's count as an object rather than a word. */
  .rqa-bead {
    display: grid;
    place-items: center;
    width: calc(26px * var(--ui-s));
    height: calc(26px * var(--ui-s));
    border-radius: var(--r-pill);
    background: var(--well-2);
    color: var(--ink);
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  .rqa-shelf[data-open] .rqa-bead {
    background: var(--card);
    box-shadow: var(--sh-1);
  }
  .rqa-bead--zero {
    background: var(--well);
    color: var(--ink-3);
    box-shadow: none;
  }
  .rqa-bar__l {
    font-size: var(--ui-text);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rqa-fold {
    display: grid;
    place-items: center;
    width: calc(24px * var(--ui-s));
    height: calc(24px * var(--ui-s));
    color: var(--ink-3);
    transition: transform var(--dur-2) var(--ease), color var(--dur-1) var(--ease);
  }
  .rqa-fold path {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .rqa-bar:hover .rqa-fold {
    color: var(--ink);
  }
  .rqa-shelf[data-open] .rqa-fold {
    transform: rotate(180deg);
    color: var(--ink);
  }

  /* The rows: the open shelf's own scroller. `overflow-anchor: none`, because
     the browser's own anchoring would otherwise adjust the position behind our
     back while the list re-renders whole. */
  .rqa-rows {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-anchor: none;
  }
  @keyframes rqa-in {
    from {
      opacity: 0;
      translate: 0 calc(6px * var(--ui-s));
    }
    to {
      opacity: 1;
      translate: 0 0;
    }
  }
  /* A shelf that has just opened: its records arrive after the fold has moved,
     the way the reference's panel content does. */
  .rqa-rows > .rqa-row {
    animation: rqa-in var(--dur-2) var(--ease) both;
  }
  .rqa-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(60px * var(--ui-s));
    padding: 0 var(--ui-pad-sm);
    transition: background var(--dur-1) var(--ease);
  }
  .rqa-row + .rqa-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .rqa-row:hover {
    background: var(--well-2);
  }
  .rqa-row__t {
    font-size: var(--ui-text);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The pocket: where a graded record goes, with its own way back. */
  .rqa-pocket {
    display: grid;
    gap: calc(4px * var(--ui-s));
    width: min(760px, 100%);
    margin-top: var(--ui-gap);
    padding: var(--ui-pad-sm) var(--ui-pad);
    border-radius: var(--r-card);
    background: var(--well);
  }
  .rqa-pocket__k {
    margin: 0;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .rqa-prow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(44px * var(--ui-s));
  }
  .rqa-prow + .rqa-prow {
    border-top: 1px dashed var(--rule-strong);
  }
  .rqa-prow__t {
    font-size: var(--ui-text);
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rqa-prow__d {
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
    white-space: nowrap;
  }

  /* ── narrow: the row's chips keep their place and the title yields; the
     shelf heights are already the register's. ────────────────────────── */
  @container (max-width: 560px) {
    .rqa-row {
      grid-template-columns: minmax(0, 1fr) auto;
      row-gap: calc(4px * var(--ui-s));
      padding: calc(8px * var(--ui-s)) var(--ui-pad-sm);
    }
    .rqa-row__t {
      grid-column: 1 / -1;
      white-space: normal;
    }
    .rqa-prow {
      grid-template-columns: minmax(0, 1fr) auto;
      row-gap: calc(4px * var(--ui-s));
    }
  }
</style>
