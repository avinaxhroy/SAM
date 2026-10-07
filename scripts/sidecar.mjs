#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · SIDECAR NAMING

   §6 Phase 0B spike S3. Tauri's `bundle.externalBin` looks for
   "binary-name{-target-triple}{.system-extension}" — `sam-aarch64-apple-darwin`,
   `sam-x86_64-pc-windows-msvc.exe` — so a plain `cargo build` output cannot be
   bundled as it stands. This gives the built CLI that name.

   It copies; it does not build. `cargo build --workspace --release` is the
   build gate's first command and has already produced the binary, and invoking
   cargo from a Tauri hook while cargo holds the target lock is how a build
   deadlocks. If the binary is missing, this fails loudly instead of shipping a
   bundle with no CLI in it.

     node tools/sidecar.mjs           # -> src-tauri/binaries/sam-<triple>
   ══════════════════════════════════════════════════════════════════════════ */

import { execFileSync } from 'node:child_process';
import { chmodSync, copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Tauri sets this for its own hooks; `rustc`'s host is the fallback. */
function targetTriple() {
  if (process.env.TAURI_ENV_TARGET_TRIPLE) return process.env.TAURI_ENV_TARGET_TRIPLE;
  const host = /^host: (.+)$/m.exec(execFileSync('rustc', ['-vV'], { encoding: 'utf8' }));
  if (!host) throw new Error('sidecar: `rustc -vV` printed no host triple');
  return host[1].trim();
}

const triple = targetTriple();
const ext = triple.includes('windows') ? '.exe' : '';
const source = join(ROOT, 'target', 'release', `sam${ext}`);

if (!existsSync(source)) {
  console.error(
    `sidecar: ${source} does not exist — run \`cargo build --workspace --release\` first ` +
      '(`cargo tauri build` does not compile the CLI itself).',
  );
  process.exit(1);
}

const destination = join(ROOT, 'src-tauri', 'binaries', `sam-${triple}${ext}`);
mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
if (!ext) chmodSync(destination, 0o755);

console.log(`sidecar: ${destination}`);
