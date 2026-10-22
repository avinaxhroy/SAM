<!--
  The list designer (Appendix C.9, §11's sidebar `+`): two modes, and they are
  the whole "new kind" story.

    Of an existing kind   list.new {name, type}      a saved view of a kind
    New kind              type.new {name, icon?, trackable?, pipeline?, field?}

  Four decisions:

  1. **The slug is the engine's.** The "New kind" mode sends the NAME and
     nothing else — `TypeOps.slug` derives the id, exactly as `DocEdit.autoId`
     derives a record's (§4.7 rule 2). A client-side slug would be a second
     implementation of the engine's own naming rule.
  2. **The pipeline picker comes from `SAM schema --json`.** `rules.pipelines`
     is read on mount, never hardcoded: a pipeline the plan declares appears
     here with no code change. A plan that declares none is a stated state, not
     a broken picker, because `pipeline` is an optional parameter.
  3. **Trackable reveals the pipeline.** A stage machine without a trackable
     kind is a pipeline with nothing to advance, so the parameter is only
     reachable once the toggle says the kind has progress.
  4. **Starter columns are `key: kind`, one per line.** That is the engine's own
     repeated `--field key:kind[:a|b]` spelling (doc_edit.rs `mod type_ops`), so
     the textarea is the parameter rather than a form pretending to be one — and
     the engine's own message is what a mistyped line reports.
-->
<script lang="ts">
  import { IpcError, dispatch } from '../ipc';
  import { app } from '../session.svelte';
  import Sheet from '../shell/Sheet.svelte';
  import Icon from '../shell/Icon.svelte';
  import IconPicker from '../composer/IconPicker.svelte';
  import '../styles/sheets.css';
  // The icon picker is the composer's own control (COMPOSER §4.7): it belongs
  // with the screen styles rather than with the sheet chrome.
  import '../styles/composer.css';

  let { onclose }: { onclose: () => void } = $props();

  /**
   * Three modes, and the screen comes first: the rail's Add is how a student
   * starts something of their own, and "a screen" is the thing this app is for.
   * The two list modes below are unchanged — same commands, same receipts.
   */
  let mode = $state<'screen' | 'existing' | 'kind'>('screen');
  let name = $state('');
  let existingType = $state('');
  let icon = $state('');
  let trackable = $state(false);
  let pipeline = $state('');
  let columns = $state('');
  let pipelines = $state<string[]>([]);
  let pipelinesError = $state<string | null>(null);
  let fetched = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const kinds = $derived(Object.keys(app.types).sort());
  const starters = $derived(
    columns
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line !== ''),
  );
  /** The one ink object dispatches one command, and it says which (C.6). */
  const command = $derived(mode === 'kind' ? 'type.new' : 'list.new');
  const ready = $derived(name.trim() !== '' && (mode !== 'existing' || existingType !== ''));

  $effect(() => {
    // Once, on mount (`fetched` is the guard, so an empty pipeline list is not
    // re-read forever). The read is scoped to the open plan: a schema from
    // another plan would be a picker of pipelines this plan does not have.
    if (fetched) return;
    fetched = true;
    void (async () => {
      try {
        const result = await dispatch('schema', {}, { plan: app.plan });
        const data = result.data as { pipelines?: Record<string, unknown> };
        pipelines = Object.keys(data.pipelines ?? {}).sort();
      } catch (failure) {
        pipelinesError = failure instanceof IpcError ? failure.text : String(failure);
      }
    })();
  });

  async function commit(): Promise<void> {
    error = null;
    const title = name.trim();
    if (title === '') {
      error =
        mode === 'existing'
          ? 'a list needs a name'
          : mode === 'screen'
            ? 'a screen needs a name'
            : 'say what you want to track';
      return;
    }
    if (mode === 'existing' && existingType === '') {
      error = 'pick the kind this list shows';
      return;
    }

    // A SCREEN (COMPOSER §4.7): `list.new` with no type at all — the engine
    // writes `{ "components": [] }` and the rail entry in one transaction. The
    // icon travels in the same write. A new screen then opens **empty, in edit
    // mode, with the catalog open**: the blank canvas is answered with a
    // one-sentence empty state and one action, not with a designer.
    if (mode === 'screen') {
      busy = true;
      try {
        const result = await app.run('list.new', {
          name: title,
          ...(icon.trim() === '' ? {} : { icon: icon.trim() }),
        });
        if (!result) {
          error = app.toast ?? 'the new screen was refused';
          app.toast = null;
          return;
        }
        const created = (result.data as { view?: string } | undefined)?.view;
        // Select first, so the draft seeds from the screen that now exists.
        if (created) await app.selectView(created);
        app.beginScreenEdit();
        app.openInserter();
      } finally {
        busy = false;
      }
      return;
    }

    const params: Record<string, unknown> =
      mode === 'existing'
        ? { name: title, type: existingType }
        : {
            name: title,
            ...(icon.trim() === '' ? {} : { icon: icon.trim() }),
            ...(trackable ? { trackable: true } : {}),
            ...(trackable && pipeline !== '' ? { pipeline } : {}),
            ...(starters.length === 0 ? {} : { field: starters }),
          };

    busy = true;
    try {
      const result = await app.run(command, params);
      if (!result) {
        // The sheet stays open holding the engine's own finding: the form is
        // still wrong, and the user is still looking at it.
        error = app.toast ?? 'the new list was refused';
        app.toast = null;
        return;
      }
      onclose();
    } finally {
      busy = false;
    }
  }
