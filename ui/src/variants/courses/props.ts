/**
 * Courses surface variant component props.
 * Displays subject hierarchies, progress ladders, and topic queues.
 */
export type CourseTopic = {
  id: string;
  label: string;
  /** The record's own kind — what the row's door (`record.panel`) needs. */
  type: string;
  /** The chapter the record names, as the detail's own heading. */
  chapter: string;
  /** done = the ladder's top rung · step = started · none = untouched. */
  state: 'done' | 'step' | 'none';
  /** The rung the topic stands on, and how many the ladder declares. */
  rung: number;
  rungs: number;
  /** Estimated minutes, when the kind declares one (null = the plan says none). */
  minutes: number | null;
  /** The stage in the plan's own words — the row's accessible sentence. */
  stage: string;
};

export type CourseCard = {
  id: string;
  /** The course's own code (`CS201`), which is what a student says out loud. */
  code: string | null;
  name: string;
  /** The identity wash (`mint` · `lilac` · `butter` · `sky`). */
  wash: string | undefined;
  started: number;
  total: number;
  minutes: number;
  /** The nearest dated thing in the course, in days from today — C ranks by it. */
  days: number | null;
  /** The first topic that is not done — what “Continue” and the hero point at. */
  nextId: string | null;
  topics: CourseTopic[];
};

export type CoursesProps = {
  /** The screen's own title (`Courses`, from the navigation entry). */
  title: string;
  courses: CourseCard[];
  summary: { courses: number; topics: number; started: number };
  /** The course whose detail is open, or null for the overview. */
  openedId: string | null;
  /** The command ids the head's doors dispatch, or null when the plan has none. */
  newCourseCommand: string | null;
  newTopicCommand: string | null;
  onOpen: (id: string) => void;
  onClose: () => void;
  /** The record-door read: the app's `record.panel` for one topic. */
  onTopic: (id: string) => void;
  onNewCourse: () => void;
  onNewTopic: () => void;
};

export type Chapter = { label: string; topics: CourseTopic[] };

/** Topics gathered under the chapter each record names, in the plan's order.
 *
 *  The gathering is by **label**, not by runs: a plan whose chapters arrive as
 *  `Chapter 1 · Chapter 2 · Chapter 1` is one chapter named twice, and two
 *  entries with the same key is the one thing Svelte's keyed `{#each}` refuses
 *  — it takes the screen down with `each_key_duplicate`, which is how this
 *  function was found. The first appearance fixes the order. */
export function chaptersOf(course: CourseCard): Chapter[] {
  const byLabel: Record<string, Chapter> = {};
  const chapters: Chapter[] = [];
  for (const topic of course.topics) {
    const held = byLabel[topic.chapter];
    if (held) held.topics.push(topic);
    else {
      const chapter: Chapter = { label: topic.chapter, topics: [topic] };
      byLabel[topic.chapter] = chapter;
      chapters.push(chapter);
    }
  }
  return chapters;
}

/** The fraction a meter or a pip draws, 0–100. */
export function percentOf(part: number, whole: number): number {
  return whole === 0 ? 0 : Math.round((part / whole) * 100);
}
