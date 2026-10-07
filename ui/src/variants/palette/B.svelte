<!--
  PALETTE · B · THE SHEET.
  Sticker-sheet style command palette with die-cut object silhouettes
  and category-specific wash assignments.
-->
<script lang="ts">
  import { groupsOf, type PaletteProps } from './props';

  let { query, rows, status, error, onQuery, onCommit, onLeave, onDismiss }: PaletteProps = $props();

  const runs = $derived(groupsOf(rows));
  const flat = $derived(runs.flatMap((run) => run.rows));

  let sel = $state(0);
  let peeling = $state<number | null>(null);
  let leaving = $state(false);
  let busy = $state(false);
  let field = $state<HTMLInputElement | null>(null);

  const ANGLES = ['-4.5deg', '2.6deg', '-1.8deg', '5.2deg', '-5.6deg', '1.4deg', '3.8deg', '-2.8deg', '4.6deg', '-1.2deg'];
  const NUDGE = ['-7px', '4px', '1px', '-3px', '6px', '-5px', '2px', '-1px'];

  $effect(() => {
    void query;
    sel = 0;
  });

  $effect(() => {
    if (sel > rows.length - 1) sel = Math.max(rows.length - 1, 0);
  });

  $effect(() => {
    if (!field) return;
    field.focus({ preventScroll: true });
    const end = field.value.length;
    field.setSelectionRange(end, end);
  });

  const live = $derived(rows[sel] ?? null);
  const verdict = $derived(live?.group === 'capture' ? 'Add it' : live?.group === 'actions' ? 'Run it' : 'Open it');
  const line = $derived(error ? { word: error, alert: true } : status ? { word: status, alert: false } : null);

  const reduced = (): boolean =>
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Animation timings for item peel and dismissal transitions.
  function beats(): { peel: number; leave: number } {
    return reduced() ? { peel: 0, leave: 0 } : { peel: 140, leave: 140 };
  }

  function clamp(at: number): void {
    sel = Math.min(Math.max(at, 0), Math.max(rows.length - 1, 0));
  }

  async function commit(at: number): Promise<void> {
    const row = rows[at];
    if (!row || busy) return;
    sel = at;
    busy = true;
    const leave = await onCommit(row);
    if (!leave) {
      busy = false;
      return;
    }
    // Animate item peel before closing palette.
    const beat = beats();
    peeling = at;
    window.setTimeout(() => {
      leaving = true;
      window.setTimeout(() => {
        peeling = null;
        onLeave();
        busy = false;
      }, beat.leave);
    }, beat.peel);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      onDismiss();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      clamp(sel + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      clamp(sel - 1);
    } else if (event.key === 'Enter' && event.target === field) {
      event.preventDefault();
      void commit(sel);
    }
  }
</script>

<div
  class="cd-palette pa-sh"
  data-leaving={leaving ? '1' : '0'}
  role="dialog"
  aria-modal="true"
  aria-label="Add, find or run something"
  tabindex="-1"
  onkeydown={onkeydown}
