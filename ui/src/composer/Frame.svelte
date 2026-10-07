<!-- Component editor wrapper frame with layout controls (COMPOSER §4.3). -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { app } from '../session.svelte';
  import Icon from '../shell/Icon.svelte';
  import { widgetLabel } from './widgets/registry';
  import { SURFACE_BY_KEY } from '../variants/catalog';
  import { styles } from '../variants/styles.svelte';
  import type { Reorder } from './reorder.svelte';

  let {
    surface,
    index,
    span,
    reorder,
    announce,
    body,
  }: {
    /** The widget's surface key, as the plan stores it. */
    surface: string;
    /** Its place in the stack. */
    index: number;
    /** How wide it stands in the stack's two columns: 1 half, 2 the full row. */
    span: 1 | 2;
    /** The stack's drag engine — the frame is its pointer, not its owner. */
    reorder: Reorder;
    /** Say something to the polite live region the screen owns. */
    announce: (text: string) => void;
    /**
     * How the component is drawn: the mount is the screen's, so the same
     * snippet draws the widget in view mode and inside the frame.
     */
    body: Snippet<[string]>;
  } = $props();

  /**
   * What a press has to miss to be a drag: the frame's own controls, and
   * anything a press already means something else to — a link, a field, an open
   * menu. The panel is a species of its own: `role="menu"` sits on its inner
   * list, so its padding and its footer sentence would otherwise walk up to the
   * frame and become a drag. The preview is inert, so a press over it always
   * reaches the frame.
   */
  const CONTROLS = 'button, a, input, [role="menu"], .cd-menu';

  const label = $derived(widgetLabel(surface));
  /** The surface's designs, in the lab's own order; none or one is a stated fact (§4.5). */
  const designs = $derived(SURFACE_BY_KEY[surface]?.variants ?? []);
  const chosen = $derived(styles.variantOf(surface));
  /** Two designs or more: the arrows and the chip have something to switch to. */
  const switchable = $derived(designs.length > 1);
  const total = $derived(app.screenSurfaces.length);
  const lifted = $derived(reorder.from === index);
  /** This frame's own node: the engine names the landing frame by element, since
   *  the index it was dragged from is not the index it lands on. */
  let node = $state<HTMLElement | null>(null);
  /** The landing this frame is in the middle of, if it is the one that landed. */
  const landing = $derived(reorder.settle !== null && reorder.settle.frame === node ? reorder.settle : null);
  /** Whether the landing translation animation is actively settling to zero. */
  const gliding = $derived(landing?.glide === true);
  /** Transform offset during active drag or landing transition. */
  const offset = $derived(
    lifted ? { x: reorder.dx, y: reorder.dy } : landing ? { x: landing.x, y: landing.y } : null,
  );

  /** The design one step from the kept one, wrapping at both ends (A→B→C→A). */
  function designAt(by: number) {
    const at = designs.findIndex((variant) => variant.id === chosen);
    if (at < 0 || designs.length === 0) return null;
    return designs[(at + by + designs.length) % designs.length] ?? null;
  }
  const previous = $derived(designAt(-1));
  const next = $derived(designAt(1));

  /**
   * The arrow's own sentence: the design it would switch to, or why it cannot.
   * A surface with one design would "wrap" onto itself, so it gets the sentence,
   * not a title promising a switch to the design already showing.
   */
  function arrowTitle(target: { name: string } | null): string {
    return switchable && target ? `Switch to ${target.name}` : 'This component has no other design';
  }

  /**
   * The width the frame is wearing, and the press that moves it (owner's call,
   * 2026-09-30). A widget stands at one of the stack's two widths — half the
   * screen, or all of it — so the gesture is one press, not a drag: the pill
   * states the width it will **switch to**, exactly as the rail's theme disc
   * states the mode it will switch to, and the write is the same single
   * `view.setComponents` every other frame gesture ends in (one list, spans and
   * all, so an Undo brings back the width with it).
   */
  function resize(): void {
    const next: 1 | 2 = span === 1 ? 2 : 1;
    void app.setScreenComponents(
      app.screenComponents.map((component) =>
        component.surface === surface ? { surface: component.surface, span: next } : component,
      ),
    );
    announce(`${label}, ${next === 1 ? 'half the screen' : 'the full width'}`);
  }

  /**
   * One press, one design, no write: the fast path through a surface's designs.
   * `styles.setVariant` is the same preference Settings › Component styles keeps
   * (§4.5) — nothing about the plan moves, and a disabled arrow is the whole
   * statement a one-design surface needs.
   */
  function step(by: number): void {
    if (!switchable) return;
    const target = designAt(by);
    if (!target) return;
    styles.setVariant(surface, target.id);
    announce(`${label}, design ${target.id.toUpperCase()} — ${target.name}.`);
  }

  /** Native `inert` support check with fallback for older environments. */
  const INERT = typeof HTMLElement !== 'undefined' && 'inert' in HTMLElement.prototype;

  /**
   * The fallback, completed. Without `inert` the CSS takes the pointer away
   * (`.cmp-frame__body`) and this takes the tab stops: a widget is full of real
   * controls (Start, Reveal, Log…), and a frame a keyboard can Tab into and
   * press Enter is exactly the failure the mechanism exists to prevent. The
   * wrapper's `aria-hidden` keeps the preview out of the tree; every parked
   * control is restored on destroy.
   */
  function inertByHand(node: HTMLElement, on: boolean) {
    if (!on) return { destroy() {} };
    const parked: Array<[HTMLElement, string | null]> = [];
    for (const el of node.querySelectorAll<HTMLElement>('a[href], button, input, select, textarea, [tabindex]')) {
      parked.push([el, el.getAttribute('tabindex')]);
      el.tabIndex = -1;
    }
    return {
      destroy() {
        for (const [el, previous] of parked) {
          if (previous === null) el.removeAttribute('tabindex');
          else el.setAttribute('tabindex', previous);
        }
      },
    };
  }

  /** One move, one write: the list goes through the session's single path. */
  async function moveTo(target: number): Promise<void> {
    const list = app.screenSurfaces;
    if (target < 0 || target >= list.length || target === index) return;
    const next = [...list];
    const [moved] = next.splice(index, 1);
    next.splice(target, 0, moved);
    await app.setScreenSurfaces(next);
    announce(`${label}, position ${target + 1} of ${list.length}`);
  }

  /** Removal is the app's own receipt with its Undo, and focus lands next door. */
  async function remove(): Promise<void> {
    const list = app.screenSurfaces;
    const rest = list.filter((_, at) => at !== index);
    await app.setScreenSurfaces(rest);
    announce(`${label} removed. ${rest.length} of ${list.length} left.`);
    // The frame that took its place — or the last one — takes the focus, so a
    // keyboard student is never dropped back at the top of the window. When the
    // screen is now empty there is no frame at all: the empty state's own door
    // takes it, else the bar's Done, else nothing (the element that had focus
    // was destroyed by the write, and the body is not a place).
    const frames = document.querySelectorAll<HTMLElement>('[data-frame]');
    const next = frames[Math.min(index, frames.length - 1)];
    if (next) next.focus();
    else {
      document
        .querySelector<HTMLElement>('.cmp-screen .cd-empty [data-opens="inserter"], .cmp-bar__done')
        ?.focus();
    }
  }

  /**
   * Starts drag-and-drop reordering when dragging the frame bar or inert preview body.
   * Clicks on nested controls are ignored, and movement must exceed the drag threshold.
   */
  function onpointerdown(event: PointerEvent): void {
    if (event.target instanceof Element && event.target.closest(CONTROLS)) return;
    reorder.begin(index, event);
  }

  function onkeydown(event: KeyboardEvent): void {
    // Only the frame itself: its own controls keep their native key behaviour
    // (a menu item's arrows are the menu's), and the frame is the tab stop that
    // makes this gesture reachable.
    if (event.target !== event.currentTarget) return;
    // Left and right are the designs, up and down are the place in the stack:
    // the two axes the frame draws — the arrows flanking it, and the drag.
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      if (!switchable) return;
      event.preventDefault();
      step(event.key === 'ArrowLeft' ? -1 : 1);
      return;
    }
    const last = total - 1;
    const target =
      event.key === 'ArrowUp'
        ? Math.max(0, index - 1)
        : event.key === 'ArrowDown'
          ? Math.min(last, index + 1)
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? last
              : null;
    if (target === null) return;
    event.preventDefault();
    void moveTo(target);
  }
