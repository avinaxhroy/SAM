/**
 * IPC gateway to the core engine (D19, §4.2, §4.10, §7).
 * Dispatches `{ id, params }` over Tauri IPC in desktop builds or HTTP in browser harnesses.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { safeUrl } from './safeUrl';

/** One dispatch result, exactly as `sam invoke` reports it. */
export type DispatchResult = {
  ok: boolean;
  /** Write: the revision the plan sits at now, and the transaction that got it there. */
  revision?: string;
  txid?: string;
  summary?: string;
  files?: string[];
  dryRun?: boolean;
  /** Read: the projection (`view`, `uicheck`, `schema`, `paths` …). */
  data?: unknown;
  /** Presentation: where it would have gone, and that this context has no GUI. */
  destination?: string;
  available?: boolean;
  reason?: string;
};

/** The engine's structured error (§4.9: `code`, `path`, `line`, `message`). */
export type Diagnostic = {
  code: string;
  path: string;
  line: number | null;
  message: string;
  /** Present on validator findings (`error` / `warning`); absent elsewhere. */
  severity?: string;
};

export class IpcError extends Error {
  readonly diagnostic: Diagnostic;
  /**
   * Every finding the engine answered with. A refused write and a plan that
   * does not load both arrive as a *list* of diagnostics (§4.9: each finding
   * carries its own path and line); the first is the head the one-line form
   * shows, and the rest belong to the surface that can list them — never to a
   * toast that would print raw JSON at the student.
   */
  readonly diagnostics: Diagnostic[];

  constructor(diagnostics: Diagnostic | Diagnostic[]) {
    const list = Array.isArray(diagnostics) ? diagnostics : [diagnostics];
    const primary = list[0];
    super(primary.message);
    this.name = 'IpcError';
    this.diagnostic = primary;
    this.diagnostics = list;
  }

  /**
   * The one-line form an inline error, a toast and a pane marker all show. When
   * the engine answered with a list, the line says how many findings wait
   * behind it — the surface that can show them all is the fault card, not this
   * sentence.
   */
  get text(): string {
    const one =
      this.diagnostic.line === null
        ? `${this.diagnostic.message} [${this.diagnostic.code}]`
        : `${this.diagnostic.path}:${this.diagnostic.line} — ${this.diagnostic.message} [${this.diagnostic.code}]`;
    return this.diagnostics.length > 1 ? `${one} (+${this.diagnostics.length - 1} more)` : one;
  }
}

type TauriBridge = {
  core: { invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown> };
  event?: { listen: (name: string, handler: (event: { payload: unknown }) => void) => Promise<() => void> };
  window?: { getCurrentWindow: () => { close: () => void } };
};

type WebBridge = { endpoint: string };

/**
 * The shell no longer injects the full `__TAURI__` API (969 KB parsed before
 * the app bundle). It injects `__TAURI_INTERNALS__`; these four members are all
 * the page uses, so the bridge below is built from the tree-shaken API package
 * and installed under the old name, keeping every call site and `transport()`
 * unchanged.
 */
const tauriScope = globalThis as {
  __TAURI__?: TauriBridge;
  __TAURI_INTERNALS__?: unknown;
  __SAM_WEB__?: WebBridge;
};
if (tauriScope.__TAURI_INTERNALS__ && !tauriScope.__TAURI__) {
  tauriScope.__TAURI__ = {
    core: {
      invoke: (command, args) => invoke(command, args),
    },
    event: {
      listen: (name, handler) =>
        listen(name, (event) => handler({ payload: event.payload })),
    },
    window: {
      getCurrentWindow: () => ({
        close: () => {
          void getCurrentWindow().close();
        },
      }),
    },
  };
}

function scope(): { __TAURI__?: TauriBridge; __SAM_WEB__?: WebBridge } {
  return tauriScope;
}

/**
 * Which transport this page has. The browser harness sets `window.__SAM_WEB__`
 * before the app mounts; the shell injects `__TAURI__` with `withGlobalTauri`.
 */
export function transport(): 'tauri' | 'web' | 'none' {
  if (scope().__TAURI__) return 'tauri';
  if (scope().__SAM_WEB__) return 'web';
  return 'none';
}

export function endpoint(): string | null {
  // The web harness serves the page and its IPC bridge from one origin; a
  // pointer anywhere else means plan content and write commands would leave
  // the origin that vouches for them. Refuse rather than follow.
  const endpoint = scope().__SAM_WEB__?.endpoint;
  if (!endpoint) return null;
  try {
    return new URL(endpoint, location.href).origin === location.origin ? endpoint : null;
  } catch {
    return null;
  }
}

