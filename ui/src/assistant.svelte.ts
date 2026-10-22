/**
 * The assistant (§4.7's in-app door; UI_SPEC "The assistant deck").
 *
 * One deck and one island, one running turn. This module owns what they draw —
 * the detected CLIs, the models each one reports, the turn's rows, the receipt,
 * the island's own open state, and the live "what is it doing right now" line —
 * while the deck's open/closed toggle stays a presentation command in the
 * session (`app.assistant`), so `present()` flips the deck exactly like it
 * flips the palette.
 *
 * The run itself lives in the shell process (`sam_core::agent` spawning a CLI
 * installed on this machine), and its edits reach the app the way every other
 * edit does: the plan watcher reloads the open plan while the veil is still
 * up. Nothing here talks to a network, and nothing here holds credentials.
 *
 * Two integrations beyond "spawn and stream" (both modelled on OpenDesign):
 * the **model** the student picked travels as the CLI's own `--model` flag
 * (discovered lazily from the CLI's listing command, with documented fallback
 * hints), and a **session id** captured from a previous turn's stream lets the
 * next turn continue the same conversation (`claude --resume`, `opencode -s`)
 * instead of re-deriving context from the plan alone.
 */
import {
  agentCancel,
  agentModels,
  agentStart,
  dispatch,
  IpcError,
  type AgentEvent,
  type AgentModel,
} from './ipc';

/** One detected CLI, as `agent.list` reports it. */
export type AgentStatus = {
  id: string;
  name: string;
  bin: string;
  path: string | null;
  version: string | null;
  available: boolean;
  installHint: string;
};

/** What the deck draws, in stream order. */
export type TranscriptRow =
  | { kind: 'text'; text: string }
  | { kind: 'tool'; name: string; summary: string; file?: string }
  | { kind: 'note'; text: string }
  | { kind: 'error'; text: string };

/** The run's transaction, offered as one Undo. */
export type Receipt = { txid: string; summary: string; files: string[] };

/** The picker's sentinel for "type any id the CLI accepts". */
const CUSTOM_MODEL = '__custom__';
const AGENT_KEY = 'cadence.assistant.agent';
const MODEL_KEY = (agent: string) => `cadence.assistant.model.${agent}`;
const CUSTOM_KEY = (agent: string) => `cadence.assistant.model.custom.${agent}`;
/** How many transcript rows one turn keeps: an hour-long turn must not grow
    the DOM without limit, and the newest steps are the ones worth reading. */
const MAX_ROWS = 500;
/** How long the shell may take to answer a spawn before the deck says so. */
const START_TIMEOUT_MS = 30_000;
/** How long a stopped turn may take to report its `Exit`. */
const STOP_TIMEOUT_MS = 3_000;

function textOf(error: unknown): string {
  return error instanceof IpcError ? error.text : String(error);
}

/** Reject `work` after `milliseconds`, so a shell that never answers cannot
 *  leave the deck inert with no way back. */
function withTimeout<T>(work: Promise<T>, milliseconds: number, message: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new IpcError({ code: 'ipc.timeout', path: '-', line: null, message }));
    }, milliseconds);
    work.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

/**
 * What the deck knows about the window, structurally — the module never
 * imports the session (the session imports *it*), so the caller hands its
 * state in and gets back the block the agent reads.
 */
export type ContextSource = {
  plan: string | null;
  revision: string | null;
  selected: string | null;
  detail: { id: string; type: string } | null;
  outcome: { view?: string; type?: string | null; components?: unknown[] } | null;
  navigation: Array<{ title: string; view: string }>;
  views: { views: Record<string, { panel?: string | null; type?: string | null }> } | null;
  systemOpen: boolean;
  settingsOpen: boolean;
  screenEditing: boolean;
  today: { weekday: string; day: number; month: string } | null;
};

/**
 * The agent's picture of the student's standing point: the screen that is
 * open, the record that is selected, the revision the app holds, the plan's
 * other destinations — and the default when a request names no screen. Built
 * from the same state the UI draws, so it cannot drift from what the student
 * sees.
 */
