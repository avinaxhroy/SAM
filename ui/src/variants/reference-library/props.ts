/**
 * Component props and shared definitions for Reference Library variants
 * (A · The Shelf, B · The Palette, C · The Folders).
 */

/** One of the two reference destinations, labelled with the plan's own title. */
export type LibrarySegment = { type: string; label: string };

/** A course as a chip names it: its code for identity, its name for prose. */
export type LibraryCourse = {
  id: string;
  code: string | null;
  name: string;
};

/** One saved thing, as the three designs draw it. */
export type LibraryRecord = {
  id: string;
  /** The record's own kind — what the record door (`record.panel`) needs. */
  type: string;
  /** The record's own label (`recordLabel`). */
  title: string;
  /** The plan's word for this record's kind of thing: the `kind` field's own
   *  vocabulary set as a noun (`Book` · `PDF` · `Video` · `Article` · `Tool`),
   *  else the plan's noun for the record's type (`Note`). */
  kind: string;
  /** The glyph the kind draws with, from the app's own icon family. */
  glyph: string;
  /** The wash room the kind's mark and spine take (`mint` · `lilac` ·
   *  `butter` · `sky`) — the lab's `KIND_ROOM`, a drawing constant. */
  room: string;
  /** The kind's own thickness registers in px: the spine's width and height
   *  (the lab's `SPINE`). A drawing constant, scaled by `--ui-s` where used. */
  spine: { w: number; h: number };
  /** Where it came from: the provider's own name for a saved resource. */
  provider: string | null;
  /** The course it is filed under — the first of its own course relations,
   *  which is the one the shipped caption counted it in. */
  course: LibraryCourse | null;
  /** Its link, when it carries one — the anchor a design draws. */
  url: string | null;
  /** The note's own text, when the record has one to unfold in place. */
  body: string | null;
};

export type ReferenceProps = {
  /** The screen's own title (the destination's navigation title). */
  title: string;
  /** The two reference kinds the plan holds, labelled with its own words. */
  segments: LibrarySegment[];
  /** The kind being looked at — one of `segments`, or null when there is none. */
  segment: string | null;
  /** The plan's own courses, in its own order — what C's folders are built from
   *  and what a record's filing is named with. */
  courses: LibraryCourse[];
  /** Every record of the active kind, in the plan's order. A design counts
   *  against this (the lab's `n of m`), never against the filter's result. */
  records: LibraryRecord[];
  /** The records the query admits, in the same order (the design narrows, the
   *  screen decides — the admission rule is the lab's, see `matchesOf`). */
  matches: LibraryRecord[];
  /** What the student typed. A design draws its Clear only while this is not
   *  empty, so the control exists exactly when there is something to clear. */
  query: string;
  /** The command id the head's Add dispatches (`<kind>.new`), or null when the
   *  plan declares no creation door for this kind. */
  newCommand: string | null;
  onSegment: (type: string) => void;
  onQuery: (value: string) => void;
  onClear: () => void;
  onAdd: () => void;
  /** The record door — the app's own `record.panel` — for a record whose own
   *  design draws no link and no text to unfold. */
  onOpen: (id: string) => void;
};

/**
 * Visual geometry and color mappings for reference kind indicators and spine displays.
 */
const KIND_WORDS: Record<string, string> = {
  video: 'Video',
  pdf: 'PDF',
  article: 'Article',
  book: 'Book',
  tool: 'Tool',
};

const KIND_GLYPHS: Record<string, string> = {
  book: 'book',
  pdf: 'textQuote',
  article: 'textQuote',
  video: 'play',
  tool: 'hammer',
};

const REGISTERS: Record<string, { room: string; w: number; h: number }> = {
  book: { room: 'lilac', w: 44, h: 176 },
  pdf: { room: 'butter', w: 38, h: 164 },
  note: { room: 'sky', w: 40, h: 148 },
  other: { room: 'mint', w: 34, h: 136 },
};

