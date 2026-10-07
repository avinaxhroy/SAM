#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE FIRST RUN, ON A FRESH MACHINE  (dev only)

   The gate suite drives one plan that already exists, so nothing in it ever
   mounts the screen a person meets first. This does — a throwaway `HOME` (the
   engine's own plans root under it, emptied), `--shell-plan none`, the real app
   in real Chromium against the real engine:

     the room draws with no chrome · three movements, named · every way to make
     a plan carries `plan.new`, the presets behind their own link · no button is
     unnamed · the ground holds in both themes and at 620 px · the press writes
     exactly one plan into the machine's own root · Today names the plan empty
     and carries the plan's own identity kind · Plan's empty card names the
     plan's own kinds and carries both doors it invites · Courses' zero state
     speaks the same kind · its door writes the student's own course, and the
     file holds the name they typed · Today teaches the next thing, carrying the
     plan's own work kind · the document re-opens over the plan, and closes ·
     a machine that already has plans opens the room on its last movement and
     opens one from a row · no page errors.

   The throwaway HOME is the point: the machine's own plans root is never
   touched (BUILDLOG #34 records the audit that parked the real root, lost a
   plan, and had to say so). Nothing here reads `~/Library`, `~/Documents` or
   the repo's fixtures; `--plan` points at the path a press *would* create.

     node tools/firstrun-smoke.mjs [--port 4411] [--out .captures/firstrun]

   Exit code = the checks that failed (0 = all passed). `--out` holds the
   captures and `verify.txt`, one PASS/FAIL line per check.
   ══════════════════════════════════════════════════════════════════════════ */
import { chromium } from '/Users/avinash/.cache/samshot/node_modules/playwright/index.mjs';
import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = join(dirname(fileURLToPath(import.meta.url)), '..');
function argument(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback;
}

// Both sides are built here, because a smoke over the last build measures the
// last build — and it measures it green. #38's lesson, the same one: a gate
// nobody re-ran is not a gate, and a gate that ran against yesterday's bundle is
// worse, because it says so in the present tense. Incremental: seconds when the
// tree has not moved.
execFileSync('cargo', ['build', '--release', '--bin', 'sam'], { cwd: REPO, stdio: 'inherit' });
execFileSync('pnpm', ['-C', 'ui', 'exec', 'vite', 'build'], { cwd: REPO, stdio: 'ignore' });

const PORT = Number(argument('--port', '4411'));
const OUT = join(REPO, argument('--out', '.captures/firstrun'));
/** The machine this run pretends to be. Removed and remade; never the real one. */
const HOME = join(tmpdir(), 'sam-firstrun-home');

rmSync(HOME, { recursive: true, force: true });
mkdirSync(HOME, { recursive: true });
mkdirSync(OUT, { recursive: true });

/** The engine's own plans root, read from the engine: the same `paths` door the
    shell reads. On a machine with no plan open this is where a press writes. */
const paths = JSON.parse(
  execFileSync(join(REPO, 'target/release/sam'), ['paths', '--json'], { cwd: REPO, env: { ...process.env, HOME } }),
);
const PLANS = paths.data.plansDir;
mkdirSync(PLANS, { recursive: true });
/** What the press will create — absent at boot, which is what a fresh machine is. */
const PLAN = join(PLANS, 'My plan');

const server = spawn('node', ['tools/web-ipc.mjs', '--plan', PLAN, '--shell-plan', 'none', '--port', String(PORT)], {
  cwd: REPO,
  env: { ...process.env, HOME },
  stdio: ['ignore', 'pipe', 'pipe'],
});
await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error('web-ipc never listened')), 20000);
  server.stdout.on('data', (chunk) => {
    if (String(chunk).includes('http://')) { clearTimeout(timer); resolve(); }
  });
  server.stderr.on('data', (chunk) => process.stderr.write(chunk));
});
process.on('exit', () => server.kill('SIGKILL'));

const log = [];
let failures = 0;
function check(label, ok, detail = '') {
  log.push(`${ok ? 'PASS' : 'FAIL'} ${label}${detail ? ` — ${detail}` : ''}`);
  console.log(log[log.length - 1]);
  writeFileSync(join(OUT, 'verify.txt'), `${log.join('\n')}\n`);
  if (!ok) failures += 1;
}

