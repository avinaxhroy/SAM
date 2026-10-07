/**
 * B · THE COUNTER BANK · the drum's arithmetic (port of
 * `refs/motion-primitives/components/core/sliding-number.tsx`, 2026-09-29).
 *
 * The reference's whole split, taken: the integer digits keep their own places,
 * the decimal digits theirs (`parseInt(decimalPart, 10)` with `place = 10^(len −
 * i − 1)`), every place renders **all ten numerals**, and the window's offset is
 * the reference's own `(10 + number − placeValue) % 10` with the `> 5 → −10`
 * correction, so a wheel turning 3 to 4 moves one step and never the five the
 * long way round. The height each numeral is translated by is **measured off the
 * pad**, never declared — that lives in `Drum.svelte`, where the DOM is.
 *
 * Refused: the spring, the motion values, the `layoutId` identity and React's
 * per-render element identity (Cadence's `--dur-2` and `--ease` instead, and no
 * roll at all under `prefers-reduced-motion`).
 */

export type Parts = {
  negative: boolean;
  intValue: number;
  intPlaces: number[];
  sep: string;
  decValue: number;
  decPlaces: number[];
};

export type Window = { value: number; place: number };

function placesOf(digits: string): number[] {
  return digits.split('').map((_, index) => 10 ** (digits.length - index - 1));
}

/** The reference's `Number` split: `'0.38'` → integer `0`, decimal `38`. */
export function partsOf(text: string): Parts {
  const negative = text.charAt(0) === '-';
  const [intText = '0', decText = ''] = String(Math.abs(Number(text))).split('.');
  return {
    negative,
    intValue: Number.parseInt(intText, 10) || 0,
    intPlaces: placesOf(intText),
    sep: decText ? '.' : '',
    decValue: Number.parseInt(decText, 10) || 0,
    decPlaces: placesOf(decText),
  };
}

/** One window per place, the integer digits first. */
export function windowsOf(parts: Parts): Window[] {
  return [
    ...parts.intPlaces.map((place) => ({ value: parts.intValue, place })),
    ...parts.decPlaces.map((place) => ({ value: parts.decValue, place })),
  ];
}

/** Whether two readings want the same windows — a place that appeared or went
 *  is a rebuild, not a roll. */
export function samePlaces(left: Window[], right: Window[]): boolean {
  return left.length === right.length && left.every((win, index) => win.place === right[index].place);
}

/** Which numeral a window at this place is showing. */
export function digitAt(win: Window): number {
  return Math.floor(win.value / win.place) % 10;
}

/** The reference's offset, taken the shorter way round. */
export function offsetOf(numeral: number, digit: number): number {
  const offset = (10 + numeral - digit) % 10;
  return offset > 5 ? offset - 10 : offset;
}
