<!--
  The composer's sidebar (COMPOSER §4.8): it edits *records*, the *screen*, and
  states the *component* a press landed on — the screen's own acts while nothing
  is picked, the selected component's standing once a frame is pressed, and the
  picked record's edit form once a canvas row is pressed. The frame's own acts —
  drag, design arrows, width, remove — stay on the frame; this pane never
  mirrors them.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import ScreenState from './inspector/ScreenState.svelte';
  import ComponentState from './inspector/ComponentState.svelte';
  import RecordState from './inspector/RecordState.svelte';
  import '../styles/inspector.css';
</script>

<aside
  class="cmp-inspector"
  data-pane="inspector"
  data-inspector-state={app.picked ? 'record' : app.selectedSurface ? 'component' : 'screen'}
  aria-label="Editing settings"
>
  {#if app.picked}
    <RecordState id={app.picked.id} type={app.picked.type} />
  {:else if app.selectedSurface}
    <ComponentState surface={app.selectedSurface} />
  {:else}
    <ScreenState />
  {/if}
</aside>
