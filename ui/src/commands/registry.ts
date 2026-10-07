/**
 * The registry's frontend projection: what a menu contains, and how a binding
 * becomes this platform's own shortcut.
 *
 * Two rules from §4.8 are enforced here rather than in each screen:
 *
 * 1. **One list, three renderings.** A context menu's items are declared once
 *    (below), rendered as the OS's native menu when the shell offers one
 *    (`ipc.popupMenu`) and as DOM when it does not, and printed by
 *    `SAM --uicheck` as the expectation the parity gate diffs against. A menu
 *    item is always a registry id or a clipboard verb the dump names.
 * 2. **Accelerators are data.** A binding is `mod+shift+j`; `mod` resolves
 *    against `data-os`, and the glyph the user sees is computed, never typed.
 *    A macOS-only "⌘K" in a string would be the app's first hardcoded platform
 *    assumption.
 */
import type { FieldRead, KeybindingRead } from '../types';

/** The clipboard verbs — UI-only, and named by the `--uicheck` dump. */
export const COPY_JSON = 'copy-json';
export const COPY_PATH = 'copy-path';

export type MenuAction = {
  /** A registry id, or one of the two clipboard verbs. */
  id: string;
  title: string;
  /** The resolved key, when one is bound to this command. */
  hint?: string | null;
  /**
   * A second line under the title, for a menu whose rows are *objects* rather
   * than commands — the design list's one-line concept, the kind picker's
   * summary. Absent on every command menu, which is what keeps a command list
   * one line per row.
   */
  note?: string;
  danger?: boolean;
  enabled?: boolean;
  run: () => void | Promise<void>;
};

export type MenuRow = MenuAction | { separator: true };

/** The column menu's declared items — the dump prints this list. */
export function columnMenuIds(field: FieldRead): string[] {
  const ids = ['column.rename', 'column.retype'];
  // Appendix C.2's defect, resolved in the dump and here by reading the SAME
  // predicate: `column.choices` exists only for the select kinds.
  if (field.type === 'select' || field.type === 'multiSelect') ids.push('column.choices');
  ids.push('column.duplicate', 'column.hide', 'column.reorder', 'column.delete', COPY_JSON, COPY_PATH);
  return ids;
}

/** The row menu's declared items. */
export function rowMenuIds(): string[] {
  return [COPY_JSON, COPY_PATH, 'record.reveal', 'record.move', 'record.delete'];
}

/** The sidebar's declared items (Appendix C.8's sidebar owner). */
export function sidebarMenuIds(): string[] {
  return ['list.new', 'type.new', COPY_JSON];
}

/** The header `+` menu's declared items. */
export function columnPlusIds(): string[] {
  return ['column.show', 'column.new'];
}

export type Binding = {
  mod: boolean;
  shift: boolean;
  alt: boolean;
  ctrl: boolean;
  name: string;
};

const NAMED: Record<string, string> = {
  comma: ',',
  period: '.',
  slash: '/',
  space: ' ',
  enter: 'Enter',
  escape: 'Escape',
  tab: 'Tab',
  backspace: 'Backspace',
  minus: '-',
  equal: '=',
};

/** `mod+shift+j` → its parts. `null` for a binding this app cannot resolve. */
export function parseBinding(key: string): Binding | null {
  const tokens = key.split('+').map((token) => token.trim().toLowerCase());
  const name = tokens.pop();
  if (!name) return null;
  const binding: Binding = { mod: false, shift: false, alt: false, ctrl: false, name };
  for (const token of tokens) {
    if (token === 'mod') binding.mod = true;
    else if (token === 'shift') binding.shift = true;
    else if (token === 'alt' || token === 'opt') binding.alt = true;
    else if (token === 'ctrl') binding.ctrl = true;
    else return null;
  }
  if (!(name in NAMED) && !/^[a-z0-9]$/.test(name)) return null;
  return binding;
}

/** Does this event match this binding, on this platform? Exact modifiers only. */
export function matchesBinding(event: KeyboardEvent, binding: Binding, os: string): boolean {
  const modPressed = os === 'mac' ? event.metaKey : event.ctrlKey;
  if (binding.mod !== modPressed && !(os !== 'mac' && binding.mod && event.metaKey)) return false;
  if (binding.shift !== event.shiftKey) return false;
  if (binding.alt !== event.altKey) return false;
  if (binding.ctrl !== (os === 'mac' && event.ctrlKey)) return false;
  const name = NAMED[binding.name] ?? binding.name;
  return event.key.toLowerCase() === name.toLowerCase();
}

/** The platform's own rendering of a binding: ⌘⇧J on macOS, Ctrl+Shift+J elsewhere. */
export function glyph(key: string, os: string): string {
  const binding = parseBinding(key);
  if (!binding) return key;
  const parts: string[] = [];
  if (binding.ctrl || (binding.mod && os !== 'mac')) parts.push(os === 'mac' ? '⌃' : 'Ctrl');
  if (binding.alt) parts.push(os === 'mac' ? '⌥' : 'Alt');
  if (binding.shift) parts.push(os === 'mac' ? '⇧' : 'Shift');
  if (binding.mod && os === 'mac') parts.unshift('⌘');
  const name = NAMED[binding.name] ?? binding.name.toUpperCase();
  if (os === 'mac' && parts.length > 0 && name.length === 1) return `${parts.join('')}${name}`;
  return [...parts, name].join('+');
}

/** The key bound to a command, as the platform shows it. */
export function shortcutFor(bindings: KeybindingRead[], id: string, os: string): string | null {
  const key = bindings.find((binding) => binding.command === id)?.key;
  return key ? glyph(key, os) : null;
}

/**
 * The palette's matcher: case-insensitive substring matching over id, title,
 * category, and aliases (§4.4, Appendix C.3).
 */
export function matchesQuery(
  command: { id: string; title: string; category: string; aliases?: string[] },
  query: string,
): boolean {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) return true;
  // An alias is a name for the same command (§4.8 P2: one operation, one
  // mechanism), so searching it must find the command it names.
  return (
    command.id.toLowerCase().includes(needle) ||
    command.title.toLowerCase().includes(needle) ||
    command.category.toLowerCase().includes(needle) ||
    (command.aliases ?? []).some((alias) => alias.toLowerCase().includes(needle))
  );
}

/**
 * Which command a keydown names. Returns the registry id the app should run —
 * the keyboard is a projection of the same keymap the menu bar and the palette
 * display (§4.8 P2).
 */
export function commandForKey(
  event: KeyboardEvent,
  bindings: KeybindingRead[],
  os: string,
): string | null {
  for (const binding of bindings) {
    const parsed = parseBinding(binding.key);
    if (parsed && matchesBinding(event, parsed, os)) return binding.command;
  }
  return null;
}

/**
 * Undo has two owners (§4.10 trap 2): while a field has focus the field owns
 * ⌘Z, and the application's transaction stack is reached only from outside an
 * editor. This is that predicate, in one place.
 */
export function fieldOwnsUndo(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  return (
    tag === 'input' ||
    tag === 'textarea' ||
    target.isContentEditable ||
    Boolean(target.closest('[data-field-editor]'))
  );
}