const browser = await chromium.launch();
process.on('exit', () => { void browser.close().catch(() => {}); });
const ctx = await browser.newContext({ viewport: { width: 1180, height: 860 }, deviceScaleFactor: 1 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 300)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 200)); });

const shot = async (name) => { await page.screenshot({ path: `${OUT}/${name}.png` }); };

// ── 1 · the fresh machine ────────────────────────────────────────────────────
await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('.ob-room', { timeout: 20000 });
await page.waitForTimeout(600);
check('the plans root holds nothing', readdirSync(PLANS).length === 0, `${readdirSync(PLANS).length} entries`);
check('the room draws without the rail', (await page.$$('.cd-nav')).length === 0);
const heading = await page.$eval('.ob-h1', (n) => n.textContent.trim()).catch(() => '');
check('the room opens with the question the app answers', heading === 'Know what to study today.', heading);
const movements = await page.$$eval('.ob-dot', (nodes) => nodes.map((n) => n.getAttribute('aria-label')));
check(
  'three movements, named, with the first current',
  movements.length === 3 && movements[0] === 'Screen 1 of 3: What it is',
  movements.join(' | '),
);
check('nothing to shut on the first screen', (await page.$$('.ob-shut:not([data-quiet="1"])')).length === 0);
await shot('room-one-light');

// Screen II — the two doors, drawn as the app's own parts.
await page.click('.ob-foot button[data-auto]');
await page.waitForTimeout(700);
const panels = await page.$$eval('.ob-panel__title', (nodes) => nodes.map((n) => n.textContent.trim()));
check('the second movement draws both doors', panels.length === 2, panels.join(' | '));
check('screen two is named', (await page.$eval('.ob-name', (n) => n.textContent.trim())) === 'How it works');
const ways = await page.$$eval('.ob-kick', (nodes) => nodes.map((n) => n.textContent.trim()));
check(
  'both ways are named as ways to change the same plan',
  ways.join(' | ') === 'In the app | In the files' &&
    (await page.$eval('.ob-h1', (n) => n.textContent.trim())) === 'Change anything, two ways.',
  ways.join(' | '),
);
check(
  'and the room promises the files are never required',
  (await page.$eval('.ob-join b', (n) => n.textContent.replace(/\s+/g, ' ').trim())) ===
    'You never have to open a file.',
);
// Each title is one line: the head row is shared, so a wrapped title makes one
// column's head taller than the other's and pushes the two cards that share
// the row below it out of line. At 26px the longer title wrapped; the size is
// `--text-lg` and this is the check that keeps it there (Correction 4).
const titleLines = await page.evaluate(() =>
  [...document.querySelectorAll('.ob-panel__title')].map((node) => {
    const range = document.createRange();
    range.selectNodeContents(node);
    return range.getClientRects().length;
  }),
);
check(
  'both titles stand on one line',
  titleLines.length === 2 && titleLines.every((n) => n === 1),
  titleLines.join(' | '),
);
// The two specimens are one teaching drawn twice — the same card, the same
// three captions — so they stand at one size on one line. They did not: the
// right title wraps to two lines and the file's card is the shorter of the
// two, so the composer's card began 90px lower and stood 90px taller, which
// reads as one card being the important one (Correction 3).
const size = await page.evaluate(() => {
  const draw = (node) => {
    const r = node.getBoundingClientRect();
    return { top: Math.round(r.top), h: Math.round(r.height) };
  };
  const cards = [...document.querySelectorAll('.ob-demo > .cd-card')].map(draw);
  return { cards, delta: Math.abs(cards[0].h - cards[1].h), line: Math.abs(cards[0].top - cards[1].top) };
});
check(
  'the two specimens stand at one size, on one line',
  size.cards.length === 2 && size.delta <= 1 && size.line <= 1,
  `${size.cards[0]?.h} vs ${size.cards[1]?.h} tall, tops ${size.cards[0]?.top} vs ${size.cards[1]?.top}`,
);
// The cell the component lands in is the composer's own: Add writes a
// full-width component (`setScreenSurfaces` gives a surface no recorded width
// `span: 2`), and the frame's width pill names the width the *next* press gives
// it, so a landed frame reads `Half width`. A specimen whose frame stood half
// wide under that pill would be teaching the opposite of what the press does.
// Both widths are compared as they are drawn — border boxes, because the
// frame's own 1px dashed edge is part of what stands in the row.
const landed = await page.evaluate(() => {
  const stack = document.querySelector('.ob-stack');
  const frame = document.querySelector('.ob-slot > .cmp-frame');
  const pill = frame?.querySelector('.cmp-width');
  const draw = (node) => Math.round(node?.getBoundingClientRect().width ?? 0);
  return { stack: draw(stack), frame: draw(frame), pill: pill?.textContent.trim() ?? '' };
});
check(
  'the landed component stands the row wide, and its pill names the next press',
  landed.stack > 0 && landed.frame === landed.stack && landed.pill === 'Half width',
  `${landed.frame} of ${landed.stack}, pill: ${landed.pill}`,
);

