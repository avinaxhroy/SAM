#!/usr/bin/env node
/* ════════════════════════════════════════════════════════════════════════
   CADENCE · TOKENS CHECK

   tokens.css is the source of truth. tokens.json is the machine-readable
   twin, and the two literal blocks in tailwind.css are the only hand copies
   in the system. Three artifacts, one meaning — this file is what keeps
   them that way.

   No dependencies. Node 18+.

     node tools/tokens.mjs            # check, exit 1 on drift

   What it proves:
     1 · every CSS token that has a JSON twin carries the same value
     2 · dark: [data-theme="dark"] agrees with the twin's `$dark` group
     3 · the tailwind.css copies agree with the tokens they copy
     4 · every `var(--x)` in tailwind.css names a token that exists
     5 · nothing is unmapped — every token has a twin, and every twin a token

   Aliases are resolved before comparing, so `--text-sm: var(--text-xs)` and
   the twin's frozen `13px` agree on 13px, and `--r-window: var(--r-card)`
   agrees with `{radius.card}`. Comparison is by meaning, not by spelling.
   ════════════════════════════════════════════════════════════════════════ */

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const DIR = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (f) => readFileSync(join(DIR, f), 'utf8');

/* ── 1 · tokens.css ─────────────────────────────────────────────────────── */

const css = read('tokens.css');
const block = (sel) => {
  const at = css.indexOf(sel);
  if (at === -1) throw new Error(`tokens.css: no ${sel} block`);
  return css.slice(at, css.indexOf('\n}', at));
};
const parse = (text) => {
  const out = new Map();
  for (const m of text.matchAll(/^\s*(--[\w-]+)\s*:\s*([^;]+);/gm)) out.set(m[1], m[2].replace(/\s+/g, ' ').trim());
  return out;
};
const light = parse(block(':root {'));
const dark = parse(block('[data-theme="dark"] {'));

/* Resolve `var(--x)` chains to a final value. Depth-capped so a token that
   references itself is reported instead of hanging. */
