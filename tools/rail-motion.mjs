#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE RAIL'S MATERIAL ON ITS SPRING  (dev only)

   The rail's hover object is one box on one spring (`App.svelte`: the tube, the
   `covered` set, `regions`, and the per-disc `--reveal` it implies), and the
   claims it makes are *motion* claims: a retarget continues the movement rather
   than restarting it, nothing the material merely passes opens a name, the name
   is invisible until the edge has covered all but the last tenth of its clip,
   the disc's own width follows that clip, and the release retracts where it
   stands and flows back to the column as a drop — never the pill ballooning
   into a slab. None of that survives a still screenshot, so this drives
   the gestures in real Chromium and judges the *traces*:

     enter   the column onto the first disc, and the settle
     hop     one disc down from a settled pill — contract to a body, crawl, open
     sweep   four discs crossed at pointer speed, stopping on the fifth
     far     first disc to the last the window can reach — the longest travel
     gap     stop in the gap between two discs — nothing left half-open
     leave   off the rail — the column comes back, every inline write cleared
     keyboard  real Tab keys carry the material disc to disc; blur releases it
     quiet   `prefers-reduced-motion`, its own context — no travel at all

   Each gesture records the tube's border box, every disc's width, `--reveal`,
   `is-bared`, the hovered key and the label's computed opacity, per animation
   frame; the checks are geometry, not taste (a reveal is legal only while the
   tube's box covers that disc's label region — one frame of aim/paint lag
   allowed, because an aim lands between paints — so no value can outlive the
   material that wrote it; a bared disc's width must read `44 + reveal`; the
   name must stay invisible until the clip has covered all but its last tenth).
   Film frames of the rail region are written per gesture beside the traces.

   node tools/rail-motion.mjs [--port 4399] [--out tmp/rail-motion]

   The app must be served *with its plan* — the rail's discs are the plan's own
   navigation, and the page reaches the engine only through the harness's IPC
   bridge:

     node tools/web-ipc.mjs --plan /tmp/samlive --port 4399

   (a plain `vite preview` serves the frontend but no plan, so the rail never
   draws). Exit code is the number of flagged checks (0 = clean).

   The run also writes `reel.txt` (a concat list at the *measured* shot times),
   so the stills cut into a video of the real cadence:

     ffmpeg -y -f concat -safe 0 -i tmp/rail-motion/reel.txt \
       -vf "scale=380:-2,fps=30" -pix_fmt yuv420p -movflags +faststart \
       tmp/rail-motion.mp4
*/
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const PORT = arg('--port', '4399');
const OUT = join(ROOT, arg('--out', 'tmp/rail-motion'));
mkdirSync(OUT, { recursive: true });
const W = 1180, H = 780;
const RAIL = { x: 12, y: 88, width: 190, height: 632 };
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 300)); });
await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 15000 });
await page.waitForTimeout(900);

/* -- the instrument: every frame, the tube's box + every disc's width/reveal */
await instrument(page);
async function instrument(page) {
await page.evaluate(() => {
  const nav = document.querySelector('.cd-nav');
  const tube = document.querySelector('.cd-nav__tube');
  window.__T__ = [];
  window.__ON__ = false;
  const t0 = performance.now();
  const key = (el) => el.dataset.rail ?? el.dataset.view ?? 'system';
  const sample = () => {
    if (window.__ON__) {
      const r = tube.getBoundingClientRect();
      const n = nav.getBoundingClientRect();
      const hover = document.querySelector('.cd-nav .cd-disc:hover');
      const hop = hover ? Number.parseFloat(getComputedStyle(hover.querySelector('.cd-disc__label')).opacity) : null;
      const discs = {};
      for (const d of document.querySelectorAll('.cd-nav .cd-disc')) {
        const db = d.getBoundingClientRect();
        discs[key(d)] = {
          l: Math.round((db.left - n.left) * 100) / 100,
          t: Math.round((db.top - n.top) * 100) / 100,
          w: Math.round(d.offsetWidth * 100) / 100,
          rv: Math.round((Number.parseFloat(d.style.getPropertyValue('--reveal')) || 0) * 100) / 100,
          bare: d.classList.contains('is-bared'),
          cur: d.getAttribute('aria-current') === 'page',
        };
      }
      window.__T__.push({
        t: Math.round(performance.now() - t0),
        x: Math.round((r.left - n.left) * 100) / 100,
        y: Math.round((r.top - n.top) * 100) / 100,
        w: Math.round(r.width * 100) / 100,
        h: Math.round(r.height * 100) / 100,
        hug: nav.classList.contains('is-hugging'),
        hop,
        hover: hover ? key(hover) : null,
        discs,
      });
    }
    requestAnimationFrame(sample);
  };
  requestAnimationFrame(sample);
});
}

