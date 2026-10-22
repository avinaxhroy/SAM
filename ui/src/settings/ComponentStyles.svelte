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

  THE PER-COMPONENT CHOICE is three letters, not three sentences. A surface's
  three designs are named in the lab's own prose ("The Day’s Weight", "The
  Filter Strip"), and printing all three names inside the segment made a
  three-way choice 300px wide and two lines tall on every row. The chosen
  design's name rides beside the row's own title instead, and each letter
  carries its name and note as its accessible name and its pointer title.

  Nothing on this place writes the plan: these are the app's own preferences,
  stored beside the theme. A write lands immediately — there is no Save.
-->
<script lang="ts">
  import Row from './Row.svelte';
  import Segmented from './Segmented.svelte';
  import { CHOOSABLE, type SurfaceDef, type VariantId } from '../variants/catalog';
  import { styles, type UiSize } from '../variants/styles.svelte';

  const SIZES: Array<{ id: UiSize; name: string; note: string }> = [
    { id: 'compact', name: 'Compact', note: 'smaller rows and type — a small window, more on screen' },
    { id: 'regular', name: 'Regular', note: 'the lab’s own size — the drawings as they were reviewed' },
    { id: 'spacious', name: 'Spacious', note: 'larger rows and type — a big display, read from further away' },
  ];

  /* A band is its title and its count. It used to carry a lead sentence naming
     the components inside it — every one of which the rows below name again, in
     the same order, one line each. */
  const GROUPS: Array<{ id: string; title: string }> = [
    { id: 'day', title: 'The day' },
    { id: 'study', title: 'The study' },
    { id: 'machine', title: 'The machine' },
  ];

  const LETTERS: VariantId[] = ['a', 'b', 'c'];

  /** The glyph a band's rows wear: one per register, so a row's icon says which
   *  part of the app the component lives in before its name is read. */
  const GROUP_ICON: Record<string, string> = { day: 'calendar', study: 'book', machine: 'gear' };

  const surfacesIn = (group: string): SurfaceDef[] => CHOOSABLE.filter((surface) => surface.group === group);

  const sizeNote = $derived(SIZES.find((size) => size.id === styles.size)?.note ?? '');

  /** The design a surface is currently wearing — the row's own receipt. */
  function chosenName(surface: SurfaceDef): string {
    const chosen = styles.variantOf(surface.key);
    return surface.variants.find((variant) => variant.id === chosen)?.name ?? chosen.toUpperCase();
  }

  /** Whether every component that offers this letter is already wearing it. */
  function everyIs(id: VariantId): boolean {
    return CHOOSABLE.every(
      (surface) =>
        !surface.variants.some((variant) => variant.id === id) || styles.variantOf(surface.key) === id,
    );
  }

  /** The letter the whole app is on, or `null` when the surfaces are mixed. */
  const uniform = $derived(LETTERS.find((letter) => everyIs(letter)) ?? null);
</script>

<div class="st-rows">
  <Row icon="layout" name="Interface size" plate="styles:size">
    {#snippet help()}
      Moves every row, pad, gap and type step. <b>{sizeNote}</b>
    {/snippet}
    {#snippet ctl()}
      <Segmented
        label="Interface size"
        value={styles.size}
        options={SIZES.map((size) => ({ id: size.id, label: size.name, note: size.note }))}
        onpick={(id) => styles.setSize(id as UiSize)}
      />
    {/snippet}
  </Row>

  <Row icon="squareStack" name="Every component" plate="styles:every">
    {#snippet trail()}
      <span class="st-state">{uniform ? `all ${uniform.toUpperCase()}` : 'mixed'}</span>
    {/snippet}
    {#snippet help()}
      One design for every component, or the lab's defaults. Four ship a single design.
    {/snippet}
    {#snippet ctl()}
      <Segmented
        label="Every component"
        value={uniform ?? ''}
        mono
        options={LETTERS.map((letter) => ({
          id: letter,
          label: letter.toUpperCase(),
          aria: `Give every component its ${letter.toUpperCase()} design`,
          note: `Give every component its ${letter.toUpperCase()} design`,
        }))}
        onpick={(id) => styles.setEveryVariant(id as VariantId)}
      />
      <button
        class="st-chipbtn"
        type="button"
        title="Return every component to the design its lab ships by default"
        onclick={() => styles.resetVariants()}
      >
        The lab's defaults
      </button>
    {/snippet}
  </Row>
</div>

{#each GROUPS as group (group.id)}
  <h3 class="st-band">
    {group.title}
    <span class="st-band__count">
      {surfacesIn(group.id).length}
      {surfacesIn(group.id).length === 1 ? 'component' : 'components'}
    </span>
  </h3>
  <div class="st-rows">
    {#each surfacesIn(group.id) as surface (surface.key)}
      <Row icon={GROUP_ICON[group.id] ?? 'panel'} name={surface.title} plate={`styles:${surface.key}`}>
        {#snippet trail()}
          <span class="st-state">{chosenName(surface)}</span>
        {/snippet}
        {#snippet help()}
          {surface.what}
        {/snippet}
        {#snippet ctl()}
          <Segmented
            label={`${surface.title} — which design`}
            value={styles.variantOf(surface.key)}
            mono
            options={surface.variants.map((variant) => ({
              id: variant.id,
              label: variant.id.toUpperCase(),
              aria: `${surface.title} — ${variant.name}`,
              note: `${variant.name} — ${variant.note}`,
            }))}
            onpick={(id) => styles.setVariant(surface.key, id as VariantId)}
          />
        {/snippet}
      </Row>
    {/each}
  </div>
{/each}