</script>

<Sheet
  title={mode === 'screen' ? 'New screen' : mode === 'existing' ? 'New list' : 'New kind'}
  subtitle={mode === 'screen'
    ? 'A screen of your own — you choose what stands on it, and in what order.'
    : mode === 'existing'
      ? 'A saved view of a kind that already exists.'
      : 'A kind of thing to track, with its own columns.'}
  icon={mode === 'screen' ? 'plus' : mode === 'existing' ? 'checklist' : 'squareStack'}
  wash={mode === 'screen' ? 'sky' : mode === 'existing' ? 'lilac' : 'mint'}
  onclose={onclose}
>
  <div class="cd-segment cd-segment--lead" role="radiogroup" aria-label="What are you making?">
    <button
      type="button"
      role="radio"
      aria-checked={mode === 'screen'}
      onclick={() => (mode = 'screen')}
    >
      <Icon name="panel" size={14} />
      A screen
    </button>
    <button
      type="button"
      role="radio"
      aria-checked={mode === 'existing'}
      onclick={() => (mode = 'existing')}
    >
      <Icon name="squareStack" size={14} />
      Of an existing kind
    </button>
    <button
      type="button"
      role="radio"
      aria-checked={mode === 'kind'}
      onclick={() => (mode = 'kind')}
    >
      <Icon name="sparkle" size={14} />
      New kind
    </button>
  </div>

  {#if mode === 'screen'}
    <div class="ld-form">
      <label class="ld-label" for="screen-name">Name</label>
      <input
        id="screen-name"
        class="ld-field"
        type="text"
        autocomplete="off"
        placeholder="A name, revision, or anything"
        bind:value={name}
      />
    </div>

    <div class="ld-form">
      <span class="ld-label" id="screen-icon">Icon</span>
      <IconPicker
        value={icon}
        labelledby="screen-icon"
        label="Icon"
        search
        onpick={(option) => (icon = icon === option ? '' : option)}
      />
      <!-- What the rail will draw, stated before the screen exists: the glyph
           and the name, in the rail's own proportions. A preview is the one
           thing a name-and-glyph form cannot do in words. -->
      <div class="cmp-screenpreview">
        <span class="cd-glyph cd-glyph--sm" data-w="sky" aria-hidden="true">
          <Icon name={icon || 'panel'} />
        </span>
        <span class="cmp-screenpreview__text">
          <span class="cmp-screenpreview__name">{name.trim() === '' ? 'Screen name' : name.trim()}</span>
          <span class="cmp-screenpreview__say">
            {icon === ''
              ? 'As the rail will draw it — with the app’s own glyph'
              : 'As the rail will draw it'}
          </span>
        </span>
      </div>
    </div>
  {:else if mode === 'existing'}
    <div class="ld-form">
      <label class="ld-label" for="list-name">Name</label>
      <input
        id="list-name"
        class="ld-field"
        type="text"
        autocomplete="off"
        placeholder="this term"
        bind:value={name}
      />
    </div>
    {#if kinds.length === 0}
      <p class="cd-empty">No kinds exist yet — create one with “New kind”.</p>
    {:else}
      <div class="ld-form">
        <label class="ld-label" for="list-kind">Kind</label>
        <select
          id="list-kind"
          class="ld-field ld-select"
          value={existingType}
          onchange={(event) => (existingType = (event.currentTarget as HTMLSelectElement).value)}
        >
          <option value="">— pick a kind</option>
          {#each kinds as kind (kind)}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
        <p class="ld-hint">The list shows this kind’s records as rows you can edit in place.</p>
      </div>
    {/if}
  {:else}
    <div class="ld-form">
      <label class="ld-label" for="kind-name">What do you want to track?</label>
      <input
        id="kind-name"
        class="ld-field"
        type="text"
        autocomplete="off"
        placeholder="chapter"
        bind:value={name}
      />
      <p class="ld-hint">the id is a slug of this name — the engine derives it</p>
    </div>

    <div class="ld-form">
      <span class="ld-label" id="kind-icon">Icon <span class="cd-sheet__key">icon</span></span>
      <!-- A kind's glyph cannot be cleared (`type.new` carries no
           `clear-icon`), so the picker offers no None tile here. -->
      <IconPicker
        value={icon}
        labelledby="kind-icon"
        label="Icon"
        clears={false}
        onpick={(option) => (icon = icon === option ? '' : option)}
      />
      <p class="ld-hint">an icon NAME, not a symbol — an unknown one falls back rather than rendering blank</p>
    </div>

    <div class="ld-toggle">
      <button
        class="ld-switch"
        type="button"
        role="switch"
        aria-checked={trackable}
        onclick={() => (trackable = !trackable)}
      >
        <span class="ld-switch__track" aria-hidden="true"><span class="ld-switch__knob"></span></span>
        Trackable
      </button>
      <span class="ld-hint">has progress through a pipeline</span>
    </div>

    {#if trackable}
      {#if pipelines.length === 0}
        <p class="cd-empty">
          This plan declares no pipelines in <code>content/rules.json</code> — the kind is still
          trackable, it just has no stage machine yet.
        </p>
      {:else}
        <div class="ld-form">
          <label class="ld-label" for="kind-pipeline">Pipeline</label>
          <select
            id="kind-pipeline"
            class="ld-field ld-select"
            value={pipeline}
            onchange={(event) => (pipeline = (event.currentTarget as HTMLSelectElement).value)}
          >
            <option value="">— pick a pipeline</option>
            {#each pipelines as option (option)}
              <option value={option}>{option}</option>
            {/each}
          </select>
        </div>
      {/if}
    {/if}

    {#if pipelinesError}
      <p class="cd-sheet__error" role="alert">
        the pipeline list could not be read — {pipelinesError}
      </p>
    {/if}

    <div class="ld-form">
      <label class="ld-label" for="kind-columns">Starter columns</label>
      <textarea
        id="kind-columns"
        class="ld-field cd-sheet__data"
        rows="4"
        spellcheck="false"
        aria-label="Starter columns"
        placeholder={'chapter:text\ndue:date\nstatus:select:pending|done'}
        bind:value={columns}
      ></textarea>
      <p class="ld-hint">
        one per line, as <code>chapter:text</code> — {starters.length}
        {starters.length === 1 ? 'column' : 'columns'}
      </p>
    </div>
  {/if}

  {#if error}
    <p class="cd-sheet__error" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    <span class="cd-sheet__hint">{app.commandDef(command)?.title ?? ''}</span>
    <span class="cd-sheet__spacer"></span>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Cancel</button>
    <button
      class="cd-pill"
      type="button"
      data-command={command}
      disabled={busy || !ready}
      onclick={commit}
    >
      {mode === 'existing' ? 'Create list' : mode === 'kind' ? 'Create kind' : 'Create screen'}
    </button>
  {/snippet}
</Sheet>

<style>
  /* ── the form ────────────────────────────────────────────────────────────
     A field is a label over a well, at the sheet's full width, and the label is
     the app's own bold small — never the two-column label/control row the
     short sheets use, because these fields are *read* (a name, a list of
     columns) rather than scanned past. */
  .ld-form {
    display: grid;
    gap: var(--space-2xs);
    min-width: 0;
  }
  .ld-label {
    font-size: var(--text-2xs);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .ld-label .cd-sheet__key { text-transform: none; letter-spacing: normal; }
  .ld-field {
    width: 100%;
    min-width: 0;
    min-height: var(--pill-h);
    padding: var(--space-2xs) var(--space-sm);
    border: 0;
    border-radius: var(--r-mini);
    background: var(--well);
    color: var(--ink);
    font-family: var(--font-body);
    font-size: var(--text-xs);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .ld-field:focus,
  .ld-field:focus-visible {
    outline: none;
    background: var(--card);
    box-shadow: inset 0 0 0 1.5px var(--ink-4);
  }
  .ld-field::placeholder { color: var(--ink-4); }
  .ld-select { width: 100%; }
  .ld-hint {
    margin: 0;
    font-size: var(--text-2xs);
    line-height: 1.45;
    color: var(--ink-3);
  }

  /* ── the yes/no ──────────────────────────────────────────────────────────
     The declared control for yes/no is the switch (`controls.md` §2), never a
     two-option segment whose first option is always lit. The track is the well
     and the knob is the card, so the state reads without colour. */
  .ld-toggle {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
  }
  .ld-switch {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink-2);
  }
  .ld-switch[aria-checked='true'] { color: var(--ink); }
  .ld-switch__track {
    position: relative;
    display: block;
    width: 36px;
    height: 22px;
    flex: none;
    border-radius: var(--r-pill);
    background: var(--well-2);
    transition: background var(--dur-1) var(--ease);
  }
  .ld-switch[aria-checked='true'] .ld-switch__track { background: var(--ink); }
  .ld-switch__knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: var(--r-pill);
    background: var(--card);
    box-shadow: var(--sh-1);
    transition: transform var(--dur-2) var(--ease);
  }
  .ld-switch[aria-checked='true'] .ld-switch__knob { transform: translateX(14px); }

  @media (prefers-reduced-motion: reduce) {
    .ld-field,
    .ld-switch__track,
    .ld-switch__knob { transition: none; }
  }
</style>
