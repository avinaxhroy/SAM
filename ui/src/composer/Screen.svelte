<!--
  THE COMPOSED SCREEN (COMPOSER §4).

  Renders configured widget components according to view declarations.
  In view mode, components render directly; in edit mode, components display
  drag handles, configuration frames, and drop targets for reordering.

  Key rules:
  - App.svelte routes to composed screens when `components` is declared or when editing.
  - Empty screens render an action to add the first component (R19).
  - Reordering dispatches `view.setComponents` via `reorder.svelte.ts`.
  - Unrecognized component surfaces render a fallback placeholder card.
-->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import Frame from './Frame.svelte';
  import { Reorder } from './reorder.svelte';
  import { hostFor, widgetLabel } from './widgets/registry';
  import { app } from '../session.svelte';
  import { nameOf } from '../types';
  import '../styles/composer.css';

  let { title = '' }: { title?: string } = $props();

  const components = $derived(app.screenComponents);
  /**
   * The screen's own name: the rail entry that points at it, else its id spoken
   * as a name (D2 — a view id never reaches the screen).
   */
  const heading = $derived(
    title ||
      app.navigation.find((entry) => entry.view === app.selected)?.title ||
      nameOf(app.selected ?? ''),
  );

  let stackEl = $state<HTMLElement | null>(null);

  /** One sentence to the polite live region this screen owns (§5); the message
   *  itself lives on the session, so the sidebar can say things too (§4.8). */
  function announce(text: string): void {
    app.announce(text);
  }

  const reorder = new Reorder({
    frames: () => Array.from(stackEl?.querySelectorAll<HTMLElement>('[data-frame]') ?? []),
    stack: () => stackEl,
    // The app has one scroller (`design.md` §12); the stack lives inside it.
    scroller: () => stackEl?.closest('.cd-scroller') ?? null,
    drop: (from, to) => void move(from, to),
    // A press on a component that never became a drag is the student asking for
    // that component (§4.8): the frame takes the selection and the sidebar
    // states it — its name, its design, its width and its place — instead of
    // only putting the pane back on the screen's own settings. The engine is
    // what tells the two gestures apart, so this answers with the frame's place
    // in the stack.
    tap: (index) => {
      const surface = app.screenComponents[index]?.surface;
      if (surface) app.pickSurface(surface);
    },
  });

  /** The drag's commit: the same one write the menu and the arrow keys use. */
  async function move(from: number, to: number): Promise<void> {
    const list = [...app.screenSurfaces];
    if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return;
    const [surface] = list.splice(from, 1);
    list.splice(to, 0, surface);
    await app.setScreenSurfaces(list);
    announce(`${widgetLabel(surface)}, position ${to + 1} of ${list.length}`);
  }

  /**
   * Take one component off the screen — the write the frame's ✕ makes, kept
   * reachable from a widget that cannot be drawn. It is also the only way out
   * on a screen with no panel and no kind to fall back to, where `--clear`
   * would leave a view that draws nothing: an explicit list (even empty) is
   * the screen's own composition, not a request to restore one.
   */
  function removeSurface(surface: string): void {
    void app.setScreenSurfaces(app.screenSurfaces.filter((candidate) => candidate !== surface));
  }

  /** The empty state's one action: into the mode, and straight to the catalog. */
  function startComposing(): void {
    app.beginScreenEdit();
    app.openInserter();
  }

  /** A view that already draws blocks: editing it composes *over* them. */
  const drawsBlocks = $derived(
    app.outcome?.components === undefined && app.outcome?.type != null && app.outcome?.panel == null,
  );

  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !app.screenEditing) return;
    // Escape closes the topmost thing first, and only then the editor:
    // a sheet or the palette (the app's own state), an open menu (which owns
    // Escape while it is up — its panel holds no focus of its own, so the
    // event's target tells us nothing), or a live drag (the engine holds
    // Escape itself while the pointer is down). A picked record or a selected
    // component is the rung under those: Escape puts the sidebar back on the
    // screen's settings before the next Escape leaves the mode (§4.8).
    if (app.sheet !== null || app.paletteOpen || reorder.dragging) return;
    if (document.querySelector('.cd-menu') !== null) return;
    event.preventDefault();
    if (app.picked !== null || app.selectedSurface !== null) {
      app.clearPick();
      return;
    }
    app.endScreenEdit();
  }

  let editingSeen = false;
  $effect(() => {
    // Entering and leaving are stated once each: a mode that turns on silently
    // is a mode a screen-reader student finds by accident.
    const editing = app.screenEditing;
    if (editing === editingSeen) return;
    editingSeen = editing;
    announce(
      editing
        ? `Editing ${heading}. Add a component, move it with the arrow keys, or press Escape when you are done.`
        : 'Done editing.',
    );
  });
