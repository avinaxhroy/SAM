#!/usr/bin/env node
/* ══════════════════════════════════════════════════════════════════════════
   SAM · THE WEB HARNESS (dev only)

   Runs the real frontend in a browser against the real engine.

   `ui/` is standard web behind a command boundary (D19), and §7 calls the
   frontend's reversibility the reason the stack change was cheap. That property
   is only worth something if it is exercised: this serves `ui/dist` and answers
   `/ipc` with **`sam invoke -`**, so the page in the browser dispatches the same
   registry ids through the same `CommandSession`, the same validator, the same
   journal — not a mock, not a fixture, not a second implementation.

   What it is NOT: a shipped surface. The app serves its assets over Tauri's
   custom protocol and creates no server (§4.10); this listens on a dev port for
   a developer, and nothing in `src-tauri` or the bundle references it.

     node tools/web-ipc.mjs --plan /tmp/plan1 [--port 4399] [--dist ui/dist]
     → http://localhost:4399

   `--shell-plan none` is the **fresh machine** (Appendix C.5's `noneExists`):
   `/shell_info` reports no remembered plan, so the page's boot asks the engine
   without one and the first run draws. Point `--plan` at the path a first run
   would create, and the press under test writes a real plan there. Omitted,
   `--shell-plan` defaults to `--plan` — the shell remembering a plan.
   ══════════════════════════════════════════════════════════════════════════ */

import { execFile } from 'node:child_process';
import { createReadStream, existsSync, readdirSync, statSync, watch } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

/** D9's debounce window: the same 150 ms the engine's watcher uses. */
const DEBOUNCE_MS = 150;

function argument(name, fallback = null) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback;
}

const plan = argument('--plan');
/** What `/shell_info` reports: the plan the *shell* remembers, which is `null`
    on a machine that has never opened one — the first run's own state. */
const shellPlan = argument('--shell-plan', plan) === 'none' ? null : argument('--shell-plan', plan);
const port = Number(argument('--port', '4399'));
const dist = join(ROOT, argument('--dist', 'ui/dist'));
const sam = join(ROOT, 'target/release/sam');
const resources = join(ROOT, 'Sources/SAMCore/Resources');

if (!plan) {
  console.error('web-ipc: --plan <directory> is required (the harness edits a real plan)');
  process.exit(2);
}
if (!existsSync(sam)) {
  console.error(`web-ipc: ${sam} is missing — run \`cargo build --workspace --release\` first`);
  process.exit(1);
}

/**
 * A stale binary is this harness's quietest failure. Every dispatch below goes
 * through `sam invoke`, so a release built before the engine's last edit
 * answers with the registry it had then: a write refuses, or lands in an older
 * shape, while the page reports what the old engine said — and every probe
 * that reads the plan afterwards blames the app. Found the hard way (writing
 * through the figure's sheet against a binary forty minutes behind the tree,
 * BUILDLOG #45). The newest `.rs` under `crates/` is the line the binary must
 * not predate; say so out loud rather than serve it silently.
 */
const newestEngineSource = (() => {
  let newest = 0;
  let where = null;
  for (const entry of readdirSync(join(ROOT, 'crates'), { recursive: true })) {
    if (!entry.endsWith('.rs')) continue;
    const { mtimeMs } = statSync(join(ROOT, 'crates', entry));
    if (mtimeMs > newest) {
      newest = mtimeMs;
      where = entry;
    }
  }
  return { newest, where };
})();
const binaryBuilt = statSync(sam).mtimeMs;
if (binaryBuilt < newestEngineSource.newest) {
  console.error(
    `web-ipc: ${sam} predates the engine sources — newest is crates/${newestEngineSource.where} at ` +
      `${new Date(newestEngineSource.newest).toISOString()}, the binary at ${new Date(binaryBuilt).toISOString()}. ` +
      'Dispatches served now go into the old engine; run `cargo build --release -p sam-cli` first.',
  );
}

/**
 * The registry ids the engine exposes. Cached, but **refreshable**
 * and **per plan**: the per-type pair (`<type>.new` / `<type>.paste`)
 * appears the moment a plan root does, and a cache that was listed
 * before the plan existed would refuse the id the app just learned
 * about — the bridge would be the one place a UI-created type could
 * not be used. The plan the *page* is on travels with every
 * dispatch, so the listing reads it: a first run creates its plan
 * mid-session, and the door that writes the first course dispatches
 * the first per-type command the registry has ever had. (The shipped
 * shell has no such list — it forwards and lets the engine decide —
 * so this is the bridge's own gate, and it must not be plan-blind.)
 */
