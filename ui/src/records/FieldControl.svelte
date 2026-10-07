<!--
  Schema-driven inline field cell editor (Appendix C.1, §4.2, §4.7; design/controls.md §4, UI_SPEC §2.2).

  Maps `FieldDef` types to accessible input controls:
    - Precedence: Derived formulas and relations resolve before type switches.
    - Control mappings: text, number, duration chips, date picker with relative labels,
      select segments / dropdowns, multiSelect chip pickers, boolean checks, and relation search.
    - Relations: Dispatches array of IDs instead of comma-joined strings.
    - Commits: Dispatches `record.setField` without direct local state mutations.
-->
<script lang="ts">
  import { DURATIONS, type FieldRead, type RecordDoc } from '../types';
  import { app } from '../session.svelte';
  import { safeUrl } from '../safeUrl';

  let {
    field,
    record,
    targets = [],
    editable = true,
    oncommit,
  }: {
    field: FieldRead;
    record: RecordDoc;
    targets?: Array<{ id: string; label: string }>;
    editable?: boolean;
    /** A relation commits an id list; every other type commits text (§3.4). */
    oncommit: (key: string, value: string | string[] | null) => void;
  } = $props();

  let editing = $state(false);
  let draft = $state('');
  let open = $state(false);
  /**
   * Escape abandons, and abandoning must survive the blur that follows it.
   * Escape removes the editor, and a removed editor can still emit `blur` —
   * without this flag the commit runs anyway and Escape has *edited* the cell,
   * which is the opposite of what it says (Appendix C.2: "commits on blur
   * unless Escape was pressed").
   */
  let abandoned = false;

  /** §4.4: a derived value is never persisted, so it is never edited. */
  const isDerived = $derived(field.type === 'formula' || field.type === 'progress');
  const isRelation = $derived(field.type === 'relation');
  const value = $derived(isDerived ? record.derived?.[field.key] : record.fields[field.key]);
  const linked = $derived(record.links?.[field.key] ?? []);
  const options = $derived(field.options ?? []);
  const multi = $derived(Array.isArray(value) ? (value as unknown[]).map(String) : []);

  /** The tooltip every control carries: the JSON identity, never hidden (§4.8 P3). */
  const identity = $derived(
    `${field.key}: ${field.type}${field.label ? ` · ${field.label}` : ''}${isDerived ? ' — computed, read-only' : ''}`,
  );

  function durationText(minutes: number): string {
    if (!Number.isFinite(minutes) || minutes < 0) return String(minutes);
    if (minutes % 60 === 0) return `${minutes / 60}h`;
    if (minutes < 60) return `${minutes}m`;
    return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
  }

  /**
   * A date reads as its distance from the plan's today (`controls.md` §2's L1
   * hint: "in 23 days"), never as an ISO string. The engine owns today; this
   * only formats the gap.
   */
  function dateText(iso: string): string {
    const today = app.today?.date;
    if (!today || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return iso;
    const days = Math.round(
      (Date.parse(`${iso}T00:00:00Z`) - Date.parse(`${today}T00:00:00Z`)) / 86_400_000,
    );
    if (Number.isNaN(days)) return iso;
    if (days === 0) return 'today';
    if (days === 1) return 'tomorrow';
    if (days === -1) return 'yesterday';
    return days > 0 ? `in ${days} days` : `${-days} days ago`;
  }

  function plusDays(days: number): string {
    const today = app.today?.date;
    if (!today) return '';
    const base = Date.parse(`${today}T00:00:00Z`);
    const next = new Date(base + days * 86_400_000);
    return next.toISOString().slice(0, 10);
  }

  function display(): string {
    if (isRelation) return linked.length === 0 ? '—' : linked.map(labelFor).join(', ');
    if (value === undefined || value === null) return isDerived ? '—' : '';
    if (field.type === 'duration') return durationText(Number(value));
    if (field.type === 'date') return dateText(String(value));
    if (field.type === 'bool') return value ? '✓' : '·';
    if (field.type === 'multiSelect') return multi.join(', ');
    if (field.type === 'rating') return '★'.repeat(Number(value));
    // A computed float reads as a number a student would say: `88.57`, never
    // `88.57142857142857` (R8: truncation is a column's business; this is the
    // value's).
    if (typeof value === 'number') return String(Math.round(value * 100) / 100);
    return String(value);
  }

  /**
   * A cell's accessible name. A cell whose whole content is a glyph (or
   * nothing) says nothing to a screen reader, and a table cell is a control
   * here — so the name carries the column and what the cell shows.
   */
  function cellName(): string {
    const label = field.label ?? field.key;
    const shown = display();
    return shown && shown !== '—' && shown !== '·'
      ? `${label}: ${shown} — activate to edit`
      : `${label}: empty — activate to edit`;
  }

  /** The label a relation shows: `title` → `label` → `name` → the id (§C.1). */
  function labelFor(id: string): string {
    return targets.find((target) => target.id === id)?.label ?? id;
  }

  /** Which editor the cell opens: an inline field, or a popover at the cell. */
  const popover = $derived(field.type === 'date' || field.type === 'duration');

  function begin(): void {
    if (!editable || isDerived || isRelation || field.type === 'bool' || field.type === 'select') return;
    draft = value === undefined || value === null ? '' : String(value);
    abandoned = false;
    editing = true;
  }

  /**
   * Blur commits unless Escape was pressed — the rule the harvest insists on,
   * "because the undecided version is what makes a table feel broken".
   */
  function commit(): void {
    editing = false;
    if (abandoned) {
      abandoned = false;
      return;
    }
    const next = draft.trim();
    const before = value === undefined || value === null ? '' : String(value);
    if (next === before) return;
    // An empty value clears the field rather than writing "" — §3.7: defaults
    // apply to absent values, never to supplied ones.
    oncommit(field.key, next === '' ? null : next);
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Escape') {
      // Escape abandons: the field keeps what was published.
      abandoned = true;
      editing = false;
    }
  }

  function pick(option: string | null): void {
    open = false;
    oncommit(field.key, option);
  }

  /**
   * A relation commits as an id **list** (§C.1's overrule of the scaffold's
   * comma-joined transport): §4.7 lets an id contain a comma, so a joined
   * string is not a value — it is a value that has already lost information.
   */
  function link(id: string): void {
    open = false;
    const next = linked.includes(id) ? linked.filter((current) => current !== id) : [...linked, id];
    oncommit(field.key, next.length === 0 ? null : next);
  }

  /** A multi-chip field toggles one option and keeps the rest — also a list. */
  function toggleOption(option: string): void {
    const next = multi.includes(option)
      ? multi.filter((current) => current !== option)
      : [...multi, option];
    oncommit(field.key, next.length === 0 ? null : next);
  }

  /** §3.4: the scheme check lives in one place (`safeUrl.ts`) — this file's
   *  link offering and `window.open` use the same rule every anchor does. */

  function openLink(): void {
    const text = String(value ?? '');
    if (!safeUrl(text)) return;
    window.open(text, '_blank', 'noopener');
  }

  /** The duration editor: chips set it outright, the field types an exact value. */
  function pickDuration(minutes: number): void {
    open = false;
    oncommit(field.key, String(minutes));
  }

  /** The date editor: the control sets the day, the offsets land near it. */
  function pickDate(iso: string): void {
    open = false;
    if (iso) oncommit(field.key, iso);
  }

  function openDate(): void {
    draft = value === undefined || value === null ? '' : String(value);
    open = true;
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

{#if isDerived}
  <span class="cd-cell cd-cell--derived" {identity}>{display() || '—'}</span>
{:else if isRelation}
  <div class="cd-picker">
    <button
      class="cd-cell cd-cell--edit"
      type="button"
      {identity}
      aria-haspopup="menu"
      aria-expanded={open}
      aria-label={cellName()}
      data-command="record.setField"
      data-column={field.key}
      data-record-id={record.id}
      disabled={!editable}
      onclick={() => (open = !open)}
    >
      {#if linked.length === 0}
        —
      {:else}
        {linked.map(labelFor).join(', ')}
      {/if}
    </button>
    {#if open}
      <div class="cd-menu cd-menu--picker" role="menu">
        <button type="button" role="menuitem" onclick={() => pick(null)}>—</button>
        {#each targets as target (target.id)}
          <button type="button" role="menuitemradio" aria-checked={linked.includes(target.id)} onclick={() => link(target.id)}>
            {linked.includes(target.id) ? '✓ ' : ''}{target.label}
          </button>
        {/each}
        {#if targets.length === 0}
          <p class="cd-menu__empty">no {field.to ?? 'target'} records yet</p>
        {/if}
      </div>
    {/if}
  </div>
{:else if field.type === 'bool'}
  <button
    class="cd-check"
    type="button"
    {identity}
    data-command="record.setField"
    data-column={field.key}
    data-record-id={record.id}
    disabled={!editable}
    aria-label={`${field.label ?? field.key}: ${value ? 'set' : 'not set'}`}
    aria-pressed={Boolean(value)}
    onclick={() => oncommit(field.key, value ? 'false' : 'true')}
  >
    {#if value}
      <svg aria-hidden="true" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.4" stroke-linecap="round"><path d="M5 13l4.5 4.5L19 7" /></svg>
    {/if}
  </button>
{:else if field.type === 'select'}
  <div class="cd-picker">
    <button
      class="cd-cell cd-cell--edit"
      type="button"
      {identity}
      aria-haspopup="menu"
      aria-expanded={open}
      aria-label={cellName()}
      data-command="record.setField"
      data-column={field.key}
      data-record-id={record.id}
      disabled={!editable}
      onclick={() => (open = !open)}
    >
      {value === undefined || value === null ? '—' : String(value)}
    </button>
    {#if open}
      <!-- ≤ 5 options: every one visible, the chosen one lit (controls.md §2).
           More than five: the picker list. -->
      {#if options.length > 0 && options.length <= 5}
        <div class="cd-menu cd-menu--picker" role="radiogroup" aria-label={field.label ?? field.key}>
          <div class="cd-segment">
            {#each options as option (option)}
              <button type="button" role="radio" aria-checked={value === option} onclick={() => pick(option)}>
                {option}
              </button>
            {/each}
          </div>
          <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => pick(null)}>Clear</button>
        </div>
      {:else}
        <div class="cd-menu cd-menu--picker" role="menu">
          <!--
            "Clear", not an em dash. The ≤5 branch below already says the word,
            and this branch was the odd one out: a menu item whose whole content is
            `—` is announced as "em dash", which names nothing — the a11y gate
            caught it as "a control whose whole name is one character". An em dash
            in a *table cell* is a fine way to say "empty"; in a list of choices it
            is a choice with no label.
          -->
          <button type="button" role="menuitem" onclick={() => pick(null)}>Clear</button>
          {#each options as option (option)}
            <button type="button" role="menuitemradio" aria-checked={value === option} onclick={() => pick(option)}>
              {value === option ? '✓ ' : ''}{option}
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
{:else if field.type === 'date'}
  <div class="cd-picker">
    <button
      class="cd-cell cd-cell--edit"
      type="button"
      {identity}
      aria-haspopup="dialog"
      aria-expanded={open}
      aria-label={cellName()}
      data-command="record.setField"
      data-column={field.key}
      data-record-id={record.id}
      disabled={!editable}
      onclick={open ? () => (open = false) : openDate}
    >
      {display() || '—'}
    </button>
    {#if open}
      <div class="cd-menu cd-menu--picker cd-datepick" role="group" aria-label={field.label ?? field.key}>
        <input
          class="cd-cellinput"
          type="date"
          bind:value={draft}
          onkeydown={(event) => {
            if (event.key === 'Enter') pickDate(draft);
            else if (event.key === 'Escape') open = false;
          }}
        />
        <div class="cd-chips">
          <button class="cd-chip" type="button" onclick={() => pickDate(app.today?.date ?? '')}>Today</button>
          <button class="cd-chip" type="button" onclick={() => pickDate(plusDays(7))}>+1 week</button>
          <button class="cd-chip" type="button" onclick={() => pickDate(plusDays(30))}>+1 month</button>
        </div>
      </div>
    {/if}
  </div>
{:else if field.type === 'duration'}
  <div class="cd-picker">
    <button
      class="cd-cell cd-cell--edit"
      type="button"
      {identity}
      aria-haspopup="dialog"
      aria-expanded={open}
      aria-label={cellName()}
      data-command="record.setField"
      data-column={field.key}
      data-record-id={record.id}
      disabled={!editable}
      onclick={() => {
        draft = value === undefined || value === null ? '' : String(value);
        open = !open;
      }}
    >
      {display() || '—'}
    </button>
    {#if open}
      <div class="cd-menu cd-menu--picker cd-durpick" role="group" aria-label={field.label ?? field.key}>
        <div class="cd-chips">
          {#each DURATIONS as minutes (minutes)}
            <button class="cd-chip" type="button" aria-pressed={Number(value) === minutes} onclick={() => pickDuration(minutes)}>
              {minutes}m
            </button>
          {/each}
        </div>
        <input
          class="cd-cellinput"
          type="number"
          min="0"
          step="5"
          inputmode="numeric"
          aria-label={`${field.label ?? field.key} — minutes`}
          bind:value={draft}
          onkeydown={(event) => {
            if (event.key === 'Enter') pick(draft);
            else if (event.key === 'Escape') open = false;
          }}
        />
      </div>
    {/if}
  </div>
{:else if field.type === 'multiSelect'}
  <!-- ⚠ The scaffold shipped this read-only; a field type that cannot be
       written is a defect in a closed vocabulary (Appendix C.1). -->
  <div class="cd-chips" role="group" aria-label={field.key} {identity}>
    {#each options as option (option)}
      <button
        class="cd-chip"
        type="button"
        data-command="record.setField"
        data-column={field.key}
        data-record-id={record.id}
        disabled={!editable}
        aria-pressed={multi.includes(option)}
        onclick={() => toggleOption(option)}
      >
        {option}
      </button>
    {/each}
    {#if options.length === 0}
      <span class="cd-cell cd-cell--unknown">no choices declared</span>
    {/if}
  </div>
{:else if field.type === 'rating'}
  <div class="cd-stars" role="radiogroup" aria-label={`${field.label ?? field.key} (1–5)`} {identity}>
    {#each [1, 2, 3, 4, 5] as star (star)}
      <button
        class="cd-star"
        type="button"
        role="radio"
        aria-checked={Number(value) === star}
        aria-label={`${star} star${star === 1 ? '' : 's'}`}
        data-command="record.setField"
        data-column={field.key}
        data-record-id={record.id}
        disabled={!editable}
        onclick={() => oncommit(field.key, String(star))}
      >
        ★
      </button>
    {/each}
  </div>
{:else if field.type === 'url'}
  <span class="cd-url">
    <button
      class="cd-cell cd-cell--edit cd-url__text"
      type="button"
      {identity}
      aria-label={cellName()}
      data-command="record.setField"
      data-column={field.key}
      data-record-id={record.id}
      disabled={!editable}
      onclick={begin}
    >
      {display()}
    </button>
    {#if typeof value === 'string' && safeUrl(value)}
      <button class="cd-iconbtn cd-url__open" type="button" onclick={openLink} aria-label="Open link" title="open">↗</button>
    {/if}
  </span>
{:else if editable}
  {#if editing}
    <input
      class="cd-cellinput"
      type={field.type === 'number' ? 'number' : 'text'}
      inputmode={field.type === 'number' ? 'decimal' : undefined}
      bind:value={draft}
      onkeydown={keydown}
      onblur={commit}
      use:focusOnMount
      aria-label={field.key}
      data-field-editor
      data-column={field.key}
      data-record-id={record.id}
    />
  {:else}
    <button
      class="cd-cell cd-cell--edit"
      type="button"
      onclick={begin}
      {identity}
      aria-label={cellName()}
      data-command="record.setField"
      data-column={field.key}
      data-record-id={record.id}
    >
      {display()}
    </button>
  {/if}
{:else}
  <span class="cd-cell" {identity}>{display()}</span>
{/if}

<style>
  /* The two popover editors carry a little padding of their own; the menu
     object itself is the system's `.cd-menu`. */
  .cd-datepick,
  .cd-durpick { display: grid; gap: var(--space-xs); }
</style>