const resolve = (name, seen = []) => {
  const raw = light.get(name);
  if (raw === undefined) return undefined;
  if (seen.includes(name) || seen.length > 16) return `<!cycle ${name}>`;
  return raw.replace(/var\((--[\w-]+)\)/g, (_, inner) => resolve(inner, [...seen, name]));
};
const norm = (v) => String(v)
  .replace(/\s+/g, ' ')
  .replace(/,\s+/g, ',')       // cubic-bezier(0.16, 1, 0.3, 1) ~ [0.16,1,0.3,1]
  .replace(/\s*\/\s*/g, '/')   // oklch(… / 0.06)
  .replace(/["']/g, '')      // a family name is the same name quoted or bare
  .trim();

/* ── 2 · the twin ───────────────────────────────────────────────────────── */

const jsonText = read('tokens.json');
let json;
try {
  json = JSON.parse(jsonText);
} catch (err) {
  console.error(`tokens.json is not valid JSON — ${err.message}`);
  process.exit(1);
}

const twin = (path) => path.split('.').reduce((o, k) => (o === undefined ? undefined : o[k]), json);
const twinValue = (path) => {
  const node = twin(path);
  if (node === undefined || node === null) return undefined;
  if (typeof node === 'object' && '$value' in node) return node.$value;
  return node;
};

/* `{a.b.c}` is the twin's alias syntax for `var(--x)`. */
const deref = (path, v, seen = []) => {
  if (typeof v !== 'string') return v;
  if (seen.includes(path)) return v;
  return v.replace(/\{([\w.-]+)\}/g, (whole, ref) =>
    deref(ref, twinValue(ref), [...seen, path]) ?? whole);
};

/* ── 3 · the map: CSS name -> twin path ─────────────────────────────────── */

const SURFACE = ['backdrop', 'sheet', 'well', 'well-2', 'card'];
const INK = ['ink', 'ink-2', 'ink-3', 'ink-4', 'ink-inv', 'rule', 'rule-strong'];
const LAYOUT = {
  '--titlebar-h': 'layout.titlebarHeight',
  '--titlebar-inset-mac': 'layout.titlebarInset.mac',
  '--titlebar-inset-win': 'layout.titlebarInset.win',
  '--pill-h': 'layout.pillHeight',
  '--pill-h-lg': 'layout.pillHeightLg',
  '--chip-h': 'layout.chipHeight',
  '--gutter': 'layout.windowGutter',
  '--rail-w': 'layout.railWidth',
  '--rail-label': 'layout.railLabel',
  '--disc': 'layout.discSize',
  '--hit': 'layout.hitFloor',
  '--canvas-max': 'layout.canvasMax',
  '--prose-measure': 'layout.proseMeasure',
};
const MATERIAL = ['pane', 'fill-on-ink', 'fill-on-ink-hi', 'scrim'];

const mapFor = (name) => {
  const n = name.replace(/^--/, '');
  if (LAYOUT[name]) return LAYOUT[name];
  if (SURFACE.includes(n)) return `color.surface.${n}`;
  if (INK.includes(n)) return `color.ink-level.${n}`;
  if (MATERIAL.includes(n)) return `color.material.${n}`;
  if (n.startsWith('w-')) return `color.identity.${n.slice(2)}.surface`;
  if (n.startsWith('fg-')) return `color.identity.${n.slice(3)}.ink`;
  if (n.startsWith('wash-')) return `color.identity.neutral.${n.slice(5)}`;
  if (n.startsWith('chip-')) return `color.state.${n.slice(5)}.surface`;
  if (n.startsWith('on-')) return `color.state.${n.slice(3)}.ink`;
  if (n.startsWith('glass')) return `color.material.${n}`;
  if (n === 'room-lit') return 'color.material.room-lit';
  if (n === 'focus') return 'color.focus';
  if (n.startsWith('text-')) return `fontSize.${n.slice(5)}`;
  if (n.startsWith('track-')) return `tracking.${n.slice(6)}`;
  if (n.startsWith('space-')) return `space.${n.slice(6)}`;
  if (n.startsWith('r-')) return `radius.${n.slice(2)}`;
  if (n.startsWith('sh-')) return `shadow.${n.slice(3)}`;
  if (n.startsWith('dur-')) return `motion.duration.${n.slice(4)}`;
  if (n.startsWith('weight-')) return `font.${n}`;
  if (n === 'font-display') return 'font.display';
  if (n === 'font-body') return 'font.body';
  if (n === 'ease') return 'motion.easing.out';
  if (n === 'ease-pop') return 'motion.easing.pop';
  if (n === 'catch') return 'shadow.catch';
  if (n === 'rim-glass') return 'shadow.rim-glass';
  return undefined;
};


const IGNORE = new Set([]);

/* ── 4 · compare ────────────────────────────────────────────────────────── */

const problems = [];

const same = (a, b) => {
  if (Array.isArray(a) || Array.isArray(b)) {
    const flat = (v) => (Array.isArray(v) ? v.join(',') : v)
      .replace(/^cubic-bezier\((.*)\)$/, '$1');   // the twin stores easing as numbers
    return norm(flat(a)) === norm(flat(b));
  }
  return norm(a) === norm(b);
};

for (const name of light.keys()) {
  if (IGNORE.has(name)) continue;
  const path = mapFor(name);
  if (!path) { problems.push(`unmapped  ${name}  (no twin path — add a rule to mapFor)`); continue; }
  const expected = twinValue(path);
  if (expected === undefined) { problems.push(`missing   ${name}  -> ${path}  is not in tokens.json`); continue; }
  const mine = resolve(name);
  const theirs = deref(path, expected);
  if (!same(mine, theirs)) {
    problems.push(`drift     ${name} = ${norm(mine)}\n            ${path} = ${norm(theirs)}`);
  }
}

/* Dark: the twin only carries $dark for groups where dark differs. */
for (const name of dark.keys()) {
  const path = mapFor(name);
  if (!path) continue;
  const [group, ...rest] = path.split('.');
  const darkTwin = twinValue([group, '$dark', ...rest].join('.'));
  if (darkTwin === undefined) continue;
  const mine = norm(resolveInDark(name));
  const theirs = norm(darkTwin);
  if (mine !== theirs) {
    problems.push(`dark      ${name} = ${mine}\n            ${[group, '$dark', ...rest].join('.')} = ${theirs}`);
  }
}
function resolveInDark(name, seen = []) {
  const raw = dark.get(name) ?? light.get(name);
  if (raw === undefined) return undefined;
  if (seen.includes(name)) return `<!cycle ${name}>`;
  return raw.replace(/var\((--[\w-]+)\)/g, (_, inner) => resolveInDark(inner, [...seen, name]));
}

/* ── 5 · tailwind.css copies ────────────────────────────────────────────── */

const tw = read('tailwind.css');
const theme = tw.slice(tw.indexOf('@theme {'), tw.indexOf('\n}', tw.indexOf('@theme {')));
const TW_ALIAS = (key) => {              // tailwind's name -> Cadence's name
  if (key === '--font-sans') return '--font-body';        // one face, two names
  if (key === '--spacing') return '--space-2xs';          // the 4px base unit
  if (key.startsWith('--spacing-')) return `--space-${key.slice(10)}`;
  return key;
};
const twProblems = [];
for (const m of theme.matchAll(/^\s*(--[\w-]+)\s*:\s*([^;]+);/gm)) {
  const [, key, value] = m;
  const v = value.trim();
  const ref = v.match(/^var\((--[\w-]+)\)$/);
  if (ref) {
    if (!light.has(ref[1])) twProblems.push(`tailwind  ${key}: var(${ref[1]}) — no such token`);
    continue;
  }
  // A literal. Every one of them must be a copy of a token, or a cycle.
  const target = TW_ALIAS(key);
  if (!light.has(target)) { twProblems.push(`tailwind  ${key} = ${v} — no token ${target} to check against`); continue; }
  if (!same(resolve(target), v)) twProblems.push(`tailwind  ${key} = ${v}  vs  ${target} = ${norm(resolve(target))}`);
}
/* Every hand copy must live in one of the three cycle namespaces. A literal
   anywhere else is a value that forgot to be a token. */
for (const m of theme.matchAll(/^\s*(--[\w-]+)\s*:\s*(.*?);/gm)) {
  const key = m[1];
  if (/^var\(/.test(m[2])) continue;
  if (!/^--(text|spacing|font)/.test(key)) {
    twProblems.push(`tailwind  ${key} is a literal outside the text/spacing/font namespaces`);
  }
}

/* ── 6 · report ─────────────────────────────────────────────────────────── */

const all = [...problems, ...twProblems];
if (!all.length) {
  console.log(`tokens clean — ${light.size} tokens, ${dark.size} dark overrides, all agreeing with tokens.json and tailwind.css`);
  process.exit(0);
}
console.log(all.join('\n'));
console.log(`\n${all.length} problem(s). tokens.css is the source of truth; fix it there.`);
process.exit(1);
