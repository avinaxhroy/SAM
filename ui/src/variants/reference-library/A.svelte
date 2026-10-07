<!--
  REFERENCE · A · THE SHELF.
  Bookshelf variant where records stand as vertical book spines on a shelf.
  Selecting a spine opens an inline foldout displaying source details, filing
  metadata, and action buttons.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { cameFrom, type LibraryRecord, type ReferenceProps } from './props';
  import { safeUrl } from '../../safeUrl';

  let {
    heading,
    segments,
    segment,
    records,
    matches,
    query,
    newCommand,
    onSegment,
    onQuery,
    onClear,
    onAdd,
    onOpen,
  }: ReferenceProps = $props();

  /** The record whose page is standing on the shelf, and the fold in flight —
   *  the lab's `A_OPEN`, plus the one movement its `closeLeaf` owned. */
  let standing = $state<string | null>(null);
  let folding = $state(false);

  /** The lab's rule, as a derivation rather than a write during render: a
   *  filter that hides the standing page puts the spine back on the shelf. */
  const admitted = $derived(new Set(matches.map((record) => record.id)));
  const page = $derived(records.find((record) => record.id === standing && admitted.has(record.id)) ?? null);

  /** Where each spine stands, so focus can return to the place it left — and
   *  the seat a fold in flight is returning to, so a newer pull can take over
   *  before the older fold's own timer comes back for the focus. */
  const spines = new Map<string, HTMLButtonElement>();
  let seat: string | null = null;

  /** The lab's lean register: the spine's own place on the shelf, flat (±1.2°),
   *  never random. A drawing constant. */
  const LEANS = [0.9, -1.2, 0.6, -0.9, 1.2, -0.6, 1.0];

  function keepSeat(node: HTMLButtonElement, id: string) {
    spines.set(id, node);
    return {
      destroy() {
        spines.delete(id);
      },
    };
  }

  /** The fold is a movement, so the node leaves when the width reaches zero —
   *  the lab's `closeLeaf`, with its own 460ms backstop for a hidden tab. */
  function push(id: string) {
    folding = true;
    seat = id;
    window.setTimeout(() => {
      if (seat !== id) return;
      seat = null;
      folding = false;
      standing = null;
      spines.get(id)?.focus();
    }, 460);
  }

  function pull(record: LibraryRecord) {
    if (page?.id === record.id && !folding) {
      push(record.id);
      return;
    }
    seat = null;
    folding = false;
    standing = record.id;
  }

  function fold(event: TransitionEvent) {
    if (!folding || event.propertyName !== 'width' || event.target !== event.currentTarget) return;
    folding = false;
    standing = null;
  }

  /** The design's own root, so a window-level Escape still only acts on a key
   *  pressed inside this screen (the lab bound it to its own stage for the same
   *  reason). */
  let root: HTMLDivElement | undefined = $state();

  /** Escape puts a standing page back and returns focus to its spine. */
  function keys(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !page || !root?.contains(document.activeElement)) return;
    push(page.id);
  }

  /**
   * The unfold: the page is laid down at width 0 for exactly one frame before it
   * is released, which is the one reliable way to start a transition on a node
   * that was just inserted (the lab's own `unfold`).
   */
  function unfold(node: HTMLElement) {
    node.classList.add('a-leaf--pre');
    void node.offsetWidth;
    node.classList.remove('a-leaf--pre');
  }

  /** The notes whose own text is unfolded inside their standing page. */
  let noted = $state<string[]>([]);

  /** Clear empties the query and hands the caret back to the field (the lab's
   *  own behaviour: the control that emptied the filter is where you type next). */
  let field: HTMLInputElement | undefined = $state();

  /**
   * The spine's label, cut to fit the book it is printed on: a binary search on
   * the live element, because `text-overflow` emits nothing in `vertical-rl`
   * (the lab's `fitTitle`). It measures again whenever the box changes — a size
   * change is a new height, and a new height is a new length that fits — and
   * once the face has landed.
   */
  function fitSpine(node: HTMLElement, full: string) {
    let live = true;
    const measure = () => {
      if (!live) return;
      node.textContent = full;
      if (node.scrollHeight <= node.clientHeight + 1) return;
      let low = 0;
      let high = full.length;
      while (low < high) {
        const mid = (low + high + 1) >> 1;
        node.textContent = `${full.slice(0, mid).replace(/\s+$/, '')}…`;
        if (node.scrollHeight <= node.clientHeight + 1) low = mid;
        else high = mid - 1;
      }
      node.textContent = `${full.slice(0, low).replace(/\s+$/, '')}…`;
    };
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    void document.fonts.ready.then(measure);
    measure();
    return {
      update(next: string) {
        full = next;
        measure();
      },
      destroy() {
        live = false;
        observer.disconnect();
      },
    };
  }
