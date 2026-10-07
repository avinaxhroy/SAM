#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE WHOLE GATE SUITE, IN ONE RUN  (dev only)

   Every instrument the project already owns, driven over one real plan in real
   Chromium, and the exit code of the whole run is the worst of any gate. It
   exists because the gates were being run one at a time by hand across five
   concurrent sessions and one of them quietly went stale: `tools/shot.mjs
   --theme dark` produced *light* captures for a whole day because
   `applyAppearance` writes the resolved theme as inline custom properties, which
   outrank the `[data-theme]` block. A gate nobody re-ran is not a gate.

     node tools/gates.mjs [--plan /tmp/samlive] [--port 4401] [--out .captures/gates]

   The browser passes drive a **running** harness server
   (`node tools/web-ipc.mjs --plan <the same plan> --port <the same port>`);
   the run stops before the build gate when nothing answers on the port, or
   when what answers is on a different plan.

   Gates, in the order a failure should stop you:
     1 · build            the frontend compiles
     2 · tsc              it typechecks
     3 · tokenlint        no literal colour, radius or duration outside the register
     4 · presets          the room's own copy of what the bundle ships
                           is the bundle's, not a transcription of it
     5 · comment balance  no unterminated /* in a stylesheet  (a swallowed rule
                           renders as *no* rule and no gate notices)
     6 · census           zero schema vocabulary in the rendered page
     7 · a11y             names, focus, dialogs, contrast, non-colour cues
     8 · parity           every write command has a designed control AND a
                           file/terminal twin; no command is palette-only
     9 · one dark object  at most one solid-ink object per screen
    10 · both themes      the same states render in light and dark — every
                           designed screen, every record table, and the
                           composer's own seven (edit mode, catalog, design
                           switcher, frame menu, empty screen, one component
                           added, the composed screen in view mode)
    11 · no page errors
   ══════════════════════════════════════════════════════════════════════════ */
import { execFileSync } from 'node:child_process';
import { mkdirSync, existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const PLAN = arg('--plan', '/tmp/samlive');
const PORT = arg('--port', '4401');
const OUT = join(ROOT, arg('--out', '.captures/gates'));
if (!existsSync(OUT)) mkdirSync(OUT, { recursive: true });

const results = [];
let worst = 0;
function gate(name, ok, detail) {
  results.push({ name, ok, detail });
  if (!ok) worst = 1;
  console.log(`${ok ? '  ok  ' : '  FAIL'}  ${name}${detail ? `  — ${detail}` : ''}`);
}
function sh(cmd, args, opts = {}) {
  try { return { ok: true, out: execFileSync(cmd, args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...opts }) }; }
  catch (error) { return { ok: false, out: `${error.stdout ?? ''}${error.stderr ?? ''}` }; }
}

console.log(`\nSAM · GATES   plan ${PLAN} · port ${PORT}\n`);

/* 0 · the harness server, on the run's own plan.

   The browser passes measure **the plan the server is on**; the CLI gates and
   the composer cleanup measure `--plan`. When a `web-ipc` from an earlier run
   still holds the port, those are two different plans and the run judges one
   plan's screens against another's fixtures — which is how a stale
   `composer-light.all` in a forgotten plan made the composer gates fail three
   runs in a row while `--plan` was clean, the census counting the engine's own
   refusal ("a view named … already exists") as schema vocabulary. `/shell_info`
   is the server's own answer to "which plan am I on", so it is the honest
   check; a run pointed at the wrong one stops here, not four minutes later. */
{
  const served = await fetch(`http://localhost:${PORT}/shell_info`)
    .then((response) => response.json())
    .catch(() => null);
  if (!served) {
    console.error(
      `gates: nothing answers on http://localhost:${PORT} — the browser passes need the harness server:\n` +
      `  node tools/web-ipc.mjs --plan ${PLAN} --port ${PORT}`,
    );
    process.exit(1);
  }
  if (resolve(served.plan ?? '') !== resolve(PLAN)) {
    console.error(
      `gates: the server on :${PORT} is on ${served.plan}, but this run says ${PLAN}.\n` +
      `  The browser passes would measure ${served.plan} while the CLI gates measure ${PLAN}.\n` +
      `  Stop the stale server, or run again with --plan ${served.plan}.`,
    );
    process.exit(1);
  }
}

/* 1 · build */
// `2>&1`: vite prints its warnings on stderr, and `execFileSync` on success
// returns stdout alone — the count read `1` while the build printed five.
{
  const r = sh('sh', ['-c', 'pnpm -C ui build 2>&1']);
  const warnings = (r.out.match(/vite-plugin-svelte\]/g) ?? []).length;
  gate('build', r.ok, r.ok ? `compiled, ${warnings} svelte warning(s)` : r.out.split('\n').filter((l) => l.includes('error')).slice(0, 2).join(' '));
}

/* 2 · tsc */
{
  const r = sh('npx', ['tsc', '--noEmit', '-p', 'tsconfig.json'], { cwd: join(ROOT, 'ui') });
  gate('tsc --noEmit', r.ok, r.ok ? 'clean' : r.out.split('\n').slice(0, 3).join(' '));
}

/* 3 · tokenlint */
{
  const r = sh('node', ['tools/tokenlint.mjs']);
  const line = r.out.trim().split('\n').pop();
  gate('tokenlint', r.ok, line);
}

/* 4 · presets — the room reads the bundled plans' own sentences and
   their own size; the copy under `ui/src/shell/presets.ts` is generated, and a
   start the bundle no longer ships (or a new one it does) fails here rather
   than on the screen. */
{
  const r = sh('node', ['tools/presets.mjs', '--check']);
  gate('presets in sync', r.ok, r.out.trim().split('\n').pop());
}

