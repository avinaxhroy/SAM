/**
 * System inspection surface variant component props.
 * Displays schema definitions, views, pipelines, destinations, and diagnostics.
 */
import type { PlaceId } from './places';

export type { PlaceId };

/** One of the machine's five places: a name, a mark and the sentence it says. */
export type Place = { id: PlaceId; label: string; mark: string; sub: string };

/** A chip's modifier — the app's own `.cd-chip--*` family, in its own words. */
export type ChipMod = 'outline' | 'risk' | 'ok' | 'info';

/** A chip: a small statement beside a row's facts. */
export type Chip = { label: string; mod?: ChipMod };

/**
 * One row of a place. `key` is the schema key the identity pair reads;
 * `facts` are plain spans; `chips` are the small statements; `action` is the
 * label of the one control that opens the row's editor, and `opens` says
 * whether that control is a door at all (a derived figure is read-only, and
 * its row still carries a door to its own detail).
 */
export type Row = {
  key: string;
  mark: string;
  title: string;
  facts: string[];
  chips: Chip[];
  action: string;
  opens: boolean;
  /** The read-only twin: when set, the row's right side is this, not an action. */
  terminal?: string;
  /** A sentence the row carries under its facts (a hint, never a key). */
  tail?: string;
};

/** A row group that is not the place's own index — the settings, the checks. */
export type Group = { heading: string; rows: Row[] };

/** One result of the machine's one search. */
export type Hit = {
  place: PlaceId;
  what: string;
  mark: string;
  title: string;
  key: string;
  meta: string;
};

/** A column of a kind, as its editor draws it. */
export type ColumnRow = {
  key: string;
  label: string;
  mark: string;
  holds: Chip;
  place: string;
  pinned: string;
  first: boolean;
  last: boolean;
  /** The CLI twin of this column's own write. */
  terminal: string;
};

/** One of the designed screens a destination may be given. */
export type PanelCard = { value: string; label: string; say: string; current: boolean };

/** One review schedule, as its radio card draws it. */
export type SchedulerCard = {
  id: string;
  label: string;
  brief: string;
  say: string;
  inUse: boolean;
};

/** One day of the plan's stored load, drawn as a meter row. */
export type LoadDay = { when: string; due: number; share: number };

/** A ladder's step, with the gate that guards it (or null). */
export type LadderStep = { stage: string; gate: string | null };

/** A ladder a kind could climb — the radio cards of the pipeline change. */
export type LadderOption = { name: string; label: string; stages: string; inUse: boolean };

/** One old stage and the new stages it may land on. */
export type MapRow = { from: string; targets: Array<{ to: string; label: string; chosen: boolean }> };

/** A ladder: its steps, its gates, and — on a kind — the choice of another. */
export type Ladder = {
  steps: LadderStep[];
  completeWhen: string | null;
  /** Present only where the ladder may be changed (a kind, not the place). */
  options?: LadderOption[];
  chosen?: string | null;
  strategy?: 'fresh' | 'carry' | null;
  map?: MapRow[];
  complete?: boolean;
};

/** A destructive row: the chip that opens the shipped delete sheet. */
export type Door = { label: string; command: string; key: string; title: string; note: string };

/** One derived figure, as its editor writes it: the five keys of its member in
 *  `content/rules.json` — a key the file does not carry is the empty string —
 *  and the declared views a fold may read (`view.kind` set: a composed screen
 *  holds no records). The fold's vocabulary is `REDUCES`, the mirror of the
 *  engine's own `BLOCK_REDUCES` (validator: `metric.bad-reduce`). */
export type FigureFields = {
  label: string;
  view: string;
  expr: string;
  reduce: string;
  unit: string;
  /** The views a fold may read — the stored name, and the words the pick shows
   *  it by (a dotted name is the identity pair's business, never page prose). */
  views: Array<{ value: string; label: string }>;
};

/**
 * The body of one row's editor. Which fields are present is decided by the
 * place; a design draws the ones it was handed and adds nothing.
 */
