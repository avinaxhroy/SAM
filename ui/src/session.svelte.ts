/**
 * Session state machine and view snapshot manager (Appendix C.5).
 * Maintains current screen selection, sheet state, and undo stack over engine snapshots (§4.8 P6, §9).
 */
import {
  IpcError,
  dispatch,
  openPreferences,
  pickDirectory,
  pickProfileFile,
  pickProfileOut,
  publishWrite,
  type DispatchResult,
  type PlanChanged,
} from './ipc';
import {
  applyAppearance,
  rememberMode,
  storedMode,
  type AppearanceRead,
  type ThemeRead,
} from './appearance';
import { durationText } from './types';
import { defaultSurfaces } from './composer/defaults';
import { widgetLabel } from './composer/widgets/registry';
import type {
  BlockNode,
  CommandRead,
  CommandsRead,
  FieldRead,
  MachineRead,
  PathsRead,
  PlanRead,
  RecordDoc,
  ScreenComponent,
  TodayFacts,
  TodayItem,
  TodayRead,
  ViewRead,
  ViewsRead,
} from './types';

/** Appendix C.5 bootstrap lifecycle state. */
export type Phase = 'booting' | 'noneExists' | 'several' | 'ready';

/** Committed edit transaction record for undo stack (§4.8 P6). */
export type UndoEntry = {
  txid: string;
  revision: string;
  title: string;
};

/** Active target for source pane inspector (Appendix C.4). */
export type SourceTarget =
  | { kind: 'record'; id: string }
  | { kind: 'file'; file: string };

/** Active study timer session (F2). Persisted to localStorage across restarts. */
export type TimerState = {
  item: { id: string; label: string; kind: string; course: { id: string; label: string } | null };
  startedAt: number;
  pausedMs: number;
  pausedAt: number | null;
  targetMin: number;
};

/** Toast action callback (F14). */
export type ToastAction = { label: string; run: () => void };

/** Active modal sheet (§11). At most one floating sheet is open at a time. */
export type Sheet =
  | { kind: 'record.new'; type: string }
  | { kind: 'today.add' }
  | { kind: 'column.new'; type: string }
  | { kind: 'list.new' }
  | { kind: 'view.edit'; name: string }
  | { kind: 'paste'; type: string }
  | { kind: 'column.rename'; type: string; field: FieldRead }
  | { kind: 'column.retype'; type: string; field: FieldRead }
  | { kind: 'column.choices'; type: string; field: FieldRead }
  | { kind: 'column.delete'; type: string; field: FieldRead }
  | { kind: 'record.delete'; record: RecordDoc }
  | { kind: 'plan.new' }
  // The plan's own folder, in a sheet (`app.changePlan`): the picker's
  // material over a working plan.
  | { kind: 'plan.folder' }
  // Entity deletions (`UI_SPEC.md` §4 E2).
  | { kind: 'type.delete'; id: string; title: string | null }
  | { kind: 'view.delete'; name: string }
  | { kind: 'list.delete'; view: string; title: string }
  | { kind: 'catchup'; items: Array<{ id: string; label: string }>; label: string }
  // Composer widget insertion (COMPOSER §4.6).
  | { kind: 'component.insert' }
  // Screen properties (§4.7).
  | { kind: 'screen.rename'; view: string }
  | { kind: 'screen.icon'; view: string }
  | { kind: 'screen.remove'; view: string }
  // First-run overview (R11).
  | { kind: 'gettingStarted' }
  | null;

/** A draft in the source pane, kept separate from the published snapshot. */
export type PaneDraft = {
  target: SourceTarget;
  text: string;
  /** A revision landed while this draft was open: offer Reload, never clobber. */
  external: boolean;
};

/** The parameters a command was given, as the entry's name. */
function describing(params: Record<string, unknown>): string {
  for (const key of ['spec', 'id', 'name', 'key', 'type', 'file', 'view']) {
    const value = params[key];
    if (typeof value === 'string' && value.length > 0) return value;
  }
  return '';
}

/**
 * A session's elapsed time, in milliseconds: the wall clock minus the pauses.
 * While paused the clock stops at the pause, which is why this is a formula and
 * not `Date.now() - startedAt`.
 */
export function elapsedMs(timer: TimerState, now: number): number {
  const until = timer.pausedAt ?? now;
  return Math.max(0, until - timer.startedAt - timer.pausedMs);
}

/**
 * Generates an accessible receipt description for screen layout mutations (F14).
 */
function receiptFor(before: ScreenComponent[], after: ScreenComponent[]): string {
  const added = after.filter((component) => !before.some((was) => was.surface === component.surface));
  const removed = before.filter((was) => !after.some((component) => component.surface === was.surface));
  if (added.length === 1 && removed.length === 0) return `Added ${widgetLabel(added[0].surface)}`;
  if (removed.length === 1 && added.length === 0) return `Removed ${widgetLabel(removed[0].surface)}`;
  if (added.length === 0 && removed.length === 0) {
    // The same members in another order: name the first one the gesture moved.
    const moved = after.find((component, at) => before[at]?.surface !== component.surface);
    if (moved !== undefined) return `Moved ${widgetLabel(moved.surface)}`;
    // The same members in the same order: what changed is how wide one stands.
    const resized = after.find((component, at) => before[at]?.span !== component.span);
    if (resized !== undefined) {
      return resized.span === 1
        ? `${widgetLabel(resized.surface)} is half the screen now`
        : `${widgetLabel(resized.surface)} is the full width again`;
    }
  }
  return 'The screen changed';
}

/** How many plans the room remembers by name (R11). A memory, not a document. */
const RECENT_MAX = 5;

/**
 * A folder and a name, joined the way the platform writes paths — the one place
 * the room and `plan.new` agree about the destination it shows and the
 * destination it writes. The engine takes either separator; the *display* must
 * match the folder the student picked, or the two lines read as two paths.
 */
export function joinPath(folder: string, name: string): string {
  const slash = folder.includes('\\') && !folder.includes('/') ? '\\' : '/';
  return (folder.replace(/[\\/]+$/, '') + slash + name.replace(/^[\\/]+/, '')).replace(/\/{2,}/g, '/');
}