const registries = new Map();
async function commandIds(planPath = null, refresh = false) {
  const key = planPath ?? '';
  const cached = registries.get(key);
  if (cached && !refresh) return cached;
  const args = ['--commands', '--json'];
  if (planPath && existsSync(planPath)) args.push('--plan', planPath);
  let data;
  try {
    data = await runSam(args);
  } catch {
    // A plan path that does not list (a plan that does not load)
    // falls back to the base registry: the command may still be a
    // base command, and the engine answers the invoke either way.
    data = await runSam(['--commands', '--json']);
  }
  const ids = new Set();
  const walk = (value) => {
    if (Array.isArray(value)) return value.forEach(walk);
    if (value && typeof value === 'object') {
      if (typeof value.id === 'string') ids.add(value.id);
      Object.values(value).forEach(walk);
    }
  };
  walk(data);
  registries.set(key, ids);
  return ids;
}

/** One `sam invoke -` run: the payload on stdin, the result on stdout. */
function runSam(args, stdin = '') {
  return new Promise((resolve, reject) => {
    const child = execFile(
      sam,
      args,
      { cwd: ROOT, env: { ...process.env, SAM_RESOURCES: resources }, maxBuffer: 32 * 1024 * 1024 },
      (error, stdout, stderr) => {
        const text = stdout.trim();
        const diagnostic = asDiagnostic(text) ?? asDiagnostic(stderr.trim());
        // A failed command answers with the engine's diagnostic — on stdout for
        // `invoke`, on stderr for a command that never reached the dispatcher —
        // and exits non-zero. That is a rejection carrying the diagnostic, not a
        // result: a page that received `{"code":"io.failure",…}` as a success
        // would render a diagnostic as data.
        if (error || !text) {
          if (!text && !diagnostic) return reject(new Error(stderr.trim() || String(error)));
          if (diagnostic) {
            const failure = new Error(diagnostic.message);
            failure.diagnostic = diagnostic;
            return reject(failure);
          }
        }
        try {
          resolve(JSON.parse(text));
        } catch {
          reject(new Error(text.slice(0, 400) || stderr.trim()));
        }
      },
    );
    child.stdin.end(stdin);
  });
}

/** The first engine diagnostic in a text stream, when there is one. */
function asDiagnostic(text) {
  if (!text) return null;
  const start = text.indexOf('{');
  if (start < 0) return null;
  const end = text.lastIndexOf('}');
  if (end <= start) return null;
  try {
    const parsed = JSON.parse(text.slice(start, end + 1));
    if (
      parsed &&
      typeof parsed === 'object' &&
      typeof parsed.code === 'string' &&
      typeof parsed.message === 'string'
    ) {
      return parsed;
    }
  } catch {
    /* not a diagnostic */
  }
  return null;
}

/** The design system's vocabulary, not Node's (§4.8: `data-os`). */
function osLabel() {
  if (process.platform === 'darwin') return 'mac';
  if (process.platform === 'win32') return 'win';
  return 'linux';
}

/**
 * D9's reload path, dev transport. The shell watches through `sam-core`'s
 * `ConfigWatcher` and emits one Tauri event per accepted revision; the harness
 * watches the same tree and pushes **one** `plan-changed` Server-Sent Event per
 * accepted revision, so the frontend's subscription code is identical and a
 * 500-line write is still one reload.
 */
const watchers = new Set();
let lastRevision = null;
let revisionPending = null;

async function planRevision(planPath = plan) {
  const envelope = await runSam(['paths', '--plan', planPath, '--json']);
  return { revision: envelope?.revision ?? null, plan: envelope?.data?.plan ?? planPath };
}

function broadcast(event) {
  const payload = `data: ${JSON.stringify(event)}\n\n`;
  for (const client of watchers) {
    try {
      client.write(payload);
    } catch {
      // A closed stream is not an error worth dying for: drop the client and
      // let the next connection re-subscribe.
      watchers.delete(client);
    }
  }
}

function watchPlan() {
  let timer = null;
  /**
   * A debounce, not a one-shot: every event resets the window, and the timer is
   * **cleared** when it fires. Leaving it latched is the bug that makes a watcher
   * publish once and then go deaf — the failure looks like "the app only reloads
   * the first time", never like a watcher problem.
   */
  const check = async () => {
    timer = null;
    try {
      const { revision } = await planRevision();
      if (revision && revision !== lastRevision) {
        lastRevision = revision;
        broadcast({ revision, plan, valid: true });
        console.log(`· revision ${String(revision).slice(0, 8)}`);
      }
    } catch (error) {
      // An invalid save is still news: the app reloads and reports the finding
      // rather than sitting on a stale screen (Appendix C.5's `invalid` phase).
      broadcast({ revision: null, plan, valid: false, message: String(error.message ?? error) });
    }
  };
  const nudge = () => {
    clearTimeout(timer);
    timer = setTimeout(() => void check().catch((error) => console.error(`web-ipc: ${error}`)), DEBOUNCE_MS);
  };
  try {
    const watcher = watch(plan, { recursive: true }, nudge);
    // The plan's parent is watched too, so deleting and recreating the plan
    // directory still triggers a reload (D9, linearmouse PR #1209).
    const parent = watch(dirname(plan), { recursive: false }, nudge);
    process.on('exit', () => {
      watcher.close();
      parent.close();
    });
  } catch (error) {
    console.error(`web-ipc: cannot watch ${plan}: ${error.message}`);
  }
}

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
};