/* 5 · comment balance — verifies stylesheets have balanced comment delimiters. */
{
  const files = [
    ...readdirSync(join(ROOT, 'ui/src/styles')).filter((f) => f.endsWith('.css')).map((f) => join(ROOT, 'ui/src/styles', f)),
    join(ROOT, 'ui/src/shell/chrome.css'),
  ];
  const broken = [];
  for (const file of files) {
    const text = readFileSync(file, 'utf8');
    let i = 0;
    const open = [];
    while (i < text.length) {
      // A slash-star preceded by an asterisk is prose ending a glob, not an
      // opener, so a comment body that mentions a jsonl file cannot be read as
      // the start of a nested comment.
      if (text.startsWith('/*', i) && text[i - 1] !== '*') { open.push(i); i += 2; continue; }
      if (text.startsWith('*/', i)) { if (!open.length) { broken.push(`${file}: stray */`); break; } open.pop(); i += 2; continue; }
      i += 1;
    }
    for (const at of open) broken.push(`${file}: unterminated /* at line ${text.slice(0, at).split('\n').length}`);
  }
  gate('comment balance', broken.length === 0, broken.length ? broken.join('; ') : `${files.length} stylesheets balanced`);
}

/** The first control that opens a **row** menu, or null when the state has no table. */
function p_firstRowMenu(page) {
  for (const selector of ['.cd-row [data-rowmenu-trigger]', '.cd-rowmenu > button', '.cd-row .cd-iconbtn', '.cd-row .cd-cell--edit']) {
    const found = page.locator(selector).first();
    if (found.count()) return found;
  }
  return null;
}

/* ── the browser half ────────────────────────────────────────────────────── */
const STATES = [
  ['today', 'today.screen'], ['plan', 'plan.screen'], ['subjects', 'subjects.screen'],
  ['practice', 'practice.screen'], ['mocks', 'mocks.screen'], ['reviews', 'reviews.screen'],
  ['progress', 'progress.screen'], ['library', 'library.screen'], ['notes', 'notes.screen'],
  ['system', null], ['settings', null],
  /* The rail only carries destinations that name a designed screen, so none of
     the eleven above renders a RECORD TABLE — and the table is where a third of
     the app's capabilities live: the column menu, the row's context menu, the
     paste door, the copy-as-JSON doors. Parity run over the panels alone reports
     every one of them as "nothing rendered it", which reads as a broken app and
     is really a blind spot in the gate.

     practice.sets is a view of type problemset with no panel, so it renders
     through the block layer and the table with it. It is chosen over
     today.topics because a problemset table carries a text column, a number
     column and a select column: the widest set of cell controls the table owns.
     It has no rail entry, so the harness drives it through the one command that
     selects a destination — the same door the app's own console would use. */
  ['table', 'practice.sets'],
];

/**
 * One table per record kind the plan actually has.
 *
 * The parity fixture diffs *every* declared write command against what the page
 * rendered, and most of those commands are per-kind: topic.paste, note.new,
 * resource.paste, milestone.new, unit.new, assessment.paste. A gate that visits
 * one table therefore reports a dozen of them as "nothing rendered it" — a
 * finding that reads as a broken app and is in fact a blind spot in the gate,
 * which is the worst kind of finding, because it teaches people to ignore it.
 *
 * The kinds and their views are read out of the plan's own views.json rather than
 * typed, so a renamed view or a new kind needs no edit here. A kind with no view
 * is skipped: there is nothing to render, and the engine's own `uicheck` already
 * covers the command's existence.
 */
function tablesFor(planDir) {
  const file = join(planDir, 'content/views.json');
  if (!existsSync(file)) return [];
  const views = JSON.parse(readFileSync(file, 'utf8')).views ?? {};
  // A **table** view per kind, not merely the first view of that type: a kind's
  // calendar, tile grid or list draws its records too, and none of them carries
  // the table's column menu — which is where the paste door, the retype door and
  // the copy-as-JSON doors live. Picking the first view and then reporting those
  // commands as unrendered is the gate measuring the wrong surface, so the
  // table is chosen explicitly and a kind without one falls back to its first.
  // Views that declare a `panel` are the **designed screens**, already visited by
  // name; selecting one here renders that panel rather than the block layer, and
  // a panel has no column menu and no paste door — so five kinds were reporting
  // "no paste control" for a screen that was never a table in the first place.
  // Only panel-less views are candidates, and among those a declared `table`
  // layout wins over a bare list.
  const byType = new Map();
  for (const [name, def] of Object.entries(views)) {
    if (!def.type || def.panel) continue;
    const rank = def.layout === 'table' ? 0 : def.layout === undefined ? 1 : 2;
    const held = byType.get(def.type);
    if (!held || rank < held.rank) byType.set(def.type, { name, rank });
  }
  return [...byType].map(([type, { name }]) => ({ type, view: name }));
}
for (const { type, view } of tablesFor(PLAN)) STATES.push([`table-${type}`, view]);

/* ── the composer's own states (COMPOSER §5) ──────────────────────────────
   The layer the student composes *with* is drawn by `ui/src/composer/**`, and
   none of the states above puts a pixel of it on screen: no destination of the
   rail enters edit mode, opens the catalog, opens the design switcher, or shows
   an empty screen. Every instrument below measures the rendered page, so the
   whole layer was shipping unmeasured — and the parity judge, which reads what
   the page rendered, classified `view.setComponents` as an OS menu item because
   no capture ever carried a control for it.

   Seven states, each driven through the app's **own** controls — the titlebar
   pencil (COMPOSER §3.3), the edit bar's *Add component*, a frame's name chip
   and the ✕ and ‹ › beside it (§4.3), the rail's Add sheet in its *A screen*
   mode (§4.7), the empty state's one action and the bar's *Done* (§4.2).
   Nothing here pokes `app.screenEditing` or writes a view through the console:
   a gate that sets the state it measures proves only that the state can be set,
   and every one of these doors is also a control the app promises. */
