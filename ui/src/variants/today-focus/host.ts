/**
 * TODAY-FOCUS · THE HOST'S OWN WORK — the one read and the two writes both
 * screens that mount the masthead perform (`panels/Today.svelte` and
 * `composer/widgets/today-focus/Host.svelte`).
 *
 * They live here because a surface must not carry two derivations of one fact:
 * the coverage is arithmetic over the plan's records, which a drawing component
 * never does for itself, and `Not this` is a transaction — a command with a
 * receipt — rather than a flag a component is left holding.
 */
import { app } from '../../session.svelte';
import { dayMonth, shiftDay } from '../problemsets-due/props';
import type { Coverage } from './props';
import type { TodayItem } from '../../types';

/** One row of the engine's review queue — the stage names it holds per record. */
type ReviewRow = { id: string; stages?: string[] };
type ReviewRead = { due?: ReviewRow[]; upcoming?: ReviewRow[]; waiting?: ReviewRow[] };

/**
 * The focused course's coverage — the arc's own datum, and its words.
 *
 * The same rule the Progress screen states in its own words: the study kind is
 * the one the reviews (then today) destination lists, a record counts for the
 * course its links point at, and it counts as covered once the engine has a
 * stage on it — *counted once you have been asked the question*. One rule, one
 * number: the arc and the coverage card print the same figure for the same
 * course, and neither invents a score (D1).
 *
 * `null` is an honest answer — no plan, no study kind, no record of that kind
 * under the course — and the card then draws one track instead of an arc of
 * nothing.
 */
export async function coverageOf(
  course: { id: string; label: string } | null,
): Promise<Coverage | null> {
  if (!course) return null;
  const declared = Object.entries(app.types ?? {});
  const trackable = declared
    .filter(([, def]) => def.trackable === true)
    .map(([name]) => name);
  const panels = Object.values(app.views?.views ?? {});
  const study =
    (panels.find((def) => def.panel === 'reviews')?.type ??
      panels.find((def) => def.panel === 'today')?.type ??
      trackable[0]) || null;
  if (!study) return null;

  const records = (await app.recordsOf(study)) ?? [];
  if (records.length === 0) return null;

  const queue = await app.run('reviews.due', {}, { tracked: false });
  const stages = new Map<string, string[]>();
  if (queue) {
    const read = queue.data as ReviewRead;
    for (const key of ['due', 'upcoming', 'waiting'] as const) {
      for (const row of read[key] ?? []) stages.set(row.id, row.stages ?? []);
    }
  }

  let total = 0;
  let tested = 0;
  for (const record of records) {
    const points = Object.values(record.links ?? {}).some((ids) => ids.includes(course.id));
    if (!points) continue;
    total += 1;
    if ((stages.get(record.id)?.length ?? 0) > 0) tested += 1;
  }
  if (total === 0) return null;
  return { value: Math.round((tested / total) * 100), label: `${course.label} covered` };
}

/**
 * `Not this` — the recommendation's day moved on by a week, in one write that
 * covers both doors the day is read from: the plan's `focus` field (the
 * `committed` group is *planned for today*) and the scheduler's due date (what
 * `late` and `due` are read from). Without the second, a due row would sit
 * still and the press would look like nothing happened; the command moves both
 * and reports what it moved. The receipt carries the undo, so a skip is a
 * transaction of its own and never a silent removal — the record is still on
 * the board, a week out.
 */
export async function setAside(item: TodayItem, today: string | null): Promise<void> {
  if (!today) return;
  const date = shiftDay(today, 7);
  const result = await app.run('record.defer', {
    ids: [item.id],
    date,
    field: 'focus',
    reviews: true,
  });
  if (result) {
    app.notice(`Set aside until ${dayMonth(date)}`, {
      label: 'Undo',
      run: () => void app.undo(),
    });
  }
}

/** The record's own door: the detail panel a row's own control opens. */
export function openRecord(item: TodayItem): void {
  void app.run('record.panel', { id: item.id, type: item.kind });
}
