/**
 * IPC gateway to the core engine (D19, §4.2, §4.10, §7).
 * Dispatches `{ id, params }` over Tauri IPC in desktop builds or HTTP in browser harnesses.
 */

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
};

export class IpcError extends Error {
  readonly diagnostic: Diagnostic;

  constructor(diagnostic: Diagnostic) {
    super(diagnostic.message);
    this.name = 'IpcError';
    this.diagnostic = diagnostic;
  }

  /** The one-line form an inline error, a toast and a pane marker all show. */
  get text(): string {
    return this.diagnostic.line === null
      ? `${this.diagnostic.message} [${this.diagnostic.code}]`
      : `${this.diagnostic.path}:${this.diagnostic.line} — ${this.diagnostic.message} [${this.diagnostic.code}]`;
  }
}

type TauriBridge = {
  core: { invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown> };
  event?: { listen: (name: string, handler: (event: { payload: unknown }) => void) => Promise<() => void> };
  window?: { getCurrentWindow: () => { close: () => void } };
};

type WebBridge = { endpoint: string };

function scope(): { __TAURI__?: TauriBridge; __SAM_WEB__?: WebBridge } {
  return globalThis as { __TAURI__?: TauriBridge; __SAM_WEB__?: WebBridge };
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
        throw new IpcError(parseDiagnostic(error));
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
 * Rather than trusting the wrapper, find the first JSON object in it — and fall
 * back to the raw text so nothing is ever swallowed.
 */
function parseDiagnostic(error: unknown): Diagnostic {
  const text =
    typeof error === 'string'
      ? error
      : error instanceof Error
        ? error.message
        : String(error);
  const start = text.indexOf('{');
  if (start >= 0) {
    const end = text.lastIndexOf('}');
    if (end > start) {
      try {
        const parsed = JSON.parse(text.slice(start, end + 1)) as Partial<Diagnostic>;
        if (typeof parsed.message === 'string') {
          return {
            code: parsed.code ?? 'ipc.error',
            path: parsed.path ?? '-',
            line: parsed.line ?? null,
            message: parsed.message,
          };
        }
      } catch {
        /* not JSON — fall through to the raw text */
      }
    }
  }
  return { code: 'ipc.error', path: '-', line: null, message: text };
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
