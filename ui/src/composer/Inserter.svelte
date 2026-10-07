<!-- Component inserter sheet presenting the widget catalog (COMPOSER §4.6). -->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import Sheet from '../shell/Sheet.svelte';
  import { previewFor } from './previews';
  import { GROUP_LABELS, WIDGETS } from './widgets/registry';

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
   * The sheet's two groups, in the catalog's own order (COMPOSER.md §3.1:
   * the day's widgets first, then the study's). The ids are the registry's and
   * the titles come from `GROUP_LABELS`, so the sheet and the catalog cannot
   * disagree about what a group is called.
   */
  const GROUPS: Array<{ id: 'day' | 'study' }> = [{ id: 'day' }, { id: 'study' }];

  /** What the student typed. Substring, case-insensitive — the catalog is twelve rows. */
  let query = $state('');

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

  function inGroup(id: string): Widget[] {
    return found.filter((widget) => widget.group === id);
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
  {onclose}
>
  <div class="ci-find">
    <input
      class="cd-sheet__well ci-field"
      type="search"
      placeholder="Search components"
      aria-label="Search components"
      autocomplete="off"
      spellcheck="false"
      bind:value={query}
      use:firstField
    />
    <span class="cd-sheet__count" aria-hidden="true">{found.length}</span>
  </div>

  {#if found.length === 0}
    <!-- Stated, not blank: the search that found nothing says so and says how
         to undo it. `role="status"` announces the empty result politely. -->
    <p class="cd-sheet__note ci-none" role="status">
      Nothing matches “{query.trim()}” — clear the search to see all {WIDGETS.length} components.
    </p>
  {:else}
    {#each GROUPS as group (group.id)}
      {@const rows = inGroup(group.id)}
      {#if rows.length > 0}
        <!-- One band per group: the heading, then the cards. The band is the
             container the card grid measures itself against, so the gallery
             drops to one column on a sheet narrower than two cards need. -->
        <section class="ci-band">
          <h3 class="ci-group">{GROUP_LABELS[group.id]}</h3>
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
                <span class="ci-name">{widget.label}</span>
                <span class="ci-note">{widget.note}</span>
                {#if onScreen.has(widget.surface)}
                  <!-- Already here: the card stays, the control states why it
                       cannot be used (a component appears once per screen). -->
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm ci-add"
                    type="button"
                    disabled
                    aria-label={`${widget.label} is already on this screen`}
                  >
                    On this screen
                  </button>
                {:else}
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm ci-add"
                    type="button"
                    data-add={widget.surface}
                    onclick={() => onadd(widget.surface)}
                  >
                    Add
                    <span class="cd-sr">{widget.label}</span>
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/each}
  {/if}
</Sheet>

<style>
  /* The field and its count share a line: the count is the search's own
     receipt, and it is `aria-hidden` because the group headings below already
     speak the cards by name. */
  .ci-find {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-xs);
  }
  .ci-field {
    flex: 1 1 auto;
    min-width: 0;
  }
  /* The ring is drawn INSIDE the field, and it has to be. The field is the
     first thing in the sheet body, whose own `overflow: auto` is what makes a
     long list scrollable — and a scroll container clips everything painted
     outside its edge. `design/base.css` rings at `outline-offset: 2px`, so the
     outward ring lost its left side (and the top of its curve) the moment the
     field took focus: the border was cut, on the side. An inset ring shares the
     field's own box and cannot be clipped by anything but the field, at any
     width, in either theme, at either size register. Both `:focus` and
     `:focus-visible` — a field the pointer clicked into wants its ring too, the
     same call `reviews.css` makes for `.rv__input`. */
  .ci-field:focus,
  .ci-field:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  /* The band is the container the grid measures itself against: the sheet's own
     width is fixed, but the window may not be, and a two-column gallery inside
     one 460px card does not fit a phone. */
  .ci-band {
    container-type: inline-size;
  }
  .ci-group {
    margin-bottom: var(--space-xs);
    font-size: var(--text-sm);
    font-weight: var(--weight-display);
    color: var(--ink);
  }

  /* ── THE GALLERY ─────────────────────────────────────────────────────────
     The cards are the app's own object at its smallest: a hairline edge, the
     card's ground, and the four facts stacked in the order the eye reads them
     — the shape, the name, the line, the control. Nothing here paints ink: the
     frames, chips and drop line of the screen behind the sheet stay the only
     dark things, and the one ink object on this screen is still the sheet's
     footer (there is none here), so the gallery stays quiet. */
  .ci-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-sm);
    grid-template-columns: minmax(0, 1fr);
  }
  /* Two columns the moment the band can hold two cards: the 460px sheet's
     body is 412px, so this is the normal case, and one column is what a
     narrowed sheet falls back to. */
  @container (min-width: 330px) {
    .ci-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  .ci-card {
    display: grid;
    /* The Add control takes the slack, so every card in a row offers its
       control on the same baseline even when one note wraps and another
       does not. */
    grid-template-rows: auto auto 1fr auto;
    gap: var(--space-2xs);
    padding: var(--space-xs);
    border: 1px solid var(--rule);
    border-radius: var(--r-mini);
  }

  /* The drawing, held at a fixed shape whatever it contains: a wireframe is
     read by comparison, so it must be the card's constant, not its variable. */
  .ci-pv {
    display: block;
    aspect-ratio: 16 / 9;
    border-radius: var(--r-mini);
    background: var(--well);
    color: var(--ink-2);
  }
  .ci-pv :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }

  .ci-name {
    font-size: var(--text-xs);
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
    line-height: 1.45;
    color: var(--ink-3);
  }

  /* The control is the card's floor, and it is the app's own quiet pill at the
     one hit height the floors allow — stretched to the card, because a gallery
     card's button is where the hand already is. */
  .ci-add {
    width: 100%;
    justify-content: center;
  }

  .ci-none {
    padding: var(--space-lg) 0;
  }
</style>
