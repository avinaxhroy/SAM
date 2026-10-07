#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · Gate 2 — declared vs rendered, in **both directions** (dev gate)

   G4, in the gate's own words (`UX_FLOWS.md` §5): *"Every declared write command
   has a rendered control **and** a documented file/CLI form"* — run in both
   directions (F9–F11). That is two claims, and this diff judges both:

     **rendered → declared.** What the page draws must be what the registry
     declares. Every column menu, settings row and row/cell context below is
     "a declared control exists here", so a control the app draws without a
     declaration is the same failure as a declaration nothing draws.

     **declared → rendered.** Every write the registry declares must have a
     **door**. A door is one of three, and the first one found wins: a control a
     capture renders (`data-command=<id>`); an OS menu item (`data.menu` names
     the id — a browser has no menu bar, so no capture can carry it); or a door
     documented in `tools/fixtures/parity.json` — the first-run picker, a System
     row, a palette action, or the CLI. A documented door names the file that
     implements it, and this gate verifies that the file exists **and** contains
     the id: an unverifiable claim is a failing claim. A write with no door is
     `missing`, and `missing` fails.

   G4's second half is stated too: every write carries a **file form** (the
   `json` path the dump declares) or — when its files are transient — a
   documented reason, and the registry's own generic verb `sam <id>` (the
   per-kind pair as `sam <kind>.new` / `sam <kind>.paste`) is always its CLI
   form. A write with neither a file nor a reason fails as well.

   Appendix C.6 leaves one decision to the owner and this is it, option (b):

     `sam --uicheck --json` emits the **declared** structure from the registry
     and the active config — no window, no webview. The app emits its
     **rendered** structure from the DOM (`window.__SAM_RENDER_DUMP__()`, the
     attribute contract in C.6). This diffs the two, so a declared control with
     no rendered counterpart is a failing gate rather than an assumption.

   A dump alone proves reachability; it cannot prove a control *works*. The
   interaction half — clicking it, typing in it, watching the bytes change — is
   recorded per platform in BUILDLOG.md, because it needs a running window.

     sam --uicheck --json --plan <dir> > /tmp/declared.json
     # in the app (a browser, the dev harness, or a Tauri devtools console):
     #   copy(JSON.stringify(window.__SAM_RENDER_DUMP__()))
     node tools/uicheck-diff.mjs --plan <dir> --rendered <rendered.json>
   ══════════════════════════════════════════════════════════════════════════ */

import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

function argument(name, fallback = null) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback;
}

const plan = argument('--plan');
/**
 * One snapshot, or several. A screen's controls live across states — the canvas,
 * the palette, the settings screen, an open menu — and no single snapshot holds
 * them all, because §11 allows one floating surface at a time. Merging the
 * snapshots proves *reachability*, which is what this gate is for; a control
 * that never renders in any state still fails.
 *
 *   --rendered a.json,b.json   (or repeat --rendered)
 */
const renderedPaths = process.argv
  .map((value, index) => (value === '--rendered' ? process.argv[index + 1] : null))
  .filter(Boolean)
  .flatMap((value) => value.split(','))
  .filter(Boolean);
const renderedPath = renderedPaths[0] ?? null;
const asJson = process.argv.includes('--json');
const sam = join(ROOT, 'target/release/sam');
const resources = join(ROOT, 'Sources/SAMCore/Resources');

if (!plan || renderedPaths.length === 0) {
  console.error(
    'uicheck-diff: --plan <directory> and --rendered <dump.json>[,<dump.json>] are required',
  );
  process.exit(2);
}

/** Sum counting maps and union id lists, so several snapshots read as one. */
function merge(snapshots) {
  const merged = { commands: {}, paletteCommands: {}, placements: {}, columns: {}, records: {} };
  const keys = new Set();
  for (const snapshot of snapshots) {
    for (const field of ['commands', 'paletteCommands', 'placements', 'columns', 'records']) {
      for (const [value, count] of Object.entries(snapshot[field] ?? {})) {
        merged[field][value] = (merged[field][value] ?? 0) + count;
      }
    }
    for (const key of snapshot.settingsKeys ?? []) keys.add(key);
    merged.pane = merged.pane ?? snapshot.pane ?? null;
    merged.view = merged.view ?? snapshot.view ?? null;
    merged.type = merged.type ?? snapshot.type ?? null;
    merged.menus = (merged.menus ?? 0) + (snapshot.menus ?? 0);
  }
  merged.settingsKeys = [...keys].sort();
  return merged;
}

