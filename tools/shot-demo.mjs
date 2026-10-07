#!/usr/bin/env node
/* Capture design/demo's routes — the acceptance floor, shot for side-by-side. */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const out = join(ROOT, arg('--out', '.captures/demo'));
const theme = arg('--theme', 'light');
const W = Number(arg('--w', 1180));
const H = Number(arg('--h', 780));
const only = arg('--only', null);
if (!existsSync(out)) mkdirSync(out, { recursive: true });

const ROUTES = ['today', 'plan', 'courses', 'course', 'review', 'progress', 'system'];
const list = only ? [only] : ROUTES;

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
const page = await ctx.newPage();
page.on('pageerror', (e) => console.log('  [pageerror]', String(e).slice(0, 200)));

for (const r of list) {
  await page.goto('file://' + resolve(ROOT, 'design/demo/index.html') + '#/' + r, { waitUntil: 'load' });
  await page.waitForTimeout(700);
  if (theme === 'dark') {
    await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
    await page.waitForTimeout(200);
  }
  const file = join(out, `${r}${theme === 'dark' ? '-dark' : ''}.png`);
  await page.screenshot({ path: file });
  console.log('✓', file.replace(ROOT + '/', ''));
}
await browser.close();