let fails = 0;
const allShots = {};
const discs = [];
for (const el of await page.$$('.cd-nav .cd-disc')) {
  const bb = await el.boundingBox();
  discs.push({ key: await el.getAttribute('data-rail') ?? await el.getAttribute('data-view') ?? 'system', box: { x: bb.x + bb.width / 2, y: bb.y + bb.height / 2 } });
}
console.log('discs:', discs.map((d) => d.key).join(' '));

let filmN = 0;
async function gesture(name, opts, fn) {
  const dir = join(OUT, `film-${name}`);
  mkdirSync(dir, { recursive: true });
  const shots = [];
  filmN = 0;
  const t0 = Date.now();
  await page.evaluate(() => { window.__T__ = []; window.__ON__ = true; });
  const shot = async () => {
    const file = join(dir, `${String(filmN++).padStart(3, '0')}.png`);
    await page.screenshot({ path: file, clip: RAIL });
    shots.push({ file, at: Date.now() - t0 });     // measured, for the reel
  };
  await fn(shot);
  await page.evaluate(() => { window.__ON__ = false; });
  const trace = await page.evaluate(() => window.__T__.slice());
  writeFileSync(join(OUT, `trace-${name}.json`), JSON.stringify(trace));
  writeFileSync(join(OUT, `timings-${name}.json`), JSON.stringify(shots));
  allShots[name] = shots;
  console.log(`\n── ${name}: ${trace.length} frames, ${shots.length} shots`);
  report(trace, opts);
  return trace;
}

