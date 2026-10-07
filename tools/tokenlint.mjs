#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · UI TOKEN LINT

   D17 says the app imports the token layer and writes its own component layer.
   The failure that rule invites is silent: someone pastes `#ffd9d9` or
   `border-radius: 24px` into ui/ and the app now has a second palette that
   looks deliberate all the way to release. §5 forbids exactly that.

   So: every colour, radius and duration in ui/ must resolve through a
   `var(--token)`. Checked here rather than by review, because "it looks fine"
   is what let this class of drift through everywhere else.

   A value that contains `var(` is accepted even if it also carries a fallback
   — `var(--ink, #333)` is flagged, because that fallback is a literal colour
   and the token should be the only source.

   Not checked: spacing and font sizes. They are numeric, they legitimately
   appear inside calc() and as one-off object dimensions, and design.md calls
   those "sizes, not rungs". Flagging them would train people to ignore this.

   Runs only if ui/ exists, so it is green on day one of Phase 0A and starts
   biting the moment the shell lands in Phase 0B.

   No dependencies. Node 18+.

     node tools/tokenlint.mjs          # exit 1 on a literal
   ══════════════════════════════════════════════════════════════════════════ */

import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const UI = join(ROOT, 'ui');

if (!existsSync(UI)) {
  console.log('tokenlint: ui/ does not exist yet (pre-Phase-0B) — nothing to check');
  process.exit(0);
}

// node_modules and build output are not ours to police.
const SKIP = /(^|\/)(node_modules|dist|build|\.svelte-kit)\//;
const EXT = /\.(css|svelte|ts|js|html)$/;

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (SKIP.test(full + '/')) continue;
    if (entry.isDirectory()) out.push(...walk(full));
    else if (EXT.test(entry.name)) out.push(full);
  }
  return out;
}

// Each rule returns a finding string, or null. Written as functions rather than
// one regex with a lookahead: `\s*` backtracks, so `(?!var\()` after it can be
// defeated by giving the whitespace back — which silently flagged correct code.
const RULES = [
  {
    name: 'colour literal',
    // A hex colour, or a colour function. `var(--ink)` contains neither.
    // A colour FUNCTION is only a literal when nothing inside it is a token:
    // `color-mix(in oklab, var(--wash) 82%, var(--card))` is the design
    // system's own wash recipe at design/components.css, and flagging it
    // would train people to ignore this rule rather than fix it. A hex inside
    // such a call is still caught, by the branch above it.
    find: (line) => {
      const m = line.match(/#[0-9a-fA-F]{3,8}\b/);
      if (m) return m[0];
      const fn = line.match(/\b(?:rgba?|hsla?|oklch|oklab|lab|lch|color-mix)\(/);
      if (!fn) return null;
      return /var\(/.test(line.slice(fn.index)) ? null : fn[0] + '…)';
    },
  },
  {
    name: 'radius literal',
    // Flagged only when the value carries no var() at all, and `0` is not a
    // literal: it is the absence of a radius — what `[data-window="flush"]`
    // means by not drawing an app-owned corner.
    find: (line) => {
      const m = line.match(/border(?:-[a-z]+)?-radius\s*:\s*([^;}"'`]+)/);
      if (!m) return null;
      const value = m[1].trim();
      return /var\(/.test(value) || value === '0' ? null : `border-radius: ${value}`;
    },
  },
  {
    name: 'duration literal',
    find: (line) => {
      const m = line.match(/(?:transition|animation)(?:-[a-z]+)?\s*:\s*([^;}"'`]+)/);
      if (!m) return null;
      const value = m[1];
      if (!/\b\d+(?:\.\d+)?m?s\b/.test(value)) return null;
      return /var\(/.test(value) ? null : `${value.trim()}`;
    },
  },
];

const findings = [];
let scanned = 0;

for (const file of walk(UI)) {
  const rel = relative(ROOT, file);
  scanned++;
  readFileSync(file, 'utf8').split('\n').forEach((line, i) => {
    for (const rule of RULES) {
      const hit = rule.find(line);
      if (hit) findings.push(`${rel}:${i + 1}: ${rule.name} — "${hit}"`);
    }
  });
}

console.log(`tokenlint: ${scanned} file(s) scanned under ui/`);

if (findings.length) {
  console.error(`\n✗ ${findings.length} literal(s) where a token belongs (D17, §5):`);
  for (const f of findings) console.error('  ' + f);
  process.exit(1);
}
console.log('✓ no literal colour, radius or duration outside the token register');