/** The presentation ids the engine's registry declares (§4.9) — a fixed set,
 * mirrored here so `run()` recognises them before the registry has arrived
 * and a rail click can never race the registry into a dead engine dispatch. */
const PRESENT: Record<string, true> = {
  'app.palette': true,
  'app.toggleSourcePane': true,
  'app.openSettings': true,
  'app.gettingStarted': true,
  'app.changePlan': true,
  'app.openSystem': true,
  'screen.edit': true,
  'rail.select': true,
  'view.edit': true,
  'record.reveal': true,
  'record.panel': true,
};

export class Session {
  // ── bootstrap ──────────────────────────────────────────────────────────
  phase = $state<Phase>('booting');
  plan = $state<string | null>(null);
  plans = $state<PlanRead[]>([]);
  plansDir = $state<string>('');
  /** The parent folder the student chose for their next plan; null = the plans root (R11). */
  planParent = $state<string | null>(null);
  /**
   * The plans this machine opened, most recent first (R11) — `localStorage`, so
   * a plan kept outside the plans root is one press away the next morning. It is
   * a *memory*, not a document: nothing here is a fact about the plan, and
   * forgetting it costs one folder pick.
   */
  recent = $state<string[]>([]);
  /** The folder an explicit open is attempting right now; null = nobody asked. */
  private opening: string | null = null;
  /**
   * The plan every piece of session state below was read for: the plan-scoped
   * state (a session, a screen draft, the undo stack) is shed when this stops
   * matching `plan` — a switch — and kept when it does not (D9's re-read of the
   * same plan, which must not stop a running timer).
   */
  private loadedPlan: string | null = null;
  paths = $state<PathsRead | null>(null);
  views = $state<ViewsRead | null>(null);
  registry = $state<CommandRead[]>([]);
  keybindings = $state<Array<{ key: string; command: string }>>([]);
  revision = $state<string | null>(null);
  /** Current date and week context resolved by the engine (R2). */
  today = $state<TodayRead | null>(null);
  /** Pending review count for the review destination badge (R3). */
  reviewCount = $state(0);
  /** Renderer path reported by shell launch (§4.10, §9). */
  rendererPath = $state<string>('native');

  /** Daily facts projection from `today.view` (P2, U1). */
  todayFacts = $state<TodayFacts | null>(null);
  /** Running study timer state (F2). */
  timer = $state<TimerState | null>(null);

  // ── appearance (§6 Phase 7) ────────────────────────────────────────────
  themes = $state<ThemeRead[]>([]);
  activeTheme = $state<string | null>(null);
  appearance = $state<AppearanceRead | null>(null);

  // ── the screen ─────────────────────────────────────────────────────────
  selected = $state<string | null>(null);
  outcome = $state<ViewRead | null>(null);
  /** Selected record ID (Appendix C.2). */
  selection = $state<string | null>(null);
  /** Record inspected in the detail panel, or null. */
  detail = $state<{ id: string; type: string } | null>(null);
  settingsOpen = $state(false);
  /** System configuration surface active state (S14). */
  systemOpen = $state(false);
  /** System view payload from `system.view`. */
  machine = $state<MachineRead | null>(null);
  /** Active section in the System surface. */
  machineSection = $state<string | null>(null);
  paletteOpen = $state(false);
  sourcePane = $state(false);
  /** File opened in source pane from System surface. */
  paneFile = $state<string | null>(null);
  sheet = $state<Sheet>(null);
  /** Screen composer editing state (COMPOSER §3.2). Draft changes are staged lazily. */
  screenEditing = $state(false);
  screenDraft = $state<ScreenComponent[] | null>(null);
  /** View ID from which the active screen draft was created. */
  screenDraftFor = $state<string | null>(null);
  /** Dry-run preview text for confirmation sheets (Appendix C.2). */
  sheetPreview = $state<string | null>(null);
  /** Parsed dry-run summary counts for confirmation sheets. */
  sheetData = $state<unknown>(null);
  draft = $state<PaneDraft | null>(null);
  /** External plan revision pending over active uncommitted draft (Appendix C.4). */
  externalPending = $state(false);

  // ── floating surfaces (§11) ───────────────────────────────────────────
  toast = $state<string | null>(null);
  /** Action callback on active toast (F14). */
  toastAction = $state<ToastAction | null>(null);
  /** Diagnostic message from most recent operation. */
  lastDiagnostic = $state<string | null>(null);
  busy = $state(false);
  /** In-flight write count. */
  pending = $state(0);
  /** Error message from failed `system.view` read. */
  machineError = $state<string | null>(null);

  // ── undo (§4.8 P6) ─────────────────────────────────────────────────────
  undoStack = $state<UndoEntry[]>([]);
  redoStack = $state<UndoEntry[]>([]);

  /** Per-kind record lists, invalidated by every accepted revision. */
  recordCache: Record<string, RecordDoc[]> = {};

  types = $derived(this.views?.types ?? {});
  type = $derived<ViewsRead['types'][string] | undefined>(
    // Composed screens do not target a single kind definition.
    this.outcome?.type ? this.types[this.outcome.type] : undefined,
  );
  navigation = $derived(this.views?.navigation ?? []);
  /** Settings schema fields for settings view. */
  settingsSchema = $derived(this.views?.settings ?? []);
  study = $derived<Record<string, unknown>>(this.views?.study ?? {});
  /** Visual wash tokens assigned to records. */
  washes = $derived<Record<string, string>>(this.views?.washes ?? {});
  warnings = $derived(this.views?.warnings ?? []);

  commandDef(id: string): CommandRead | undefined {
    return this.registry.find((def) => def.id === id);
  }

  /** Keyboard shortcut bound to command ID. */
  keyOf(id: string): string | null {
    return this.keybindings.find((binding) => binding.command === id)?.key ?? null;
  }

  /** Resolves field definition from parameter spec. */
  fieldOf(params: Record<string, unknown>): FieldRead | undefined {
    const spec = params.spec;
    if (typeof spec !== 'string') return undefined;
    const type = this.outcome?.type ?? this.selectedType();
    return this.types[type]?.fields.find((field) => field.key === spec.split('.')[1]);
  }

  selectedType(): string {
    return this.outcome?.type ?? '';
  }