function report(trace, opts) {
  if (!trace.length) { console.log('  (no frames)'); return; }
  const f = (v) => (Math.round(v * 100) / 100).toFixed(2).padStart(7);
  console.log('  t     x       y       w       h       hug  hover   reveals');
  const step = Math.max(1, Math.floor(trace.length / 40));
  for (let i = 0; i < trace.length; i += step) {
    const s = trace[i];
    const rv = Object.entries(s.discs).filter(([, d]) => d.rv > 0.01 || d.bare).map(([k, d]) => `${k}:${d.rv}${d.bare ? 'B' : ''}`).join(' ');
    console.log(`  ${String(s.t).padStart(5)} ${f(s.x)} ${f(s.y)} ${f(s.w)} ${f(s.h)} ${s.hug ? ' yes' : '  no'} ${String(s.hover).padEnd(7)} ${rv}`);
  }
  // settle: last frame where any axis moves more than 0.05px
  let settle = trace[0].t;
  for (let i = 1; i < trace.length; i++) {
    const a = trace[i - 1], b = trace[i];
    if (Math.abs(b.x - a.x) > 0.05 || Math.abs(b.y - a.y) > 0.05 || Math.abs(b.w - a.w) > 0.05 || Math.abs(b.h - a.h) > 0.05) settle = b.t;
  }
  const last = trace[trace.length - 1];
  const first = trace[0];
  console.log(`  → settle at +${settle - first.t}ms after first frame; final box ${f(last.x)} ${f(last.y)} ${f(last.w)} ${f(last.h)}`);
  // overshoot: a bounce — the box *leaves* the final value, comes back to it,
  // and only then goes past it. That order is the whole definition, because
  // three readings a simpler metric took here were artifacts of the gesture's
  // own arithmetic, not bounces: a start a hair above a final a hair below it
  // invented a direction out of a rounding error (a settled pill at 114.00
  // aiming at its own 113.98 read the crawl's thin stretch as a 76px
  // overshoot); a start *at* the final value (every hop's width — the material
  // begins the gesture wearing the box it is heading for) read the body's
  // contract as an overshoot; and the leave's width, which *passes through* the
  // column's 60 on its way down to the body's 42.69 before the release expands
  // it back, read that pass-through as an arrival. The return therefore has to
  // be *sustained* — three consecutive frames within 1px — not a crossing: a
  // body at speed spends one frame in that window, a settling box spends its
  // tail there. Leave, return, then past — and if any of the three never
  // happens, the answer is zero. What the number reports is the *tail's* own
  // excursion — how much a box still moves after it has arrived — so a healthy
  // damped arrival prints a fraction of a pixel, and a build whose size spring
  // is underdamped lifts it off the floor (a scratch `c: 4` spring read
  // 1.7–4.4px here against the damping below 0.7px; it does not report the
  // swing's widest point, only that a swing outlived the arrival).
  const over = {};
  for (const axis of ['x', 'y', 'w', 'h']) {
    const goal = last[axis];
    let m = 0, left = false, reached = false, at = 0;
    for (const s of trace) {
      const dev = Math.abs(s[axis] - goal);
      if (!left) { if (dev > 1) left = true; continue; }
      if (!reached) {
        at = dev <= 1 ? at + 1 : 0;
        if (at >= 3) reached = true;
        continue;
      }
      if (dev > m) m = dev;
    }
    over[axis] = Math.round(m * 100) / 100;
  }
  console.log(`  → overshoot past the final box: ${JSON.stringify(over)}`);
  // max width deviation between the first and last 100ms (the pill must not contract)
  const mid = trace.filter((s) => s.t > first.t + 120 && s.t < settle - 60);
  if (mid.length) {
    const ws = mid.map((s) => s.w);
    console.log(`  → width through the travel: min ${Math.min(...ws)} max ${Math.max(...ws)} (first ${first.w} last ${last.w})`);
  }
  // reveals ever seen, and the max reveal per disc
  const maxRv = {};
  let stray = [];
  // A reveal is geometry: `paint` writes it only while the material covers that
  // disc's label region — the hug, and the leavers whose edges are still
  // clearing it — and zeroes every other disc, which is what keeps a name from
  // opening on a disc the material only passes. So a non-zero reveal with the
  // tube nowhere near that disc's row is a value the paint has stopped
  // maintaining: the stray-open defect this check exists for. One frame of lag
  // is allowed, because an aim lands between paints and the frame the pointer
  // moves on still carries the previous paint's value.
  const region = (d) => ({ l: d.l + 44, t: d.t, w: 70, h: 44 });
  const covers = (s, d) => {
    const r = region(d);
    return Math.min(s.x + s.w, r.l + r.w) > Math.max(s.x, r.l)
      && Math.min(s.y + s.h, r.t + r.h) > Math.max(s.y, r.t);
  };
  for (let i = 0; i < trace.length; i++) {
    const s = trace[i];
    for (const [k, d] of Object.entries(s.discs)) {
      maxRv[k] = Math.max(maxRv[k] ?? 0, d.rv);
      if (d.rv <= 0.5) continue;
      if (covers(s, d) || (i > 0 && covers(trace[i - 1], d))) continue;
      stray.push(`${k}@${s.t} rv=${d.rv} tube=${s.x},${s.y},${s.w},${s.h}`);
    }
  }
  console.log(`  → max reveal per disc: ${Object.entries(maxRv).filter(([, v]) => v > 0.5).map(([k, v]) => `${k}:${v}`).join(' ') || '(none)'}`);
  if (stray.length) { fails++; console.log(`  → ! reveals with the material off the disc's row: ${stray.slice(0, 12).join(', ')}${stray.length > 12 ? ` … ${stray.length}` : ''}`); }
  // the name's own gate, in geometry: invisible until the edge has covered all
  // but the last tenth of the label, then fading in across it.
  const gate = [];
  for (const s of trace) {
    if (s.hover === null || s.hop === null) continue;
    const d = s.discs[s.hover];
    if (!d) continue;
    if (d.rv > 0.5 && d.rv < 61 && s.hop > 0.02) gate.push(`visible@${s.t} rv=${d.rv} op=${s.hop}`);
    if (d.rv >= 69.5 && s.hop < 0.9) gate.push(`hidden@${s.t} rv=${d.rv} op=${s.hop}`);
  }
  if (gate.length) { fails++; console.log(`  → ! name gate (cut glyph or missing name): ${gate.slice(0, 8).join(', ')}${gate.length > 8 ? ` … ${gate.length}` : ''}`); }
  else console.log('  → name invisible under 61px of clip, in at 67+ (no cut glyph)');
  // width of the disc whose reveal is open, vs its reveal (the CSS calc)
  const mismatch = [];
  for (const s of trace) {
    for (const [k, d] of Object.entries(s.discs)) {
      const want = d.bare && s.hug ? Math.round((44 + d.rv) * 100) / 100 : null;
      if (want !== null && Math.abs(d.w - want) > 1.5) mismatch.push(`${k}@${s.t} w=${d.w} want=${want}`);
    }
  }
  if (mismatch.length) { fails++; console.log(`  → ! disc width vs --reveal: ${mismatch.slice(0, 8).join(', ')}${mismatch.length > 8 ? ` … ${mismatch.length}` : ''}`); }
  else console.log('  → disc width tracks --reveal on every frame (44 + reveal)');
  // continuity, in all four dimensions of the box: a cut (the old flicker) is a
  // single frame that teleports; a fast spring is a run of large-but-consecutive
  // steps. Both are reported, so speed is never mistaken for a discontinuity —
  // and a sample gap (a screenshot between two animation frames) reads as a
  // large *step*, not as a cut: the app's own integrator clamps dt. The total is
  // the sum of those steps — the box's own path length, the number a share like
  // "all but a tenth of the travel" has to be measured against.
  let maxStep = 0, maxRun = 0, run = 0, total = 0;
  const boxStep = (a, b) => Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y), Math.abs(b.w - a.w), Math.abs(b.h - a.h));
  for (let i = 1; i < trace.length; i++) {
    const d = boxStep(trace[i - 1], trace[i]);
    total += d;
    if (d > maxStep) maxStep = d;
    if (d > 6) { run += d; } else { if (run > maxRun) maxRun = run; run = 0; }
  }
  if (run > maxRun) maxRun = run;
  console.log(`  → continuity: largest one-frame step ${Math.round(maxStep * 100) / 100}px (x/y/w/h); the box's path covers ${Math.round(total)}px — longest fast run ${Math.round(maxRun)}px of it in consecutive >6px frames`);
  // The beats (gestures that ask for them): on the way to the destination the
  // material must *be* a body — a contracted box, neither the pill it left
  // (114×44), nor the column (60 tall-body), nor a pill passing through. The
  // pill's own box only has to be absent until the arrival, which is the
  // owner's order: contract where it stands, then move, then open. A tolerance
  // of 20px across the two axes keeps the stretch's deformation and the
  // sub-pixel tail out of the judgment; the shape's radius is not judged (the
  // capsule formula is `chrome.css`'s business, and the tool already checks the
  // reveals the same box writes).
  if (opts.beats) {
    const body = trace.find((s) => s.w <= 56 && s.h <= 72 && Math.abs(s.w - s.h) <= 20);
    const landed = trace[trace.length - 1];
    const opened = landed.w >= 110 && landed.h <= 48;
    console.log(body
      ? `  → beats: contracted body at ${body.t}ms ${body.x},${body.y} ${body.w}×${body.h}; destination opened ${opened ? 'yes' : 'no'}`
      : `  → ! beats: the material never contracted to a body — it travelled as a ${trace[0].w}×${trace[0].h} pill`);
    if (!body || !opened) fails++;
  }
}

