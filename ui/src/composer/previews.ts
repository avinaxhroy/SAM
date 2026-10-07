/**
 * Static 160×90 SVG wireframe previews for widget catalog cards (COMPOSER §3.1, §4.6).
 * Renders layout shapes via `currentColor` without mock data.
 */
import type { WIDGETS } from './widgets/registry';

/** The surfaces the catalog offers — the map below is checked against it. */
type Surface = (typeof WIDGETS)[number]['surface'];

// SVG wireframe primitives

/** A filled block: the geometry the eye reads as *content*, at `opacity`. */
function mark(x: number, y: number, w: number, h: number, opacity: number, rx = 1.6): string {
  return `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}" fill="currentColor" fill-opacity="${opacity}" stroke="none"/>`;
}

/** An outlined block: a capsule, a cell, a well, a button. */
function edge(x: number, y: number, w: number, h: number, opacity = 0.45, rx = 1.6): string {
  return `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}" stroke-opacity="${opacity}"/>`;
}

/** A hairline: a baseline, a spine, a divider. */
function rule(x1: number, y1: number, x2: number, y2: number, opacity = 0.5, width = 1.1): string {
  return `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke-width="${width}" stroke-opacity="${opacity}"/>`;
}

/** A dashed hairline: a rule the widget draws as *not settled* (the aim line). */
function dashed(x1: number, y1: number, x2: number, y2: number, opacity = 0.35): string {
  return `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke-width="1.1" stroke-opacity="${opacity}" stroke-dasharray="3 3"/>`;
}

/** An outlined dot: a tick that is not done. */
function dot(cx: number, cy: number, r: number, opacity = 0.5): string {
  return `<circle cx="${cx}" cy="${cy}" r="${r}" stroke-opacity="${opacity}"/>`;
}

/** A filled dot: a tick that is done, a day that carries something. */
function disc(cx: number, cy: number, r: number, opacity = 0.8): string {
  return `<circle cx="${cx}" cy="${cy}" r="${r}" fill="currentColor" fill-opacity="${opacity}" stroke="none"/>`;
}

/** The card edge a card-shaped widget wears — the component's own outline. */
function frame(y = 2, h = 86): string {
  return edge(2, y, 156, h, 0.45, 5);
}

/** The head every card-shaped widget opens with: icon tile, title, caption. */
function head(): string {
  return mark(10, 10, 10, 10, 0.75, 3) + mark(26, 11.5, 46, 3.2, 0.5) + mark(26, 17, 30, 2.4, 0.28);
}

/** The head a page-shaped widget opens with: its title, and its one-line sub. */
function pagehead(): string {
  return mark(8, 7, 48, 5, 0.6, 2.5) + mark(8, 16, 32, 2.6, 0.28);
}

/** A row of work: its tick, its title, and the minutes at its right end. */
function workRow(y: number, title: number): string {
  return dot(16, y + 4, 3.6) + mark(24, y + 1.8, title, 3.4, 0.42) + mark(92, y + 2.4, 16, 2.4, 0.24);
}

/** A row of shelf goods: its name, and the one line it previews. */
function shelfRow(y: number): string {
  return mark(10, y, 64, 3.4, 0.55) + mark(10, y + 6, 124, 2.6, 0.24);
}

/** The drawing, in the one box every preview shares. */
function box(inner: string): string {
  return (
    '<svg viewBox="0 0 160 90" fill="none" stroke="currentColor" stroke-width="1.1" ' +
    `stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${inner}</svg>`
  );
}

/* ── THE TWELVE ───────────────────────────────────────────────────────── */

/**
 * One preview per catalog surface, drawn from that surface's A design. The map
 * is keyed by the surface key — the same string `view.setComponents` stores —
 * so the lookup is the identity the plan uses, not a label a translator could
 * change; the type is the catalog's own union, so a widget added to
 * `widgets/registry.ts` without a drawing fails the type check rather than
 * arriving on the sheet as a hole.
 */