/** Run one registry command. The verb IS the registry id (§4.9). */
export async function dispatch(
  id: string,
  params: Record<string, unknown> = {},
  options: { plan?: string | null; ifRevision?: string | null } = {},
): Promise<DispatchResult> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    // A command error arrives as a string carrying the engine's own envelope.
    const raw = await tauri.core
      .invoke('dispatch', {
        id,
        params,
        plan: options.plan ?? null,
        ifRevision: options.ifRevision ?? null,
      })
      .catch((error: unknown) => {
        throw new IpcError(parseDiagnostics(error));
      });
    return raw as DispatchResult;
  }

  const base = endpoint();
  if (transport() === 'web' && base) {
    const response = await fetch(`${base}/ipc`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id, params, plan: options.plan ?? null, ifRevision: options.ifRevision ?? null }),
    });
    const body = (await response.json()) as DispatchResult | Diagnostic;
    if (!response.ok) throw new IpcError(body as Diagnostic);
    return body as DispatchResult;
  }

  throw new IpcError({
    code: 'ipc.unavailable',
    path: '-',
    line: null,
    message: 'no shell and no web bridge — this page has no engine behind it',
  });
}

/**
 * Engine errors reach the frontend as JSON text inside whatever the transport
 * wraps them in (a Rust `Err(String)`, an HTTP body, an exception message).
 * A refused plan carries **every finding**, one JSON object per line, so the
 * reader walks the text for balanced objects instead of trusting one slice of
 * it: the whole diagnostics list is what lets a fault card say "12 findings"
 * rather than a toast pasting the raw stream at the student. Anything that is
 * not an engine object falls back to the raw text so nothing is swallowed.
 */
function parseDiagnostics(error: unknown): Diagnostic[] {
  const text =
    typeof error === 'string'
      ? error
      : error instanceof Error
        ? error.message
        : String(error);
  const found: Diagnostic[] = [];
  let index = 0;
  while (index < text.length) {
    const start = text.indexOf('{', index);
    if (start < 0) break;
    const end = balancedObjectEnd(text, start);
    if (end < 0) break;
    index = end;
    try {
      const parsed = JSON.parse(text.slice(start, end)) as Partial<Diagnostic>;
      if (typeof parsed.message === 'string') {
        found.push({
          code: parsed.code ?? 'ipc.error',
          path: parsed.path ?? '-',
          line: parsed.line ?? null,
          message: parsed.message,
          ...(typeof parsed.severity === 'string' ? { severity: parsed.severity } : {}),
        });
      }
    } catch {
      /* a brace that is not JSON — keep walking */
    }
  }
  if (found.length > 0) return found;
  return [{ code: 'ipc.error', path: '-', line: null, message: text }];
}

/** The index just past the JSON object opening at `start`, or -1 when it never closes. */
function balancedObjectEnd(text: string, start: number): number {
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let at = start; at < text.length; at += 1) {
    const character = text[at];
    if (inString) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') inString = false;
      continue;
    }
    if (character === '"') inString = true;
    else if (character === '{') depth += 1;
    else if (character === '}') {
      depth -= 1;
      if (depth === 0) return at + 1;
    }
  }
  return -1;
}

/** The OS label the shell reports, or the page's floor when there is no shell. */
export type ShellInfo = {
  os: 'mac' | 'win' | 'linux';
  resourcesDir: string;
  plansDir: string;
  dataDir: string;
  indexesDir: string;
  plan?: string | null;
  /** §9/D21: the renderer path this launch took; "native" = no override. */
  rendererPath?: string;
  /**
   * Whether window controls overlay web content (`titleBarStyle: "Overlay"`).
   * Drives `--titlebar-inset-mac` and `--titlebar-inset-win`.
   */
  titlebarOverlay?: boolean;
};

export async function shellInfo(): Promise<ShellInfo | null> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      return (await tauri.core.invoke('shell_info')) as ShellInfo;
    } catch {
      return null;
    }
  }
  const base = endpoint();
  if (transport() === 'web' && base) {
    try {
      const response = await fetch(`${base}/shell_info`);
      if (!response.ok) return null;
      return (await response.json()) as ShellInfo;
    } catch {
      return null;
    }
  }
  return null;
}

/**
 * Hand one `http(s)` address to the platform's browser (§4.10).
 *
 * The shell owns the act: the packaged app installs no opener plugin, so a
 * page's `window.open` or a `target=_blank` anchor has nothing to catch it and
 * a link drawn that way is a button that does nothing. The browser harness has
 * no shell, so it uses the browser's own tab. The scheme is checked here as well
 * as in the shell: every link press funnels through this one function.
 */
