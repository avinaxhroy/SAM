/**
 * C · THE BAND · the two mechanics (ports, 2026-09-29).

  * `ticks` is `refs/d3-array/src/ticks.js` — the round-number ladder (1-2-5)
   the plate's grid and its day axis are graduated with: `tickSpec`'s step →
   power → error → factor, the rounding of the two ends and the trim
   (`if (i1 * inc < start) ++i1; if (i2 * inc > stop) --i2`), the negative-`inc`
   form for `power < 0`, the 0.5 ≤ count < 2 retry, and `tickStep`/`roundUp` —
   the round maximum that covers a value without trimming it away. (The retired
   B · The Rule borrowed this ladder; the design is gone and the helper survives
   only here, as the chart's grid.)
  * `gapBands` is `refs/plot/src/marks/difference.js` re-expressed as geometry:
   the region where the second series is *above* the first and the region where
   it is *below*, split at the exact crossing of the two lines — the geometric
   identity of the clip `clipDifference` performs with two `clipPath`s, computed
   here instead of delegated. The degenerate pair (two flat lines) is the
   reference's own clause, and is exactly C's days panel.
 */

export type Point = { x: number; a: number; b: number };

/** The plate's own drawing box: the lab's 300×176 units, its margins and the
 *  line the two series are measured from. */
export const CHART = { w: 300, h: 176, left: 40, right: 288, top: 16, mid: 74, days: 104, bot: 152 };

const E10 = Math.sqrt(50);
const E5 = Math.sqrt(10);
const E2 = Math.sqrt(2);

function tickSpec(start: number, stop: number, count: number): [number, number, number] {
  const step = (stop - start) / Math.max(0, count);
  const power = Math.floor(Math.log10(step));
  const error = step / 10 ** power;
  const factor = error >= E10 ? 10 : error >= E5 ? 5 : error >= E2 ? 2 : 1;
  let i1: number;
  let i2: number;
  let inc: number;
  if (power < 0) {
    inc = 10 ** -power / factor;
    i1 = Math.round(start * inc);
    i2 = Math.round(stop * inc);
    if (i1 / inc < start) ++i1;
    if (i2 / inc > stop) --i2;
    inc = -inc;
  } else {
    inc = 10 ** power * factor;
    i1 = Math.round(start / inc);
    i2 = Math.round(stop / inc);
    if (i1 * inc < start) ++i1;
    if (i2 * inc > stop) --i2;
  }
  if (i2 < i1 && count >= 0.5 && count < 2) return tickSpec(start, stop, count * 2);
  return [i1, i2, inc];
}

/** The round numbers between two ends, inclusive, at about `count` of them. */
export function ticks(start: number, stop: number, count: number): number[] {
  if (!(count > 0)) return [];
  if (start === stop) return [start];
  const reverse = stop < start;
  const [i1, i2, inc] = reverse ? tickSpec(stop, start, count) : tickSpec(start, stop, count);
  if (!(i2 >= i1)) return [];
  const n = i2 - i1 + 1;
  const out: number[] = [];
  for (let index = 0; index < n; index += 1) {
    const step = reverse ? i2 - index : i1 + index;
    out.push(inc < 0 ? step / -inc : step * inc);
  }
  return out;
}

/** The round maximum that covers a value without trimming it away. */
export function roundUp(value: number, count: number): number {
  if (!(value > 0)) return 1;
  const [, , inc] = tickSpec(0, value, count);
  const step = inc < 0 ? 1 / -inc : inc;
  return step * Math.ceil(value / step);
}

/** The region between two series, inked: above the first, below it. */
export function gapBands(points: Point[]): { surplus: string; deficit: string } {
  const above: Point[][] = [];
  const below: Point[][] = [];
  let run: Point[] = [];
  let side = 0;
  const flush = (): void => {
    if (run.length > 1) (side > 0 ? above : below).push(run);
    run = [];
  };
  const pathFor = (segments: Point[][]): string =>
    segments
      .map((segment) => {
        const forward = segment
          .map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.b}`)
          .join(' ');
        const back = [...segment]
          .reverse()
          .map((point) => ` L ${point.x} ${point.a}`)
          .join('');
        return `${forward}${back} Z`;
      })
      .join(' ');
  if (points.length < 2) return { surplus: '', deficit: '' };
  for (let index = 0; index < points.length - 1; index += 1) {
    const p = points[index];
    const q = points[index + 1];
    // `d > 0` means the second series is above the first (y grows downward).
    const dp = p.a - p.b;
    const dq = q.a - q.b;
    let sp = dp > 0 ? 1 : dp < 0 ? -1 : 0;
    let sq = dq > 0 ? 1 : dq < 0 ? -1 : 0;
    if (sp === 0 && sq === 0) {
      flush();
      side = 0;
      continue;
    }
    if (sp === 0) sp = sq;
    if (sq === 0) sq = sp;
    if (side !== 0 && sp !== side) flush();
    if (side === 0) side = sp;
    if (run.length === 0) run = [p];
    if (sp === sq) {
      run.push(q);
      continue;
    }
    const t = dp / (dp - dq);
    const cross: Point = {
      x: p.x + (q.x - p.x) * t,
      a: p.a + (q.a - p.a) * t,
      b: p.b + (q.b - p.b) * t,
    };
    run.push(cross);
    flush();
    side = sq;
    run = [cross, q];
  }
  flush();
  return { surplus: pathFor(above), deficit: pathFor(below) };
}
