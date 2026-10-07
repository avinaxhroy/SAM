<!-- Record panel variant A: The Diagram. Drafting plate with dynamic SVG wire routing (D2). -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import Control from './Control.svelte';
  import { dayMonth, relative, stageWord, type FieldFact, type RecordPanelProps } from './props';
  import { rectOf, routeWire } from './wires';

  let {
    kind,
    address,
    today,
    quantities,
    dates,
    links,
    history,
    onSet,
    onOpen,
    onReveal,
    onDelete,
  }: RecordPanelProps = $props();

  /** The columns the box's face carries: the quantities, then the days. */
  const pads = $derived<FieldFact[]>([...quantities, ...dates]);

  /** The one editor that is open, if any — opening one folds the other. */
  let openKey = $state<string | null>(null);

  let grid = $state<HTMLElement | null>(null);
  let box = $state<HTMLElement | null>(null);
  let paths = $state<Array<{ to: string; d: string }>>([]);
  let frame = $state({ w: 0, h: 0 });
  let live = $state<string | null>(null);

  /** The reference's `calculate_path()`, on the boxes as they are now. */
  function route(): void {
    const element = grid;
    const from = box;
    const base = element?.getBoundingClientRect();
    if (!element || !from || !base || !base.width) return;
    frame = { w: Math.round(base.width), h: Math.round(base.height) };
    const origin = rectOf(from, base);
    const next: Array<{ to: string; d: string }> = [];
    for (const terminal of element.querySelectorAll('.rp-term[data-term]')) {
      const to = terminal.getAttribute('data-term') ?? '';
      const d = routeWire(origin, rectOf(terminal, base));
      if (d) next.push({ to, d });
    }
    paths = next;
  }

  $effect(() => {
    const element = grid;
    // Whatever moves a box re-routes the drawing: the relations, the pads, and
    // an editor opening (the box grows under it).
    void links.length;
    void pads.length;
    void openKey;
    if (!element) return;
    const observer = new ResizeObserver(() => route());
    observer.observe(element);
    const painted = requestAnimationFrame(route);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(painted);
    };
  });
</script>