</script>

<svelte:window onkeydown={onkeydown} />

{#snippet widget(surface: string)}
  {@const drawing = hostFor(surface)}
  <svelte:boundary>
    {#if drawing}
      {@const Host = drawing}
      <Host />
    {:else}
      <!-- A surface this build does not carry: a plan written by a later version,
           or a host that has not landed. Stated, and its place kept. -->
      <div class="cd-card cmp-unknown">
        <h2 class="cd-card__title">{widgetLabel(surface)}</h2>
        <p class="cd-card__sub">
          This build does not carry this component yet — its place on the screen is kept.
        </p>
      </div>
    {/if}
    {#snippet failed(error)}
      <!-- A widget that cannot be drawn is one card on the screen, not the
           screen: the rest of the composition keeps drawing, and the card
           carries the act that ends the state — the same `view.setComponents`
           write the frame's own ✕ makes. Without it a failed widget on a
           screen with no panel had no way out but a reload. -->
      <section class="cd-card cmp-unknown" data-fault={surface}>
        <h2 class="cd-card__title">{widgetLabel(surface)} could not be drawn</h2>
        <p class="cd-card__sub"><code>{String(error)}</code></p>
        <div class="cd-fault__acts">
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            data-command="view.setComponents"
            data-placement="composer.frame"
            onclick={() => removeSurface(surface)}
          >
            Take this component off the screen
          </button>
        </div>
      </section>
    {/snippet}
  </svelte:boundary>
{/snippet}

<section
  class="cmp-screen"
  data-view={app.outcome?.view}
  data-editing={app.screenEditing ? '1' : undefined}
>
  {#if app.screenEditing}
    <!-- Sticky, always visible: the mode's own controls must not scroll away
         from the thing they act on (§4.2). -->
    <header class="cmp-bar">
      <p class="cmp-bar__where">
        <span class="cmp-bar__label">Editing</span>
        <span class="cmp-bar__title">{heading}</span>
      </p>
      <!-- The bar's one sentence (§4.2), and it is only worth saying when
           there is something to say it about: a screen with nothing on it has
           already said so in its own middle, and a bar repeating it is noise. -->
      {#if components.length > 0}
        <p class="cmp-bar__note">
          {app.screenHasDefault
            ? 'Drag a component to move it. Your first change makes this screen yours.'
            : 'Drag a component to move it.'}
        </p>
      {/if}
      <span class="cmp-bar__spacer"></span>
      <!-- The sidebar's toggle (`screen.inspector`, §4.8), and it is a real
           toggle: `aria-pressed` carries whether the pane is drawn, so the bar
           always holds a way back to it even after the pane's own ✕ has put it
           away. Its mark is the pane itself — a screen with its panel on the
           right (`panel`), not a generic layout picture — and its two states are
           told apart by *material*, which is how every other toggle in the app
           reads without spending the screen's one dark object: pressed is the
           key, unpressed is the bare mark. -->
      <button
        class="cd-iconbtn cmp-keybtn cmp-panekey"
        type="button"
        aria-pressed={app.inspectorOpen}
        aria-label="Editing settings"
        title={app.inspectorOpen ? 'Hide the editing settings' : 'Show the editing settings'}
        data-command="screen.inspector"
        data-placement="composer.inspector"
        onclick={() => void app.run('screen.inspector')}
      >
        <Icon name="panel" />
      </button>
      <!-- The mode's affirmative control, and it is the ink object (the owner's
           call, 2026-10-12): *Add component* is what edit mode is for, so it
           wears the solid black everywhere it stands — the bar's and the empty
           state's are now one button. Its plus is what pressing it does, read
           before the label is. -->
      <button
        class="cd-pill cd-pill--sm cmp-keybtn"
        type="button"
        data-opens="inserter"
        onclick={() => app.openInserter()}
      >
        <Icon name="plus" size={14} />
        Add component
      </button>
      <!-- The screen's own acts — rename, icon, reset, remove — live in the
           sidebar's screen state (§4.8): one home for each, beside the mode
           they belong to. Done is the mode's exit and wears the ghost the app
           gives a leaving action (`variants/settings/A.svelte`), so the bar
           has one affirmative control rather than two of equal weight. -->
      <button
        class="cd-pill cd-pill--ghost cd-pill--sm cmp-keybtn cmp-bar__done"
        type="button"
        onclick={() => app.endScreenEdit()}
      >
        Done
      </button>
    </header>
  {/if}

  {#if components.length === 0}
    <section class="cd-card">
      <div class="cd-empty">
        <span class="cd-empty__art"><Icon name="layout" size={28} /></span>
        <div class="cd-empty__t">
          {drawsBlocks ? 'This screen draws its own blocks' : 'Nothing on this screen yet'}
        </div>
        <div class="cd-empty__s">
          {drawsBlocks
            ? 'A component you add here stands on the screen in their place; the blocks stay in the file, and taking the components away puts them back.'
            : 'Add a component and the screen starts being yours — you can move it, change its design, or take it away.'}
        </div>
        <button
          class="cd-pill cd-pill--sm cmp-keybtn"
          type="button"
          data-opens="inserter"
          onclick={startComposing}
        >
          <Icon name="plus" size={14} />
          Add component
        </button>
      </div>
    </section>
  {:else if app.screenEditing}
    <!-- The press here is a *deselect*, and only when it lands on the stack's
         own gap (the handler checks `target === currentTarget`): a press on a
         frame is the drag's, a press on a row is the sidebar's. A static
         element with no interaction role is exactly what this is. -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="cmp-stack"
      bind:this={stackEl}
      onpointerdown={(event) => {
        // A press on the stack's own gap — not on a frame or a row — puts the
        // sidebar back on the screen's settings (§4.8), dropping a picked
        // record or the selected component with it.
        if (event.target === event.currentTarget) app.clearPick();
      }}
    >
      {#each components as component, at (component.surface)}
        <Frame
          surface={component.surface}
          span={component.span}
          index={at}
          {reorder}
          {announce}
          body={widget}
        />
      {/each}
      {#if reorder.dragging && reorder.to !== null}
        <!-- The landing, and the only thing that moves: the stack holds its
             places so the student never re-reads it mid-drag. The line stands
             where the widget would land — beside the object it goes before, not
             above it, because the stack is two columns wide. -->
        <div
          class="cmp-dropline"
          style:left={`${reorder.line.x}px`}
          style:top={`${reorder.line.y}px`}
          style:height={`${reorder.line.h}px`}
          aria-hidden="true"
        ></div>
      {/if}
    </div>
  {:else}
    <div class="cmp-view">
      {#each components as component (component.surface)}
        <!-- The reading copy of the same grid: the cell carries the width the
             frame sets while editing, so view mode and edit mode cannot
             disagree about how wide a widget stands. -->
        <div class="cmp-cell" style:--cmp-span={component.span}>
          {@render widget(component.surface)}
        </div>
      {/each}
    </div>
  {/if}

  <p class="cd-sr cmp-live" role="status">{app.live}</p>
</section>