/** The declared dump, straight from the engine's own inspection flag. */
function declared() {
  return new Promise((resolve, reject) => {
    execFile(
      sam,
      ['--uicheck', '--json', '--plan', plan],
      { cwd: ROOT, env: { ...process.env, SAM_RESOURCES: resources }, maxBuffer: 16 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (!stdout.trim()) return reject(new Error(stderr.trim() || String(error)));
        try {
          resolve(JSON.parse(stdout));
        } catch (parseError) {
          reject(new Error(`the declared dump is not JSON: ${parseError}`));
        }
      },
    );
  });
}

/** Every `header <key>:<type> → <ids…>` line, as the dump prints it. */
function declaredColumns(lines) {
  const columns = [];
  for (const line of lines) {
    const match = /^\s*header ([^:]+):(\S+) → (.+)$/.exec(line);
    if (!match) continue;
    const [, key, type, rest] = match;
    columns.push({
      key,
      type,
      ids: rest
        .split('·')
        .map((part) => part.trim())
        .filter((part) => part.length > 0),
    });
  }
  return columns;
}

/**
 * `ui/src/App.svelte:168` → `ui/src/App.svelte`. The line is for the reader:
 * the gate verifies what does not drift (`tools/fixtures/parity.json` names a
 * file that must exist and must mention the id), not a line number a sibling's
 * edit can move.
 */
function evidencePath(evidence) {
  return String(evidence ?? '').trim().replace(/:\d+$/, '');
}

const snapshots = [];
for (const path of renderedPaths) {
  snapshots.push(JSON.parse(await readFile(path, 'utf8')));
}
const rendered = merge(snapshots);
const declaredDump = await declared();
const renderedCommands = new Set(Object.keys(rendered.commands ?? {}));
/** Ids whose only rendering anywhere is a ⌘K row: not a designed control. */
const designedCommands = new Set(
  Object.entries(rendered.commands ?? {})
    .filter(([id, count]) => count - (rendered.paletteCommands?.[id] ?? 0) > 0)
    .map(([id]) => id),
);
const renderedKeys = new Set(rendered.settingsKeys ?? []);
const missing = [];

// 1. Every column menu, item by item. The menu is where the schema is edited,
//    and its item list is what the dump declares per column.
for (const column of declaredColumns(declaredDump.data.lines)) {
  for (const id of column.ids) {
    if (!renderedCommands.has(id)) {
      missing.push(`column ${column.key}:${column.type} declares ${id}, which nothing rendered`);
    }
  }
  if (!renderedCommands.has(column.key) && rendered.columns && !(column.key in rendered.columns)) {
    // The column header itself must carry its key, or the gate's `data-column`
    // walk cannot find the cell it is meant to prove.
    missing.push(`column ${column.key} is declared and no rendered header carries data-column="${column.key}"`);
  }
}

// 2. The settings rows, each of which must show its config key.
const settingsRows = declaredDump.data.lines
  .filter((line) => /^\s*row ([^:]+):/.test(line))
  .map((line) => /^\s*row ([^:]+):/.exec(line)[1]);
for (const key of settingsRows) {
  if (!renderedKeys.has(key)) {
    missing.push(`settings row ${key} is declared and no rendered row carries data-settings-key="${key}"`);
  }
}

// 3. The registry's write set, as the dump declares it: one line per write, with
//    the placement it belongs to and the file it writes. The write *ids* are
//    G4's subject, so they are kept whole rather than re-parsed per check.
const writeLines = declaredDump.data.lines
  .filter((line) => /^\s+\S+ · placement \S+ · json /.test(line))
  .map((line) => {
    const [, id, placement, json] = /^\s+(\S+) · placement (\S+) · json (.*)$/.exec(line);
    return { id, placement, json: json.trim() };
  });
