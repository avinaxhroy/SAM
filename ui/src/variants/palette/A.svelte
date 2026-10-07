<!--
  PALETTE · A · THE INDEX.
  Letterpress index style command palette. Displays matching commands and
  navigation targets in a single column with a highlight plate positioned
  relative to the active row offset (`--y`).
-->
<script lang="ts">
  import { groupsOf, type PaletteProps, type PaletteRow } from './props';

  let { query, rows, status, error, onQuery, onCommit, onLeave, onDismiss }: PaletteProps = $props();

  const runs = $derived(groupsOf(rows));

  let sel = $state(0);
  let busy = $state(false);
  let field = $state<HTMLInputElement | null>(null);
  let nodes = $state<HTMLElement[]>([]);
  // Avoid scrolling on first paint.
  let moved = false;
  let y = $state(0);

  $effect(() => {
    void query;
    sel = 0;
  });

  // Track vertical offset of the selected row.
  $effect(() => {
    void rows;
    y = nodes[sel]?.offsetTop ?? 0;
  });

  $effect(() => {
    if (!field) return;
    field.focus({ preventScroll: true });
    const end = field.value.length;
    field.setSelectionRange(end, end);
  });

  const live = $derived(rows[sel] ?? null);

  const face = $derived(
    live
      ? { key: live.plate?.key ?? 'The plan', value: live.plate?.value ?? live.label, note: live.note ?? '' }
      : { key: 'The plan', value: `${rows.length} ${rows.length === 1 ? 'name' : 'names'}`, note: '' },
  );

  const line = $derived(error ? { word: error, alert: true } : status ? { word: status, alert: false } : null);

  function clamp(at: number): void {
    sel = Math.min(Math.max(at, 0), Math.max(rows.length - 1, 0));
    if (moved && nodes[sel]) nodes[sel].scrollIntoView({ block: 'nearest' });
  }

  async function commit(at: number): Promise<void> {
    const row = rows[at];
    if (!row || busy) return;
    busy = true;
    sel = at;
    const leave = await onCommit(row);
    if (leave) onLeave();
    busy = false;
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      onDismiss();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      moved = true;
      clamp(sel + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      moved = true;
      clamp(sel - 1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      moved = true;
      clamp(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      moved = true;
      clamp(rows.length - 1);
    } else if (event.key === 'Enter' && event.target === field) {
      event.preventDefault();
      void commit(sel);
    }
  }
</script>

<div
  class="cd-palette pa-ix"
  role="dialog"
  aria-modal="true"
  aria-label="Add, find or run something"
  tabindex="-1"
  onkeydown={onkeydown}
>
  <div class="pa-ix__margin" aria-hidden="true">
    <div class="pa-ix__plate" style={`--y: ${y}px`}>
      <span class="pa-ix__plate-k">{face.key}</span>
      <span class="pa-ix__plate-rule"></span>
      <span class="pa-ix__plate-v">{face.value}</span>
      {#if face.note}
        <span class="pa-ix__plate-n" data-developer>{face.note}</span>
      {/if}
    </div>
  </div>

  <div class="pa-ix__names">
    <input
      class="pa-ix__field"
      type="text"
      role="combobox"
      aria-expanded="true"
      aria-controls="sam-palette-results"
      aria-autocomplete="list"
      aria-activedescendant={live ? `pa-a-${sel}` : undefined}
      aria-label="Add a title, or type to find something"
      placeholder="Add a title, or type to find something"
      autocomplete="off"
      bind:this={field}
      value={query}
      oninput={(event) => onQuery(event.currentTarget.value)}
    />

    <div class="pa-ix__list" id="sam-palette-results" role="listbox" aria-label="Results">
      {#each runs as run (run.group)}
        <span class="pa-ix__mark" aria-hidden="true">{run.name}</span>
        {#each run.rows as entry (entry.row.key)}
          <button
            class="pa-ix__row"
            type="button"
            role="option"
            id={`pa-a-${entry.at}`}
            aria-selected={entry.at === sel}
            aria-disabled={entry.row.blocked}
            data-live={entry.at === sel ? '' : undefined}
            data-command={entry.row.command}
            data-placement={entry.row.placement}
            data-kind={entry.row.kind ?? undefined}
            data-object={entry.row.noun ?? undefined}
            data-target={entry.row.target ?? undefined}
            disabled={entry.row.blocked}
            bind:this={nodes[entry.at]}
            onclick={() => void commit(entry.at)}
            onmouseenter={() => (sel = entry.at)}
          >{entry.row.label}</button>
        {/each}
      {/each}

      {#if rows.length === 0}
        <p class="pa-ix__none">Nothing here matches “{query.trim()}”</p>
      {/if}
    </div>

    <!-- The hint line's own row, and the only foot A has. The state word is the
         design's own line for the machine's state, so it is the row the switch
         stands in — and it stands here, outside `role="listbox"`, because a
         capsule is not a result. -->
    <div class="pa-ix__foot">
      {#if line}
        <p class="pa-ix__state" data-alert={line.alert ? '' : undefined} role={line.alert ? 'alert' : 'status'}>
          {line.word}
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  /* Letterpress index container with two-column layout. */
  .pa-ix {
    container-type: inline-size;
    width: min(920px, 100%);
    height: 100%;
    padding: calc(var(--space-2xl) * var(--ui-s)) calc(var(--space-2xl) * var(--ui-s))
      calc(var(--space-3xl) * var(--ui-s));
    border-radius: var(--r-card);
    background: var(--card);
    box-shadow: var(--sh-pop);
    animation: none;
    overflow-y: auto;
    display: grid;
    grid-template-columns: calc(196px * var(--ui-s)) minmax(0, 1fr);
    gap: calc(var(--space-2xl) * var(--ui-s));
    align-items: stretch;
  }

  /* Left margin band holding the highlight plate. */
  .pa-ix__margin {
    position: relative;
    background: var(--well-2);
    border-radius: var(--r-tile);
  }
  .pa-ix__plate {
    position: absolute;
    left: var(--ui-gap-sm);
    right: var(--ui-gap-sm);
    top: 0;
    padding: var(--ui-pad) var(--ui-pad) var(--ui-pad-sm);
    border-radius: var(--r-mini);
    background: var(--ink);
    color: var(--ink-inv);
    box-shadow: var(--sh-ink);
    display: grid;
    gap: var(--space-3xs);
    translate: 0 var(--y, 0px);
    transition: translate var(--dur-2) var(--ease);
  }
  .pa-ix__plate-k {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
  }
  .pa-ix__plate-rule {
    height: 1px;
    background: var(--fill-on-ink);
    margin: var(--space-2xs) 0;
  }
  .pa-ix__plate-v {
    font-family: var(--font-display);
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    line-height: 1.24;
    overflow-wrap: anywhere;
  }
  .pa-ix__plate-n {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    overflow-wrap: anywhere;
  }

  .pa-ix__names {
    position: relative;
    display: grid;
  }
  /* The field fills the row it sits in — the shipped box was 26.6px, under the
     app's own 32px floor. */
  .pa-ix__field {
    width: 100%;
    height: calc(var(--pill-h) * var(--ui-s));
    padding: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--ink);
    font-family: var(--font-display);
    font-size: calc(var(--text-lg) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
  }
  .pa-ix__field::placeholder {
    color: var(--ink-3);
    font-weight: var(--weight-body);
  }
  /* A row inside a scroll port is clipped by its own list: the ring is drawn
     inside the control. */
  .pa-ix__field:focus-visible,
  .pa-ix__row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .pa-ix__list {
    display: grid;
    justify-items: start;
    align-content: start;
  }
  .pa-ix__mark {
    margin: calc(var(--space-xl) * var(--ui-s)) 0 calc(var(--space-2xs) * var(--ui-s));
    font-size: calc(var(--text-2xs) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .pa-ix__list > .pa-ix__mark:first-child {
    margin-top: calc(var(--space-md) * var(--ui-s));
  }
  .pa-ix__row {
    min-height: calc(var(--hit) * var(--ui-s));
    padding: var(--space-2xs) var(--space-2xs) var(--space-2xs) 0;
    text-align: left;
    color: var(--ink-2);
    font-family: var(--font-display);
    font-size: calc(var(--text-2xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.1;
    transition:
      color var(--dur-2) var(--ease),
      translate var(--dur-2) var(--ease);
  }
  .pa-ix__row:hover,
  .pa-ix__row[data-live] {
    color: var(--ink);
    translate: var(--space-xs) 0;
  }
  .pa-ix__row[aria-disabled='true'] {
    opacity: 0.55;
  }
  .pa-ix__none {
    padding: calc(var(--space-lg) * var(--ui-s)) 0;
    color: var(--ink-2);
    font-size: calc(var(--text-md) * var(--ui-s));
  }
  /* The app's own machine state — the read in flight, the read that failed, a
     refused write — as the column's closing line. */
  .pa-ix__state {
    padding: var(--ui-pad-sm) 0 0;
    color: var(--ink-2);
    font-size: calc(var(--text-sm) * var(--ui-s));
  }
  .pa-ix__state[data-alert] {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  /* The foot row: the state word at its left, the surface's design switch at
     its right. It stands where the state line already stood — under the results
     — and it is always there, so the switch is too. */
  .pa-ix__foot {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    align-self: start;
  }

  /* The narrow branch: the band yields 100px so the names keep one line each,
     and the plate's type steps down with the names. The query is the card's own
     width, not the window's — the card is a fixed overlay and a media query
     could not see the room it was handed. */
  @container (max-width: 820px) {
    .pa-ix {
      grid-template-columns: calc(96px * var(--ui-s)) minmax(0, 1fr);
      padding: calc(var(--space-lg) * var(--ui-s));
      gap: calc(var(--space-lg) * var(--ui-s));
    }
    .pa-ix__margin {
      border-radius: var(--r-mini);
    }
    .pa-ix__plate {
      left: var(--space-2xs);
      right: var(--space-2xs);
      padding: var(--ui-gap-sm);
      border-radius: var(--r-mark);
    }
    .pa-ix__plate-v,
    .pa-ix__row {
      font-size: calc(var(--text-lg) * var(--ui-s));
    }
  }

  /* base.css zeroes durations, not transition-delay: the travelling plate is a
     transition of the page's own and is answered here. */
  @media (prefers-reduced-motion: reduce) {
    .pa-ix__plate,
    .pa-ix__row {
      transition: none;
    }
  }
</style>