export async function openExternal(url: string): Promise<boolean> {
  const text = url.trim();
  if (!safeUrl(text)) return false;
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      await tauri.core.invoke('open_external', { url: text });
      return true;
    } catch {
      return false;
    }
  }
  window.open(text, '_blank', 'noopener');
  return true;
}

/** One plan-changed publication (D9): the revision, or why it is not valid. */
export type PlanChanged = { revision: string | null; plan: string; valid: boolean; message?: string };

/** Subscribe to plan changes across Tauri events or web SSE (D9). */
export function onPlanChanged(handler: (event: PlanChanged) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    void tauri.event
      .listen('plan-changed', (event) => handler(event.payload as PlanChanged))
      .then((off) => {
        unlisten = off;
      });
    return () => unlisten?.();
  }

  const base = endpoint();
  if (transport() === 'web' && base) {
    const source = new EventSource(`${base}/events`);
    source.onmessage = (event) => {
      try {
        handler(JSON.parse(event.data) as PlanChanged);
      } catch {
        /* a malformed frame is not a plan change */
      }
    };
    return () => source.close();
  }
  return () => {};
}

/** Notify web bridge to bypass watcher debounce on write. */
export async function publishWrite(plan: string | null = null): Promise<void> {
  const base = endpoint();
  if (transport() === 'web' && base) {
    try {
      await fetch(`${base}/publish`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ plan }),
      });
    } catch {
      /* the watcher will catch it anyway */
    }
  }
}

/** One item of a context menu: an id to dispatch, or a separator. */
export type MenuItemSpec = { id: string; title: string; enabled?: boolean } | { separator: true };

/** Present native context menu if supported; returns false when DOM fallback is needed (§4.10). */
export async function popupMenu(items: MenuItemSpec[]): Promise<boolean> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      await tauri.core.invoke('popup_menu', { items });
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

/** Subscribe to menu choice events from native menus (§4.8 Principle 2). */
export function onMenuChoice(handler: (id: string) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    void tauri.event
      .listen('menu', (event) => handler(String(event.payload ?? '')))
      .then((off) => {
        unlisten = off;
      });
    return () => unlisten?.();
  }
  return () => {};
}

/**
 * A file asked for by the Preferences window (§4.10). Preferences is a window
 * whose whole content is Settings, so it has no source pane: its file doors ask
 * the shell, and the shell hands the path to the window that draws the pane.
 * Main window only — the shell addresses `main` explicitly.
 */
export function onOpenSource(handler: (file: string) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    void tauri.event
      .listen('open-source', (event) => handler(String(event.payload ?? '')))
      .then((off) => {
        unlisten = off;
      });
    return () => unlisten?.();
  }
  return () => {};
}

/**
 * Open a file in the **main** window's source pane. Returns `true` when a live
 * window took it; `false` means there is no shell here (the browser renders
 * Settings in the canvas, where `openFile`'s own state is enough).
 */
export async function openSourceInMainWindow(file: string): Promise<boolean> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      return (await tauri.core.invoke('open_source', { file })) === true;
    } catch {
      return false;
    }
  }
  return false;
}

/** Close this window — the Preferences window's *Done* (§4.10). */
export function closeWindow(): void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.window) {
    tauri.window.getCurrentWindow().close();
  }
}

/**
 * One app preference written in one window and needed by the other. The keys are
 * the stores that live in the page rather than the plan: `component-styles`
 * carries the per-component designs and the interface size as one snapshot,
 * `theme` carries the light/dark choice.
 */
export type PreferenceChange = { key: 'component-styles' | 'theme'; value: string };

const PREFERENCE_CHANNEL = 'sam.preferences';

/**
 * Hand a preference to the window that draws it (§4.10). Preferences live in the
 * webview's own `localStorage`, and the Preferences window is a **second
 * webview** with its own copy of every store: a design chosen there changed the
 * copy in that window and nothing else, so the app behind kept the old design
 * until a relaunch re-read the store. Desktop crosses on the shell's event bus;
 * a browser window has no shell, so the same message goes over a
 * `BroadcastChannel` — the harness can drive it, and two web windows agree too.
 */
export function sendPreference(change: PreferenceChange): void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    void tauri.core.invoke('publish_preference', change).catch(() => {
      /* the window that made the choice still has it */
    });
    return;
  }
  if (transport() === 'web') {
    try {
      const channel = new BroadcastChannel(PREFERENCE_CHANNEL);
      channel.postMessage(change);
      channel.close();
    } catch {
      /* no channel here: this window's own store already has the choice */
    }
  }
}

