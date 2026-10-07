<!--
  COMPONENT STYLES — the design lab's choices, made by the student (2026-09-29).

  The lab draws three structurally different designs for each surface, and the
  decision this place serves is exactly that: **which design does each component
  wear?** Not "which one won" — all three stay, and the choice is per component,
  because a person who likes the quiet table may still want the loud session
  card. The list is read from `variants/catalog.ts`, so a surface appears here
  the day it exists and no second list is kept.

  TWO ROWS OVER THE LIST. *Interface size* moves every ported component at once
  (they measure themselves from `--ui-s`, `variants.css`), and *every component*
  hands the whole app one letter. Both are preferences with a few values, so
  both are radio groups, and each row wears the note of the value it is on — the
  choice is explained in words, never by colour alone.

  Nothing on this place writes the plan: these are the app's own preferences,
  stored beside the theme. A write lands immediately — there is no Save.
-->
<script lang="ts">
  import { CHOOSABLE, type SurfaceDef, type VariantId } from '../variants/catalog';
  import { styles, type UiSize } from '../variants/styles.svelte';

  const SIZES: Array<{ id: UiSize; name: string; note: string }> = [
    { id: 'compact', name: 'Compact', note: 'smaller rows and type — a small window, more on screen' },
    { id: 'regular', name: 'Regular', note: 'the lab’s own size — the drawings as they were reviewed' },
    { id: 'spacious', name: 'Spacious', note: 'larger rows and type — a big display, read from further away' },
  ];

  const GROUPS: Array<{ id: string; title: string; lead: string }> = [
    { id: 'day', title: 'The day', lead: 'Today, and the dated objects it shares with Mocks and Progress' },
    { id: 'study', title: 'The study', lead: 'Plan, Courses, Practice, Mocks, Reviews, Progress, Library and the record tables' },
    { id: 'machine', title: 'The machine', lead: 'System, the record panel, the palette, the session clock and this screen' },
  ];

  const LETTERS: VariantId[] = ['a', 'b', 'c'];

  const surfacesIn = (group: string): SurfaceDef[] => CHOOSABLE.filter((surface) => surface.group === group);

  const sizeNote = $derived(SIZES.find((size) => size.id === styles.size)?.note ?? '');

  /** The note of the design a surface is currently wearing — the row's own receipt. */
  function chosenNote(surface: SurfaceDef): string {
    const chosen = styles.variantOf(surface.key);
    return surface.variants.find((variant) => variant.id === chosen)?.note ?? '';
  }

  /** Whether every component that offers this letter is already wearing it. */
  function everyIs(id: VariantId): boolean {
    return CHOOSABLE.every(
      (surface) =>
        !surface.variants.some((variant) => variant.id === id) || styles.variantOf(surface.key) === id,
    );
  }
</script>

<div class="st-rows">
  <div class="st-row" data-plate="styles:size">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>Interface size</span>
      </span>
      <span class="st-row__help">
        One choice moves every row, pad, gap and type step together. Each component still reflows on
        its own window — <b>{sizeNote}</b>
      </span>
    </div>
    <div class="st-row__ctl">
      <span class="st-seg" role="radiogroup" aria-label="Interface size">
        {#each SIZES as size (size.id)}
          <button
            class="st-seg__pill"
            type="button"
            role="radio"
            aria-checked={styles.size === size.id}
            title={size.note}
            onclick={() => styles.setSize(size.id)}
          >
            {size.name}
          </button>
        {/each}
      </span>
    </div>
  </div>

  <div class="st-row" data-plate="styles:every">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>Every component</span>
      </span>
      <span class="st-row__help">
        Give every component that offers it the same design, or hand the whole app back to the lab's
        own defaults. Four components ship a single design and are left alone either way.
      </span>
    </div>
    <div class="st-row__ctl">
      <span class="st-seg" role="radiogroup" aria-label="Every component">
        {#each LETTERS as letter (letter)}
          <button
            class="st-seg__pill"
            type="button"
            role="radio"
            aria-checked={everyIs(letter)}
            aria-label={`Give every component its ${letter.toUpperCase()} design`}
            title={`Give every component its ${letter.toUpperCase()} design`}
            onclick={() => styles.setEveryVariant(letter)}
          >
            {letter.toUpperCase()}
          </button>
        {/each}
      </span>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => styles.resetVariants()}>
        The lab's defaults
      </button>
    </div>
  </div>
</div>

{#each GROUPS as group (group.id)}
  <h3 class="st-band">
    {group.title}
    <span class="st-band__count">
      {surfacesIn(group.id).length}
      {surfacesIn(group.id).length === 1 ? 'component' : 'components'}
    </span>
  </h3>
  <p class="st-band__lead">{group.lead}</p>

  <div class="st-rows">
    {#each surfacesIn(group.id) as surface (surface.key)}
      <div class="st-row" data-plate={`styles:${surface.key}`}>
        <div class="st-row__label">
          <span class="st-row__name">
            <span>{surface.title}</span>
          </span>
          <span class="st-row__help">
            {surface.what} — <b>{chosenNote(surface)}</b>
          </span>
        </div>
        <div class="st-row__ctl">
          <span class="st-seg" role="radiogroup" aria-label={`${surface.title} — which design`}>
            {#each surface.variants as variant (variant.id)}
              <!-- The pressed design is the one this component draws with; the
                   row's help line is that design's own idea in one line, so the
                   choice is explained without a preview nobody can see from
                   here. Title carries the note too, for the pointer. -->
              <button
                class="st-seg__pill st-pick"
                type="button"
                role="radio"
                aria-checked={styles.variantOf(surface.key) === variant.id}
                title={variant.note}
                onclick={() => styles.setVariant(surface.key, variant.id)}
              >
                <b>{variant.id.toUpperCase()}</b>
                <span>{variant.name}</span>
              </button>
            {/each}
          </span>
        </div>
      </div>
    {/each}
  </div>
{/each}