// Screen II's specimens run a twelve-second cycle, so each photograph of it is
// taken twice: the *pick* (the catalog's card standing over the cell) and the
// *stand* (the component landed, the file's line in). One shot per room state
// would report whichever beat the clock happened to be on — #39 shipped three
// captures of a screen whose second half no instrument had photographed.
await shot('room-two-light-pick');
await page.waitForTimeout(6300);
await shot('room-two-light-stand');

// Screen III — the press, and the presets one deliberate reach away.
await page.click('.ob-foot button[data-auto]');
await page.waitForTimeout(700);
check('the third screen asks for the plan', (await page.$eval('.ob-h1', (n) => n.textContent.trim())) === 'Make the plan.');
const doors = await page.$$eval('[data-command="plan.new"]', (nodes) => nodes.map((n) => n.textContent.trim().slice(0, 40)));
check('one press makes the plan', doors.length === 1 && /Create plan/.test(doors[0]), doors.join(' | '));
await page.click('.ob-link[aria-expanded="false"]');
await page.waitForTimeout(300);
const presets = await page.$$eval('.ob-preset', (nodes) => nodes.map((n) => n.textContent.trim().slice(0, 30)));
check('the presets are a deliberate reach, six of them', presets.length === 6, `${presets.length}: ${presets[0] ?? ''}`);
await page.click('.ob-link[aria-expanded="true"]');
await page.waitForTimeout(300);
const unlabelled = await page.$$eval('button', (nodes) =>
  nodes.filter((n) => !n.textContent.trim() && !n.getAttribute('aria-label')).length);
check('no button is unnamed', unlabelled === 0, `${unlabelled} unnamed`);
await shot('room-three-light');

// The paper in the OS's other theme, and back. The room carries the app's own
// theme disc (there is no rail behind it at first run), and the *assertion* is
// the mode the engine resolved - a capture that lies cost this suite a day once,
// so the check reads the resolved mode rather than showing a photograph.
const disc = await page.$('.ob-out button[aria-label]');
check('the room carries the app’s own theme disc', Boolean(disc), disc ? await disc.getAttribute('aria-label') : 'none');
await page.emulateMedia({ colorScheme: 'dark' });
await page.reload({ waitUntil: 'domcontentloaded' });
await page.waitForSelector('.ob-room', { timeout: 20000 });
await page.waitForTimeout(900);
const darkMode = await page.evaluate(() => document.documentElement.dataset.theme);
check('a dark system boots the room dark', darkMode === 'dark', darkMode ?? 'none');
await shot('room-one-dark');
await page.click('.ob-dots button:last-child');
await page.waitForTimeout(600);
await shot('room-three-dark');
// Screen II is the one with two columns in it, so it is the one the ground's
// other theme and the narrow width are worth a photograph of.
await page.click('.ob-dots button:nth-child(2)');
await page.waitForTimeout(600);
await shot('room-two-dark-pick');
await page.waitForTimeout(6300);
await shot('room-two-dark-stand');
await page.emulateMedia({ colorScheme: 'light' });
await page.reload({ waitUntil: 'domcontentloaded' });
await page.waitForSelector('.ob-room', { timeout: 20000 });
await page.waitForTimeout(900);
const lightMode = await page.evaluate(() => document.documentElement.dataset.theme);
check('and a light system boots it light', lightMode === 'light', lightMode ?? 'none');
await page.click('.ob-dots button:last-child');
await page.waitForTimeout(600);

