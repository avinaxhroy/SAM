<!--
  The model picker (§4.7's model pill; UI_SPEC "model pill").

  The pill opens a searchable listbox instead of the native `<select>` the
  first pass shipped: a CLI like Oh My Pi enumerates hundreds of models, and an
  OS popup can be neither searched nor capped. Rows group by the provider the
  id or label already carries (`provider/model`, `model · provider`), *Default*
  stays pinned first, *Custom…* last — the same three commitments the pill
  always made, now legible at 500 rows.

  The list is read lazily: opening the card is what asks the CLI
  (`onOpen` → `agent.ensureModels`), so the deck itself never waits on
  `opencode models`' network round-trip.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { AgentModel } from '../ipc';

  let {
    label,
    value,
    models,
    source,
    loading = false,
    disabled = false,
    onPick,
    onOpen,
  }: {
    /** What the closed pill shows (the parent owns how a choice reads). */
    label: string;
    /** `default` · a model id · `__custom__`. */
    value: string;
    models: AgentModel[];
    source: 'live' | 'fallback' | 'profile' | 'none' | null;
    loading?: boolean;
    disabled?: boolean;
    onPick: (option: string) => void;
    onOpen?: () => void;
  } = $props();

  const CUSTOM = '__custom__';

  let open = $state(false);
  let query = $state('');
  let sel = $state(0);
  let field = $state<HTMLInputElement | null>(null);
  let nodes = $state<HTMLElement[]>([]);
  let moved = $state(false);
  /** The id set the highlight was last aligned against. */
  let aligned = '';

  /** The provider an id or label already names — the row's only grouping. */
  function providerOf(model: AgentModel): string {
    const slash = model.id.indexOf('/');
    if (slash > 0) return model.id.slice(0, slash);
    const dot = model.label.indexOf(' · ');
    if (dot > 0) return model.label.slice(dot + 3);
    return '';
  }

  /**
   * The card's one derivation: filtered models in CLI order, grouped by
   * provider (first-seen first), each row carrying its flat index so the
   * arrows and the DOM ids cannot disagree. Index 0 is *Default*, the last
   * index is *Custom…*.
   */
  const picker = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const keep = models.filter(
      (model) =>
        needle.length === 0 ||
        model.id.toLowerCase().includes(needle) ||
        model.label.toLowerCase().includes(needle),
    );
    const order: string[] = [];
    const byProvider = new Map<string, Array<{ model: AgentModel; at: number }>>();
    let at = 1;
    for (const model of keep) {
      const provider = providerOf(model);
      if (!byProvider.has(provider)) {
        byProvider.set(provider, []);
        order.push(provider);
      }
      byProvider.get(provider)!.push({ model, at: at });
      at += 1;
    }
    const customAt = at;
    const ids: string[] = ['default', ...keep.map((model) => model.id), CUSTOM];
    const groups = order.map((name) => ({ name, rows: byProvider.get(name)! }));
    return {
      groups,
      customAt,
      ids,
      showMarks: groups.length > 1 || (groups[0]?.name.length ?? 0) > 0,
    };
  });

  // Focus the search field the moment the card opens.
  $effect(() => {
    if (open) field?.focus();
  });

  /**
   * The list arrives after the card opens (`agent.models` is a network round
   * trip): an index taken against the empty list would point at a different
   * row once it lands, and Enter would commit a model the student never
   * highlighted — and persist it. Until the student moves the highlight, it
   * follows the chosen *value*, wherever that value lands in the new list.
   */
  $effect(() => {
    const ids = picker.ids;
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
    if ((event.target as Element | null)?.closest?.('.cd-mpick')) return;
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
      const at = picker.ids.indexOf(value);
      sel = at >= 0 ? at : 0;
      moved = false;
      onOpen?.();
    }
  }

  function clamp(at: number): void {
    sel = Math.min(Math.max(at, 0), Math.max(picker.ids.length - 1, 0));
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
      clamp(picker.ids.length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      const option = picker.ids[sel] ?? 'default';
      commit(option);
    }
  }
</script>

<span class="cd-deck__pill cd-mpick" class:is-disabled={disabled}>
  <Icon name="cube" size={13} />
  <button
    class="cd-mpick__b"
    type="button"
    disabled={disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label="Which model the agent should use"
    onclick={toggle}
  >
    <span class="cd-mpick__value" title={value === 'default' ? 'Default' : value}>{label}</span>
    <Icon name="chevronDown" size={12} />
  </button>

  {#if open}
    <div class="cd-mpick__card">
      <input
        class="cd-mpick__q"
        type="search"
        role="combobox"
        aria-expanded="true"
        aria-controls="cd-mpick-list"
        aria-autocomplete="list"
        aria-activedescendant={`cd-mpick-${sel}`}
        aria-label="Search the models this CLI reports"
        placeholder="Search models…"
        autocomplete="off"
        spellcheck="false"
        bind:this={field}
        bind:value={query}
        oninput={() => {
          // Typing is a move: the highlight stays where the student put it
          // (the top of the new list) instead of chasing the stored value.
          sel = 0;
          moved = true;
        }}
        onkeydown={onkeydown}
      />
      <div class="cd-mpick__list" id="cd-mpick-list" role="listbox" aria-label="Models">
        <button
          class="cd-mpick__row"
          type="button"
          role="option"
          id="cd-mpick-0"
          aria-selected={sel === 0}
          data-on={sel === 0 ? '' : undefined}
          bind:this={nodes[0]}
          onclick={() => commit('default')}
          onmouseenter={() => (sel = 0)}
        >
          <span class="cd-mpick__name">Default</span>
          <span class="cd-mpick__note">the CLI's own choice</span>
        </button>

        {#each picker.groups as group (group.name)}
          {#if picker.showMarks && group.name}
            <span class="cd-mpick__mark" aria-hidden="true">{group.name}</span>
          {/if}
          {#each group.rows as row (row.at)}
            <button
              class="cd-mpick__row"
              type="button"
              role="option"
              id={`cd-mpick-${row.at}`}
              aria-selected={sel === row.at}
              data-on={sel === row.at ? '' : undefined}
              bind:this={nodes[row.at]}
              onclick={() => commit(row.model.id)}
              onmouseenter={() => (sel = row.at)}
            >
              <span class="cd-mpick__name">{row.model.label}</span>
              {#if row.model.label !== row.model.id}
                <span class="cd-mpick__id" title={row.model.id}>{row.model.id}</span>
              {/if}
            </button>
          {/each}
        {/each}

        {#if query.trim() && picker.groups.length === 0}
          <p class="cd-mpick__none">Nothing matches “{query.trim()}”</p>
        {/if}

        <button
          class="cd-mpick__row"
          type="button"
          role="option"
          id={`cd-mpick-${picker.customAt}`}
          aria-selected={sel === picker.customAt}
          data-on={sel === picker.customAt ? '' : undefined}
          bind:this={nodes[picker.customAt]}
          onclick={() => commit(CUSTOM)}
          onmouseenter={() => (sel = picker.customAt)}
        >
          <span class="cd-mpick__name">Custom…</span>
          <span class="cd-mpick__note">any id the CLI accepts</span>
        </button>
      </div>
      <p class="cd-mpick__hint" role="status">
        {#if loading}
          Reading the models this CLI reports…
        {:else if source === 'fallback'}
          The CLI's documented set — Custom… takes anything newer.
        {:else if source === 'live'}
          Read from the CLI itself.
        {:else if source === 'profile'}
          The ids this profile declares.
        {:else if models.length === 0}
          This CLI lists no models — Custom… takes any id it accepts.
        {/if}
      </p>
    </div>
  {/if}
</span>