</script>

<svelte:window onkeydown={keys} />

<div class="v-fit a-page" bind:this={root}>
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{heading}</h1>
    </div>
    <span class="cd-pagehead__aside">
      {#if segments.length > 1}
        <span class="cd-seg" role="group" aria-label="Which kind of saved thing to show">
          {#each segments as option (option.type)}
            <button
              class="cd-seg__pill"
              type="button"
              aria-pressed={option.type === segment}
              onclick={() => onSegment(option.type)}
            >
              {option.label}
            </button>
          {/each}
        </span>
      {/if}
      {#if newCommand}
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          data-command={newCommand}
          data-placement="today.screen"
          aria-label={`Add a ${segment ?? 'record'}`}
          onclick={onAdd}
        >
          <Icon name="plus" size={13} />
          Add
        </button>
      {/if}
    </span>
  </header>

  <div class="a-tools">
    <label class="cd-searchpill ref-find">
      <Icon name="search" size={14} />
      <input
        type="search"
        placeholder="Search saved things"
        aria-label="Search saved things"
        autocomplete="off"
        value={query}
        bind:this={field}
        oninput={(event) => onQuery(event.currentTarget.value)}
      />
    </label>
    <span class="a-tally">
      {#if query.trim().length > 0}
        <b class="num">{matches.length}</b> of <b class="num">{records.length}</b> match
      {:else}
        <b class="num">{records.length}</b> saved things
      {/if}
    </span>
    {#if query.trim().length > 0}
      <button
        class="ref-clear"
        type="button"
        onclick={() => {
          onClear();
          field?.focus();
        }}
      >
        <Icon name="close" size={12} />
        Clear
      </button>
    {/if}
  </div>

  <div class="a-board">
    <div class="a-run">
      {#each records as record, index (record.id)}
        <button
          class="a-spine"
          type="button"
          data-w={record.room}
          aria-expanded={page?.id === record.id}
          aria-label={`${record.title} — ${record.kind}, ${cameFrom(record)}`}
          style={`--h: calc(${record.spine.h}px * var(--ui-s)); --w: calc(${record.spine.w}px * var(--ui-s)); --lean: ${LEANS[index % LEANS.length]}deg; --tip: ${LEANS[index % LEANS.length] < 0 ? -3.5 : 3.5}deg`}
          use:keepSeat={record.id}
          onclick={() => pull(record)}
        >
          <span class="a-spine__t" aria-hidden="true" use:fitSpine={record.title}>{record.title}</span>
          <span class="a-spine__g" aria-hidden="true"><Icon name={record.glyph} size={16} /></span>
        </button>

        {#if page?.id === record.id}
          <div class="a-leaf" class:is-open={!folding} use:unfold ontransitionend={fold}>
            <div class="a-leaf__in" data-w={page.room}>
              <span class="a-leaf__top">
                <span class="a-mark"><Icon name={page.glyph} size={16} /></span>
                <span class="a-kind">{page.kind}</span>
              </span>
              <h2 class="a-leaf__t" title={page.title}>{page.title}</h2>
              <p class="a-leaf__f">{cameFrom(page)}</p>
              <span class="a-leaf__act">
                {#if safeUrl(page.url)}
                  <a
                    class="cd-pill cd-pill--sm cd-pill--quiet a-openlink"
                    href={page.url}
                    target="_blank"
                    rel="noreferrer"
                    title={page.url}
                  >
                    Open
                    <span class="cd-sr">— {page.title} opens in a new tab</span>
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 16 16"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      aria-hidden="true"
                      focusable="false"
                    ><path d="M4.6 11.4 11.4 4.6" /><path d="M6.2 4.6h5.2v5.2" /></svg>
                  </a>
                {:else if page.body}
                  <button
                    class="cd-pill cd-pill--sm cd-pill--quiet"
                    type="button"
                    aria-expanded={noted.includes(page.id)}
                    onclick={() =>
                      (noted = noted.includes(page.id)
                        ? noted.filter((id) => id !== page.id)
                        : [...noted, page.id])}
                  >
                    {noted.includes(page.id) ? 'Hide note' : 'Read note'}
                  </button>
                {:else}
                  <button
                    class="cd-pill cd-pill--sm cd-pill--quiet"
                    type="button"
                    data-command="record.panel"
                    data-placement="recordTable.rowContext"
                    onclick={() => onOpen(page.id)}
                  >
                    Open record
                  </button>
                {/if}
              </span>
              {#if page.body}
                <div class="a-note" data-open={noted.includes(page.id) ? '1' : '0'} inert={noted.includes(page.id) ? undefined : true}>
                  <div class="a-note__in"><p class="a-note__tx">{page.body}</p></div>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      {/each}

      {#if matches.length === 0}
        <p class="a-none">Nothing saved here matches “{query.trim()}”.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  /* The board. The shelf is a `--well` room; its plank is the system's own
     dashed rule, drawn as the run's own bottom edge. */
  .a-board {
    background: var(--well);
    border-radius: var(--r-tile);
    padding: var(--space-xl) var(--space-xl) 0;
  }
  .a-run {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-md);
    min-height: calc(232px * var(--ui-s));
    padding-bottom: var(--space-sm);
  }
  .a-run::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: var(--space-sm);
    border-top: 1px dashed var(--rule-strong);
  }

  /* The tools row: the field, the tally, and the Clear that exists only while
     there is something to clear. */
  .a-tools {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    margin-bottom: var(--space-lg);
  }
  .a-tally {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }
  .a-tally b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* A spine: a washed bar whose two sizes are its kind's registers (the width
     and height arrive inline, scaled by the size register), with room under it
     for the glyph and the label read up it. */
  .a-spine {
    position: relative;
    flex: none;
    display: grid;
    grid-template-rows: minmax(0, 1fr) calc(22px * var(--ui-s));
    justify-items: center;
    align-items: stretch;
    gap: var(--space-2xs);
    width: var(--w);
    height: var(--h);
    padding: var(--space-sm) 0 var(--space-2xs);
    border-radius: var(--r-key);
    background-color: var(--wash);
    background-image: var(--wash-grad);
    color: var(--onwash);
    box-shadow: var(--sh-1);
    transform-origin: bottom center;
    transform: rotate(var(--lean));
    transition: transform var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease);
  }
  /* Hover straightens the book and lifts it off the plank — a shelf's own
     grammar, and the raised z-index keeps a lifted book from being clipped by
     its neighbour. */
  .a-spine:hover {
    transform: translateY(-4px) rotate(0deg);
    box-shadow: var(--sh-2);
    z-index: 3;
  }
  .a-spine:active {
    transform: translateY(-1px) rotate(0deg);
    box-shadow: var(--sh-1);
  }
  .a-spine:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 3px;
  }
  /* Pulled: the book leans out of the shelf and stays where it stood. */
  .a-spine[aria-expanded='true'] {
    transform: translateY(-6px) rotate(var(--tip));
    box-shadow: var(--sh-2);
    z-index: 3;
  }
  .a-spine__t {
    align-self: stretch;
    min-height: 0;
    max-height: 100%;
    overflow: hidden;
    writing-mode: vertical-rl;
    text-orientation: mixed;
    transform: rotate(180deg);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    white-space: nowrap;
  }
  .a-spine__g {
    align-self: center;
  }

  /* The page, unfolding out of the spine's own place on the shelf: the run
     grows by its width and everything after it slides along the plank. The
     unfold is a transition on `is-open` (not a keyframe), so putting the page
     back is the same movement in reverse. */
  .a-leaf {
    --leaf-w: calc(360px * var(--ui-s));
    flex: none;
    /* `min-width: 0` is load-bearing: a flex item's automatic minimum size would
       hold the page at the width of its own contents and the unfold would jump
       instead of running. */
    width: 0;
    min-width: 0;
    max-width: 100%;
    opacity: 0;
    overflow: hidden;
    border-radius: var(--r-tile);
    background: var(--card);
    box-shadow: var(--sh-1);
    transition: width var(--dur-3) var(--ease), opacity var(--dur-2) var(--ease);
  }
  .a-leaf.is-open {
    width: var(--leaf-w);
    opacity: 1;
  }
  /* The pull applies this for exactly one frame before the page unfolds, so the
     browser has a computed width of 0 to travel from. It changes no transition
     of its own — the transition stays declared on `.a-leaf`. */
  .a-leaf:global(.a-leaf--pre) {
    width: 0;
    opacity: 0;
    transition: none;
  }
  .a-leaf__in {
    /* The page's own measure, so its contents never reflow while it unfolds —
       and never wider than the room it is unfolding into. */
    width: var(--leaf-w);
    max-width: 100%;
    padding: var(--space-md);
    display: grid;
    gap: var(--space-xs);
    justify-items: start;
    align-content: start;
  }
  .a-leaf__top {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .a-mark {
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--r-mini);
    background-color: var(--wash, var(--well-2));
    background-image: var(--wash-grad, none);
    color: var(--onwash, var(--ink-2));
    display: grid;
    place-items: center;
  }
  .a-kind {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .a-leaf__t {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.2;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .a-leaf__f {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .a-leaf__act {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }
  .a-openlink {
    gap: var(--space-2xs);
  }
  /* A note's own text, disclosed in place: the reference's accordion, one row. */
  .a-note {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-2) var(--ease);
  }
  .a-note[data-open='1'] {
    grid-template-rows: 1fr;
  }
  .a-note__in {
    min-height: 0;
    overflow: hidden;
  }
  .a-note__tx {
    padding-top: var(--space-sm);
    border-top: 1px dashed var(--rule-strong);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    line-height: 1.5;
  }
  .a-none {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* A shelf is a row that wraps, so the narrow answer is the one it already has.
     What changes is the page's own measure: a 360px page in a 320px room would
     be an overflow, so at a narrow container the page takes the room it has. */
  @container (max-width: 460px) {
    .a-board {
      padding: var(--space-md) var(--space-md) 0;
    }
  }

  /* base.css collapses every animation and transition to 1ms here; said again in
     this file so the movers are named where they are declared. */
  /* Four controls on one line is a line that runs out. The head wraps before it
     does, and the field takes the room that is left. */
  @container (max-width: 720px) {
    .cd-pagehead {
      flex-wrap: wrap;
      align-items: flex-start;
    }
    .cd-pagehead__aside {
      flex: 1 1 auto;
      flex-wrap: wrap;
      justify-content: flex-end;
    }
    .ref-find {
      width: auto;
      flex: 1 1 200px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .a-spine,
    .a-leaf,
    .a-note {
      transition: none;
    }
  }
</style>
