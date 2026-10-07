<!--
  `+` → *New column…* (Appendix C.2's header menu).

  Thirteen kinds, one of them `relation` (with a `to` picker over the active
  types) and one `formula` (with an expression). This is the sheet that makes a
  student able to define a column — including a computed one — without being told
  what a column is, and it dispatches the same `column.new` the terminal does:

    SAM column.new problemset.pct --kind formula --expr "pct(solved, total)"
-->
<script lang="ts">
  import Sheet from '../shell/Sheet.svelte';
  import { IpcError, dispatch } from '../ipc';
  import { app } from '../session.svelte';

  let { type, onclose }: { type: string; onclose: () => void } = $props();

  let key = $state('');
  let kind = $state('text');
  let options = $state('');
  let expr = $state('');
  let to = $state('');
  let busy = $state(false);

  /** The closed vocabulary (§3.4), in the order the menu lists it. */
  const KINDS = [
    'text',
    'longtext',
    'number',
    'duration',
    'date',
    'daterange',
    'bool',
    'select',
    'multiSelect',
    'rating',
    'url',
    'relation',
    'formula',
    'json',
  ];
  const SELECTISH = ['select', 'multiSelect'];

  /**
   * A computed column, built from pickers rather than typed (§4.8: the only
   * genuine learning curve in the app is relations and calculated columns, and
   * it is meant to be *picker-driven*). Four shapes cover what a study plan
   * actually computes; the fifth is the L2 escape hatch, and it is still the
   * engine's evaluator either way.
   */
  const SHAPES = [
    { id: 'percent', label: 'Percent of two numbers', needs: 'number', compose: (a: string, b: string) => `pct(${a}, ${b})` },
    { id: 'days', label: 'Days between two dates', needs: 'date', compose: (a: string, b: string) => `daysBetween(${a}, ${b})` },
    { id: 'difference', label: 'One number minus another', needs: 'number', compose: (a: string, b: string) => `${a} - ${b}` },
    { id: 'hours', label: 'Minutes as hours, rounded', needs: 'minutes', compose: (a: string) => `round(${a} / 60)` },
    { id: 'custom', label: 'An expression of my own', needs: 'any', compose: (a: string, b: string) => `${a}` },
  ] as const;

  let shape = $state('percent');
  let left = $state('');
  let right = $state('');
  let preview = $state<string | null>(null);
  let previewing = $state(false);

  /** The fields a shape can compute from — the type's own, filtered by kind. */
  function candidates(kindWanted: string): string[] {
    const fields = app.types[type]?.fields ?? [];
    return fields
      .filter((field) =>
        kindWanted === 'number'
          ? field.type === 'number' || field.type === 'duration' || field.type === 'rating'
          : kindWanted === 'date'
            ? field.type === 'date' || field.type === 'daterange'
            : kindWanted === 'minutes'
              ? field.type === 'duration'
              : true,
      )
      .map((field) => field.key);
  }

  const current = $derived(SHAPES.find((candidate) => candidate.id === shape) ?? SHAPES[0]);
  const fields = $derived(candidates(current.needs));
  const composed = $derived(
    shape === 'custom'
      ? expr
      : current.needs === 'minutes'
        ? current.compose(left, '')
        : current.compose(left, right),
  );

  // Default the pickers to the first usable fields, so the common case needs no
  // clicks at all and the expression below is already a working example.
  $effect(() => {
    if (fields.length > 0 && !fields.includes(left)) left = fields[0];
    if (fields.length > 1 && !fields.includes(right)) right = fields[1];
  });

  /**
   * The live preview: the engine's own evaluator on one real record (§4.4). A
   * frontend that evaluated the expression to preview it would be a second
   * implementation, and the preview would then disagree with the committed
   * value — which is exactly what a preview is for.
   */
  let timer: ReturnType<typeof setTimeout> | null = null;
  let sampleId: string | null = null;
  $effect(() => {
    const text = composed;
    if (kind !== 'formula' || text.trim() === '') {
      preview = null;
      return;
    }
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      void (async () => {
        previewing = true;
        try {
          if (sampleId === null) {
            const records = await app.recordsOf(type);
            sampleId = records[0]?.id ?? '';
          }
          const result = await dispatch(
            'expr.eval',
            { expr: text, ...(sampleId ? { id: sampleId } : {}) },
            { plan: app.plan },
          );
          const data = result.data as { value?: unknown; type?: string };
          const shown =
            data.value === null || data.value === undefined ? 'null' : String(data.value);
          preview = `on ${sampleId || 'no record yet'} → ${shown} (${data.type})`;
        } catch (failure) {
          preview = failure instanceof IpcError ? failure.text : String(failure);
        } finally {
          previewing = false;
        }
      })();
    }, 250);
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  async function commit(): Promise<void> {
    if (key.trim().length === 0) {
      app.notice('a column needs a key — letters, numbers and _');
      return;
    }
    busy = true;
    try {
      const params: Record<string, unknown> = {
        spec: `${type}.${key.trim()}`,
        kind,
      };
      if (SELECTISH.includes(kind)) params.options = options;
      if (kind === 'formula') params.expr = composed.trim();
      if (kind === 'relation') params.to = to || Object.keys(app.types)[0] || '';
      const result = await app.run('column.new', params);
      if (result) onclose();
    } finally {
      busy = false;
    }
  }
</script>

<Sheet
  title="New column"
  subtitle={`${type} — the table header is the schema editor`}
  onclose={onclose}
>
  <div class="cd-formrow">
    <label class="cd-formrow__label" for="column-key">Key</label>
    <code class="cd-formrow__key">key</code>
    <input id="column-key" class="cd-wellfield" placeholder="est" bind:value={key} />
  </div>
  <div class="cd-formrow">
    <label class="cd-formrow__label" for="column-kind">Kind</label>
    <code class="cd-formrow__key">type</code>
    <select id="column-kind" class="cd-wellfield" bind:value={kind}>
      {#each KINDS as option (option)}<option value={option}>{option}</option>{/each}
    </select>
  </div>
  {#if SELECTISH.includes(kind)}
    <div class="cd-formrow">
      <label class="cd-formrow__label" for="column-options">Choices</label>
      <code class="cd-formrow__key">options</code>
      <input id="column-options" class="cd-wellfield" placeholder="easy, medium, hard" bind:value={options} />
    </div>
  {:else if kind === 'formula'}
    <div class="cd-formrow">
      <label class="cd-formrow__label" for="column-shape">Computed as</label>
      <code class="cd-formrow__key">expr</code>
      <select id="column-shape" class="cd-wellfield" bind:value={shape}>
        {#each SHAPES as option (option.id)}<option value={option.id}>{option.label}</option>{/each}
      </select>
    </div>
    {#if shape === 'custom'}
      <div class="cd-formrow">
        <label class="cd-formrow__label" for="column-expr">Expression</label>
        <code class="cd-formrow__key">expr</code>
        <input id="column-expr" class="cd-wellfield" placeholder="pct(solved, total)" bind:value={expr} />
      </div>
    {:else}
      <div class="cd-formrow">
        <label class="cd-formrow__label" for="column-left">
          {current.needs === 'date' ? 'From' : current.needs === 'minutes' ? 'Minutes' : 'First'}
        </label>
        <code class="cd-formrow__key">field</code>
        <select id="column-left" class="cd-wellfield" bind:value={left}>
          {#each fields as name (name)}<option value={name}>{name}</option>{/each}
        </select>
      </div>
      {#if current.needs !== 'minutes'}
        <div class="cd-formrow">
          <label class="cd-formrow__label" for="column-right">
            {current.needs === 'date' ? 'To' : 'Second'}
          </label>
          <code class="cd-formrow__key">field</code>
          <select id="column-right" class="cd-wellfield" bind:value={right}>
            {#each fields as name (name)}<option value={name}>{name}</option>{/each}
          </select>
        </div>
      {/if}
    {/if}
    <p class="cd-sheet__hint">
      {#if fields.length === 0 && shape !== 'custom'}
        this kind has no field of that shape yet — pick another shape, or write one yourself
      {:else}
        <code data-formula={composed}>{composed}</code>
      {/if}
    </p>
    <p class="cd-sheet__hint" data-preview={preview === null ? 'none' : 'value'} aria-live="polite">
      {#if previewing}
        evaluating…
      {:else if preview}
        {preview}
      {:else}
        the preview evaluates on one real record, through the engine's own evaluator
      {/if}
    </p>
  {:else if kind === 'relation'}
    <div class="cd-formrow">
      <label class="cd-formrow__label" for="column-to">Points at</label>
      <code class="cd-formrow__key">to</code>
      <select id="column-to" class="cd-wellfield" bind:value={to}>
        {#each Object.keys(app.types) as name (name)}<option value={name}>{name}</option>{/each}
      </select>
    </div>
  {/if}
  <div class="cd-sheet__actions">
    <button class="cd-pill" type="button" data-command="column.new" onclick={commit} disabled={busy}>
      Add column
    </button>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Cancel</button>
  </div>
</Sheet>