export function buildAppContext(state: ContextSource): string {
  const lines: string[] = [];
  if (state.today) {
    lines.push(`- Today is ${state.today.weekday} ${state.today.day} ${state.today.month}.`);
  }
  if (state.plan === null) {
    lines.push('- No plan is open in the window yet.');
  } else if (state.settingsOpen) {
    lines.push('- The window is showing **Settings**.');
  } else if (state.systemOpen) {
    lines.push('- The window is showing the **System** surface (kinds, views, rules, schedulers).');
  } else if (state.screenEditing) {
    lines.push('- The student is **editing the screen** in the composer.');
  } else if (state.outcome?.view) {
    const title =
      state.navigation.find((entry) => entry.view === state.outcome?.view)?.title ??
      state.outcome.view;
    const kind = state.outcome.type
      ? `a list of \`${state.outcome.type}\` records`
      : 'a composed screen';
    const components = state.outcome.components?.length
      ? `, ${state.outcome.components.length} components`
      : '';
    lines.push(
      `- The student is looking at **${title}** (view \`${state.outcome.view}\` — ${kind}${components}).`,
    );
  }
  if (state.selected) {
    const kind = state.detail?.type ? ` (a \`${state.detail.type}\` record)` : '';
    lines.push(`- The selected record is \`${state.selected}\`${kind}.`);
  }
  if (state.revision) {
    lines.push(
      `- The app's plan revision is \`${state.revision}\` — pass it as \`--if-revision\` so a stale plan is caught instead of overwritten.`,
    );
  }
  const destinations = state.navigation
    .slice(0, 12)
    .map((entry) => `${entry.title} (\`${entry.view}\`)`);
  if (destinations.length > 0) {
    lines.push(`- The rail's destinations: ${destinations.join(', ')}.`);
  }
  const screens = Object.entries(state.views?.views ?? {})
    .filter(([, view]) => view.panel)
    .map(([name]) => name)
    .slice(0, 12);
  if (screens.length > 0) {
    lines.push(`- The plan's composed screens: ${screens.join(', ')}.`);
  }
  lines.push(
    '- When the request names no screen, it is about the one above; for a plan-wide change, say so in your reply before touching anything.',
  );
  return lines.join('\n');
}

class Assistant {
  open = $state(false);
  /**
   * The island's process panel. The island itself is the window's own chrome —
   * a small ink pill at the head of the room, black in the light theme and
   * white in the dark one, the Dynamic-Island reading the owner asked for —
   * and this is the one thing it holds: closed by default (the pill says what
   * is happening in a word), opened by the press, opened on a turn's end
   * because that is the moment there is something to read.
   */
  island = $state(false);
  /** The detection read (`null` before the first open). */
  agents = $state<AgentStatus[] | null>(null);
  sam = $state<{ path: string | null; available: boolean } | null>(null);
  /** The chosen CLI, remembered across sessions. */
  agentId = $state<string | null>(null);
  /** The models the chosen CLI reports (lazy; empty until the picker asks). */
  models = $state<AgentModel[]>([]);
  modelSource = $state<'live' | 'fallback' | 'profile' | 'none' | null>(null);
  modelsLoading = $state(false);
  /** `default` · a model id · `__custom__` (the typed-id escape hatch). */
  modelOption = $state<string>('default');
  customModel = $state('');
  prompt = $state('');
  running = $state(false);
  runId = $state<string | null>(null);
  rows = $state<TranscriptRow[]>([]);
  receipt = $state<Receipt | null>(null);
  /** One line of state: the deck's footer and its polite live region. */
  status = $state('');
  /** A turn ended and its steps have not been put away yet — the island's own
      reason to stay when a silent turn left no rows to draw. */
  settled = $state(false);
  /** The transcript dropped its oldest rows to stay bounded. */
  trimmed = $state(false);
  /** Why the custom-profile escape hatch did not load, when it exists. */
  profilesError = $state<string | null>(null);

