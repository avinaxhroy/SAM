<!--
  Record door sheet variant A.
  Modal form supporting record creation and paste import, field-level validation,
  and in-place kind switching (§11).
-->
<script lang="ts">
  import Sheet from '../../shell/Sheet.svelte';
  import type { DoorField, RecordDoorProps } from './props';

  let {
    mode,
    title,
    subtitle,
    kind,
    kindOptions,
    fixedKind,
    trailing,
    rows,
    busy,
    error = null,
    receipt = null,
    hint = '',
    idValue = '',
    idHint = '',
    buffer = '',
    hasHeader = false,
    delimiter = 'tab',
    columns = [],
    targets = [],
    count: countLine = '',
    lines = '',
    why = null,
    onInput,
    onPick,
    onKind,
    onRelation,
    onLink,
    onBuffer,
    onHeader,
    onDelimiter,
    onMap,
    onCommit,
    onCancel,
  }: RecordDoorProps = $props();

  /** The command this door dispatches, and the same id the control carries. */
  const command = $derived(`${kind}.${mode}`);
  const pill = $derived(
    receipt
      ? 'Done'
      : busy
        ? mode === 'new'
          ? 'Creating…'
          : 'Pasting…'
        : mode === 'new'
          ? 'Create'
          : `Paste ${lines}`,
  );
  /** A disabled commit is a door that cannot leave, and never a refusal: the
      reason is printed where the cause is (`why`), and the pill stays live. */
  const blocked = $derived(!receipt && (busy || why !== null));

  /** The licence to open the kind card lives in the drawing, not the door: it
      is how the control is being read, not what the record will be. */
  let kindsOpen = $state(false);

  /** The door's own kind stands first, then the plan's declaration order: the
      row the student is looking for is never below the fold. */
  const ordered = $derived([
    ...kindOptions.filter((option) => option.key === kind),
    ...kindOptions.filter((option) => option.key !== kind),
  ]);

  /** A `relation`'s own disclosure, and the reason a row announces itself. */
  function opened(row: DoorField): boolean {
    return row.control === 'relation' && row.open;
  }

  // The nested surface answers Escape before the sheet that holds it: a
  // capture-phase listener runs before `Sheet.svelte`'s own window listener, so
  // the kind card closes without the whole door going with it.
  function escapeCapture(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !kindsOpen) return;
    event.stopPropagation();
    kindsOpen = false;
  }
  // The reference's own dismissal: a pointerdown outside the open control
  // collapses it, and it runs before the click it belongs to.
  function outsideCapture(event: PointerEvent): void {
    if (!kindsOpen) return;
    if ((event.target as Element | null)?.closest?.('.rda-kind')) return;
    kindsOpen = false;
  }

  $effect(() => {
    globalThis.addEventListener('keydown', escapeCapture, true);
    globalThis.addEventListener('pointerdown', outsideCapture, true);
    return () => {
      globalThis.removeEventListener('keydown', escapeCapture, true);
      globalThis.removeEventListener('pointerdown', outsideCapture, true);
    };
  });
</script>