/**
 * The four drawing facts a record's kind decides: its word, its glyph, the room
 * its mark and spine are washed in, and the two thickness registers a spine is
 * built from. The fallbacks are the plan's own — the type's noun and the
 * `icon` `types.json` declares for it — so a kind this file has never heard of
 * is named by the plan rather than invented here.
 */
export function kindFacts(
  type: string,
  value: string | null,
  typeNoun: string,
  typeGlyph: string,
): { kind: string; glyph: string; room: string; spine: { w: number; h: number } } {
  const word = (value ?? '').toLowerCase();
  const register = REGISTERS[word] ?? REGISTERS[type] ?? REGISTERS.other;
  return {
    kind: value ? (KIND_WORDS[word] ?? value.charAt(0).toUpperCase() + value.slice(1)) : typeNoun,
    glyph: KIND_GLYPHS[word] ?? typeGlyph,
    room: register.room,
    spine: { w: register.w, h: register.h },
  };
}

/** Returns the course code or name if assigned, else 'No course'. */
function filedAs(record: LibraryRecord): string {
  return record.course?.code ?? record.course?.name ?? 'No course';
}

/** Formats provider and course filing into a single caption line (D2). */
export function cameFrom(record: LibraryRecord): string {
  const filing = filedAs(record);
  return record.provider ? `${record.provider} · ${filing}` : filing;
}

const RX_ESC = /[.*+?^${}()|[\]\\]/g;

/**
 * The filter is an admission test, not a rank (the lab's `cmdk` insight, with
 * its score refused): a record matches if the query is a substring of it, OR if
 * every word of the query starts a word in it — so `intro alg` finds
 * “Introduction to Algorithms” and nothing that matched before stops matching.
 * A list that reorders itself under a typist's fingers is worse than one whose
 * order the reader already knows.
 */
function admitted(record: LibraryRecord, needle: string): boolean {
  const hay = [
    record.title,
    record.provider ?? '',
    record.kind,
    record.course?.code ?? '',
    record.course?.name ?? '',
  ]
    .join(' ')
    .toLowerCase();
  if (hay.includes(needle)) return true;
  const words = needle.split(/\s+/).filter(Boolean);
  if (words.length < 2) return false;
  return words.every((word) => new RegExp(`(^|[^a-z0-9])${word.replace(RX_ESC, '\\$&')}`).test(hay));
}

/** The records a query admits, in the order they were handed over. */
export function matchesOf(records: LibraryRecord[], query: string): LibraryRecord[] {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) return records;
  return records.filter((record) => admitted(record, needle));
}

/** One collection per course, in the plan's own course order, with the unfiled
 *  records as their own group **last** — being unfiled is work to do, not a
 *  course (the lab's rule, kept). */
export type Folder = { key: string; label: string; records: LibraryRecord[] };

/**
 * The folders C's board draws: built from the WHOLE library, so a folder a
 * filter emptied still knows how big it really is, and a course that holds
 * nothing of this kind is not drawn — a sleeve with no papers in it is a control
 * that cannot be operated.
 *
 * A record whose own relations name several courses is drawn once, in the first
 * — the same single-course rule the shipped caption counted with, kept here so
 * that a record cannot appear twice on one board and be two places to look.
 */
export function foldersOf(records: LibraryRecord[], courses: LibraryCourse[]): Folder[] {
  const byCourse = new Map<string, LibraryRecord[]>();
  const loose: LibraryRecord[] = [];
  for (const record of records) {
    if (!record.course) loose.push(record);
    else {
      const held = byCourse.get(record.course.id);
      if (held) held.push(record);
      else byCourse.set(record.course.id, [record]);
    }
  }
  const folders: Folder[] = [];
  for (const course of courses) {
    const held = byCourse.get(course.id);
    if (held && held.length > 0) folders.push({ key: course.id, label: course.name, records: held });
  }
  if (loose.length > 0) folders.push({ key: 'none', label: 'Without a course', records: loose });
  return folders;
}
