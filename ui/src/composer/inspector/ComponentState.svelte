<!--
  The sidebar's component state (COMPOSER §4.8): displays the selected frame's
  component name and variant. Read-only by design: per-instance controls stay on
  the frame (drag, variant toggle, width, remove) and are not mirrored here
  (COMPOSER §3.1, §4.8).
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { app } from '../../session.svelte';
  import { SURFACE_BY_KEY } from '../../variants/catalog';
  import { styles } from '../../variants/styles.svelte';
  import { widgetLabel, widgetNote } from '../widgets/registry';

  let { surface }: { surface: string } = $props();

  const label = $derived(widgetLabel(surface));
  const note = $derived(widgetNote(surface));

  const components = $derived(app.screenComponents);
  const at = $derived(components.findIndex((component) => component.surface === surface));
  const span = $derived(components[at]?.span ?? 2);
  const position = $derived(at >= 0 ? at + 1 : null);
  const total = $derived(components.length);

  /** The design it is wearing, by the id the frame prints (`A`, `B`, `C`). */
  const chosen = $derived(styles.variantOf(surface));
  const design = $derived(
    SURFACE_BY_KEY[surface]?.variants.find((variant) => variant.id === chosen) ?? null,
  );

  const designLine = $derived(
    design ? `Design ${design.id.toUpperCase()} — ${design.name}` : 'Its own design',
  );
</script>

<header class="cmp-insp__head">
  <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
    <Icon name="layout" />
  </span>
  <div class="cmp-insp__who">
    <p class="cmp-insp__eyebrow">Component</p>
    <h2 class="cmp-insp__title">{label}</h2>
    {#if note}<p class="cmp-insp__sub">{note}</p>{/if}
  </div>
  <div class="cmp-insp__headacts">
    <!-- Back to the screen's own settings, without putting the pane away: a
         selected component is a place the pane can be in, not the pane's only
         subject. -->
    <button
      class="cd-iconbtn"
      type="button"
      aria-label="Back to the screen’s own settings"
      title="Back to the screen’s own settings"
      onclick={() => app.unpickRecord()}
    >
      <Icon name="chevronleft" />
    </button>
    <button
      class="cd-iconbtn"
      type="button"
      data-command="screen.inspector"
      data-placement="composer.inspector"
      aria-label="Close the editing sidebar"
      title="Put this pane away — press any component to bring it back"
      onclick={() => app.closeInspector()}
    >
      <Icon name="close" />
    </button>
  </div>
</header>

<section class="cmp-insp__group" aria-label="This component">
  <p class="cmp-insp__eyebrow">On this screen</p>
  {#if position !== null}
    <div class="cmp-insp__state">
      <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
        <Icon name="pointer" />
      </span>
      <span class="cmp-insp__acttext">
        <span class="cmp-insp__actname">Component {position} of {total}</span>
        <span class="cmp-insp__actsay">Drag the frame to move it in the stack.</span>
      </span>
    </div>
  {/if}
  <div class="cmp-insp__state">
    <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
      <Icon name="palette" />
    </span>
    <span class="cmp-insp__acttext">
      <span class="cmp-insp__actname">{designLine}</span>
      <span class="cmp-insp__actsay">The arrows beside the component switch its design.</span>
    </span>
  </div>
  <div class="cmp-insp__state">
    <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
      <Icon name="layout" />
    </span>
    <span class="cmp-insp__acttext">
      <span class="cmp-insp__actname">{span === 1 ? 'Half the row' : 'The whole row'}</span>
      <span class="cmp-insp__actsay">The width pill on the frame switches it.</span>
    </span>
  </div>
</section>

<p class="cmp-insp__tip">
  The frame carries every gesture this component has: drag it, step its design with the
  arrows, switch its width with the pill, or press ✕ to take it off the screen.
</p>

<section class="cmp-insp__group" aria-label="Back">
  <button class="cmp-insp__act" type="button" onclick={() => app.unpickRecord()}>
    <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true"><Icon name="chevronleft" /></span>
    <span class="cmp-insp__acttext">
      <span class="cmp-insp__actname">Back to the screen</span>
      <span class="cmp-insp__actsay">Leave this component and edit the screen’s own settings</span>
    </span>
  </button>
</section>