const COMPOSER_STATES = [
  'composer-edit', //     the pencil → edit mode on Today: the bar and the frames
  'composer-inserter', // the catalog open over the screen
  'composer-design', //   a frame's name chip → that component's three designs
  'composer-actions', //  a frame's acts on the surface: ✕ removes, ‹ › switch design
  'composer-empty', //    rail Add → *A screen* → the empty state it opens on
  'composer-add', //      the empty state's one action → one component added
  'composer-view', //     Done → the student's arrangement, drawn
];
for (const name of COMPOSER_STATES) STATES.push([name, null]);

// The view ids are read from the running page rather than typed, for the reason
// `tools/seed-plan.mjs` reads its view names out of the plan: a renamed view
// would otherwise make this harness wait out a timeout per state and report a
// failure that is really a stale list in a test file.
let views = [];

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1180, height: 780 } });
const page = await ctx.newPage();
const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e).slice(0, 200)));
page.on('console', (m) => { if (m.type() === 'error') pageErrors.push(m.text().slice(0, 200)); });

await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1500);
views = await page.$$eval('.cd-nav .cd-disc[data-view]', (nodes) => nodes.map((n) => n.getAttribute('data-view')));

/** Dark is reached through the rail's own disc — never by setting `data-theme`. */
async function setTheme(want) {
  for (let i = 0; i < 3; i += 1) {
    const lightness = await page.evaluate(() => {
      const raw = getComputedStyle(document.documentElement).getPropertyValue('--card').trim();
      const m = raw.match(/oklch\(([\d.]+)%/);
      return m ? Number(m[1]) : null;
    });
    if (lightness !== null && (lightness < 60) === (want === 'dark')) return true;
    await page.locator('.cd-disc--theme').click({ force: true });
    await page.waitForTimeout(500);
  }
  return false;
}

async function goTo(name, view) {
  // The composer states have their own driver, and it comes first: they are
  // reached by pressing controls (a pencil, a chip, a frame's ✕, the rail's Add
  // sheet), not by naming a destination.
  if (name.startsWith('composer-')) {
    try {
      await goComposer(name);
    } catch (error) {
      // A state the driver could not reach is reported as a failing gate of its
      // own rather than killing the run. The instruments below then measure
      // whatever is on screen, the parity judge reports the controls that did
      // not render, and the failure is named — a silent skip would be a green
      // gate over pixels nobody looked at.
      composerFailures.push(`${name}-${themeNow}: ${String(error?.message ?? error).split('\n')[0].slice(0, 160)}`);
    }
    return;
  }
  // The Preferences window is a **separate document** with no rail, so anything
  // visited after it has to go back to the main one first. Without this the next
  // rail click waits out a timeout on a rail that is not there, and the gate
  // reports a navigation failure for a state that is perfectly fine.
  if (name !== 'settings' && !(await page.locator('.cd-nav .cd-disc').count())) {
    await page.goto(`http://localhost:${PORT}/?back=${Date.now()}`, { waitUntil: 'domcontentloaded' });
    await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
    views = await page.$$eval('.cd-nav .cd-disc[data-view]', (nodes) => nodes.map((n) => n.getAttribute('data-view')));
    await page.waitForTimeout(600);
  }
  if (name === 'settings') {
    // A same-document hash does not reload, and the app does not listen for
    // `hashchange`, so navigating to the URL it is already on leaves the previous
    // state standing and the dump is taken from whatever was there before. The
    // cache-buster therefore goes in the **search**, not in the hash — and that
    // detail is load-bearing: `main.ts` asks `location.hash === '#preferences'`,
    // an exact comparison, so a query appended to the hash makes the app render
    // the main window instead of Settings and the gate reports four settings
    // rows as "no rendered row carries data-settings-key" for a screen that was
    // never on screen.
    await page.goto(`http://localhost:${PORT}/?pass=${Date.now()}#preferences`, { waitUntil: 'domcontentloaded' });
  } else if (name === 'system') {
    await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
    await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
    // The rail paints before the plan is read, and the window's own `load()`
    // resets the machine when it lands (`shedPlanState`), so a disc clicked in
    // between is silently un-opened — this suite's previous `system` capture is
    // Today for exactly that reason. A *destination* disc exists only once the
    // plan has been read, so waiting for one is waiting for `load()` to have
    // finished, and the click that follows sticks.
    await page.waitForSelector('.cd-nav .cd-disc[data-view]', { timeout: 20000 });
    await page.locator('.cd-disc[data-command="app.openSystem"]').click();
    await page.waitForSelector('.sys-groups', { timeout: 20000 });
    // Then the Rules place, with its first figure's own editor open. A figure's
    // fields (`metric.set`) are written in the row body, and that body only
    // renders once the row asks for it — a dump that stopped at the index would
    // report a declared, writable command as "nothing rendered it", and the
    // parity judge would then be reading this walk's missed click, not the app.
    await page.locator('.sys-groups button', { hasText: 'Rules' }).first().click();
    await page.waitForSelector('[data-place="rules"] .cd-coll__row .sys-open', { timeout: 20000 });
    await page.locator('[data-place="rules"] .cd-coll__row .sys-open').first().click();
  } else if (views.includes(view ?? name)) {
    // `STATES` carries the plan's own view id for each destination, and the list
    // is checked against the rail that actually rendered, so a plan that renamed
    // a view fails here with a clear message rather than a timeout.
    await page.click(`.cd-nav .cd-disc[data-view="${view ?? name}"]`, { timeout: 10000 });
  } else {
    // A view with no rail entry — the generic block layer, and the only place the
    // record table renders. It is driven through the one command that selects a
    // destination, which is the same door the app's own console uses, and the
    // table's own header is what tells us the block layer arrived.
    //
    // The wait is tolerant on purpose. A kind's view does not have to render a
    // TABLE: `course` draws a tile grid and `week` draws cards, so "no table
    // arrived" is a fact about the view rather than a failure. What would be a
    // failure is the gate asserting a shape it has not earned and then reporting
    // every kind that differs from it as a missing control.
    await page.evaluate((id) => globalThis.__SAM_APP__?.run('rail.select', { view: id }), view);
    await page.waitForSelector('.cd-canvas h1, .cd-canvas h2, .cd-empty', { timeout: 10000 });
  }
  // The app is a shell over an IPC bridge, so a fixed 800ms was dumping the page
  // mid-flight: the Settings window's four `data-settings-key` rows all render
  // (verified in the page) and the dump said `[]`, because the dump was taken
  // before they painted. The wait is for the app to be *quiet* rather than for a
  // number of milliseconds — the second one is a guess that fails on a slower
  // machine and quietly judges nothing on this one.
  await page.waitForLoadState('networkidle').catch(() => {});
  await page.waitForTimeout(600);
}

/* ── the composer states' driver ───────────────────────────────────────────
   The reference `goTo` above navigates by picking a destination. The composer
   is not a destination — it is a *mode* on whatever destination is showing — so
   these states are driven by pressing the app's controls in the order a student
   would, and each helper states which contract clause it is exercising. */

/**
 * Close whatever is floating, and (unless the state is the exit itself) leave
 * edit mode — both by pressing Escape, which is the app's own dismissal for a
 * sheet, a menu and the mode, in that order (`Screen.svelte`'s window handler
 * defers to the open floating surface). §11 allows one floating surface at a
 * time, so a state that starts with the previous state's menu still open would
 * measure the wrong thing; a gate that opens a menu and never closes it is a
 * gate that measures its own leftovers.
 */
async function composerSettle(leaveEdit = true) {
  for (let i = 0; i < 5; i += 1) {
    const floating = (await page.locator('.cd-sheet, .cd-menu').count()) > 0;
    const editing = (await page.locator('[data-command="screen.edit"][aria-pressed="true"]').count()) > 0;
    if (!floating && (!editing || !leaveEdit)) return;
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);
  }
}

