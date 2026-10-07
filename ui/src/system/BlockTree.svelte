<!--
  Block tree editor for composed screen views (P3 · U6, UI_SPEC §4 E2, UI_PLAN.md U6, UX_FLOWS.md §212).
  Invariants:
    - Display naming via label pairs and IdPair keys (D2, D13).
    - Dispatch commands via `view.block.*` with parity attributes.
    - Inline error alerts on dispatch rejection (D8).
    - Recursion bounded by MAX_BLOCK_DEPTH.
-->
<script lang="ts">
  import IdPair from '../shell/IdPair.svelte';
  import { BLOCK_KINDS, MAX_BLOCK_DEPTH, isContainerKind, requiredKeys } from '../blocks/registry';
  import { nameOf } from '../types';
  import { app } from '../session.svelte';

  /** Block representation corresponding to engine `BlockDef`. */
  type Block = {
    kind: string;
    view?: string;
    title?: string;
    label?: string;
    text?: string;
    expr?: string;
    reduce?: string;
    when?: string;
    limit?: number;
    x?: string;
    y?: string;
    days?: number;
    blocks?: Block[];
    else?: Block[];
    [key: string]: unknown;
  };

  type Owner = { path: string; index: number; parent: string };

  let {
    view,
    blocks,
    onrun,
    busy = false,
  }: {
    view: string;
    blocks: ReadonlyArray<Record<string, unknown>> | undefined;
    onrun: (id: string, params: Record<string, unknown>) => unknown;
    busy?: boolean;
  } = $props();

  const tree = $derived((blocks ?? []) as Block[]);

  const KIND_WORDS: Record<string, string> = {
    list: 'List',
    table: 'Table',
    board: 'Board',
    calendar: 'Calendar',
    timeline: 'Timeline',
    graph: 'Graph',
    tree: 'Tree',
    cardGrid: 'Card grid',
    stat: 'Stat',
    chart: 'Chart',
    callout: 'Callout',
    columns: 'Two columns',
    conditional: 'Conditional',
    repeat: 'Repeat',
  };

  const KEY_WORDS: Record<string, string> = {
    title: 'Title',
    label: 'Label',
    text: 'Text',
    expr: 'Value',
    reduce: 'Fold',
    when: 'Show when',
    view: 'Source view',
    x: 'Bucket by',
    y: 'Value',
    days: 'Days back',
    limit: 'How many to draw',
  };

  /** Required keys per kind for `view.block.add`, validated whole-plan (§3.7). */
  const EXTRA_REQUIRED: Record<string, Array<{ key: string; label: string; hint: string }>> = {
    stat: [{ key: 'label', label: 'Label', hint: 'the name the number is read by' }],
  };

  const TREE_KEYS = new Set(['kind', 'blocks', 'else']);

  let expanded = $state<string | null>(null);
  let drafts = $state<Record<string, string>>({});
  let choosing = $state<{ parent: string; index: number | null; where: string } | null>(null);
  let asking = $state<string | null>(null);
  let answers = $state<Record<string, string>>({});
  let failure = $state<{ at: string; message: string } | null>(null);

  function kindWord(kind: string): string {
    return KIND_WORDS[kind] ?? nameOf(kind.replace(/([a-z0-9])([A-Z])/g, '$1 $2'));
  }

  function plainKey(key: string): string {
    return KEY_WORDS[key] ?? nameOf(key.replace(/([a-z0-9])([A-Z])/g, '$1 $2'));
  }

  function addressOf(container: string, index: number): string {
    return container === '' ? String(index) : `${container}.${index}`;
  }

  function needs(kind: string): Array<{ key: string; label: string; hint: string }> {
    return [...requiredKeys(kind), ...(EXTRA_REQUIRED[kind] ?? [])];
  }

  function countBlocks(items: Block[]): number {
    let total = 0;
    for (const block of items) {
      total += 1 + countBlocks(block.blocks ?? []) + countBlocks(block.else ?? []);
    }
    return total;
  }

  const total = $derived(countBlocks(tree));

  function summary(block: Block): Array<{ key: string; label: string; value: string; mono: boolean }> {
    const parts: Array<{ key: string; label: string; value: string; mono: boolean }> = [];
    const take = (key: 'title' | 'label' | 'text'): void => {
      const value = block[key];
      if (typeof value === 'string' && value.trim() !== '') {
        parts.push({ key, label: KEY_WORDS[key], value, mono: false });
      }
    };
    take('title');
    take('label');
    take('text');
    // Maintain IdPair display for engine keys and expressions (D2, D13).
    for (const key of ['expr', 'view'] as const) {
      const value = block[key];
      if (typeof value === 'string' && value.trim() !== '') {
        parts.push({ key, label: KEY_WORDS[key], value, mono: true });
      }
    }
    return parts;
  }

  function stored(value: unknown): string {
    if (value === null || value === undefined) return '';
    if (typeof value === 'string') return value;
    if (typeof value === 'number' || typeof value === 'boolean') return String(value);
    return JSON.stringify(value) ?? '';
  }

  function draftKey(path: string, key: string): string {
    return `${path}:${key}`;
  }

  function textFor(path: string, key: string, value: unknown): string {
    const draft = drafts[draftKey(path, key)];
    return draft === undefined ? stored(value) : draft;
  }

  function editableKeys(block: Block): string[] {
    return Object.keys(block).filter((key) => !TREE_KEYS.has(key));
  }

  function messageOf(thrown: unknown): string {
    if (thrown && typeof thrown === 'object') {
      const record = thrown as { text?: unknown; message?: unknown };
      if (typeof record.text === 'string' && record.text !== '') return record.text;
      if (typeof record.message === 'string' && record.message !== '') return record.message;
    }
    const text = String(thrown);
    return text === '' || text === '[object Object]' ? 'the engine refused that change' : text;
  }

  async function run(id: string, params: Record<string, unknown>, at: string): Promise<boolean> {
    failure = null;
    try {
      const answer = await Promise.resolve(onrun(id, params));
      if (typeof answer === 'string' && answer.trim() !== '') {
        failure = { at, message: answer };
        return false;
      }
      if (answer === false) {
        failure = { at, message: 'the engine refused that change' };
        return false;
      }
      // Propagate rejection diagnostic if dispatch returned null/undefined.
      if (answer === null || answer === undefined) {
        failure = { at, message: app.lastDiagnostic ?? 'the engine refused that change' };
        return false;
      }
      return true;
    } catch (thrown) {
      failure = { at, message: messageOf(thrown) };
      return false;
    }
  }

  function moveParams(path: string, container: string, index: number): Record<string, unknown> {
    return { name: view, path, parent: container, index };
  }

  function move(path: string, container: string, index: number): void {
    void run('view.block.move', moveParams(path, container, index), path);
  }

  function indent(path: string, previous: string): void {
    void run('view.block.move', { name: view, path, parent: `${previous}.blocks`, index: 0 }, path);
  }

  function outdent(path: string, owner: Owner): void {
    void run('view.block.move', moveParams(path, owner.parent, owner.index + 1), path);
  }

  function remove(path: string): void {
    void run('view.block.remove', { name: view, path }, path);
  }

  function openPicker(parent: string, index: number | null, where: string): void {
    choosing = { parent, index, where };
    asking = null;
    answers = {};
    failure = null;
  }

  function closePicker(): void {
    choosing = null;
    asking = null;
    answers = {};
  }

  function choose(kind: string): void {
    const wanted = needs(kind);
    if (wanted.length === 0) {
      void add(kind, {});
      return;
    }
    asking = kind;
    answers = Object.fromEntries(wanted.map((field) => [field.key, '']));
  }

  const answered = $derived(
    asking === null || needs(asking).every((field) => (answers[field.key] ?? '').trim() !== ''),
  );

  async function add(kind: string, values: Record<string, string>): Promise<void> {
    const place = choosing;
    if (place === null) return;
    const params: Record<string, unknown> = { name: view, kind };
    if (place.parent !== '') params.parent = place.parent;
    if (place.index !== null) params.index = place.index;
    const pairs = Object.entries(values)
      .filter(([, value]) => value.trim() !== '')
      .map(([key, value]) => `${key}=${value}`);
    if (pairs.length > 0) params.value = pairs;
    if (await run('view.block.add', params, place.parent)) closePicker();
  }

  async function commitKey(path: string, key: string, text: string): Promise<void> {
    if (!(await run('view.block.set', { name: view, path, key, value: text }, path))) return;
    const next = { ...drafts };
    delete next[draftKey(path, key)];
    drafts = next;
  }
