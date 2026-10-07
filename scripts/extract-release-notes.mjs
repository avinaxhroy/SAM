#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · CHANGELOG RELEASE PARSER
   Extracts the latest release version and change notes from CHANGELOG.md
   for GitHub Actions automated releases.
   ══════════════════════════════════════════════════════════════════════════ */

import { existsSync, readFileSync, writeFileSync, appendFileSync } from 'node:fs';
import { resolve } from 'node:path';

const ROOT = resolve('.');
const changelogPath = existsSync(resolve(ROOT, 'CHANGELOG.md'))
  ? resolve(ROOT, 'CHANGELOG.md')
  : existsSync(resolve(ROOT, 'changelog.md'))
    ? resolve(ROOT, 'changelog.md')
    : null;

if (!changelogPath) {
  console.error('Error: Neither CHANGELOG.md nor changelog.md found in workspace.');
  process.exit(1);
}

const content = readFileSync(changelogPath, 'utf8');

// Matches headings like:
// ## [0.1.0] - 2026-10-07
// ## [0.1.0]
// ## 0.1.0 - 2026-10-07
// ## v0.1.0
const versionRegex = /^##\s+\[?v?([0-9]+\.[0-9]+\.[0-9]+[^\]\s]*)\]?(?:\s+-\s+.*)?$/m;
const match = versionRegex.exec(content);

if (!match) {
  console.error('Error: No version heading matching SemVer found in changelog.');
  process.exit(1);
}

const version = match[1].trim();
const tag = `v${version}`;

const startIdx = match.index + match[0].length;
const rest = content.slice(startIdx);
const nextHeaderIdx = rest.search(/^##\s+/m);
const notes = (nextHeaderIdx === -1 ? rest : rest.slice(0, nextHeaderIdx)).trim();

const isPrerelease = /-(beta|alpha|rc|preview|dev)/i.test(version);

const args = process.argv.slice(2);
function argVal(name) {
  const idx = args.indexOf(name);
  return idx !== -1 && idx + 1 < args.length ? args[idx + 1] : null;
}

const outNotes = argVal('--out-notes');
if (outNotes) {
  writeFileSync(outNotes, notes, 'utf8');
}

const outVersion = argVal('--out-version');
if (outVersion) {
  writeFileSync(outVersion, version, 'utf8');
}

if (args.includes('--github-output') && process.env.GITHUB_OUTPUT) {
  const ghOutput = process.env.GITHUB_OUTPUT;
  appendFileSync(ghOutput, `version=${version}\n`, 'utf8');
  appendFileSync(ghOutput, `tag=${tag}\n`, 'utf8');
  appendFileSync(ghOutput, `prerelease=${isPrerelease}\n`, 'utf8');
  // Multiline delimiter for GITHUB_OUTPUT
  appendFileSync(ghOutput, `notes<<EOF\n${notes}\nEOF\n`, 'utf8');
}

console.log(JSON.stringify({ version, tag, isPrerelease, notesLength: notes.length }, null, 2));