/** The rail's own disc for a destination — the student's door, not `rail.select`. */
async function composerSelect(view) {
  await page.click(`.cd-nav .cd-disc[data-view="${view}"]`, { timeout: 10000 });
  await page.waitForTimeout(500);
}

/** The titlebar pencil (COMPOSER §3.3): the mode's one door on a designed screen. */
async function composerEnterEdit() {
  if (await page.locator('.cmp-bar').count()) return;
  await page.locator('[data-command="screen.edit"]').click({ timeout: 10000 });
  await page.waitForSelector('.cmp-bar', { timeout: 10000 });
  await page.waitForTimeout(400);
}

async function goComposer(name) {
  // Composer states run last, but a state is a state: recover the main window
  // first for the same reason `goTo` does (Settings is another document, and
  // the rail is what every door below is found through).
  if (!(await page.locator('.cd-nav .cd-disc').count())) {
    await page.goto(`http://localhost:${PORT}/?back=${Date.now()}`, { waitUntil: 'domcontentloaded' });
    await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
    views = await page.$$eval('.cd-nav .cd-disc[data-view]', (nodes) => nodes.map((n) => n.getAttribute('data-view')));
    await page.waitForTimeout(600);
  }
  // The exit state keeps the mode until its own control is pressed: `Done` is
  // the door under test there, so settling must not pre-empt it.
  await composerSettle(name !== 'composer-view');

  if (name === 'composer-edit') {
    await composerSelect('today.screen');
    await composerEnterEdit();
    const frames = await page.locator('[data-frame]').count();
    if (frames === 0) throw new Error('edit mode on Today drew no frame');
    return;
  }

  if (name === 'composer-inserter') {
    await composerSelect('today.screen');
    await composerEnterEdit();
    await page.locator('.cmp-bar [data-opens="inserter"]').click({ timeout: 10000 });
    await page.waitForSelector('.ci-field', { timeout: 10000 });
    await page.waitForTimeout(500);
    return;
  }

  if (name === 'composer-design') {
    // The carousel, pressed (§4.5): `›` steps the design in place — the label
    // moves A→B, the plan revision must not move, and a frame menu must not
    // exist (the chip is a label now; the arrows are the only controls).
    await composerSelect('today.screen');
    await composerEnterEdit();
    const before = await page.evaluate(() => globalThis.__SAM_APP__.revision);
    const label = page.locator('.cmp-frame__name').first();
    const read = async () => (await label.innerText()).replace(/\s+/g, ' ').trim();
    const start = await read();
    if (!/·\s*[ABC]\s*$/.test(start)) throw new Error(`the carousel label reads “${start}” — no design letter`);
    await page.locator('.cmp-step[aria-label^="Next design"]').first().click({ timeout: 10000 });
    await page.waitForTimeout(400);
    const stepped = await read();
    if (stepped === start) throw new Error(`the next arrow did not change the label (“${start}”)`);
    const after = await page.evaluate(() => globalThis.__SAM_APP__.revision);
    if (after !== before) throw new Error('stepping a design wrote the plan');
    if ((await page.locator('.cmp-frame .cd-menu, .cmp-frame .cd-vm').count()) !== 0) {
      throw new Error('a frame still opens a menu');
    }
    await page.waitForTimeout(200);
    return;
  }

  if (name === 'composer-actions') {
    // A frame's acts are on the surface now, so this state's job is to put that
    // chrome on screen and to assert the shape: ✕ on every frame, the two design
    // arrows beside the name chip, and no second menu — a frame that still
    // carried the old ⋯ would be a capture of the previous build, and the state
    // would be green over it.
    await composerSelect('today.screen');
    await composerEnterEdit();
    const frames = await page.locator('[data-frame]').count();
    const removes = await page.locator('.cmp-frame button[aria-label^="Remove "]').count();
    if (frames === 0) throw new Error('edit mode on Today drew no frame');
    if (removes !== frames) throw new Error(`${frames} frames, ${removes} on-surface remove controls`);
    const prev = await page.locator('.cmp-step[aria-label^="Previous design"]').count();
    const next = await page.locator('.cmp-step[aria-label^="Next design"]').count();
    if (prev !== frames || next !== frames) {
      throw new Error(`${frames} frames, ${prev} previous / ${next} next design arrows`);
    }
    if ((await page.locator('.cmp-more').count()) || (await page.locator('.cmp-frame .cd-menu').count())) {
      throw new Error('a frame still carries a second menu');
    }
    await page.waitForTimeout(400);
    return;
  }

  if (name === 'composer-empty') {
    // The rail's Add, in its *A screen* mode (§4.7): a name, an icon from the
    // app's own picker, and the engine's `list.new` — after which the app
    // selects the new screen, enters edit mode and opens the catalog.
    //
    // The screen is created **per theme pass**, and the name says which: the
    // empty state is a state a screen has once, so a run that re-used the light
    // pass's screen in the dark pass would be capturing a screen with a
    // component on it and calling it empty.
    await page.locator('.cd-disc--add').click({ timeout: 10000 });
    await page.waitForSelector('#screen-name', { timeout: 10000 });
    await page.fill('#screen-name', `Composer ${themeNow}`);
    await page.locator('.cmp-iconpick__one').nth(2).click();
    await page.waitForTimeout(200);
    await page.locator('.cd-sheet [data-command="list.new"]').click({ timeout: 10000 });
    await page.waitForSelector('.ci-field', { timeout: 10000 });
    await page.waitForTimeout(800);
    const created = await page.evaluate(() => document.querySelector('.cd-canvas [data-view]')?.getAttribute('data-view') ?? null);
    if (!created) throw new Error('the new screen never became the canvas');
    composedScreen = created;
    // The catalog the app opened is dismissed with the same Escape a student
    // uses, so the state under measure is the empty screen, not the sheet.
    await composerSettle(false);
    if ((await page.locator('.cd-empty').count()) === 0) throw new Error('no empty state rendered');
    return;
  }

  if (name === 'composer-add') {
    if (!composedScreen) throw new Error('no screen was created for this pass');
    await composerSelect(composedScreen);
    // The empty state's one action (§4.1) — the door a screen with nothing on it
    // offers, and the same catalog the bar's pill opens.
    await page.locator('.cd-empty [data-opens="inserter"]').click({ timeout: 10000 });
    await page.waitForSelector('.ci-field', { timeout: 10000 });
    await page.waitForTimeout(400);
    const row = page.locator('.ci-list [data-add]:not([disabled])').first();
    if ((await row.count()) === 0) throw new Error('the catalog offered nothing to add');
    const surface = await row.getAttribute('data-add');
    await row.click({ timeout: 10000 });
    await page.waitForTimeout(1200);
    if ((await page.locator('[data-frame]').count()) === 0) {
      throw new Error(`${surface} was chosen and nothing landed on the screen`);
    }
    return;
  }

  if (name === 'composer-view') {
    if (!composedScreen) throw new Error('no screen was created for this pass');
    await composerSelect(composedScreen);
    // Done is the student's own exit (§4.2) — the pencil's twin, and the half of
    // the mode the harness would otherwise never press.
    const done = page.locator('.cmp-bar button:text-is("Done")');
    if (await done.count()) {
      await done.click({ timeout: 10000 });
      await page.waitForTimeout(700);
    }
    if (await page.locator('.cmp-bar').count()) throw new Error('Done did not leave edit mode');
    if ((await page.locator('.cmp-view > *').count()) === 0) throw new Error('the composed screen drew nothing');
    return;
  }

  throw new Error(`no driver for this state`);
}

