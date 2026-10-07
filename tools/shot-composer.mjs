#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE COMPOSER REVIEW CAPTURES (dev only, 2026-09-29)

   The composer's flows land in `.captures/composer/` as a screen × state ×
   theme grid: Today in view mode and in edit mode, an empty composed screen,
   the inserter sheet, the design switcher, a frame's on-surface chrome (the
   design carousel and the ✕), a drag mid-flight and the new-screen sheet —
   each in light and in dark, at the tool's own default window (1180×780,
   deviceScaleFactor 2).

     node tools/shot-composer.mjs --port 4462 --out .captures/composer

   Every state is reached through the app's **own controls** — the titlebar
   pencil (`data-command="screen.edit"`), the edit bar, the rail's Add disc,
   the frame's own buttons — never by poking store state, because a capture
   taken through a side door proves nothing about the door the student uses.

   Two consequences of that rule, stated rather than hidden:

   - The tool **writes to the plan it is pointed at**: a drag mid-flight is
     captured and then released exactly where it began (a no-op), but the
     empty screen it needs is minted through the rail's Add sheet
     (`list.new` without a type) if the plan does not already carry one named
     `--empty` (default *My screen*). Point it at a scratch plan.
   - Light and dark come from the app's own stored preference
     (`cadence.theme`, the same key `appearance.ts` reads) via `addInitScript`,
     never from a class poked into the DOM.
   ══════════════════════════════════════════════════════════════════════════ */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { existsSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(name, fallback) {
  const at = process.argv.indexOf(name);
  return at >= 0 && process.argv[at + 1] ? process.argv[at + 1] : fallback;
}

const PORT = arg('--port', '4462');
const OUT = join(ROOT, arg('--out', '.captures/composer'));
const EMPTY_NAME = arg('--empty', 'My screen');
const THEMES = (arg('--theme', 'both') === 'both' ? ['light', 'dark'] : [arg('--theme', 'both')]).filter(
  (theme) => theme === 'light' || theme === 'dark',
);
const W = Number(arg('--w', '1180'));
const H = Number(arg('--h', '780'));

if (!existsSync(OUT)) mkdirSync(OUT, { recursive: true });
if (THEMES.length === 0) {
  console.error('shot-composer: --theme must be light, dark or both');
  process.exit(2);
}

const errors = [];

/** One window, one theme, one walk through every state. */
async function captureTheme(browser, theme) {
  const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
  await ctx.addInitScript((mode) => {
    try {
      localStorage.setItem('cadence.theme', mode);
    } catch (e) {
      /* a blocked store shows the floor; the capture still runs */
    }
  }, theme);

  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(`[${theme}] ${String(e).slice(0, 200)}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`[${theme}] ${m.text().slice(0, 200)}`);
  });

  const shot = async (name) => {
    const label = `${name}-${theme}.png`;
    await page.screenshot({ path: join(OUT, label) });
    console.log(`  ${label}`);
  };
  const pause = (ms = 350) => page.waitForTimeout(ms);
  const escape = async () => {
    await page.keyboard.press('Escape');
    await pause();
  };

  await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
  await pause(1200);

  // ── Today, the designed screen, in the student's hands ─────────────────
  await page.locator('.cd-nav .cd-disc[data-view="today.screen"]').first().click();
  await page.waitForTimeout(900);
  await shot('today-view');

  // The pencil in the titlebar — the app's own door into edit mode.
  await page.locator('button[data-command="screen.edit"]').first().click();
  await page.waitForSelector('.cmp-bar', { timeout: 10000 });
  await pause();
  await shot('today-edit');

  // The inserter, from the edit bar's own Add component.
  await page.locator('.cmp-bar [data-opens="inserter"]').first().click();
  await page.waitForSelector('.cd-sheet', { timeout: 10000 });
  await pause(400);
  await shot('inserter');
  await escape();

  // The design carousel, pressed (a surface with designs): `›` steps it in
  // place — the label moves A→B and nothing opens (COMPOSER §4.5).
  await page.locator('.cmp-step[aria-label^="Next design"]').first().click();
  await pause(400);
  await shot('design-carousel');
  await escape();

  // A frame's own acts, which live on the surface (COMPOSER §4.3): the two
  // design arrows around the name chip, and the ✕ at the bar's right edge. The
  // frame carries no second menu, so there is nothing to open here — the shot
  // is the chrome itself.
  await shot('frame-chrome');

  // A drag mid-flight: press the handle (or, since a press anywhere on the
  // frame starts the drag, any other part of it), clear the 4px threshold, and
  // hold there while the drop line is up. Released back where it began, so the
  // gesture leaves the plan exactly as it found it.
  const handle = page.locator('.cmp-handle').first();
  const box = await handle.boundingBox();
  if (box) {
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    // The engine captures the pointer **on the press**, so the moves below are
    // retargeted to the element that was pressed wherever they land; the 6px
    // first step only has to clear the 4px threshold that turns a press into a
    // drag.
    await page.mouse.move(x, y + 6, { steps: 2 });
    await page.waitForTimeout(120);
    await page.mouse.move(x, y + 70, { steps: 8 });
    await page.waitForTimeout(250);
    await shot('drag');
    await page.mouse.move(x, y, { steps: 8 });
    await page.mouse.up();
    await pause();
  } else {
    errors.push(`[${theme}] no drag handle to press`);
  }

  // And out again, by the same pencil.
  await page.locator('button[data-command="screen.edit"]').first().click();
  await pause();

  // ── The new-screen sheet, from the rail's Add disc ─────────────────────
  await page.locator('.cd-nav .cd-disc[data-command="list.new"]').first().click();
  await page.waitForSelector('.cd-sheet', { timeout: 10000 });
  await pause(400);
  await shot('new-screen-sheet');

  // The empty state: mint the screen through this sheet the first time it is
  // needed, then open it by its own rail disc on every later pass.
  const existing = page.locator('.cd-nav .cd-disc', { hasText: EMPTY_NAME });
  if ((await existing.count()) === 0) {
    await page.locator('#screen-name').fill(EMPTY_NAME);
    await page.locator('.cd-sheet__foot button[data-command="list.new"]').click();
    // A new screen opens in edit mode with the catalog open (§4.7): close the
    // inserter, leave the mode, and look at the empty state itself.
    await page.waitForSelector('.cmp-bar', { timeout: 10000 });
    await pause(500);
    await escape();
    await page.locator('button[data-command="screen.edit"]').first().click();
    await pause();
  } else {
    await page.locator('.cd-sheet__head button[aria-label="Close"]').click();
    await pause();
    await existing.first().click();
    await page.waitForTimeout(700);
  }
  await shot('empty');

  await ctx.close();
}

const browser = await chromium.launch();
for (const theme of THEMES) {
  console.log(`\n${theme}:`);
  await captureTheme(browser, theme);
}
await browser.close();

if (errors.length) {
  console.log(`\npage errors (${errors.length}):`);
  for (const error of errors.slice(0, 8)) console.log('  ' + error);
  process.exitCode = 1;
}
