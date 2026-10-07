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
  import Menu from '../shell/Menu.svelte';
  import Frame from './Frame.svelte';
  import { Reorder } from './reorder.svelte';
  import { hostFor, widgetLabel } from './widgets/registry';
  import { app } from '../session.svelte';
  import { nameOf } from '../types';
  import type { MenuRow } from '../commands/registry';
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
  let announcement = $state('');

  /** Announces reorder updates to screen readers. Clears first so repeated text triggers an announcement. */
  function announce(text: string): void {
    announcement = '';
    queueMicrotask(() => (announcement = text));
  }

  const reorder = new Reorder({
    frames: () => Array.from(stackEl?.querySelectorAll<HTMLElement>('[data-frame]') ?? []),
    stack: () => stackEl,
    // The app has one scroller (`design.md` §12); the stack lives inside it.
    scroller: () => stackEl?.closest('.cd-scroller') ?? null,
    drop: (from, to) => void move(from, to),
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

  /** The empty state's one action: into the mode, and straight to the catalog. */
  function startComposing(): void {
    app.beginScreenEdit();
    app.openInserter();
  }

  /** The view this screen is — the id every screen act addresses. */
  const viewId = $derived(app.outcome?.view ?? app.selected ?? '');

  /**
   * Whether the view draws something of its own to fall back to: a panel SAM
   * designed, or its own blocks. Only then can the student undo their
   * composition, because only then is there an arrangement behind it.
   */
  const hasArrangement = $derived(app.outcome?.panel != null || app.outcome?.type != null);
  /** A view that already draws blocks: editing it composes *over* them. */
  const drawsBlocks = $derived(
    app.outcome?.components === undefined && app.outcome?.type != null && app.outcome?.panel == null,
  );

  let screenMenu = $state(false);
  /** The screen's own acts (§4.7), each one a sheet of its own. */
  const screenRows = $derived<MenuRow[]>([
    {
      id: 'list.set',
      title: 'Rename screen…',
      run: () => void app.openSheet({ kind: 'screen.rename', view: viewId }),
    },
    {
      id: 'list.set',
      title: 'Change icon…',
      run: () => void app.openSheet({ kind: 'screen.icon', view: viewId }),
    },
    { separator: true },
    {
      id: 'view.delete',
      title: 'Remove screen',
      danger: true,
      run: () => void app.openSheet({ kind: 'screen.remove', view: viewId }),
    },
  ]);

  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !app.screenEditing) return;
    // Escape closes the topmost thing first, and only then the editor:
    // a sheet or the palette (the app's own state), an open menu (which owns
    // Escape while it is up — its panel holds no focus of its own, so the
    // event's target tells us nothing), or a live drag (the engine holds
    // Escape itself while the pointer is down).
    if (app.sheet !== null || app.paletteOpen || reorder.dragging) return;
    if (document.querySelector('.cd-menu') !== null) return;
    event.preventDefault();
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
      {#if app.screenHasDefault}
        <p class="cmp-bar__note">
          Drag a component anywhere on its card to move it. The first change makes this screen
          yours — until then it stays as SAM designed it.
        </p>
      {:else}
        <p class="cmp-bar__note">Drag a component anywhere on its card to move it.</p>
      {/if}
      <span class="cmp-bar__spacer"></span>
      <button
        class="cd-pill cd-pill--quiet cd-pill--sm"
        type="button"
        data-opens="inserter"
        onclick={() => app.openInserter()}
      >Add component</button>
      {#if app.screenIsCustom && hasArrangement}
        <!-- Reset to default layout when view has a preset arrangement. -->
        <button
          class="cd-pill cd-pill--quiet cd-pill--sm"
          type="button"
          data-command="view.setComponents"
          title="Take the components away and draw the screen as it was arranged before"
          onclick={() => void app.resetScreen()}
        >Use SAM's design again</button>
      {/if}

      <!-- The screen's own acts (§4.7): its name, its rail glyph, and its
           removal. Behind one quiet control, because none of them is what the
           student came here to do — the components are. -->
      <span class="cd-menu-wrap">
        <button
          class="cd-pill cd-pill--quiet cd-pill--sm"
          type="button"
          aria-haspopup="menu"
          aria-expanded={screenMenu}
          aria-label={`Screen actions for ${heading}`}
          onclick={(event) => {
            // The menu mounts inside this click and dismisses on window clicks;
            // the opening click is stopped, the rest still close it.
            event.stopPropagation();
            screenMenu = !screenMenu;
          }}
        >Screen ▾</button>
        {#if screenMenu}
          <Menu rows={screenRows} native={false} onclose={() => (screenMenu = false)} />
        {/if}
      </span>

      <button class="cd-pill cd-pill--quiet cd-pill--sm cmp-bar__done" type="button" onclick={() => app.endScreenEdit()}>
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
          class="cd-pill cd-pill--sm"
          type="button"
          data-opens="inserter"
          onclick={startComposing}
        >Add component</button>
      </div>
    </section>
  {:else if app.screenEditing}
    <div class="cmp-stack" bind:this={stackEl}>
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

  <p class="cd-sr cmp-live" role="status">{announcement}</p>
</section>
