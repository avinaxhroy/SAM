#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE ACCESSIBILITY REPORT CHECK (dev only)

   Phase 7's interaction gate, judged: `tools/a11y-page.js` runs inside the page
   and writes a report (JSON); this file decides whether the report passes. It
   exists so the check can fail — a report nobody validates is a screenshot.

     node tools/a11ycheck.mjs --report <file.json>[,<file.json>]

   Checks, each one a fact the page measured:
     1 · every interactive control and every `data-command` control has an
         accessible name and is keyboard-focusable
     2 · every dialog announces itself (`role="dialog"`, `aria-modal="true"`)
     3 · with `prefers-reduced-motion: reduce`, nothing animates longer than
         20 ms (design/base.css's one global rule)
     4 · no cue is colour-only: a wash or chip with no text and no name fails
     5 · normal text clears 4.5:1 and large text 3:1 against its resolved
         background — recomputed from `getComputedStyle`, never from metadata
     6 · a raised text scale does not overflow the window horizontally
   ══════════════════════════════════════════════════════════════════════════ */

import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);
// Every `--report` flag is accepted, and each one takes one or more paths —
// comma-separated or separate arguments, allowing shell globs to expand directly.
const flattened = [];
for (let index = 0; index < args.length; index += 1) {
  if (args[index] !== '--report') continue;
  for (let next = index + 1; next < args.length && !args[next].startsWith('--'); next += 1) {
    for (const path of args[next].split(',')) {
      const trimmed = path.trim();
      if (trimmed) flattened.push(trimmed);
    }
  }
}
if (flattened.length === 0) {
  console.error('usage: node tools/a11ycheck.mjs --report <file.json> [<file.json> …]');
  process.exit(2);
}

const problems = [];
const summaries = [];

for (const path of flattened) {
  let report;
  try {
    report = JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    problems.push(`${path}: cannot read — ${error.message}`);
    continue;
  }
  const fail = (message) => problems.push(`${path}: ${message}`);
  const list = (name) => (Array.isArray(report[name]) ? report[name] : []);

  if (list('withoutName').length > 0) {
    fail(`${list('withoutName').length} control(s) with no accessible name: ${list('withoutName').slice(0, 6).join(', ')}`);
  }
  if (list('weakName').length > 0) {
    fail(`${list('weakName').length} control(s) whose whole name is one character: ${list('weakName').slice(0, 6).join(', ')}`);
  }
  if (list('unfocusable').length > 0) {
    fail(`${list('unfocusable').length} control(s) not reachable by keyboard: ${list('unfocusable').slice(0, 6).join(', ')}`);
  }
  if (list('dialogsMissingRole').length > 0) {
    fail(`${list('dialogsMissingRole').length} dialog(s) without role=dialog: ${list('dialogsMissingRole').join(', ')}`);
  }
  if (list('dialogsMissingModal').length > 0) {
    fail(`${list('dialogsMissingModal').length} dialog(s) without aria-modal: ${list('dialogsMissingModal').join(', ')}`);
  }
  if (list('reducedMotion').length > 0) {
    fail(`${list('reducedMotion').length} element(s) still animate under reduced motion: ${list('reducedMotion').slice(0, 4).join(', ')}`);
  }
  if (list('colourOnly').length > 0) {
    fail(`${list('colourOnly').length} colour-only cue(s): ${list('colourOnly').slice(0, 6).join(', ')}`);
  }
  if (list('contrastFailures').length > 0) {
    const worst = list('contrastFailures').slice(0, 4).map((failure) => `${failure.element} ${failure.ratio}:1 (needs ${failure.floor})`);
    fail(`${list('contrastFailures').length} contrast failure(s): ${worst.join('; ')}`);
  }
  if (typeof report.overflow === 'number' && report.overflow > 1) {
    fail(`the document overflows horizontally by ${report.overflow}px at text scale ${report.textScale || '?'}`);
  }

  summaries.push(
    `${path}: ${report.interactive ?? '?'} controls · ${report.dialogs ?? 0} dialogs · ` +
      `${report.contrastChecked ?? 0} text nodes · theme ${report.themeId ?? report.theme ?? '?'}`
  );
}

for (const line of summaries) console.log(line);
if (problems.length > 0) {
  for (const problem of problems) console.log(`✗ ${problem}`);
  console.log(`\n${problems.length} problem(s).`);
  process.exit(1);
}
console.log('✓ accessible names, focus, dialogs, reduced motion, non-colour cues and computed contrast all pass');
process.exit(0);