/** The other transport of `sendPreference` — the window that draws it listens. */
export function onPreferenceChange(handler: (change: PreferenceChange) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    void tauri.event
      .listen('preference-changed', (event) => {
        const payload = event.payload as PreferenceChange | null;
        if (payload && typeof payload.key === 'string' && typeof payload.value === 'string') {
          handler(payload);
        }
      })
      .then((off) => {
        unlisten = off;
      });
    return () => unlisten?.();
  }
  if (transport() === 'web') {
    let channel: BroadcastChannel | null = null;
    try {
      channel = new BroadcastChannel(PREFERENCE_CHANNEL);
      channel.onmessage = (event) => {
        const payload = event.data as PreferenceChange | null;
        if (payload && typeof payload.key === 'string' && typeof payload.value === 'string') {
          handler(payload);
        }
      };
    } catch {
      channel = null;
    }
    return () => channel?.close();
  }
  return () => {};
}

/** A second launch's arguments, routed to the running window (§4.10). */
export function onLaunch(handler: (argv: string[]) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    void tauri.event
      .listen('launch', (event) => handler((event.payload as string[]) ?? []))
      .then((off) => {
        unlisten = off;
      });
    return () => unlisten?.();
  }
  return () => {};
}

/** Ask the shell to watch the plan root and publish one event per revision (D9). */
export async function watchPlan(plan: string): Promise<void> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      await tauri.core.invoke('watch_plan', { plan });
    } catch {
      /* the app still re-reads on its own writes */
    }
  }
}

/** Stop watching a plan the window has left (`app.plan` became null). */
export async function unwatchPlan(): Promise<void> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      await tauri.core.invoke('unwatch_plan');
    } catch {
      /* the next watch replaces it anyway */
    }
  }
}

/**
 * Tell the shell which plan folder this window loaded (R11). The shell reports
 * it back in `shell_info` on the next launch, so the app opens the folder the
 * student was working in rather than whichever plan the plans root resolves.
 * Called once the plan has read, never for a folder that only resolved a root.
 */
export async function rememberPlan(plan: string): Promise<void> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      await tauri.core.invoke('remember_plan', { plan });
    } catch {
      /* the folder list still remembers it for this window's session */
    }
  }
}

/**
 * One event from the running agent turn (§4.7 through the deck), exactly as
 * `sam_core::agent::AgentEvent` serializes it, tagged with the run id the
 * start call returned. `exit` is the last event of every run.
 */
export type AgentEvent =
  | { runId: string; kind: 'started'; agent: string; bin: string }
  | { runId: string; kind: 'session'; id: string }
  | { runId: string; kind: 'text' | 'thinking'; text: string }
  | { runId: string; kind: 'tool'; id: string; name: string; summary: string }
  | { runId: string; kind: 'file'; path: string }
  | { runId: string; kind: 'toolResult'; toolUseId: string; isError: boolean; content: string }
  | { runId: string; kind: 'result'; summary: string }
  | { runId: string; kind: 'error'; message: string }
  | { runId: string; kind: 'log' | 'stderr'; line: string }
  | { runId: string; kind: 'exit'; code: number };

export type AgentStart = { runId: string; agent: string; plan: string };

/** One model row from `agent.models` (`source` says live list vs hints). */
export type AgentModel = { id: string; label: string };
export type AgentModels = { agent: string; source: 'live' | 'fallback' | 'profile' | 'none'; models: AgentModel[] };

/**
 * What one CLI says its models are. Asked lazily — `opencode models` and
 * `agy models` are network round-trips — and answered from the adapter's
 * documented hints when the CLI has no listing command.
 */
export async function agentModels(agentId: string | null): Promise<AgentModels> {
  const result = await dispatch('agent.models', agentId ? { agent: agentId } : {});
  return result.data as AgentModels;
}

/**
 * Start one assistant turn. Desktop only on purpose: the CLIs it drives live
 * on the user's machine, which a browser harness cannot reach — the deck
 * renders the honest "runs in the desktop app" blocker instead of failing late.
 */
export async function agentStart(options: {
  agentId?: string | null;
  model?: string | null;
  resume?: string | null;
  context?: string | null;
  prompt: string;
  plan: string;
}): Promise<AgentStart> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      return (await tauri.core.invoke('agent_start', {
        agentId: options.agentId ?? null,
        model: options.model ?? null,
        resume: options.resume ?? null,
        context: options.context ?? null,
        prompt: options.prompt,
        plan: options.plan,
      })) as AgentStart;
    } catch (error) {
      throw new IpcError(parseDiagnostics(error));
    }
  }
  throw new IpcError({
    code: 'ipc.unavailable',
    path: '-',
    line: null,
    message: 'the assistant runs the coding agents installed on this machine — open the desktop app',
  });
}

