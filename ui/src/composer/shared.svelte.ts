/**
 * Shared ephemeral state between composed widgets on the active screen:
 * session duration targets and pinned recall cards.
 */
class ComposerShared {
  /** The length a session started from any session widget runs for. */
  targetMin = $state(25);
  /** The record the student put on the recall card, by id. */
  pinnedReview = $state<string | null>(null);
}

export const shared = new ComposerShared();