// Narrow holds without sideways scroll, and so does the window's own minimum.
await page.setViewportSize({ width: 620, height: 900 });
await page.waitForTimeout(300);
const narrow = await page.evaluate(() => ({ scrollW: document.documentElement.scrollWidth, clientW: document.documentElement.clientWidth }));
check('620 px holds without sideways scroll', narrow.scrollW <= narrow.clientW, `${narrow.scrollW} vs ${narrow.clientW}`);
await shot('room-three-narrow');
await page.click('.ob-dots button:nth-child(2)');
await page.waitForTimeout(600);
await shot('room-two-narrow-pick');
await page.waitForTimeout(6300);
await shot('room-two-narrow-stand');
await page.click('.ob-dots button:first-child');
await page.waitForTimeout(600);
const cta = await page.$eval('.ob-foot .cd-pill', (n) => n.getBoundingClientRect().bottom);
check('620×900 holds the first screen’s press above the fold', cta <= 900, `${Math.round(cta)} of 900`);
await shot('room-one-narrow');
await page.setViewportSize({ width: 560, height: 420 });
await page.waitForTimeout(400);
const floor = await page.evaluate(() => {
  const room = document.querySelector('.ob-room');
  return { scrollW: room.scrollWidth, clientW: room.clientWidth, tall: room.scrollHeight > room.clientHeight };
});
check('the window’s own minimum (560×420) only scrolls', floor.scrollW <= floor.clientW && floor.tall, `${floor.scrollW} vs ${floor.clientW}, scrolls: ${floor.tall}`);
await page.click('.ob-dots button:last-child');
await page.setViewportSize({ width: 1180, height: 860 });
await page.waitForTimeout(300);

// ── 2 · the press writes a plan ──────────────────────────────────────────────
await page.click('button[data-command="plan.new"]'); // the leading press: an empty plan
for (let i = 0; i < 40 && readdirSync(PLANS).length === 0; i += 1) await page.waitForTimeout(250);
const root = readdirSync(PLANS);
check('the press writes exactly one plan into the root', root.length === 1, root.join(' · '));
check('the plan is a folder the student can see', root[0] === 'My plan', root[0] ?? '');
await page.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page.waitForTimeout(1500);
await shot('today-empty-plan-light');

// ── 3 · Today's empty plan names the plan's own kind ─────────────────────────
const emptyTitle = await page.$eval('.tw-first .cd-card__title', (n) => n.textContent.trim()).catch(() => '');
check('Today says the plan is empty', emptyTitle === 'Your plan is empty', emptyTitle);
const emptyDoor = await page.$('[data-command="course.new"]');
check('the empty card carries the plan’s own identity kind', Boolean(emptyDoor), emptyDoor ? await emptyDoor.textContent() : 'none');
const emptyStory = await page.$('[data-command="app.gettingStarted"]');
check('and the document, from Today too', Boolean(emptyStory));

// ── 4 · Plan's empty card, while the plan is still empty ─────────────────────
await page.click('.cd-nav .cd-disc[data-view="plan.screen"]');
await page.waitForTimeout(1200);
await shot('plan-empty-light');
const planSay = await page.$eval('.cd-empty__s', (n) => n.textContent.replace(/\s+/g, ' ').trim()).catch(() => '');
check('Plan’s empty card names the plan’s own kinds', /\bweek\b/.test(planSay) && /\btopic\b/.test(planSay), planSay.slice(0, 140));
const weekDoor = await page.$('[data-command="week.new"]');
const topicDoor = await page.$('[data-command="topic.new"]');
check('and carries both doors it invites', Boolean(weekDoor) && Boolean(topicDoor), `${weekDoor ? await weekDoor.textContent() : 'none'} | ${topicDoor ? await topicDoor.textContent() : 'none'}`);
const planDoors = await page.$$eval('.cd-empty [data-command]', (nodes) => nodes.length);
check('no door the copy does not invite', planDoors === 2, `${planDoors} doors`);
await page.click('.cd-nav .cd-disc[data-view="today.screen"]');
await page.waitForTimeout(800);