</script>

<div class="cd-card cd-bt">
  <div class="cd-card__head">
    <h2 class="cd-card__title">Blocks</h2>
    <span class="cd-card__sub">
      {total}{total === 1 ? ' block' : ' blocks'} on this screen
    </span>
  </div>

  {#if total === 0}
    <p class="cd-hint">
      This view is a plain list. Add a block and it becomes a screen of its own — a row of stats, a
      chart over the last fortnight, a note above the table.
    </p>
  {/if}

  {#snippet group(items: Block[], container: string, depth: number, owner: Owner | null)}
    <div class="cd-bt__group">
      {#each items as block, index (addressOf(container, index))}
        {@const path = addressOf(container, index)}
        {@const word = kindWord(block.kind)}
        {#if depth >= MAX_BLOCK_DEPTH}
          <p class="cd-hint">
            A block sits deeper than {MAX_BLOCK_DEPTH} levels. SAM stops here and does not draw it —
            the same limit the renderer has, so what you see is what the screen shows.
          </p>
        {:else}
          <div class="cd-bt__row">
            <div class="cd-bt__head">
              <span class="cd-bt__kind">{word}</span>
              <IdPair label="At" title={`Copy the address of this ${word}`} value={path} />
              {#each summary(block) as part (part.key)}
                {#if part.mono}
                  <IdPair label={part.label} value={part.value} />
                {:else}
                  <span class="cd-bt__say">{part.label} <b>{part.value}</b></span>
                {/if}
              {/each}

              <span class="cd-bt__spacer"></span>

              <div class="cd-bt__acts">
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  data-command="view.block.move"
                  data-placement="system"
                  disabled={busy || index === 0}
                  aria-label={`Move up: ${word}`}
                  onclick={() => move(path, container, index - 1)}
                >
                  Move up
                </button>
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  data-command="view.block.move"
                  data-placement="system"
                  disabled={busy || index >= items.length - 1}
                  aria-label={`Move down: ${word}`}
                  onclick={() => move(path, container, index + 1)}
                >
                  Move down
                </button>
                {#if index > 0 && isContainerKind(items[index - 1].kind)}
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm"
                    type="button"
                    data-command="view.block.move"
                    data-placement="system"
                    disabled={busy}
                    aria-label={`Indent: ${word}`}
                    title={`Put this ${word} inside the ${kindWord(items[index - 1].kind)} above it`}
                    onclick={() => indent(path, addressOf(container, index - 1))}
                  >
                    Indent
                  </button>
                {/if}
                {#if owner}
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm"
                    type="button"
                    data-command="view.block.move"
                    data-placement="system"
                    disabled={busy}
                    aria-label={`Outdent: ${word}`}
                    title="Lift this block out of its container"
                    onclick={() => outdent(path, owner)}
                  >
                    Outdent
                  </button>
                {/if}
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm cd-bt__keys-toggle"
                  type="button"
                  aria-expanded={expanded === path}
                  aria-label={`Keys: ${word}`}
                  title="Read and change every key this block carries"
                  onclick={() => (expanded = expanded === path ? null : path)}
                >
                  {expanded === path ? 'Hide keys' : 'Keys'}
                </button>
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm cd-bt__remove"
                  type="button"
                  data-command="view.block.remove"
                  data-placement="system"
                  disabled={busy}
                  aria-label={`Remove: ${word}`}
                  onclick={() => remove(path)}
                >
                  Remove
                </button>
              </div>
            </div>

            {#if failure?.at === path}
              <p class="cd-bt__error" role="alert">{failure.message}</p>
            {/if}

            {#if expanded === path}
              <div class="cd-form cd-bt__keys-list">
                {#each editableKeys(block) as key (key)}
                  <div class="cd-formrow cd-bt__keyrow">
                    <label class="cd-formrow__label" for={`blk-${path}-${key}`}>{plainKey(key)}</label>
                    <IdPair value={key} />
                    <input
                      id={`blk-${path}-${key}`}
                      class="cd-wellfield cd-bt__input"
                      type="text"
                      autocomplete="off"
                      spellcheck="false"
                      aria-label={`${plainKey(key)}: ${key}`}
                      value={textFor(path, key, block[key])}
                      data-command="view.block.set"
                      data-placement="system"
                      oninput={(event) => {
                        drafts = { ...drafts, [draftKey(path, key)]: event.currentTarget.value };
                      }}
                      onchange={(event) => void commitKey(path, key, event.currentTarget.value)}
                    />
                  </div>
                {/each}
                {#if editableKeys(block).length === 0}
                  <p class="cd-hint">
                    This block carries nothing but its kind. Its children are the tree below it.
                  </p>
                {/if}
                <p class="cd-hint">
                  Leave a field empty to drop that key. A number, `true` or a quoted list is read as
                  JSON; anything else is kept as words.
                </p>
              </div>
            {/if}

            {#if isContainerKind(block.kind)}
              <div class="cd-bt__children">
                <div class="cd-bt__nest">
                  <span class="cd-bt__grouplabel">Inside</span>
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm"
                    type="button"
                    disabled={busy}
                    aria-label={`Add child: ${word}`}
                    onclick={() => openPicker(`${path}.blocks`, null, `inside the ${word}`)}
                  >
                    Add child
                  </button>
                </div>

                {@render group(
                  block.blocks ?? [],
                  `${path}.blocks`,
                  depth + 1,
                  { path, index, parent: container },
                )}

                {#if choosing?.parent === `${path}.blocks`}
                  {@render picker(`${path}.blocks`)}
                {/if}

                <div class="cd-bt__nest">
                  <span class="cd-bt__grouplabel">Otherwise</span>
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm"
                    type="button"
                    disabled={busy}
                    aria-label={`Add else: ${word}`}
                    title="The blocks drawn when this test does not pass"
                    onclick={() => openPicker(`${path}.else`, null, `in the ${word}'s otherwise group`)}
                  >
                    {block.else ? 'Add to else' : 'Add else'}
                  </button>
                </div>

                {#if block.else}
                  {@render group(
                    block.else,
                    `${path}.else`,
                    depth + 1,
                    { path, index, parent: container },
                  )}
                {/if}

                {#if choosing?.parent === `${path}.else`}
                  {@render picker(`${path}.else`)}
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {/snippet}

  {#snippet picker(parent: string)}
    <div class="cd-bt__picker">
      <p class="cd-hint">
        {choosing?.where === 'this view'
          ? 'Add a block to this view'
          : `Add a block ${choosing?.where ?? ''}`}
      </p>
      <div class="cd-chips" role="group" aria-label="Block kinds">
        {#each BLOCK_KINDS as kind (kind)}
          <button
            class="cd-chip"
            type="button"
            data-command="view.block.add"
            data-placement="system"
            disabled={busy}
            aria-pressed={asking === kind}
            aria-label={`Add a ${kindWord(kind)} block`}
            onclick={() => choose(kind)}
          >
            {kindWord(kind)}
          </button>
        {/each}
      </div>

      {#if asking}
        <div class="cd-form cd-bt__ask">
          {#each needs(asking) as field (field.key)}
            <div class="cd-formrow cd-bt__keyrow">
              <label class="cd-formrow__label" for={`blk-${parent}-${field.key}`}>{field.label}</label>
              <IdPair value={field.key} />
              <input
                id={`blk-${parent}-${field.key}`}
                class="cd-wellfield cd-bt__input"
                type="text"
                autocomplete="off"
                aria-label={`${field.label}: ${field.key}`}
                placeholder={field.hint}
                value={answers[field.key] ?? ''}
                oninput={(event) => {
                  answers = { ...answers, [field.key]: event.currentTarget.value };
                }}
              />
            </div>
          {/each}
          <p class="cd-hint">
            A {kindWord(asking)} needs {needs(asking).map((field) => field.label.toLowerCase()).join(' and ')} before
            the engine will accept it — so it is asked for here, not guessed.
          </p>
          <div class="cd-rowflex">
            <button
              class="cd-pill cd-pill--sm"
              type="button"
              data-command="view.block.add"
              data-placement="system"
              disabled={busy || !answered}
              aria-label={`Add the ${kindWord(asking)} block`}
              onclick={() => void add(asking ?? '', answers)}
            >
              Add it
            </button>
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              onclick={() => {
                asking = null;
                answers = {};
              }}
            >
              Choose another kind
            </button>
          </div>
        </div>
      {/if}

      <div class="cd-rowflex">
        <button
          class="cd-pill cd-pill--quiet cd-pill--sm"
          type="button"
          aria-label="Stop adding a block"
          onclick={closePicker}
        >
          Cancel
        </button>
      </div>

      {#if failure?.at === parent}
        <p class="cd-bt__error" role="alert">{failure.message}</p>
      {/if}
    </div>
  {/snippet}

  {@render group(tree, '', 0, null)}

  {#if choosing?.parent === ''}
    {@render picker('')}
  {:else}
    <div class="cd-rowflex cd-bt__add">
      <button
        class="cd-pill cd-pill--sm"
        type="button"
        disabled={busy}
        aria-label="Add a block to this view"
        onclick={() => openPicker('', tree.length, 'this view')}
      >
        Add a block
      </button>
    </div>
  {/if}
</div>

<style>
  .cd-bt__group {
    display: grid;
    gap: var(--space-2xs);
  }
  .cd-bt__row {
    display: grid;
    gap: var(--space-2xs);
    padding: var(--space-xs) 0;
  }
  .cd-bt__row + .cd-bt__row,
  .cd-bt__group > .cd-bt__row + .cd-hint {
    border-top: 1px dashed var(--rule);
    padding-top: var(--space-sm);
  }
  .cd-bt__head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-xs);
  }
  .cd-bt__kind {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink);
  }
  .cd-bt__say {
    font-size: var(--text-2xs);
    color: var(--ink-3);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 32ch;
  }
  .cd-bt__say b {
    font-weight: var(--weight-body);
    color: var(--ink-2);
  }
  .cd-bt__spacer {
    flex: 1;
    min-width: var(--space-sm);
  }
  .cd-bt__acts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3xs);
  }
  .cd-bt__remove {
    color: var(--on-overdue);
  }
  .cd-bt__error {
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--r-mini);
    background: var(--chip-risk);
    color: var(--on-risk);
    font-size: var(--text-2xs);
  }
  .cd-bt__keys-list {
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--r-mini);
    background: var(--well);
  }
  .cd-bt__keyrow {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.1fr) minmax(0, 1.4fr);
  }
  .cd-bt__input {
    font-family: var(--font-mono, ui-monospace);
  }
  .cd-bt__children {
    display: grid;
    gap: var(--space-2xs);
    margin: var(--space-xs) 0 var(--space-2xs) var(--space-md);
    padding-left: var(--space-md);
    border-left: 1.5px dashed var(--rule-strong);
  }
  .cd-bt__nest {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .cd-bt__grouplabel {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .cd-bt__picker {
    display: grid;
    gap: var(--space-xs);
    justify-items: start;
    padding: var(--space-md);
    border: 1.5px dashed var(--rule-strong);
    border-radius: var(--r-tile);
  }
  .cd-bt__ask {
    width: 100%;
  }
  .cd-bt__add {
    margin-top: var(--space-md);
  }
</style>