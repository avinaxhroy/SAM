/**
 * Component variant registry catalog.
 * Defines available design styles (A, B, C) per surface, grouping,
 * descriptions, and default variant configurations.
 */

export type VariantId = 'a' | 'b' | 'c';

/** Which band of the settings screen a surface is listed under. */
export type SurfaceGroup = 'day' | 'study' | 'machine';

export type VariantDef = {
  id: VariantId;
  /** The lab's own name for the design (`A · The Index`). */
  name: string;
  /** One line — the concept, from the lab's brief. */
  note: string;
};

export type SurfaceDef = {
  /** The folder under `ui/src/variants/`, and the lab's own target name. */
  key: string;
  title: string;
  /** Where it appears, in the app's words. */
  what: string;
  group: SurfaceGroup;
  variants: VariantDef[];
  /** The style a fresh install gets. */
  default: VariantId;
};

export const SURFACES: SurfaceDef[] = [
  /* ── THE DAY ─────────────────────────────────────────────────────────── */
  {
    key: 'today-focus',
    title: 'The session card',
    what: 'Today — what to begin, and for how long',
    group: 'day',
    default: 'a',
    variants: [
      { id: 'a', name: 'Slide to begin', note: 'The action is a band with an ink knob — drag it right; past the end it becomes the check.' },
      { id: 'b', name: 'The card that opens', note: 'The head row unfolds the reason and the session’s own actions beneath it.' },
      { id: 'c', name: 'The focus card', note: 'The demo’s own focus card — the course wash, the chip and tag, the why-line, Start with the length — and the arc that says how much of the course is covered.' },
    ],
  },
  {
    key: 'today-queue',
    title: 'The day’s queue',
    what: 'Today — everything the day holds',
    group: 'day',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Day’s Weight', note: 'Every block is drawn at its own estimate — a fifteen-minute errand is a slip, a two-hour set is a slab — and the day’s state is the block’s own fill, told once; the rest of a band waits behind its count.' },
      { id: 'b', name: 'The Filter Strip', note: 'Course and state pills over one list; the arithmetic is stated under the strip.' },
      { id: 'c', name: 'The State Pills', note: 'Late · Due · Planned — each pill carries its count as a badge.' },
    ],
  },
  {
    key: 'coming-up',
    title: 'The dated rows',
    what: 'Today’s “Coming up”, and the Mocks waiting list',
    group: 'day',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Deck', note: 'One dated card centre-stage with three peers behind it; the arrows move through it.' },
      { id: 'b', name: 'The Reveal', note: 'A scrolling stack in time order; each card assembles as it is read.' },
      { id: 'c', name: 'The Split', note: 'Date panel and body panel part as a row reaches the middle of the port.' },
    ],
  },
  {
    key: 'week-chart',
    title: 'The week chart',
    what: 'Today and Progress — the week, measured',
    group: 'day',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Grid', note: 'Seven day cards measuring from one shared floor, the aim a labelled rule.' },
      { id: 'b', name: 'The Fan', note: 'Seven leaning sticks stacked like a fanned hand; height is minutes.' },
      { id: 'c', name: 'The Shelf', note: 'Seven tiles; today’s is open on arrival and reads its distance from the target.' },
    ],
  },
  /* ── THE STUDY ───────────────────────────────────────────────────────── */
  {
    key: 'problemsets-due',
    title: 'The list card',
    what: 'A saved view drawn as a list — the record list card',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Day Tape', note: 'A printed tape ruled by day; each bar runs from today to its own due day.' },
      { id: 'b', name: 'The Spike', note: 'The view’s records threaded on a needle — same slips, any order, none redrawn.' },
      { id: 'c', name: 'The Tray', note: 'An in-tray whose lip is the boundary: Done and Later are its two spills.' },
    ],
  },
  {
    key: 'plan-spine',
    title: 'The term spine',
    what: 'Plan — the weeks and the work in them',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Deck', note: 'Week sheets you move through; the weeks you walked stack above with their dates.' },
      { id: 'b', name: 'The Workspace', note: 'A phase rail down the left, the week you are reading as the canvas beside it.' },
      { id: 'c', name: 'The Carousel', note: 'One week on the stage, the term to its left; the chips wheel past as you step.' },
    ],
  },
  {
    key: 'courses',
    title: 'The course tiles',
    what: 'Courses — the tiles, and the course detail',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Index', note: 'Text-forward and monochrome; the numeral is the progress language.' },
      { id: 'b', name: 'The Shelf', note: 'Washed tiles against each other, and a pip per topic on the detail.' },
      { id: 'c', name: 'The Next Step', note: 'One featured course and an ink hero naming the next topic.' },
    ],
  },
  {
    key: 'practice-banks',
    title: 'The bank rows',
    what: 'Practice — the banks and the logging',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Row That Opens', note: 'A row grows out of its place into its bench while the others give way.' },
      { id: 'b', name: 'Folders In A Rack', note: 'A 2×4 rack; opening one slides its logging card out of the pocket.' },
      { id: 'c', name: 'The Tactile Commit', note: 'The pen docks under the row; the cap squashes into the well to commit.' },
    ],
  },
  {
    key: 'mocks-calendar',
    title: 'The month and the waiting row',
    what: 'Mocks — the month grid, and setting a date',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Month That Opens', note: 'A calm month; a day’s card opens in flow under its own column.' },
      { id: 'b', name: 'The Day Picker Rail', note: 'One rail of 31 keys; picking a key answers with that day below.' },
      { id: 'c', name: 'Guided Steps', note: 'The waiting record’s own two steps, then the dated list beneath.' },
    ],
  },
  {
    key: 'reviews-recall',
    title: 'The recall card',
    what: 'Reviews — the card you answer',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Top Card', note: 'A visible deck; a grade sends the card off and the next rises into its place.' },
      { id: 'b', name: 'Three Stages', note: 'Prompt · Evidence · Grade — one panel at a time, the rail opening as you step.' },
      { id: 'c', name: 'The Split', note: 'Question left, answer right, one seam; the four grades unfold beneath.' },
    ],
  },
  {
    key: 'reviews-queue',
    title: 'The backlog',
    what: 'Reviews — what is waiting, and its next step',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Shelves', note: 'Four shelves, one per lateness window, each carrying its count in a bead.' },
      { id: 'b', name: 'The Board', note: 'Four trays of washed cards; acting on one re-lays the whole board.' },
      { id: 'c', name: 'The Inline Defer', note: 'The age capsule becomes its own day picker where the row stands.' },
    ],
  },
  {
    key: 'progress-stats',
    title: 'The week’s figures',
    what: 'Progress — the figures the week states',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Outline Numerals', note: 'The four figures as outline display type over seven quiet rods.' },
      { id: 'b', name: 'The Vessels', note: 'Each figure is a vessel filled to its own capacity, printed in words beneath.' },
      { id: 'c', name: 'The Dot Matrix', note: 'The term as a dot grid — a day is a dot; the figures sit beneath as rows.' },
    ],
  },
  {
    key: 'reference-library',
    title: 'The library rows',
    what: 'Library — the record rows and the filter',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Shelf', note: 'Records stand on a plank as spines; pulling one unfolds its page in place.' },
      { id: 'b', name: 'The Palette', note: 'One field with a chevron, and a deck of result cards that unfolds from it.' },
      { id: 'c', name: 'The Folders', note: 'Every course is a sleeve with the paper standing in it and its count on the pocket.' },
    ],
  },
  {
    key: 'record-table',
    title: 'The record table',
    what: 'Any record table — every kind’s rows',
    group: 'study',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Rail', note: 'A quiet table; the focused row opens a bay with a small tool rail in it.' },
      { id: 'b', name: 'The Profile', note: 'Bars, not a table; opening one unfolds that record’s profile in place.' },
      { id: 'c', name: 'The Morph', note: 'Tiles over the record’s own sheet, and a cell becomes its editor where it stands.' },
    ],
  },
  /* ── THE MACHINE ─────────────────────────────────────────────────────── */
  {
    key: 'system',
    title: 'System',
    what: 'The five places — kinds, columns, views, rules, schedules',
    group: 'machine',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Index', note: 'A plain index of rows; a row’s chip opens a sheet that owns the whole job.' },
      { id: 'b', name: 'The Workbench', note: 'Edited where it is read — a row opens inline under itself, the tree stays a tree.' },
      { id: 'c', name: 'The Ladder', note: 'One document of five sections, with a breadcrumb out to a full-page editor.' },
    ],
  },
  {
    key: 'record-panel',
    title: 'The record panel',
    what: 'The panel that opens beside the canvas for one record',
    group: 'machine',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Diagram', note: 'The record as a box on a drafting plate, wired to the records it points at.' },
      { id: 'b', name: 'The Counter Bank', note: 'Quantity columns are drums; pressing one opens the app’s own control.' },
      { id: 'c', name: 'The Band', note: 'The plate inks the gap between what was planned and what the plan logged.' },
    ],
  },
  {
    key: 'views',
    title: 'The record layouts',
    what: 'The block layer — List · Table · Board · Timeline · Calendar · Outline · Cards · Map',
    group: 'machine',
    default: 'a',
    variants: [{ id: 'a', name: 'One Set, Eight Shapes', note: 'Eight drawings over one set of records, each with the read’s own head.' }],
  },
  {
    key: 'palette',
    title: 'The palette',
    what: '⌘K — capture, actions and objects by name',
    group: 'machine',
    default: 'c',
    variants: [
      { id: 'a', name: 'The Index', note: 'The plan’s own names at a large step, with a plate that follows the live row.' },
      { id: 'b', name: 'The Sheet', note: 'The offers are die-cut objects on one field; committing peels one off the sheet.' },
      { id: 'c', name: 'The Plain', note: 'The palette in its own material, finished — no prop, nothing to explain.' },
    ],
  },
  {
    key: 'source-pane',
    title: 'The source pane',
    what: 'The engine’s own bytes for a record or a document',
    group: 'machine',
    default: 'a',
    variants: [{ id: 'a', name: 'The Door', note: 'A mono well, the engine’s findings, and the one Commit.' }],
  },
  {
    key: 'record-doors',
    title: 'The record doors',
    what: 'New and Paste — every sheet that files a record',
    group: 'machine',
    default: 'a',
    variants: [{ id: 'a', name: 'The Sheet', note: 'One grammar for every record door: identity head, rows, the commit alone in the foot.' }],
  },
  {
    key: 'sheet-catchup',
    title: 'The catch-up sheet',
    what: 'The sheet that moves an overdue backlog in one act',
    group: 'machine',
    default: 'a',
    variants: [
      { id: 'a', name: 'The Tray', note: 'Three date slots on the back wall; a card’s position is its state.' },
      { id: 'b', name: 'The Docked Drawer', note: 'The work docks to the window’s right edge under a curved leading edge.' },
      { id: 'c', name: 'The Slide-Up Panel', note: 'A bar on the window’s floor that rises into the working panel.' },
    ],
  },
  {
    key: 'timer-chrome',
    title: 'The running session',
    what: 'The titlebar’s timer, in every state',
    group: 'machine',
    default: 'a',
    variants: [
      { id: 'a', name: 'The notch slab', note: 'A fixed slug on the tools row; the session grows out of the window’s top edge.' },
      { id: 'b', name: 'The morphing pill', note: 'The clock and the length are one object; the end cap opens it in place.' },
      { id: 'c', name: 'The floor bar', note: 'A bar docked at the window’s floor — separate objects with air between them.' },
    ],
  },
  {
    key: 'settings',
    title: 'Settings',
    what: 'This screen — the five places and the exit',
    group: 'machine',
    default: 'a',
    variants: [
      {
        id: 'a',
        name: 'The Stencil',
        note: 'A rail of five places beside one pane of rows, a find field over both, and the session’s writes at the pane’s foot.',
      },
    ],
  },
];

/** The surfaces a person can actually choose between. */
export const CHOOSABLE: SurfaceDef[] = SURFACES.filter((surface) => surface.variants.length > 1);

/** The one place a key becomes a surface; a `Record`, because keys are static. */
export const SURFACE_BY_KEY: Record<string, SurfaceDef> = Object.fromEntries(
  SURFACES.map((surface) => [surface.key, surface]),
);
