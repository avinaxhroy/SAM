#!/usr/bin/env node
/* Measure the arriving geometry of the two panels in real Chromium, so the
   density numbers are measured rather than assumed.
   node tools/measure-panels.mjs [--port 4399] [--view plan.screen|today.screen] */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';

function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const port = arg('--port', '4399');
const view = arg('--view', 'plan.screen');

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1180, height: 780 } });
await page.goto(`http://localhost:${port}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1200);
await page.click(`.cd-nav .cd-disc[data-view="${view}"]`);
await page.waitForTimeout(900);

const out = await page.evaluate(() => {
  const box = (sel) => [...document.querySelectorAll(sel)].slice(0, 4).map((n) => {
    const r = n.getBoundingClientRect();
    const cs = getComputedStyle(n);
    return {
      cls: n.className.toString().slice(0, 70),
      h: Math.round(r.height),
      w: Math.round(r.width),
      minH: cs.minHeight,
      pad: `${cs.paddingTop} ${cs.paddingBottom}`,
      align: cs.alignItems,
      box: cs.boxSizing,
    };
  });
  return {
    row: box('.cd-coll__row'),
    band: box('.tw-band'),
    week: box('.tw-week'),
    bandBody: box('.tw-band .cd-coll__body'),
    bandRight: box('.tw-band__right'),
    bandTitle: box('.tw-band .cd-coll__title'),
    bandMeta: box('.tw-band .cd-rowmeta'),
    bandMeter: box('.tw-band .cd-meter'),
    bandGrip: box('.tw-band .cd-round, .tw-band .tw-grip'),
    card: box('.cd-card'),
    focus: box('.cd-focus'),
    dial: box('.tw-dial'),
    empty: box('.cd-empty'),
    canvas: box('.cd-canvas'),
  };
});
console.log(JSON.stringify(out, null, 1));
await browser.close();
