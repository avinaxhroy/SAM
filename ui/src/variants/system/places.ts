/**
 * SYSTEM · the machine's five places (2026-09-29).
 *
 * Five, not six: `system.view` returns six groups, and the sixth — recovery —
 * is a place you visit once, so it is a control rather than a mode. The
 * `destinations` group is the rail's own entries into views, so it lives inside
 * *Screens*; `metrics` are rules' derived figures, so they live inside *Rules*.
 * That fold is the shipped screen's own decision (`System.svelte`'s header) and
 * it is kept here: the three designs all draw the same five.
 */
export type PlaceId = 'kinds' | 'screens' | 'rules' | 'scheduling' | 'stages';

export type PlaceDef = { id: PlaceId; label: string; mark: string; sub: string };

export const PLACES: PlaceDef[] = [
  { id: 'kinds', label: 'Kinds', mark: 'cube', sub: 'what your plan can hold — a thing, and the columns it carries' },
  { id: 'screens', label: 'Screens', mark: 'layout', sub: 'every screen this plan opens, and how each one is built' },
  { id: 'rules', label: 'Rules', mark: 'checklist', sub: 'what the app counts, folds and checks' },
  { id: 'scheduling', label: 'Scheduling', mark: 'refresh', sub: 'when a review comes back, and what is already scheduled' },
  { id: 'stages', label: 'Stages', mark: 'stairs', sub: 'the steps a tracked thing moves through, and the gate between each' },
];

/** The five labels by id — what the search says about *where* a hit lives. */
export const PLACE_LABEL: Record<PlaceId, string> = {
  kinds: 'Kinds',
  screens: 'Screens',
  rules: 'Rules',
  scheduling: 'Scheduling',
  stages: 'Stages',
};