/* 1 · ENTER — from the column onto Today, and the settle. */
await gesture('enter', {}, async (shot) => {
  await shot();
  await page.mouse.move(discs[0].box.x, discs[0].box.y, { steps: 1 });
  for (let i = 0; i < 8; i++) { await shot(); await page.waitForTimeout(50); }
  await page.waitForTimeout(1300);
  await shot();
});

/* 2 · HOP — one disc down, from a settled pill: the material must contract to a
   body where it stands, crawl, and open on the next disc — never slide across
   as a pill. That order is the owner's own words (three passes), and the check
   is geometric: *some* frame on the way must be a contracted body, not the
   pill's own box. */
await gesture('hop', { beats: true }, async (shot) => {
  await shot();
  await page.mouse.move(discs[1].box.x, discs[1].box.y, { steps: 1 });
  for (let i = 0; i < 8; i++) { await shot(); await page.waitForTimeout(50); }
  await page.waitForTimeout(1300);
  await shot();
});

/* 3 · SWEEP — four discs crossed at pointer speed, stopping on the fifth:
   every retarget must continue the movement, not restart it, and the discs
   merely passed must never open. */
await gesture('sweep', {}, async (shot) => {
  await page.mouse.move(discs[0].box.x, discs[0].box.y, { steps: 1 });
  await page.waitForTimeout(120);
  for (let i = 1; i <= 4; i++) {
    await page.mouse.move(discs[i].box.x, discs[i].box.y, { steps: 1 });
    await shot();
    await page.waitForTimeout(45);
  }
  await page.waitForTimeout(1300);
  await shot();
});

