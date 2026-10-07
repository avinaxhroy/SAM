<!--
  App header / titlebar (design.md §12, UI_SPEC R2, D2, D14).

  Houses app identity, plan context (e.g. current week / date from `app.today`),
  command palette trigger, timer chrome, and secondary drawers (source/detail):
    - Drag region: Uses sibling `data-tauri-drag-region` elements to avoid swallowing clicks on controls.
    - Shortcuts: Resolves shortcut glyphs dynamically via `shortcutFor` per platform (D14).
    - Host for timer variants: `<Timer/>` delegates to `ui/src/variants/timer-chrome/`.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import Timer from './Timer.svelte';
  import { shortcutFor } from '../commands/registry';
  import { app } from '../session.svelte';
  // The identity mark is the bundle's artwork, not a redrawn glyph, so the tab,
  // the Dock and the titlebar are one mark. The cross into `design/` is the same
  // one `styles/app.css` already makes for `design/components.css`.
  import markLight from '../design/mark-light.png';
  import markDark from '../design/mark-dark.png';

  let { os = 'mac', quiet = false }: { os?: 'mac' | 'win' | 'linux'; quiet?: boolean } = $props();

  const paletteKey = $derived(shortcutFor(app.keybindings, 'app.palette', os) ?? '');
  const sourceKey = $derived(shortcutFor(app.keybindings, 'app.toggleSourcePane', os) ?? '');

  /** The context slot: the week the plan places today in, else the date. */
  const context = $derived.by(() => {
    const today = app.today;
    if (!today) return '';
    const week = today.week;
    if (week) {
      return week.of === null || week.of === undefined
        ? `Week ${week.index}`
        : `Week ${week.index} of ${week.of}`;
    }
    return `${today.weekday} ${today.day} ${today.month}`;
  });
</script>

<header class="cd-titlebar" data-os={os}>
  <div class="cd-titlebar__group">
    <span class="cd-mark" aria-hidden="true">
      <img src={markLight} alt="" data-mark="light" />
      <img src={markDark} alt="" data-mark="dark" />
    </span>
    <span class="cd-appname">SAM</span>
    {#if context}
      <span class="cd-term">{context}</span>
    {/if}
  </div>

  <div class="cd-titlebar__spacer" data-tauri-drag-region></div>

  <div class="cd-titlebar__tools">
    <!-- QUIET: the room is the whole window on a
         machine with no plan open, so the tools — the search pill,
         the editor's pencil, the two doors — do not draw over
         the starts. The app does not leak past its own start
         screen (R11 restated); that screen carries the one
         control that is true without a plan (the mode switch).
         A timer cannot run on a plan that does not exist, so
         it is quiet with the rest. -->
    {#if !quiet}
    <!--
      THE PLAN'S OWN FOLDER, IN ONE PRESS.

      The window works in one plan folder and said so nowhere: changing it
      meant quitting, or a second launch, or the macOS menu (§4.10 —
      `Open Recent`, a file association, `--plan`), none of which is *in* the
      window. This is that door: a folder beside the plan's own context slot,
      dispatching `app.changePlan`, which opens the picker's material — the
      plans on this machine and the ones this window remembers — as a sheet
      over the working plan. Plan-scoped like the timer beside it, so with no
      plan open it is quiet with the rest: the start screen already carries
      this choice, and it is where that choice belongs.
    -->
    <button
      class="cd-iconbtn"
      type="button"
      title="Change the plan folder"
      aria-label="Change the plan folder"
      data-command="app.changePlan"
      data-placement="app.window"
      onclick={() => app.run('app.changePlan')}
    >
      <Icon name="folder" />
    </button>

    <Timer />

    <!--
      THE SCREEN EDITOR'S DOOR (COMPOSER §3.3).

      One quiet pencil, hidden when there is no screen to edit and while the
      app's own surfaces (Settings, System) are showing — those are not the
      student's screens, so editing them is not a thing. While the mode is on
      it becomes a check and its name becomes *Done editing*: like the rail's
      theme disc, the control shows the action it will perform rather than
      mirroring the state it is in. It dispatches the presentation command
      `screen.edit`; the editor itself lives in the session.
    -->
    {#if app.outcome && !app.settingsOpen && !app.systemOpen}
      <button
        class="cd-iconbtn"
        type="button"
        title={app.screenEditing ? 'Done editing' : 'Edit screen'}
        aria-label={app.screenEditing ? 'Done editing' : 'Edit screen'}
        aria-pressed={app.screenEditing}
        data-command="screen.edit"
        data-placement="app.window"
        onclick={() => app.run('screen.edit')}
      >
        <Icon name={app.screenEditing ? 'check' : 'pencil'} />
      </button>
    {/if}

    <!--
      ONE SEARCH SURFACE IN THE WHOLE APP.

      This used to be a bare 32px magnifier icon — the affordance for "open a
      panel" and the label for "type a query", which are two different actions,
      and the app does both. The system's `.cd-searchpill` is a `--well` pill
      carrying a placeholder and a real keycap, so the shortcut is **visible**
      instead of filed in a `title` attribute: a shortcut nobody can see is a
      shortcut nobody learns, and this window is opened eight times a day
      (`design.md` §2, finding 6).

      It is a `<button>`, not an `<input>` — the system is explicit that the
      field is a button that opens the console, because two focusable search
      inputs in one window is two mental models for one verb.
    -->
    <button
      class="cd-searchpill cd-titlebar__search"
      type="button"
      aria-haspopup="dialog"
      data-command="app.palette"
      data-placement="app.window"
      onclick={() => app.run('app.palette')}
    >
      <Icon name="search" size={15} />
      <span class="cd-titlebar__searchtext">Search</span>
      <kbd class="cd-kbd">{paletteKey || '⌘K'}</kbd>
    </button>

    <!-- The other two are doors, not destinations: quiet icon buttons, beside
         the search pill rather than competing with it. The source pane is a
         toggle and says so with `aria-pressed`; settings opens a window. -->
    <button
      class="cd-iconbtn"
      type="button"
      title={sourceKey ? `Source pane (${sourceKey})` : 'Source pane'}
      aria-label="Toggle the source pane"
      data-command="app.toggleSourcePane"
      data-placement="app.window"
      aria-pressed={app.sourcePane}
      onclick={() => app.run('app.toggleSourcePane')}
    >
      <Icon name="code" />
    </button>
    <button
      class="cd-iconbtn"
      type="button"
      title="Settings"
      aria-label="Open Settings"
      data-command="app.openSettings"
      data-placement="app.window"
      onclick={() => app.run('app.openSettings')}
    >
      <Icon name="gear" />
    </button>
    {/if}
  </div>
</header>
