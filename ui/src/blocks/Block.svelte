<!--
  The block renderer (D8).

  One typed registry keyed by kind: this file is the walk, and each kind has
  exactly one component. The engine has already resolved everything — filters,
  sorts, folds, buckets, the recursion boundary — so nothing here evaluates an
  expression, and nothing here decides what a block *means*.

  Three rules are visible below:

  1. **The boundary is finite and stated.** `MAX_BLOCK_DEPTH` stops the walk and
     draws the placeholder, so a hand-edited tree cannot recurse the renderer to
     death. The loader warns about the same tree; this is the second half.
  2. **An unknown kind is stated, never blank.** A kind this build does not know
     draws `unknown block: "X" · known: …` with a Remove control — because the
     student is the one who can fix it, and the message has to say what the fix
     could be.
  3. **A block owns its own data.** A record block re-reads nothing: it draws the
     projection the engine handed it. `targetsFor` is the one thing a table or a
     relation cell needs from the app — the records a relation points at.
-->
<script lang="ts">
  import Variant from '../variants/Variant.svelte';
  import Stat from './Stat.svelte';
  import Chart from './Chart.svelte';
  import Callout from './Callout.svelte';
  import Columns from './Columns.svelte';
  import Conditional from './Conditional.svelte';
  import Repeat from './Repeat.svelte';
  import { MAX_BLOCK_DEPTH, isRecordKind, unknownKindMessage } from './registry';
  import { app } from '../session.svelte';
  import type { BlockNode, FieldRead } from '../types';

  let {
    node,
    depth = 0,
    targetsFor = () => [],
  }: {
    node: BlockNode;
    depth?: number;
    targetsFor?: (field: FieldRead) => Array<{ id: string; label: string }>;
  } = $props();

  /**
   * The eight record layouts are one surface, with one design
   * (`ui/src/variants/views/`): the head species, the read's own facts and the
   * shape each kind draws. This file is the screen for it, so the design never
   * reads `app` for data — the schema, the plan's today and the app's one
   * selection are handed down here.
   */
  const type = $derived(app.types[node.type ?? '']);

  const record = $derived(isRecordKind(node.kind));
  const placeholder = $derived(
    node.kind === 'placeholder'
      ? {
          message:
            node.message ??
            (node.reason === 'depth'
              ? `blocks nest deeper than ${MAX_BLOCK_DEPTH} — not drawn`
              : unknownKindMessage(node.unknown ?? '')),
        }
      : null,
  );
  const tooDeep = $derived(depth >= MAX_BLOCK_DEPTH);

  /**
   * The block's own findings, one row per **code** — the same refusal fires once
   * per record, so a raw list is one sentence nineteen times. The engine's own
   * words (a path, a record id: `record "topic.r1" sort key: …`) ride the row's
   * `title`; the student reads a sentence (D2, and the census rule behind it —
   * `records/RecordTable.svelte` carries the same line for the same reason).
   */
  const warnings = $derived.by(() => {
    const byCode = new Map<string, { code: string; count: number; detail: string }>();
    for (const warning of node.warnings ?? []) {
      const seen = byCode.get(warning.code);
      if (seen) {
        seen.count += 1;
        continue;
      }
      byCode.set(warning.code, {
        code: warning.code,
        count: 1,
        detail: `${warning.path}: ${warning.message}`,
      });
    }
    return [...byCode.values()];
  });

  /** One selection, one door, one create: the three things a drawing dispatches. */
  function select(id: string): void {
    app.selection = id;
  }

  function createRecord(): void {
    void app.openSheet({ kind: 'record.new', type: node.type ?? '' });
  }

  /**
   * Remove this block. A block the renderer cannot draw is still a *block* in
   * the file, so the fix is reachable from where the finding is: the command is
   * `view.block.remove` with the enclosing view and the block's index, which is
   * why the resolved node carries its own position.
   */
  function remove(): void {
    const name = node.view ?? app.outcome?.view ?? '';
    const index = app.blockIndex(node);
    if (name === '' || index === null) return;
    void app.run('view.block.remove', { name, index: String(index) });
  }
</script>

{#if placeholder}
  <!-- The stated fallback: what this build does not know, and what it does. -->
  <section class="cd-card blk blk--placeholder" data-block="placeholder">
    <p class="blk__fallback">
      <code>{placeholder.message}</code>
    </p>
    <span class="blk__spacer"></span>
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm"
      type="button"
      data-command="view.block.remove"
      data-placement="view.editor"
      title="Remove this block from the view's blocks"
      onclick={remove}
    >
      Remove block
    </button>
  </section>
{:else if tooDeep}
  <section class="cd-card blk" data-block="depth">
    <p class="blk__fallback">
      <code>blocks nest deeper than {MAX_BLOCK_DEPTH} — not drawn</code>
    </p>
  </section>
{:else if record}
  <svelte:boundary>
    <Variant
      surface="views"
      {node}
      {type}
      {targetsFor}
      today={app.today?.date ?? null}
      selected={app.selection}
      onSelect={select}
      onNew={createRecord}
    />
    {#snippet failed(error)}
      <section class="cd-card blk">
        <h2 class="cd-card__title">This block could not be drawn</h2>
        <p class="blk__fallback"><code>{String(error)}</code></p>
      </section>
    {/snippet}
  </svelte:boundary>
{:else if node.kind === 'stat'}
  <Stat {node} />
{:else if node.kind === 'chart'}
  <Chart {node} />
{:else if node.kind === 'callout'}
  <Callout {node} />
{:else if node.kind === 'columns'}
  <Columns {node} {depth} {targetsFor} />
{:else if node.kind === 'conditional'}
  <Conditional {node} {depth} {targetsFor} />
{:else if node.kind === 'repeat'}
  <Repeat {node} {depth} {targetsFor} />
{:else}
  <!-- A kind that passes the depth and record checks but has no renderer: the
       same stated fallback, so the vocabulary cannot silently grow a hole. -->
  <section class="cd-card blk blk--placeholder" data-block={node.kind}>
    <p class="blk__fallback"><code>{unknownKindMessage(node.kind)}</code></p>
    <span class="blk__spacer"></span>
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm"
      type="button"
      data-command="view.block.remove"
      data-placement="view.editor"
      onclick={remove}
    >
      Remove block
    </button>
  </section>
{/if}

{#each warnings as finding (finding.code)}
  <p class="blk__note" title={finding.detail}>
    A sort or filter this block asks for cannot be read for every record{#if finding.count > 1}
      ({finding.count} findings){/if} — hover this line for the engine’s own words.
  </p>
{/each}
