<!--
  The sidebar's record state (COMPOSER §4.8): a canvas row was pressed, so the
  plan's own fields come here to be edited — every editable type, through the
  same control family the table's cells use (FieldControl). Reveal and Delete
  are the record's two other doors; both are the app's existing sheets and
  commands, and neither is mirrored from anywhere.

  The head names the record by its kind first (the eyebrow) and its own label
  second, which is how every other surface in the app introduces a record; the
  close button is the one press back to the screen's own settings, and Escape
  is its twin (`Screen.svelte`).
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import FieldControl from '../../records/FieldControl.svelte';
  import { app } from '../../session.svelte';
  import { labelOf, nameOf, recordLabel, type FieldRead, type RecordDoc } from '../../types';

  let { id, type }: { id: string; type: string } = $props();

  let record = $state<RecordDoc | null>(null);
  let ready = $state(false);
  let targets = $state<Record<string, Array<{ id: string; label: string }>>>({});
  let writes = $state<Record<string, { state: 'saving' | 'saved' | 'failed'; reason?: string }>>({});
  /** The read's own guard — deliberately not `$state` (the `RecordDetail` rule). */
  const loaded = { key: null as string | null };

  const fields = $derived((app.types[type]?.fields ?? []) as FieldRead[]);

  $effect(() => {
    const revision = app.revision;
    const key = `${type}:${id}:${revision}`;
    if (key === loaded.key) return;
    loaded.key = key;
    ready = false;
    void (async () => {
      const declared = (app.types[type]?.fields ?? []) as FieldRead[];
      const found = (await app.recordsOf(type)).find((candidate) => candidate.id === id) ?? null;
      const next: Record<string, Array<{ id: string; label: string }>> = {};
      for (const field of declared) {
        if (field.type === 'relation' && field.to) {
          next[field.to] = (await app.recordsOf(field.to)).map((target) => ({
            id: target.id,
            label: recordLabel(target),
          }));
        }
      }
      if (loaded.key !== key) return;
      targets = next;
      record = found;
      ready = true;
    })();
  });

  /**
   * One `record.setField`, exactly as the table's cells dispatch it: `saving`
   * while the engine has it, `saved` for a moment, and on refusal the engine's
   * own words where the edit happened — never floated in a toast (F14).
   */
  async function commit(key: string, value: string | string[] | null): Promise<void> {
    const target = record;
    if (!target) return;
    writes = { ...writes, [key]: { state: 'saving' } };
    const result = await app.run('record.setField', { id: target.id, field: key, value });
    if (result) {
      writes = { ...writes, [key]: { state: 'saved' } };
      setTimeout(() => {
        if (writes[key]?.state !== 'saved') return;
        const next = { ...writes };
        delete next[key];
        writes = next;
      }, 1400);
      return;
    }
    const reason = app.lastDiagnostic ?? 'the engine refused the change';
    app.toast = null;
    writes = { ...writes, [key]: { state: 'failed', reason } };
  }
</script>

<header class="cmp-insp__head">
  <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
    <Icon name={app.types[type]?.icon || 'squareStack'} />
  </span>
  <div class="cmp-insp__who">
    <h2 class="cmp-insp__title">{record ? recordLabel(record) : 'Reading…'}</h2>
    <p class="cmp-insp__sub">{nameOf(type)}</p>
  </div>
  <div class="cmp-insp__headacts">
      <!-- Back to the screen's own settings, without putting the pane away
           (§4.8): a picked record is a place the pane can be in, not the pane's
           only subject. -->
      <button
        class="cd-iconbtn"
        type="button"
        aria-label="Back to the screen’s own settings"
        title="Back to the screen’s own settings"
        onclick={() => app.unpickRecord()}
      >
        <Icon name="chevronleft" />
      </button>
      <button
        class="cd-iconbtn"
        type="button"
        data-command="screen.inspector"
        data-placement="composer.inspector"
        aria-label="Close the editing sidebar"
        title="Put this pane away — press any component to bring it back"
        onclick={() => app.closeInspector()}
      >
        <Icon name="close" />
      </button>
  </div>
</header>

{#if !ready}
  <!-- The wait is shaped like the form that is coming — a stub per field, the
       label line and the value line — so the pane does not jump when the read
       lands, and the region states what it is doing in words. -->
  <div class="cmp-insp__skel" aria-busy="true">
    <p class="cd-sr" role="status">Reading the record…</p>
    {#each [0, 1, 2] as row (row)}
      <span class="cmp-insp__skelfield">
        <span class="cd-skel" style={`--w: ${row === 1 ? 30 : 24}%`}></span>
        <span class="cd-skel" style={`--w: ${row === 1 ? 62 : 44}%`}></span>
      </span>
    {/each}
  </div>
{:else if !record}
  <!-- The record is not in the plan any more: a stated fact (D8), with one
       press back to the screen's own settings. -->
  <p class="cmp-insp__note">
    This record is not in the plan any more. It may have been deleted, or moved to another kind.
  </p>
  <div class="cmp-insp__group">
    <button class="cmp-insp__act" type="button" onclick={() => app.unpickRecord()}>
      <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true"><Icon name="chevronleft" /></span>
      <span class="cmp-insp__acttext">
        <span class="cmp-insp__actname">Back to the screen</span>
        <span class="cmp-insp__actsay">Leave this record and edit the screen’s own settings</span>
      </span>
    </button>
  </div>
{:else if record}
  <div class="cmp-insp__fields">
    {#each fields as field (field.key)}
      <div class="cmp-insp__field">
        <span class="cmp-insp__label">{labelOf(field, field.key)}</span>
        <div class="cmp-insp__ctl">
          <FieldControl
            {field}
            {record}
            targets={field.type === 'relation' && field.to ? (targets[field.to] ?? []) : []}
            oncommit={(key, value) => void commit(key, value)}
          />
          {#if writes[field.key]?.state === 'saving'}
            <span class="cmp-insp__state" role="status">Saving…</span>
          {:else if writes[field.key]?.state === 'saved'}
            <span class="cmp-insp__state" role="status">Saved</span>
          {:else if writes[field.key]?.state === 'failed'}
            <span class="cmp-insp__state cmp-insp__state--bad" role="alert">That change was refused.</span>
            <span class="cmp-insp__dev" data-developer>{writes[field.key]?.reason} · {type}.{field.key} · {id}</span>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <div class="cmp-insp__group">
    <button
      class="cmp-insp__act"
      type="button"
      data-command="record.reveal"
      data-placement="composer.inspector"
      onclick={() => void app.reveal(record)}
    >
      <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true"><Icon name="code" /></span>
      <span class="cmp-insp__acttext">
        <span class="cmp-insp__actname">Reveal in source</span>
        <span class="cmp-insp__actsay">Open the file this record lives in</span>
      </span>
      <span class="cmp-insp__go" aria-hidden="true"><Icon name="chevronright" size={14} /></span>
    </button>
    <button
      class="cmp-insp__act cmp-insp__act--danger"
      type="button"
      data-command="record.delete"
      data-placement="composer.inspector"
      onclick={() => void app.openSheet({ kind: 'record.delete', record })}
    >
      <span class="cd-glyph cd-glyph--sm cd-glyph--danger" aria-hidden="true"><Icon name="close" /></span>
      <span class="cmp-insp__acttext">
        <span class="cmp-insp__actname">Delete record</span>
        <span class="cmp-insp__actsay">Take this record out of the plan</span>
      </span>
      <span class="cmp-insp__go" aria-hidden="true"><Icon name="chevronright" size={14} /></span>
    </button>
  </div>
{/if}