  // ── bootstrap contract (Appendix C.5) ──────────────────────────────────
  /** Bootstraps plan connection and determines lifecycle phase (Appendix C.5). */
  async boot(plan: string | null = null): Promise<void> {
    this.busy = true;
    this.loadRecent();
    try {
      const listing = await dispatch('plans', {}, { plan });
      const data = listing.data as { plansDir: string; plans: PlanRead[]; plan: string | null };
      this.plansDir = data.plansDir;
      this.plans = data.plans;

      // The picker's two states are already in hand. Only one plan can resolve
      // without being named, so on a machine with none — or with several — the
      // `paths` probe would be refused by design (`io.failure`), and the start
      // screen's own boot is exactly the plan-less case (BUILDLOG 2026-10-06).
      if (plan === null && data.plans.length !== 1) {
        await this.enterPicker(data.plans);
        return;
      }

      // What the window is on now — an open that fails has to leave it exactly
      // here, plan, paths and revision alike (`boot` is also the *switch*, and
      // the folder picker asks for one while a plan is open).
      const was = this.plan;
      const held = { paths: this.paths, revision: this.revision };

      try {
        const paths = await dispatch('paths', {}, { plan });
        this.paths = paths.data as PathsRead;
        this.plan = this.paths.plan;
        this.revision = this.paths.revision;
        if (this.plan) this.rememberPlan(this.plan);
        await this.load();
        this.phase = 'ready';
      } catch (failure) {
        // The plan root resolved but its own read did not: `load()` shed the
        // old folder's state on the way in (it is entering `load()` on a
        // different folder), so the folder it came from is re-read rather than
        // left half-shed under the new name — and that read brings its session
        // back with it (`restoreTimer`).
        if (was !== null && this.loadedPlan !== was) {
          this.plan = was;
          this.paths = held.paths;
          this.revision = held.revision;
          this.loadedPlan = null;
          await this.reload();
        }
        // No plan root resolves: that is the picker's two states, not an error.
        if (failure instanceof IpcError && failure.diagnostic.code === 'io.failure') {
          // …unless the student named the folder themselves: a typed path, a
          // picked directory or a remembered plan that is not a plan has to say
          // so, or the press reads as one that did nothing.
          if (this.opening !== null) this.report(failure);
          // A refused *switch* is not a refused *start*: a window that already
          // has a plan stays on it — the sheet that asked is still drawn over
          // it and the engine's words are in the toast — instead of falling
          // back to the room under a plan the titlebar still names.
          if (this.plan === null) await this.enterPicker(data.plans);
        } else {
          this.report(failure);
          if (this.plan === null) this.phase = 'several';
        }
      }
    } catch (failure) {
      this.report(failure);
      if (this.plan === null) this.phase = 'several';
    } finally {
      this.busy = false;
    }
  }

  /**
   * Open a plan that exists: the picker's and the shell's shared entry. An
   * explicit open — a pressed row, a typed path, a picked folder — is recorded
   * for the length of the attempt, so a folder that is not a plan reports its
   * refusal instead of silently landing back on the room.
   */
  async openPlan(plan: string): Promise<void> {
    // A session belongs to the folder it was started in: switching folders
    // under it would leave a running clock on work the plan being opened has
    // never heard of, and its "Stop · log" would write the minutes into the
    // wrong plan. The app already speaks this way when a session is asked to
    // start over one that is running, so an open says the same sentence and
    // waits — stopping is one press, and it is the student's.
    if (this.timer) {
      this.notice(`${this.timer.item.label} is still running — stop it first`);
      return;
    }
    this.busy = true;
    this.opening = plan;
    try {
      await this.boot(plan);
    } finally {
      this.opening = null;
      this.busy = false;
    }
  }

  /**
   * The plan-less states — `noneExists` (nothing on disk yet) and `several`
   * (plans exist, none is active) — where the room draws instead of
   * the shell. The registry is fetched anyway: it is the engine's base
   * registry without a plan (`commands` needs none), and the room
   * names commands of its own — the document's link is `app.gettingStarted`,
   * and the commands it does not carry are exactly the plan-scoped ones.
   */
  private async enterPicker(plans: PlanRead[]): Promise<void> {
    this.plan = null;
    this.phase = plans.length === 0 ? 'noneExists' : 'several';
    try {
      await this.loadRegistry();
    } catch (failure) {
      this.report(failure);
    }
    // The appearance resolves without a plan too (`appearance.resolve` needs
    // none), so the room draws in the mode the student chose rather
    // than in the document's defaults — the rail that normally carries the
    // switch is not drawn here.
    void this.resolveAppearance();
  }

  /** The registry and its keybindings for the current plan (§4.6, §8). */
  async loadRegistry(): Promise<void> {
    const registry = await dispatch('commands', {}, { plan: this.plan });
    const data = registry.data as CommandsRead;
    this.registry = data.commands;
    this.keybindings = data.keybindings;
  }

  /**
   * `plan.new` — the room creates the plan it will open (§3.2). The
   * preset is the caller's: `blank` when the app makes its own plan, or any id
   * the bundle ships when the screen's own row names one (`shell/presets.ts`).
   *
   * `parent` is the storage door (R11): the folder the student chose in the OS
   * dialog, else the plans root — and the destination rule stays the engine's
   * (`config_store::plan_destination`), so a name the terminal would refuse is
   * refused here with the engine's own words.
   */
  async createPlan(name: string, preset = 'blank', parent: string | null = null): Promise<void> {
    // The destination door, and nothing more: only a name the engine would accept
    // becomes a path, because `plan.new`'s `target` door is verbatim — a name with
    // a separator in it is handed over *as a name* and refused in the engine's own
    // words ("a name is one directory, not a path") rather than creating a tree.
    const target =
      parent !== null && name.trim().length > 0 && !/[\\/]/.test(name)
        ? joinPath(parent, name)
        : null;
    const created = await this.run(
      'plan.new',
      target === null ? { name, preset } : { target, preset },
      { tracked: false },
    );
    if (!created) return;
    const plan = (created.data as { plan?: string } | undefined)?.plan ?? null;
    if (plan) await this.openPlan(plan);
    else await this.boot();
  }