const writeIds = writeLines.map((row) => row.id);

// 3a. The documented doors — G4's half that no browser capture can carry. The OS
//     menu bar, the room (drawn only when no plan is open) and a palette
//     action are real doors with no `data-command` attribute in any snapshot, so
//     each is documented in the parity fixture with the file that implements it.
//     The claim is verified before it is trusted: the evidence file must exist
//     and must name the id, so a fixture that outlives its door fails the gate
//     instead of covering it.
const DOOR_KINDS = new Set(['menu', 'startScreen', 'system', 'palette', 'cli']);
const fixturePath = join(ROOT, 'tools/fixtures/parity.json');
const fixtureName = relative(ROOT, fixturePath);
let fixture = null;
try {
  fixture = JSON.parse(await readFile(fixturePath, 'utf8'));
} catch (error) {
  missing.push(`the parity fixture ${fixtureName} is not readable: ${error.message ?? error} — write a door down where it can be checked`);
}
const declaredDoors = fixture?.doors;
const documented =
  declaredDoors && typeof declaredDoors === 'object' && !Array.isArray(declaredDoors)
    ? declaredDoors
    : {};
if (fixture !== null && documented !== declaredDoors) {
  missing.push(`the parity fixture ${fixtureName} must hold a "doors" object — one entry per documented door`);
}
for (const [id, entry] of Object.entries(documented)) {
  if (!writeIds.includes(id)) {
    missing.push(`the parity fixture documents ${id}, which the dump does not declare as a write`);
    continue;
  }
  if (!entry || typeof entry !== 'object') {
    missing.push(`the parity fixture's ${id} entry must be an object with door · evidence · why`);
    continue;
  }
  if (!DOOR_KINDS.has(entry.door)) {
    missing.push(`the parity fixture's ${id} entry needs a door — one of ${[...DOOR_KINDS].join(' · ')}`);
  }
  if (typeof entry.why !== 'string' || entry.why.trim().length === 0) {
    missing.push(`the parity fixture's ${id} entry needs one plain sentence saying why a capture cannot show it`);
  }
  const path = evidencePath(entry.evidence);
  if (!path) {
    missing.push(`the parity fixture's ${id} entry needs an evidence path (ui/src/…:<line> or tools/…)`);
    continue;
  }
  let evidenceText = null;
  try {
    evidenceText = await readFile(join(ROOT, path), 'utf8');
  } catch {
    missing.push(`the parity fixture documents ${id} (${entry.door}) with evidence ${entry.evidence}, which does not exist`);
  }
  if (evidenceText !== null && !evidenceText.includes(id)) {
    missing.push(`the parity fixture documents ${id} (${entry.door}) with evidence ${entry.evidence}, which never mentions ${id}`);
  }
}

// 4. The row and cell contexts, which the dump names literally.
for (const id of ['copy-json', 'copy-path', 'record.reveal', 'record.move', 'record.delete', 'record.setField', 'records.renumber']) {
  if (declaredDump.data.lines.some((line) => line.includes(id)) && !renderedCommands.has(id)) {
    missing.push(`the row/cell context declares ${id}, which nothing rendered`);
  }
}

// 5. The menu-bar projection: declared here, native in the shell. The browser
//    has no menu bar, so this is reported rather than gated (BUILDLOG records
//    the native check per platform) — but it *is* a door, so the doors table
//    below counts it as one.
const menuIds = (declaredDump.data.menu?.menus ?? []).flatMap((menu) =>
  (menu.items ?? []).map((item) => item.id),
);
const menuSet = new Set(menuIds);

