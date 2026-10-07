/*
 * Settings surface component props.
 *
 * The screen is a **rail of places** beside **one pane of rows**. Everything
 * the design draws comes from these four records: the places themselves (each
 * with its live summary), the five panes' own snippets, the find index built
 * from the same table, and the session's change lines.
 *
 * `A · The Stencil` is the only design, so these types are the screen's own
 * contract rather than a variant's: a second design would keep them and change
 * only how it draws.
 */
import type { Snippet } from 'svelte';

/** The five places this screen holds, in the student's own order. */
export type SettingsPanelId = 'day' | 'work' | 'look' | 'styles' | 'plan';

/** One row of the find field's index — built from the page's own table. */
export type FindRow = {
  /** The place the setting lives in (a `SettingsPlace.id`). */
  group: SettingsPanelId;
  /** The app's own words for the setting. */
  name: string;
  /** The plate's own id — `data-plate` on the row a hit jumps to. */
  key: string;
};

/** One place in the rail: a name, a glyph, and where it stands right now. */
export type SettingsPlace = {
  id: SettingsPanelId;
  /** The place's own name, in the app's own words (`The day`). */
  name: string;
  /** A glyph from the icon family (`clock`, `checklist`, `character`, …). */
  icon: string;
  /**
   * The live one-line value, in the app's own words — `4 h a day ·
   * Asia/Kolkata · fixed gaps`. Drawn in the rail, so what is set is readable
   * without opening anything.
   */
  summary: string;
  /** One sentence over the pane's sheet, or `null` when the place has none. */
  lead: string | null;
};

/** One accepted write this session, reversible through the command that owns it. */
export type ChangeLine = {
  id: number;
  /** The setting's own name in the app's words. */
  label: string;
  /** The value it was at, and the value it is at, both in words. */
  from: string;
  to: string;
  /** True once the line's Undo has landed. The line stays, struck. */
  taken: boolean;
  /** Run the write that puts it back. */
  run: () => Promise<boolean>;
};

/** What a design receives. `A · The Stencil` is the only one that draws it. */
export type SettingsSurfaceProps = {
  /** The screen's own title (`Settings`). */
  title: string;
  /** The head's one sentence — what this page decides, in the app's words. */
  say: string;
  /** The head's measured tallies, e.g. `['5 places', '9 keys']`. */
  tally: string[];
  places: SettingsPlace[];
  /** The open place's rows, one snippet per place. All five are handed over;
   *  the design mounts the open one and keeps the rest hidden, because the
   *  registry's own inspection walks the whole document. */
  panels: Record<SettingsPanelId, Snippet>;
  find: FindRow[];
  changes: ChangeLine[];
  onUndoChange: (id: number) => void;
  onDone: () => void;
};
