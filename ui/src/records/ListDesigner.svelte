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
  title={mode === 'screen' ? 'New screen' : 'New list'}
  subtitle={mode === 'screen'
    ? 'A screen of your own — you choose what stands on it, and in what order'
    : mode === 'existing'
      ? 'A saved view of a kind that already exists'
      : 'A kind of thing to track, with its own columns'}
  onclose={onclose}
>
  <div class="cd-segment" role="radiogroup" aria-label="What are you making?">
    <button
      type="button"
      role="radio"
      aria-checked={mode === 'screen'}
      onclick={() => (mode = 'screen')}
    >A screen</button>
    <button
      type="button"
      role="radio"
      aria-checked={mode === 'existing'}
      onclick={() => (mode = 'existing')}
    >Of an existing kind</button>
    <button
      type="button"
      role="radio"
      aria-checked={mode === 'kind'}
      onclick={() => (mode = 'kind')}
    >New kind</button>
  </div>

  {#if mode === 'screen'}
    <div class="cd-sheet__field">
      <label class="cd-sheet__label" for="screen-name">Name</label>
      <input
        id="screen-name"
        class="cd-sheet__well"
        type="text"
        autocomplete="off"
        placeholder="revision, or anything"
        bind:value={name}
      />
    </div>

    <div class="cd-sheet__stack">
      <span class="cd-sheet__label" id="screen-icon">Icon</span>
      <IconPicker
        value={icon}
        labelledby="screen-icon"
        label="Icon"
        onpick={(option) => (icon = icon === option ? '' : option)}
      />
      <p class="cd-sheet__hint">
        the glyph the rail shows beside this screen’s name — pick one, or leave it
      </p>
    </div>
  {:else if mode === 'existing'}
    <div class="cd-sheet__field">
      <label class="cd-sheet__label" for="list-name">Name</label>
      <input
        id="list-name"
        class="cd-sheet__well"
        type="text"
        autocomplete="off"
        placeholder="this term"
        bind:value={name}
      />
    </div>
    {#if kinds.length === 0}
      <p class="cd-empty">No kinds exist yet — create one with “New kind”.</p>
    {:else}
      <div class="cd-sheet__field">
        <label class="cd-sheet__label" for="list-kind">Kind</label>
        <select
          id="list-kind"
          class="cd-sheet__well cd-sheet__select"
          value={existingType}
          onchange={(event) => (existingType = (event.currentTarget as HTMLSelectElement).value)}
        >
          <option value="">— pick a kind</option>
          {#each kinds as kind (kind)}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
      </div>
    {/if}
  {:else}
    <div class="cd-sheet__field">
      <label class="cd-sheet__label" for="kind-name">What do you want to track?</label>
      <input
        id="kind-name"
        class="cd-sheet__well"
        type="text"
        autocomplete="off"
        placeholder="chapter"
        bind:value={name}
      />
    </div>
    <p class="cd-sheet__hint">the id is a slug of this name — the engine derives it</p>

    <div class="cd-sheet__field">
      <label class="cd-sheet__label" for="kind-icon">
        Icon <span class="cd-sheet__key">icon</span>
      </label>
      <input
        id="kind-icon"
        class="cd-sheet__well"
        type="text"
        autocomplete="off"
        placeholder="squareStack"
        bind:value={icon}
      />
    </div>
    <p class="cd-sheet__hint">
      an icon NAME, not a symbol — an unknown one falls back rather than rendering blank
    </p>

    <div class="cd-sheet__row">
      <button
        class="cd-sheet__toggle"
        type="button"
        aria-pressed={trackable}
        onclick={() => (trackable = !trackable)}
      >
        <span class="cd-check" aria-hidden="true">{trackable ? '✓' : ''}</span>
        Trackable
      </button>
      <span class="cd-sheet__hint">has progress through a pipeline</span>
    </div>

    {#if trackable}
      {#if pipelines.length === 0}
        <p class="cd-empty">
          This plan declares no pipelines in <code>content/rules.json</code> — the kind is still
          trackable, it just has no stage machine yet.
        </p>
      {:else}
        <div class="cd-sheet__field">
          <label class="cd-sheet__label" for="kind-pipeline">Pipeline</label>
          <select
            id="kind-pipeline"
            class="cd-sheet__well cd-sheet__select"
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

    <div class="cd-sheet__stack">
      <label class="cd-sheet__label" for="kind-columns">Starter columns</label>
      <textarea
        id="kind-columns"
        class="cd-sheet__well cd-sheet__data"
        rows="4"
        spellcheck="false"
        aria-label="Starter columns"
        placeholder={'chapter:text\ndue:date\nstatus:select:pending|done'}
        bind:value={columns}
      ></textarea>
      <p class="cd-sheet__hint">
        one per line, as <code>key: kind</code> — {starters.length}
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