  /** Events that arrived before `agentStart` returned (the spawn is fast). */
  private pending: AgentEvent[] = [];
  /** `running` only turns true when the spawn answers; this is the window in
      between, and it is reactive because the deck draws its controls from it. */
  private awaitingStart = $state(false);
  private pendingFile: string | null = null;
  private plan: string | null = null;
  /** Bumped by every turn: a receipt or a stale spawn from turn N must never
      land in turn N+1. */
  private generation = 0;
  /** The stop watchdog: a kill the shell never answers must not leave the deck
      on "Stopping" for the rest of the session. */
  private stopTimer: ReturnType<typeof setTimeout> | null = null;
  /**
   * Text deltas arrive per token. They are buffered and flushed once a frame,
   * so one message costs one reactive update instead of hundreds — the deck
   * must feel like typing, not like a firehose of IPC messages.
   */
  private textBuffer = '';
  private flushHandle: number | null = null;
  /** Bumped on every flush; the deck's scroll effect depends on it. */
  flushed = $state(0);
  private detection: Promise<void> | null = null;
  /** Per-agent caches, this app session only. */
  private modelsByAgent: Record<string, { models: AgentModel[]; source: Assistant['modelSource'] }> = {};
  /** In-flight model reads, per agent — a second open joins, never duplicates. */
  private modelsPending: Record<string, Promise<void>> = {};
  /** The picker asked for models before an agent was chosen (detection still
      resolving when the card opened); `choose` fires the read when it lands. */
  private modelsWanted = false;
  /** The CLI's own session handles, captured from streams — the resume keys,
      keyed `agentId:plan`: a handle is one plan's history, and switching or
      moving folders must start a fresh conversation. */
  private sessions: Record<string, string> = {};

  constructor() {
    try {
      this.agentId = localStorage.getItem(AGENT_KEY);
      if (this.agentId) {
        this.modelOption = localStorage.getItem(MODEL_KEY(this.agentId)) ?? 'default';
        this.customModel = localStorage.getItem(CUSTOM_KEY(this.agentId)) ?? '';
      }
    } catch {
      /* private mode: the choices simply do not persist */
    }
  }

  async toggle(): Promise<void> {
    if (this.open) this.close();
    else await this.openDeck();
  }

  close(): void {
    this.open = false;
    this.island = false;
    // Closing the deck closes the assistant: a completed turn's steps go with
    // it, or the pill would stay at the head of the room after the box it
    // belongs to was put away. A live turn keeps its island — it is the one
    // Stop on screen when the deck is closed (`finish()` puts it away then).
    if (!this.busy()) this.dismissProcess();
  }

  /** The island's press: open the process, or put it away. */
  toggleIsland(): void {
    this.island = !this.island;
  }

  /**
   * Whether the island is on screen at all. It is chrome, so it is up while
   * there is a run to report — live, finished with rows to read, or finished
   * with nothing to say (a Stop before the first token still has a state to
   * show) — and gone once the process is put away. A finished run is part of
   * the deck: once the deck is closed, its rows are not chrome for this window
   * any more, so they cannot keep the pill standing over the screen.
   */
  showsIsland(): boolean {
    return this.running || (this.open && (this.rows.length > 0 || this.settled));
  }

  /**
   * Put the process away: the panel closes and the turn's row list goes with
   * it, which is what takes the island off the screen once nothing is running.
   * The receipt is not touched — Undo lives in the deck, where the prompt is.
   */
  dismissProcess(): void {
    this.island = false;
    this.rows = [];
    this.trimmed = false;
    this.settled = false;
    this.status = '';
  }

  async openDeck(): Promise<void> {
    this.open = true;
    await this.detect();
    // Models are read when the picker opens, never here: `opencode models`
    // is a network round-trip and the deck must open instantly.
  }

  /**
   * The palette's `agent.run` door: open the deck with the prompt in hand and
   * start the turn. Returns whether the turn actually started — a palette row
   * that reports success while a turn is already running is a lie the student
   * pays for with a lost draft.
   */
  async start(prompt: string, plan: string | null, context: string | null = null): Promise<boolean> {
    await this.openDeck();
    if (this.busy()) {
      this.status = 'A turn is already running';
      return false;
    }
    this.prompt = prompt;
    if (plan === null || !this.canRun(plan)) return false;
    return this.run(plan, context);
  }

  /**
   * The machine's CLIs. Re-read every time the deck opens — the engine caches
   * the probe for 30 s, so this is cheap — because a CLI installed while the
   * app is open must appear, and the deck's own blocker tells the student to
   * reopen the deck. A failed read is never memoized: the next open asks again.
   */
  async detect(): Promise<void> {
    if (this.detection) return this.detection;
    this.detection = (async () => {
      try {
        const result = await dispatch('agent.list');
        const data = result.data as
          | {
              agents?: AgentStatus[];
              sam?: { path: string | null; available: boolean };
              profilesError?: string | null;
            }
          | undefined;
        this.agents = data?.agents ?? [];
        this.sam = data?.sam ?? null;
        this.profilesError = data?.profilesError ?? null;
        // Keep a remembered choice only while it is still installed; otherwise
        // fall to the first available CLI.
        if (!this.agents.some((agent) => agent.id === this.agentId && agent.available)) {
          this.choose(this.available()[0]?.id ?? null);
        }
      } catch (error) {
        this.agents = [];
        this.sam = null;
        this.profilesError = null;
        this.status = textOf(error);
      } finally {
        this.detection = null;
      }
    })();
    return this.detection;
  }

