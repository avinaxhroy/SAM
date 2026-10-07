<!--
  The sheets a column and a record are changed in (Appendix C.9's one floating
  surface, R17: 460px, one question each).

  This session rewrote their *content*: every one of them now states what the
  write would do in words, read from the engine's own dry run, and none of them
  prints engine JSON as its evidence (D11 names that as the defect they replace —
  the old retype and delete sheets rendered `JSON.stringify(result.data)`).

  Three disciplines are visible below and all three are about *when* the engine
  is asked:

  - **Delete counts before it commits.** The open itself reads
    `column.delete --dry-run`, and the button carries the number it will act on
    (`Delete this column — 8 values`): the consequence at the control (R13, R5),
    with undo as the escape rather than a confirmation wall.
  - **A record that is still linked** is refused by the engine, and the refusal
    is the useful preview: the two policies it names are offered with their own
    counts read from their own dry runs, so nothing cascades by accident.
  - **Retype is two-phase, dry-run first**: *Preview* reads what the change would
    do to the values already stored, and only then does the button commit.

  No `window.confirm` and no `alert` anywhere in this file: §11's budget is one
  floating surface, and it is this one.
-->
<script lang="ts">
  import Sheet from '../shell/Sheet.svelte';
  import IdPair from '../shell/IdPair.svelte';
  import { COLUMN_KINDS, KIND_WORDS } from './columnKinds';
  import { labelOf, nameOf, recordLabel } from '../types';
  import { app } from '../session.svelte';

  /** What a destructive write would do, in the numbers the engine returned. */
  type Preview =
    | { kind: 'column.delete'; values: number; views: string[] }
    | { kind: 'record.delete'; safe: boolean; unlinks: number; cascades: number };

  let draft = $state<Record<string, string>>({});
  /**
   * The retype's dry run, as words (D11): either the change counts its values, or
   * the engine blocked it and names the records that hold a value the new kind
   * cannot read — both read from `column.retype --dry-run`, never from its JSON.
   */
  type Retype =
    | { state: 'counts'; converted: number; dropped: number; to: string }
    | { state: 'blocked'; names: string[]; to: string };
  let retype = $state<Retype | null>(null);
  let preview = $state<Preview | null>(null);
  /** A new choice being typed into the choice editor. */
  let extra = $state('');
  /** The engine's reason, when it refuses a write this sheet asked for. */
  let error = $state<string | null>(null);
  /** The address the preview was read for — plain, never reactive. */
  let readFor: string | null = null;

  const sheet = $derived(app.sheet);

  function close(): void {
    retype = null;
    preview = null;
    extra = '';
    error = null;
    app.closeSheet();
  }

  /**
   * One write, with its refusal kept where it was asked for: the reason renders
   * in this sheet (it is the surface that is open), and the toast is cleared
   * rather than repeating it where nothing can be done about it.
   */
  async function commit(id: string, params: Record<string, unknown>): Promise<void> {
    error = null;
    const result = await app.run(id, params);
    if (result) {
      // A write that lands says what it did, where it applies, with the one
      // action that reverses it (F14, R5): undo is the escape, not a wall.
      const line = receipt(id, params);
      if (line) app.notice(line, { label: 'Undo', run: () => void app.undo() });
      close();
      return;
    }
    error = app.lastDiagnostic ?? `the engine refused the ${id} write`;
    app.toast = null;
  }

  /** The receipt a landed write leaves: its subject in the student's words. */
  function receipt(id: string, params: Record<string, unknown>): string {
    switch (id) {
      case 'column.rename':
        return `The column reads as “${String(params.label)}” now`;
      case 'column.retype':
        return `The column holds ${KIND_WORDS[String(params.kind)] ?? String(params.kind)} now`;
      case 'column.choices':
        return 'The column’s choices are saved';
      case 'column.delete':
        return 'The column and its values are gone — undo brings them back';
      case 'record.delete':
        return 'The record is gone — undo brings it back';
      default:
        return '';
    }
  }

  /**
   * The numbers a destructive sheet states, read once per open through the one
   * dispatch path. A record that other records point at is refused by the
   * engine; that refusal is what makes its two policies worth offering, so the
   * sheet then asks each policy for its own count.
   */
  $effect(() => {
    const open = app.sheet;
    if (!open || (open.kind !== 'column.delete' && open.kind !== 'record.delete')) {
      readFor = null;
      preview = null;
      return;
    }
    const address =
      open.kind === 'column.delete' ? `column.delete:${open.type}.${open.field.key}` : `record.delete:${open.record.id}`;
    if (readFor === address) return;
    readFor = address;
    preview = null;
    void (async () => {
      if (open.kind === 'column.delete') {
        const result = await app.run(
          'column.delete',
          { spec: `${open.type}.${open.field.key}`, 'dry-run': true },
          { tracked: false },
        );
        if (readFor !== address) return;
        const data = result?.data as { recordsTouched?: number; views?: string[] } | undefined;
        if (!result) return;
        preview = {
          kind: 'column.delete',
          values: Number(data?.recordsTouched ?? 0),
          views: (data?.views ?? []).map(spokenView),
        };
        return;
      }
      const plain = await app.run('record.delete', { id: open.record.id, 'dry-run': true }, { tracked: false });
      if (readFor !== address) return;
      if (plain) {
        preview = { kind: 'record.delete', safe: true, unlinks: 0, cascades: 1 };
        return;
      }
      const [unlink, cascade] = await Promise.all([
        app.run('record.delete', { id: open.record.id, policy: 'unlink', 'dry-run': true }, { tracked: false }),
        app.run('record.delete', { id: open.record.id, policy: 'cascade', 'dry-run': true }, { tracked: false }),
      ]);
      if (readFor !== address) return;
      const unlinkData = unlink?.data as { unlink?: string[] } | undefined;
      const cascadeData = cascade?.data as { delete?: string[] } | undefined;
      preview = {
        kind: 'record.delete',
        safe: false,
        // `unlink` names the records that keep existing; `delete` names every
        // record that goes, this one included.
        unlinks: (unlinkData?.unlink ?? []).length,
        cascades: Math.max(0, (cascadeData?.delete ?? []).length - 1),
      };
    })();
  });

  /** A view named as a place: `reviews.queue` reads as “Reviews Queue” here. */
  const spokenView = (name: string): string =>
    app.navigation.find((entry) => entry.view === name)?.title ?? nameOf(name);

  /** `8 values in 2 views` — what a delete would act on, as a sentence. */
  const deleteWords = $derived.by(() => {
    const current = preview;
    if (!current || current.kind !== 'column.delete') return 'reading what it holds…';
    const values = `${current.values} ${current.values === 1 ? 'value' : 'values'}`;
    if (current.views.length === 0) return `${values}, and nothing draws it`;
    return `${values} in ${current.views.join(' · ')}`;
  });

  /** The kinds a column may be retyped to: the closed vocabulary, in words. */
  const kind = $derived(draft.kind ?? (sheet?.kind === 'column.retype' ? sheet.field.type : 'text'));

  /** Phase one asks, phase two commits — one click after a seen preview. */
  async function askRetype(type: string, key: string): Promise<void> {
    const params = {
      spec: `${type}.${key}`,
      kind,
      options: draft.options ?? '',
      expr: draft.expr ?? '',
    };
    if (retype?.state !== 'counts') {
      error = null;
      const result = await app.run('column.retype', { ...params, 'dry-run': true }, { tracked: false });
      if (!result) {
        // A preview the engine refuses is the most useful preview there is: it
        // says so here, at the control that asked, instead of doing nothing.
        error = app.lastDiagnostic ?? 'the engine will not make this change';
        app.toast = null;
        return;
      }
      const data = result.data as {
        blocked?: boolean;
        problems?: Array<{ record?: string }>;
        converted?: number;
        valuesDropped?: number;
        to?: string;
      } | undefined;
      if (data?.blocked === true) {
        // `problems[].record` addresses the value (`…jsonl#<id>/fields/<key>`);
        // only the record's id is needed, and the record is then spoken by its
        // own label rather than by the address the engine wrote (D2).
        const held = await app.recordsOf(type);
        const names = (data.problems ?? []).map((problem) => {
          const id = (problem.record ?? '').split('#').pop()?.split('/')[0] ?? '';
          const record = held.find((candidate) => candidate.id === id);
          return record ? recordLabel(record) : 'a record';
        });
        // Blocked is a state, not a draft: nothing to confirm, so the button
        // stays a *preview* button rather than offering a write the engine has
        // already said it will refuse.
        retype = { state: 'blocked', names, to: kind };
        return;
      }
      retype = {
        state: 'counts',
        converted: Number(data?.converted ?? 0),
        dropped: Number(data?.valuesDropped ?? 0),
        to: data?.to ?? kind,
      };
      return;
    }
    await commit('column.retype', params);
  }

  /** The choices editor's working list, as the comma string the engine reads. */
  const chosen = $derived(
    (draft.options ?? (sheet?.kind === 'column.choices' ? (sheet.field.options ?? []).join(', ') : ''))
      .split(',')
      .map((option) => option.trim())
      .filter((option) => option.length > 0),
  );
