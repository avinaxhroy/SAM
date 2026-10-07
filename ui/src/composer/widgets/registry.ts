/**
 * THE WIDGET CATALOG (COMPOSER §3.1, 2026-09-29).
 *
 * The twelve widgets a student may put on a screen — no more. The bound is the
 * feature: a closed catalog of named parts is what keeps composition from
 * becoming a schema designer, and it is why a widget can be added later by
 * adding a host folder and one line here.
 *
 * Two registries, one per question:
 *
 * - **`WIDGETS`** is what the app *offers*: the surface key the plan stores,
 *   the name the student reads, the one line the inserter states, and the group
 *   it is filed under. Labels are English and never the key (D2); the key
 *   travels to `view.setComponents` and nowhere onto a screen.
 * - **`HOSTS`** is what the build can *draw*, resolved with one
 *   `import.meta.glob` — the same idiom as `variants/registry.ts`: a host
 *   folder that exists is a host that ships. There is no second list to keep in
 *   sync and no import to forget, so a surface a bookmarked plan still names
 *   draws the moment its host lands, and one whose host is missing draws the
 *   frame's stated placeholder rather than a blank.
 *
 * A host takes **no props** (§3.1): it derives what it draws from the app store
 * and its own reads, owns its own ephemeral state, and mounts its design's
 * component with the same props the panel derives today. That is what lets the
 * same widget stand on Today and on a screen the student built.
 */
import type { Component } from 'svelte';
import { nameOf } from '../../types';

/** Which band of the inserter a widget is listed under (§4.6). */
export type WidgetGroup = 'day' | 'study';

export type WidgetDef = {
  /** The plan's own surface key — what `view.setComponents` stores. */
  surface: string;
  /** The student's name for it. */
  label: string;
  /** One line: what it shows, for the inserter's row. */
  note: string;
  group: WidgetGroup;
};

/** The groups, said the way the inserter says them. */
export const GROUP_LABELS: Record<WidgetGroup, string> = {
  day: 'Today',
  study: 'Study',
};

/**
 * The catalog, in the order the inserter lists it: the day's widgets first,
 * then the study's. Adding a widget is adding a paragraph here.
 */
export const WIDGETS: WidgetDef[] = [
  {
    surface: 'today-focus',
    label: 'Session card',
    note: 'What to begin, and for how long',
    group: 'day',
  },
  {
    surface: 'today-queue',
    label: "Today's queue",
    note: 'Everything the day holds, and what is done',
    group: 'day',
  },
  {
    surface: 'coming-up',
    label: 'Coming up',
    note: 'The dated rows waiting ahead',
    group: 'day',
  },
  {
    surface: 'week-chart',
    label: 'Week chart',
    note: 'The week measured against your aim — drawn with Today’s own words',
    group: 'day',
  },
  {
    surface: 'plan-spine',
    label: 'Plan spine',
    note: "The term's weeks, and the work placed in them",
    group: 'study',
  },
  {
    surface: 'courses',
    label: 'Courses',
    note: 'Every course, and how far through it you are',
    group: 'study',
  },
  {
    surface: 'practice-banks',
    label: 'Practice banks',
    note: 'The banks you practise from, and logging a set',
    group: 'study',
  },
  {
    surface: 'mocks-calendar',
    label: 'Mocks calendar',
    note: 'The month, and the tests waiting in it',
    group: 'study',
  },
  {
    surface: 'reviews-recall',
    label: 'Recall card',
    note: 'The card you answer, one at a time',
    group: 'study',
  },
  {
    surface: 'reviews-queue',
    label: 'Review queue',
    note: 'What is waiting to be recalled, oldest first',
    group: 'study',
  },
  {
    surface: 'progress-stats',
    label: 'Progress figures',
    note: "The week's own numbers, in the open",
    group: 'study',
  },
  {
    surface: 'reference-library',
    label: 'Library',
    note: 'The things you keep, searchable by name',
    group: 'study',
  },
];

/**
 * The name a widget is called. An unknown surface is a widget this build does
 * not carry — a plan written by a later version, or a host that has not landed
 * — so it is spoken as a name and its frame states the placeholder; the key
 * itself never reaches the screen (D2).
 */
export function widgetLabel(surface: string): string {
  const known = WIDGETS.find((widget) => widget.surface === surface);
  return known ? known.label : nameOf(surface);
}

const HOSTS = import.meta.glob<{ default: Component<Record<string, unknown>> }>('./*/Host.svelte', {
  eager: true,
});

/** `./week-chart/Host.svelte` → `week-chart`. */
const BY_SURFACE: Record<string, Component<Record<string, unknown>>> = Object.fromEntries(
  Object.entries(HOSTS).map(([path, module]) => [
    path.slice(2, -'/Host.svelte'.length),
    module.default,
  ]),
);

/**
 * The component a surface draws with, or `null` when this build has no host for
 * it. The caller draws its stated placeholder in that case — never nothing.
 */
export function hostFor(surface: string): Component<Record<string, unknown>> | null {
  const host = BY_SURFACE[surface];
  if (!host) return null;
  return host;
}
