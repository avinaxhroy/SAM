<!--
  Modal sheet container (Appendix C.9, D17, §11).

  Unified modal container for dialogs, forms, and sheets across the app:
    - Focus trap: the shared `modal` action (`shell/focusTrap.ts`) puts focus on
      the first field, cycles Tab/Shift+Tab inside the card, and hands the
      keyboard back to the triggering element on dismiss.
    - Dismissal: Handles Escape key and scrim clicks.
    - Accessibility: Configures `role="dialog"`, `aria-modal="true"`, and links `aria-labelledby`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  // The sheet chrome is its own stylesheet (`styles/sheets.css`); the palette
  // uses the same scrim, so both import it rather than relying on load order.
  import '../styles/sheets.css';
  // Focus placement, Tab cycling, Escape and focus return: the app's one modal
  // keyboard contract, shared with the re-readable room.
  import { modal } from './focusTrap';

  let {
    title,
    subtitle = '',
    onclose,
    children,
    head,
    footer,
  }: {
    title: string;
    subtitle?: string;
    onclose: () => void;
    children?: Snippet;
    /**
     * The door's own identity row, rendered under the title and above the
     * subtitle (`record-doors` A: the kind the door declares, and the command
     * key it dispatches, as an identity pair). A sheet without one is unchanged
     * — nothing else in the app passes it.
     */
    head?: Snippet;
    footer?: Snippet;
  } = $props();

  /** The title element's own id, so the dialog is named by the title it shows. */
  const titleId = $props.id();
  const named = $derived(title.trim().length > 0);

  /** The card: the boundary focus is placed in, cycled inside and measured by
      (`shell/focusTrap.ts` — the app's one keyboard contract for a modal). */
  let card = $state<HTMLElement | null>(null);

  /**
   * **Armed one tick after mount.** A sheet opened by a click mounts *inside*
   * that click's dispatch, so a window listener registered now would receive the
   * same event — with the opener as its target, which is outside the card — and
   * dismiss the sheet the student just asked for. The listener is therefore
   * inert until the opening event has finished propagating. (Found by driving
   * the real page with trusted input: the sheet mounted and unmounted within
   * 0.3 ms, so no click-opened sheet was reachable at all.)
   */
  let armed = $state(false);

  $effect(() => {
    const id = setTimeout(() => (armed = true), 0);
    return () => clearTimeout(id);
  });

  /**
   * The scrim dismisses by a window click tested for containment: a click
   * handler on a non-interactive div is a lint finding, and the scrim is not a
   * control. A click inside the card never reaches here.
   */
  function dismissOutside(event: MouseEvent): void {
    if (!armed) return;
    const target = event.target as Element | null;
    if (!target?.closest?.('.cd-sheet')) onclose();
  }
</script>

<svelte:window onclick={dismissOutside} />

<div class="cd-scrim" role="presentation" data-command="app.dismiss"></div>

<div
  class="cd-sheet"
  role="dialog"
  aria-modal="true"
  aria-labelledby={named ? titleId : undefined}
  aria-label={named ? undefined : 'Dialog'}
  tabindex="-1"
  bind:this={card}
  use:modal={{ onclose }}
>
  <header class="cd-sheet__head">
    <div>
      <h2 class="cd-sheet__title" id={titleId}>{title}</h2>
      {#if head}
        {@render head()}
      {/if}
      {#if subtitle}<p class="cd-sheet__sub">{subtitle}</p>{/if}
    </div>
    <button class="cd-iconbtn" type="button" onclick={onclose} aria-label="Close">✕</button>
  </header>

  <div class="cd-sheet__body">
    {@render children?.()}
  </div>

  {#if footer}
    <footer class="cd-sheet__foot">{@render footer()}</footer>
  {/if}
</div>
