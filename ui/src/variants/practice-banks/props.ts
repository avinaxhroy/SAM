/**
 * Practice banks surface variant component props.
 * Displays problemset banks, progress ratios, and practice logging controls.
 */

/** Problemset bank display row. `total` is null when bank size is undeclared. */
export type PracticeBank = {
  id: string;
  /** The record's own kind — what the design's door (`record.panel`) needs. */
  type: string;
  /** `recordLabel` — the bank's own name (its chapter, in the JEE preset). */
  label: string;
  /** The course it belongs to, its own name — null while the names load. */
  course: string | null;
  /** The course's own code (`PHY`), for the chip; null when it declares none. */
  code: string | null;
  /** The identity wash (`mint` · `lilac` · `butter` · `sky`). */
  wash: string | undefined;
  /** The bank's own size, or null when the plan declares none. */
  total: number | null;
  attempted: number;
  solved: number;
  /** The solved share, rounded 0–100 — what the figure and the bar read. */
  share: number | null;
  /** Duration since last attempt in words, or fallback if unrecorded. */
  since: string;
};

/** The page head's own counts, re-derived from the banks under it. */
export type PracticeSummary = {
  banks: number;
  questions: number;
  attempted: number;
  solved: number;
  /** How many banks are under 60% solved — the head's own reading. */
  thin: number;
};

/** A course would be a design's chip; its name is what a student says. */
export type NamesState = 'loading' | 'ready' | 'failed';

export type PracticeProps = {
  /** The screen's own title (`Practice`, from the navigation entry). */
  title: string;
  banks: PracticeBank[];
  summary: PracticeSummary;
  /** The bank whose entry is open, or null — the screen owns this one fact. */
  openedId: string | null;
  namesState: NamesState;
  /** The command the head's one door dispatches, or null when the plan has none. */
  newBankCommand: string | null;
  /** The day a save stamps, in the plan's own words (`Mon 29 Sep`), or null. */
  stamp: string | null;
  /** The write is in flight; every commit control is disabled while it is. */
  busy: boolean;
  onOpen: (id: string) => void;
  onClose: () => void;
  /** The record door: the app's own `record.panel` for one bank. */
  onRecord: (id: string) => void;
  onNewBank: () => void;
  /** The name read failed; the design offers the app's own retry. */
  onRetryNames: () => void;
  /**
   * THE ONE WRITE. Both numbers are already integers the bank can hold (the
   * design checks `writable` first), and the panel refuses nothing it was not
   * told to: it writes `record.setField` for each field that changed, stamps
   * the day, and announces `logged …` with an Undo.
   */
  onLog: (id: string, attempted: number, solved: number) => Promise<boolean>;
};

/**
 * One run of a sentence, and whether it is the emphasised part of it. The
 * designs draw the sentence in their own type; the *words* are the screen's, so
 * three designs can never say three different things about one write.
 */
export type Run = { t: string; b?: boolean };

/** The page head's one line, in the words the lab states it in. */
export function subLine(summary: PracticeSummary): string {
  const noun = summary.banks === 1 ? 'bank' : 'banks';
  return `${summary.questions} questions in ${summary.banks} ${noun} · ${summary.thin} of ${summary.banks} under 60% solved`;
}

/** A bank's own reading: `8 of 40 solved`, and never a bare percentage. */
export function figureOf(bank: PracticeBank): string {
  return bank.total === null ? `${bank.solved} solved` : `${bank.solved} of ${bank.total} solved`;
}

/** The bank's solved share as the crest and the pocket draw it — exact, so the
 *  dashed mark starts where the committed fill really ends and not a rounded
 *  pixel away from it. */
export function exactShare(bank: PracticeBank): number {
  return bank.total === null || bank.total === 0 ? 0 : (bank.solved / bank.total) * 100;
}

/** Digits only: the one thing these fields ever accept typed. */
function digits(text: string): string {
  return String(text).replace(/[^0-9]/g, '');
}

