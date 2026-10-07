/**
 * Today focus card variant component props.
 * Renders daily greeting, focus status, and session prompt.
 */
export type WhyFact = { key: string; value: string };

/** The three states the masthead can be in, derived from the day's own facts. */
export type FocusState = 'decision' | 'clear' | 'gap';

/**
 * The 180° arc's datum: the percent it draws, and the words under it
 * (`50%` · `CS310 covered`). The number is a fact of the plan, so a card
 * without one draws no arc instead of an arc of nothing.
 */
export type Coverage = { value: number; label: string };

export type TodayFocusProps = {
  /** The greeting the frame opens with (`Good morning`, from the clock). */
  greeting: string;
  /** The day, already spoken: `Sun 27 Sep`. */
  dateLine: string;
  /** Masthead focus state. Named `mode` to avoid shadowing Svelte `$state`. */
  mode: FocusState;
  /** The session's name — absent in the all-clear, where nothing is waiting. */
  title: string | null;
  /** The course the focused record belongs to, with its own wash. */
  course: { label: string; wash?: string } | null;
  /** Why the card is here, in the plan's own words (`Next in the plan`). */
  tag: string | null;
  /** The decisive fact one line under the title. */
  why: WhyFact | null;
  /** `Studied today`, in words — the all-clear's one fact (`25 min`). */
  studiedToday: string;
  /** The session length the Start will run for, in minutes. */
  minutes: number;
  onMinutes: (value: number) => void;
  /** The plan's own time-logging kind, or null when it declares none. */
  sessionCommand: string | null;
  onStart: () => void;
  /** The all-clear's one way forward: plan something for today. */
  onPlan: () => void;
  /**
   * The focused course's coverage — the arc's datum and its words. Null when
   * the plan carries no covered thing for it (or nothing is waiting), and then
   * the card draws one track instead of an arc of nothing.
   */
  coverage: Coverage | null;
  /**
   * The focused record's own kind, in the plan's words: the ghost pill says
   * `Open topic`. Null in the all-clear, where there is no record to open.
   */
  kind: string | null;
  /** The record's own door — the detail panel (`record.panel`). */
  onOpen: () => void;
  /**
   * `Not this`: the recommendation's day is moved on by a week
   * (`record.defer`), so the day ranks the next thing instead. A write, so it
   * leaves a receipt the student can undo — never a silent skip.
   */
  onDefer: () => void;
};
