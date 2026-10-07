/**
 * The kinds a column can hold, each in the words a student would use for it.
 *
 * One list, two readers — the column menu says what a column *holds*, and the
 * retype sheet asks what it should hold instead — so the vocabulary is declared
 * once instead of drifting between the two. The keys on the right are the
 * engine's own (`SAM_PLAN.md` §3.4's closed vocabulary); the words on the left
 * are the only form a screen shows (`UI_PLAN` D2).
 */
export const COLUMN_KINDS: Array<{ kind: string; word: string }> = [
  { kind: 'text', word: 'Text' },
  { kind: 'longtext', word: 'Long text' },
  { kind: 'number', word: 'Number' },
  { kind: 'duration', word: 'Minutes' },
  { kind: 'date', word: 'Date' },
  { kind: 'daterange', word: 'Date range' },
  { kind: 'bool', word: 'Yes or no' },
  { kind: 'select', word: 'One choice' },
  { kind: 'multiSelect', word: 'Several choices' },
  { kind: 'rating', word: 'A rating' },
  { kind: 'url', word: 'A link' },
  { kind: 'json', word: 'Structured text' },
  { kind: 'formula', word: 'Computed' },
];

/**
 * Every kind the engine declares, spoken as words — including the two a column
 * can hold but never be changed *into* (Appendix C.2: `relation` is a different
 * namespace and `progress` is derived, so a value change is not what either
 * needs). A caller that meets a kind this build does not know falls back to the
 * key rather than printing nothing.
 */
export const KIND_WORDS: Record<string, string> = {
  ...Object.fromEntries(COLUMN_KINDS.map((entry) => [entry.kind, entry.word])),
  relation: 'A link to another kind',
  progress: 'Progress through a ladder',
};
