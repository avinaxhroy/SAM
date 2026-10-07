<!--
Specimen A of screen II (R11): the composer, one press wide.

Everything in it is the design floor's material, drawn where the app draws it —
`.cd-card` for the window's ground, `.cmp-bar` with the name of the screen being
edited and its two pills, `.cmp-stack`'s single cell, `.cmp-frame` with the
dashed edge of a component that is not being *read*, only moved, and the
catalog's card as `composer/Inserter.svelte` draws it: the structural preview in
a well, the name, the note, the Add pill — the words read from
`composer/widgets/registry.ts` rather than restated here, so the specimen cannot
drift from the sheet.

The beat is the whole of the teaching. The catalog's card stands over the cell
the component will take; its Add is pressed (the floor's own inset 2px ring,
worn as a state and not as a shadow); the card leaves; and the frame is simply
there. Nothing travels on a curve: a sheet is dismissed rather than thrown, and
a component does not slide in from off the screen — it is there the way a line
of the file is.

The frame's body holds the same structural drawing the catalog offered. The
app's own frame holds the live widget in that cell, and a widget host takes no
props because it derives its own facts from the plan (`widgets/registry.ts`) —
this room runs before a plan exists, which is why the drawing stands in for it
and every word in the card is a constant.

Nothing here is a control. Each part is a span wearing the class of the button
it stands for, the card carries `aria-hidden` whole, and the three captions
under it are what a screen reader reads.
-->
<script lang="ts">
  import { previewFor } from '../composer/previews';
  import { WIDGETS } from '../composer/widgets/registry';

  /** The component this beat adds, read from the catalog's own row. The
      registry is a list, so the lookup is guarded rather than asserted: the
      guard is the type's, not a state this room can be in. */
  const pick = WIDGETS.find((widget) => widget.surface === 'week-chart');
</script>

<div class="ob-demo">
  <section class="cd-card">
    <header class="cmp-bar" aria-hidden="true">
      <p class="cmp-bar__where">
        <span class="cmp-bar__label">Editing</span>
        <span class="cmp-bar__title">Today</span>
      </p>
      <span class="cmp-bar__spacer"></span>
      <span class="cd-pill cd-pill--quiet cd-pill--sm">Add component</span>
      <span class="cd-pill cd-pill--quiet cd-pill--sm cmp-bar__done">Done</span>
    </header>

    <div class="cmp-stack ob-stack" aria-hidden="true">
      <div class="ob-slot">
        <div class="cmp-frame">
          <div class="cmp-frame__bar">
            <span class="cmp-handle">⠿</span>
            <span class="cmp-frame__name">Week chart</span>
            <span class="cmp-frame__spacer"></span>
            <span class="cd-pill cd-pill--quiet cd-pill--sm cmp-width">Half width</span>
            <span class="cd-iconbtn cmp-remove">✕</span>
          </div>
          <div class="ob-body">{@html previewFor('week-chart')}</div>
        </div>

        {#if pick}
          <div class="ob-pick">
            <span class="ob-pick__pv">{@html previewFor(pick.surface)}</span>
            <span class="ob-pick__name">{pick.label}</span>
            <span class="ob-pick__note">{pick.note}</span>
            <span class="cd-pill cd-pill--quiet cd-pill--sm ob-pick__add">Add</span>
          </div>
        {/if}
      </div>
    </div>

    <div class="ob-caps">
      <span class="ob-cap" data-key="1"><b>pick</b> the catalog offers one drawing for each component</span>
      <span class="ob-cap" data-key="2"><b>press</b> Add, and the component stands on the screen</span>
      <span class="ob-cap" data-key="3"><b>order</b> components stand in the order of the list</span>
    </div>
  </section>
</div>
