/**
 * WHAT A DESIGNED SCREEN IS MADE OF (COMPOSER §3.1, 2026-09-29).
 *
 * A panel is not a screen with a fixed body — it is a screen drawn from a list
 * of widgets, and this table names that list: `today` draws a session card, the
 * day's queue, what is coming up and the week chart, in that order. Everything
 * the composer does depends on the table being data:
 *
 * - the pencil opens a designed screen **as a draft** of this list, so the
 *   student starts from what they can already see rather than from nothing;
 * - *Use SAM's design again* is the same list, one `--clear` away;
 * - the edit bar can say whether the screen still wears SAM's design, which is
 *   `panel` present and `components` not yet written.
 *
 * The keys are the plan's own panel names (`views.json#/views/<name>/panel`),
 * so a renamed panel is a missing row rather than a wrong one — and a view that
 * names a panel this build does not know draws its own stated placeholder,
 * exactly as it did before the composer existed.
 *
 * The surfaces are the widget catalog's keys; `composer/widgets/registry.ts`
 * owns what each one is called and what it shows.
 */
export const PANEL_DEFAULTS: Record<string, string[]> = {
  today: ['today-focus', 'today-queue', 'coming-up', 'week-chart'],
  plan: ['plan-spine'],
  subjects: ['courses'],
  practice: ['practice-banks'],
  mocks: ['mocks-calendar'],
  library: ['reference-library'],
  notes: ['reference-library'],
  reviews: ['reviews-recall', 'reviews-queue'],
  progress: ['progress-stats', 'week-chart'],
};

/**
 * The widgets a panel draws, or nothing when the view names no panel this build
 * knows — the caller then falls back to the view's own blocks, as always.
 * A fresh array every call: the draft the composer works on is never a
 * reference into this table.
 */
export function defaultSurfaces(panel: string | null | undefined): string[] {
  if (!panel) return [];
  return [...(PANEL_DEFAULTS[panel] ?? [])];
}