// ── 5 · Courses' zero state, on the plan that has no course ──────────────────
await page.click('.cd-nav .cd-disc[data-view="courses.screen"]');
await page.waitForTimeout(1200);
await shot('courses-zero-light');
const zeroText = await page.$eval('.cd-empty__t', (n) => n.textContent.trim()).catch(() => '');
check('Courses’ zero state speaks the plan’s own kind', zeroText === 'This plan has no courses yet', zeroText);
const zeroDoor = await page.$('[data-command="course.new"]');
check('the zero state carries the same door Today does', Boolean(zeroDoor), zeroDoor ? await zeroDoor.textContent() : 'none');

// ── 6 · that door writes the student's own course ────────────────────────────
if (zeroDoor) {
  await zeroDoor.click();
  await page.waitForTimeout(700);
  await page.fill('input[aria-label="Name"], input[aria-label="Title"]', 'Physics').catch(() => {});
  await page.waitForTimeout(200);
  await page.click('.cd-sheet button[data-command="course.new"]');
  await page.waitForTimeout(1200);
  await shot('course-receipt-light');
  const receipt = await page.$eval('.cd-sheet', (n) => n.textContent.replace(/\s+/g, ' ').trim().slice(0, 200)).catch(() => '');
  check('the door answers with a receipt naming the kind', /Created\s+Course/i.test(receipt), receipt.slice(0, 120));
  await page.keyboard.press('Escape');
  await page.waitForTimeout(900);
  await shot('courses-one-light');
}
const written = readFileSync(join(PLAN, 'content/records/course.jsonl'), 'utf8').trim();
check('the plan holds the course the student named', /"name":"Physics"/.test(written), written.slice(0, 120));
const summary = await page.$eval('.cd-pagehead__sub, .crsa-titlerow, .crsa-sum', (n) => n.textContent.replace(/\s+/g, ' ').trim()).catch(() => '');
check('the course is on the screen', /\b1\b/.test(summary) && /course/i.test(summary), summary.slice(0, 120));

// ── 7 · Today now teaches the next thing ─────────────────────────────────────
await page.click('.cd-nav .cd-disc[data-view="today.screen"]');
await page.waitForTimeout(1200);
await shot('today-teach-light');
const teach = await page.$eval('.tw-first .cd-card__title', (n) => n.textContent.trim()).catch(() => '');
check('Today names the next thing to add', teach === 'Nothing to study yet', teach);
const teachDoor = await page.$('[data-command="topic.new"]');
check('the teach card carries the plan’s own work kind', Boolean(teachDoor), teachDoor ? await teachDoor.textContent() : 'none');

// ── 8 · the document, re-readable over the plan ──────────────────────────────
if (emptyStory) {
  await page.click('[data-command="app.gettingStarted"]');
  await page.waitForTimeout(900);
  await shot('doc-from-today-light');
  const overRoom = await page.$('.ob-room');
  check('the document opens over the plan', Boolean(overRoom), overRoom ? await page.$eval('.ob-name', (n) => n.textContent.trim()) : 'none');
  check('the overlay carries the shut control', (await page.$$('.ob-shut')).length === 1);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(500);
  check('Escape closes it', (await page.$$('.ob-room')).length === 0);
}
// ── 9 · plans on disk, none open: the room opens where the rows are ─────────
// The state the old picker was the whole of (Appendix C.5's `several`): a plan
// or two in the root and no shell remembering one. It needs its own server and
// a fresh context — the first page remembers its plan in its own storage, and
// `--shell-plan none` over an empty root is `noneExists`, not this. The room is
// the same three movements; what changes is where it opens: the last one, rows
// first, with the two before it one press back. A third plan is written
// *outside* the root and handed to the page's own memory — the rows are the
// root plus what the machine remembers, which is the only way a plan kept
// elsewhere is one press away the next morning.
execFileSync(
  join(REPO, 'target/release/sam'),
  ['plan.new', '--name', 'Second plan', '--preset', 'self-study', '--json'],
  { cwd: REPO, env: { ...process.env, HOME }, stdio: 'ignore' },
);
const elsewhere = join(HOME, 'Documents', 'Keep me');
execFileSync(
  join(REPO, 'target/release/sam'),
  ['plan.new', '--preset', 'self-study', '--plan', elsewhere, '--json'],
  { cwd: REPO, env: { ...process.env, HOME }, stdio: 'ignore' },
);
const server2 = spawn(
  'node',
  ['tools/web-ipc.mjs', '--plan', join(PLANS, 'Second plan'), '--shell-plan', 'none', '--port', String(PORT + 1)],
  { cwd: REPO, env: { ...process.env, HOME }, stdio: ['ignore', 'pipe', 'pipe'] },
);
await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error('the second web-ipc never listened')), 20000);
  server2.stdout.on('data', (chunk) => {
    if (String(chunk).includes('http://')) { clearTimeout(timer); resolve(); }
  });
  server2.stderr.on('data', (chunk) => process.stderr.write(chunk));
});
process.on('exit', () => server2.kill('SIGKILL'));

