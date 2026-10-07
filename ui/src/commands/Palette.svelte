<!--
  Command palette modal (⌘K, R20, P2 · U5, D2, D13, F3, R4).

  Provides unified quick capture, command execution, and entity search:
    1. Capture: Immediate record draft creation from query text or `#kind` prefix.
    2. Actions: Registered command dispatch (`commands/registry.ts`).
    3. Objects: Record, view, and kind search results from the index.

  State management handles search debouncing, parameter input forms, localStorage
  draft persistence across opens, and variant switching (`ui/src/variants/palette/`).
-->
  <script lang="ts">
    import IdPair from '../shell/IdPair.svelte';
    import Variant from '../variants/Variant.svelte';
    import type { PaletteRow } from '../variants/palette/props';
    import { matchesQuery } from './registry';
    import { app } from '../session.svelte';
    import { nameOf, recordLabel, WASHES, type CommandRead, type RecordDoc } from '../types';
    import '../styles/palette.css';

    const DRAFT_KEY = 'sam.palette.draft';
    const FILLER = new Set(['add', 'new', 'create', 'make']);
    const BY_OBJECT = new Set(['rail.select', 'record.panel', 'search']);
    const TITLE_KEYS = ['title', 'label', 'name', 'chapter'];
    const OBJECT_LIMIT = 5;

    type Capture = { id: string; kind: string; title: string };
    type Place = {
      key: string;
      command: string;
      label: string;
      noun: string;
      plate: string;
      target: string;
      kind: string | null;
    };

    const draftText = (): string => {
      try {
        return localStorage.getItem(DRAFT_KEY) ?? '';
      } catch {
        return '';
      }
    };

    let query = $state(draftText());
    let chosen = $state<CommandRead | null>(null);
    let draft = $state<Record<string, string>>({});
    let error = $state<string | null>(null);
    let found = $state<RecordDoc[]>([]);
    let searching = $state(false);
    let searchFailed = $state(false);
    let pending: (() => void) | null = null;
    let layer = $state<HTMLElement | null>(null);

    function kindNamed(word: string): string | null {
      const bare = word.replace(/^#/, '').toLowerCase().replace(/s$/, '');
      if (bare.length === 0) return null;
      for (const name of Object.keys(app.types)) {
        if (name.toLowerCase().replace(/s$/, '') === bare) return name;
      }
      return null;
    }

    function surfaceKind(): string | null {
      const type = app.outcome?.type;
      if (type && app.types[type]) return type;
      const trackable = Object.entries(app.types ?? {}).find(([, def]) => def.trackable === true)?.[0];
      return trackable ?? Object.keys(app.types ?? {})[0] ?? null;
    }

    function titleField(kind: string): string | null {
      const fields = app.types[kind]?.fields ?? [];
      for (const key of TITLE_KEYS) if (fields.some((field) => field.key === key)) return key;
      return fields.find((field) => field.type === 'text' || field.type === 'longtext')?.key ?? null;
    }

    /** Derive capture proposals for the query across matched or surface kinds (F3). */
    const captures = $derived.by<Capture[]>(() => {
      const raw = query.trim();
      if (raw.length === 0) return [];
      const words = raw.split(/\s+/);
      let named: string | null = null;
      let title = raw;
      const head = (words[0] ?? '').toLowerCase();
      if (FILLER.has(head)) {
        if (words.length > 1) title = words.slice(1).join(' ');
      } else if (head === 'log') {
        named = app.sessionKind();
        title = words.length > 1 ? words.slice(1).join(' ') : '';
      } else {
        named = kindNamed(words[0] ?? '');
        if (named) title = words.length > 1 ? words.slice(1).join(' ') : '';
      }
      if (title.trim().length === 0) return [];
      const kinds: string[] = [];
      const consider = (kind: string | null): void => {
        if (kind && app.types[kind] && !kinds.includes(kind)) kinds.push(kind);
      };
      consider(named);
      for (const word of words) consider(kindNamed(word));
      consider(surfaceKind());
      return kinds.slice(0, 3).map((kind) => ({ id: `${kind}.new`, kind, title: title.trim() }));
    });

    const captureIds = $derived(new Set(captures.map((row) => row.id)));

    /** The action class: the registry, minus the three whose door is an object. */
    const actions = $derived(
      (app.registry ?? []).filter(
        (command) => !BY_OBJECT.has(command.id) && !captureIds.has(command.id) && matchesQuery(command, query),
      ),
    );

    const needle = $derived(query.trim().toLowerCase());

    const viewPlaces = $derived.by<Place[]>(() => {
      const rows: Place[] = [];
      for (const [name, def] of Object.entries(app.views?.views ?? {})) {
        const destination = app.navigation.find((entry) => entry.view === name)?.title ?? null;
        const label = destination ?? nameOf(name);
        if (needle.length > 0 && !`${label} ${name}`.toLowerCase().includes(needle)) continue;
        rows.push({
          key: `view:${name}`,
          command: 'rail.select',
          label: destination ? `Go to ${label}` : label,
          noun: def.panel ? 'screen' : 'view',
          plate: label,
          target: name,
          kind: def.type ?? null,
        });
        if (rows.length >= OBJECT_LIMIT) break;
      }
      return rows;
    });

    const kindPlaces = $derived.by<Place[]>(() => {
      const rows: Place[] = [];
      for (const name of Object.keys(app.types ?? {})) {
        if (needle.length > 0 && !name.toLowerCase().includes(needle)) continue;
        const list = Object.entries(app.views?.views ?? {}).find(
          ([, def]) => def.type === name && !def.panel,
        )?.[0];
        if (!list) continue;
        rows.push({
          key: `kind:${name}`,
          command: 'rail.select',
          label: nameOf(name),
          noun: 'kind',
          plate: nameOf(name),
          target: list,
          kind: name,
        });
        if (rows.length >= OBJECT_LIMIT) break;
      }
      return rows;
    });

    function homeOf(kind: string): string {
      const list = Object.entries(app.views?.views ?? {}).find(
        ([, def]) => def.type === kind && !def.panel,
      )?.[0];
      return list ? nameOf(list) : nameOf(kind);
    }

    /** Maps kind position to wash color token cycling for variant visuals. */
    function kindRoom(kind: string | null): string {
      if (!kind) return 'none';
      const at = Object.keys(app.types ?? {}).indexOf(kind);
      return at < 0 ? 'none' : WASHES[at % WASHES.length];
    }

    function effectWord(command: CommandRead): string {
      if (command.planRequired && app.plan === null) return 'needs a plan';
      return command.effect === 'write' ? 'writes' : command.effect === 'read' ? 'reads' : 'opens';
    }

    /** Unified row order: capture proposals, registry actions, and objects (records/views/kinds). */
    const rows = $derived.by<PaletteRow[]>(() => {
      const out: PaletteRow[] = [];

      for (const row of captures) {
        out.push({
          key: `capture:${row.id}`,
          group: 'capture',
          label: `Add “${row.title}”`,
          word: `as ${nameOf(row.kind)}`,
          title: row.title,
          kind: row.kind,
          plate: { key: 'Writes', value: row.title },
          note: titleField(row.kind) ?? 'title',
          wash: kindRoom(row.kind),
          shape: 'tag',
          command: row.id,
          placement: 'palette.capture',
          noun: null,
          target: row.kind,
          blocked: false,
        });
      }

      for (const command of actions) {
        const blocked = command.planRequired && app.plan === null;
        out.push({
          key: `action:${command.id}`,
          group: 'actions',
          label: command.title,
          word: effectWord(command),
          title: null,
          kind: null,
          plate: { key: 'Runs', value: effectWord(command) },
          note: command.id,
          wash: 'none',
          shape: 'slab',
          command: command.id,
          placement: 'palette',
          noun: null,
          target: null,
          blocked,
        });
      }

      for (const record of found) {
        out.push({
          key: `record:${record.id}`,
          group: 'objects',
          label: recordLabel(record),
          word: nameOf(record.type),
          title: null,
          kind: record.type,
          plate: { key: 'Lives in', value: homeOf(record.type) },
          note: `record ${record.id}`,
          wash: app.washes[record.id] ?? kindRoom(record.type),
          shape: 'notch',
          command: 'record.panel',
          placement: 'palette.objects',
          noun: nameOf(record.type),
          target: record.id,
          blocked: false,
        });
      }

      for (const place of [...viewPlaces, ...kindPlaces]) {
        out.push({
          key: place.key,
          group: 'objects',
          label: place.label,
          word: place.noun,
          title: null,
          kind: place.kind,
          plate: { key: 'Goes to', value: place.plate },
          note: place.noun,
          wash: kindRoom(place.kind),
          shape: 'ticket',
          command: place.command,
          placement: 'palette.objects',
          noun: place.noun,
          target: place.target,
          blocked: false,
        });
      }

      return out;
    });

    const status = $derived(
      searching
        ? 'Looking through the plan…'
        : searchFailed
          ? "The plan's search is not answering — the actions above still work."
          : null,
    );

    /** Debounced search for objects matching query text. */
    $effect(() => {
      const text = query.trim();
      if (text.length < 2) {
        found = [];
        searching = false;
        searchFailed = false;
        return;
      }
      const handle = setTimeout(() => {
        searching = true;
        void (async () => {
          const result = await app.run('search', { query: text, limit: String(OBJECT_LIMIT) }, { tracked: false });
          searching = false;
          if (!result) {
            searchFailed = true;
            app.toast = null;
            return;
          }
          searchFailed = false;
          const data = result.data as { records?: RecordDoc[] } | undefined;
          found = (data?.records ?? []).slice(0, OBJECT_LIMIT);
        })();
      }, 150);
      return () => clearTimeout(handle);
    });

    /** Persist query to localStorage draft key. */
    $effect(() => {
      const text = query;
      try {
        if (text.length === 0) localStorage.removeItem(DRAFT_KEY);
        else localStorage.setItem(DRAFT_KEY, text);
      } catch {
        // Silently ignore storage quota or access errors.
      }
    });

    const forget = (): void => {
      try {
        localStorage.removeItem(DRAFT_KEY);
      } catch {
        // Silently ignore storage access errors.
      }
      query = '';
    };

    function fail(message: string): void {
      error = message;
      app.toast = null;
    }

    /**
     * Executes or navigates to the committed row.
     * Returns true when the palette should close; false to retain palette.
     */
    async function commit(row: PaletteRow): Promise<boolean> {
      if (row.blocked) return false;

      if (row.group === 'objects') {
        if (!row.target) return false;
        if (row.command === 'record.panel' && row.kind) {
          app.detail = { id: row.target, type: row.kind };
          app.selection = row.target;
        } else {
          void app.run('rail.select', { view: row.target });
        }
        return false;
      }

      if (row.group === 'actions') {
        const command = app.registry.find((entry) => entry.id === row.command);
        if (!command) return false;
        if (paramsOf(command).length > 0) {
          chosen = command;
          draft = {};
          error = null;
          return false;
        }
        return perform(command);
      }

      return write(row);
    }

    async function write(row: PaletteRow): Promise<boolean> {
      const kind = row.kind;
      if (!kind) return false;
      const field = titleField(kind);
      const noun = nameOf(kind).toLowerCase();
      const title = row.title ?? '';
      if (!field) {
        fail(`a ${noun} has nowhere to put a title — open its list and use New ${nameOf(kind)}`);
        return false;
      }
      const result = await app.run(row.command, { [field]: title });
      if (!result) {
        fail(app.lastDiagnostic ?? `the ${noun} was refused`);
        return false;
      }
      forget();
      pending = () =>
        app.notice(`Added “${title}” as a ${nameOf(kind)}`, {
          label: 'Undo',
          run: () => void app.undo(),
        });
      return true;
    }

    /** Filter out context-managed plan parameter from prompt inputs. */
    const paramsOf = (command: CommandRead): CommandRead['params'] =>
      command.params.filter((param) => param.name !== 'plan');
    const collectable = $derived(chosen ? paramsOf(chosen) : []);

    function leave(): void {
      const after = pending;
      pending = null;
      chosen = null;
      error = null;
      app.paletteOpen = false;
      after?.();
    }

    /** Dismiss palette on Esc or backdrop click, keeping draft in storage (F3, R4). */
    function dismiss(): void {
      pending = null;
      chosen = null;
      error = null;
      app.paletteOpen = false;
    }

    function missingRequired(command: CommandRead): number {
      return paramsOf(command).filter(
        (param) => param.required && (draft[param.name] ?? '').trim().length === 0,
      ).length;
    }

    async function perform(command: CommandRead): Promise<boolean> {
      const params: Record<string, unknown> = {};
      for (const param of command.params) {
        const value = draft[param.name];
        if (param.name === 'plan' || value === undefined || value === '') continue;
        if (param.type === 'boolean') {
          params[param.name] = value === 'true';
        } else if (param.repeats) {
          params[param.name] = value
            .split(',')
            .map((item) => item.trim())
            .filter((item) => item.length > 0);
        } else {
          params[param.name] = value;
        }
      }
      const result = await app.run(command.id, params);
      if (result) {
        forget();
        pending = null;
        return true;
      }
      // Keep command open in form to display diagnostic inline (§4.8 P5).
      chosen = command;
      fail(app.lastDiagnostic ?? 'the command failed');
      return false;
    }

    async function runChosen(): Promise<void> {
      if (!chosen) return;
      if (await perform(chosen)) leave();
    }

    function onkeydown(event: KeyboardEvent): void {
      if (!chosen) return;
      if (event.key === 'Escape') {
        chosen = null;
        return;
      }
      if (event.key === 'Enter' && missingRequired(chosen) === 0) {
        event.preventDefault();
        void runChosen();
      }
    }

    function focusCard(node: HTMLElement): void {
      node.focus();
    }

    /**
     * Scrim dismissal: check event.target === layer directly because committing
     * a row replaces DOM nodes before window click handler fires.
     * Armed after mount tick to prevent trigger click from immediately dismissing.
     */
    let armed = $state(false);

    $effect(() => {
      const id = setTimeout(() => (armed = true), 0);
      return () => clearTimeout(id);
    });

    function dismissOutside(event: MouseEvent): void {
      if (!armed || !layer) return;
      if (event.target === layer) dismiss();
    }
  </script>

  <svelte:window onkeydown={onkeydown} onclick={dismissOutside} />

<div class="cd-layer" role="presentation" bind:this={layer}>
  {#if chosen}
    <div
      class="cd-palette"
      role="dialog"
      aria-modal="true"
      aria-label="Add, find or run something"
      tabindex="-1"
      use:focusCard
    >
      <div class="cd-palette__field">
        <button class="cd-iconbtn" type="button" aria-label="Back to the results" onclick={() => (chosen = null)}>
          ‹
        </button>
        <input
          value={query}
          oninput={(event) => (query = event.currentTarget.value)}
          placeholder={chosen.title}
          aria-label="Add a title, or type to find something"
          autocomplete="off"
        />
        <kbd class="cd-kbd">{app.keyOf('app.palette') ?? 'mod+k'}</kbd>
      </div>

      <div class="cd-palette__params">
        <span><IdPair label={chosen.title} value={chosen.id} /></span>
        {#each collectable as param (param.name)}
          <div class="cd-palette__param">
            <label class="cd-palette__paramlabel" for={`param-${param.name}`}>
              {param.help && param.help.length > 0 ? param.help : nameOf(param.name)}
            </label>
            <span class="cd-palette__paramctl">
              <IdPair value={param.name} />
              {#if param.type === 'boolean'}
                <button
                  class="cd-switch"
                  type="button"
                  role="switch"
                  id={`param-${param.name}`}
                  aria-checked={draft[param.name] === 'true'}
                  aria-label={param.help && param.help.length > 0 ? param.help : param.name}
                  onclick={() => (draft[param.name] = draft[param.name] === 'true' ? 'false' : 'true')}
                ></button>
              {:else}
                <input
                  class="cd-wellfield"
                  id={`param-${param.name}`}
                  placeholder={param.help}
                  bind:value={draft[param.name]}
                  aria-label={param.help && param.help.length > 0 ? param.help : param.name}
                  autocomplete="off"
                />
              {/if}
            </span>
            {#if param.repeats}
              <p class="cd-palette__paramnote">Separate several with commas.</p>
            {/if}
          </div>
        {/each}
        {#if collectable.length === 0}
          <p class="cd-hint">This one takes nothing to run.</p>
        {/if}
        {#if error}
          <p class="cd-error" role="alert">{error}</p>
        {/if}
      </div>

      <div class="cd-palette__foot">
        <span class="cd-palette__keys">
          {missingRequired(chosen) === 0
            ? 'Enter runs it · Esc goes back to the results'
            : `${missingRequired(chosen)} field${missingRequired(chosen) === 1 ? '' : 's'} still to fill in`}
        </span>
        <button
          class="cd-pill cd-pill--sm"
          type="button"
          data-command={chosen.id}
          disabled={missingRequired(chosen) > 0}
          onclick={() => void runChosen()}
        >
          Run it
        </button>
      </div>
    </div>
  {:else}
    <Variant
      surface="palette"
      {query}
      {rows}
      {status}
      {error}
      onQuery={(text) => (query = text)}
      onCommit={commit}
      onLeave={leave}
      onDismiss={dismiss}
    />
  {/if}
</div>
