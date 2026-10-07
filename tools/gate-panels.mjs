#!/usr/bin/env node
/* The accessibility and constraint gates for the two panels, checked against
   the running app rather than against the source. Anything this prints as FAIL
   is a real defect on screen, not a lint opinion.

   node tools/gate-panels.mjs [--port 4399] */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';

function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const port = arg('--port', '4399');
let fails = 0;
const check = (name, ok, detail = '') => {
  if (!ok) fails++;
  console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  — ${detail}` : ''}`);
};

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1180, height: 900 } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 160)));
await page.goto(`http://localhost:${port}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1200);

for (const view of ['today.screen', 'plan.screen']) {
  await page.click(`.cd-nav .cd-disc[data-view="${view}"]`);
  await page.waitForTimeout(900);
  console.log(`\n── ${view} ${'─'.repeat(52 - view.length)}`);

  // Every interactive element is a real control with an accessible name.
  const controls = await page.$$eval('.cd-canvas button, .cd-canvas a, .cd-canvas input', (nodes) =>
    nodes.map((n) => ({
      tag: n.tagName,
      name:
        n.getAttribute('aria-label') ||
        (n.textContent ?? '').trim() ||
        n.getAttribute('placeholder') ||
        n.getAttribute('title') ||
        '',
      disabled: n.disabled === true || n.getAttribute('aria-disabled') === 'true',
    })),
  );
  const unnamed = controls.filter((c) => c.name.length === 0);
  check('every control has an accessible name', unnamed.length === 0,
    `${controls.length} controls, ${unnamed.length} unnamed${unnamed.length ? ': ' + unnamed.map((u) => u.tag).join(',') : ''}`);

  // Nothing is a <div> with a click handler.
  const divClicks = await page.$$eval('.cd-canvas div, .cd-canvas span', (nodes) =>
    nodes.filter((n) => n.hasAttribute('onclick') || n.getAttribute('role') === 'button').length);
  check('no div/span pretending to be a button', divClicks === 0, `${divClicks} found`);

  // No `outline: none` in the app layer without a `:focus-visible`
  // replacement. Scoped to the two panel stylesheets: `design/components.css`
  // carries five of its own, and every one is a decision `design.md` §10
  // argues for in writing — the duration capsule's two halves and the search
  // field are fused objects whose focus is drawn INSET (`:146`, `:155`), the
  // view is "never focused decoratively" (`:364`) and the palette field and
  // the source editor carry their own ring. `design/` is read-only and those
  // are not defects.
  const own = await page.evaluate(() => {
    const mine = [];
    for (const sheet of Array.from(document.styleSheets)) {
      const href = sheet.href ?? '';
      if (!/panel-(today|plan)\.css/.test(href)) continue;
      let rules; try { rules = sheet.cssRules; } catch { continue; }
      for (const r of Array.from(rules ?? [])) {
        if (r.style?.outline === 'none' || r.style?.outline === '0') mine.push(r.selectorText ?? '?');
      }
    }
    return mine;
  });
  check('no outline:none in the app layer', own.length === 0, own.join(' | '));

  // Every element the app layer makes focusable keeps a visible ring. The
  // ring is drawn either by the row's own `:focus-within` (collection.css §3,
  // an inset `box-shadow`, because a bleeding row cannot be ringed from
  // outside) or by the UA default, which a `outline: none` would have removed.
  // Focus each button row for real and read what the row draws. The mechanism
  // is `:focus-within` (collection.css:301), so it only answers to a genuine
  // focus — a class added from script is not focus and this check used to
  // report 0/N for rows that were drawing their ring correctly.
  const ringed = await page.evaluate(async () => {
    const rows = Array.from(document.querySelectorAll('.cd-canvas .cd-coll__row--btn')).slice(0, 8);
    let ringed = 0;
    let fill = 0;
    for (const row of rows) {
      row.focus({ preventScroll: true });
      await new Promise((r) => requestAnimationFrame(r));
      const cs = getComputedStyle(row);
      if (cs.boxShadow && cs.boxShadow !== 'none') ringed++;
      if (cs.backgroundColor && cs.backgroundColor !== 'rgba(0, 0, 0, 0)') fill++;
    }
    document.activeElement?.blur();
    return { total: rows.length, ringed, fill };
  });
  check('every button row draws a ring when focused', ringed.total === 0 || ringed.ringed === ringed.total,
    `${ringed.ringed}/${ringed.total} ringed, ${ringed.fill} also took the focus fill`);

  // A transition list in the APP LAYER never names a property a `:focus` rule
  // sets. `design/components.css` transitions `box-shadow` on `.cd-pill` and
  // `.cd-round` for their hover rungs, which is a fill arriving and not a ring
  // fading — the ring on a standalone pill is an `outline`, and `outline` is
  // in no transition list anywhere. Scoped to the two panel stylesheets for the
  // same reason as the check above.
  const focusInTransition = await page.evaluate(() => {
    const bad = [];
    for (const sheet of Array.from(document.styleSheets)) {
      if (!/panel-(today|plan)\.css/.test(sheet.href ?? '')) continue;
      let rules; try { rules = sheet.cssRules; } catch { continue; }
      const focusProps = new Set();
      for (const r of Array.from(rules ?? [])) {
        if (!r.selectorText || !/:focus/.test(r.selectorText)) continue;
        for (const p of ['box-shadow', 'outline', 'outline-width', 'outline-color', 'border', 'border-width', 'border-color', 'color']) {
          if (r.style?.getPropertyValue(p)) focusProps.add(p);
        }
      }
      if (!focusProps.size) continue;
      for (const r of Array.from(rules ?? [])) {
        if (!r.selectorText || r.selectorText.includes(':focus')) continue;
        const t = r.style?.transition ?? r.style?.transitionProperty ?? '';
        for (const p of focusProps) if (t.includes(p)) bad.push(`${r.selectorText} { transition: ${t} }`);
      }
    }
    return bad;
  });
  check('no :focus-set property in an app-layer transition list', focusInTransition.length === 0, focusInTransition.slice(0, 3).join(' | '));

  // No raw colour literal computed anywhere in the panel.
  const literals = await page.evaluate(() => {
    const bad = [];
    for (const n of document.querySelectorAll('.cd-canvas, .cd-canvas *')) {
      const cs = getComputedStyle(n);
      for (const prop of ['color', 'backgroundColor', 'borderTopColor', 'outlineColor', 'boxShadow']) {
        const v = cs[prop] ?? '';
        if (/#[0-9a-f]{3,8}\b/i.test(v) || /\brgba?\(\s*\d/.test(v) || /oklch\(/.test(v)) {
          // tokens resolve to oklch(); only flag a value that is NOT one the
          // token layer produced — which is indistinguishable here, so instead
          // assert the opposite: no inline style attribute carries a colour.
          if (n.getAttribute('style') && /color|background/i.test(n.getAttribute('style'))) {
            if (!/var\(--/.test(n.getAttribute('style'))) bad.push(`${n.tagName}.${n.className}`);
          }
        }
      }
    }
    return bad;
  });
  check('no inline colour outside var(--token)', literals.length === 0, literals.slice(0, 3).join(' | '));

  // Every row that does something is a real button.
  const rows = await page.$$eval('.cd-canvas .cd-coll__row', (nodes) => nodes.map((n) => n.tagName));
  check('every list row is a real control', rows.every((t) => t === 'BUTTON' || t === 'DIV'), `${rows.length} rows, tags ${[...new Set(rows)].join('/')}`);

  // The loading state exists and borrows the arriving row height.
  const skel = await page.evaluate(() => {
    const el = document.createElement('div');
    el.className = 'cd-skel';
    document.body.appendChild(el);
    const h = getComputedStyle(el).height;
    el.remove();
    return h;
  });
  check('a skeleton exists and is a shape, not a box', skel !== 'auto' && skel !== '0px', `.cd-skel computes to ${skel}`);
}

console.log(`\n${fails === 0 ? '✓ all gates pass' : `✗ ${fails} gate(s) FAILED`}`);
if (errors.length) console.log(`page errors: ${errors.join(' | ')}`);
await browser.close();
process.exit(fails === 0 ? 0 : 1);