/* 4 · FAR — first disc to the last one the pointer can land on inside this
   window, one move: the longest travel the rail can make here. A disc whose
   centre sits below the fold cannot be aimed at (the rail can outgrow a short
   window), and it is named in the report instead of quietly shortening the
   gesture. */
const lastReach = [...discs].reverse().find((d) => d.box.y <= H);
const belowFold = discs.filter((d) => d.box.y > H);
await gesture('far', { beats: true }, async (shot) => {
  await page.mouse.move(discs[0].box.x, discs[0].box.y, { steps: 1 });
  await page.waitForTimeout(650);
  await shot();
  await page.mouse.move(lastReach.box.x, lastReach.box.y, { steps: 1 });
  for (let i = 0; i < 8; i++) { await shot(); await page.waitForTimeout(50); }
  await page.waitForTimeout(1300);
  await shot();
});
if (belowFold.length) console.log(`  → below the fold at ${W}×${H}: ${belowFold.map((d) => d.key).join(', ')} — the far ends on ${lastReach.key}`);

/* 5 · GAP — cross two discs and stop in the gap between them: nothing may be
   left half-open. */
await gesture('gap', {}, async (shot) => {
  await page.mouse.move(discs[1].box.x, discs[1].box.y, { steps: 1 });
  await page.waitForTimeout(650);
  await shot();
  await page.mouse.move(discs[2].box.x, discs[2].box.y + 22, { steps: 1 });
  await page.waitForTimeout(1300);
  await shot();
});

/* 6 · LEAVE — off the rail into the canvas: the column comes back. */
await gesture('leave', {}, async (shot) => {
  await page.mouse.move(discs[2].box.x, discs[2].box.y, { steps: 1 });
  await page.waitForTimeout(700);
  await shot();
  await page.mouse.move(700, 400, { steps: 1 });
  for (let i = 0; i < 8; i++) { await shot(); await page.waitForTimeout(50); }
  await page.waitForTimeout(700);
  await shot();
  const style = await page.evaluate(() => document.querySelector('.cd-nav__tube').getAttribute('style'));
  console.log(`  → tube style attribute at rest: ${style === null || style === '' ? '(empty)' : style}`);
  const discState = await page.evaluate(() => [...document.querySelectorAll('.cd-nav .cd-disc')].map((d) => ({ k: d.dataset.rail ?? d.dataset.view ?? 'system', w: d.offsetWidth, rv: d.style.getPropertyValue('--reveal'), bare: d.classList.contains('is-bared') })));
  console.log(`  → discs at rest: ${discState.map((d) => `${d.k}:w${d.w}${d.bare ? ' B(' + (d.rv || '0') + ')' : ''}`).join(' ')}`);
});