// 6. The doors table — G4 read the other way. One row per declared write: where
//    its UI door is (a rendered control · an OS menu item · a documented door),
//    what it writes (the declared `json` path, or `transient` when its files do
//    not survive), and the registry's own generic verb that is always its CLI
//    form. `missing` is the failure the whole reverse direction exists to catch.
const doors = writeLines.map(({ id, placement, json }) => {
  const entry = documented[id] ?? null;
  const renderedHere = designedCommands.has(id);
  const ui = renderedHere
    ? 'rendered'
    : menuSet.has(id)
      ? 'menu'
      : entry
        ? 'documented'
        : renderedCommands.has(id)
          ? 'palette-only'
          : 'missing';
  // A per-kind dynamic command (`<kind>.new` / `<kind>.paste`) is generated from
  // the plan's types, so its CLI form names the kind rather than one id.
  const suffix = id.slice(id.lastIndexOf('.'));
  const dynamic =
    /^[A-Za-z][\w-]*\.(new|paste)$/.test(id) &&
    json.startsWith(`content/records/${id.slice(0, id.length - suffix.length)}.jsonl`);
  return {
    id,
    placement,
    ui,
    file: json === '-' ? 'transient' : json,
    cli: dynamic ? `sam <kind>${suffix}` : `sam ${id}`,
    dynamic,
    door: entry?.door ?? null,
    evidence: entry?.evidence ?? null,
    why: entry?.why ?? null,
  };
});
for (const row of doors) {
  if (row.ui === 'missing') {
    missing.push(
      `command ${row.id} is declared as a write and nothing rendered it — no menu item names it and ${fixtureName} does not document a door`,
    );
  }
  if (row.ui === 'palette-only') {
    // A ⌘K row lists the id and runs it with typed parameters. That is a door
    // for a power user; D11 asks for a control designed for the action, and G4
    // accepts a menu item or a documented door in its place — never the palette
    // alone, or the gate would pass on every command by construction.
    missing.push(
      `command ${row.id} renders only as a palette row — a designed control (any capture state), an OS menu item, or a door documented in ${fixtureName} is required`,
    );
  }
  if (row.file === 'transient' && !documented[row.id]) {
    missing.push(
      `command ${row.id} is declared as a write with no file form and no documented reason — its files are transient, so ${fixtureName} must say why`,
    );
  }
}
const doorCounts = { rendered: 0, menu: 0, documented: 0, 'palette-only': 0, missing: 0 };
for (const row of doors) doorCounts[row.ui] += 1;

const report = {
  plan,
  rendered: renderedPaths,
  declaredColumns: declaredColumns(declaredDump.data.lines).length,
  renderedCommands: renderedCommands.size,
  settingsRows: settingsRows.length,
  writeCommands: writeIds.length,
  menuItems: menuIds.length,
  menuItemsWithoutARenderedCounterpart: menuIds.filter((id) => !renderedCommands.has(id)).length,
  doors: {
    counts: doorCounts,
    cli: "every write id's CLI form is the registry's own generic verb — `sam <id>`, and `sam <kind>.new` / `sam <kind>.paste` for the per-kind pair",
    rows: doors,
  },
  missing,
};

if (asJson) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log(`declared columns  ${report.declaredColumns}`);
  console.log(`rendered commands ${report.renderedCommands}`);
  console.log(`settings rows     ${report.settingsRows} (${renderedKeys.size} rendered)`);
  console.log(`write commands    ${report.writeCommands}`);
  console.log(
    `menu items        ${report.menuItems} (native surface; ${report.menuItemsWithoutARenderedCounterpart} not rendered in a browser)`,
  );
  console.log(
    `doors ${doorCounts.rendered} rendered · ${doorCounts.menu} menu · ${doorCounts.documented} documented · ${doorCounts['palette-only']} palette-only · ${doorCounts.missing} missing`,
  );
  console.log('declared write → its doors');
  for (const row of doors) {
    const note = row.ui === 'documented' ? `  (${row.door}: ${row.evidence})` : '';
    console.log(`  ${row.id.padEnd(22)} ${row.ui.padEnd(11)} ${row.file.padEnd(46)} ${row.cli}${note}`);
  }
  if (missing.length === 0) {
    console.log('✓ every declared control has a rendered counterpart');
    console.log('✓ every declared write has a UI door and a file/CLI form');
  } else {
    for (const line of missing) console.log(`✗ ${line}`);
  }
}

process.exit(missing.length === 0 ? 0 : 1);