const ctx2 = await browser.newContext({ viewport: { width: 1180, height: 860 }, deviceScaleFactor: 1 });
/** The page's own memory of plans (`session.recent`), as the last session left it. */
await ctx2.addInitScript(
  (path) => localStorage.setItem('sam.recentPlans', JSON.stringify([path])),
  elsewhere,
);
const page2 = await ctx2.newPage();
const errors2 = [];
page2.on('pageerror', (e) => errors2.push(String(e).slice(0, 300)));
page2.on('console', (m) => { if (m.type() === 'error') errors2.push(m.text().slice(0, 200)); });
await page2.goto(`http://localhost:${PORT + 1}/`, { waitUntil: 'domcontentloaded' });
await page2.waitForSelector('.ob-room', { timeout: 20000 });
await page2.waitForTimeout(900);
const opened = await page2.$eval('.ob-name', (n) => n.textContent.trim());
check('plans on disk, none open: the room opens on its last movement', opened === 'Get started', opened);
const rows = await page2.$$eval('.ob-row__name', (nodes) => nodes.map((n) => n.textContent.trim()));
check(
  'and lists the plans on disk and the one this machine remembers',
  rows.length === 3 && rows.includes('My plan') && rows.includes('Second plan') && rows.includes('Keep me'),
  rows.join(' | '),
);
check('the shell is behind the room, not drawn', (await page2.$$('.cd-nav')).length === 0);
await page2.screenshot({ path: `${OUT}/several-light.png` });
await page2.click('.ob-dots button:first-child');
await page2.waitForTimeout(600);
const back = await page2.$eval('.ob-h1', (n) => n.textContent.trim());
check('the movements are one press back', back === 'Know what to study today.', back);
await page2.click('.ob-dots button:last-child');
await page2.waitForTimeout(600);
await page2.locator('.ob-row', { hasText: 'My plan' }).first().click();
await page2.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page2.waitForTimeout(1200);
const pressed = await page2.$eval('.tw-first .cd-card__title', (n) => n.textContent.trim()).catch(() => '');
check('a pressed row opens that plan', pressed === 'Nothing to study yet', `${pressed} · room: ${(await page2.$$('.ob-room')).length}`);
await page2.screenshot({ path: `${OUT}/several-opened-light.png` });
// A remembered plan is a path the plans root knows nothing about: pressing it
// opens what is there, from the room, exactly as a listed row does.
await page2.goto(`http://localhost:${PORT + 1}/?again=${Date.now()}`, { waitUntil: 'domcontentloaded' });
await page2.waitForSelector('.ob-room', { timeout: 20000 });
await page2.waitForTimeout(800);
await page2.locator('.ob-row', { hasText: 'Keep me' }).first().click();
await page2.waitForSelector('.cd-nav .cd-disc', { timeout: 20000 });
await page2.waitForTimeout(1200);
check(
  'a plan remembered outside the root opens the same way',
  (await page2.$$('.ob-room')).length === 0 && errors2.length === 0,
  `room: ${(await page2.$$('.ob-room')).length} · errors: ${errors2.slice(0, 1).join(' | ')}`,
);
check('the second machine leaves no page errors', errors2.length === 0, errors2.slice(0, 2).join(' | '));
await ctx2.close();

check('no page errors and no console errors', errors.length === 0, errors.slice(0, 2).join(' | '));

console.log(`\n${failures === 0 ? '✓ all checks passed' : `✗ ${failures} check(s) failed`} — ${OUT}`);
await browser.close();
server.kill();
process.exit(failures === 0 ? 0 : 1);