/** A whole number the bank can hold, or null — clamped by the bank's own size,
 *  which is a fact about the record rather than a policy. */
export function readNum(text: string, ceiling: number | null): number | null {
  const d = digits(text);
  if (d.length === 0) return null;
  const value = Number.parseInt(d, 10);
  return ceiling === null ? value : Math.min(value, ceiling);
}

/** Typed past what the bank holds. Never silently clamped into a save. */
function over(text: string, ceiling: number | null): boolean {
  if (ceiling === null) return false;
  const d = digits(text);
  return d.length > 0 && Number.parseInt(d, 10) > ceiling;
}

/**
 * A write is possible only while both numbers are numbers this bank could have,
 * neither is more than the other, and at least one of them is new. The commit
 * control is disabled otherwise, so a refusal is never a control that does
 * nothing.
 */
export function writable(bank: PracticeBank, attText: string, solText: string): boolean {
  if (over(attText, bank.total) || over(solText, bank.total)) return false;
  const attempted = readNum(attText, bank.total);
  const solved = readNum(solText, bank.total);
  if (attempted === null || solved === null || solved > attempted) return false;
  return attempted !== bank.attempted || solved !== bank.solved;
}

/**
 * WHAT THE WRITE WILL DO, said at the control before the write.
 *
 * Nothing to write is not a sentence: when the two fields already say the
 * bank's own numbers the runs are empty and the line takes no room. A refusal
 * carries its own reason. Everything else is about the share — the reading the
 * designs draw — and the stamp the save writes.
 */
export function forecast(
  bank: PracticeBank,
  attText: string,
  solText: string,
  stamp: string | null,
): { bad: boolean; runs: Run[] } {
  const day = stamp === null ? 'today' : stamp;
  if (over(attText, bank.total)) {
    return {
      bad: true,
      runs: [
        { t: 'This bank has ' },
        { t: `${bank.total} questions`, b: true },
        { t: ' — ' },
        { t: `${digits(attText)} tried`, b: true },
        { t: ' is more than it holds.' },
      ],
    };
  }
  if (over(solText, bank.total)) {
    return {
      bad: true,
      runs: [
        { t: 'This bank has ' },
        { t: `${bank.total} questions`, b: true },
        { t: ' — ' },
        { t: `${digits(solText)} got right`, b: true },
        { t: ' is more than it holds.' },
      ],
    };
  }
  const attempted = readNum(attText, bank.total);
  const solved = readNum(solText, bank.total);
  if (attempted !== null && solved !== null && solved > attempted) {
    return { bad: true, runs: [{ t: 'You cannot have got more right than you tried.' }] };
  }
  // A field that is empty (or the two numbers the bank already holds) is not a
  // write, so there is no consequence to state: the line is silent and takes no
  // room, rather than forecasting a share that cannot move. The commit control
  // is disabled in both states, so nothing here is a control that does nothing.
  if (attempted === null || solved === null) return { bad: false, runs: [] };
  if (attempted === bank.attempted && solved === bank.solved) return { bad: false, runs: [] };

  const runs: Run[] = [{ t: 'After saving: ' }];
  if (bank.total === null || bank.share === null) {
    // A bank the plan gave no size: there is no share to move, so the sentence
    // is about the two numbers themselves. Never a percentage from nowhere.
    runs.push({ t: 'the bank reads ' });
    runs.push({ t: `${attempted} tried · ${solved} right`, b: true });
  } else {
    const from = bank.share;
    const to = Math.round((solved / bank.total) * 100);
    if (to === from) {
      runs.push({ t: 'the share stays at ' });
      runs.push({ t: `${from}%`, b: true });
      runs.push({ t: ' — you tried more without solving more' });
    } else {
      runs.push({ t: 'the share goes ' });
      runs.push({ t: `${from}% → ${to}%`, b: true });
    }
  }
  runs.push({ t: ' · ' });
  runs.push({ t: 'tried today', b: true });
  runs.push({ t: ` — saving stamps ${day}.` });
  return { bad: false, runs };
}
