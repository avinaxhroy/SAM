<!--
  The `stat` block: one number, its caption, and what it was counted over.

  The value is the engine's — a fold over the source view's records (§4.4's
  aggregate semantics for sum/avg/min/max, and `count` of the instances whose
  expression is neither null nor false) — so nothing here recomputes anything.

  What this component decides is only the reading: a null is *no value*, never a
  zero (§4.4), and a fold over a `duration` field is minutes, printed with the
  hours-and-minutes the design system uses everywhere else. The field's kind
  comes from the type the engine resolved, never from a guess about the number.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import type { BlockNode } from '../types';

  let { node }: { node: BlockNode } = $props();

  const value = $derived(node.value);
  const caption = $derived(node.label ?? node.title ?? node.expr ?? node.reduce ?? '');
  const missing = $derived(value === null || value === undefined);

  /** The field the expression names, when it names one — a duration reads as time. */
  const field = $derived(
    (app.types[node.type ?? '']?.fields ?? []).find((candidate) => candidate.key === node.expr),
  );
  const minutes = $derived(
    field?.type === 'duration' && typeof value === 'number' ? Math.round(value) : null,
  );
  const asMinutes = $derived(
    minutes === null
      ? ''
      : minutes < 60
        ? `${minutes}m`
        : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`,
  );
  const text = $derived(
    missing
      ? '—'
      : typeof value === 'number'
        ? String(Number.isInteger(value) ? value : Math.round(value * 100) / 100)
        : String(value),
  );

  /** The denominator, in the student's words: "12 topics", never `count of view`. */
  const counted = $derived.by(() => {
    // A stat block names the view it folded, not the type: the type comes from
    // the view's own definition.
    const type = node.type ?? (node.view ? app.views?.views?.[node.view]?.type : undefined) ?? '';
    const of = node.of ?? 0;
    if (type === '') return String(of);
    const plural = type.endsWith('s') ? type : `${type}s`;
    return `${of} ${plural}`;
  });
</script>

<section class="cd-card blk" data-block="stat">
  <div class="blk__stat">
    <span class="blk__caption">{caption}</span>
    <span class="blk__value" data-value={missing ? 'null' : String(value)}>{text}</span>
    <span class="blk__caption">
      {#if missing}
        no value — the expression could not be computed
      {:else if asMinutes}
        {asMinutes} across {counted}
      {:else if node.reduce === 'count'}
        {counted}
      {:else}
        across {counted}
      {/if}
    </span>
  </div>
</section>