export type EditorBody = {
  place: PlaceId;
  /** The subject's own schema key — what a handler needs, never printed. */
  key: string;
  title: string;
  sub: string;
  /** The CLI twin of this subject's write, spelled for the door. */
  terminal: string;
  /** kinds: the columns, in the plan's own order. */
  columns?: ColumnRow[];
  /** kinds: the ladder the kind climbs, with its own assignment cards. */
  ladder?: Ladder | null;
  /** kinds: the add-a-column door (the label the control wears). */
  addColumn?: string | null;
  /** kinds: the privacy toggle — `type.setPrivate`, a state, not a ladder.
   *  `next` is the value the press would write, said out loud. */
  privacy?: { label: string; note: string; next: boolean } | null;
  /** screens: the door into the view's own editor — `view.edit`. */
  ownQuery?: boolean;
  /** kinds / screens: the delete door. */
  door?: Door | null;
  /** screens: the view's blocks, exactly as the file holds them. */
  blocks?: { view: string; nodes: ReadonlyArray<Record<string, unknown>> };
  /** screens: which designed screen the destination draws. */
  panels?: PanelCard[];
  /** rules: what the figure folds, and the keys its editor writes. */
  figure?: { folds: string; source: string; fields: FigureFields };
  /** scheduling: the schedules, the fixed gaps and the stored load. */
  schedulers?: SchedulerCard[];
  gaps?: number[] | null;
  load?: LoadDay[];
  /** The file this subject lives in — the file door's own target name. */
  file: string;
};

/** A projection's sentence: what changes, and then the rest of it. */
export type ProjectionLine = { lead: string; rest: string };

/**
 * The projection of one pending write: what changes, what does not, and the
 * doors. Only the accept carries the command id — the choice costs nothing.
 */
export type Projection = {
  command: string;
  accept: string;
  disabled: boolean;
  danger: boolean;
  busy: boolean;
  lines: ProjectionLine[];
  terminal: string;
  /** A dry run in flight, or a choice that is not complete yet. */
  pending: boolean;
  wait: string | null;
  error: string | null;
  /** The typed key a kind's delete asks for, when records go with it. */
  confirm: { label: string; value: string } | null;
};

/** One place on the rail, as the rail's own card draws it. */
export type RailRow = {
  view: string;
  title: string;
  mark: string;
  opens: string;
  missing: boolean;
  /** The CLI twin of this destination's own write. */
  terminal: string;
};

/** The strip under the title: the plan's own counts. */
export type Strip = { value: number; label: string };

/** The read-back: the sentence a landed write leaves, and the way back. */
export type Receipt = { text: string; undo: boolean };

/** One place's index, and the row groups that follow it. */
export type PlaceRows = { rows: Row[]; groups: Group[] };

/** Everything the panel hands a design. Data in, callbacks out — no fetching. */
export type SystemProps = {
  places: Place[];
  /** C is one document of five sections, so every place's rows are handed over;
   *  A and B read the one their place names. */
  index: Record<PlaceId, PlaceRows>;
  place: PlaceId;
  /** The head. */
  strip: Strip[];
  query: string;
  searching: boolean;
  /** The row whose editor is open, and the editor it opens. */
  openKey: string | null;
  editor: EditorBody | null;
  /** The full page a design may open (B's own page, C's editor route). */
  page: EditorBody | null;
  projection: Projection | null;
  receipt: Receipt | null;
  /** Recovery: the control's own state, and the engine's retained backups. */
  recovering: boolean;
  backups: Array<{ txid: string; when: string; documents: number; summary: string }> | null;
  restoring: string | null;
  /** The one search's results. */
  hits: Hit[];
  /** The rail, in its own order. */
  rail: RailRow[];
  /** Every icon name the plan already uses, plus the app's own chrome set —
   *  so a destination's glyph field suggests real names. */
  iconNames: string[];
  busy: boolean;
  acts: SystemActions;
};

/** Every write a design may ask for. Each one is dispatched by the panel. */
export type SystemActions = {
  onPlace: (id: PlaceId) => void;
  onQuery: (value: string) => void;
  onClear: () => void;
  onOpen: (key: string) => void;
  onClose: () => void;
  onPage: (key: string) => void;
  onBack: () => void;
  onRecover: () => void;
  onRestore: (txid: string) => void;
  onReloadBackups: () => void;
  onGoHit: (hit: Hit) => void;
  onFile: (file: string) => void;
  onRailName: (view: string, title: string) => void;
  onRailIcon: (view: string, icon: string) => void;
  /** A column's own verbs: `column.rename` / `retype` / `choices` / `delete` / `duplicate` / `reorder`. */
  onColumn: (command: string, kind: string, key: string, step?: number) => void;
  onNewColumn: (kind: string) => void;
  /** `type.setPrivate` — a kind's records stay behind, or travel. */
  onPrivate: (kind: string, next: boolean) => void;
  /** `view.edit` — the view's own query, in its editor. */
  onEditView: (view: string) => void;
  onDelete: (door: Door) => void;
  /** The three projections. */
  onChooseLadder: (kind: string, to: string) => void;
  onStrategy: (fresh: boolean) => void;
  onMapStage: (from: string, to: string) => void;
  onAccept: () => void;
  onCancel: () => void;
  onChooseSchedule: (id: string) => void;
  onGap: (index: number, step: number) => void;
  onGapsReset: () => void;
  /** `metric.set` — one key of the open figure, as its editor spells it. */
  onFigure: (key: 'label' | 'view' | 'expr' | 'reduce' | 'unit', value: string) => void;
  onChoosePanel: (view: string, panel: string) => void;
  onTyped: (value: string) => void;
  onUndo: () => void;
  onBlock: (command: string, params: Record<string, unknown>) => void;
};

