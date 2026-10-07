<!--
  The `chart` block: one fold over time, drawn as bars.

  Every bar is the engine's. The buckets arrive already bucketed, sorted
  ascending and gap-filled, so this component never re-folds, re-buckets or
  re-sorts — the only thing it computes is the *scale*: a height as a percentage
  of the series maximum, which is a property of the drawing rather than of the
  data. A null bucket is *no value* (§4.4), never a zero: it draws at the bar's
  minimum height in the zero tone and reads `—`, the same distinction `stat`
  makes. The block's own warnings are drawn by the dispatcher, not twice here.
-->
<script lang="ts">
  import type { BlockNode } from '../types';

  let { node }: { node: BlockNode } = $props();

  /** A bucket is measured only when the fold produced a finite number. */
  function amount(value: unknown): number | null {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
  }

  /** The same reading `stat` prints: an integer as-is, a fraction to two decimals. */
  function read(value: number | null): string {
    if (value === null) return '—';
    return String(Number.isInteger(value) ? value : Math.round(value * 100) / 100);
  }

  const max = $derived(
    Math.max(0, ...(node.buckets ?? []).map((bucket) => amount(bucket.value) ?? 0)),
  );

  const bars = $derived(
    (node.buckets ?? []).map((bucket) => {
      const value = amount(bucket.value);
      // Relative to the series maximum, so the tallest non-empty bucket fills
      // the plot; a series that folds to nothing draws every bar at zero.
      const height = value === null || max <= 0 ? 0 : Math.max(0, (value / max) * 100);
      return {
        key: bucket.key,
        text: read(value),
        // A `YYYY-MM-DD` key reads as its day; any other key as its last two characters.
        tick: bucket.key.slice(-2) || '—',
        height,
        // The one inline style in the block layer, and it is data, not design.
        style: `height: ${Math.round(height * 10) / 10}%`,
      };
    }),
  );

  const label = $derived(node.label ?? node.title ?? 'chart');
  const window = $derived(
    node.days === null || node.days === undefined ? 'all time' : `last ${node.days} days`,
  );
  const subtitle = $derived(
    `${node.reduce ?? 'sum'} of ${node.y ?? node.x ?? 'value'} · ${window} · ${bars.length} buckets`,
  );
  const series = $derived(bars.map((bar) => `${bar.key}: ${bar.text}`).join(' · '));
</script>

<section class="cd-card blk" data-block="chart">
  <header class="cd-card__head">
    <div>
      <h2 class="cd-card__title">{label}</h2>
      <p class="cd-card__sub">{subtitle}</p>
    </div>
  </header>

  {#if bars.length === 0}
    <p class="blk__empty" data-block="empty">this chart has no buckets yet</p>
  {:else}
    <div class="blk__chart">
      <div class="blk__bars">
        {#each bars as bar, position (position)}
          <div
            class="blk__bar"
            data-bucket={bar.key}
            role="img"
            aria-label="{bar.key}: {bar.text}"
            title="{bar.key}: {bar.text}"
          >
            <div
              class="blk__barfill"
              class:blk__barfill--zero={bar.height <= 0}
              data-value={bar.text}
              style={bar.style}
            ></div>
          </div>
        {/each}
      </div>

      <div class="blk__barlabels" aria-hidden="true">
        {#each bars as bar, position (position)}
          <span class="blk__barlabel" title="{bar.key}: {bar.text}">{bar.tick}</span>
        {/each}
      </div>
    </div>

    <!-- The series, spelled out: the bars carry their own labels, and this is
         what a screen reader reads instead of twenty unlabelled boxes. -->
    <p class="cd-sr">{label}: {series}</p>
  {/if}
</section>