  /** The storage door (R11): where the next plan is written. `null` = the dialog
      did not run on this transport (the web harness has no OS picker), which is
      the room's cue to offer the path field instead. */
  async choosePlanParent(): Promise<string | null> {
    const chosen = await pickDirectory();
    if (chosen) this.planParent = chosen.replace(/[\\/]+$/, '');
    return chosen;
  }

  /** The open door (R11): a folder the student picks becomes the open plan. */
  async openPlanFolder(): Promise<boolean> {
    const chosen = await pickDirectory();
    if (!chosen) return false;
    await this.openPlan(chosen);
    return true;
  }

  /** Reads the memory back; anything that is not a path is dropped, never repaired. */
  private loadRecent(): void {
    try {
      const held: unknown = JSON.parse(localStorage.getItem('sam.recentPlans') ?? '[]');
      if (!Array.isArray(held)) return;
      this.recent = held
        .filter((path): path is string => typeof path === 'string' && path.length > 0)
        .slice(0, RECENT_MAX);
    } catch {
      // A first run, or a store this browser will not hand over: the list is
      // empty for this session and the plans root still lists what is on disk.
    }
  }

  /** Writes the memory: most recent first, one entry per plan, bounded. */
  private rememberPlan(path: string): void {
    const kept = [path, ...this.recent.filter((held) => held !== path)].slice(0, RECENT_MAX);
    this.recent = kept;
    try {
      localStorage.setItem('sam.recentPlans', JSON.stringify(kept));
    } catch {
      // The session still remembers it; only the next launch forgets.
    }
  }

  /** Everything the app needs before it can draw a screen. */
  async load(): Promise<void> {
    // The plan's own state is read once, for *a* plan: entering `load()` on a
    // different folder than the last one drops what the old folder produced
    // before anything is re-read. `reload()` (D9) comes through here too and
    // must keep the session's state — a revision from an agent's write does
    // not end a running timer — so the shedding is decided by the folder, not
    // by the call.
    if (this.loadedPlan !== this.plan) {
      this.shedPlanState();
      this.loadedPlan = this.plan;
    }
    const views = await dispatch('views', {}, { plan: this.plan });
    this.views = views.data as ViewsRead;
    this.today = this.views.today ?? null;
    void this.refreshReviewCount();
    void this.loadToday();
    this.restoreTimer();
    await this.loadRegistry();
    // The appearance is resolved from the same snapshot a screen is drawn from:
    // the pinned theme, the machine's mode and the text scale, all as data.
    void this.loadAppearance();

    const todayView = Object.entries(this.views.views).find(([, def]) => def.panel === 'today')?.[0];
    const first = todayView ?? this.navigation[0]?.view ?? Object.keys(this.views.views).sort()[0];
    this.selected = this.selected && this.views.views[this.selected] ? this.selected : (first ?? null);
    // A screen that no longer exists cannot be edited: a delete leaves the mode
    // rather than opening it on whatever screen loads next.
    if (this.screenEditing && this.screenDraftFor && !this.views.views[this.screenDraftFor]) {
      this.endScreenEdit();
    }
    if (this.selected) await this.refresh(this.selected);
  }

  /**
   * The state that belongs to *one* plan folder, dropped when the window moves
   * to another (`load()`): a session that would otherwise log its minutes into
   * a plan it did not run in, a screen draft and a source draft written against
   * the old plan's files, the undo stack of its writes, and the System
   * surface's read of its machine. The surfaces themselves (Settings, the
   * source pane, a sheet) keep their place: they are windows onto the plan, and
   * they re-read what they show.
   */
  private shedPlanState(): void {
    this.timer = null;
    this.undoStack = [];
    this.redoStack = [];
    this.selection = null;
    this.detail = null;
    this.recordCache = {};
    this.endScreenEdit();
    this.draft = null;
    this.externalPending = false;
    this.systemOpen = false;
    this.machine = null;
    this.machineSection = null;
    this.machineError = null;
  }

  /** Refreshes pending review item count for the review destination badge (R3). */
  async refreshReviewCount(): Promise<void> {
    if (!this.plan) {
      this.reviewCount = 0;
      return;
    }
    try {
      const result = await dispatch('reviews.due', {}, { plan: this.plan });
      const queue = result.data as { due?: unknown[] } | undefined;
      this.reviewCount = Array.isArray(queue?.due) ? queue.due.length : 0;
    } catch {
      this.reviewCount = 0;
    }
  }

  /** Loads daily facts projection from `today.view` (P2, U1). */
  async loadToday(): Promise<void> {
    if (!this.plan) {
      this.todayFacts = null;
      return;
    }
    try {
      const result = await dispatch('today.view', {}, { plan: this.plan });
      this.todayFacts = result.data as TodayFacts;
    } catch (failure) {
      this.report(failure);
    }
  }

  // ── session timer (F2) ─────────────────────────────────────────────────
  /** Finds the record type configured with both date and min fields for session logging. */
  sessionKind(): string | null {
    for (const [name, type] of Object.entries(this.types)) {
      const keys = (type.fields ?? []).map((field) => field.key);
      if (keys.includes('min') && keys.includes('date')) return name;
    }
    return null;
  }

  /** Finds the record type configured with the identity color role. */
  identityKind(): string | null {
    for (const [name, type] of Object.entries(this.types)) {
      if (type.colorRole === 'identity') return name;
    }
    return null;
  }

  /** Starts a study timer session for the given record or today item. */
  startSession(item: TodayItem | RecordDoc, minutes: number): void {
    const started = this.timer ?? null;
    if (started) {
      this.notice(`${started.item.label} is still running — stop it first`);
      return;
    }
    this.timer = {
      item: {
        id: item.id,
        label: 'label' in item ? item.label : item.id,
        kind: 'kind' in item ? item.kind : (item as RecordDoc).type,
        course: 'course' in item ? (item.course ?? null) : null,
      },
      startedAt: Date.now(),
      pausedMs: 0,
      pausedAt: null,
      targetMin: Math.max(1, Math.round(minutes)),
    };
    this.persistTimer();
  }

