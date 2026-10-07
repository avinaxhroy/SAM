#!/usr/bin/env node
/* Capture a panel at a tall viewport.
   The app has ONE scroll container (`design.md` §10: `.cd-scroller`, not the
   page), so `fullPage: true` in a normal shot returns the window and nothing
   else. Growing the window is the only way to photograph a whole screen.

   node tools/shot-tall.mjs --view today.screen [--theme dark] [--h 2400] [--tag x] */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { mkdirSync } from 'node:fs';

function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const port = arg('--port', '4399');
const view = arg('--view', 'today.screen');
const theme = arg('--theme', 'light');
const W = Number(arg('--w', 1180));
const H = Number(arg('--h', 2400));
const tag = arg('--tag', '');
const out = arg('--out', '.captures/after');
mkdirSync(out, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 160)); });

await page.goto(`http://localhost:${port}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1200);
if (theme === 'dark') {
  // The plan pins appearance in content/appearance.json. Use the rail's theme
  // disc to trigger applyTheme() directly instead of writing data-theme to the
  // root element, which is overwritten on the next read.
  const before = await page.getAttribute('html', 'data-theme');
  await page.click('.cd-disc--theme');
  await page.waitForTimeout(400);
  const after = await page.getAttribute('html', 'data-theme');
  if (after !== 'dark') throw new Error(`theme did not switch: ${before} → ${after}`);
  console.log(`   theme ${before} → ${after}`);
}
await page.click(`.cd-nav .cd-disc[data-view="${view}"]`);
await page.waitForTimeout(900);
const name = view.replace(/[^\w.-]/g, '_');
const file = `${out}/${name}${theme === 'dark' ? '-dark' : ''}${tag ? '-' + tag : ''}-tall.png`;
await page.screenshot({ path: file });
console.log(`✓ ${file}${errors.length ? '  ERRORS: ' + errors.join(' | ') : ''}`);
await browser.close();
