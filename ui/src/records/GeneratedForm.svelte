<!--
  The generator (§4.2's claim, made twice): **one** grid over `[FieldDef]`
  produces the record form and the settings screen. A new type costs zero view
  code because nothing here knows a type.

  Each row is the identity pair — the friendly label with its key beside it
  (`IdPair`, D2) — and the control is the one `design/controls.md` §4 declares
  for the field's type: a switch for yes/no, a segment for 2–5 choices (all
  visible, one lit), a numeric field for a number or a duration, a date control
  for a date, the well field for text. A `*` marks a required field.

  The draft is `Record<string, String>`, which is a decision and not a shortcut:
  the form defers every typing question to the engine's one validator instead of
  guessing a number from a date, so the value that commits is the value the
  engine parsed — not the one a browser input coerced.
-->
<script lang="ts">
  import IdPair from '../shell/IdPair.svelte';
  import { labelOf, type FieldRead } from '../types';

  let {
    fields,
    draft = $bindable<Record<string, string>>({}),
    idField = false,
    idLabel = '(blank = auto)',
  }: {
    fields: FieldRead[];
    draft?: Record<string, string>;
    /** A record form takes an id; the settings screen does not. */
    idField?: boolean;
    idLabel?: string;
  } = $props();

  /** Derived values are never persisted, so they are never drafted (C.1). */
  const rows = $derived(fields.filter((field) => field.type !== 'formula' && field.type !== 'progress'));

  function inputType(field: FieldRead): string {
    if (field.type === 'number' || field.type === 'duration') return 'number';
    if (field.type === 'date') return 'date';
    return 'text';
  }

  function placeholder(field: FieldRead): string {
    if (field.type === 'duration') return '30';
    if (field.type === 'relation') return 'id, id';
    return '';
  }
</script>

<div class="cd-form">
  {#if idField}
    <div class="cd-formrow">
      <label class="cd-formrow__label" for="form-id">ID</label>
      <IdPair label={null} value="id" title={`Copy “${idLabel}”`} />
      <input id="form-id" class="cd-wellfield" bind:value={draft.id} placeholder={idLabel} />
    </div>
  {/if}
  {#each rows as field (field.key)}
    <div class="cd-formrow">
      <label class="cd-formrow__label" for={`form-${field.key}`}>{labelOf(field, field.key)}</label>
      <IdPair value={field.key} />
      {#if field.type === 'bool'}
        <button
          id={`form-${field.key}`}
          class="cd-switch cd-formrow__input"
          type="button"
          role="switch"
          aria-checked={draft[field.key] === 'true'}
          aria-label={labelOf(field, field.key)}
          onclick={() => (draft[field.key] = draft[field.key] === 'true' ? 'false' : 'true')}
        ></button>
      {:else if (field.type === 'select' || field.type === 'multiSelect') && (field.options ?? []).length > 0 && (field.options ?? []).length <= 5}
        <div class="cd-formrow__input cd-segment" role="radiogroup" aria-label={labelOf(field, field.key)}>
          {#each field.options ?? [] as option (option)}
            <button
              type="button"
              role="radio"
              aria-checked={draft[field.key] === option}
              onclick={() => (draft[field.key] = draft[field.key] === option ? '' : option)}
            >{option}</button>
          {/each}
        </div>
      {:else if field.type === 'select' || field.type === 'multiSelect'}
        <div class="cd-formrow__input">
          {#if (field.options ?? []).length > 0}
            <select id={`form-${field.key}`} class="cd-wellfield" bind:value={draft[field.key]}>
              <option value="">—</option>
              {#each field.options ?? [] as option (option)}<option value={option}>{option}</option>{/each}
            </select>
          {:else}
            <input id={`form-${field.key}`} class="cd-wellfield" placeholder="(no choices declared)" bind:value={draft[field.key]} />
          {/if}
        </div>
      {:else if field.type === 'longtext'}
        <textarea
          id={`form-${field.key}`}
          class="cd-wellfield cd-wellfield--area"
          rows="4"
          bind:value={draft[field.key]}
        ></textarea>
      {:else}
        <input
          id={`form-${field.key}`}
          class="cd-wellfield"
          type={inputType(field)}
          inputmode={field.type === 'number' || field.type === 'duration' ? 'numeric' : undefined}
          placeholder={placeholder(field)}
          bind:value={draft[field.key]}
        />
      {/if}
    </div>
  {/each}
</div>