<div class="rp-fit rp-panel">
  <section class="rp-plate" aria-label={`${kind}: the record and what it points at`}>
    <div class="rp-grid" bind:this={grid}>
      <svg class="rp-wires" aria-hidden="true" width={frame.w} height={frame.h}>
        {#each paths as path (path.to)}
          <path d={path.d} data-to={path.to} data-live={live === path.to ? 'true' : undefined} />
        {/each}
      </svg>

      <div class="rp-box" bind:this={box}>
        <div class="rp-box__top">
          <p class="rp-box__stamp">{kind}</p>
        </div>
        {#if pads.length > 0}
          <ul class="rp-pads">
            {#each pads as field (field.key)}
              <li class="rp-pad">
                <span class="rp-pad__k">
                  <IdPair label={field.label} value={field.key} title={`Copy the key “${field.key}”`} />
                  {#if field.derived}<span class="rp-fn" title="computed by the kind">ƒ</span>{/if}
                </span>
                {#if field.derived}
                  <!-- A derived value is computed by the kind and never written
                       (§3.4) — the plate is marked with its ƒ and has no press. -->
                  <span class="rp-pad__b">
                    <span class="rp-pad__v">{field.reading}</span>
                  </span>
                {:else}
                  <button
                    class="rp-pad__b"
                    type="button"
                    data-command="record.setField"
                    data-field-key={field.key}
                    aria-expanded={openKey === field.key}
                    aria-label={`${field.label}: ${field.reading} — activate to edit`}
                    onclick={() => (openKey = openKey === field.key ? null : field.key)}
                  >
                    <span class="rp-pad__v">{field.reading}</span>
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
          <div class="rp-pads__editors">
            {#each pads as field (field.key)}
              {#if !field.derived}
                <Control
                  {field}
                  {address}
                  {today}
                  open={openKey === field.key}
                  onSet={async (key, value) => {
                    const why = await onSet(key, value);
                    if (why === null) openKey = null;
                    return why;
                  }}
                />
              {/if}
            {/each}
          </div>
        {:else}
          <!-- A kind that publishes no column is stated, not faked. -->
          <p class="rp-after">This kind keeps no quantity and no day for the plate.</p>
        {/if}
      </div>

      {#if links.length === 0}
        <p class="rp-term rp-term--none">—</p>
      {:else}
        {#each links as link (link.key)}
          {#each link.targets as target (target.id)}
            {#if target.label}
              <button
                class="rp-term"
                type="button"
                data-term={target.id}
                aria-label={`Open ${target.label} — the record’s ${link.label.toLowerCase()}`}
                onpointerenter={() => (live = target.id)}
                onpointerleave={() => (live = null)}
                onfocus={() => (live = target.id)}
                onblur={() => (live = null)}
                onclick={() => onOpen(target.id, link.to)}
              >
                <span class="rp-term__s">{link.label}</span>
                {#if target.code}<span class="rp-term__code">{target.code}</span>{/if}
                <span class="rp-term__t">{target.label}</span>
              </button>
            {:else}
              <span class="rp-term rp-term--none">a link to something not here</span>
            {/if}
          {/each}
        {/each}
      {/if}
    </div>

    <div class="rp-titleblock">
      <p class="rp-sheet">
        <span class="rp-sheet__k">sheet</span>
        <span class="rp-sheet__v" data-developer>{address}</span>
      </p>
      {#if history}
        <ul class="rp-stamps">
          <li class="rp-stamp">
            <span class="rp-stamp__k">first</span>
            <span class="rp-stamp__v">
              {history.first ? `${dayMonth(history.first)} · ${relative(history.first, today)}` : '—'}
            </span>
          </li>
          <li class="rp-stamp">
            <span class="rp-stamp__k">reviewed</span>
            <span class="rp-stamp__v">
              {history.reviewed
                ? `${dayMonth(history.reviewed)} · ${history.rating ?? relative(history.reviewed, today)}`
                : 'not yet'}
            </span>
          </li>
          <li class="rp-stamp">
            <span class="rp-stamp__k">next</span>
            <span class="rp-stamp__v">
              {history.due
                ? `${dayMonth(history.due)} · ${relative(history.due, today)}${history.late > 0 ? ` · ${history.late} days late` : ''}`
                : 'nothing scheduled yet'}
            </span>
          </li>
        </ul>
        {#if history.rungs.length > 0}
          <ol class="rp-rungs">
            {#each history.rungs as rung (rung.name)}
              <li class="rp-rung" data-state={rung.state}>
                <span class="rp-rung__n">{rung.name}</span>
                <span class="rp-rung__s">{stageWord(rung.state)}</span>
              </li>
            {/each}
          </ol>
        {/if}
        {#if history.complete || history.asks.length > 0}
          <div class="rp-tb__say">
            {#if history.complete}
              <p class="rp-after">finished — the last stage is passed</p>
            {/if}
            {#if history.asks.length > 0}
              <p class="rp-after">waiting on {history.asks.join(' · ')}</p>
            {/if}
          </div>
        {/if}
      {/if}
      <div class="rp-tb__acts">
        <button class="rp-ink" type="button" data-command="record.reveal" onclick={onReveal}>
          <Icon name="code" size={13} />
          Edit as JSON
        </button>
        <button
          class="rp-quiet rp-danger"
          type="button"
          data-command="record.delete"
          onclick={onDelete}
        >
          <Icon name="close" size={13} />
          Delete this record…
        </button>
      </div>
    </div>
  </section>
</div>

<style>
  /* The panel's own head row: the stamp that names the kind, and the surface's
     design switch at the row's right end. The box's grid is untouched — the
     stamp stood on its own line and still does; the line is a row now. */
  .rp-box__top {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
  }
</style>