export const PREVIEWS: Record<Surface, string> = {
  /* Session card — the greeting, then one card: the session's headline block
     with its duration capsule, and the bar that begins it. */
  'today-focus': box(
    pagehead() +
      frame(24, 64) +
      mark(10, 34, 78, 6, 0.7, 2.5) +
      mark(10, 44, 54, 3.4, 0.36) +
      edge(98, 34, 52, 11, 0.4, 5.5) +
      mark(106, 38, 24, 3, 0.35) +
      mark(10, 66, 42, 12, 0.85, 6) +
      edge(58, 66, 30, 12, 0.45, 6) +
      dot(144, 72, 4, 0.45),
  ),

  /* Today's queue — the stack: one row lifted out of the list (it is the one
     the day is on), a tick on each, minutes at the right. */
  'today-queue': box(
    frame() +
      head() +
      mark(8, 28, 144, 14, 0.1, 4) +
      disc(16, 35, 3.6, 0.85) +
      mark(24, 33.2, 54, 3.4, 0.6) +
      mark(84, 34, 20, 2.6, 0.3) +
      workRow(44, 62) +
      workRow(58, 50) +
      workRow(72, 68),
  ),

  /* Coming up — dated rows: each is a day tile, what is on that day, and a
     mark at the end; the first one carries the filled dot. */
  'coming-up': box(
    frame() +
      head() +
      [28, 46, 64]
        .map((y, i) =>
          [
            edge(10, y, 20, 16, 0.5, 3),
            mark(14, y + 4, 12, 2.4, 0.5),
            mark(14, y + 9, 8, 2, 0.28),
            mark(38, y + 2, 62, 3.4, 0.52),
            mark(38, y + 8, 40, 2.6, 0.28),
            i === 0 ? disc(144, y + 8, 3, 0.8) : dot(144, y + 8, 3, 0.5),
          ].join(''),
        )
        .join(''),
  ),

  /* Week chart — the week measured against the aim: a dashed target rule, the
     baseline, seven bars, and the peak taller than the rest with its mark. */
  'week-chart': box(
    frame() +
      head() +
      dashed(14, 42, 146, 42) +
      rule(14, 70, 146, 70, 0.6) +
      [14, 26, 10, 38, 30, 20, 34]
        .map((h, i) => mark(14 + i * 19, 70 - h, 13, h, h === 38 ? 0.85 : 0.32, 2.5))
        .join('') +
      disc(77.5, 27, 2.6, 0.85),
  ),

  /* Plan spine — the term: a page head, a vertical spine, a node and a tick
     for each week (the first is the one in hand), and the work placed in it at
     the right. */
  'plan-spine': box(
    pagehead() +
      rule(18, 28, 18, 78, 0.6, 1.6) +
      [34, 52, 70]
        .map(
          (y, i) =>
            rule(18, y, 27, y, 0.5) +
            (i === 0 ? disc(18, y, 3.2, 0.85) : dot(18, y, 3.2, 0.45)) +
            mark(33, y - 4, 83, 3.6, 0.5) +
            mark(33, y + 2, 52, 2.6, 0.26) +
            mark(122, y - 4, 30, 3.6, 0.26),
        )
        .join(''),
  ),

  /* Courses — every course as a tile: its title, its coverage meter, and the
     one line under it, three across. */
  courses: box(
    pagehead() +
      edge(122, 6, 30, 12, 0.45, 6) +
      [28, 58]
        .map((y) =>
          [8, 57, 106]
            .map(
              (x) =>
                edge(x, y, 42, 26, 0.45, 3) +
                mark(x + 6, y + 6, 20, 3.2, 0.5) +
                mark(x + 6, y + 15, 30, 3, 0.18) +
                mark(x + 6, y + 15, 18, 3, 0.7) +
                mark(x + 6, y + 21, 14, 2.4, 0.26),
            )
            .join(''),
        )
        .join(''),
  ),

  /* Practice banks — the banks you practise from: a name and its size on the
     left, the button that logs a set at the right (one of them lit). */
  'practice-banks': box(
    pagehead() +
      [28, 48, 68]
        .map((y, i) =>
          [
            mark(10, y, 64, 3.4, 0.52),
            mark(10, y + 7, 44, 2.6, 0.26),
            i === 1 ? mark(116, y, 34, 13, 0.8, 6.5) : edge(116, y, 34, 13, 0.45, 6.5),
          ].join(''),
        )
        .join(''),
  ),

  /* Mocks calendar — the month: the month bar with its two arrows, the seven
     weekday marks, and the grid, two days carrying something. */
  'mocks-calendar': box(
    '<path d="M56 8 l-4 4 l4 4" stroke-opacity=".5"/>' +
      '<path d="M104 8 l4 4 l-4 4" stroke-opacity=".5"/>' +
      mark(66, 8, 28, 4, 0.6, 2) +
      [0, 1, 2, 3, 4, 5, 6].map((i) => mark(19 + i * 19, 20, 10, 2.4, 0.3, 1.2)).join('') +
      [0, 1, 2, 3, 4]
        .map((row) =>
          [0, 1, 2, 3, 4, 5, 6]
            .map((col) => {
              const x = 19 + col * 19;
              const y = 26 + row * 12;
              const carried = (row === 1 && col === 2) || (row === 3 && col === 4);
              return edge(x, y, 16, 10, 0.3, 2) + (carried ? mark(x, y, 16, 10, 0.12, 2) + disc(x + 8, y + 5, 2.2, 0.85) : '');
            })
            .join(''),
        )
        .join(''),
  ),

  /* Recall card — the deck's sheets behind one card: the head chips, the
     question, and the grades that follow the reveal. */
  'reviews-recall': box(
    edge(10, 6, 140, 78, 0.28, 5) +
      edge(6, 10, 148, 74, 0.35, 5) +
      frame(14, 74) +
      mark(12, 22, 30, 8, 0.22, 4) +
      mark(48, 22, 22, 8, 0.22, 4) +
      mark(18, 36, 104, 4.6, 0.62, 2.3) +
      mark(18, 45, 72, 3, 0.3) +
      [10, 47, 84, 121]
        .map((x, i) => (i === 1 ? mark(x, 62, 30, 13, 0.8, 6.5) : edge(x, 62, 30, 13, 0.45, 6.5)))
        .join(''),
  ),

  /* Review queue — what is waiting, oldest first: two groups, each with its
     own name and the count waiting on it. */
  'reviews-queue': box(
    frame() +
      head() +
      mark(10, 30, 30, 3.4, 0.5) +
      mark(130, 27, 20, 9, 0.24, 4.5) +
      workRow(38, 58) +
      workRow(50, 46) +
      mark(10, 63, 30, 3.4, 0.5) +
      mark(130, 60.5, 20, 9, 0.24, 4.5) +
      workRow(70, 64),
  ),

  /* Progress figures — four figures in a row, over the hairline, over the
     week's own bars: the numbers first, the shape they came from under them. */
  'progress-stats': box(
    frame() +
      [10, 46, 82, 118]
        .map((x, i) => mark(x, 12, 20, 11, i === 0 ? 0.8 : 0.55, 2.5) + mark(x, 28, 26, 2.6, 0.28) + mark(x, 34, 18, 2.2, 0.18))
        .join('') +
      rule(10, 52, 150, 52, 0.35) +
      [10, 18, 8, 22, 16, 12, 20].map((h, i) => mark(14 + i * 19, 80 - h, 13, h, 0.3, 2.5)).join(''),
  ),

  /* Library — what you keep: the search well, then each thing as its name and
     the one line it previews. */
  'reference-library': box(
    pagehead() +
      edge(8, 26, 144, 13, 0.4, 6.5) +
      dot(17, 32.5, 3) +
      rule(19.6, 35, 22.5, 38, 0.5) +
      shelfRow(46) +
      shelfRow(60) +
      shelfRow(74),
  ),
};

/**
 * The drawing for a surface, or a plain frame when this build has no preview
 * for it. A widget whose preview has not been drawn yet is a stated gap, never
 * a hole in the gallery: the card still shows its name, note and Add, over the
 * one shape every widget has in common (a line on the screen).
 */
export function previewFor(surface: string): string {
  return (
    PREVIEWS[surface as Surface] ??
    box(frame() + mark(24, 40, 112, 4, 0.4, 2) + mark(60, 48, 40, 2.6, 0.22))
  );
}