/* A run of the suite must be re-runnable. `composer-empty` creates its screens
   through the app's own Add sheet, and a finished pass — or one that died
   before its last capture — leaves `composer-light.all` and `composer-dark.all`
   in the plan. The next pass's `list.new` then refuses with `cli.usage: a view
   named … already exists`, and the four gates that read the composer states
   judge a page that never changed state: census counts the error string, one
   dark object counts the Add sheet's own buttons, no page errors counts the
   400, and `composer states driven` times out waiting for a screen that was
   never made. Cleared here, through the plan's terminal surface — `view.delete`
   removes the view, its destinations and its rail entry — before any browser
   work, so the pass always starts from the same plan. */
for (const stale of ['composer-light.all', 'composer-dark.all']) {
  // stderr is captured, not inherited: "no view named …" is the ordinary
  // answer on a plan that was already clean, and a gate suite whose console
  // prints CLI errors it is ignoring teaches people to ignore CLI errors.
  sh(join(ROOT, 'target/release/sam'), ['view.delete', stale, '--json', '--plan', PLAN], { stdio: ['ignore', 'pipe', 'pipe'] });
}

/* 5, 6, 8, 9, 10 — one pass, both themes, every state. */
const censusReports = [];
const a11yReports = [];
const inkByState = [];
const renderDumps = [];
const themeOk = { light: true, dark: true };
/** The theme pass in flight: the new screen's name is per pass (see `composer-empty`). */
let themeNow = 'light';
/** The screen this pass created, from `composer-empty` through `composer-view`. */
let composedScreen = null;
/** States the driver could not reach — reported as a gate rather than swallowed. */
const composerFailures = [];

