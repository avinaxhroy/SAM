<!--
  PALETTE · C · THE PLAIN.
  Minimal command palette variant. Uses transparent row backgrounds with well
  fills on hover and selection, explicit focus outlines, and a reserved status
  row with subtle fade transitions.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import { groupsOf, type PaletteProps } from './props';

  let { query, rows, status, error, onQuery, onCommit, onLeave, onDismiss }: PaletteProps = $props();

  const runs = $derived(groupsOf(rows));

  let sel = $state(0);
  let shown = $state('');
  let on = $state(false);
  let leaving = $state(true);
  let busy = $state(false);
  let field = $state<HTMLInputElement | null>(null);
  let sayTimer: number | undefined;

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

  // Delay removing leaving state until the next frame for the entrance transition.
  $effect(() => {
    if (!leaving) return;
    if (reduced()) {
      leaving = false;
      return;
    }
    const id = requestAnimationFrame(() => requestAnimationFrame(() => (leaving = false)));
    return () => cancelAnimationFrame(id);
  });

  const reduced = (): boolean =>
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Cross-fade status messages.
  function say(next: string): void {
    if (sayTimer !== undefined) {
      clearTimeout(sayTimer);
      sayTimer = undefined;
    }
    if (reduced()) {
      shown = next;
      on = next.length > 0;
      return;
    }
    if (shown === next) return;
    if (shown === '') {
      shown = next;
      on = next.length > 0;
      return;
    }
    if (next === '') {
      on = false;
      sayTimer = window.setTimeout(() => {
        shown = '';
        sayTimer = undefined;
      }, 140);
      return;
    }
    on = false;
    sayTimer = window.setTimeout(() => {
      shown = next;
      on = true;
      sayTimer = undefined;
    }, 140);
  }

  $effect(() => {
    const next = error ?? status ?? '';
    untrack(() => say(next));
  });

  function clamp(at: number): void {
    sel = Math.min(Math.max(at, 0), Math.max(rows.length - 1, 0));
  }

  // Animate palette dismissal before invoking callback.
  function leave(after: () => void): void {
    leaving = true;
    if (reduced()) {
      after();
      return;
    }
    window.setTimeout(after, 140);
  }

  async function commit(at: number): Promise<void> {
    const row = rows[at];
    if (!row || busy) return;
    sel = at;
    busy = true;
    const leavePalette = await onCommit(row);
    if (!leavePalette) {
      busy = false;
      return;
    }
    leave(() => {
      onLeave();
      busy = false;
    });
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      leave(onDismiss);
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      clamp(sel + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      clamp(sel - 1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      clamp(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      clamp(rows.length - 1);
    } else if (event.key === 'Enter' && event.target === field) {
      event.preventDefault();
      void commit(sel);
    }
  }
</script>

<div
  class="cd-palette pa-pl"
  data-leaving={leaving ? '1' : '0'}
  role="dialog"
  aria-modal="true"
  aria-label="Add, find or run something"
  tabindex="-1"
  onkeydown={onkeydown}
>
  <div class="cd-palette__field">
    <input
      class="pa-pl__field"
      type="text"
      role="combobox"
      aria-expanded="true"
      aria-controls="sam-palette-results"
      aria-autocomplete="list"
      aria-activedescendant={rows[sel] ? `pa-c-${sel}` : undefined}
      aria-label="Add a title, or type to find something"
      placeholder="Add a title, or type to find something"
      autocomplete="off"
      bind:this={field}
      value={query}
      oninput={(event) => onQuery(event.currentTarget.value)}
    />
  </div>

  <ul class="cd-palette__list" id="sam-palette-results" role="listbox" aria-label="Results">
    {#each runs as run (run.group)}
      <li class="cd-palette__group pa-pl__mark" role="presentation">{run.name}</li>
      {#each run.rows as entry (entry.row.key)}
        <li role="presentation">
          <button
            class="cd-palette__item pa-pl__row"
            type="button"
            role="option"
            id={`pa-c-${entry.at}`}
            aria-selected={entry.at === sel}
            aria-disabled={entry.row.blocked}
            data-group={entry.row.group}
            data-sel={entry.at === sel ? '' : undefined}
            data-command={entry.row.command}
            data-placement={entry.row.placement}
            data-kind={entry.row.kind ?? undefined}
            data-object={entry.row.noun ?? undefined}
            data-target={entry.row.target ?? undefined}
            disabled={entry.row.blocked}
            onclick={() => void commit(entry.at)}
            onmouseenter={() => (sel = entry.at)}
          >
            <span class="cd-palette__label pa-pl__label">{entry.row.label}</span>
            <span class="pa-pl__word">{entry.row.word}</span>
          </button>
        </li>
      {/each}
    {/each}

    {#if rows.length === 0}
      <li class="cd-palette__empty">Nothing here matches “{query.trim()}”</li>
    {/if}
  </ul>

  <div class="cd-palette__foot">
    <span class="pa-pl__state" data-on={on ? '1' : '0'} role="status">{shown}</span>
  </div>
</div>

<style>
  /* Base card layout and exit transition. */
  .pa-pl {
    container-type: inline-size;
    --pa-leave: cubic-bezier(0.42, 0, 1, 1);
    width: min(600px, 100%);
    border-radius: var(--r-card);
    background: var(--card);
    box-shadow: var(--sh-pop);
    overflow: hidden;
    animation: none;
    transition:
      opacity var(--dur-2) var(--ease),
      translate var(--dur-2) var(--ease);
  }
  .pa-pl[data-leaving='1'] {
    opacity: 0;
    translate: 0 -10px;
    transition-duration: var(--dur-1);
    transition-timing-function: var(--pa-leave);
  }

  /* Search input field. */
  .cd-palette__field {
    height: calc(58px * var(--ui-s));
    padding: 0 var(--ui-pad);
  }
  .pa-pl__field {
    align-self: stretch;
    height: auto;
    padding: 0;
    /* The field is the card's own head: it stretches to the card's top edge, so
       its box reaches the top corners and it wears the corner it sits in
       (`--r-card` less the field's own inline padding). A focused field's ring —
       drawn 2px inside, which shrinks its radius by the same 2px — then runs
       concentric with the card's clip instead of being squared off under it. */
    border-radius: calc(var(--r-card) - var(--ui-pad)) calc(var(--r-card) - var(--ui-pad)) 0 0;
    font-size: calc(var(--text-md) * var(--ui-s));
    letter-spacing: var(--track-body);
  }
  .pa-pl .cd-palette__field .pa-pl__field:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  .cd-palette__list {
    padding: var(--ui-gap-sm);
  }

  /* Group section headers. */
  .pa-pl__mark {
    padding: var(--ui-gap-sm) var(--ui-pad) var(--space-2xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .pa-pl__mark:first-child {
    padding-top: var(--space-2xs);
  }

  /* Palette rows and states. */
  .cd-palette__item {
    min-height: calc(var(--pill-h) * var(--ui-s));
    padding: var(--ui-gap-sm) var(--ui-pad);
    background: transparent;
  }
  .cd-palette__item[data-sel],
  .cd-palette__item:hover {
    background: var(--well);
  }
  .cd-palette__item:active {
    background: var(--well-2);
  }
  .cd-palette__item:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .pa-pl__label {
    letter-spacing: var(--track-body);
  }
  .pa-pl__row[data-group='capture'] .pa-pl__label {
    font-weight: var(--weight-label);
  }
  /* Consequence tag for palette actions. */
  .pa-pl__word {
    flex: none;
    font-size: calc(var(--text-2xs) * var(--ui-s));
    letter-spacing: var(--track-body);
    text-transform: none;
    color: var(--ink-2);
  }

  /* Reserved status row to prevent layout shifts. */
  .cd-palette__foot {
    min-height: calc(var(--hit) * var(--ui-s));
    padding: var(--ui-gap-sm) var(--ui-pad);
    border-top: 0;
    font-size: calc(var(--text-2xs) * var(--ui-s));
  }
  .pa-pl__state {
    flex: none;
    color: var(--ink-2);
    opacity: 0;
    transition: opacity var(--dur-1) var(--ease);
  }
  .pa-pl__state[data-on='1'] {
    opacity: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    .pa-pl,
    .pa-pl__state {
      transition: none;
    }
  }
</style>