  available(): AgentStatus[] {
    return (this.agents ?? []).filter((agent) => agent.available);
  }

  choose(id: string | null): void {
    this.agentId = id;
    try {
      if (id) localStorage.setItem(AGENT_KEY, id);
      else localStorage.removeItem(AGENT_KEY);
    } catch {
      /* the choice still holds for this session */
    }
    // The model and the conversation belong to the CLI that was chosen.
    try {
      this.modelOption = id ? (localStorage.getItem(MODEL_KEY(id)) ?? 'default') : 'default';
      this.customModel = id ? (localStorage.getItem(CUSTOM_KEY(id)) ?? '') : '';
    } catch {
      this.modelOption = 'default';
      this.customModel = '';
    }
    // The list itself is read when the picker opens; a cached list comes back
    // now so the stored choice can still show its label.
    this.applyModels(id);
    // A picker opened before this choice landed asked for the list; fire now.
    if (this.modelsWanted) void this.ensureModels();
  }

  chosen(): AgentStatus | null {
    return this.available().find((agent) => agent.id === this.agentId) ?? null;
  }

  // ── models ────────────────────────────────────────────────────────────────

  /**
   * The chosen CLI's models, asked once per app session — `opencode models`
   * and `agy models` are network round-trips the engine bounds at 15 s, so the
   * picker asks when it opens and the deck itself never waits. A second open
   * while the first answer is in flight joins it instead of spawning twice.
   */
  async ensureModels(): Promise<void> {
    const id = this.agentId;
    if (!id) {
      // The picker can open while detection is still resolving: remember that
      // the list was asked for, and `choose` fires the read when it lands.
      this.modelsWanted = true;
      return;
    }
    this.modelsWanted = false;
    if (this.modelsByAgent[id]) {
      this.applyModels(id);
      return;
    }
    this.modelsPending[id] ??= (async () => {
      if (this.agentId === id) this.modelsLoading = true;
      try {
        const result = await agentModels(id);
        const source =
          result.source === 'live' || result.source === 'fallback' || result.source === 'profile'
            ? result.source
            : 'none';
        this.modelsByAgent[id] = { models: result.models, source };
      } catch {
        this.modelsByAgent[id] = { models: [], source: 'none' };
      } finally {
        delete this.modelsPending[id];
        if (this.agentId === id) {
          this.modelsLoading = false;
          this.applyModels(id);
        }
      }
    })();
    return this.modelsPending[id];
  }

  private applyModels(id: string | null): void {
    const cached = id ? this.modelsByAgent[id] : undefined;
    this.models = cached?.models ?? [];
    this.modelSource = cached?.source ?? null;
  }

  /** The label the model pill shows. */
  modelLabel(): string {
    if (this.modelOption === CUSTOM_MODEL) return this.customModel.trim() || 'Custom…';
    if (this.modelOption === 'default') return 'Default';
    return this.models.find((model) => model.id === this.modelOption)?.label ?? this.modelOption;
  }

  /** What the run sends: `null` = the CLI's own configured default. */
  private activeModel(): string | null {
    if (this.modelOption === CUSTOM_MODEL) return this.customModel.trim() || null;
    if (this.modelOption === 'default') return null;
    return this.modelOption;
  }

  setModelOption(option: string): void {
    this.modelOption = option;
    this.persistModel();
  }

  setCustomModel(text: string): void {
    this.customModel = text;
    this.persistModel();
  }

  private persistModel(): void {
    const id = this.agentId;
    if (!id) return;
    try {
      localStorage.setItem(MODEL_KEY(id), this.modelOption);
      localStorage.setItem(CUSTOM_KEY(id), this.customModel);
    } catch {
      /* the choice still holds for this session */
    }
  }

  // ── running ───────────────────────────────────────────────────────────────