>
  <div class="pa-sh__head">
    <input
      class="pa-sh__field"
      type="text"
      role="combobox"
      aria-expanded="true"
      aria-controls="sam-palette-results"
      aria-autocomplete="list"
      aria-activedescendant={live ? `pa-b-${sel}` : undefined}
      aria-label="Add a title, or type to find something"
      placeholder="Add a title, or type to find something"
      autocomplete="off"
      bind:this={field}
      value={query}
      oninput={(event) => onQuery(event.currentTarget.value)}
    />
    <!-- The one affordance this design owns: it says what the live sticker does. -->
    <button class="cd-pill cd-pill--sm pa-sh__commit" type="button" onclick={() => void commit(sel)}>
      {verdict}
    </button>
  </div>

  <!-- The card's own state row, and the only foot this design has: the state
       word keeps the line it had, and the surface's design switch takes the
       row's right end. -->
  <div class="pa-sh__foot">
    {#if line}
      <p class="pa-sh__state" data-alert={line.alert ? '' : undefined} role={line.alert ? 'alert' : 'status'}>
        {line.word}
      </p>
    {/if}
  </div>

  <div class="pa-sh__sheet" id="sam-palette-results" role="listbox" aria-label="Results">
    {#each flat as entry (entry.row.key)}
      <button
        class="pa-st"
        type="button"
        role="option"
        id={`pa-b-${entry.at}`}
        aria-selected={entry.at === sel}
        aria-disabled={entry.row.blocked}
        data-w={entry.row.wash}
        data-shape={entry.row.shape}
        data-live={entry.at === sel ? '' : undefined}
        data-peel={peeling === entry.at ? '' : undefined}
        data-command={entry.row.command}
        data-placement={entry.row.placement}
        data-kind={entry.row.kind ?? undefined}
        data-object={entry.row.noun ?? undefined}
        data-target={entry.row.target ?? undefined}
        style={`--rot: ${ANGLES[entry.at % ANGLES.length]}; --dy: ${NUDGE[entry.at % NUDGE.length]}`}
        disabled={entry.row.blocked}
        onclick={() => void commit(entry.at)}
        onmouseenter={() => (sel = entry.at)}
      >
        <span class="pa-st__shadow" aria-hidden="true"></span>
        <span class="pa-st__die">
          <span class="pa-st__face">
            <span class="pa-st__name">{entry.row.label}</span>
            {#if entry.row.group === 'capture' && entry.row.note}
              <span class="pa-st__sub" data-developer>{entry.row.note}</span>
            {:else if entry.row.group !== 'actions'}
              <span class="pa-st__sub">{entry.row.word}</span>
            {/if}
          </span>
          <span class="pa-st__curl" aria-hidden="true"></span>
        </span>
      </button>
    {/each}

    {#if rows.length === 0}
      <p class="pa-sh__none">Nothing here matches “{query.trim()}”</p>
    {/if}
  </div>
</div>

<style>
  /* Sticker-sheet container layout and exit transition. */
  .pa-sh {
    container-type: inline-size;
    --pa-leave: cubic-bezier(0.42, 0, 1, 1);
    width: 100%;
    height: 100%;
    min-height: 0;
    padding: var(--ui-pad);
    border-radius: var(--r-card);
    background: var(--backdrop);
    box-shadow: var(--sh-pop);
    animation: none;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    gap: var(--ui-gap-sm);
    transition:
      opacity var(--dur-2) var(--ease),
      translate var(--dur-2) var(--ease),
      rotate var(--dur-2) var(--ease);
  }
  .pa-sh[data-leaving='1'] {
    opacity: 0;
    translate: 0 -30px;
    rotate: -1.4deg;
    transition-duration: var(--dur-1);
    transition-timing-function: var(--pa-leave);
  }

  .pa-sh__head {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
  }
  .pa-sh__field {
    flex: 1;
    min-width: 0;
    height: calc(var(--pill-h) * var(--ui-s));
    padding: 0 var(--ui-pad);
    border: 0;
    border-radius: var(--r-pill);
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--sh-1);
    font-family: var(--font-display);
    font-size: calc(var(--text-sm) * var(--ui-s));
    letter-spacing: var(--track-body);
  }
  .pa-sh__field::placeholder {
    color: var(--ink-3);
  }
  .pa-sh__commit {
    flex: none;
  }
  .pa-sh__field:focus-visible,
  .pa-st:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  /* The app's own machine state, above the sheet and out of the paste-up's way. */
  .pa-sh__state {
    color: var(--ink-2);
    font-size: calc(var(--text-sm) * var(--ui-s));
  }
  .pa-sh__state[data-alert] {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  /* The state word's own row, now a row: it was the card's one line for the
     machine's state, so it is the row the surface's design switch stands in.
     The card's three rows are unchanged — head · this · the sheet. */
  .pa-sh__foot {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
  }

  .pa-sh__sheet {
    display: flex;
    flex-wrap: wrap;
    align-content: space-between;
    gap: var(--ui-gap) var(--ui-gap-sm);
    padding: var(--ui-pad-sm) var(--ui-gap-sm);
    overflow-y: auto;
  }

  /* ── the die-cut object ─────────────────────────────────────────────────
     Two silhouettes per shape and the inner one derives from the die margin: a
     face cut `--die` inside its backing loses that much radius (14 → 11) and
     `die·√2` of run on every 45° chamfer, or the sticker would read as a
     different thickness at each edge. */
  .pa-st {
    --die: 3px;
    --chamf: 16px;
    --chamf-in: calc(var(--chamf) - var(--die) * 1.4142);
    position: relative;
    display: inline-grid;
    rotate: var(--rot, 0deg);
    translate: 0 var(--dy, 0px);
    transition:
      rotate var(--dur-2) var(--ease),
      translate var(--dur-2) var(--ease);
  }
  .pa-st[data-w='mint'] {
    --w: var(--w-mint);
    --on: var(--fg-mint);
  }
  .pa-st[data-w='lilac'] {
    --w: var(--w-lilac);
    --on: var(--fg-lilac);
  }
  .pa-st[data-w='butter'] {
    --w: var(--w-butter);
    --on: var(--fg-butter);
  }
  .pa-st[data-w='sky'] {
    --w: var(--w-sky);
    --on: var(--fg-sky);
  }
  .pa-st[data-w='none'] {
    --w: var(--card);
    --on: var(--ink);
  }
  .pa-st[data-shape='slab'] {
    --shape: inset(0 round var(--r-item));
    --shape-in: inset(0 round calc(var(--r-item) - var(--die)));
  }
  .pa-st[data-shape='tag'] {
    --shape: polygon(0 0, calc(100% - var(--chamf)) 0, 100% var(--chamf), 100% 100%, 0 100%);
    --shape-in: polygon(0 0, calc(100% - var(--chamf-in)) 0, 100% var(--chamf-in), 100% 100%, 0 100%);
  }
  .pa-st[data-shape='ticket'] {
    --shape: polygon(0 0, 100% 0, 100% 100%, var(--chamf) 100%, 0 calc(100% - var(--chamf)));
    --shape-in: polygon(0 0, 100% 0, 100% 100%, var(--chamf-in) 100%, 0 calc(100% - var(--chamf-in)));
  }
  .pa-st[data-shape='notch'] {
    --chamf: 18px;
    --shape: polygon(0 0, calc(100% - var(--chamf)) 0, 100% var(--chamf), 100% 100%, var(--chamf) 100%, 0 calc(100% - var(--chamf)));
    --shape-in: polygon(0 0, calc(100% - var(--chamf-in)) 0, 100% var(--chamf-in), 100% 100%, var(--chamf-in) 100%, 0 calc(100% - var(--chamf-in)));
  }
  .pa-st__shadow {
    position: absolute;
    inset: 0;
    clip-path: var(--shape);
    background: color-mix(in oklab, var(--ink) 15%, transparent);
    translate: 2px 3px;
    transition: translate var(--dur-2) var(--ease);
  }
  .pa-st__die {
    position: relative;
    display: grid;
    padding: var(--die);
    clip-path: var(--shape);
    background: var(--card);
  }
  .pa-st__face {
    display: grid;
    gap: 2px;
    align-content: center;
    min-height: var(--ui-row-h);
    padding: var(--ui-gap-sm) var(--ui-pad);
    clip-path: var(--shape-in);
    background: var(--w);
    color: var(--on);
  }
  .pa-st__name {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-body);
  }
  .pa-st__sub {
    font-size: calc(var(--text-2xs) * var(--ui-s));
  }
  /* The lift is the object's own: 4px up, its offset copy deepening under it. */
  .pa-st:hover,
  .pa-st[data-live] {
    translate: 0 -4px;
  }
  .pa-st:hover .pa-st__shadow,
  .pa-st[data-live] .pa-st__shadow {
    translate: 3px 7px;
  }
  .pa-st[aria-disabled='true'] {
    opacity: 0.55;
  }

  /* The peel: the object leaves its place on the page's leaving curve, and the
     sheet follows it. Both are `--dur-1` — 140 + 140 is the whole departure. */
  .pa-st[data-peel] {
    translate: 30px -44px;
    rotate: calc(var(--rot, 0deg) + 7deg);
    transition-duration: var(--dur-1);
    transition-timing-function: var(--pa-leave);
  }
  /* A lifted corner: the sheet's own white over its own ground, and the crease
     drawn as a hairline — no third material is introduced. */
  .pa-st__curl {
    position: absolute;
    right: 0;
    bottom: 0;
    width: calc(26px * var(--ui-s));
    height: calc(26px * var(--ui-s));
    opacity: 0;
    clip-path: polygon(100% 0, 100% 100%, 0 100%);
    background: linear-gradient(135deg, var(--card) 0 52%, var(--sheet) 52% 100%);
    transition: opacity var(--dur-1) var(--ease);
  }
  .pa-st__curl::after {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    width: calc(26px * var(--ui-s));
    height: 1.5px;
    background: var(--rule-strong);
    transform-origin: 0 0;
    rotate: 45deg;
  }
  .pa-st[data-peel] .pa-st__curl {
    opacity: 1;
  }

  .pa-sh__none {
    flex-basis: 100%;
    padding: var(--ui-pad);
    color: var(--ink-2);
    font-size: calc(var(--text-md) * var(--ui-s));
  }

  @media (prefers-reduced-motion: reduce) {
    .pa-sh,
    .pa-st,
    .pa-st__shadow,
    .pa-st__curl {
      transition: none;
    }
  }
</style>