  /** Pauses or resumes the active study session. */
  togglePause(): void {
    const timer = this.timer;
    if (!timer) return;
    if (timer.pausedAt === null) {
      this.timer = { ...timer, pausedAt: Date.now() };
    } else {
      this.timer = {
        ...timer,
        pausedMs: timer.pausedMs + (Date.now() - timer.pausedAt),
        pausedAt: null,
      };
    }
    this.persistTimer();
  }

  /** Stops active session and commits duration record via `record.new` (§4.9). */
  async stopSession(): Promise<void> {
    const timer = this.timer;
    if (!timer) return;
    const elapsed = elapsedMs(timer, Date.now());
    this.timer = null;
    this.persistTimer();
    if (elapsed < 60_000) {
      this.notice('nothing logged — under a minute');
      return;
    }
    await this.logTime(timer.item, Math.round(elapsed / 60_000));
  }

  /** Log time after the fact — the same write, the duration chosen by hand. */
  async logTime(
    item: { id: string; label: string; kind: string; course: { id: string; label: string } | null },
    minutes: number,
  ): Promise<void> {
    const kind = this.sessionKind();
    if (!kind) {
      this.notice('this plan has no kind that records minutes');
      return;
    }
    const params: Record<string, unknown> = { min: String(Math.max(1, Math.round(minutes))) };
    const date = this.todayFacts?.date;
    if (date) params.date = date;
    if (item.course?.id) params.course = item.course.id;
    if (item.label) params.note = item.label;
    const week = this.todayFacts?.week?.id;
    if (week) params.week = week;
    const result = await this.run(`${kind}.new`, params);
    if (!result) return;
    this.notice(`${durationText(minutes)} on ${item.label}`, { label: 'Undo', run: () => void this.undo() });
    await this.loadToday();
  }

  private persistTimer(): void {
    try {
      if (this.timer) localStorage.setItem('sam.timer', JSON.stringify({ plan: this.plan, timer: this.timer }));
      else localStorage.removeItem('sam.timer');
    } catch {
      /* a private-mode window that refuses storage still runs the timer */
    }
  }

  /** A timer survives a relaunch; it does not survive a *different* plan. */
  private restoreTimer(): void {
    try {
      const raw = localStorage.getItem('sam.timer');
      if (!raw) return;
      const saved = JSON.parse(raw) as { plan?: string | null; timer?: TimerState };
      if (!saved.timer || saved.plan !== this.plan) return;
      this.timer = saved.timer;
    } catch {
      /* unreadable storage is a timer we never had */
    }
  }

  // ── intention markers (P2, U1) ─────────────────────────────────────────
  /** Pin record focus date to today (P2, U1). */
  async addToToday(id: string): Promise<void> {
    const date = this.todayFacts?.date;
    if (!date) return;
    await this.run('record.setField', { id, field: 'focus', value: date });
    await this.loadToday();
  }

  /** Clears record focus date field. */
  async removeFromToday(id: string): Promise<void> {
    await this.run('record.setField', { id, field: 'focus', value: null });
    await this.loadToday();
  }

  /** Re-reads the complete plan snapshot following an accepted revision (D9). */
  async reload(): Promise<void> {
    // No plan is open: there is nothing to re-read, and a plan-scoped read
    // would be refused — which on the room would land as a toast over
    // the starts (the watcher fires the moment the shell's plan folder
    // appears).
    if (!this.plan) return;
    this.busy = true;
    try {
      const paths = await dispatch('paths', {}, { plan: this.plan });
      this.paths = paths.data as PathsRead;
      this.revision = this.paths.revision;
      this.recordCache = {};
      await this.load();
      this.phase = 'ready';
    } catch (failure) {
      this.report(failure);
    } finally {
      this.busy = false;
    }
  }

  /** Refreshes a saved view projection (§4.9). */
  async refresh(view = this.selected): Promise<void> {
    if (!view) return;
    try {
      const result = await dispatch('view', { name: view }, { plan: this.plan });
      // Ignore responses for views that are no longer selected.
      if (view !== this.selected) return;
      this.outcome = result.data as ViewRead;
      // Discard draft when persisted components match active view (COMPOSER §3.2).
      if (
        this.screenEditing &&
        this.screenDraftFor === this.outcome.view &&
        this.outcome.components !== undefined
      ) {
        this.screenDraft = null;
      }
      if (this.selection && !this.rows().some((record) => record.id === this.selection)) {
        this.selection = null;
      }
    } catch (failure) {
      if (view !== this.selected) return;
      this.outcome = null;
      this.report(failure);
    }
  }

  /**
   * Records of one kind, for a relation picker (Appendix C.1: *"a menu over the
   * target type's records"*). Read once per kind and kept until the next
   * revision — the engine's snapshot is the truth, and a write invalidates it.
   */
  async recordsOf(type: string): Promise<RecordDoc[]> {
    if (!type) return [];
    const cached = this.recordCache[type];
    if (cached) return cached;
    const result = await this.run('records', { type }, { tracked: false });
    const records = ((result?.data as { records?: RecordDoc[] } | undefined)?.records ?? []);
    this.recordCache[type] = records;
    return records;
  }

  rows(): RecordDoc[] {
    const outcome = this.outcome;
    if (!outcome) return [];
    if (outcome.grouped && outcome.groups) return outcome.groups.flatMap((group) => group.records);
    return outcome.records ?? [];
  }

  /**
   * Where a resolved block sits in the screen: its **top-level** index, or
   * `null` when it is nested. `view.block.remove` and `view.block.set` address
   * `blocks/<index>`, so a nested block is edited through its parent's JSON
   * (`view.block.set` with a `blocks` value) rather than by an index that would
   * mean something different to the next reader.
   */
  blockIndex(node: BlockNode): number | null {
    const blocks = this.outcome?.blocks ?? [];
    const index = blocks.indexOf(node);
    return index >= 0 ? index : null;
  }

