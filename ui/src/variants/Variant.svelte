<!--
  THE VARIANT HOST (2026-09-29).

  One component the panels mount, so a screen never writes the same `{#if}` three
  times and a variant is never reachable from one screen and not another:

      <Variant surface="courses" {subjects} onopen={(id) => …} />

  It draws the style the student kept for that surface (`variants/styles`), with
  the props it was handed. It adds no wrapper element: a variant is a child of
  the screen's own grid, exactly as the shipped body was, so no layout depends
  on the host.
-->
<script lang="ts">
  import { componentFor } from './registry';
  import { styles } from './styles.svelte';

  let { surface, ...rest }: { surface: string } & Record<string, unknown> = $props();

  const chosen = $derived(componentFor(surface, styles.variantOf(surface)));
</script>

{#if chosen}
  {@const Drawing = chosen}
  <Drawing {...rest} />
{/if}
