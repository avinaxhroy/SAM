#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · A REAL PLAN TO LOOK AT  (dev only)

   The `jee` preset provides an unpopulated schema fixture. This script
   materializes the preset into a plan and populates realistic study
   data through the engine's transactional write path (`SAM apply`).

   So this materialises the preset into a plan and then **fills it through the
   engine's own bulk door** — `SAM apply <batch>.jsonl --dry-run` then
   `--if-revision <hash>`, which is the agent loop the engine documents
   (`sam guide`) and the same validator, transaction engine and journal the app
   uses. Nothing here edits JSON inside the plan: the batch is written outside,
   previewed, and committed at the reviewed revision, so a row that appears in
   the UI appeared because the engine accepted a write.

   What it produces, and which screen's missing input each one is:
     · dated study sessions              → the stem chart on Progress, the day's
                                          standing on Today
     · topics carried up the ladder      → the coverage meters, the mastery marks
     · graded reviews, several overdue   → the recall card with four real intervals
     · dated assessments, one already sat→ the calendar, the countdown chips
     · weeks and topics given real dates → the "coming up" column, the "why" line
     · two banks left without a day      → the "no day" sentence on Progress

   node tools/seed-plan.mjs [--plan /tmp/samlive] [--port 4401] [--fresh]
   ══════════════════════════════════════════════════════════════════════════ */