function body(response, status, text, type = 'application/json; charset=utf-8') {
  response.writeHead(status, {
    'content-type': type,
    'access-control-allow-origin': '*',
    'cache-control': 'no-store',
  });
  response.end(text);
}

/** The page, with the bridge injected: the SAME app, told where the engine is. */
async function indexHtml() {
  const file = join(dist, 'index.html');
  if (!existsSync(file)) {
    return `<h1>ui/dist is missing</h1><p>Run <code>pnpm -C ui build</code> first.</p>`;
  }
  const html = await readFile(file, 'utf8');
  const injected = `<script>window.__SAM_WEB__ = { endpoint: location.origin };</script>`;
  return html.replace('</head>', `${injected}\n</head>`);
}

/**
 * The `/ipc` door. It is a function rather than an inline branch because the
 * whole route must survive a failure *before* a command runs: the engine's own
 * refusal for a request that carries a plan root which does not exist arrives
 * here, and an unhandled rejection used to leave the page's fetch pending
 * forever (found by pointing the harness at a missing plan to reach the
 * picker). An error still answers with the engine's own diagnostic shape, which
 * is what `IpcError` reads.
 */
async function ipc(request, response) {
  if (request.method !== 'POST') return body(response, 405, JSON.stringify({ code: 'ipc.method', path: '/ipc', line: null, message: 'POST only' }));
  let payload = '';
  for await (const chunk of request) payload += chunk;
  let parsed;
  try {
    parsed = JSON.parse(payload || '{}');
  } catch (error) {
    return body(response, 400, JSON.stringify({ code: 'ipc.payload', path: '/ipc', line: null, message: `not JSON: ${error}` }));
  }
  const { id, params = {}, ifRevision = null, plan: requested = null } = parsed ?? {};
  if (typeof id !== 'string' || !id) {
    return body(response, 400, JSON.stringify({ code: 'ipc.payload', path: '/ipc', line: null, message: 'an "id" is required' }));
  }
  // The plan the page is on, straight through (the same rule the
  // invoke below follows): the bridge never carries its own.
  const target = typeof requested === 'string' && requested.length > 0 ? requested : null;
  // Forward ONLY registry ids: the bridge must not become a general shell.
  // One refresh on a miss, because the registry grows during a session —
  // and the listing is per plan, because the per-type pair is born when
  // the plan is (a first run creates both in one session).
  if (
    !(await commandIds(target)).has(id)
    && !(await commandIds(target, true)).has(id)
  ) {
    return body(response, 404, JSON.stringify({ code: 'ipc.unknown-id', path: '/ipc', line: null, message: `unknown command id "${id}" — SAM --commands lists the registry` }));
  }
  // The revision pin travels with the request (§4.9: a stale write is a
  // conflict, not a retry) — a bridge that dropped it would let the web route
  // pass tests the shipped shell fails, and vice versa.
  const envelope = JSON.stringify({ id, params });
  const args = target ? ['invoke', '-', '--plan', target, '--json'] : ['invoke', '-', '--json'];
  if (typeof ifRevision === 'string' && ifRevision.length > 0) args.push('--if-revision', ifRevision);
  try {
    const result = await runSam(args, envelope);
    const first = Array.isArray(result?.results) ? result.results[0] : result;
    return body(response, 200, JSON.stringify(first ?? {}));
  } catch (error) {
    // The engine's own diagnostic travels through, so the page builds an
    // `IpcError` a person can read instead of a JSON blob inside a message.
    const diagnostic = error?.diagnostic ?? {
      code: 'engine.error',
      path: '/ipc',
      line: null,
      message: String(error.message ?? error),
    };
    return body(response, 400, JSON.stringify(diagnostic));
  }
}