for (const theme of ['light', 'dark']) {
  themeNow = theme;
  if (!(await setTheme(theme))) themeOk[theme] = false;
  for (const [name, view] of STATES) {
    await goTo(name, view);
    await page.waitForTimeout(500);
    if (name === 'settings') await setTheme(theme);

    // Wait for the state to actually BE the state before measuring it. Every
    // instrument below reads the rendered page, and a report taken while the
    // *previous* screen is still on it is a report about the wrong screen — which
    // is how a session table came to be judged as "speaks the schema" over a
    // string that was on a panel, and how four settings rows came to be "not
    // rendered" for a screen that had not been opened. The canvas's own identity
    // is the assertion: the table layer stamps `data-view` on what it draws, and
    // the rail's selection is compared against it.
    if (view && views.includes(view)) {
      await page.waitForFunction(
        (id) => document.querySelector('.cd-canvas [data-view]')?.getAttribute('data-view') === id,
        view,
        { timeout: 10000 },
      ).catch(() => {});
    }
    await page.waitForTimeout(350);

    const tag = `${name}-${theme}`;
    await page.screenshot({ path: join(OUT, `${tag}.png`) });

    // Open the menus that carry the schema verbs, so the parity judge sees the
    // surfaces it diffs. A gate that only ever looks at a closed screen is
    // measuring a fraction of the app and calling it complete.
    if (name.startsWith('table')) {
      // The row menu — where `record.move`, `record.advanceStage` and
      // `record.logReview` live — is opened by a button inside a row, not by a
      // context-menu event. It is opened here, on a table, because a table row is
      // the only place the app puts one.
      // The row menu is declared by `rowMenuIds()` (copy-json, copy-path,
      // record.reveal, record.move, record.delete) and it only exists on a
      // **RecordTable** — a kind whose view is a `list` draws a RecordList with no
      // row menu, so the trigger is looked for on the row's own actions and the
      // count is reported when the state had none.
      const trigger = p_firstRowMenu(page);
      if (trigger) {
        await trigger.click({ timeout: 5000 }).catch(() => {});
        await page.waitForTimeout(500);
        const opened = await page.evaluate(() => globalThis.__SAM_RENDER_DUMP__?.() ?? null);
        if (opened) renderDumps.push({ name: `${tag}-rowmenu`, dump: opened });
        await page.screenshot({ path: join(OUT, `${tag}-rowmenu.png`) });
      }
    }

    if (name === 'reviews') {
      // Drive the recall loop by its own keyboard path. `Space` reveals the answer
      // and the four grade buttons appear — and `record.logReview` lives on those
      // buttons, so without this the gate reports the app's core write as having
      // no control. Driving it by keyboard rather than by click is deliberate: the
      // flow's own claim is that a review completes with `Space` and `1`–`4`, and
      // this is the only check in the suite that would notice if it stopped being
      // true.
      await page.keyboard.press('Space').catch(() => {});
      await page.waitForTimeout(600);
      const revealed = await page.locator('.cd-rate button, [data-command="record.logReview"]').count();
      if (revealed) {
        const dump = await page.evaluate(() => globalThis.__SAM_RENDER_DUMP__?.() ?? null);
        if (dump) renderDumps.push({ name: `${tag}-revealed`, dump });
        await page.screenshot({ path: join(OUT, `${tag}-revealed.png`) });
      }
    }

    if (name === 'reviews') {
      // `record.advanceStage` and `record.logReview` are not on the recall card's
      // face — they live in the **queue row's** own menu, so the gate has to open
      // it or it reports two of the app's core writes as unrendered.
      // The row menu is opened by the row's **own** control — a button inside the
      // row, not a context-menu event. A right-click guessed at the trigger and
      // found nothing, which is how `record.move`, `record.advanceStage` and
      // `record.logReview` came to be reported as three writes with no door when
      // all three are in that menu.
      const trigger = page.locator('.cd-rowmenu, [data-row-menu], .cd-cellbox button, .cd-row button').first();
      if (await trigger.count()) {
        await trigger.click({ timeout: 5000 }).catch(() => {});
        await page.waitForTimeout(500);
        const opened = await page.evaluate(() => globalThis.__SAM_RENDER_DUMP__?.() ?? null);
        if (opened) renderDumps.push({ name: `${tag}-rowmenu`, dump: opened });
      }
    }

    if (name === 'reviews') {
      // `record.advanceStage` is the queue row's own next step, and the redesigned
      // queue states it **on the row** — a `data-part="waiting"` row whose single
      // button opens the step's detail, where the write's door lives. The gate
      // clicks the app's own control rather than a class it guessed: the shipped
      // selector (`.cd-row …`) belonged to the pre-redesign table and found
      // nothing here, which is how the command came back as "nothing rendered it".
      const step = page.locator('[data-part="waiting"] button').first();
      if (await step.count()) {
        await step.click({ timeout: 5000 }).catch(() => {});
        await page.waitForTimeout(500);
        const opened = await page.evaluate(() => globalThis.__SAM_RENDER_DUMP__?.() ?? null);
        if (opened) renderDumps.push({ name: `${tag}-step`, dump: opened });
        await page.screenshot({ path: join(OUT, `${tag}-step.png`) });
      }
    }

    if (name.startsWith('table')) {
      const column = page.locator('.cd-collabel').first();
      if (await column.count()) {
        await column.click({ timeout: 5000 }).catch(() => {});
        await page.waitForTimeout(400);
        const opened = await page.evaluate(() => globalThis.__SAM_RENDER_DUMP__?.() ?? null);
        if (opened) renderDumps.push({ name: `${tag}-colmenu`, dump: opened });
        await page.screenshot({ path: join(OUT, `${tag}-colmenu.png`) });
      }
    }

    // Parity's input: the app's own rendered-structure hook, one dump per state,
    // so the judge diffs *this* run's DOM against the registry rather than
    // whatever a previous session left on disk.
    const dump = await page.evaluate(() => globalThis.__SAM_RENDER_DUMP__?.() ?? null);
    if (dump) renderDumps.push({ name: tag, dump });

    // 8 · the one dark object, resolved against --ink and not a lightness band.
    //    Two measured traps lived here: `--ink` declares `oklch(24.0% …)` while
    //    a matching element computes `oklch(0.24 …)`, so the unit mismatch made
    //    the comparison never true and the gate vacuous (58/58 states read 0
    //    objects); and collapsing labels into a Set hid a second object sharing
    //    a first class. Both fixed: units are normalised, and every matching
    //    *element* counts. Chrome is excluded by ancestor, not by class name.
    const ink = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      const parse = (v) => {
        const m = String(v).match(/oklch\(\s*([\d.]+)(%?)/);
        if (!m) return null;
        const n = Number(m[1]);
        return m[2] === '%' ? n / 100 : n;
      };
      const target = parse(root.getPropertyValue('--ink').trim());
      const found = [];
      for (const el of document.querySelectorAll('.cd-pill, .cd-round, .cd-seg__pill, .cd-disc, button, [aria-pressed="true"]')) {
        // `[data-pane]` (the source pane) and `.cd-detailpane` (the record pane)
        // are surfaces of their own, drawn beside the canvas and never covering
        // it — their objects are not the screen's.
        if (el.closest('.cd-nav, .cd-titlebar, .cd-toasts, [data-pane], .cd-detailpane')) continue;
        const bg = parse(getComputedStyle(el).backgroundColor);
        if (bg !== null && target !== null && Math.abs(bg - target) < 0.02 && el.getBoundingClientRect().width > 8) {
          found.push(`${el.tagName.toLowerCase()}.${el.className.split(' ')[0]}`);
        }
      }
      return found;
    });
    inkByState.push({ state: tag, ink });

    // 5 + 6 · the page-side instruments, evaluated in this page
    const scripts = join(ROOT, 'tools');
    // The a11y report's own premise is *with `prefers-reduced-motion: reduce`,
    // nothing animates longer than 20 ms* (`a11y-page.js`'s header, the global
    // rule in `design/base.css`). Without emulating the preference the report
    // measured ordinary motion and judged it against the reduced-motion floor —
    // which passed only while a token bug had every transition computing to 0s
    // (fixed 2026-09-29). Emulate it for the instrument, then restore.
    await page.emulateMedia({ reducedMotion: 'reduce' }).catch(() => {});
    for (const [file, bucket] of [['census-page.js', censusReports], ['a11y-page.js', a11yReports]]) {
      const source = readFileSync(join(scripts, file), 'utf8');
      // The instruments are **exported as the string a driver evaluates**. Each
      // file is a node module that interpolates its rules into a template
      // literal, because the same rule table has to exist on both sides of the
      // boundary. So the module is imported here and its export handed to the
      // page whole.
      //
      // Three wrong ways to do this were tried first and each one produced a
      // *passing* gate, which is why it is written down: passing the file
      // evaluates the `export` and throws; slicing at the first `(` cuts into an
      // object literal and throws; catching either and carrying on judges `{}`,
      // which is 0 findings, which is green. A gate that cannot fail is not a
      // gate, so the report's shape is checked and a missing one throws.
      const module = await import(join(scripts, file));
      const script = Object.values(module).find((value) => typeof value === 'string' && value.includes('(() =>'));
      if (!script) throw new Error(`${file} exports no page script`);
      const report = await page.evaluate(script).catch((e) => ({ error: String(e).slice(0, 200) }));
      if (report === undefined || report === null) throw new Error(`${file} returned no report for ${tag}`);
      if (report.error) throw new Error(`${file} on ${tag}: ${report.error}`);
      if (typeof report.total !== 'number' && typeof report.interactive !== 'number') {
        throw new Error(`${file} on ${tag} returned ${Object.keys(report).join(',')} — not a report`);
      }
      writeFileSync(join(OUT, `${file.replace('.js', '')}-${tag}.json`), JSON.stringify(report, null, 1));
      if (bucket === censusReports) bucket.push({ state: tag, report });
      else bucket.push({ state: tag, report });
    }
    await page.emulateMedia({ reducedMotion: 'no-preference' }).catch(() => {});
  }
}