  /**
   * Whether a turn can start. `running` only turns true when the spawn has
   * answered, so the window in between is guarded by `awaitingStart` —
   * otherwise two doors onto one prompt (a press and a palette row, or a
   * doubled keydown) would both spawn a turn for it.
   */
  canRun(plan: string | null): boolean {
    return (
      !this.busy() &&
      plan !== null &&
      this.prompt.trim().length > 0 &&
      this.available().length > 0 &&
      this.sam?.available !== false
    );
  }

  /**
   * A turn is live, or its spawn is in flight. The window between Send and
   * `Started` owns the same controls as the run itself: it can be stopped, and
   * the field belongs to the next turn, not this one.
   */
  busy(): boolean {
    return this.running || this.awaitingStart;
  }

  /** Start one turn. The plan is the folder the window has open. */
  async run(plan: string | null, context: string | null = null): Promise<boolean> {
    if (!this.canRun(plan) || plan === null) return false;
    const prompt = this.prompt.trim();
    // The field is emptied before the spawn, not after: whatever is typed in
    // the window between Send and `Started` belongs to the next turn.
    this.prompt = '';
    const generation = ++this.generation;
    this.rows = [];
    this.receipt = null;
    this.pendingFile = null;
    this.pending = [];
    this.textBuffer = '';
    this.trimmed = false;
    this.settled = false;
    // A new turn opens as the small pill: the island says the state in a word
    // while the work is happening, and the press (or the turn's end) shows the
    // steps. Nothing from the last turn is carried into this one.
    this.island = false;
    if (this.flushHandle !== null) {
      cancelAnimationFrame(this.flushHandle);
      this.flushHandle = null;
    }
    this.awaitingStart = true;
    this.plan = plan;
    this.status = 'Working';
    try {
      // A resume handle belongs to one plan's history: a turn in a different
      // folder starts a fresh conversation instead of the old plan's memory.
      const sessionKey = this.agentId ? `${this.agentId}:${plan}` : null;
      const started = await withTimeout(
        agentStart({
          agentId: this.agentId,
          model: this.activeModel(),
          // Continue the same CLI conversation when one was captured — the
          // agent keeps its working memory instead of re-reading the plan.
          resume: sessionKey ? (this.sessions[sessionKey] ?? null) : null,
          context,
          prompt,
          plan,
        }),
        START_TIMEOUT_MS,
        'the shell did not answer in time — the turn was not started',
      );
      if (generation !== this.generation) return false;
      this.runId = started.runId;
      this.running = true;
      this.awaitingStart = false;
      // Replay anything the stream delivered while the invoke was in flight.
      const queued = this.pending;
      this.pending = [];
      for (const event of queued) {
        if (event.runId === started.runId) this.handleEvent(event);
      }
      return true;
    } catch (error) {
      // A spawn that timed out may still be starting in the shell: stop it
      // rather than leave a turn no page can see or cancel.
      void agentCancel().catch(() => false);
      if (generation !== this.generation) return false;
      this.awaitingStart = false;
      this.running = false;
      this.runId = null;
      // Nothing was sent, so the text goes back to the field.
      if (this.prompt.length === 0) this.prompt = prompt;
      this.addRow({ kind: 'error', text: textOf(error) });
      this.status = 'Could not start';
      return false;
    }
  }

  async cancel(): Promise<void> {
    if (!this.busy()) return;
    this.status = 'Stopping';
    const runId = this.runId;
    const cancelled = await agentCancel().catch(() => false);
    if (!cancelled) {
      // The shell had nothing to stop: the turn is over whatever the deck last
      // drew. That covers a finished run whose `Exit` was dropped, and a spawn
      // that never answered.
      this.awaitingStart = false;
      await this.finish(-2);
      return;
    }
    this.armStopWatchdog(runId);
  }

  /**
   * A kill the shell never reports must not leave the deck on "Stopping".
   * `Exit` is the normal end; this is the backstop when the event never lands.
   */
  private armStopWatchdog(runId: string | null): void {
    if (this.stopTimer !== null) clearTimeout(this.stopTimer);
    this.stopTimer = setTimeout(() => {
      this.stopTimer = null;
      if (!this.running || this.runId !== runId) return;
      this.addRow({
        kind: 'note',
        text: 'the shell did not report the stop; the turn was ended here',
      });
      void this.finish(-2);
    }, STOP_TIMEOUT_MS);
  }