</script>

<!--
  The frame is focusable on purpose (COMPOSER §4.3, §5): it is the tab stop the
  arrow-key reorder lives on, and `role="region"` with the widget's name is what
  it actually is — a labelled region of the screen, not a control. Svelte's
  rules cannot tell that apart from a stray `tabindex` on a div, so the two
  findings are silenced here rather than answered with a wrong role: a
  `role="button"` would promise an activation this frame does not have.
-->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="cmp-frame"
  class:is-lifted={lifted}
  class:is-settling={gliding}
  style:--cmp-span={span}
  style:transform={offset ? `translate(${offset.x}px, ${offset.y}px)` : null}
  bind:this={node}
  data-frame
  data-surface={surface}
  role="region"
  tabindex="0"
  aria-label={label}
  onkeydown={onkeydown}
  onpointerdown={onpointerdown}
>
  <div class="cmp-frame__bar">
    <button
      class="cmp-handle"
      type="button"
      aria-label={`Reorder ${label}`}
      title={`Reorder ${label} — drag, or use the arrow keys`}
      data-command="view.setComponents"
      data-placement="composer.frame"
      onpointerdown={(event) => reorder.begin(index, event)}
    >⠿</button>

    <!-- The design carousel (§4.5), redrawn where the owner asked for it: the
         two arrows flank the component itself — `‹ [component] ›`, outside it
         on both sides and centred on it — so stepping through the designs
         reads as moving the object sideways. The bar keeps the *name* of the
         design it is wearing as a label; the middle is never a control, and a
         surface with one design states the fact instead of offering a switch
         (`disabled`, with the reason in the title). -->
    <span class="cmp-frame__name">
      {label}<span class="cmp-frame__design">{switchable ? ` · ${chosen.toUpperCase()}` : ''}</span>
    </span>

    <span class="cmp-frame__spacer"></span>

    <!-- The width control: the frame's one sizing act, and its label is the
         width the press will give the component (`Half width` while it stands
         full, `Full width` while it stands half) — the same forward-stating
         rule the rail's theme disc follows. -->
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm cmp-width"
      type="button"
      aria-label={`${span === 1 ? 'Full width' : 'Half width'} for ${label}`}
      title={span === 1 ? 'Give it the whole row' : 'Half the row — two components can stand side by side'}
      data-command="view.setComponents"
      data-placement="composer.frame"
      onclick={resize}
    >{span === 1 ? 'Full width' : 'Half width'}</button>

    <!-- Removal is on the surface, not in a menu: the act the student came to
         the frame to do should be the act they can see. -->
    <button
      class="cd-iconbtn cmp-remove"
      type="button"
      aria-label={`Remove ${label}`}
      title={`Remove ${label} from this screen`}
      data-command="view.setComponents"
      data-placement="composer.frame"
      onclick={() => void remove()}
    >✕</button>
  </div>

  <button
    class="cd-iconbtn cmp-step cmp-step--prev"
    type="button"
    aria-label={`Previous design of ${label}`}
    title={arrowTitle(previous)}
    disabled={!switchable}
    onclick={() => step(-1)}
  ><Icon name="chevronleft" size={15} /></button>

  <div
    class="cmp-frame__body"
    {...INERT ? { inert: true } : { tabindex: -1 }}
    use:inertByHand={!INERT}
    aria-hidden={INERT ? undefined : 'true'}
  >
    {@render body(surface)}
  </div>

  <button
    class="cd-iconbtn cmp-step cmp-step--next"
    type="button"
    aria-label={`Next design of ${label}`}
    title={arrowTitle(next)}
    disabled={!switchable}
    onclick={() => step(1)}
  ><Icon name="chevronright" size={15} /></button>
