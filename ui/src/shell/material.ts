/**
 * LAYER 1 — THE MATERIAL. Painted once and never animated: the surface the
 * light in `veil.ts` is seen through. A **variable-dot halftone** — a screen of
 * six-pixel cells, each holding a dot whose radius is the tone of the patch it
 * sits in — written as ink, one polarity, no white tooth at all: this texture
 * only ever takes light away, which is what a print on a surface does. No blend
 * mode is needed anywhere, which matters — a full-window blend re-composites the
 * whole app behind it every frame.
 *
 * A CSS gradient stack cannot do this: repeating gradients are periodic by
 * construction, and a period is a stripe. The tile is periodic too, at 240px —
 * forty dots of a six-pixel pitch — which is fine, because a halftone's period
 * *is* its subject. The eye finding the repeat of a screen is not the same
 * failure as the eye finding the repeat of a blot: the screen was always a
 * screen, and the tone carried inside it is not periodic at all.
 */
import { MAX_DPR } from './veil';

const TILE = 240;
/** Pixels between dot centres. Eight is a print's pitch: at 1:1 the dots
    resolve as dots, at arm's length they average back into a tone, and the
    screen stays an octave away from the size of a glyph — a dot pattern at the
    pitch of text shimmers against the text it sits under. */
const PITCH = 8;

/** The tone a cell is holding, 0..1 — how big its dot is allowed to be. This is
    a halftone, not a stipple: neighbouring cells hold similar tones, so the dots
    grow and shrink in patches the way a screen's do, and the eye reads a
    surface printed with a tone rather than a scatter of specks. */
function tone(cx: number, cy: number): number {
  return noise(cx * PITCH + PITCH / 2, cy * PITCH + PITCH / 2, 120);
}

/** A hash, not a PRNG: the same tile every launch, on every machine. */
function rnd(x: number, y: number): number {
  let h = Math.imul(x, 374761393) + Math.imul(y, 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
}

/** Value noise on a lattice that wraps at the tile edge, so the tile tiles. */
function noise(x: number, y: number, pitch: number): number {
  const span = TILE / pitch;
  const fx = x / pitch;
  const fy = y / pitch;
  const x0 = Math.floor(fx);
  const y0 = Math.floor(fy);
  const tx = fx - x0;
  const ty = fy - y0;
  const sx = tx * tx * (3 - 2 * tx);
  const sy = ty * ty * (3 - 2 * ty);
  const wrap = (v: number) => ((v % span) + span) % span;
  const xa = wrap(x0);
  const xb = wrap(x0 + 1);
  const ya = wrap(y0);
  const yb = wrap(y0 + 1);
  const top = rnd(xa, ya) + (rnd(xb, ya) - rnd(xa, ya)) * sx;
  const bottom = rnd(xa, yb) + (rnd(xb, yb) - rnd(xa, yb)) * sx;
  return top + (bottom - top) * sy;
}

/**
 * The tile itself — 240 square, tileable, and the only form the material has.
 * The live field samples it as a texture (`veil.ts`) and the no-WebGL2 path
 * fills the layer with it as a pattern; neither owns a second copy.
 */
export function materialTile(): HTMLCanvasElement {
  const made = document.createElement('canvas');
  made.width = TILE;
  made.height = TILE;
  const ctx = made.getContext('2d');
  if (!ctx) return made;
  const bytes = ctx.createImageData(TILE, TILE);
  const data = bytes.data;
  for (let y = 0; y < TILE; y += 1) {
    for (let x = 0; x < TILE; x += 1) {
      const at = (y * TILE + x) * 4;
      // A screen, not a speck. Every dot belongs to a cell of a fixed pitch, and
      // its radius is the tone that cell is holding — so the dots are *places*
      // first and marks second. That is the whole difference between a texture
      // and dust: dust has no cell to belong to, which is why no amount of it
      // resolves into anything.
      const cx = Math.floor(x / PITCH);
      const cy = Math.floor(y / PITCH);
      // The radius *is* the tone, and the range is wide on purpose: patches
      // where the dots nearly touch and patches where they shrink to a pinprick
      // are what a screen printed at a gradient actually looks like. A narrow
      // range makes every dot the same size, and a screen of identical dots is a
      // mesh over the window rather than a tone held by the surface.
      const radius = 0.14 + 0.40 * tone(cx, cy);
      const dx = (x % PITCH) / PITCH - 0.5;
      const dy = (y % PITCH) / PITCH - 0.5;
      const d = Math.sqrt(dx * dx + dy * dy);
      // One edge, in tone space rather than pixel space, so a dot is round at
      // every pitch and no dot is ever a square.
      const cover = 1 - Math.min(1, Math.max(0, (d - radius) / 0.055 + 0.5));
      const ink = 0;
      data[at] = ink;
      data[at + 1] = ink;
      data[at + 2] = ink;
      // Ink only. A dot is where the surface is *darkest*, so this texture can
      // take light away and can never add it — which is what keeps it a print
      // on the surface instead of a sparkle coming off it.
      data[at + 3] = Math.round(cover * 235);
    }
  }
  ctx.putImageData(bytes, 0, 0);
  return made;
}

/**
 * Paints the material into `canvas` for its current box, and keeps doing so when
 * the box changes. One pattern fill per size, never per frame.
 */
export function paintMaterial(canvas: HTMLCanvasElement, grain: HTMLCanvasElement): () => void {
  const ctx = canvas.getContext('2d', { alpha: true });
  let stop = false;

  function paint() {
    if (stop || !ctx) return;
    const box = canvas.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, MAX_DPR);
    const width = Math.max(1, Math.round(box.width * dpr));
    const height = Math.max(1, Math.round(box.height * dpr));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    const pattern = ctx.createPattern(grain, 'repeat');
    if (!pattern) return;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = pattern;
    ctx.fillRect(0, 0, width, height);
  }

  paint();
  const observer = new ResizeObserver(paint);
  observer.observe(canvas);

  return () => {
    stop = true;
    observer.disconnect();
  };
}