const server = createServer(async (request, response) => {
  const url = new URL(request.url ?? '/', `http://localhost:${port}`);

  if (url.pathname === '/ipc') {
    try {
      await ipc(request, response);
    } catch (error) {
      // A command that cannot even be listed — a plan root that does not exist,
      // say — must still answer, or the page waits forever on a fetch and the
      // dev route stops being a route.
      if (response.headersSent) return;
      const diagnostic = error?.diagnostic ?? {
        code: 'engine.error',
        path: '/ipc',
        line: null,
        message: String(error?.message ?? error),
      };
      return body(response, 400, JSON.stringify(diagnostic));
    }
    return;
  }

  if (url.pathname === '/shell_info') {
    return body(
      response,
      200,
      JSON.stringify({
        os: osLabel(),
        resourcesDir: resources,
        plansDir: dirname(plan),
        dataDir: '',
        indexesDir: '',
        // What the shell remembers. `--plan` by default — *"the shell was
        // started for this plan"*, which the page opens at boot; `null` with
        // `--shell-plan none`, which is a machine that has never opened one,
        // and is the only way the page reaches Appendix C.5's `noneExists`.
        plan: shellPlan,
      }),
    );
  }

  // The reload subscription the shell provides as a Tauri event. One event per
  // accepted revision, debounced — the same contract, a different transport.
  if (url.pathname === '/events') {
    response.writeHead(200, {
      'content-type': 'text/event-stream',
      'cache-control': 'no-store',
      connection: 'keep-alive',
      'access-control-allow-origin': '*',
    });
    response.write(': connected\n\n');
    watchers.add(response);
    request.on('close', () => watchers.delete(response));
    // The initial frame describes the plan *being watched*. With no plan root
    // there is nothing to describe: the shell does not watch a plan it never
    // opened, and announcing "the plan on disk does not load" for a path that
    // was never opened put the picker's diagnostic on the first run's screen.
    if (!existsSync(plan)) return;
    planRevision()
      .then(({ revision }) => {
        lastRevision = revision;
        try {
          response.write(`data: ${JSON.stringify({ revision, plan, valid: true })}\n\n`);
        } catch {
          watchers.delete(response);
        }
      })
      .catch(() => {
        try {
          response.write(`data: ${JSON.stringify({ revision: null, plan, valid: false })}\n\n`);
        } catch {
          watchers.delete(response);
        }
      });
    return;
  }

  if (url.pathname === '/plan') {
    return body(response, 200, JSON.stringify({ plan }));
  }

  if (url.pathname === '/publish') {
    // The page tells the harness which plan it just wrote, so the
    // harness re-reads that plan's revision now instead of waiting
    // for the watcher's debounce. One reload per accepted commit (D9),
    // whether the write came from the page or from git. The plan
    // travels because a first run's plan is born mid-session — a
    // harness started for a plan that does not exist yet can only
    // re-read the plan the page names.
    let payload = '';
    for await (const chunk of request) payload += chunk;
    let written = null;
    try {
      written = JSON.parse(payload || '{}').plan ?? null;
    } catch {
      /* no plan named: the harness's own is what it re-reads */
    }
    planRevision(written ?? undefined)
      .then(({ revision, plan: writtenPlan }) => {
        if (revision && revision !== lastRevision) {
          lastRevision = revision;
          broadcast({ revision, plan: writtenPlan, valid: true });
        }
        return body(response, 200, JSON.stringify({ ok: true, revision }));
      })
      .catch((error) => body(response, 500, JSON.stringify({ ok: false, message: String(error.message ?? error) })));
    return;
  }

  if (url.pathname === '/' || url.pathname === '/index.html') {
    // Never cached. The page names hashed assets, so a cached copy of it points
    // the browser at the *previous* build — a failure that reads as "my change
    // did nothing" and costs an hour to find.
    response.writeHead(200, {
      'content-type': TYPES['.html'],
      'cache-control': 'no-store, must-revalidate',
    });
    return response.end(await indexHtml());
  }

  // Static, confined to dist: a traversal attempt is a 404, not a read.
  const candidate = normalize(join(dist, decodeURIComponent(url.pathname)));
  if (!candidate.startsWith(dist) || !existsSync(candidate) || !statSync(candidate).isFile()) {
    return body(response, 404, 'not found', 'text/plain; charset=utf-8');
  }
  response.writeHead(200, { 'content-type': TYPES[extname(candidate)] ?? 'application/octet-stream' });
  createReadStream(candidate).pipe(response);
});

// A dev harness must survive its own diagnostics: an engine that refuses to
// load a plan is news to send the page, not a reason to stop serving.
process.on('unhandledRejection', (reason) => {
  console.error(`web-ipc: unhandled rejection — ${reason}`);
});

server.listen(port, () => {
  console.log(`web harness · http://localhost:${port} · plan ${plan}`);
  console.log(`  the page dispatches the registry through \`sam invoke\`, so it is the real engine`);
  console.log(`  /events streams one reload per accepted revision (D9: watch · hash · debounce)`);
  watchPlan();
});