<Sheet {title} {subtitle} onclose={onCancel}>
  {#snippet head()}
    <div class="rda-idrow">
      {#if fixedKind}
        <!-- The same chip where the kind is a statement, not a control: Paste's
             mappings were made against one kind's keys, so this door does not
             offer what it would have to re-derive. -->
        <span class="rda-kind" data-fixed="true"><span class="rda-kind__n">{kind}</span></span>
      {:else}
        <span class="rda-kind" data-open={kindsOpen}>
          {#if kindsOpen}
            <div class="rda-kinds" role="listbox" aria-label="Kind">
              {#each ordered as option (option.key)}
                <button
                  class="rda-kinds__o"
                  type="button"
                  role="option"
                  aria-selected={option.key === kind}
                  onclick={() => {
                    kindsOpen = false;
                    onKind(option.key);
                  }}
                >
                  <span class="rda-kinds__k">{option.key}</span>
                  <span class="rda-kinds__n">{option.name}</span>
                  <span class="rda-kinds__g" aria-hidden="true">{option.key === kind ? '✓' : ''}</span>
                </button>
              {/each}
            </div>
          {:else}
            <button
              class="rda-kind__b"
              type="button"
              aria-haspopup="listbox"
              aria-expanded="false"
              aria-label={`Kind — ${kind}`}
              onclick={() => {
                kindsOpen = true;
              }}
            >
              <span class="rda-kind__n">{kind}</span>
              <span class="rda-kind__g" aria-hidden="true">⌄</span>
            </button>
          {/if}
        </span>
      {/if}
      {#if trailing}<span class="rda-tail">{trailing}</span>{/if}
      <!-- The one identity pair: the command this door dispatches, beside the
           kind. It is read, not copied, and marked as the identity pair the
           census exempts. -->
      <span class="rda-key" data-identity>{command}</span>
    </div>
  {/snippet}

  {#if receipt}
    <!-- A receipt, not a close: what the write left behind. -->
    <div class="rda-receipt">
      <p class="rda-receipt__t">{receipt.title}</p>
      {#if receipt.rows.length > 0}
        <div class="rda-read">
          {#each receipt.rows as line (line.key)}
            <div class="rda-read__r">
              <span class="rda-read__k">
                <span>{line.label}</span>
                <span class="rda-key" data-identity>{line.key}</span>
                {#if line.derived}
                  <span class="rda-d" title="computed from the record">ƒ</span>
                {/if}
              </span>
              <span class="rda-read__v">{line.value}</span>
            </div>
          {/each}
        </div>
      {/if}
      <p class="rda-read__f">{receipt.note}</p>
    </div>
  {:else if mode === 'new'}
    <div class="cd-sheet__field" data-invalid={error?.row === 'id' ? 'true' : undefined}>
      <label class="cd-sheet__label" for="new-record-id">
        ID
        <span class="cd-sheet__hint">{idHint}</span>
        <span class="cd-sheet__key" data-identity>id</span>
      </label>
      <input
        id="new-record-id"
        class="cd-sheet__well"
        type="text"
        autocomplete="off"
        placeholder={idHint}
        value={idValue}
        oninput={(event) => onInput('id', (event.currentTarget as HTMLInputElement).value)}
      />
    </div>
    {#if error?.row === 'id'}
      <p class="rda-err" role="alert">{error.message}</p>
    {/if}

    {#each rows as row (row.key)}
      <div class="cd-sheet__field" data-invalid={error?.row === row.key ? 'true' : undefined}>
        <span class="rda-rowl">
          <span class="rda-rowl__n">{row.label}</span>
          <span class="cd-sheet__key" data-identity>{row.key}</span>
          {#if row.control === 'relation'}<span class="cd-sheet__hint">→ {row.to}</span>{/if}
        </span>

        {#if row.control === 'bool'}
          <button
            class="cd-sheet__toggle"
            type="button"
            aria-pressed={row.value}
            onclick={() => onPick(row.key, row.value ? 'false' : 'true')}
          >
            <span class="cd-check" aria-hidden="true">{row.value ? '✓' : ''}</span>
            {row.label}
          </button>
        {:else if row.control === 'relation'}
          <div class="cd-picker">
            <button
              class="cd-sheet__well cd-sheet__select"
              type="button"
              aria-haspopup="menu"
              aria-expanded={opened(row)}
              aria-label={`${row.label} — choose`}
              onclick={() => onRelation(row.key)}
            >
              {#if row.chosen.length === 0}
                —
              {:else}
                {row.chosen
                  .map((id) => row.targets.find((target) => target.id === id)?.label ?? id)
                  .join(', ')}
              {/if}
            </button>
            {#if opened(row)}
              <div class="cd-menu cd-menu--picker" role="menu">
                {#each row.targets as target (target.id)}
                  <button
                    type="button"
                    role="menuitemradio"
                    aria-checked={row.chosen.includes(target.id)}
                    onclick={() => onLink(row.key, target.id)}
                  >
                    {row.chosen.includes(target.id) ? '✓ ' : ''}{target.label}
                  </button>
                {/each}
                {#if row.targets.length === 0}
                  <p class="cd-menu__empty">nothing to link to yet</p>
                {/if}
              </div>
            {/if}
          </div>
        {:else if row.control === 'segment'}
          <div class="cd-segment" role="radiogroup" aria-label={row.label}>
            {#each row.options as option (option)}
              <button
                type="button"
                role="radio"
                aria-checked={row.value === option}
                onclick={() => onPick(row.key, row.value === option ? '' : option)}
              >{option}</button>
            {/each}
          </div>
        {:else if row.control === 'select'}
          <select
            class="cd-sheet__well cd-sheet__select"
            aria-label={row.label}
            value={row.value}
            onchange={(event) => onPick(row.key, (event.currentTarget as HTMLSelectElement).value)}
          >
            <option value="">—</option>
            {#each row.options as option (option)}
              <option value={option}>{option}</option>
            {/each}
          </select>
        {:else if row.control === 'multi'}
          <div class="cd-chips" role="group" aria-label={row.label}>
            {#each row.options as option (option)}
              <button
                class="cd-chip"
                type="button"
                aria-pressed={row.values.includes(option)}
                onclick={() => onPick(row.key, option)}
              >{option}</button>
            {/each}
            {#if row.options.length === 0}
              <span class="cd-sheet__hint">no choices declared</span>
            {/if}
          </div>
        {:else if row.control === 'longtext'}
          <textarea
            class="cd-sheet__well"
            rows="3"
            aria-label={row.label}
            value={row.value}
            oninput={(event) => onInput(row.key, (event.currentTarget as HTMLTextAreaElement).value)}
          ></textarea>
        {:else}
          <input
            class="cd-sheet__well"
            type={row.control === 'number' || row.control === 'duration' ? 'number' : row.control === 'date' ? 'date' : 'text'}
            inputmode={row.control === 'number' || row.control === 'duration' ? 'numeric' : undefined}
            autocomplete="off"
            aria-label={row.label}
            placeholder={row.placeholder ?? ''}
            value={row.value}
            oninput={(event) => onInput(row.key, (event.currentTarget as HTMLInputElement).value)}
          />
        {/if}
      </div>
      {#if error?.row === row.key}
        <p class="rda-err" role="alert">
          {#if error.at}<span class="rda-err__at">{error.at}</span>{/if}
          <span>{error.message}</span>
          {#if error.code}<span class="rda-err__c">[{error.code}]</span>{/if}
        </p>
      {/if}
    {/each}

    {#if rows.length === 0}
      <p class="cd-empty">
        {kind} declares no settable fields — the engine can still create a bare record.
      </p>
    {/if}
  {:else}
    <div class="rda-modes">
      <button
        class="cd-sheet__toggle"
        type="button"
        aria-pressed={hasHeader}
        onclick={() => onHeader?.(!hasHeader)}
      >
        <span class="cd-check" aria-hidden="true">{hasHeader ? '✓' : ''}</span>
        First row is a header
      </button>
      <span class="cd-sheet__spacer"></span>
      <div class="cd-segment" role="radiogroup" aria-label="Delimiter">
        <button
          type="button"
          role="radio"
          aria-checked={delimiter === 'tab'}
          onclick={() => onDelimiter?.('tab')}
        >Tab</button>
        <button
          type="button"
          role="radio"
          aria-checked={delimiter === 'comma'}
          onclick={() => onDelimiter?.('comma')}
        >Comma</button>
      </div>
    </div>

    <textarea
      class="cd-sheet__well cd-sheet__data rda-data"
      rows="7"
      spellcheck="false"
      aria-label={`Paste ${kind} rows`}
      placeholder={delimiter === 'tab' ? 'title\tdue\tchapter' : 'title,due,chapter'}
      value={buffer}
      oninput={(event) => onBuffer?.((event.currentTarget as HTMLTextAreaElement).value)}
    ></textarea>

    <p class="rda-count">{countLine}</p>

    {#if columns.length === 0}
      <p class="cd-empty">Nothing pasted yet — the columns appear here as you type.</p>
    {:else}
      <div class="cd-sheet__maps">
        {#each columns as column (column.index)}
          <div class="cd-sheet__map">
            <span class="cd-sheet__maphead">{column.name}</span>
            <select
              class="cd-sheet__well cd-sheet__select"
              aria-label={`Column ${column.index + 1}`}
              value={column.target}
              onchange={(event) => onMap?.(column.index, (event.currentTarget as HTMLSelectElement).value)}
            >
              {#each targets as target (target.value)}
                <option value={target.value}>{target.label}</option>
              {/each}
            </select>
          </div>
        {/each}
      </div>
    {/if}

    {#if error}
      <p class="rda-err" role="alert" id="rda-paste-why">
        {#if error.at}<span class="rda-err__at">{error.at}</span>{/if}
        <span>{error.message}</span>
        {#if error.code}<span class="rda-err__c">[{error.code}]</span>{/if}
      </p>
    {:else if why}
      <p class="rda-err" role="alert" id="rda-paste-why">
        <span>{why}</span>
      </p>
    {/if}
  {/if}

  {#snippet footer()}
    <span class="cd-sheet__hint">{hint}</span>
    <span class="cd-sheet__spacer"></span>
    {#if receipt}
      <button class="cd-pill" type="button" onclick={onCancel}>Done</button>
    {:else}
      <button class="cd-pill cd-pill--quiet" type="button" onclick={onCancel}>Cancel</button>
      <button
        class="cd-pill"
        type="button"
        data-command={command}
        aria-disabled={blocked}
        aria-busy={busy}
        aria-describedby={why ? 'rda-paste-why' : undefined}
        onclick={onCommit}
      >
        {pill}
      </button>
    {/if}
  {/snippet}
</Sheet>