  // ── the one dispatch path ──────────────────────────────────────────────
  /**
   * Run one registry command. A **presentation** id never reaches the engine:
   * it is the app's own verb, and this is the single interception table the
   * palette, the menu and a keyboard shortcut all go through (Appendix C.3 —
   * otherwise `app.openSettings` works from ⌘K and does nothing from the menu).
   */
  async run(
    id: string,
    params: Record<string, unknown> = {},
    options: { ifRevision?: string | null; tracked?: boolean; quiet?: boolean } = {},
  ): Promise<DispatchResult | null> {
    // A presentation command is handled here, not by the engine — and it must
    // not wait on the registry's arrival to be recognised as one: the rail can
    // be clicked before `loadRegistry` answers, and a dispatch sent in that
    // window reaches the engine, which answers "no GUI" and changes nothing.
    // The engine's own list (§4.9) is fixed, so the switch below is the
    // authority and the registry only refines the *others*.
    if (PRESENT[id]) return this.present(id, params);
    const def = this.commandDef(id);
    if (def && def.effect === 'presentation') return this.present(id, params);

    this.pending += 1;
    try {
      const result = await dispatch(id, params, {
        plan: this.plan,
        ifRevision: options.ifRevision ?? null,
      });
      this.recordCache = {};
      if (options.tracked !== false && result.txid && result.revision) {
        this.undoStack = [
          ...this.undoStack,
          {
            txid: result.txid,
            revision: result.revision,
            title: `${def?.title ?? id} ${describing(params)}`.trim(),
          },
        ];
        this.redoStack = [];
        this.revision = result.revision;
        await publishWrite(this.plan);
      }
      await this.afterWrite(result.files ?? []);
      return result;
    } catch (failure) {
      // A refusal the caller handles itself (`quiet`): the engine's words are
      // kept where the caller reads them, and the student is not told about a
      // gate the caller is about to offer as the next step. Today's door is the
      // case: `pipeline.anchor-required` is the recall's invitation.
      if (options.quiet) this.diagnose(failure);
      else this.report(failure);
      return null;
    } finally {
      this.pending -= 1;
    }
  }

  /**
   * Refresh state modified by the write. Structural edits (schema/list/settings)
   * reload the plan; record edits only refresh the current view.
   */
  private async afterWrite(files: string[]): Promise<void> {
    if (files.length > 0) {
      void this.refreshReviewCount();
      void this.loadToday();
    }
    const structural = files.some((file) => file !== undefined && !file.startsWith('content/records/'));
    if (structural) {
      await this.load();
    } else if (this.selected) {
      await this.refresh(this.selected);
    }
    if (this.machine && files.length > 0) {
      await this.loadMachine();
    }
    if (files.some((file) => file === 'content/appearance.json')) {
      await this.resolveAppearance();
    }
  }

  // ── appearance (§6 Phase 7) ────────────────────────────────────────────
  /** Fetch theme catalog and active theme ID for settings swatches. */
  async loadAppearance(): Promise<void> {
    try {
      const listing = await dispatch('theme.list', {}, { plan: this.plan });
      const data = listing.data as { themes?: ThemeRead[]; active?: string };
      this.themes = data.themes ?? [];
      this.activeTheme = data.active ?? null;
    } catch (failure) {
      this.report(failure);
    }
    await this.resolveAppearance();
  }

  /** Resolve appearance for stored mode preference and apply to document. */
  async resolveAppearance(): Promise<void> {
    try {
      const resolved = await dispatch(
        'appearance.resolve',
        { mode: storedMode() },
        { plan: this.plan },
      );
      const appearance = resolved.data as AppearanceRead;
      this.appearance = appearance;
      this.activeTheme = appearance.theme;
      applyAppearance(appearance);
    } catch (failure) {
      this.report(failure);
    }
  }

  /** The machine's light/dark preference, stored where the pre-paint script reads it. */
  setMode(mode: 'light' | 'dark'): void {
    rememberMode(mode);
    void this.resolveAppearance();
  }

  /** Import a `*.samprofile` (§4.10) as a new plan and switch to it. */
  async importProfile(path: string): Promise<void> {
    const result = await this.run('profile.import', { file: path }, { tracked: false });
    if (!result) return;
    const plan = (result.data as { plan?: string } | undefined)?.plan;
    if (plan) {
      this.notice(`imported ${plan}`);
      await this.openPlan(plan);
    }
  }

  /** Import a `.samprofile` or open a plan directory from launch args. */
  async handleLaunchArgument(argument: string): Promise<void> {
    if (argument.toLowerCase().endsWith('.samprofile')) {
      await this.importProfile(argument);
    } else {
      await this.openPlan(argument);
    }
  }

  /** Export profile through native file dialog (§4.10). */
  async exportProfile(personal: boolean): Promise<void> {
    const chosen = await pickProfileOut();
    if (!chosen) {
      this.notice('this window has no save dialog — use SAM profile.export --out <file>');
      return;
    }
    const result = await this.run('profile.export', { out: chosen, personal }, { tracked: false });
    if (result) {
      const includes = (result.data as { includes?: { recordCount?: number } } | undefined)?.includes;
      this.notice(`exported ${chosen} · ${includes?.recordCount ?? 0} records`);
    }
  }

  /** Import profile through native file dialog. */
  async importProfileViaDialog(): Promise<void> {
    const chosen = await pickProfileFile();
    if (!chosen) {
      this.notice('this window has no open dialog — use SAM profile.import --file <file>');
      return;
    }
    await this.importProfile(chosen);
  }


