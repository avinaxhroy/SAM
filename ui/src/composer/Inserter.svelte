<!--
  THE COMPONENT CATALOG (COMPOSER §4.6).

  Twelve widgets, and the sheet is a *gallery*: each one is a drawing of the
  shape it draws on the screen, its name, and one line saying what it is for.
  The student is choosing a picture, so the picture leads.

  Five decisions:

  1. **The filter is one row of chips over the catalog's own bands.** "All" is
     the default, then one chip per group the registry declares — so the row can
     never offer a band that holds nothing, and a band that empties under a
     search drops its chip rather than sitting there dead.
  2. **A band says how many it holds, and folds.** Twelve cards in two bands is
     a scroll; the count is the band's receipt and the fold is how a student who
     has decided reads past the eight they are not choosing.
  3. **The card is one shape for every widget**: the drawing on its own plate,
     the name, the line, and the one control at the card's trailing edge. The
     control is a disc rather than a bar, because the card is a *choice* and a
     full-width bar under every card in the gallery would be twelve floors of
     eight identical buttons.
  4. **A widget already on this screen is a stated fact, not a dead control.**
     The card stays (the student may be looking for where it lives), the well
     says "On this screen", and the disc is gone.
  5. **Nothing here paints ink.** The frames, chips and drop line of the screen
     behind the sheet stay the only dark things on this screen, and the sheet's
     own commit is the one ink object the layer allows — there is none here.
-->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import Sheet from '../shell/Sheet.svelte';
  import Icon from '../shell/Icon.svelte';
  import { previewFor } from './previews';
  import { GROUP_LABELS, WIDGETS, type WidgetGroup } from './widgets/registry';

  /** The catalog's entry, read from the registry rather than restated here. */
  type Widget = (typeof WIDGETS)[number];

  const { used, onadd, onclose }: {
    /** The surfaces this screen already carries — shown, never offered again. */
    used: string[];
    /** Choosing a widget: the caller closes the sheet and writes the list. */
    onadd: (surface: string) => void;
    onclose: () => void;
  } = $props();

  /**
   * The bands, in the catalog's own order (COMPOSER.md §3.1: the day's widgets
   * first, then the study's). The ids are the registry's and the titles come
   * from `GROUP_LABELS`, so the sheet, the chips and the catalog cannot disagree
   * about what a group is called.
   */
  const GROUPS: WidgetGroup[] = [...new Set(WIDGETS.map((widget) => widget.group))];

  /** What the student typed. Substring, case-insensitive — the catalog is twelve rows. */
  let query = $state('');
  /** Which band the chips are filtering to; `''` is All. */
  let band = $state<WidgetGroup | ''>('');
  /** The bands the student folded shut, by id. */
  let folded = $state<Record<string, boolean>>({});

  /** The control the caret came from, so closing can hand it back. */
  let opener: HTMLElement | null = null;

  const onScreen = $derived(new Set(used));

  /**
   * The rows a search leaves. Matching reads the name, the note and the
   * surface key: the key is the one word the plan file and the CLI use, so a
   * student who has read `view.setComponents` finds a widget by it too (the
   * key is never *printed* — §5's census — only matched).
   */
  const found = $derived.by<Widget[]>(() => {
    const text = query.trim().toLowerCase();
    if (!text) return WIDGETS;
    return WIDGETS.filter(
      (widget) =>
        widget.label.toLowerCase().includes(text) ||
        widget.note.toLowerCase().includes(text) ||
        widget.surface.toLowerCase().includes(text),
    );
  });

  function inGroup(id: WidgetGroup): Widget[] {
    return found.filter((widget) => widget.group === id && (band === '' || band === id));
  }

  /** A band the search emptied loses its chip: the row offers what exists. */
  const chips = $derived(
    GROUPS.filter((id) => found.some((widget) => widget.group === id) || band === id),
  );

  function toggleFold(id: WidgetGroup): void {
    folded = { ...folded, [id]: !folded[id] };
  }

  /**
   * The field takes the keyboard when the sheet mounts, and the caret's
   * departure point is read here rather than in an effect: an effect runs after
   * the action, by which time this same action has already moved focus.
   */
  function firstField(node: HTMLInputElement): void {
    const active = document.activeElement;
    opener =
      active instanceof HTMLElement && active !== document.body
        ? active
        : document.querySelector<HTMLElement>('[data-opens="inserter"]');
    node.focus();
    node.select();
  }

  onDestroy(() => {
    // A detached opener (the screen was left, the bar was redrawn) takes
    // nothing: focus falls where the browser puts it rather than on a ghost.
    if (opener?.isConnected) opener.focus();
  });
