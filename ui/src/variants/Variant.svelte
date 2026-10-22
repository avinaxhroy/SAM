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
  import type { Component } from 'svelte';
  import { loadComponent } from './registry';
  import { styles } from './styles.svelte';

  let { surface, ...rest }: { surface: string } & Record<string, unknown> = $props();

  // The design is a chunk: the first mount of a surface loads it, and a
  // variant switch loads the other design without putting every design in the
  // boot bundle. The effect re-runs when the store moves at all; a load that
  // resolves for a design no longer wanted is dropped by comparing ids.
  let chosen = $state<Component<Record<string, unknown>> | null>(null);
  /** The design `chosen` was loaded for — and the one every pending load is
   *  checked against. */
  let loaded: string | null = null;
  let wantedId: string | null = null;
  $effect(() => {
    const wanted = surface;
    const variant = styles.variantOf(surface);
    const id = `${wanted}:${variant}`;
    // Another surface's choice was made: this host's design did not change, so
    // it keeps drawing. Emptied and reloaded on every write, a host remounted
    // its whole subtree — the settings screen lost the place it was on every
    // time a variant was pressed.
    if (id === loaded) return;
    wantedId = id;
    chosen = null;
    void loadComponent(wanted, variant).then((component) => {
      // A load that lands after the choice moved on is not this host's answer.
      if (wantedId !== id) return;
      chosen = component;
      loaded = id;
    });
  });
</script>

{#if chosen}
  {@const Drawing = chosen}
  <Drawing {...rest} />
{/if}