  /**
   * The island's one word: the tool the newest step named, else what the
   * stream last did, else the state. The pill stays a pill — the sentence an
   * agent is writing belongs to the process, one press away.
   */
  brief(): string {
    if (!this.running) return this.status;
    for (let index = this.rows.length - 1; index >= 0; index -= 1) {
      const row = this.rows[index];
      if (row.kind === 'tool') return row.name;
      if (row.kind === 'text' && row.text.trim()) return 'Writing…';
    }
    return 'Thinking…';
  }

  /**
   * The live "what is it doing" line: the newest tool call (the argument a
   * human would name it by), else the tail of the newest prose, else a state
   * word. The island's own process panel reads this; the closed pill reads the
   * shorter `brief`.
   */
  activity(): string {
    if (!this.running) return this.status;
    for (let index = this.rows.length - 1; index >= 0; index -= 1) {
      const row = this.rows[index];
      if (row.kind === 'tool') {
        return row.summary ? `${row.name} — ${row.summary}` : row.name;
      }
      if (row.kind === 'text' && row.text.trim()) {
        // Only the tail is read: a long turn's prose must not be scanned on
        // every frame just to show 120 characters of it.
        return row.text.slice(-400).trim().replace(/\s+/g, ' ').slice(-120);
      }
    }
    return 'thinking…';
  }

  /** One streamed event, from the shell. A stale run's events are dropped. */
  handleEvent(event: AgentEvent): void {
    if (this.awaitingStart) {
      this.pending.push(event);
      return;
    }
    if (!this.runId || event.runId !== this.runId) return;
    // Text is buffered per frame; every other row must land in transcript order.
    if (event.kind !== 'text') this.flushText();
    switch (event.kind) {
      case 'started':
        this.status = `${event.agent} is working`;
        break;
      case 'session':
        // The CLI named its session: keep it for this agent *and this plan*
        // so the next turn can continue it, and never draw it in the
        // transcript.
        if (this.agentId && this.plan) this.sessions[`${this.agentId}:${this.plan}`] = event.id;
        break;
      case 'text':
        this.queueText(event.text);
        break;
      case 'thinking':
        // Kept out of the deck on purpose: the record shows what the agent
        // did, not the transcript of its reasoning.
        break;
      case 'tool':
        this.addRow({
          kind: 'tool',
          name: event.name,
          summary: event.summary,
          file: this.pendingFile ?? undefined,
        });
        this.pendingFile = null;
        break;
      case 'file':
        this.pendingFile = event.path;
        break;
      case 'toolResult':
        // Only failures carry a line worth drawing — a `sam apply` diagnostic
        // is exactly this case.
        if (event.isError && event.content) {
          this.addRow({ kind: 'note', text: event.content });
        }
        break;
      case 'result':
        if (event.summary) this.addRow({ kind: 'note', text: event.summary });
        break;
      case 'error':
        this.addRow({ kind: 'error', text: event.message });
        break;
      case 'log':
      case 'stderr':
        this.addRow({ kind: 'note', text: event.line });
        break;
      case 'exit':
        void this.finish(event.code);
        break;
    }
  }

  /** One row in transcript order, with the transcript bounded. */
  private addRow(row: TranscriptRow): void {
    this.rows.push(row);
    if (this.rows.length > MAX_ROWS) {
      this.rows.splice(0, this.rows.length - MAX_ROWS);
      this.trimmed = true;
    }
  }

  private appendText(text: string): void {
    const last = this.rows[this.rows.length - 1];
    if (last?.kind === 'text') last.text += text;
    else this.addRow({ kind: 'text', text });
  }

  private queueText(text: string): void {
    this.textBuffer += text;
    if (this.flushHandle !== null) return;
    this.flushHandle = requestAnimationFrame(() => this.flushText());
  }

  /**
   * One rows mutation per frame, in transcript order: called before any
   * non-text row lands and when a turn ends, so nothing overtakes the prose.
   */
  private flushText(): void {
    if (this.flushHandle !== null) {
      cancelAnimationFrame(this.flushHandle);
      this.flushHandle = null;
    }
    const text = this.textBuffer;
    this.textBuffer = '';
    if (text.length === 0) return;
    this.appendText(text);
    this.flushed += 1;
  }

