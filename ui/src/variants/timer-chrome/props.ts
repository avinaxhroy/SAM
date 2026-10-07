/** Props for TimerChrome surface variants. */
export type TimerProps = {
  /** The session's subject, as the row that started it named it. */
  label: string;
  /**
   * The course the session inherits, named (`CS310`), or null when the row
   * carries none — the confirm asks about a course only when there is one.
   */
  course: string | null;
  /** The running clock, `mm:ss`, derived by the screen from `elapsedMs`. */
  clock: string;
  /** The instant the run began, as a wall-clock `HH:MM` for the facts panel. */
  started: string;
  /** Whether the clock is held. Paused is stated, never drawn as a colour. */
  paused: boolean;
  /** The length the student chose, in minutes — `TimerState.targetMin`. */
  targetMin: number;
  /** Elapsed time in minutes, fractional, for a meter or a fact. */
  elapsedMin: number;
  /** The record kind's write id (`<kind>.new`), or null when the plan has none. */
  command: string | null;
  /** The placement the write is declared against, as the screen declares it. */
  placement: string;
  /** True once the session has ended and receipt is active. */
  ended: boolean;
  /** Hold or release the clock. */
  onPause: () => void;
  /** End the session and write it (the ordinary `record.new` path). */
  onStop: () => void;
  /** The length the student chose; the screen keeps it on the session. */
  onMinutes: (minutes: number) => void;
  /** The receipt's Undo — the app's own `edit.undo` on the write just made. */
  onUndo: () => void;
};
