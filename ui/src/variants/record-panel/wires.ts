/**
 * A · THE DIAGRAM · the routed connector (port of `refs/gantt/src/arrow.js`,
 * 2026-09-29).
 *
 * The reference's whole mechanic, re-expressed on two rectangles: the origin is
 * the source box's own centre, the start steps back ten at a time while the
 * target collides and ten more at the end, the target is met a few pixels short
 * of its edge, the route is branched on that collision test into the wrap-around
 * (three fillets, `H`/`V` runs) or the single elbow, and the head is the
 * reference's own open chevron (`m -5 -5 l 5 5 l -5 5`, stroked, never filled).
 *
 * Re-expressed, never copied: the boxes are Cadence plates, the run is 10px, and
 * its relation `end = pad − curve` is kept — which is why the wrap route's last
 * segment no longer doubles back against the fillet it just drew.
 */

export type Rect = { left: number; top: number; width: number; height: number };

/** The reference's own four numbers, at this port's tighter pad. */
const WIRE = { pad: 10, back: 10, curve: 6, end: 5, head: 5 };

function r2(value: number): number {
  return Math.round(value * 100) / 100;
}

/** A node's box, in its grid's own coordinates — the drawing's frame. */
export function rectOf(node: Element, base: DOMRect): Rect {
  const rect = node.getBoundingClientRect();
  return {
    left: r2(rect.left - base.left),
    top: r2(rect.top - base.top),
    width: r2(rect.width),
    height: r2(rect.height),
  };
}

/** The reference's `calculate_path()`, on two rectangles. */
export function routeWire(from: Rect, to: Rect): string {
  if (!from.width || !to.width) return '';
  let startX = from.left + from.width / 2;
  const colliding = (): boolean => to.left < startX + WIRE.pad && startX > from.left + WIRE.pad;
  while (colliding()) startX -= WIRE.back;
  startX -= WIRE.back;
  // The reference steps back past the bar it hangs from because the bar is
  // opaque and covers the run; a box on a plate is not, so the origin is brought
  // back to the box's own face — the wire must touch the box it leaves.
  startX = Math.max(startX, from.left);
  const startY = from.top + from.height / 2;
  const endX = to.left - WIRE.end;
  const endY = to.top + to.height / 2;
  const fromBelow = from.top + from.height / 2 > to.top + to.height / 2;
  let curve = WIRE.curve;
  const clockwise = fromBelow ? 1 : 0;
  let curveY = fromBelow ? -curve : curve;
  const head = ` m ${-WIRE.head} ${-WIRE.head} l ${WIRE.head} ${WIRE.head} l ${-WIRE.head} ${WIRE.head}`;

  if (to.left <= from.left + WIRE.pad) {
    let downOne = WIRE.pad / 2 - curve;
    if (downOne < 0) {
      downOne = 0;
      curve = WIRE.pad / 2;
      curveY = fromBelow ? -curve : curve;
    }
    const downTwo = to.top + to.height / 2 - curveY;
    const left = to.left - WIRE.pad;
    return (
      `M ${r2(startX)} ${r2(startY)}` +
      ` v ${r2(downOne)}` +
      ` a ${r2(curve)} ${r2(curve)} 0 0 1 ${r2(-curve)} ${r2(curve)}` +
      ` H ${r2(left)}` +
      ` a ${r2(curve)} ${r2(curve)} 0 0 ${clockwise} ${r2(-curve)} ${r2(curveY)}` +
      ` V ${r2(downTwo)}` +
      ` a ${r2(curve)} ${r2(curve)} 0 0 ${clockwise} ${r2(curve)} ${r2(curveY)}` +
      ` L ${r2(endX)} ${r2(endY)}${head}`
    );
  }
  if (endX < startX + curve) curve = Math.max(0.5, endX - startX);
  const offset = fromBelow ? endY + curve : endY - curve;
  return (
    `M ${r2(startX)} ${r2(startY)}` +
    ` V ${r2(offset)}` +
    ` a ${r2(curve)} ${r2(curve)} 0 0 ${clockwise} ${r2(curve)} ${r2(curveY)}` +
    ` L ${r2(endX)} ${r2(endY)}${head}`
  );
}