  private async present(id: string, params: Record<string, unknown>): Promise<DispatchResult> {
    switch (id) {
      case 'app.palette':
        this.paletteOpen = true;
        break;
      case 'app.toggleSourcePane':
        this.sourcePane = !this.sourcePane;
        break;
      case 'app.openSettings':
        // Preferences is an OS window in the shell (§4.10) and a screen in the
        // browser; either way it is this one command.
        if (!(await openPreferences())) {
          this.paletteOpen = false;
          this.settingsOpen = true;
        }
        break;
      case 'app.gettingStarted':
        // Reopen getting-started sheet (R11).
        this.openSheet({ kind: 'gettingStarted' });
        break;
      case 'app.changePlan': {
        // The plan's own folder (§4.10 restated, in the window): the picker's
        // own material as a sheet. A `path` param opens that folder instead of
        // showing the choices — the door the palette, the menu and an agent's
        // JSON all take when the answer is already known.
        const path = typeof params.path === 'string' ? params.path.trim() : '';
        if (path.length > 0) await this.openPlan(path);
        else this.openSheet({ kind: 'plan.folder' });
        break;
      }
      case 'app.openSystem':
        // System view (Q10). Clear `selected` so rail doesn't highlight two destinations (`design.md` §6 rule 4).
        this.settingsOpen = false;
        this.paletteOpen = false;
        this.systemOpen = !this.systemOpen;
        if (this.systemOpen) {
          this.selected = null;
          if (typeof params.section === 'string') this.machineSection = params.section;
          await this.loadMachine();
        }
        break;
      case 'screen.edit':
        // Screen editor toggle (COMPOSER §4.2).
        if (this.screenEditing) {
          this.endScreenEdit();
        } else {
          this.settingsOpen = false;
          this.systemOpen = false;
          this.beginScreenEdit();
        }
        break;
      case 'rail.select':
        // Navigating to a view closes floating sheets (D7).
        this.settingsOpen = false;
        this.systemOpen = false;
        if (typeof params.view === 'string') await this.selectView(params.view);
        break;
      case 'view.edit':
        // View editor sheet (§4.8 P3).
        await this.openSheet({
          kind: 'view.edit',
          name: typeof params.name === 'string' && params.name.length > 0 ? params.name : (this.selected ?? ''),
        });
        break;
      case 'record.reveal':
        this.sourcePane = true;
        if (typeof params.id === 'string') {
          this.draft = null;
          this.selection = params.id;
        }
        break;
      // In-line record detail panel (§3).
      case 'record.panel': {
        const id = typeof params.id === 'string' ? params.id : null;
        const type = typeof params.type === 'string' ? params.type : (this.outcome?.type ?? '');
        if (!id) break;
        this.detail = this.detail?.id === id ? null : { id, type };
        if (this.detail) this.selection = id;
        break;
      }
      default:
        this.notice(`${id} has no handler in this window`);
    }
    return { ok: true, destination: id, available: true };
  }

  /** Open a sheet and fetch dry-run preview if destructive (Appendix C.2). */
  async openSheet(sheet: Sheet): Promise<void> {
    this.sheetPreview = null;
    this.lastDiagnostic = null;
    this.sheet = sheet;
    if (sheet?.kind === 'column.delete') {
      const result = await this.run(
        'column.delete',
        { spec: `${sheet.type}.${sheet.field.key}`, 'dry-run': true },
        { tracked: false },
      );
      this.sheetPreview = result ? JSON.stringify(result.data ?? {}, null, 2) : this.lastDiagnostic;
    } else if (sheet?.kind === 'record.delete') {
      const result = await this.run('record.delete', { id: sheet.record.id, 'dry-run': true }, { tracked: false });
      // Capture dry-run result or refusal diagnostics (referencing records).
      this.formatPreview(result);
    } else if (sheet?.kind === 'type.delete') {
      // Dry-run type deletion with records mode to show affected counts.
      const result = await this.run(
        'type.delete',
        { name: sheet.id, mode: 'records', 'dry-run': true },
        { tracked: false },
      );
      this.formatPreview(result);
    } else if (sheet?.kind === 'view.delete') {
      this.formatPreview(await this.run('view.delete', { name: sheet.name, 'dry-run': true }, { tracked: false }));
    } else if (sheet?.kind === 'screen.remove') {
      // Screen removal dry-run (§4.7).
      this.formatPreview(
        await this.run('view.delete', { name: sheet.view, 'dry-run': true }, { tracked: false }),
      );
    } else if (sheet?.kind === 'list.delete') {
      this.formatPreview(
        await this.run(
          'list.delete',
          { name: sheet.view, title: sheet.title, 'dry-run': true },
          { tracked: false },
        ),
      );
    }
  }

  /** Store dry-run output in `sheetData` or refusal error in `sheetPreview`. */
  private formatPreview(result: DispatchResult | null): void {
    this.sheetData = result ? (result.data ?? null) : null;
    this.sheetPreview = result ? null : this.lastDiagnostic;
  }

  closeSheet(): void {
    this.sheet = null;
    this.sheetPreview = null;
    this.sheetData = null;
  }

  /** Load machine status for the System surface. */
  async loadMachine(): Promise<void> {
    if (!this.plan) return;
    try {
      const result = await dispatch('system.view', {}, { plan: this.plan });
      this.machine = result.data as MachineRead;
      this.machineError = null;
    } catch (error) {
      this.machine = null;
      this.machineError = error instanceof IpcError ? error.text : String(error);
    }
  }

  /** Open a file directly in the source pane (D3). */
  openFile(file: string): void {
    this.paneFile = file;
    this.draft = null;
    this.sourcePane = true;
  }

  async selectView(view: string): Promise<void> {
    // Exit screen edit mode when navigating to a different view.
    if (view !== this.selected && this.screenEditing) this.endScreenEdit();
    this.selected = view;
    this.selection = null;
    await this.refresh(view);
  }

  // ── composing the screen (COMPOSER §3.2) ───────────────────────────────
  /**
   * Resolved component list for current screen: draft ?? outcome.components ?? default.
   * Normalizes spans to 1 or 2 (COMPOSER §2.1).
   */
  get screenComponents(): ScreenComponent[] {
    if (this.screenDraft !== null && this.screenDraftFor === this.outcome?.view) {
      return this.screenDraft;
    }
    const components = this.outcome?.components;
    if (components !== undefined) {
      return components.map((component) => ({
        surface: component.surface,
        span: component.span === 1 ? 1 : 2,
      }));
    }
    return defaultSurfaces(this.outcome?.panel).map((surface) => ({ surface, span: 2 as const }));
  }

  /** Surface identifiers for current screen components. */
  get screenSurfaces(): string[] {
    return this.screenComponents.map((component) => component.surface);
  }

  /** Whether the screen defines custom components. */
  get screenIsCustom(): boolean {
    return this.outcome?.components !== undefined;
  }

  /** Whether the screen has no components. */
  get screenIsEmpty(): boolean {
    return this.screenSurfaces.length === 0;
  }

  /** Whether the view uses default panel surfaces. */
  get screenHasDefault(): boolean {
    return !this.screenIsCustom && defaultSurfaces(this.outcome?.panel).length > 0;
  }