</script>

{#snippet failure()}
  {#if error}
    <!-- The sentence is the sheet's; the engine's own finding rides beside it
         marked as a door, because that text — a path, a record id, a field key —
         is exactly what the terminal prints and what the source pane opens
         (D2's four doors). It is never the only thing said. -->
    <p class="cd-sheet__error" role="alert">
      That change was refused. <span class="cd-sheet__why" data-developer>{error}</span>
    </p>
  {/if}
{/snippet}

{#if sheet?.kind === 'column.rename'}
  {@const field = sheet.field}
  <Sheet title="Column name" subtitle="The name is what the grid shows" onclose={close}>
    <div class="cd-sheet__field">
      <span class="cd-sheet__label"><IdPair label={labelOf(field, field.key)} value={field.key} /></span>
      <input
        id="rename-label"
        class="cd-sheet__well"
        autocomplete="off"
        placeholder={labelOf(field, field.key)}
        value={draft.label ?? field.label ?? ''}
        oninput={(event) => (draft.label = (event.currentTarget as HTMLInputElement).value)}
        aria-label="The name this column is shown by"
      />
    </div>
    <p class="cd-sheet__note">
      Records, views and formulas keep pointing at <code class="cd-code">{field.key}</code> — a
      name is how the column reads, not what it is.
    </p>
    {@render failure()}
    <div class="cd-sheet__actions">
      <button
        class="cd-pill"
        type="button"
        data-command="column.rename"
        onclick={() => commit('column.rename', { spec: `${sheet.type}.${field.key}`, label: draft.label ?? '' })}
      >
        Save the name
      </button>
      <button class="cd-pill cd-pill--quiet" type="button" onclick={close}>Cancel</button>
    </div>
  </Sheet>
{:else if sheet?.kind === 'column.retype'}
  {@const field = sheet.field}
  <Sheet title="What it holds" subtitle="Changing the kind rewrites every value in the column" onclose={close}>
    <div class="cd-sheet__field">
      <span class="cd-sheet__label"><IdPair label={labelOf(field, field.key)} value={field.key} /></span>
      <select
        id="retype-kind"
        class="cd-sheet__well cd-sheet__select"
        value={kind}
        aria-label="What this column should hold"
        onchange={(event) => {
          draft.kind = (event.currentTarget as HTMLSelectElement).value;
          retype = null;
          error = null;
        }}
      >
        {#each COLUMN_KINDS as entry (entry.kind)}<option value={entry.kind}>{entry.word}</option>{/each}
      </select>
    </div>
    {#if kind === 'formula'}
      <div class="cd-sheet__field">
        <span class="cd-sheet__label">How it is worked out</span>
        <input
          id="retype-expr"
          class="cd-sheet__well"
          autocomplete="off"
          placeholder="solved ÷ total"
          bind:value={draft.expr}
          aria-label="How this column is worked out"
        />
      </div>
    {:else if kind === 'select' || kind === 'multiSelect'}
      <div class="cd-sheet__field">
        <span class="cd-sheet__label">The choices it allows</span>
        <input
          id="retype-options"
          class="cd-sheet__well"
          autocomplete="off"
          placeholder="easy, medium, hard"
          bind:value={draft.options}
          aria-label="The choices this column allows"
        />
      </div>
    {/if}
    <p class="cd-sheet__note">
      Values the new kind cannot hold block the change; nothing is written until you confirm.
    </p>
    {#if retype?.state === 'counts'}
      <p class="cd-sheet__note" data-preview="column.retype">
        {retype.converted} {retype.converted === 1 ? 'value' : 'values'} become
        {KIND_WORDS[retype.to] ?? retype.to}{retype.dropped > 0
          ? ` · ${retype.dropped} cannot be read that way and would be dropped`
          : ' · nothing would be dropped'}.
      </p>
    {:else if retype?.state === 'blocked'}
      <p class="cd-sheet__note" data-preview="column.retype" role="status">
        {retype.names.slice(0, 3).join(' · ')}{retype.names.length > 3
          ? ` and ${retype.names.length - 3} more`
          : ''}
        {retype.names.length === 1 ? 'holds a value' : 'hold values'} the new kind cannot read, so the
        engine will not make this change. Fix
        {retype.names.length === 1 ? 'that value first' : 'those values first'}, or pick another kind.
      </p>
    {/if}
    {@render failure()}
    <div class="cd-sheet__actions">
      <button
        class="cd-pill"
        type="button"
        data-command="column.retype"
        onclick={() => void askRetype(sheet.type, field.key)}
      >
        {retype?.state === 'counts' ? 'Change it' : retype?.state === 'blocked' ? 'Check again' : 'Preview'}
      </button>
      <button class="cd-pill cd-pill--quiet" type="button" onclick={close}>Cancel</button>
    </div>
  </Sheet>
{:else if sheet?.kind === 'column.choices'}
  {@const field = sheet.field}
  <Sheet title="The choices" subtitle="What this column is allowed to hold" onclose={close}>
    <div class="cd-sheet__field">
      <span class="cd-sheet__label"><IdPair label={labelOf(field, field.key)} value={field.key} /></span>
      <div class="cd-chips" role="group" aria-label="The choices this column allows">
        {#each chosen as option (option)}
          <span class="cd-chip">
            {option}
            <button
              class="cd-chip__drop"
              type="button"
              aria-label={`Remove “${option}”`}
              onclick={() => (draft.options = chosen.filter((kept) => kept !== option).join(', '))}
            >✕</button>
          </span>
        {/each}
        {#if chosen.length === 0}
          <span class="cd-sheet__hint">No choices yet — add the first one below.</span>
        {/if}
      </div>
    </div>
    <div class="cd-sheet__row">
      <input
        class="cd-sheet__well"
        autocomplete="off"
        placeholder="Add a choice"
        bind:value={extra}
        aria-label="Add a choice"
      />
      <button
        class="cd-pill cd-pill--quiet cd-pill--sm"
        type="button"
        disabled={extra.trim().length === 0}
        onclick={() => {
          const value = extra.trim();
          if (value.length === 0) return;
          draft.options = [...chosen, value].join(', ');
          extra = '';
        }}
      >Add</button>
    </div>
    <p class="cd-sheet__note">
      {chosen.length} {chosen.length === 1 ? 'choice' : 'choices'} — a record whose value is not on
      the list is refused, so remove carefully.
    </p>
    {@render failure()}
    <div class="cd-sheet__actions">
      <button
        class="cd-pill"
        type="button"
        data-command="column.choices"
        onclick={() => commit('column.choices', { spec: `${sheet.type}.${field.key}`, options: chosen.join(', ') })}
      >
        Save the choices
      </button>
      <button class="cd-pill cd-pill--quiet" type="button" onclick={close}>Cancel</button>
    </div>
  </Sheet>
{:else if sheet?.kind === 'column.delete'}
  {@const field = sheet.field}
  <Sheet title="Delete this column" subtitle="Its values go with it in every record" onclose={close}>
    <div class="cd-sheet__field">
      <span class="cd-sheet__label"><IdPair label={labelOf(field, field.key)} value={field.key} /></span>
      <span class="cd-sheet__count">{deleteWords}</span>
    </div>
    <p class="cd-sheet__note">
      The engine counts what it would touch before it touches it. Undo brings the column and its
      values straight back.
    </p>
    {@render failure()}
    <div class="cd-sheet__actions">
      <button
        class="cd-pill cd-danger"
        type="button"
        data-command="column.delete"
        onclick={() => commit('column.delete', { spec: `${sheet.type}.${field.key}` })}
      >
        {preview?.kind === 'column.delete'
          ? `Delete this column — ${preview.values} ${preview.values === 1 ? 'value' : 'values'}`
          : 'Delete this column'}
      </button>
      <button class="cd-pill cd-pill--quiet" type="button" onclick={close}>Cancel</button>
    </div>
  </Sheet>
{:else if sheet?.kind === 'record.delete'}
  {@const record = sheet.record}
  <Sheet title="Delete this record" subtitle="Undo brings it back, straight after" onclose={close}>
    {#if !preview}
      <p class="cd-sheet__note" role="status">Reading what points at this record…</p>
    {:else if preview.kind === 'record.delete' && preview.safe}
      <p class="cd-sheet__note">
        Nothing else in the plan points at this record, so it goes on its own.
      </p>
    {:else if preview.kind === 'record.delete'}
      <p class="cd-sheet__note">
        {preview.unlinks} {preview.unlinks === 1 ? 'record' : 'records'} still point here. Choose
        what should happen to them — the count on each button is what the engine would write.
      </p>
    {/if}
    {@render failure()}
    <div class="cd-sheet__actions">
      {#if preview?.kind === 'record.delete' && preview.safe}
        <button
          class="cd-pill cd-danger"
          type="button"
          data-command="record.delete"
          onclick={() => commit('record.delete', { id: record.id })}
        >
          Delete this record
        </button>
      {:else if preview?.kind === 'record.delete'}
        <!-- §4.6: a delete with inbound links is refused unless the caller
             chooses a previewed policy. The click path offers exactly the two
             the diagnostic names, each with its own count. -->
        <button
          class="cd-pill cd-danger"
          type="button"
          data-command="record.delete"
          data-policy="unlink"
          onclick={() => commit('record.delete', { id: record.id, policy: 'unlink' })}
        >
          Delete it, keep the {preview.unlinks} that point here
        </button>
        <button
          class="cd-pill cd-danger"
          type="button"
          data-command="record.delete"
          data-policy="cascade"
          onclick={() => commit('record.delete', { id: record.id, policy: 'cascade' })}
        >
          Delete it and the {preview.cascades} that point here
        </button>
      {/if}
      <button class="cd-pill cd-pill--quiet" type="button" onclick={close}>Cancel</button>
    </div>
  </Sheet>
{/if}

<style>
  /* A declared choice is a chip that carries its own remove control: the design
     system's chip object, with one verb inside it. */
  .cd-chip__drop {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--chip-h);
    height: var(--chip-h);
    margin-inline-end: calc(-1 * var(--space-2xs));
    border-radius: var(--r-pill);
    color: var(--ink-3);
  }
  .cd-chip__drop:hover {
    color: var(--ink);
    background: var(--well-2);
  }
  /* The engine's own words, in the mono face the doors use. */
  .cd-sheet__why {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
  }
</style>