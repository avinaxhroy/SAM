#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE STRING CENSUS, JUDGED (dev gate)

   `UI_PLAN.md` D2 and §6.3: no surface a student reads speaks the schema. This
   file decides whether a page's census report passes. It exists so the check
   can fail — a report nobody validates is a screenshot.

     node tools/uicensus.mjs --report <file.json> [<file.json> …]

   Each report comes from `tools/census-page.js` run inside the rendered page,
   named by its driver (`report.page`). Every rule in `CENSUS_RULES` must total
   **zero** across every page; a single JSON pointer in a subtitle fails the
   gate, which is the point — the count is the acceptance criterion, not a
   style opinion.

   `--json` prints the merged counts instead of the human summary.
   ══════════════════════════════════════════════════════════════════════════ */

import { readFileSync } from 'node:fs';

import { CENSUS_RULES } from './census-page.js';

const args = process.argv.slice(2);
// Every `--report` flag is accepted, and each one takes one or more paths —
// comma-separated or separate arguments, allowing shell globs to expand directly.
const flattened = [];
for (let index = 0; index < args.length; index += 1) {
  if (args[index] !== '--report') continue;
  for (let next = index + 1; next < args.length && !args[next].startsWith('--'); next += 1) {
    flattened.push(...args[next].split(','));
  }
}
const asJson = args.includes('--json');

if (flattened.filter(Boolean).length === 0) {
  console.error('usage: node tools/uicensus.mjs --report <file.json> [<file.json> …]');
  process.exit(2);
}

const totals = {};
const samples = {};
const problems = [];
const summaries = [];

for (const path of flattened.filter(Boolean)) {
  let report;
  try {
    report = JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    problems.push(`${path}: not a readable report (${error.message})`);
    continue;
  }

  const page = report.page ?? path;
  const counts = report.counts ?? {};
  const leaked = CENSUS_RULES.reduce((sum, rule) => sum + (counts[rule.id] ?? 0), 0);

  for (const rule of CENSUS_RULES) {
    const count = counts[rule.id] ?? 0;
    if (count === 0) continue;
    totals[rule.id] = (totals[rule.id] ?? 0) + count;
    const list = samples[rule.id] ?? (samples[rule.id] = []);
    for (const text of report.samples?.[rule.id] ?? []) {
      if (list.length < 12 && list.indexOf(text) === -1) list.push(`${page} · ${text}`);
    }
  }

  summaries.push(`${page}: ${report.total ?? '?'} rendered strings · ${leaked} in the forbidden vocabulary`);
}

for (const line of summaries) console.log(line);

if (asJson) {
  console.log(JSON.stringify({ totals, samples }, null, 2));
}

if (problems.length > 0) {
  console.error('\n' + problems.map((line) => `✗ ${line}`).join('\n'));
}

const failing = CENSUS_RULES.filter((rule) => (totals[rule.id] ?? 0) > 0);
for (const rule of failing) {
  console.error(`\n✗ ${totals[rule.id]} × ${rule.label}:`);
  for (const sample of samples[rule.id].slice(0, 6)) console.error(`    ${sample}`);
}

if (problems.length > 0 || failing.length > 0) {
  const total = Object.values(totals).reduce((sum, count) => sum + count, 0);
  console.error(`\n✗ the interface speaks the schema: ${total} occurrence(s) across ${failing.length} rule(s)`);
  process.exit(1);
}

console.log('✓ every rendered string is a name a student would say out loud');
process.exit(0);
