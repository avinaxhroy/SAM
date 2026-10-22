<script lang="ts">
  /**
   * THE WORKING VEIL (§4.7). The layer is the app's: the scrim, the grain, the
   * pointer set, the z-index — and, when the machine will not run a field, the
   * two CSS sweeps. `shell/veil.ts` draws the light over them.
   *
   * Two layers, in the order the brief asked for them: the **texture** is a
   * still material (`shell/material.ts`), and the **light** flows across it
   * (`shell/veil.ts`). The field reads the material as a texture and multiplies
   * its own light by it, so the surface is only visible where it is lit — that
   * is what a surface under a light does, and it is also why the material can
   * never read as dust lying over the app. The canvas below only draws it when
   * there is no field to sample it, which is why `.cd-ai-veil[data-field="live"]
   * __weave` is transparent.
   *
   * `live` starts optimistic so the fallback sweeps never flash in front of a
   * field that is about to draw; a failure flips it in the same tick, before
   * the browser paints.
   */
  import { onMount } from 'svelte';
  import { materialTile, paintMaterial } from './material';
  import { mountVeil, type FieldState } from './veil';

  let field = $state<FieldState>('live');
  let canvas: HTMLCanvasElement | undefined;
  let weave: HTMLCanvasElement | undefined;

  onMount(() => {
    // One tile, two consumers: the field samples it as a texture, and the
    // fallback fills the layer with it when there is no field to sample it.
    const grain = materialTile();
    const material = weave ? paintMaterial(weave, grain) : undefined;
    if (!canvas) return material;
    const veil = mountVeil(canvas, grain, (state) => {
      field = state;
    });
    field = veil.state;
    return () => {
      veil.destroy();
      material?.();
    };
  });
</script>

<div class="cd-ai-veil" data-field={field} aria-hidden="true">
  <canvas class="cd-ai-veil__weave" bind:this={weave}></canvas>
  <canvas class="cd-ai-veil__field" bind:this={canvas}></canvas>
</div>