</script>

<Sheet
  title="Add component"
  subtitle={`${WIDGETS.length} components — each one is one line on the screen`}
  icon="plus"
  wide
  {onclose}
>
  <div class="ci-find">
    <span class="ci-find__well">
      <Icon name="search" />
      <input
        class="ci-field"
        type="search"
        placeholder="Search components…"
        aria-label="Search components"
        autocomplete="off"
        spellcheck="false"
        bind:value={query}
        use:firstField
      />
    </span>
    <span class="cd-sheet__count" aria-hidden="true">{found.length}</span>
  </div>

  {#if chips.length > 1}
    <!-- The filter row: All, then one chip per band the search left something
         in. A chip is a choice, so it is a group of radios and never a row of
         buttons that look pressed. -->
    <div class="ci-chips" role="radiogroup" aria-label="Which band of the catalog">
      <button
        class="ci-chip"
        class:is-on={band === ''}
        type="button"
        role="radio"
        aria-checked={band === ''}
        onclick={() => (band = '')}
      >
        All
      </button>
      {#each chips as id (id)}
        <button
          class="ci-chip"
          class:is-on={band === id}
          type="button"
          role="radio"
          aria-checked={band === id}
          onclick={() => (band = id)}
        >
          {GROUP_LABELS[id]}
        </button>
      {/each}
    </div>
  {/if}

  {#if found.length === 0}
    <!-- Stated, not blank: the search that found nothing says so and says how
         to undo it. `role="status"` announces the empty result politely. -->
    <p class="cd-sheet__note ci-none" role="status">
      Nothing matches “{query.trim()}” — clear the search to see all {WIDGETS.length} components.
    </p>
  {:else}
    {#each GROUPS as group (group)}
      {@const rows = inGroup(group)}
      {#if rows.length > 0}
        <!-- One band per group: its heading, its count, its fold, then the
             cards. The band is the container the card grid measures itself
             against, so the gallery drops to one column on a sheet narrower
             than two cards need. -->
        <section class="ci-band">
          <button
            class="ci-band__head"
            type="button"
            aria-expanded={!folded[group]}
            onclick={() => toggleFold(group)}
          >
            <h3 class="ci-group">{GROUP_LABELS[group]}</h3>
            <span class="ci-band__n">
              {rows.length} {rows.length === 1 ? 'component' : 'components'}
            </span>
            <span class="ci-band__fold" aria-hidden="true">
              <Icon name="chevronDown" />
            </span>
          </button>
          {#if !folded[group]}
            <ul class="ci-list">
              {#each rows as widget (widget.surface)}
                <li class="ci-card" data-surface={widget.surface}>
                  <!-- The drawing, then the words. `{@html}` is safe and
                       deliberate: the markup is a constant in this build's own
                       `./previews.ts`, built from the numbers in that file, and
                       it carries no text, no attribute from the plan and no
                       caller input — it is the one kind of HTML that never
                       interpolates anything. -->
                  <span class="ci-pv" aria-hidden="true">{@html previewFor(widget.surface)}</span>
                  <span class="ci-cardtext">
                    <span class="ci-name">{widget.label}</span>
                    <span class="ci-note">{widget.note}</span>
                  </span>
                  {#if onScreen.has(widget.surface)}
                    <!-- Already here: the card stays — the student may be
                         looking for where it lives — and the disc's own column
                         carries a mark with the reason as its title. A
                         component appears once per screen, so there is no
                         second write to offer. -->
                    <span
                      class="ci-here"
                      title={`${widget.label} is already on this screen`}
                      aria-label={`${widget.label} is already on this screen`}
                      role="img"
                    >
                      <Icon name="check" />
                    </span>
                  {:else}
                    <button
                      class="ci-add"
                      type="button"
                      data-add={widget.surface}
                      aria-label={`Add ${widget.label}`}
                      onclick={() => onadd(widget.surface)}
                    >
                      <Icon name="plus" />
                    </button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    {/each}
  {/if}
</Sheet>

<style>
  /* ── the find ────────────────────────────────────────────────────────────
     The field is a *well with a glyph in it*, not a bare input: the magnifier
     is what says "this narrows the list" before the placeholder is read, and it
     is the one shape every search in this app already wears. The count sits
     outside the well at the row's trailing edge — it counts the list, not the
     field. */
  .ci-find {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .ci-find__well {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex: 1 1 auto;
    min-width: 0;
    height: var(--pill-h);
    padding: 0 var(--space-sm);
    border-radius: var(--r-pill);
    background: var(--well);
    color: var(--ink-3);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .ci-find__well:focus-within {
    background: var(--card);
    box-shadow: inset 0 0 0 1.5px var(--ink-4);
  }
  .ci-field {
    flex: 1 1 auto;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-xs);
  }
  .ci-field:focus { outline: none; }
  /* The field is `type="search"`; WebKit draws its own clear button inside it,
     which would sit beside nothing — this app's search wells have one control
     each. */
  .ci-field::-webkit-search-cancel-button { appearance: none; }

  /* ── the filter row ──────────────────────────────────────────────────────
     A chip is a *filter*, so it carries its state as a fill — unlike the
     segment, whose chosen option is the screen's one ink object. Two rules
     hold the pair apart: `is-on` is `--ink` text on the card with a ring, at
     rest it is `--ink-2` on the well. Never ink-filled: twelve cards and a
     lit chip is the ink budget this layer does not have. */
  .ci-chips {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    flex-wrap: wrap;
    margin-top: calc(var(--space-2xs) * -1);
  }
  .ci-chip {
    height: var(--hit);
    padding: 0 var(--space-sm);
    border-radius: var(--r-pill);
    background: var(--well);
    color: var(--ink-2);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    white-space: nowrap;
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  .ci-chip:hover { background: var(--well-2); color: var(--ink); }
  .ci-chip.is-on {
    background: var(--card);
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong);
  }

  /* ── the band ────────────────────────────────────────────────────────────
     The band is the container the grid measures itself against: the sheet's own
     width is fixed, but the window may not be, and a two-column gallery inside
     one card does not fit a narrow window. */
  .ci-band {
    container-type: inline-size;
    display: grid;
    gap: var(--space-xs);
    margin-top: var(--space-2xs);
  }
  /* The heading is the fold: a band of eight is a scroll, and a student who has
     decided wants to read past it. The whole line is the hit target. */
  .ci-band__head {
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
    width: 100%;
    padding: var(--space-3xs) var(--space-3xs) var(--space-2xs);
    text-align: left;
    color: var(--ink-3);
  }
  .ci-group {
    font-size: var(--text-xs);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    color: var(--ink);
  }
  .ci-band__n {
    margin-left: auto;
    font-size: var(--text-2xs);
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .ci-band__fold {
    display: grid;
    place-items: center;
    color: var(--ink-4);
    transition: transform var(--dur-2) var(--ease);
  }
  .ci-band__head[aria-expanded='false'] .ci-band__fold { transform: rotate(-90deg); }
  .ci-band__head:hover .ci-band__fold { color: var(--ink-2); }

  /* ── THE GALLERY ─────────────────────────────────────────────────────────
     One card per widget, and every card is the same object: the drawing on its
     plate at the left, the name and its line at the right, the control at the
     trailing edge. A card is a hairline and the sheet's own ground — no fill,
     because twelve filled cards is a wall of grey. */
  .ci-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2xs);
    grid-template-columns: minmax(0, 1fr);
  }
  /* Two columns, and the *sheet* is what makes them honest. Inside the 460px
     question sheet the card's own columns left the sentence under the name
     98px, and a two-line clamp in 98px holds about sixteen characters — so
     every card clipped its own note, whatever the grid did. At the gallery
     width (700px, `styles/sheets.css`) a card is 322px, the sentence gets
     ~218px, and the longest line in the catalog is two lines. One column
     remains the fallback for a window too narrow to hold two. */
  @container (min-width: 560px) {
    .ci-list { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }

  .ci-card {
    position: relative;
    display: grid;
    /* The drawing, then the words, then the control. The words take the slack
       and the *bounds* are the card's own: an `auto` control column is sized by
       its content, and at 460px the sheet's body holds two ~200px cards, so the
       disc and the gaps left the note 74px — a two-line clamp cannot hold
       "Everything the day holds, and what is done" in 74px, and every card in
       the gallery clipped its own sentence. The control is 32px of the card
       whatever it contains, the drawing is 56, and the rest is the note's. */
    grid-template-columns: calc(56px * var(--ui-s)) minmax(0, 1fr) var(--hit);
    align-items: center;
    gap: var(--space-2xs);
    padding: var(--space-2xs);
    border: 1px solid var(--rule);
    border-radius: var(--r-mini);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  /* The hover is a *boundary* change, not a lift: a lifted card in a 4px-gap
     grid casts onto its neighbour, and the gallery is one object rather than
     twelve floating ones. */
  .ci-card:hover {
    background: var(--card);
    border-color: var(--rule-strong);
  }
  /* The card whose disc was just pressed: the ring is the receipt, so the
     student sees which one they chose before the sheet closes under them. */
  .ci-card:focus-within { box-shadow: inset 0 0 0 1.5px var(--ink-4); }

  /* The drawing, held at a fixed shape whatever it contains: a wireframe is
     read by comparison, so it must be the card's constant, not its variable. */
  .ci-pv {
    display: block;
    width: 100%;
    aspect-ratio: 16 / 10;
    border-radius: var(--r-mini);
    background: var(--well);
    color: var(--ink-2);
    overflow: hidden;
  }
  .ci-pv :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }

  .ci-cardtext {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .ci-name {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink);
  }
  /* The catalog's line, at most two lines: it is a sentence about the widget,
     not its documentation, and a card that grew to four lines would break the
     grid it sits in. (`-webkit-line-clamp` is the working form of the clamp in
     this window — a WebKit view — and the standard property rides beside it.) */
  .ci-note {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    font-size: var(--text-2xs);
    line-height: 1.4;
    color: var(--ink-3);
  }

  /* The control: a disc at the card's trailing edge, out of the flow so the
     text column keeps the full width of everything above it. It is the only
     *raised* object in the gallery and never ink — the sheet's commit is the
     layer's one dark object. */
  .ci-add {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border-radius: var(--r-pill);
    background: var(--well-2);
    color: var(--ink);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  .ci-card:hover .ci-add { background: var(--card); box-shadow: var(--sh-1); }
  .ci-add:hover { background: var(--ink); color: var(--ink-inv); }
  .ci-add:active { transform: scale(0.94); }

  /* Already on this screen: the disc's column, holding the mark instead of the
     control — so the two states of a card line up down the gallery's trailing
     edge. Never a disabled disc: the card is not offering a write it cannot
     make, it is reporting one it already made. */
  .ci-here {
    display: grid;
    place-items: center;
    width: 100%;
    height: var(--hit);
    color: var(--ink-3);
  }

  .ci-none {
    padding: var(--space-lg) 0;
  }
</style>
