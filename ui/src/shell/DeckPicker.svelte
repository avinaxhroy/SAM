<!--
  The deck's pill listbox (UI_SPEC "model pill").

  Both pills in the deck's head — the model and the CLI — open this card. A
  native `<select>` can be neither searched nor capped, and it drops its popup
  *below* a deck that is anchored to the floor, where the window edge eats it:
  the card opens upward over the page instead, keeps its own 264px scroll, and
  answers Escape and outside presses on the capture phase so the deck never
  sees them.

  Rows arrive grouped. `first` and `last` sit outside the filter: they are the
  list's two ways out — *Default* ("whatever the CLI does") and *Custom…*
  ("anything it accepts") — so a query must not be able to hide them.

  The list is opened lazily (`onOpen`), so a pill whose rows need a network
  round-trip never makes the deck wait on it.
-->
<script module lang="ts">
  let counter = 0;
</script>

<script lang="ts">
  import Icon from './Icon.svelte';
  import type { PickGroup, PickRow } from './picker';

  let {
    icon = 'cube',
    label,
    value,
    groups = [],
    first = null,
    last = null,
    placeholder = 'Search…',
    listLabel = 'Choose',
    hint = '',
    disabled = false,
    onPick,
    onOpen,
  }: {
    /** The pill's glyph — the same one the closed pill has always worn. */
    icon?: string;
    /** What the closed pill shows (the parent owns how a choice reads). */
    label: string;
    /** The chosen row's id; unknown ids just leave the highlight at the top. */
    value: string;
    groups?: PickGroup[];
    first?: PickRow | null;
    last?: PickRow | null;
    placeholder?: string;
    /** The accessible name of the pill, its card and its list. */
    listLabel?: string;
    /** The status line under the list: what these rows are, or why they are few. */
    hint?: string;
    disabled?: boolean;
    onPick: (option: string) => void;
    onOpen?: () => void;
  } = $props();

  /** One instance, one set of DOM ids: two open cards cannot share them. */
  const listId = `cd-pick-list-${(counter += 1)}`;

  let open = $state(false);
  let query = $state('');
  let sel = $state(0);
  let field = $state<HTMLInputElement | null>(null);
  let nodes = $state<HTMLElement[]>([]);
  let moved = $state(false);
  /** The id set the highlight was last aligned against. */
  let aligned = '';

  /**
   * The card's one derivation: rows in source order, filtered by the query,
   * groups emptied by the filter dropped, provider marks left on the first
   * surviving row of each group — each row carrying its flat index, so the
   * arrows, the DOM ids and `aria-activedescendant` cannot disagree.
   */
  const view = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const keep = (row: PickRow): boolean =>
      needle.length === 0 ||
      row.id.toLowerCase().includes(needle) ||
      row.label.toLowerCase().includes(needle) ||
      (row.note ? row.note.toLowerCase().includes(needle) : false);

    const kept = groups
      .map((group) => ({ name: group.name, rows: group.rows.filter(keep) }))
      .filter((group) => group.rows.length > 0);
    const marks = kept.length > 1 || (kept[0]?.name.length ?? 0) > 0;

    const flat: Array<PickRow & { mark: string }> = [];
    if (first) flat.push({ ...first, mark: '' });
    for (const group of kept) {
      group.rows.forEach((row, at) => {
        flat.push({ ...row, mark: marks && at === 0 ? group.name : '' });
      });
    }
    if (last) flat.push({ ...last, mark: '' });

    return { flat, ids: flat.map((row) => row.id), empty: kept.length === 0 };
  });

  // Focus the search field the moment the card opens.
  $effect(() => {
    if (open) field?.focus();
  });

  /**
   * The rows can land after the card opens (`agent.models` is a network round
   * trip): an index taken against the empty list would point at a different row
   * once they arrive, and Enter would commit a choice the student never
   * highlighted — and persist it. Until the student moves the highlight, it
   * follows the chosen *value*, wherever that value lands in the new rows.
   */
  $effect(() => {
    const ids = view.ids;
    const key = ids.join('\u0000');
    if (key === aligned) return;
    aligned = key;
    if (!moved) {
      const at = ids.indexOf(value);
      sel = at >= 0 ? at : 0;
    } else if (sel > ids.length - 1) {
      sel = Math.max(ids.length - 1, 0);
    }
  });

  // The highlight is scrolled into view when the card opens on it: with 500
  // rows the current choice can sit far below the fold.
  $effect(() => {
    if (!open || moved) return;
    const node = nodes[sel];
    if (node) requestAnimationFrame(() => node.scrollIntoView({ block: 'nearest' }));
  });

  // A nested surface answers Escape before the deck: capture phase, stop
  // propagation — the record-doors pattern (variants/record-doors/A.svelte).
  function escapeCapture(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !open) return;
    event.stopPropagation();
    open = false;
  }

  function outsideCapture(event: PointerEvent): void {
    if (!open) return;
    if ((event.target as Element | null)?.closest?.('.cd-pick')) return;
    open = false;
  }

  $effect(() => {
    globalThis.addEventListener('keydown', escapeCapture, true);
    globalThis.addEventListener('pointerdown', outsideCapture, true);
    return () => {
      globalThis.removeEventListener('keydown', escapeCapture, true);
      globalThis.removeEventListener('pointerdown', outsideCapture, true);
    };
  });

  function toggle(): void {
    if (disabled) return;
    open = !open;
    if (open) {
      query = '';
      // Open on the current choice: the list is scrolled where the student left.
      const at = view.ids.indexOf(value);
      sel = at >= 0 ? at : 0;
      moved = false;
      onOpen?.();
    }
  }

  function clamp(at: number): void {
    sel = Math.min(Math.max(at, 0), Math.max(view.ids.length - 1, 0));
    if (moved && nodes[sel]) nodes[sel].scrollIntoView({ block: 'nearest' });
  }

  function commit(option: string): void {
    open = false;
    onPick(option);
  }

  function onkeydown(event: KeyboardEvent): void {
    // The card is a nested surface: the deck's own handlers (Enter submits)
    // must not fire while its search field has the keys.
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      event.stopPropagation();
      moved = true;
      clamp(sel + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      moved = true;
      clamp(sel - 1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      event.stopPropagation();
      moved = true;
      clamp(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      event.stopPropagation();
      moved = true;
      clamp(view.ids.length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      const option = view.ids[sel];
      if (option) commit(option);
    }
  }
</script>

<span class="cd-deck__pill cd-pick" class:is-disabled={disabled}>
  <Icon name={icon} size={13} />
  <button
    class="cd-pick__b"
    type="button"
    disabled={disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={listLabel}
    onclick={toggle}
  >
    <span class="cd-pick__value" title={value}>{label}</span>
    <Icon name="chevronDown" size={12} />
  </button>

  {#if open}
    <div class="cd-pick__card">
      <input
        class="cd-pick__q"
        type="search"
        role="combobox"
        aria-expanded="true"
        aria-controls={listId}
        aria-autocomplete="list"
        aria-activedescendant={`${listId}-${sel}`}
        aria-label={listLabel}
        {placeholder}
        autocomplete="off"
        spellcheck="false"
        bind:this={field}
        bind:value={query}
        oninput={() => {
          // Typing is a move: the highlight stays where the student put it
          // (the top of the new rows) instead of chasing the stored value.
          sel = 0;
          moved = true;
        }}
        onkeydown={onkeydown}
      />
      <div class="cd-pick__list" id={listId} role="listbox" aria-label={listLabel}>
        {#each view.flat as row, at (row.id)}
          {#if row.mark}
            <span class="cd-pick__mark" aria-hidden="true">{row.mark}</span>
          {/if}
          <button
            class="cd-pick__row"
            type="button"
            role="option"
            id={`${listId}-${at}`}
            aria-selected={sel === at}
            data-on={sel === at ? '' : undefined}
            bind:this={nodes[at]}
            onclick={() => commit(row.id)}
            onmouseenter={() => (sel = at)}
          >
            <span class="cd-pick__name">{row.label}</span>
            {#if row.note}
              <span class="cd-pick__note" title={row.note}>{row.note}</span>
            {/if}
          </button>
        {/each}

        {#if query.trim() && view.empty}
          <p class="cd-pick__none">Nothing matches “{query.trim()}”</p>
        {/if}
      </div>
      <p class="cd-pick__hint" role="status">{hint}</p>
    </div>
  {/if}
</span>
