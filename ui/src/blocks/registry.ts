/**
 * The block registry (Phase 5, D8) — the renderer's side of the engine's closed
 * vocabulary.
 *
 * One list, one meaning: the engine validates a kind, the renderer draws it, and
 * the two lists share the same vocabulary. A kind known to only one side
 * renders a placeholder, and the `--uicheck` gate validates parity between both.
 *
 * What lives here is the vocabulary and its limits. Which component draws which
 * kind is the dispatcher's table (`Block.svelte`), because that is the file that
 * imports the components and the place a new kind is added.
 */

/** §3.6's eight record layouts, which are also block kinds. */
export const RECORD_KINDS = [
  'list',
  'table',
  'board',
  'timeline',
  'calendar',
  'tree',
  'cardGrid',
  'graph',
] as const;

/** The presentation blocks: one number, a series, a sentence, and the containers. */
export const PRESENTATION_KINDS = [
  'stat',
  'chart',
  'callout',
  'columns',
  'conditional',
  'repeat',
] as const;

/** The whole closed vocabulary, in the order a picker lists it. */
export const BLOCK_KINDS = [...RECORD_KINDS, ...PRESENTATION_KINDS] as readonly string[];

/**
 * The recursion boundary (D8, spike S5). The loader warns past it and the
 * renderer stops at it — a finite, tested limit rather than recursion until the
 * stack says otherwise.
 */
export const MAX_BLOCK_DEPTH = 6;

/** The most instances one `repeat` draws; truncation is reported, never silent. */
export const MAX_BLOCK_INSTANCES = 60;

export function isRecordKind(kind: string): boolean {
  return (RECORD_KINDS as readonly string[]).includes(kind);
}

export function isContainerKind(kind: string): boolean {
  return kind === 'columns' || kind === 'conditional' || kind === 'repeat';
}

/**
 * The placeholder's sentence — `unknown block: "X" · known: …` — built from one
 * list, so the card a student reads names the same vocabulary the engine
 * validates against, and never a blank screen.
 */
export function unknownKindMessage(kind: string): string {
  return `unknown block: "${kind}" · known: ${BLOCK_KINDS.join(' · ')}`;
}

/**
 * The keys a block kind needs before it can draw anything — what the editor's
 * add form asks for, and what `view.block.add` must be given in the same
 * transaction (§3.7 rejects a half-built block).
 */
export function requiredKeys(kind: string): Array<{ key: string; label: string; hint: string }> {
  switch (kind) {
    case 'callout':
      return [{ key: 'text', label: 'Text', hint: 'the sentence the card shows' }];
    case 'chart':
      return [
        { key: 'x', label: 'Bucket by', hint: 'an expression, usually a date field' },
        { key: 'y', label: 'Value', hint: 'the number to fold' },
      ];
    case 'conditional':
      return [{ key: 'when', label: 'Show when', hint: 'an L2 expression' }];
    default:
      return [];
  }
}
