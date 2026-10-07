<!--
  The view editor (Phase 5, §4.8 Principle 3).

  A view is a query plus a shape, and this sheet shows both halves as what they
  are: one row per **query key** (`filter`, `sort`, `group`, `limit`, `columns`)
  and one row per **block**, each naming the command it dispatches and the JSON
  pointer it writes. That is Principle 1's rule kept literally — one list, one
  vocabulary, no mirror surface:

    filter    →  view.setFilter   content/views.json#/views/<name>/filter
    block 2   →  view.block.set   content/views.json#/views/<name>/blocks/2
    show as…  →  view.setLayout   …/layout, or …/blocks/<index>/kind

  Two decisions worth naming:

  * **An empty field clears the key.** It is removed, never written as `""`: an
    undeclared filter and an empty one are different documents, and only the
    first means "no filter" (`view_ops::set_query` takes the same route).
  * **The preview is the engine's own evaluator** (`expr.eval`, §4.4). A frontend
    that evaluated `filter` to preview it would be a second implementation with
    its own null and date rules — and it would then disagree with the commit it
    is previewing.
-->
<script lang="ts">
  import Sheet from '../shell/Sheet.svelte';
  import Menu from '../shell/Menu.svelte';
  import { IpcError, dispatch } from '../ipc';
  import { app } from '../session.svelte';
  import { BLOCK_KINDS, RECORD_KINDS, requiredKeys } from '../blocks/registry';
  import { type MenuRow } from '../commands/registry';
  import type { ViewDefRead } from '../types';

  let { name, onclose }: { name: string; onclose: () => void } = $props();

  const view = $derived<ViewDefRead | null>(app.views?.views[name] ?? null);
  const declared = $derived(view?.blocks ?? []);

  /** Drafts, keyed by JSON pointer path — a keystroke is not a write. */
  let drafts = $state<Record<string, string>>({});
  let previews = $state<Record<string, string>>({});
  let addKind = $state('stat');
  let addSource = $state('');
  let addValues = $state<Record<string, string>>({});
  let error = $state<string | null>(null);
  let busy = $state(false);
  let openMenu = $state<number | null>(null);

  /** What a declared key holds right now, as the file has it. */
  function stored(key: string): string {
    const value = view ? (view as unknown as Record<string, unknown>)[key] : null;
    if (value === null || value === undefined) return '';
    if (Array.isArray(value)) return value.join(', ');
    return String(value);
  }

  function draftOf(key: string): string {
    return key in drafts ? drafts[key] : stored(key);
  }

  /** Every saved view but this one: what a block may draw or fold. */
  const sources = $derived(Object.keys(app.views?.views ?? {}).sort());

  /** The record a preview evaluates against — the first of the view's kind. */
  async function sample(): Promise<string | null> {
    const type = view?.type ?? '';
    if (type === '') return null;
    const records = await app.recordsOf(type);
    return records[0]?.id ?? null;
  }

  /**
   * The live preview of an expression row, from the engine (§4.4), debounced —
   * the answer arrives when typing pauses, not on every keystroke.
   */
  let previewTimer: ReturnType<typeof setTimeout> | null = null;
  function preview(key: string, text: string): void {
    if (previewTimer) clearTimeout(previewTimer);
    previewTimer = setTimeout(() => {
      void (async () => {
        if (text.trim() === '') {
          previews = { ...previews, [key]: '' };
          return;
        }
        try {
          const id = await sample();
          const result = await dispatch(
            'expr.eval',
            { expr: text, ...(id ? { id } : {}) },
            { plan: app.plan },
          );
          const data = result.data as { value?: unknown; type?: string };
          const shown =
            data.value === null || data.value === undefined ? 'null' : String(data.value);
          previews = { ...previews, [key]: `on ${id ?? 'no record'} → ${shown} (${data.type})` };
        } catch (failure) {
          previews = {
            ...previews,
            [key]: failure instanceof IpcError ? failure.text : String(failure),
          };
        }
      })();
    }, 250);
  }

  /**
   * One write, through the app's single dispatch path — which is what makes it
   * undoable (D14) and what keeps the editor from having a private mutation.
   */
  async function write(id: string, params: Record<string, unknown>): Promise<boolean> {
    busy = true;
    error = null;
    try {
      const result = await app.run(id, params);
      if (!result) {
        error = app.toast ?? 'the engine refused that';
        app.toast = null;
        return false;
      }
      drafts = {};
      return true;
    } finally {
      busy = false;
    }
  }

  async function setKey(key: string, command: string): Promise<void> {
    const text = draftOf(key);
    await write(command, { name, [key]: text.trim() === '' ? null : text.trim() });
  }

  async function addBlock(): Promise<void> {
    const params: Record<string, unknown> = { name, kind: addKind };
    if (addSource !== '') params.source = addSource;
    const values = Object.entries(addValues)
      .filter(([, value]) => value.trim() !== '')
      .map(([key, value]) => `${key}=${value}`);
    if (values.length > 0) params.value = values;
    if (await write('view.block.add', params)) {
      addValues = {};
      addSource = '';
    }
  }

  async function setBlockKey(index: number, key: string, value: string | null): Promise<void> {
    const params: Record<string, unknown> = { name, index: String(index), key };
    if (value !== null && value.trim() !== '') params.value = value;
    await write('view.block.set', params);
  }

  function blockRows(index: number): MenuRow[] {
    const kind = String(declared[index]?.kind ?? '');
    return RECORD_KINDS.map((layout) => ({
      id: 'view.setLayout',
      title: layout === kind ? `${layout} ✓` : layout,
      run: () => void write('view.setLayout', { name, block: String(index), layout }),
    }));
  }

  /** The simple keys a block of this kind exposes as text rows. */
  function keysOf(kind: string): Array<{ key: string; label: string }> {
    switch (kind) {
      case 'stat':
        return [
          { key: 'label', label: 'Label' },
          { key: 'expr', label: 'Value' },
          { key: 'reduce', label: 'Fold' },
          { key: 'view', label: 'Source view' },
        ];
      case 'chart':
        return [
          { key: 'label', label: 'Label' },
          { key: 'x', label: 'Bucket by' },
          { key: 'y', label: 'Value' },
          { key: 'days', label: 'Days back' },
          { key: 'view', label: 'Source view' },
        ];
      case 'callout':
        return [
          { key: 'title', label: 'Title' },
          { key: 'text', label: 'Text' },
        ];
      case 'conditional':
        return [
          { key: 'when', label: 'Show when' },
          { key: 'view', label: 'Source view' },
        ];
      default:
        return [
          { key: 'title', label: 'Title' },
          { key: 'view', label: 'Source view' },
        ];
    }
  }

  function textAt(index: number, key: string): string {
    const value = declared[index]?.[key];
    return value === null || value === undefined ? '' : String(value);
  }

  const QUERY_ROWS = [
    { key: 'filter', command: 'view.setFilter', label: 'Filter', hint: 'an L2 expression' },
    { key: 'sort', command: 'view.setSort', label: 'Sort', hint: 'keys, `-` for descending' },
    { key: 'group', command: 'view.setGroup', label: 'Group', hint: 'a board reads this' },
  ];