  /** Enter screen edit mode, seeding working draft from resolved components. */
  beginScreenEdit(): void {
    if (this.screenDraft === null || this.screenDraftFor !== this.outcome?.view) {
      this.screenDraft = this.screenComponents.map((component) => ({ ...component }));
      this.screenDraftFor = this.outcome?.view ?? this.selected;
    }
    this.screenEditing = true;
  }

  /** Exit screen edit mode and discard working draft. */
  endScreenEdit(): void {
    this.screenEditing = false;
    this.screenDraft = null;
    this.screenDraftFor = null;
  }

  /**
   * Commit screen components (§4.4). Skips dispatch if unchanged.
   * Writes both surface order and spans in one transaction.
   */
  async setScreenComponents(components: ScreenComponent[]): Promise<void> {
    const name = this.outcome?.view ?? this.selected;
    if (!name) return;
    const before = this.screenComponents;
    const unchanged =
      before.length === components.length &&
      before.every(
        (component, at) =>
          component.surface === components[at].surface && component.span === components[at].span,
      );
    if (unchanged) return;
    const previous = this.screenDraft;
    // Optimistically update draft; rollback if dispatch fails.
    this.screenDraft = components.map((component) => ({ ...component }));
    this.screenDraftFor = name;
    const result = await this.run('view.setComponents', {
      name,
      surfaces: components.map((component) => component.surface).join(','),
      spans: components.map((component) => component.span).join(','),
    });
    if (!result) {
      this.screenDraft = previous;
      this.screenDraftFor = previous === null ? null : name;
      return;
    }
    this.notice(receiptFor(before, components), {
      label: 'Undo',
      run: () => void this.undo(),
    });
  }

  /** Update screen surfaces while preserving existing component spans. */
  async setScreenSurfaces(surfaces: string[]): Promise<void> {
    const widths = new Map(this.screenComponents.map((component) => [component.surface, component.span]));
    return this.setScreenComponents(
      surfaces.map((surface) => ({ surface, span: widths.get(surface) ?? 2 })),
    );
  }

  /** Reset screen components to defaults (`view.setComponents --clear`, §4.2). */
  async resetScreen(): Promise<void> {
    const name = this.outcome?.view ?? this.selected;
    if (!name) return;
    this.screenDraft = null;
    if (!this.screenIsCustom) return;
    const result = await this.run('view.setComponents', { name, clear: true });
    if (!result) return;
    this.notice('Back to the design SAM made', { label: 'Undo', run: () => void this.undo() });
  }

  /** Open add-component sheet (§4.6). */
  openInserter(): void {
    void this.openSheet({ kind: 'component.insert' });
  }

  // ── undo and redo (§4.8 P6, Appendix C.5) ──────────────────────────────
  /**
   * Revert the most recent edit. If rejected due to concurrent revision,
   * prunes the entry from the undo stack.
   */
  async undo(): Promise<void> {
    const entry = this.undoStack[this.undoStack.length - 1];
    if (!entry) {
      this.notice('nothing to undo');
      return;
    }
    const result = await this.run('edit.undo', { txid: entry.txid }, {
      ifRevision: entry.revision,
      tracked: false,
    });
    if (!result) {
      this.undoStack = this.undoStack.filter((candidate) => candidate.revision !== entry.revision);
      this.notice(`undo skipped: the plan changed externally since “${entry.title}”`);
      return;
    }
    this.undoStack = this.undoStack.slice(0, -1);
    if (result.txid && result.revision) {
      this.redoStack = [...this.redoStack, { txid: result.txid, revision: result.revision, title: entry.title }];
    }
  }

  /** Reapply the most recently undone edit. */
  async redo(): Promise<void> {
    const entry = this.redoStack[this.redoStack.length - 1];
    if (!entry) {
      this.notice('nothing to redo');
      return;
    }
    const result = await this.run('edit.redo', { txid: entry.txid }, {
      ifRevision: entry.revision,
      tracked: false,
    });
    if (!result) {
      this.redoStack = this.redoStack.slice(0, -1);
      this.notice(`redo skipped: the plan changed externally since “${entry.title}”`);
      return;
    }
    this.redoStack = this.redoStack.slice(0, -1);
    if (result.txid && result.revision) {
      this.undoStack = [...this.undoStack, { txid: result.txid, revision: result.revision, title: entry.title }];
    }
  }

  // ── external change (D9, §4.8 P6) ──────────────────────────────────────
  /** Handle external plan change notification (D9, §4.8 P6). */
  handlePlanChanged(event: PlanChanged): void {
    if (event.valid === false) {
      this.notice(`the plan on disk does not load — ${event.message ?? 'run SAM --configcheck'}`);
      return;
    }
    if (!event.revision || event.revision === this.revision) return;
    if (this.draft) {
      this.externalPending = true;
      return;
    }
    void this.reload();
  }

  // ── small shared verbs ─────────────────────────────────────────────────
  report(failure: unknown): void {
    this.diagnose(failure);
    this.toast = this.lastDiagnostic;
    this.toastAction = null;
  }

  /**
   * The engine's own words for a failure, kept where a caller that handles the
   * refusal itself can read them (`run`'s `quiet`), without raising a toast.
   */
  diagnose(failure: unknown): void {
    this.lastDiagnostic =
      failure instanceof IpcError
        ? failure.text
        : String(failure instanceof Error ? failure.message : failure);
  }

  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  notice(message: string, action: ToastAction | null = null): void {
    this.toast = message;
    this.toastAction = action;
    if (this.toastTimer !== null) clearTimeout(this.toastTimer);
    // Auto-dismiss transient notices after 8s; persistent errors remain until cleared.
    this.toastTimer = action === null ? null : setTimeout(() => {
      this.toast = null;
      this.toastAction = null;
      this.toastTimer = null;
    }, 8000);
  }

  /** Update a record field value (Appendix C.1, §4.7). */
  async setField(record: RecordDoc, key: string, value: string | string[] | null): Promise<void> {
    await this.run('record.setField', { id: record.id, field: key, value });
  }

  /** Reveal a record in the source pane. */
  async reveal(record: RecordDoc): Promise<void> {
    await this.run('record.reveal', { id: record.id });
  }
}

/** The app's one session. */
export const app = new Session();
