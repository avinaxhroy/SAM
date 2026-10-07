#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE BUNDLED STARTS, READ OFF THE BUNDLE  (dev only)

   The first screen offers the presets the bundle ships (`plan.new --preset`),
   and a student chooses a start by what it holds — so the screen needs each
   preset's own sentence and its size. Both are *derived here*, from
   `Sources/SAMCore/Resources/presets/<id>/`, and written to
   `ui/src/shell/presets.ts`: the bundle stays the one source, the UI carries a
   generated copy, and `--check` (run by `tools/gates.mjs`) fails the moment
   the copy and the bundle disagree — a start added, removed or resized.

     node tools/presets.mjs --write    regenerate ui/src/shell/presets.ts
     node tools/presets.mjs --check    verify the copy is current (gate)

   The titles live here because a title is a display choice, not a fact of the
   bundle; every id the bundle ships must be named (or skipped) here, and the
   check says so when one is not. `blank` is the one press the screen already
   makes and `seed` is the harness's demo, so neither is a *choice* on the
   screen — the check keeps them unlisted on purpose.
   ══════════════════════════════════════════════════════════════════════════ */
import { readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const PRESETS = join(ROOT, 'Sources/SAMCore/Resources/presets');
const OUT = join(ROOT, 'ui/src/shell/presets.ts');

/** Display titles for the starts the screen offers, in the order the row lists them; every other id is either `blank` (the one press) or skipped below. */
const TITLES = {
  jee: 'JEE',
  neet: 'NEET',
  'cbse11-pcm': 'CBSE 11 PCM',
  'university-term': 'University term',
  language: 'Language',
  'self-study': 'Self-study',
};

/** The plural a count is said with, for the plan's own work kinds (`est`-bearing, `trackable`). */
const PLURALS = {
  topic: 'topics',
  problemset: 'problem sets',
  word: 'words',
  sentence: 'sentences',
  dialogue: 'dialogues',
  session: 'sessions',
};

/** The README's promise, in its own words: the first paragraph that is neither a heading nor a bold label. */
function lineOf(readme) {
  const paragraphs = readme
    .split(/\n\s*\n/)
    .map((block) => block.trim())
    .filter(Boolean);
  for (const block of paragraphs) {
    if (block.startsWith('#')) continue;
    const stripped = block
      .replace(/^\*\*(One sentence|Promise)\.?:?\*\*\s*/i, '')
      .replace(/[*`_]/g, '')
      .replace(/\s+/g, ' ')
      .trim();
    if (stripped.length < 20) continue;
    const sentence = (stripped.match(/^.*?[.!?](?=\s|$)/) ?? [stripped])[0].trim();
    const text = sentence.charAt(0).toUpperCase() + sentence.slice(1);
    if (text.length <= 180) return text;
    const cut = text.slice(0, 180);
    return `${cut.slice(0, cut.lastIndexOf(' '))}…`;
  }
  return '';
}

/**
 * One preset's facts, said with the plan's own nouns: the kind the plan itself
 * counts as work is the `trackable` kind carrying the most records (the JEE
 * start's spine is its `topic`s, the language start's is its `word`s), and the
 * minutes are that kind's own `est`.
 */
function factsOf(id) {
  const dir = join(PRESETS, id);
  const readme = existsSync(join(dir, 'README.md')) ? readFileSync(join(dir, 'README.md'), 'utf8') : '';
  const content = join(dir, 'content');
  const types = existsSync(join(content, 'types.json'))
    ? (JSON.parse(readFileSync(join(content, 'types.json'), 'utf8')).types ?? {})
    : {};
  const trackable = Object.entries(types)
    .filter(([, type]) => type?.trackable === true)
    .map(([name]) => name);
  const records = join(content, 'records');
  const rows = existsSync(records)
    ? readdirSync(records)
        .filter((file) => file.endsWith('.jsonl'))
        .flatMap((file) =>
          readFileSync(join(records, file), 'utf8')
            .split('\n')
            .filter(Boolean)
            .map((line) => JSON.parse(line)),
        )
    : [];
  const work = trackable
    .map((kind) => {
      const ofKind = rows.filter((row) => row.type === kind);
      const minutes = ofKind.reduce((total, row) => {
        const est = row.fields?.est;
        return total + (typeof est === 'number' ? est : 0);
      }, 0);
      return { kind, count: ofKind.length, minutes };
    })
    .filter((entry) => entry.count > 0)
    .sort((a, b) => b.count - a.count)[0];
  return {
    id,
    title: TITLES[id],
    line: lineOf(readme),
    work: work?.count ?? 0,
    workLabel: work ? (PLURALS[work.kind] ?? work.kind) : '',
    minutes: work?.minutes ?? 0,
  };
}

/** The generated module, verbatim: the header names this file so nobody edits the copy. */
function render(starts) {
  const rows = starts
    .map(
      (start) =>
        `  { id: ${JSON.stringify(start.id)}, title: ${JSON.stringify(start.title)}, line: ${JSON.stringify(
          start.line,
        )}, work: ${start.work}, workLabel: ${JSON.stringify(start.workLabel)}, minutes: ${start.minutes} },`,
    )
    .join('\n');
  return `// GENERATED by \`node tools/presets.mjs --write\` — the bundled starts, read off
// \`Sources/SAMCore/Resources/presets/\` (their README's own sentence and their
// records' own size). Do not edit by hand; \`node tools/presets.mjs --check\`
// (a gate in \`tools/gates.mjs\`) fails the moment this copy and the bundle
// disagree. Fields: the registry's \`plan.new --preset\` id, the display title,
// one sentence of what it holds, the plan's own work kind and its count, and the
// minutes that work plans.
export type BundledStart = {
  id: string;
  title: string;
  line: string;
  work: number;
  workLabel: string;
  minutes: number;
};

export const BUNDLED_STARTS: BundledStart[] = [
${rows}
];
`;
}

const present = readdirSync(PRESETS, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);
const missing = Object.keys(TITLES).filter((id) => !present.includes(id));
if (missing.length) {
  console.error(`presets: TITLES names ${missing.join(', ')}, which the bundle does not ship — fix tools/presets.mjs`);
  process.exit(1);
}
const starts = Object.keys(TITLES).map(factsOf);
const body = render(starts);

if (process.argv.includes('--write')) {
  writeFileSync(OUT, body);
  console.log(`presets: wrote ui/src/shell/presets.ts — ${starts.length} starts, ${starts.reduce((n, s) => n + s.work, 0)} work items`);
} else {
  const current = existsSync(OUT) ? readFileSync(OUT, 'utf8') : '';
  if (current !== body) {
    console.error('presets: ui/src/shell/presets.ts is stale — run `node tools/presets.mjs --write`');
    process.exit(1);
  }
  console.log(`presets: ${starts.length} starts in sync`);
}