/* ── the small words every design shares ───────────────────────────────── */

/** `3 records` / `1 record` — the app's own plural, never `3 record(s)`. */
export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

/** `weekStart` → `Week start`: a key spoken as words, never as it is stored. */
export function words(key: string): string {
  return key
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[._-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^./, (letter) => letter.toUpperCase());
}

/** The layout vocabulary, in words a student reads. */
export const LAYOUTS: Record<string, string> = {
  list: 'List',
  table: 'Table',
  board: 'Board',
  timeline: 'Timeline',
  calendar: 'Calendar',
  tree: 'Tree',
  cardGrid: 'Cards',
  graph: 'Graph',
};

/** The designed screens a destination can draw. */
export const PANELS: Record<string, string> = {
  today: 'Today',
  plan: 'Plan',
  subjects: 'Subjects',
  practice: 'Practice',
  mocks: 'Mocks',
  library: 'Library',
  notes: 'Notes',
  reviews: 'Reviews',
  progress: 'Progress',
};

/** What a column holds, in words. */
export const TYPES: Record<string, string> = {
  text: 'Text',
  longtext: 'Long text',
  number: 'A number',
  duration: 'Minutes',
  date: 'A date',
  bool: 'Yes or no',
  select: 'Pick one',
  multiSelect: 'Pick several',
  relation: 'Points at',
  url: 'A link',
  formula: 'Computed',
  progress: 'A share',
};

/** The mark a column's kind wears — the one thing a list of ten needs to scan. */
export const TYPE_MARKS: Record<string, string> = {
  text: 'character',
  longtext: 'textQuote',
  number: 'chart',
  duration: 'clock',
  date: 'calendar',
  bool: 'check',
  select: 'checklist',
  multiSelect: 'checklist',
  relation: 'route',
  url: 'globe',
  formula: 'waveform',
  progress: 'stairs',
};

/** The block vocabulary, in words (`Table`, `Two columns`). */
export const BLOCKS: Record<string, string> = {
  list: 'List',
  table: 'Table',
  board: 'Board',
  calendar: 'Calendar',
  timeline: 'Timeline',
  graph: 'Graph',
  tree: 'Tree',
  cardGrid: 'Card grid',
  stat: 'Stat',
  chart: 'Chart',
  callout: 'Callout',
  columns: 'Two columns',
  conditional: 'Conditional',
  repeat: 'Repeat',
  sparkline: 'Sparkline',
};

/** What a derived figure does with the column it folds. */
export const REDUCES: Record<string, string> = {
  sum: 'added up',
  count: 'counted',
  avg: 'averaged',
  min: 'the smallest',
  max: 'the largest',
};

/** A figure's unit. */
export const UNITS: Record<string, string> = {
  min: 'minutes',
  questions: 'questions',
  topics: 'topics',
};

/** What a check's severity means. */
export const SEVERITIES: Record<string, string> = {
  error: 'must be fixed',
  warning: 'advice',
  advisory: 'advice',
};

/**
 * The marks that are safe to hand `<Icon>`: the plan names icons this build may
 * not draw, and an unknown name renders an empty box. `Record`, not `Set` — a
 * static membership table read by name.
 */
export const MARKS: Record<string, true> = {
  bolt: true, book: true, bubble: true, calendar: true, character: true, checklist: true,
  clock: true, flag: true, globe: true, graduationcap: true, hammer: true, pencil: true,
  squareStack: true, textQuote: true, trayFull: true, waveform: true, dot: true, search: true,
  code: true, gear: true, plus: true, close: true, check: true, refresh: true, chart: true,
  play: true, route: true, cube: true, layout: true, stairs: true, undo: true, minus: true,
  arrowup: true, arrowdown: true,
};
