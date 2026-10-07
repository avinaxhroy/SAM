#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE VARIANT REVIEW CAPTURES (dev only, 2026-09-29)

   Every surface the redesign lab carries now ships three designs, and the app
   draws the one the student kept (`cadence.component-styles`). This tool drives
   the running web harness through those choices, so a screen can be looked at
   per design and per interface size:

     node tools/shot-variants.mjs --port 4401 --variant b --size compact \
       --dest today,plan --out .captures/variants

   `--variant` sets the choice for every surface the catalog declares, read out
   of `ui/src/variants/catalog.ts` so the list has one home; `--surface courses`
   narrows the write to that one surface and leaves the rest at their defaults.
   `--size` sets the interface size. Both are written with `addInitScript` —
   the app's own storage keys, applied before boot — never a class poked into
   the DOM, which would prove nothing about the real store.

   Destinations are the rail's own view ids (`today.screen`, …), plus `system`
   (the rail's cube) and `settings` (the keymap's own ⌘,).
   ══════════════════════════════════════════════════════════════════════════ */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(name, fallback) {
  const at = process.argv.indexOf(name);
  return at >= 0 && process.argv[at + 1] ? process.argv[at + 1] : fallback;
}

const PORT = arg('--port', '4401');
const VARIANT = arg('--variant', 'a'); // a | b | c
const SURFACE = arg('--surface', null); // one surface key, or null = every surface
const SIZE = arg('--size', null); // compact | regular | spacious
const DEST = arg('--dest', 'today.screen'); // comma-separated destinations
const OUT = join(ROOT, arg('--out', '.captures/variants'));
const W = Number(arg('--w', '1180'));
const H = Number(arg('--h', '780'));
const FULL = process.argv.includes('--full');

if (!existsSync(OUT)) mkdirSync(OUT, { recursive: true });

/** Every surface key the catalog declares — the one list, read from source. */
const KEYS = [...readFileSync(join(ROOT, 'ui/src/variants/catalog.ts'), 'utf8').matchAll(/^ {4}key: '([^']+)',$/gm)].map(
  (match) => match[1],
);
if (KEYS.length === 0) console.warn('shot-variants: no catalog keys read — capturing defaults only');

const choices = {};
for (const key of KEYS) {
  if (!SURFACE || SURFACE === key) choices[key] = VARIANT;
}

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));
page.on('console', (m) => {
  if (m.type() === 'error') errors.push(m.text().slice(0, 200));
});

await ctx.addInitScript(
  ([picked, size]) => {
    try {
      localStorage.setItem('cadence.component-styles', JSON.stringify(picked));
      if (size) localStorage.setItem('cadence.ui-size', size);
    } catch (e) {
      /* a blocked store shows the defaults; the capture still runs */
    }
  },
  [choices, SIZE],
);

await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1200);

/** Open a destination through the app's own doors, never by poking state. */
async function open(dest) {
  const disc = page.locator(`.cd-nav .cd-disc[data-view="${dest}"]`).first();
  if (await disc.count()) {
    await disc.click();
    return true;
  }
  if (dest === 'system') {
    await page.locator('.cd-nav .cd-disc[data-command="app.openSystem"]').first().click();
    return true;
  }
  if (dest === 'settings') {
    await page.keyboard.press('Meta+,');
    await page.waitForTimeout(600);
    return (await page.locator('.set-group').count()) > 0;
  }
  // A view with no rail entry — the generic block layer, where a record table
  // renders — is driven through the one command that selects a destination,
  // the same door the gate suite uses (`tools/gates.mjs`).
  const driven = await page.evaluate((id) => {
    const hook = globalThis.__SAM_APP__;
    if (!hook || typeof hook.run !== 'function') return false;
    void hook.run('rail.select', { view: id });
    return true;
  }, dest);
  if (driven) {
    await page.waitForSelector('.cd-canvas h1, .cd-canvas h2, .cd-empty', { timeout: 10000 }).catch(() => {});
    return true;
  }
  return false;
}

for (const dest of DEST.split(',').map((value) => value.trim()).filter(Boolean)) {
  const opened = await open(dest);
  await page.waitForTimeout(900);
  const label = `${SURFACE ?? 'all'}-${VARIANT}${SIZE ? `-${SIZE}` : ''}-${dest.replace(/\W+/g, '_')}.png`;
  if (FULL) await page.screenshot({ path: join(OUT, label), fullPage: true });
  else await page.screenshot({ path: join(OUT, label) });
  console.log(`  ${label}${opened ? '' : '  (destination not found)'}`);
}

await browser.close();
if (errors.length) {
  console.log(`\npage errors (${errors.length}):`);
  for (const error of errors.slice(0, 6)) console.log('  ' + error);
  process.exitCode = 1;
}
