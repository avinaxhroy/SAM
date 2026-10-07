/**
 * The plan, as the work surfaces read it (UI P2 · U2).
 *
 * The screens that show work — the plan's weeks, the subjects, the question
 * banks, the mocks, the reference lists — all need the same facts: the plan's
 * **kinds**, its records, their relations and each record's progress. This
 * module reads them once per revision through the app's own cache
 * (`app.recordsOf`), so a screen costs no second read of anything another
 * screen already read and every screen sees the same snapshot.
 *
 * Nothing here computes a fact the engine owns: progress comes from
 * `reviews.due` (the ladder the engine resolved), durations come from the
 * records' own duration fields, and a percentage comes from the plan's own
 * `derived` value when the kind declares the formula.
 */
import { app } from '../session.svelte';
import { recordLabel, type RecordDoc } from '../types';

/** What the engine knows about one trackable record's ladder. */
export type ProgressFact = { stages: string[]; complete: boolean };

/** The plan's own shape, for the screens that draw it. */
export type PlanModel = {
  /** The kinds work lives in — the plan's `trackable` types, in plan order. */
  workKinds: string[];
  /** The kinds that carry identity (a course, a track) — what a chip names. */
  identityKinds: string[];
  /** Every work record, all kinds, in plan order. */
  items: RecordDoc[];
  /** Every identity record, in the plan's own order. */
  identities: RecordDoc[];
  /** Every other record the plan holds — chapters, notes, resources, weeks. */
  other: RecordDoc[];
  /** Every record, by id — what a relation target is looked up in. */
  byId: Map<string, RecordDoc>;
  /** Every week record; empty when the plan declares no weeks. */
  weeks: RecordDoc[];
  /** Progress by record id — `reviews.due`'s answer, not a second derivation. */
  progress: Map<string, ProgressFact>;
  /** The stage ladders the plan declares, by machine name. */
  pipelines: Record<string, string[]>;
};

/** Read the whole model. Safe on every revision — every read is cached. */
export async function readPlanModel(): Promise<PlanModel> {
  const kinds = Object.entries(app.types).map(([name, def]) => ({ name, def }));
  const workKinds = kinds.filter(({ def }) => def.trackable === true).map(({ name }) => name);
  const identityKinds = kinds.filter(({ def }) => def.colorRole === 'identity').map(({ name }) => name);

  const items: RecordDoc[] = [];
  const identities: RecordDoc[] = [];
  const other: RecordDoc[] = [];
  for (const { name } of kinds) {
    const records = (await app.recordsOf(name)) ?? [];
    if (workKinds.includes(name)) items.push(...records);
    else if (identityKinds.includes(name)) identities.push(...records);
    else other.push(...records);
  }
  const byId = new Map<string, RecordDoc>();
  for (const record of [...items, ...identities, ...other]) byId.set(record.id, record);
  const weeks = other.filter((record) => record.type === 'week');

  const progress = new Map<string, ProgressFact>();
  const queue = await app.run('reviews.due', {}, { tracked: false });
  const queueData = queue?.data as
    | { due?: unknown[]; upcoming?: unknown[]; waiting?: unknown[]; pipelines?: Record<string, { stages?: string[] }> }
    | undefined;
  for (const key of ['due', 'upcoming', 'waiting'] as const) {
    for (const row of (queueData?.[key] ?? []) as Array<{ id: string; stages?: string[]; complete?: boolean }>) {
      progress.set(row.id, { stages: row.stages ?? [], complete: row.complete === true });
    }
  }
  const pipelines: Record<string, string[]> = {};
  for (const [name, machine] of Object.entries(queueData?.pipelines ?? {})) {
    pipelines[name] = machine?.stages ?? [];
  }

  return { workKinds, identityKinds, items, identities, other, byId, weeks, progress, pipelines };
}

/** The plan's order for weeks: the record's own index, else its file order. */
export function weekIndex(weeks: RecordDoc[], id: string | undefined): number | null {
  if (!id) return null;
  const position = weeks.findIndex((week) => week.id === id);
  if (position < 0) return null;
  const index = weeks[position].fields.index;
  return typeof index === 'number' ? index : position + 1;
}