</script>

<Sheet
  title="Edit view"
  subtitle={`${name} · content/views.json#/views/${name}`}
  {onclose}
>
  {#if !view}
    <p class="blk__empty">this view is not in the plan any more.</p>
  {:else}
    <div class="cd-sheet__stack">
      <p class="cd-sheet__hint">
        {#if view.components !== undefined}
          a composed screen — your components, not a query
        {:else if view.type}
          shows <strong>{view.type}</strong> · declared as <code>{view.layout}</code>
        {:else}
          this view declares neither a kind nor components
        {/if}
        {#if declared.length > 0}· {declared.length} block(s){/if}
      </p>

      {#each QUERY_ROWS as row (row.key)}
        <div class="cd-sheet__field">
          <label class="cd-sheet__label" for={`view-${row.key}`}>
            {row.label}
            <span class="cd-sheet__key">{row.key}</span>
          </label>
          <input
            id={`view-${row.key}`}
            class="cd-sheet__well"
            type="text"
            autocomplete="off"
            value={draftOf(row.key)}
            data-view-key={row.key}
            placeholder={row.hint}
            oninput={(event) => {
              const text = (event.currentTarget as HTMLInputElement).value;
              drafts = { ...drafts, [row.key]: text };
              preview(row.key, text);
            }}
            onkeydown={(event) => {
              if (event.key === 'Enter') void setKey(row.key, row.command);
            }}
          />
        </div>
        <div class="cd-sheet__row">
          <span class="cd-sheet__hint">{previews[row.key] ?? ''}</span>
          <span class="cd-sheet__spacer"></span>
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            data-command={row.command}
            data-placement="view.editor"
            disabled={busy}
            onclick={() => void setKey(row.key, row.command)}
          >
            Write
          </button>
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            data-command={row.command}
            data-placement="view.editor"
            title="Remove the key — an undeclared filter and an empty one are different documents"
            disabled={busy}
            onclick={() => void write(row.command, { name, [row.key]: null })}
          >
            Clear
          </button>
        </div>
      {/each}

      <div class="cd-sheet__field">
        <label class="cd-sheet__label" for="view-limit">Limit <span class="cd-sheet__key">limit</span></label>
        <input
          id="view-limit"
          class="cd-sheet__well"
          type="number"
          min="0"
          value={draftOf('limit')}
          data-view-key="limit"
          oninput={(event) =>
            (drafts = { ...drafts, limit: (event.currentTarget as HTMLInputElement).value })}
        />
      </div>
      <div class="cd-sheet__row">
        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button"
          data-command="view.setLimit" data-placement="view.editor" disabled={busy}
          onclick={() => void setKey('limit', 'view.setLimit')}>Write</button>
        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button"
          data-command="view.setLimit" data-placement="view.editor" disabled={busy}
          onclick={() => void write('view.setLimit', { name, limit: null })}>Clear</button>
      </div>

      <div class="cd-sheet__field">
        <label class="cd-sheet__label" for="view-columns">
          Columns <span class="cd-sheet__key">columns</span>
        </label>
        <input
          id="view-columns"
          class="cd-sheet__well"
          type="text"
          autocomplete="off"
          value={draftOf('columns')}
          data-view-key="columns"
          placeholder="key, key, key — empty shows every column"
          oninput={(event) =>
            (drafts = { ...drafts, columns: (event.currentTarget as HTMLInputElement).value })}
        />
      </div>
      <div class="cd-sheet__row">
        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button"
          data-command="view.setColumns" data-placement="view.editor" disabled={busy}
          onclick={() => void setKey('columns', 'view.setColumns')}>Write</button>
        <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button"
          data-command="view.setColumns" data-placement="view.editor" disabled={busy}
          onclick={() => void write('view.setColumns', { name, columns: null })}>Clear</button>
      </div>

      <h3 class="cd-settings__title">Blocks</h3>
      {#if declared.length === 0}
        <p class="blk__empty">
          {#if view.components !== undefined}
            This screen declares no blocks — it draws the components you put on it.
            A block added here stands in their place.
          {:else if view.layout}
            This view declares no blocks, so it draws itself as one <code>{view.layout}</code>
            block. Add one below to compose it.
          {:else}
            This view declares no blocks, and no layout to draw itself as.
          {/if}
        </p>
      {:else}
        {#each declared as block, index (index)}
          <div class="cd-sheet__row" data-block-index={index}>
            <span class="cd-settings__keys">#{index}</span>
            <div class="cd-menu-wrap">
              <button
                class="cd-pill cd-pill--quiet cd-pill--sm"
                type="button"
                aria-haspopup="menu"
                aria-expanded={openMenu === index}
                data-command="view.setLayout"
                data-placement="view.editor"
                title="A block's kind is its layout"
                onclick={() => (openMenu = openMenu === index ? null : index)}
              >
                {String(block.kind ?? '?')} ▾
              </button>
              {#if openMenu === index}
                <Menu rows={blockRows(index)} native={false} onclose={() => (openMenu = null)} />
              {/if}
            </div>
            <span class="cd-settings__value">
              {block.view ? `draws ${block.view}` : 'draws this view'}
            </span>
            <span class="cd-sheet__spacer"></span>
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              data-command="view.block.remove"
              data-placement="view.editor"
              disabled={busy}
              title="Remove this block — removing the last one returns the view to a single list"
              onclick={() => void write('view.block.remove', { name, index: String(index) })}
            >
              Remove
            </button>
          </div>
          {#each keysOf(String(block.kind ?? '')) as row (row.key)}
            {@const path = `blocks/${index}/${row.key}`}
            <div class="cd-sheet__field">
              <label class="cd-sheet__label" for={`view-${index}-${row.key}`}>
                {row.label}<span class="cd-sheet__key">#{index} {row.key}</span>
              </label>
              <input
                id={`view-${index}-${row.key}`}
                class="cd-sheet__well"
                type="text"
                autocomplete="off"
                value={path in drafts ? drafts[path] : textAt(index, row.key)}
                data-view-key={path}
                oninput={(event) =>
                  (drafts = { ...drafts, [path]: (event.currentTarget as HTMLInputElement).value })}
                onkeydown={(event) => {
                  if (event.key === 'Enter') {
                    void setBlockKey(index, row.key, (event.currentTarget as HTMLInputElement).value);
                  }
                }}
              />
            </div>
            <div class="cd-sheet__row">
              <span class="cd-sheet__spacer"></span>
              <button
                class="cd-pill cd-pill--quiet cd-pill--sm"
                type="button"
                data-command="view.block.set"
                data-placement="view.editor"
                disabled={busy}
                onclick={() => void setBlockKey(index, row.key, path in drafts ? drafts[path] : textAt(index, row.key))}
              >
                Set
              </button>
            </div>
          {/each}
          {@const nested = `blocks/${index}/blocks`}
          <div class="cd-sheet__field">
            <label class="cd-sheet__label" for={`view-${index}-nested`}>
              Children<span class="cd-sheet__key">#{index} blocks</span>
            </label>
            <input
              id={`view-${index}-nested`}
              class="cd-sheet__well"
              type="text"
              autocomplete="off"
              value={nested in drafts ? drafts[nested] : JSON.stringify(block.blocks ?? [])}
              data-view-key={nested}
              placeholder="the children of a container block, as JSON"
              oninput={(event) =>
                (drafts = { ...drafts, [nested]: (event.currentTarget as HTMLInputElement).value })}
            />
          </div>
          <div class="cd-sheet__row">
            <span class="cd-sheet__spacer"></span>
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              data-command="view.block.set"
              data-placement="view.editor"
              disabled={busy}
              onclick={() =>
                void setBlockKey(index, 'blocks', nested in drafts ? drafts[nested] : '[]')}
            >
              Set children
            </button>
          </div>
        {/each}
      {/if}

      <h3 class="cd-settings__title">Add a block</h3>
      <div class="cd-sheet__field">
        <label class="cd-sheet__label" for="view-add-kind">Kind <span class="cd-sheet__key">kind</span></label>
        <select
          id="view-add-kind"
          class="cd-sheet__well cd-sheet__select"
          value={addKind}
          onchange={(event) => {
            addKind = (event.currentTarget as HTMLSelectElement).value;
            addValues = {};
          }}
        >
          {#each BLOCK_KINDS as kind (kind)}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
      </div>
      <div class="cd-sheet__field">
        <label class="cd-sheet__label" for="view-add-source">
          Draws <span class="cd-sheet__key">view</span>
        </label>
        <select
          id="view-add-source"
          class="cd-sheet__well cd-sheet__select"
          value={addSource}
          onchange={(event) => (addSource = (event.currentTarget as HTMLSelectElement).value)}
        >
          <option value="">this view</option>
          {#each sources as source (source)}
            <option value={source}>{source}</option>
          {/each}
        </select>
      </div>
      {#each requiredKeys(addKind) as row (row.key)}
        <div class="cd-sheet__field">
          <label class="cd-sheet__label" for={`view-add-${row.key}`}>
            {row.label}<span class="cd-sheet__key">{row.key}</span>
          </label>
          <input
            id={`view-add-${row.key}`}
            class="cd-sheet__well"
            type="text"
            autocomplete="off"
            value={addValues[row.key] ?? ''}
            placeholder={row.hint}
            oninput={(event) =>
              (addValues = {
                ...addValues,
                [row.key]: (event.currentTarget as HTMLInputElement).value,
              })}
          />
        </div>
      {/each}
      <p class="cd-sheet__hint">
        a block is validated with the rest of the plan: a callout needs its text, a chart its axes
      </p>
    </div>
  {/if}

  {#if error}
    <p class="cd-sheet__error" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    <span class="cd-sheet__hint">{declared.length} block(s) · ⌘Z undoes any write here</span>
    <span class="cd-sheet__spacer"></span>
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm"
      type="button"
      data-command="view.block.add"
      data-placement="view.editor"
      disabled={busy}
      onclick={() => void addBlock()}
    >
      Add block
    </button>
    <button class="cd-pill" type="button" onclick={onclose}>Done</button>
  {/snippet}
</Sheet>
