/** Props for Palette surface variants. */
export type PaletteGroup = 'capture' | 'actions' | 'objects';

/** The two facts the live row's plate states, in the app's own words. */
export type PalettePlate = {
  /** `Writes` · `Runs` · `Lives in` · `Goes to`. */
  key: string;
  /** The consequence itself — never an id, always the app's own word. */
  value: string;
};

export type PaletteRow = {
  /** Unique across the three classes — what a keyed `{#each}` needs. */
  key: string;
  group: PaletteGroup;
  /** The row's name as drawn: `Add “…”` for a capture, else the app's own label. */
  label: string;
  /** The one consequence word. */
  word: string;
  /** A capture's typed sentence; null for the other two classes. */
  title: string | null;
  /** A capture's matched kind, or an object's own kind; null for an action. */
  kind: string | null;
  /** The live row's second line, where a design draws one. */
  plate: PalettePlate | null;
  /** The developer line (a field key, a registry id, a record id) — `data-developer`. */
  note: string | null;
  /**
   * The room a design may paint the row with: one of the plan's four washes, or
   * `none`. Read from the app's own identity rule (a record's assigned wash) and,
   * failing that, from the kind's place in the plan's declared kind order.
   */
  wash: string;
  /** Which die-cut silhouette the row is (B's own law; A and C ignore it). */
  shape: 'tag' | 'slab' | 'notch' | 'ticket';
  /** The registry id the row runs — what the parity census reads as a palette row. */
  command: string;
  /** The placement the shipped row carried (`palette` · `palette.capture` · `palette.objects`). */
  placement: string;
  /** An object row's noun (`screen` · `view` · `kind`) and the id its door addresses. */
  noun: string | null;
  target: string | null;
  /** A command that needs a plan with none open: drawn, never run. */
  blocked: boolean;
};

/** The run names, in the app's own words and the app's own order. */
export const GROUP_NAMES: Record<PaletteGroup, string> = {
  capture: 'Add',
  actions: 'Actions',
  objects: 'Find',
};

/** One row as a run holds it: the row, and its index in the flat list. */
export type PaletteRunRow = { row: PaletteRow; at: number };

/** One of the three runs: its class, the app's own name for it, and its rows. */
export type PaletteRun = { group: PaletteGroup; name: string; rows: PaletteRunRow[] };

/**
 * The rows gathered into the runs they arrived in — `Add` · `Actions` · `Find`,
 * never re-ordered. The `at` is the row's own index in the flat list, which is
 * what `aria-activedescendant` and the design's cursor address.
 */
export function groupsOf(rows: PaletteRow[]): PaletteRun[] {
  const runs: PaletteRun[] = [];
  rows.forEach((row, at) => {
    const last = runs[runs.length - 1];
    if (last && last.group === row.group) last.rows.push({ row, at });
    else runs.push({ group: row.group, name: GROUP_NAMES[row.group], rows: [{ row, at }] });
  });
  return runs;
}

export type PaletteProps = {
  /** The draft, owned by the shell (written to storage on every keystroke). */
  query: string;
  /** The three classes, flattened in their shipped order. */
  rows: PaletteRow[];
  /**
   * The app's own machine word for the debounced read (`Looking through the
   * plan…`, or the app's search-failed sentence); null when nothing is being
   * read. A design prints it where its own quiet line goes — never as prose.
   */
  status: string | null;
  /** A refused write's own sentence, printed where it was attempted. */
  error: string | null;
  /** Every keystroke, so the shell's draft is never a frame behind. */
  onQuery: (text: string) => void;
  /**
   * Perform the row. Resolves `true` when the palette leaves — a capture wrote
   * a record, a registry command ran — and `false` when it stays: an object row
   * opened its door, a write was refused, or the row needs parameters and the
   * app's own form is up. The design plays its exit beat on `true` and then
   * calls `onLeave`.
   */
  onCommit: (row: PaletteRow) => Promise<boolean>;
  /** The card has finished leaving: close the palette and state the outcome. */
  onLeave: () => void;
  /** Escape, without a commit: close, keeping the draft where it stands. */
  onDismiss: () => void;
};
