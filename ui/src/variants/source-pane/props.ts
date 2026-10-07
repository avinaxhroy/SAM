/**
 * Source pane component props (Appendix C.4).
 * Manages raw JSON/JSONL document inspection, validation diagnostics, and edits.
 */

/** One finding, exactly as `source.check` reports it (§4.9's Diagnostic). */
export type SourceFinding = {
  code: string;
  /** The engine's address for the finding: the file, and the pointer in it. */
  path: string;
  line: number | null;
  message: string;
};

/**
 * What the pane holds, as the app speaks it (D2): a record reads as its own
 * label with its id beside it, a document as its path alone — the engine has no
 * other name for a file, and the port does not invent one.
 */
export type SourceIdentity = {
  /** A record's own label (`recordLabel`); empty for a document. */
  label: string;
  /** The key beside the label: a record's id, or the document's path. */
  key: string;
  /** What the key copies: the engine's own address (`file#id`, or the path). */
  address: string;
};

/** One row of the target list: the pane's own subject first, then the engine's files. */
export type SourceOption = {
  /** The pane's own key for the row (`record:<id>` / `file:<path>`). */
  key: string;
  /** What the row reads: a record's label, or a file's path. */
  label: string;
  /** The mono tail after the label: `#ps7` on a record, empty on a file. */
  tail: string;
  /** True for the row the pane is pointed at. */
  current: boolean;
};

/** The save control's three states — the reference's own machine (`save-button.tsx`). */
export type SourceStatus = 'idle' | 'working' | 'done';

export type SourcePaneProps = {
  /** The engine's files, with the pane's subject in front (see the header). */
  options: SourceOption[];
  /** The plan-relative file the text belongs to — what `source.check`/`apply` need. */
  file: string;
  /** What the pane holds; null before the first read lands. */
  identity: SourceIdentity | null;
  /** The text in the well: a record's canonical line, or a document's bytes. */
  text: string;
  /** The engine's own diagnostics for the current text. */
  findings: SourceFinding[];
  /** A read is in flight: the well shows its own baselines, not a stale draft. */
  loading: boolean;
  /** Nothing to point at yet — the pane's stated empty case. */
  pointed: boolean;
  /** A revision landed under an open draft (§4.8 P6): offer Reload, never clobber. */
  external: boolean;
  /** The save control's state. */
  status: SourceStatus;
  /** The engine's own summary line for the last write (`source.apply <file>`), or null. */
  receipt: string | null;
  /** The commit accelerator, resolved from `data-os` by the screen (`glyph`). */
  commitKey: string;
  /** The pane's own toggle accelerator, for the close control's tooltip. */
  toggleKey: string;
  /** A row of the target list was pressed. */
  onPick: (key: string) => void;
  /** The user typed: the text becomes the session's draft. */
  onText: (text: string) => void;
  /** Commit the draft — check first, then write. */
  onCommit: () => void;
  /** Take the file as it is now, discarding the draft. */
  onReload: () => void;
  /** Close the pane: the same command as the titlebar's toggle. */
  onClose: () => void;
  /** Copy the engine's own address for what is open. */
  onCopy: () => void;
  /** A finding was hovered or focused (`null` when the pointer left). */
  onFlag: (finding: SourceFinding | null) => void;
};
