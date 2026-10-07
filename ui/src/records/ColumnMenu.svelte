<!--
  The column menu (P2 · U5) — the header's own control, and the one place a
  column is reshaped.

  It speaks the student's language and carries the model beside it: every row
  names an action in words (`Column name`, `What it holds`, `Delete this
  column`), and every row carries the **identity pair** for the column it edits
  — the field's key, mono and copyable, so the file door is learnable from the
  menu without ever being required (D2, D13).

  Two rules from the plan are visible here:

  - **The consequence is at the control, before the write** (§4.2 R13). The
    delete row reads the engine's own dry run when the menu opens and states what
    would go in words — `17 values in 2 views` — never as engine JSON (D11).
  - **Undo is the escape, not a confirmation wall** (R5). Nothing here asks
    *are you sure*; the write is announced with its own Undo, and ⌘Z reverses it.

  The chip is the drag handle, the button that opens this menu, and the thing
  that says which column this is. It no longer *claims* a command id of its own:
  a control that opens a menu is not a second door for the menu's items (D3).
-->
<script lang="ts">
  import IdPair from '../shell/IdPair.svelte';
  import { COPY_JSON, COPY_PATH, columnMenuIds } from '../commands/registry';
  import { KIND_WORDS } from './columnKinds';
  import { labelOf, type FieldRead } from '../types';
  import { app } from '../session.svelte';

  let {
    field,
    type,
    view,
    previous,
    next,
    columns,
    ondrag,
    oncommand,
    onsheet,
  }: {
    field: FieldRead;
    type: string;
    view: string;
    /** The neighbour keys, so reorder names a destination instead of an index. */
    previous: FieldRead | null;
    next: FieldRead | null;
    /**
     * The order this view draws, as keys. A header move is a move *in this
     * view* — `view.setColumns` writes that list — because a kind's own field
     * order is not what the grid shows, and a control that moved the kind's
     * order would move nothing the student can see.
     */
    columns: string[];
    /** The drag is reported up: the drop owns the transaction (C.2's overrule). */
    ondrag: (key: string) => void;
    oncommand: (id: string, params: Record<string, unknown>) => void;
    onsheet: (field: FieldRead, kind: 'rename' | 'retype' | 'choices' | 'delete') => void;
  } = $props();

  type Row = {
    id: string;
    title: string;
    /** The destination a move addresses — the attribute an audit can read. */
    move?: 'left' | 'right';
    /** What this row would do, in words — the consequence, at the control. */
    note?: string;
    danger?: boolean;
    /** A destination that does not exist cannot be chosen (no dead ends). */
    disabled?: boolean;
    run: () => void;
  };

  let open = $state(false);
  /** What a delete would touch, read from `column.delete --dry-run` on open. */
  let consequence = $state<{ values: number; views: number } | null>(null);
  /** The address the count was read for — a plain handle, never reactive: an
   *  effect that reads what it writes re-runs forever. */
  let countedFor: string | null = null;

  const spec = $derived(`${type}.${field.key}`);
  /** The address contract: the field, by key (§4.3). */
  const pointer = $derived(`content/types.json#/types/${type}/fields/${field.key}`);
  const label = $derived(labelOf(field, field.key));

  /** The delete row's count, read once per open through the one dispatch path. */
  $effect(() => {
    if (!open) {
      countedFor = null;
      consequence = null;
      return;
    }
    const address = spec;
    if (countedFor === address) return;
    countedFor = address;
    void (async () => {
      const result = await app.run('column.delete', { spec: address, 'dry-run': true }, { tracked: false });
      if (!open || countedFor !== address) return;
      const data = result?.data as { recordsTouched?: number; views?: string[] } | undefined;
      consequence = result
        ? { values: Number(data?.recordsTouched ?? 0), views: (data?.views ?? []).length }
        : null;
    })();
  });

  /** The rows, in the order the declared menu lists them. */
  const rows = $derived.by<Row[]>(() => {
    const list: Row[] = [];
    for (const id of columnMenuIds(field)) {
      switch (id) {
        case 'column.rename':
          list.push({
            id,
            title: 'Column name',
            note: `shown as “${label}”`,
            run: () => onsheet(field, 'rename'),
          });
          break;
        case 'column.retype':
          list.push({
            id,
            title: 'What it holds',
            note: KIND_WORDS[field.type] ?? field.type,
            run: () => onsheet(field, 'retype'),
          });
          break;
        case 'column.choices':
          list.push({ id, title: 'The choices', note: 'what this one allows', run: () => onsheet(field, 'choices') });
          break;
        case 'column.duplicate':
          list.push({
            id,
            title: 'Duplicate this column',
            note: 'a second column, same kind',
            run: () => oncommand('column.duplicate', { spec }),
          });
          break;
        case 'column.hide':
          list.push({
            id,
            title: 'Hide from this view',
            note: 'the column keeps its values',
            run: () => oncommand('column.hide', { spec, view }),
          });
          break;
        case 'column.reorder':
          // The header shows the view's own column list, so a move here writes
          // that list: the row says which neighbour it swaps with, and the
          // write is the one the view editor performs.
          list.push({
            id: 'view.setColumns',
            title: 'Move left',
            move: 'left',
            note: previous ? `to the left, past ${labelOf(previous, previous.key)}` : 'it is already first',
            disabled: previous === null || view.length === 0,
            run: () => oncommand('view.setColumns', { name: view, columns: swapped(-1).join(',') }),
          });
          list.push({
            id: 'view.setColumns',
            title: 'Move right',
            move: 'right',
            note: next ? `to the right, past ${labelOf(next, next.key)}` : 'it is already last',
            disabled: next === null || view.length === 0,
            run: () => oncommand('view.setColumns', { name: view, columns: swapped(1).join(',') }),
          });
          break;
        case 'column.delete':
          list.push({
            id,
            title:
              consequence === null
                ? 'Delete this column'
                : `Delete this column — ${countText(consequence)}`,
            note: consequence === null ? 'reading what it holds…' : 'undoable, straight after',
            danger: true,
            run: () => onsheet(field, 'delete'),
          });
          break;
        case COPY_JSON:
          list.push({
            id,
            title: 'Copy as JSON',
            note: 'the whole column definition',
            run: () => copy(JSON.stringify(field, null, 2), `the ${field.key} definition`),
          });
          break;
        case COPY_PATH:
          list.push({ id, title: 'Copy the path', note: 'where the file keeps it', run: () => copy(pointer, 'the path') });
          break;
        default:
          break;
      }
    }
    return list;
  });

  /** This view's column order with the column moved one place either way. */
  function swapped(step: number): string[] {
    const from = columns.indexOf(field.key);
    const to = from + step;
    if (from < 0 || to < 0 || to >= columns.length) return columns;
    const next = [...columns];
    next[from] = columns[to];
    next[to] = field.key;
    return next;
  }

  /** `17 values in 2 views` — the count a delete would act on, in words. */
  function countText(count: { values: number; views: number }): string {
    const values = `${count.values} ${count.values === 1 ? 'value' : 'values'}`;
    if (count.views === 0) return values;
    return `${values} in ${count.views} ${count.views === 1 ? 'view' : 'views'}`;
  }

  async function copy(text: string, what: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      app.notice(`copied ${what}`);
    } catch {
      // A webview without clipboard permission is a real state; say so rather
      // than pretending the copy happened.
      app.notice(`cannot copy ${what} — the clipboard is unavailable here`);
    }
  }

  /** Close first, then act: one surface at a time (§11). */
  function run(row: Row): void {
    open = false;
    row.run();
  }

  /** Arrow keys walk the rows, so the menu is not a pointer-only surface. */
  function walk(event: KeyboardEvent): void {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    const buttons = Array.from(
      (event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('button'),
    );
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const step = event.key === 'ArrowDown' ? 1 : -1;
    const nextAt = (at + step + buttons.length) % buttons.length;
    buttons[nextAt]?.focus();
  }

  /** Escape closes the menu and puts the caret back on the chip that opened it. */
  function onkeydown(event: KeyboardEvent): void {
    if (!open || event.key !== 'Escape') return;
    open = false;
    const chip = document.querySelector<HTMLButtonElement>(`.cd-collabel[data-column="${CSS.escape(field.key)}"]`);
    chip?.focus();
  }

  /** A click outside the column closes it; a click inside it is the menu's. */
  function dismissOutside(event: MouseEvent): void {
    const target = event.target as Element | null;
    if (open && !target?.closest?.('.cd-colmenu')) open = false;
  }
</script>

<svelte:window onkeydown={onkeydown} onclick={dismissOutside} />

<div class="cd-colmenu">
  <button
    class="cd-collabel"
    type="button"
    aria-expanded={open}
    aria-haspopup="menu"
    aria-label={`${label} — the column menu`}
    data-column={field.key}
    data-placement="columnMenu"
    draggable="true"
    ondragstart={(event) => {
      // The chip is the drag handle; the drop target decides, and only the drop
      // writes (`RecordTable` owns that, per C.2's overrule).
      event.dataTransfer?.setData('text/plain', field.key);
      event.dataTransfer?.setDragImage?.(event.currentTarget as Element, 0, 0);
      ondrag(field.key);
    }}
    ondragend={() => ondrag('')}
    onclick={() => (open = !open)}
  >
    {label}
    {#if field.type === 'formula'}<span class="cd-fn" aria-label="computed">ƒ</span>{/if}
  </button>

  {#if open}
    <!-- The menu's own header is the identity pair: the name the column is read
         by, and the key the file knows it as. Every row carries the same key
         again, because every row edits that same column. -->
    <div class="cd-menu cd-menu--context cd-colmenu__menu" role="menu" onkeydown={walk}>
      <div class="cd-colmenu__head">
        <IdPair {label} value={field.key} copy={pointer} />
      </div>
      {#each rows as row (`${row.id}:${row.title}`)}
        <button
          type="button"
          role="menuitem"
          class:cd-danger={row.danger}
          disabled={row.disabled === true}
          data-command={row.id}
          data-move={row.move}
          onclick={() => run(row)}
        >
          <span class="cd-colmenu__what">
            <span class="cd-colmenu__title">{row.title}</span>
            {#if row.note}<span class="cd-colmenu__note">{row.note}</span>{/if}
          </span>
          <IdPair value={field.key} copy={pointer} />
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  /* The chip is the app's own species: the design system's label bar has no
     menu of its own. It stays a button on the header baseline. */
  .cd-collabel {
    cursor: pointer;
  }

  /* The menu is the system's `.cd-menu`; these are the two pieces a row of this
     menu needs — a title over its consequence, and the column's key beside it. */
  .cd-colmenu__menu {
    min-width: 260px;
  }
  .cd-colmenu__head {
    padding: var(--space-2xs) var(--space-sm) var(--space-xs);
    border-bottom: 1px solid var(--rule);
    margin-bottom: var(--space-3xs);
    font-size: var(--text-xs);
    color: var(--ink);
  }
  .cd-colmenu__menu button {
    align-items: center;
    gap: var(--space-sm);
  }
  .cd-colmenu__what {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: var(--space-3xs);
  }
  .cd-colmenu__title {
    font-size: var(--text-xs);
    color: inherit;
  }
  /* The consequence is a sentence, so it is smaller than the action it belongs
     to and it never wraps into a second column. */
  .cd-colmenu__note {
    font-size: var(--text-2xs);
    color: var(--ink-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