/* 7 · QUIET — `prefers-reduced-motion: reduce` on its own context: the material
   is *where the pointer is*, in one frame, and the name is open on landing. */
{
  const qctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1, reducedMotion: 'reduce' });
  const qpage = await qctx.newPage();
  await qpage.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
  await qpage.waitForSelector('.cd-nav .cd-disc', { timeout: 15000 });
  await qpage.waitForTimeout(900);
  await instrument(qpage);
  await qpage.evaluate(() => { window.__ON__ = true; });
  const box = await qpage.evaluate(() => {
    const d = document.querySelectorAll('.cd-nav .cd-disc')[1].getBoundingClientRect();
    return { x: d.left + d.width / 2, y: d.top + d.height / 2 };
  });
  await qpage.mouse.move(box.x, box.y, { steps: 1 });
  await qpage.waitForTimeout(400);
  const trace = await qpage.evaluate(() => window.__T__);
  await qctx.close();
  const last = trace[trace.length - 1];
  const pill = last.hover !== null && last.discs[last.hover] && Math.abs(last.discs[last.hover].w - (44 + last.discs[last.hover].rv)) < 1.5 && last.hop > 0.9;
  // every frame after the first with the pointer down must already be the pill's box
  const pillBox = [last.x, last.y, last.w, last.h];
  const moved = trace.filter((s) => s.hover !== null).slice(1).filter((s) => [s.x, s.y, s.w, s.h].some((v, i) => Math.abs(v - pillBox[i]) > 0.5));
  const quiet = moved.length === 0 && last.w === 114 && last.h === 44 && last.hug;
  console.log(`── quiet: ${trace.length} frames, then ${moved.length} frame(s) between the hover and the pill's box`);
  console.log(quiet === true ? `  → material is at the pill in one frame (${last.x},${last.y},${last.w},${last.h}), name open, is-hugging held` : `  → ! reduced-motion path moved over ${moved.length} frames (last ${last.x},${last.y},${last.w},${last.h} hug=${last.hug} hover=${last.hover})`);
  if (quiet !== true) fails++;
}

/* 8 · KEYBOARD — the material follows focus: real Tab keys carry it to the next
   disc, Shift+Tab to the previous, and a blur releases it to the column. The
   second pass fixed the `focusout` that could never release it; the spring
   rewrite must not have lost that path. */
{
  await instrument(page);
  let focused = null;
  for (let i = 0; i < 8 && focused === null; i++) {
    await page.keyboard.press('Tab');
    focused = await page.evaluate(() => {
      const el = document.activeElement;
      const disc = el && el.closest ? el.closest('.cd-nav .cd-disc') : null;
      return disc ? (disc.dataset.rail ?? disc.dataset.view ?? 'system') : null;
    });
  }
  const read = async () => page.evaluate(() => {
    const r = document.querySelector('.cd-nav__tube').getBoundingClientRect();
    const n = document.querySelector('.cd-nav').getBoundingClientRect();
    return { x: Math.round(r.left - n.left), y: Math.round(r.top - n.top), w: Math.round(r.width), h: Math.round(r.height) };
  });
  await page.waitForTimeout(1900);
  const atFirst = await read();
  await page.keyboard.press('Tab');
  await page.waitForTimeout(1900);
  const atSecond = await read();
  await page.evaluate(() => document.activeElement && document.activeElement.blur());
  await page.waitForTimeout(1900);
  const released = await read();
  const carried = focused !== null && atFirst.w === 114 && atSecond.w === 114 && atSecond.y !== atFirst.y;
  const back = released.h > 600 && released.w === 60;
  console.log(`── keyboard: focused ${focused} → ${JSON.stringify(atFirst)}; Tab → ${JSON.stringify(atSecond)}; blur → ${JSON.stringify(released)}`);
  console.log(carried && back ? '  → Tab carries the material disc to disc; blur gives the column back' : `  → ! keyboard path carried=${carried} released=${back}`);
  if (!(carried && back)) fails++;
}

/* the reel: the stills at the times they were actually taken, gesture after
   gesture, the last frame of each held for the pause that followed it. */
const reel = [];
for (const [name, shots] of Object.entries(allShots)) {
  for (let i = 0; i < shots.length; i++) {
    const dur = i + 1 < shots.length ? (shots[i + 1].at - shots[i].at) / 1000 : 0.7;
    reel.push(`file '${shots[i].file}'`, `duration ${dur.toFixed(3)}`);
  }
  reel.push(`file '${shots[shots.length - 1].file}'`);
}
writeFileSync(join(OUT, 'reel.txt'), reel.join('\n') + '\n');

console.log(`\ncast/page errors: ${errors.length ? [...new Set(errors)].slice(0, 6).join(' | ') : 'none'}`);
await browser.close();
if (errors.length) fails++;
console.log(fails === 0 ? 'rail-motion: clean' : `rail-motion: ${fails} flagged check(s)`);
process.exit(Math.min(fails, 99));
