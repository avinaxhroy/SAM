<!--
  THE SCREEN'S OWN SHEET (COMPOSER §4.7).

  The edit bar's Screen menu owns three acts on the screen itself — rename it,
  change its rail glyph, remove it — and each one is a sheet because that is how
  this app asks a question (never a browser prompt, never a second dialog).

  Three decisions:

  1. **`list.set` is the rename and the icon.** One command, two fields: the
     destination's title and its glyph are the same record the System surface's
     rail editor writes, so a name changed here reads there and vice versa.
  2. **The removal states the engine's own reading first.** The sheet asks the
     engine for a dry run of the very command the confirm will commit, so the
     consequence is the one that will happen — and when the engine refuses (the
     last view of its kind), the refusal is what the student reads, not a
     cheerful sentence beside a dead button.
  3. **Nothing is deleted but the view.** The records it read belong to the
     kinds; a screen is a saved query and a rail entry, and the sheet says so.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import { nameOf } from '../types';
  import Sheet from '../shell/Sheet.svelte';
  import IconPicker from './IconPicker.svelte';
  import '../styles/sheets.css';
  import '../styles/composer.css';

  let {
    what,
    view,
    onclose,
  }: {
    /** Which of the three acts this sheet is asking about. */
    what: 'rename' | 'icon' | 'remove';
    /** The view the screen is — the id the destination points at. */
    view: string;
    onclose: () => void;
  } = $props();

  /** The rail entry that names this screen, when it is on the rail. */
  const railEntry = (): { title: string; icon?: string | null } | null =>
    app.navigation.find((row) => row.view === view) ?? null;
  const entry = $derived(railEntry());
  /** What the rail calls it now — the sheet's own subject line. A screen that
      is not on the rail yet is spoken as a name, never as its id (D2). */
  const current = $derived(entry?.title ?? nameOf(view));

  // The fields are seeded once, from the same entry: from the moment the
  // student types, the value is theirs and must not follow the store.
  let name = $state(railEntry()?.title ?? '');
  let icon = $state<string>(railEntry()?.icon ?? '');

  let error = $state<string | null>(null);
  let busy = $state(false);

  async function commit(): Promise<void> {
    error = null;
    busy = true;
    try {
      if (what === 'remove') {
        const result = await app.run('view.delete', { name: view });
        if (!result) {
          error = app.toast ?? 'the screen was not removed';
          app.toast = null;
          return;
        }
        app.notice('Removed the screen');
        onclose();
        return;
      }
      if (what === 'rename') {
        const title = name.trim();
        if (title === '') {
          error = 'a screen needs a name';
          return;
        }
        const result = await app.run('list.set', { view, title });
        if (!result) {
          error = app.toast ?? 'the new name was refused';
          app.toast = null;
          return;
        }
        app.notice(`The rail reads “${title}” now`);
        onclose();
        return;
      }
      // The glyph: `list.set --icon`, or `--clear-icon` when the student wants
      // the app's own fallback back.
      const result =
        icon === ''
          ? await app.run('list.set', { view, 'clear-icon': true })
          : await app.run('list.set', { view, icon });
      if (!result) {
        error = app.toast ?? 'the new glyph was refused';
        app.toast = null;
        return;
      }
      app.notice(icon === '' ? 'The app’s own glyph is back' : 'The rail’s glyph is set');
      onclose();
    } finally {
      busy = false;
    }
  }
</script>

<Sheet
  title={what === 'rename' ? 'Rename screen' : what === 'icon' ? 'Change icon' : 'Remove screen'}
  subtitle={what === 'remove'
    ? 'Take this screen off the rail and out of the plan'
    : what === 'rename'
      ? 'What the rail calls this screen'
      : 'The glyph the rail draws beside it'}
  {onclose}
>
  {#if what === 'rename'}
    <div class="cd-sheet__field">
      <label class="cd-sheet__label" for="screen-rename">Name</label>
      <input
        id="screen-rename"
        class="cd-sheet__well"
        type="text"
        autocomplete="off"
        bind:value={name}
      />
    </div>
  {:else if what === 'icon'}
    <div class="cd-sheet__stack">
      <span class="cd-sheet__label" id="screen-icon">Icon</span>
      <IconPicker value={icon} labelledby="screen-icon" onpick={(option) => (icon = icon === option ? '' : option)} />
      <p class="cd-sheet__hint">
        the glyph the rail shows beside this screen’s name — pick one, or clear it for the app’s own
      </p>
    </div>
  {:else}
    <p class="cd-sheet__note">
      <strong>{current}</strong> leaves the rail and the plan. Nothing it counted is deleted — a
      screen is a saved query, and the records it read stay where they are.
    </p>
    {#if app.sheetPreview}
      <!-- The engine's own refusal, in its own words: the button below would be
           refused for exactly this reason. -->
      <p class="cd-sheet__error" role="alert">{app.sheetPreview}</p>
    {/if}
  {/if}

  {#if error}
    <p class="cd-sheet__error" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    <span class="cd-sheet__hint">{app.commandDef(what === 'remove' ? 'view.delete' : 'list.set')?.title ?? ''}</span>
    <span class="cd-sheet__spacer"></span>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Cancel</button>
    <button
      class="cd-pill"
      type="button"
      data-command={what === 'remove' ? 'view.delete' : 'list.set'}
      disabled={busy}
      onclick={commit}
    >
      {what === 'rename' ? 'Rename' : what === 'icon' ? 'Set icon' : 'Remove screen'}
    </button>
  {/snippet}
</Sheet>