/** Stop the running turn (the deck's Stop). `false` = nothing was running. */
export async function agentCancel(): Promise<boolean> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      return (await tauri.core.invoke('agent_cancel')) === true;
    } catch {
      return false;
    }
  }
  return false;
}

/** One line of the CLI-install answer (`install_cli`). */
export type CliInstall = {
  installed: boolean;
  needsElevation?: boolean;
  elevated?: boolean;
  action?: string;
  path?: string;
  target?: string | null;
  command?: string;
};

/**
 * Put the bundled `sam` on this machine's PATH (§4.10's machine seam): the
 * View menu's item and the palette's `app.installCli` row. The desktop's shell
 * owns the platform's own permission ask; a browser has nothing to link.
 */
export async function installCli(remove = false): Promise<CliInstall> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      return (await tauri.core.invoke('install_cli', { remove })) as CliInstall;
    } catch (error) {
      throw new IpcError(parseDiagnostics(error));
    }
  }
  throw new IpcError({
    code: 'ipc.unavailable',
    path: '-',
    line: null,
    message: 'installing the command line tool needs the desktop app',
  });
}

/** Subscribe to the running turn's event stream. Desktop only, like `agentStart`. */
export function onAgentEvent(handler: (event: AgentEvent) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    // A subscription that is torn down before `listen` resolves must not leak:
    // the returned `off` would be stored after the fact and never called, and
    // the next mount would handle every event twice.
    let cancelled = false;
    void tauri.event
      .listen('agent-event', (event) => handler(event.payload as AgentEvent))
      .then((off) => {
        if (cancelled) off();
        else unlisten = off;
      })
      .catch((error) => {
        // A stream that never subscribes would leave every run at "Working"
        // with no rows and no exit: say it in the console the window already
        // keeps, rather than failing silently.
        console.warn('the agent event stream could not subscribe', error);
      });
    return () => {
      cancelled = true;
      unlisten?.();
      unlisten = null;
    };
  }
  return () => {};
}

/**
 * Open Preferences as an OS window (§4.10). Returns `true` when a real window
 * took it — false means "render it here", which is what the browser does.
 */
export async function openPreferences(): Promise<boolean> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      const opened = await tauri.core.invoke('open_preferences');
      return opened === true;
    } catch {
      return false;
    }
  }
  return false;
}

/** Native open dialog for the plan picker (§4.10). `null` = no dialog here. */
export async function pickDirectory(): Promise<string | null> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      const chosen = await tauri.core.invoke('pick_directory');
      return typeof chosen === 'string' ? chosen : null;
    } catch {
      return null;
    }
  }
  return null;
}

/**
 * A file the OS opened by its association (macOS `RunEvent::Opened`), or a
 * Windows/Linux double-click that the shell resolved to the same event. The
 * payload is the file path; the app runs the import flow, never a page load.
 */
export function onOpenProfile(handler: (path: string) => void): () => void {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri?.event) {
    let unlisten: (() => void) | null = null;
    void tauri.event
      .listen('open-profile', (event) => handler(String(event.payload ?? '')))
      .then((off) => {
        unlisten = off;
      });
    return () => unlisten?.();
  }
  return () => {};
}

/**
 * An association launch that arrived before this window existed. macOS can
 * deliver the open event first; the shell keeps the path until the page asks.
 */
export async function takePendingOpen(): Promise<string[]> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      const pending = await tauri.core.invoke('take_pending_open');
      return Array.isArray(pending) ? (pending as string[]) : [];
    } catch {
      return [];
    }
  }
  return [];
}

/** The OS's own save dialog for a profile export (§4.10). `null` = no dialog. */
export async function pickProfileOut(name?: string): Promise<string | null> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      const chosen = await tauri.core.invoke('pick_profile_out', { name: name ?? null });
      return typeof chosen === 'string' ? chosen : null;
    } catch {
      return null;
    }
  }
  return null;
}

/** The OS's own open dialog for a profile import. `null` = no dialog here. */
export async function pickProfileFile(): Promise<string | null> {
  const tauri = scope().__TAURI__;
  if (transport() === 'tauri' && tauri) {
    try {
      const chosen = await tauri.core.invoke('pick_profile_file');
      return typeof chosen === 'string' ? chosen : null;
    } catch {
      return null;
    }
  }
  return null;
}