/** A week spoken as the plan names it: its label, else `Week N`. */
export function weekName(week: RecordDoc, fallbackIndex: number): string {
  const label = week.fields.label;
  if (typeof label === 'string' && label.length > 0) return label;
  return `Week ${typeof week.fields.index === 'number' ? week.fields.index : fallbackIndex}`;
}

/** One duration field of a record, in minutes, when the kind declares one. */
export function minutesOf(record: RecordDoc, key = 'est'): number | null {
  const value = record.fields[key];
  return typeof value === 'number' ? value : null;
}

/** A record's relation targets, as records — what a chip or a group names. */
export function targets(model: PlanModel, record: RecordDoc, key: string): RecordDoc[] {
  return (record.links?.[key] ?? [])
    .map((id) => model.byId.get(id))
    .filter((found): found is RecordDoc => Boolean(found));
}

/**
 * How far along a record is, in words a student says: `step 2 of 3`, `done`,
 * `not started`. The ladder's own stage names belong to the machine (the plan's
 * `rules.json`), so a *count* is what a row states — the stage that comes next
 * is the Reviews screen's business, where its evidence is asked for.
 */
export function stageWord(fact: ProgressFact | undefined, ladder: string[]): string {
  if (!fact || fact.stages.length === 0) return 'not started';
  if (fact.complete || ladder.length === 0) return 'done';
  return `step ${Math.min(fact.stages.length + 1, ladder.length)} of ${ladder.length}`;
}

/** The ladder a record's kind follows, by the pipeline its type declares. */
export function ladderOf(model: PlanModel, type: string): string[] {
  const pipeline = app.types[type]?.pipeline;
  return typeof pipeline === 'string' ? (model.pipelines[pipeline] ?? []) : [];
}

/** The work that belongs to one record: anything whose links name it. */
export function workUnder(model: PlanModel, id: string): RecordDoc[] {
  return model.items.filter((item) =>
    Object.values(item.links ?? {}).some((ids) => ids.includes(id)),
  );
}

/** The chapter a work record sits in, as the label a group heading shows. */
export function chapterOf(model: PlanModel, record: RecordDoc, subjectId: string): string {
  for (const [key, ids] of Object.entries(record.links ?? {})) {
    if (ids.length === 0 || key === 'week' || key === 'resource') continue;
    const target = model.byId.get(ids[0]);
    if (!target || target.id === subjectId) continue;
    if (model.identityKinds.includes(target.type)) continue;
    if (target.type === 'week' || target.type === 'resource') continue;
    return recordLabel(target);
  }
  return 'Everything else';
}

/**
 * The kind a plan's work lives in — the one that carries both a name and a time
 * estimate, because that is what "add a thing to this week" needs. A plan with
 * only one work kind gets that one; a plan with none gets null, and the screen
 * says so instead of writing into a guessed kind.
 */
export function workKindOf(model: PlanModel): string | null {
  const scored = model.workKinds
    .map((kind) => {
      const fields = app.types[kind]?.fields ?? [];
      const named = fields.some((field) => NAMING_KEYS.includes(field.key));
      const timed = fields.some((field) => field.key === 'est');
      const weeks = fields.some((field) => field.key === 'week' && field.type === 'relation');
      return { kind, score: (named ? 2 : 0) + (timed ? 1 : 0) + (weeks ? 1 : 0) };
    })
    .sort((left, right) => right.score - left.score);
  return scored[0]?.kind ?? null;
}

/** The keys a kind uses for a row's own name, in the order they win. */
export const NAMING_KEYS = ['title', 'label', 'name', 'chapter', 'prompt', 'term'];

/** The field a kind keeps its name in, or null when it declares no text field. */
export function titleKeyOf(kind: string): string | null {
  const fields = app.types[kind]?.fields ?? [];
  const named = fields.find(
    (field) => NAMING_KEYS.includes(field.key) && (field.type === 'text' || field.type === 'longtext'),
  );
  if (named) return named.key;
  const text = fields.find((field) => field.type === 'text' || field.type === 'longtext');
  return text ? text.key : null;
}

/** Whether a kind declares a field — a control is only offered for a real one. */
export function hasField(kind: string, key: string): boolean {
  return (app.types[kind]?.fields ?? []).some((field) => field.key === key);
}

/** A record's label, for the screens that list it (never its id). */
export function labelOfRecord(record: RecordDoc): string {
  return recordLabel(record);
}
