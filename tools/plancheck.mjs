#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · PLAN CHECK

   SAM_PLAN.md is a spec that a session reads, not a document a reader skims.
   Every "§4.10" and every "Phase 0B" in it is a pointer a session will follow
   — and a pointer to a section that was renamed, or a phase that was split,
   sends that session somewhere that does not exist.

   The plan has been rewritten twice for the stack revision. Renumbering is
   exactly when references rot silently, so this checks the three things a
   renumber breaks and nothing else:

     1 · every §N / §N.N reference names a heading that exists
     2 · every Phase <label> reference names a phase heading that exists
         A line may reference a *removed* label deliberately — "the Swift
         Phases 0–3", "the superseded Phase 0" — because that sentence only
         makes sense as history. Those lines must say so, using a HISTORICAL
         marker. The marker is the point: an unmarked dead reference is still
         a bug, which is what this pass is for.
     3 · every "Appendix X" reference names an appendix that exists
         (written in prose, not as §X — which is why it needs its own pass)

   Not checked here, deliberately: heading nesting and table cell counts. The
   markdown linter owns those and already reports them.

   No dependencies. Node 18+.

     node tools/plancheck.mjs            # check, exit 1 on a dangling reference
   ══════════════════════════════════════════════════════════════════════════ */

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const FILE = join(dirname(fileURLToPath(import.meta.url)), '..', 'SAM_PLAN.md');
const lines = readFileSync(FILE, 'utf8').split('\n');

// ── what the document defines ────────────────────────────────────────────
const sections = new Set(); // "4", "4.10", "1.2", "9", "B" (appendix)
const phases = new Set();   // "0A", "0B", "1" … "8"

for (const line of lines) {
  const heading = line.match(/^#{2,6}\s+(.*)$/);
  if (!heading) continue;
  const text = heading[1];
  // "## 4 · Architecture", "### 4.10 · Desktop…", "### 1.2 Acceptance criteria"
  const section = text.match(/^(\d+(?:\.\d+)*)\b/);
  if (section) sections.add(section[1]);
  // "## Appendix A · Sources"  →  referenceable as §A
  const appendix = text.match(/^Appendix\s+([A-Z])\b/);
  if (appendix) sections.add(appendix[1]);
  const phase = text.match(/^Phase\s+(\d+[AB]?)\b/);
  if (phase) phases.add(phase[1]);
}

// A "§11" on a line about design.md is a section of *that* file, not this one.
// Two ways to tell, because the plan cites the design system both ways:
//  1 · by qualifier on the line — "`design/design.md` §11 records what Cadence refuses"
//  2 · bare, inside §5's lint block, where "(§11)" means design.md §11 on every row
// The second case has no qualifier to read, so the foreign ids are named once,
// here, with the reason. This plan has no §11 of its own, ever.
const FOREIGN = new Set(['11']); // design/design.md §11 (the taste refusals)
const isForeignRef = (line, index, id) =>
  FOREIGN.has(id) ||
  /design\.md|design\/|ux\.md/.test(line.slice(Math.max(0, index - 70), index));

// ── what the prose points at ─────────────────────────────────────────────
const HISTORICAL = /\b(Swift|superseded|original|earlier)\b/;

const failures = [];
let sectionRefs = 0;
let phaseRefs = 0;
let appendixRefs = 0;

lines.forEach((line, i) => {
  const at = i + 1;

  for (const m of line.matchAll(/§(\d+(?:\.\d+)*|[A-Z])\b/g)) {
    sectionRefs++;
    if (isForeignRef(line, m.index, m[1])) continue;
    if (!sections.has(m[1])) failures.push(`${at}: dead section reference §${m[1]}`);
  }

  for (const m of line.matchAll(/\bAppendix\s+([A-Z])\b/g)) {
    appendixRefs++;
    if (!sections.has(m[1])) failures.push(`${at}: dead appendix reference "Appendix ${m[1]}"`);
  }

  for (const m of line.matchAll(/\bPhases?\s+(\d+[AB]?)\b/g)) {
    phaseRefs++;
    if (!phases.has(m[1]) && !HISTORICAL.test(line)) {
      failures.push(`${at}: dead phase reference "Phase ${m[1]}"`);
    }
  }
});

// ── report ───────────────────────────────────────────────────────────────
console.log(`sections defined : ${sections.size}`);
console.log(`phases defined   : ${[...phases].join(', ')}`);
console.log(`section refs     : ${sectionRefs}`);
console.log(`phase refs       : ${phaseRefs}`);
console.log(`appendix refs    : ${appendixRefs}`);

if (sections.size === 0 || phases.size === 0) {
  console.error('✗ extracted nothing — the heading format changed; fix this checker');
  process.exit(1);
}
if (failures.length) {
  console.error(`\n✗ ${failures.length} dangling reference(s):`);
  for (const f of failures) console.error('  ' + f);
  process.exit(1);
}
console.log('\n✓ every reference resolves');