import { execFileSync } from 'node:child_process';
import { existsSync, rmSync, readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
function arg(n, d) { const i = process.argv.indexOf(n); return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : d; }
const PLAN = arg('--plan', '/tmp/samlive');
const PORT = arg('--port', '4401');
const SAM = join(ROOT, 'target/release/sam');
const RESOURCES = join(ROOT, 'Sources/SAMCore/Resources');
const ENV = { ...process.env, SAM_RESOURCES: RESOURCES };

if (!existsSync(SAM)) {
  console.error(`seed-plan: ${SAM} is missing — run \`cargo build --release\` first`);
  process.exit(1);
}

function run(args, opts = {}) {
  return execFileSync(SAM, args, { env: ENV, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...opts });
}

/** `sam <args…>` → the engine's own JSON envelope, or a throw with its reason. */
function sam(...args) {
  const parsed = JSON.parse(run(['--plan', PLAN, ...args]));
  if (!parsed.ok) throw new Error(`${args[0]} refused: ${parsed.error?.message ?? JSON.stringify(parsed).slice(0, 300)}`);
  return parsed.data;
}

const iso = (daysAgo) => {
  const d = new Date();
  d.setDate(d.getDate() - daysAgo);
  return d.toISOString().slice(0, 10);
};

/* ── 0 · a fresh plan from the preset ─────────────────────────────────────── */
// A bare `--fresh` is the documented spelling, so the flag is read as a flag:
// `arg()` only sees a *following* token, and a trailing `--fresh` returned null
// here, so the wipe never ran and a re-seed died on duplicate records.
if (process.argv.includes('--fresh')) rmSync(PLAN, { recursive: true, force: true });
if (!existsSync(PLAN)) run(['plan.new', '--plan', PLAN, '--preset', 'jee'], { stdio: 'inherit' });

/* ── 1 · read what the plan actually holds, through its own reads ─────────── */
const views = JSON.parse(readFileSync(join(PLAN, 'content/views.json'), 'utf8')).views ?? {};
const unique = (rows) => [...new Map(rows.filter((r) => r?.id).map((r) => [r.id, r])).values()];
/* View names are read out of the plan rather than typed: a renamed view would
   make this script seed nothing while still exiting 0. */
const byType = (type) => unique(
  Object.entries(views).filter(([, d]) => d.type === type).flatMap(([name]) => sam('view', name).records ?? []),
);
const topics = byType('topic');
const weeks = byType('week');
const courses = byType('course');
const sets = byType('problemset');
const mocks = byType('assessment');
const byCourse = (id) => topics.filter((t) => (t.links?.course ?? [])[0] === id);

console.log(
  `seed: ${topics.length} topics · ${weeks.length} weeks · ${courses.length} courses · ` +
    `${sets.length} banks · ${mocks.length} assessments`,
);
if (!topics.length || !courses.length) {
  console.error('seed: the preset did not yield records — nothing to seed against');
  process.exit(1);
}

/* ── 2 · the batch ────────────────────────────────────────────────────────────
   One JSON object per line: `fields` for values, `links` for relations as id
   arrays, `formula`/`progress` never persisted (the engine's own authoring
   rule, `sam guide`). Ids are opaque and stable, and never encode week or
   course. */
const batch = [];
/** `links` arrives already in the read shape — id *arrays* — because a record
    read through `view` hands back `links: { course: ["c.jee.physics"] }` and
    re-wrapping it is the mistake the dry run above caught: "expected a record
    id string, found an array". Relations are id arrays; a scalar is a field. */
const push = (type, id, fields = {}, links = {}) =>
  batch.push({ schemaVersion: 1, type, id, fields, links });

/* Weeks get real dates: a plan with no dates has no horizon, and three of the
   screens are about the horizon. */
weeks.forEach((week, index) => {
  const start = new Date();
  start.setDate(start.getDate() - index * 7 + 3);
  const end = new Date(start);
  end.setDate(end.getDate() + 6);
  push('week', week.id, {
    ...week.fields,
    start: start.toISOString().slice(0, 10),
    end: end.toISOString().slice(0, 10),
  }, week.links);
});

/* The study history. */
const SESSIONS = [
  [26, 95, 0, 3], [25, 140, 1, 3], [24, 45, 2, 3], [22, 180, 0, 3],
  [21, 60, 2, 3], [19, 120, 1, 2], [18, 200, 0, 2], [16, 75, 2, 2],
  [15, 160, 1, 1], [13, 90, 0, 1], [12, 210, 2, 1], [11, 55, 1, 1],
  [9, 130, 0, 0], [8, 175, 1, 0], [6, 100, 2, 0], [5, 145, 0, 0],
  [4, 80, 2, 0], [2, 190, 1, 0], [1, 110, 0, 0], [0, 65, 2, 0],
];
SESSIONS.forEach(([daysAgo, min, courseIndex, weekIndex], i) => {
  const course = courses[courseIndex];
  if (!course) return;
  push('session', `p.jee.s${String(i + 1).padStart(2, '0')}`,
    { min, date: iso(daysAgo), note: '' },
    { course: [course.id], week: weeks[weekIndex] ? [weeks[weekIndex].id] : [] });
});

/* Focus dates: what the countdown chips and the "why" line read. Negative is a
   date in the future, positive is in the past — the two overdue entries are
   what the overdue row and the overdue chip are for. */
const FOCUS = [[-2, 0], [-4, 1], [-1, 2], [1, 0]];
const focused = new Set();
for (const [offset, courseIndex] of FOCUS) {
  const course = courses[courseIndex];
  if (!course) continue;
  const topic = byCourse(course.id).find((t) => !focused.has(t.id));
  if (!topic) continue;
  focused.add(topic.id);
  push('topic', topic.id, { ...topic.fields, focus: iso(offset) }, topic.links);
}

/* A plan with 17 topics is a schema fixture, not a study plan, and every density
   decision on Plan, Today and Subjects is a decision about how a *long* list
   reads. The preset is the wrong size to judge that, so the batch carries the
   rest of the term: 63 more topics spread across the three courses and the eight
   weeks, in the plan's own vocabulary (unit · week · course · est · kind), with
   the estimates and the kinds an actual syllabus has. Ids stay opaque and
   stable and encode nothing — the engine's own authoring rule. */
const CHAPTERS = {
  physics: [
    'Kinematics', 'Gravity and Motion', 'Work, Energy and Power',
    'Rotational Motion', 'Thermodynamics', 'Oscillations and Waves',
    'Electrostatics', 'Current Electricity', 'Magnetism',
    'Electromagnetic Induction', 'Optics', 'Wave Optics', 'Modern Physics',
  ],
  chemistry: [
    'Atomic Structure', 'Chemical Bonding', 'Stoichiometry', 'Solutions',
    'Thermodynamics', 'Equilibrium', 'Redox Reactions', 'Electrochemistry',
    'Chemical Kinetics', 'Periodic Table', 'Organic Basics', 'Hydrocarbons',
    'Coordination Chemistry',
  ],
  mathematics: [
    'Sets, Relations and Functions', 'Matrices and Determinants', 'Permutations',
    'Quadratic Equations', 'Complex Numbers', 'Sequences and Series',
    'Binomial Theorem', 'Limits and Continuity', 'Differentiation',
    'Integration', 'Vectors and Geometry', 'Three Dimensional Geometry',
    'Probability',
  ],
};
const KINDS = ['watch', 'read', 'practice'];
const slug = (text) => text.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

courses.forEach((course, courseIndex) => {
  const key = Object.keys(CHAPTERS)[courseIndex] ?? 'physics';
  const chapters = CHAPTERS[key];
  // 8 weeks x ~3 topics, moving through the chapter list, front-loaded into the
  // weeks the plan has already reached.
  for (let w = 0; w < weeks.length; w += 1) {
    for (let n = 0; n < 3; n += 1) {
      const chapter = chapters[(w * 3 + n + courseIndex) % chapters.length];
      const suffix = w * 3 + n + 1;
      const est = 30 + ((w * 7 + n * 11 + courseIndex * 5) % 4) * 15;
      push('topic', `t.jee.${key}.${slug(chapter)}.${String(suffix).padStart(2, '0')}`, {
        title: chapter,
        kind: KINDS[(w + n + courseIndex) % KINDS.length],
        est,
      }, {
        course: course.links?.course ?? [course.id],
        week: weeks[w].links?.week ?? [weeks[w].id],
        unit: (course.links?.unit ?? []).slice(0, 1),
      });
    }
  }
});

/* Assessments: three dated (one overdue, two ahead) and one left without a day,
   because "waiting for a day" is a state the Mocks screen has to draw. */
const MOCK_DATES = [2, 11, 24, null];
mocks.slice(0, 4).forEach((mock, i) => {
  const fields = { ...mock.fields };
  if (MOCK_DATES[i]) fields.date = iso(MOCK_DATES[i]);
  if (i === 0) fields.score = 214;
  if (i === 1) fields.score = 268;
  push('assessment', mock.id, fields, mock.links);
});

/* Two banks worked recently, so Progress's "no day" sentence is about the few
   rather than about all eight. */
sets.slice(0, 2).forEach((set) =>
  push('problemset', set.id, { ...set.fields, lastAttempt: iso(1) }, set.links));

/* ── 3 · preview, then commit at the reviewed revision (the engine's loop) ─── */
const dir = mkdtempSync(join(tmpdir(), 'sam-seed-'));
const file = join(dir, 'batch.jsonl');
writeFileSync(file, batch.map((r) => JSON.stringify(r)).join('\n') + '\n');

const preview = sam('apply', file, '--dry-run');
console.log(`seed: preview ${preview.written ?? preview.records ?? batch.length} record(s), revision ${preview.revision ?? preview.reviewedRevision ?? '—'}`);
if (preview.problems?.length || preview.warnings?.length) {
  console.log('seed: preview reported', JSON.stringify(preview.problems ?? preview.warnings).slice(0, 400));
}
const revision = preview.revision ?? preview.reviewedRevision ?? preview.baseRevision;
const committed = sam('apply', file, ...(revision ? ['--if-revision', revision] : []));
console.log(`seed: committed at ${committed.revision ?? '—'}`);

const seen = new Set(focused);
/* The ladder is a ladder: `content/rules.json#/pipelines/flip/gates` says
   `proved requires learned` and `anchored requires proved`, and the engine
   refuses a jump — which is the product working. So each topic is climbed in
   order, one transaction per rung, and only then rated. */
const LADDER = [
  // [daysAgo, rating, courseIndex, rungs]
  [19, 'again', 0, 'anchored'], [17, 'again', 1, 'proved'], [15, 'hard', 0, 'proved'],
  [14, 'again', 2, 'anchored'], [12, 'hard', 0, 'learned'], [11, 'again', 1, 'anchored'],
  [10, 'good', 0, 'proved'], [9, 'hard', 1, 'learned'], [8, 'again', 2, 'proved'],
  [7, 'good', 1, 'anchored'], [6, 'again', 0, 'learned'], [5, 'good', 2, 'anchored'],
  [4, 'again', 1, 'learned'], [3, 'hard', 0, 'proved'], [2, 'good', 2, 'learned'],
  [1, 'again', 0, 'learned'],
];
const RUNGS = ['learned', 'proved', 'anchored'];
let advanced = 0;
let reviewed = 0;
LADDER.forEach(([daysAgo, rating, courseIndex, top], index) => {
  const course = courses[courseIndex];
  if (!course) return;
  const topic = byCourse(course.id).find((t) => !seen.has(t.id));
  if (!topic) return;
  seen.add(topic.id);
  for (const stage of RUNGS.slice(0, RUNGS.indexOf(top) + 1)) {
    try {
      sam('record.advanceStage', '--id', topic.id, '--stage', stage,
        ...(stage === 'proved' ? ['--problems', String(3 + (index % 4))] : []),
        '--reason', stage === 'anchored' ? 'returned to it twice, unaided'
          : stage === 'proved' ? 'solved the set unaided' : 'worked the examples',
        '--at', iso(daysAgo + 2 - RUNGS.indexOf(stage)));
      advanced += 1;
    } catch (error) {
      return; /* the gate refused; this topic stays where it is */
    }
  }
  try {
    sam('record.logReview', '--id', topic.id, '--rating', rating, '--at', iso(daysAgo));
    reviewed += 1;
  } catch { /* a record with no stage has no review to log */ }
});

console.log(
  `seed: +${batch.length} records applied · +${advanced} advanced · +${reviewed} reviews\n` +
    `seed: plan is at ${PLAN} — serve it with \`node tools/web-ipc.mjs --plan ${PLAN} --port ${PORT}\``,
);