gate(
  'composer states driven',
  composerFailures.length === 0,
  composerFailures.length
    ? composerFailures.join(' | ')
    : `${COMPOSER_STATES.length} states × 2 themes reached through the app's own controls`,
);
gate('both themes render', themeOk.light && themeOk.dark, `light ${themeOk.light ? 'ok' : 'FAIL'} · dark ${themeOk.dark ? 'ok' : 'FAIL'}`);

/* census */
{
  const offenders = [];
  for (const { state, report } of censusReports) {
    const occurrences = Object.values(report?.counts ?? {}).reduce((sum, n) => sum + n, 0);
    if (occurrences) offenders.push(`${state}:${occurrences} [${Object.entries(report.counts).map(([k, v]) => `${k}×${v}`).join(', ')}]`);
  }
  const total = censusReports.reduce((sum, c) => sum + (c.report?.total ?? 0), 0);
  gate('census (schema vocabulary)', offenders.length === 0,
    offenders.length ? offenders.join(' ') : `0 occurrences over ${censusReports.length} states, ${total} rendered strings`);
}

/* a11y */
{
  const offenders = [];
  for (const { state, report } of a11yReports) {
    // `colourOnly` is an ARRAY of element descriptions in the report and every
    // other tally is a number, so `+` on the two produces NaN — which is falsy,
    // which is "no problems", which is a gate that cannot fail. Counted by kind.
    const lists = ['colourOnly'];
    const tallies = ['withoutName', 'weakName', 'unfocusable', 'contrastFailures',
      'dialogsMissingRole', 'dialogsMissingModal', 'overflow'];
    const counted = { ...Object.fromEntries(lists.map((k) => [k, report?.[k]?.length ?? 0])) };
    for (const k of tallies) counted[k] = report?.[k] ?? 0;
    const bad = Object.values(counted).reduce((sum, n) => sum + (Number.isFinite(n) ? n : 0), 0);
    if (bad) {
      offenders.push(`${state}:${bad} ` + Object.entries(counted).filter(([, n]) => n).map(([k, n]) => `${k} ${n}`).join(', '));
    }
  }
  gate('a11y', offenders.length === 0, offenders.length ? offenders.join(' ') : `${a11yReports.length} states, 0 unnamed / 0 unfocusable / 0 colour-only / 0 contrast failures`);
}