</div>

<style>
  /* ── the frame as an object being carried (COMPOSER §4.4, 2026-09-30) ──────
     The gesture the owner asked for: the frame *follows the hand* in both axes
     and floats above the stack, so the drag reads as moving a thing rather than
     as aiming at a line. The transform itself is the engine's `dx, dy` (and, in
     the beat after the drop, the landing offset it hands over to) written inline
     — it is data, not chrome; everything else the float needs is here.

     The shadow is `--sh-pop`, the floating layer's own rung — a card's `--sh-2`
     is what a frame standing in the stack wears, and a greater object needs a
     greater shadow to say so. The scale is 1.01: enough for the eye to read
     "picked up" beside the shadow, small enough that the frame never appears to
     change size. The z-index is the ladder's drag-ghost rung (UI_SPEC §2.2, 70),
     which is what puts the carried frame over every other frame, the drop line
     and the sticky bar.

     `will-change: transform` promotes the frame on the compositor for the length
     of the gesture, so the per-move transform is a paint and not a re-layout.

     **The transition is not here.** A live `transition: transform` on the lifted
     frame would interpolate every `pointermove` — the frame would trail the hand
     by a fifth of a second, which is exactly the "not smooth" the owner
     reported. It belongs to the one change that *should* take time: the release,
     when the transform returns to nothing. That is `.is-settling`, the class the
     engine holds on the released frame for one beat — see `holdSettle`. */
  .cmp-frame.is-lifted {
    z-index: 70;
    box-shadow: var(--sh-pop);
    scale: 1.01;
    cursor: grabbing;
    will-change: transform;
  }

  /* The landing. The class carries the transition, and the engine turns it on
     one rendering update *after* the drop has redrawn the stack — the frame it
     lands on is moved in the DOM by the keyed `{#each}`, and a moved element has
     its transitions reset, so a transition started in the same recalc as the
     move never runs (`reorder.svelte.ts › holdSettle` has the measurement). Once
     the move is a frame behind, the transform goes from the box the pointer let
     the frame go in back to nothing — a transition declared on the after-change
     style is carried from the previous computed value — so the frame glides into
     the slot instead of teleporting. The class is dropped again one beat later,
     which is what keeps the next drag free of it: while a drag is live there is
     no transform transition at all, on purpose. `box-shadow` and `scale` are in
     the list because declaring `transition` replaces the frame's own box-shadow
     beat (`.cmp-frame`, composer.css). */
  .cmp-frame.is-settling {
    transition:
      transform var(--dur-1) var(--ease),
      scale var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }

  @media (prefers-reduced-motion: reduce) {
    /* The one animation the gesture has, gone — and gone *after* the rule above,
       which is the whole reason this block exists: the app's own reduced-motion
       rule names `.cmp-frame` alone, and this rule is more specific than it. */
    .cmp-frame.is-settling {
      transition: none;
    }
  }
</style>
