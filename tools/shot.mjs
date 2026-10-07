#!/usr/bin/env node
/* Capture every SAM destination in real Chromium over the web harness.
   Navigation is a rail command (the app has no URL router), so this clicks discs.
   node tools/shot.mjs [--out .captures] [--port 4399] [--theme dark] [--w 1180] [--h 780]
*/
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { mkdirSync, existsSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const out = join(ROOT, arg('--out', '.captures'));
const port = arg('--port', '4399');
const theme = arg('--theme', 'light');
const W = Number(arg('--w', 1180));
const H = Number(arg('--h', 780));
const only = arg('--only', null);
const tag = arg('--tag', '');
const full = process.argv.includes('--full');

if (!existsSync(out)) mkdirSync(out, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => { errors.push(String(e).slice(0, 300)); });
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 200)); });

await page.goto(`http://localhost:${port}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1200);
/**
 * Toggles theme via the rail's theme disc. Inline custom properties set on
 * <html> take precedence over [data-theme="dark"], so modifying the attribute
 * directly is overridden by applyAppearance().
 *
 * Selected by accessible name to avoid matching the System navigation disc.
 */
async function setTheme(page, want) {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const current = await page.evaluate(() => {
      const raw = getComputedStyle(document.documentElement).getPropertyValue('--card').trim();
      const m = raw.match(/oklch\(([\d.]+)%/);
      return m ? Number(m[1]) : null;
    });
    const isDark = current !== null && current < 60;
    if (isDark === (want === 'dark')) return;
    const disc = page.locator('.cd-disc--theme');
    await disc.click({ force: true });
    await page.waitForTimeout(500);
  }
  const got = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card').trim());
  console.log(`  ! could not reach the ${want} theme; --card is ${got}`);
}

// The rail is the app's navigation: read it, then click each disc in order.
const discs = await page.$$eval('.cd-nav .cd-disc[data-view]', (nodes) =>
  nodes.map((n, i) => ({ i, view: n.getAttribute('data-view'), label: n.getAttribute('aria-label') })),
);
const systemDisc = await page.$('.cd-nav .cd-disc:not([data-view]):not(.cd-disc--add)');
console.log('rail:', discs.map((d) => d.view).join(', '));

const targets = only
  ? discs.filter((d) => d.view === only || d.label === only)
  : discs;
if (systemDisc && (!only || only === 'system')) targets.push({ i: 'system', view: 'system' });

for (const t of targets) {
  if (t.view === 'system') {
    await page.click('.cd-nav .cd-disc:not([data-view]):not(.cd-disc--add)');
  } else {
    await page.click(`.cd-nav .cd-disc[data-view="${t.view}"]`);
  }
  await page.waitForTimeout(700);
  if (theme === 'dark') await setTheme(page, 'dark');
  const name = t.view === 'system' ? 'system' : String(t.view).replace(/[^\w.-]/g, '_');
  const file = join(out, `${name}${theme === 'dark' ? '-dark' : ''}${tag ? '-' + tag : ''}.png`);
  await page.screenshot({ path: file, fullPage: full });
  console.log('✓', file.replace(ROOT + '/', ''));
}

if (errors.length) {
  console.log('\n--- page errors ---');
  for (const e of [...new Set(errors)].slice(0, 12)) console.log(' ', e);
  writeFileSync(join(out, 'errors.txt'), [...new Set(errors)].join('\n'));
}

await browser.close();