/* one dark object — the rail, the titlebar and the toasts are chrome, and a
   pane is a surface of its own, both excluded in the page above. Two
   pre-existing offenders are *named* rather than silently skipped
   (the log's C1): practice draws one ink `Save attempt` pill per open
   bank form (8 at once on the seeded plan), and reviews reaches 2 only with the
   recall revealed *and* a waiting row's step expanded — the panel's own two
   actions, both ink. Each waiver is one named state and is printed in the gate
   line; a new offender anywhere fails. */
{
  const CHROME = /cd-disc|cd-nav|cd-titlebar|cd-mark/;
  const WAIVED_INK = new Set(['practice-light', 'practice-dark', 'reviews-light', 'reviews-dark']);
  const offenders = [];
  const waived = [];
  for (const { state, ink } of inkByState) {
    if (ink.filter((c) => !CHROME.test(c)).length <= 1) continue;
    (WAIVED_INK.has(state) ? waived : offenders).push(`${state}: ${ink.length} [${ink.slice(0, 3).join(', ')}]`);
  }
  gate('one dark object per screen', offenders.length === 0,
    offenders.length
      ? offenders.join(' ')
      : `${inkByState.length} states at most 1 screen object${waived.length ? ` · waived (pre-existing, carried): ${waived.join(' · ')}` : ''}`);
}

gate('no page errors', pageErrors.length === 0, pageErrors.length ? [...new Set(pageErrors)].slice(0, 3).join(' | ') : 'clean across every state and theme');

await browser.close();

/* 7 · parity, last, because it needs the rendered dumps the pass just wrote. */
{
  // The judge is fed the reports this run just wrote, so it is judging *this*
  // build and not whatever happens to be on disk from a previous session.
  const censusFiles = censusReports.map((c) => join(OUT, `census-page-${c.state}.json`));
  const a11yFiles = a11yReports.map((a) => join(OUT, `a11y-page-${a.state}.json`));
  const census = sh('node', ['tools/uicensus.mjs', '--report', ...censusFiles]);
  gate('census judge', census.ok, (census.out.trim().split('\n').pop() ?? '').slice(0, 120));
  const a11y = sh('node', ['tools/a11ycheck.mjs', '--report', ...a11yFiles]);
  gate('a11y judge', a11y.ok, (a11y.out.trim().split('\n').pop() ?? '').slice(0, 120));
  // One file per state. Pushing them all as a single JSON *array* is the trap
  // `uicheck-diff.mjs` sets: the array arrives as one array-shaped snapshot,
  // every count reads 0, and a green "23 rendered" is a number nobody measured.
  const dumpFiles = renderDumps.map(({ name, dump }) => {
    const file = join(OUT, `render-${name}.json`);
    writeFileSync(file, JSON.stringify(dump));
    return file;
  });
  const r = dumpFiles.length
    ? sh('node', ['tools/uicheck-diff.mjs', '--plan', PLAN, '--rendered', dumpFiles.join(',')])
    : { ok: false, out: 'no render dumps collected' };
  const tail = r.out.trim().split('\n').slice(-3).join(' · ');
  gate('parity (writes have a control and a file/CLI twin)', r.ok, tail.slice(0, 160));
}

console.log(`\n${worst === 0 ? '✓ every gate passed' : '✗ at least one gate failed'}`);
console.log(`  reports and captures in ${OUT.replace(ROOT + '/', '')}\n`);
process.exit(worst);