  private async finish(code: number): Promise<void> {
    if (this.stopTimer !== null) {
      clearTimeout(this.stopTimer);
      this.stopTimer = null;
    }
    const runId = this.runId;
    const plan = this.plan;
    // Ending the turn invalidates anything still in flight: a spawn that
    // resolves *after* a Stop (the shell was slow, not absent) must not
    // resurrect the turn the student already ended. The receipt claim below
    // takes this same generation, so it still owns its own answer.
    this.generation += 1;
    const generation = this.generation;
    this.running = false;
    this.awaitingStart = false;
    this.runId = null;
    this.pendingFile = null;
    // A turn that ends is a turn to read: the island opens on its own steps —
    // the agent's last words, and (below the field) the receipt's one Undo. A
    // deck that was closed mid-turn asked not to see it: the turn ends into
    // nothing rather than growing a pill over the screen the student went back
    // to. The receipt still waits in the deck for the next open.
    this.settled = true;
    this.island = this.open;
    if (code === 0) {
      this.status = 'Finished';
    } else if (code === -2) {
      // -2 is the engine's own mark for a child the Stop took out of its slot.
      // A signal the OS sent (-9 from a memory-pressure kill) is *not* a stop,
      // and calling it one would hide a crash.
      this.status = 'Stopped';
    } else {
      // The pill says one word, so the sentence goes to the process, where it
      // can actually be read: the exit code is part of the turn's record.
      this.status = 'Failed';
      this.addRow({ kind: 'error', text: `The agent exited with code ${code}` });
    }
    // A turn can commit and still exit non-zero — a `sam apply` that landed,
    // then a failed `configcheck`. The writes are undoable either way, so the
    // receipt is read on every exit.
    await this.claimReceipt(plan, runId, generation);
    // A deck that is closed when the turn ends (or was closed while the receipt
    // was being read) asked not to see it: the rows written in the meantime
    // (`Failed`, the receipt's note) go with it, or they would grow a pill over
    // the screen the student went back to. A newer turn owns the transcript
    // now, so it is never wiped.
    if (generation === this.generation) {
      if (this.open) {
        this.island = true;
      } else {
        this.island = false;
        this.rows = [];
        this.trimmed = false;
        this.settled = false;
      }
    }
  }

  /**
   * The turn's own transactions, read from `tx.list`. The engine stamps every
   * transaction committed by the agent child with the run id (`SAM_RUN`), so
   * the match is exact: a write the student made in a terminal during the same
   * minute is somebody else's and is never offered as this turn's Undo. With
   * several writes in one turn, Undo reverts the newest and the deck says so.
   */
  private async claimReceipt(
    plan: string | null,
    runId: string | null,
    generation: number,
  ): Promise<void> {
    if (!plan || !runId) return;
    try {
      const result = await dispatch('tx.list', {}, { plan });
      // A newer turn owns the receipt now: this answer is stale.
      if (generation !== this.generation) return;
      const backups = (result.data as
        | {
            backups?: Array<{
              txid: string;
              summary: string;
              files?: string[];
              date: string;
              run?: string | null;
            }>;
          }
        | undefined)?.backups ?? [];
      const mine = backups.filter((backup) => backup.run === runId);
      const fresh = mine[0];
      if (mine.length > 1) {
        this.addRow({
          kind: 'note',
          text: `${mine.length} transactions were committed this turn; Undo reverts the newest (${mine[0].txid}).`,
        });
      }
      this.receipt = fresh
        ? { txid: fresh.txid, summary: fresh.summary, files: fresh.files ?? [] }
        : null;
    } catch {
      if (generation === this.generation) this.receipt = null;
    }
  }

  /** Undo the run's transaction — the same restore the CLI's `tx.restore` runs. */
  async undo(): Promise<void> {
    const receipt = this.receipt;
    if (!receipt) return;
    this.receipt = null;
    this.status = 'Restoring';
    try {
      await dispatch('tx.restore', { txid: receipt.txid }, { plan: this.plan });
      this.status = 'Restored';
      this.addRow({ kind: 'note', text: `Restored the plan to before ${receipt.txid}` });
    } catch (error) {
      this.receipt = receipt;
      this.addRow({ kind: 'error', text: textOf(error) });
    }
  }
}

/** The app's one assistant deck. */
export const assistant = new Assistant();
