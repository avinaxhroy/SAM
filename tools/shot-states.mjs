#!/usr/bin/env node
/* Exercise the two panels' states in real Chromium and photograph each one, so
   "every state exists" is a capture rather than a claim.

   node tools/shot-states.mjs [--port 4399] */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { mkdirSync } from 'node:fs';

function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const port = arg('--port', '4399');
const out = arg('--out', '.captures/after');
mkdirSync(out, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1180, height: 820 }, deviceScaleFactor: 1 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 160)); });
await page.goto(`http://localhost:${port}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1200);

const shot = async (name) => { await page.screenshot({ path: `${out}/state-${name}.png` }); console.log(`✓ state-${name}.png`); };

// ── PLAN · a collapsed band opens ────────────────────────────────────────────
await page.click('.cd-nav .cd-disc[data-view="plan.screen"]');
await page.waitForTimeout(800);
const before = await page.$$eval('.tw-week', (n) => n.length);
await page.click('.tw-band[data-band="w.jee.03"]');
await page.waitForTimeout(400);
const after = await page.$$eval('.tw-week', (n) => n.length);
console.log(`   plan band expand: ${before} → ${after} work rows (${after > before ? 'opens' : 'NO CHANGE'})`);
await shot('plan-band-open');

// ── PLAN · keyboard: a band and a work row are both focusable and named ─────
const tabbable = await page.$$eval('.tw-band, .tw-week, .tw-foot', (nodes) => ({
  bands: nodes.filter((n) => n.classList.contains('tw-band')).length,
  work: nodes.filter((n) => n.classList.contains('tw-week')).length,
  tags: [...new Set(nodes.map((n) => n.tagName))],
  unnamed: nodes.filter((n) => !n.textContent.trim()).length,
  notAButton: nodes.filter((n) => n.tagName !== 'BUTTON').length,
}));
console.log(`   plan rows: ${tabbable.bands} bands, ${tabbable.work} work rows, tags ${tabbable.tags}, ${tabbable.unnamed} unnamed, ${tabbable.notAButton} not <button>`);
await page.focus('.tw-week');
await page.waitForTimeout(200);
const ring = await page.$eval('.tw-week', (n) => {
  const cs = getComputedStyle(n);
  return { shadow: cs.boxShadow, bg: cs.backgroundColor };
});
console.log(`   plan row focus-visible: box-shadow ${ring.shadow.slice(0, 60)}… bg ${ring.bg}`);
await shot('plan-focus');

// ── TODAY · a queue row's hover actions arrive ───────────────────────────────
await page.click('.cd-nav .cd-disc[data-view="today.screen"]');
await page.waitForTimeout(900);
const row = await page.$('#today-queue .tw-coll__row');
await row.hover();
await page.waitForTimeout(400);
const acts = await page.$eval('#today-queue .tw-coll__row .tw-acts', (n) => getComputedStyle(n).opacity);
const actsN = await page.$$eval('#today-queue .tw-coll__row:first-of-type .tw-acts button', (n) => n.length);
console.log(`   today row hover: acts opacity ${acts}, ${actsN} action buttons revealed`);
await shot('today-row-hover');

// ── TODAY · the "and N more" row is a real button that reveals ──────────────
const rowsBefore = await page.$$eval('#today-queue .tw-coll__row', (n) => n.length);
await page.click('#today-queue .tw-more');
await page.waitForTimeout(400);
const rowsAfter = await page.$$eval('#today-queue .tw-coll__row', (n) => n.length);
const label = await page.$eval('#today-queue .tw-more .cd-coll__body', (n) => n.textContent.trim());
console.log(`   today "show the other": ${rowsBefore} → ${rowsAfter} rows, label now "${label}"`);
await shot('today-more-open');

// ── exactly one solid-ink object per screen (§6 rule 4) ────────────────────
// The test is against the RESOLVED `--ink`, not a lightness band. A band
// catches `--card` (L98.6) and `--well-2` (L90.6) along with ink (L24), which
// is how a detector reports 41 inked objects on a screen with one. The token
// is resolved by asking the browser what `var(--ink)` computes to, then every
// element whose own background is that value.
const inkScan = () => page.evaluate(() => {
  const probe = document.createElement('div');
  probe.style.cssText = 'position:absolute;visibility:hidden;background:var(--ink)';
  document.body.appendChild(probe);
  const ink = getComputedStyle(probe).backgroundColor;
  probe.remove();
  const hits = [];
  for (const n of document.querySelectorAll('.cd-canvas *')) {
    if (getComputedStyle(n).backgroundColor !== ink) continue;
    const r = n.getBoundingClientRect();
    if (r.width < 8 || r.height < 8) continue;
    const cls = typeof n.className === 'string' ? n.className : n.getAttribute('class') ?? '';
    hits.push(`${n.tagName.toLowerCase()}${cls ? '.' + cls.split(' ').slice(0, 2).join('.') : ''} ${Math.round(r.width)}x${Math.round(r.height)}`);
  }
  return { ink, hits };
});

const todayInk = await inkScan();
console.log(`\n   TODAY  --ink resolves to ${todayInk.ink} → ${todayInk.hits.length} solid-ink object(s):`);
for (const h of todayInk.hits) console.log(`     · ${h}`);

await page.click('.cd-nav .cd-disc[data-view="plan.screen"]');
await page.waitForTimeout(700);
const planInk = await inkScan();
console.log(`   PLAN   --ink resolves to ${planInk.ink} → ${planInk.hits.length} solid-ink object(s):`);
for (const h of planInk.hits) console.log(`     · ${h}`);

console.log(errors.length ? `\n   PAGE ERRORS: ${errors.join(' | ')}` : '\n   no page errors');
await browser.close();
