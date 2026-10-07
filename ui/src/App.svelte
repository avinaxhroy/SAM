<!--
  App shell root view (§4.8, §4.10).

  Uses `data-window="flush"` for frameless native window styling.
  All UI interactions route through single registry commands via `app.run`
  to match CLI and command palette dispatch semantics.

  Global keyboard handling delegates ⌘Z / undo to focused inputs (§4.10 trap 2),
  falling back to app-level undo when no text field has focus.
-->
<script lang="ts">
  import Titlebar from './shell/Titlebar.svelte';
  import Menu from './shell/Menu.svelte';
  import Icon from './shell/Icon.svelte';
  import Block from './blocks/Block.svelte';
  import TodayPanel from './panels/Today.svelte';
  import Onboarding from './onboarding/Onboarding.svelte';
  import ThemeDisc from './shell/ThemeDisc.svelte';
  import PlanPanel from './panels/Plan.svelte';
  import CoursesPanel from './panels/Courses.svelte';
  import PracticePanel from './panels/Practice.svelte';
  import MocksPanel from './panels/Mocks.svelte';
  import ReferencePanel from './panels/Reference.svelte';
  import ReviewsPanel from './panels/Reviews.svelte';
  import ProgressPanel from './panels/Progress.svelte';
  import ColumnSheets from './records/ColumnSheets.svelte';
  import Settings from './records/Settings.svelte';
  import System from './system/System.svelte';
  import ViewEditor from './views/ViewEditor.svelte';
  import Palette from './commands/Palette.svelte';
  import SourcePane from './records/SourcePane.svelte';
  import RecordDetail from './records/RecordDetail.svelte';
  import PasteSheet from './records/PasteSheet.svelte';
  import NewRecordSheet from './records/NewRecordSheet.svelte';
  import ListDesigner from './records/ListDesigner.svelte';
  import NewColumnSheet from './records/NewColumnSheet.svelte';
  import TodayPicker from './records/TodayPicker.svelte';
  import Catchup from './panels/Catchup.svelte';
  import Screen from './composer/Screen.svelte';
  import Inserter from './composer/Inserter.svelte';
  import ScreenSheet from './composer/ScreenSheet.svelte';
  import PlanFolder from './shell/PlanFolder.svelte';
  import { commandForKey, fieldOwnsUndo, type MenuRow } from './commands/registry';
  import { RECORD_KINDS } from './blocks/registry';
  import { onLaunch, onMenuChoice, onOpenProfile, onPlanChanged, takePendingOpen, transport, watchPlan } from './ipc';
  import { dispatchMenuChoice } from './commands/menuBus';
  import { recordLabel, nameOf, type BlockNode, type FieldRead } from './types';
  import { app } from './session.svelte';

  let {
    os = 'mac',
    plan = null,
    preferences = false,
  }: { os?: 'mac' | 'win' | 'linux'; plan?: string | null; preferences?: boolean } = $props();

  /** Destination title resolved from navigation entry or view name (§4.1, D2). */
  const screenTitle = $derived(
    app.navigation.find((entry) => entry.view === app.selected)?.title ?? nameOf(app.selected ?? ''),
  );

  /** Active review view if declared by destination panel configuration (R3). */
  const reviewView = $derived(
    Object.entries(app.views?.views ?? {}).find(([, def]) => def.panel === 'reviews')?.[0] ?? null,
  );

  /** Display names for the eight layout kinds (§3.6). */
  const LAYOUT_WORDS: Record<string, string> = {
    list: 'List',
    table: 'Table',
    board: 'Board',
    timeline: 'Timeline',
    calendar: 'Calendar',
    tree: 'Outline',
    cardGrid: 'Cards',
    graph: 'Map',
  };

  function layoutWords(layout: string | null | undefined): string {
    if (!layout) return 'Layout';
    return LAYOUT_WORDS[layout] ?? layout;
  }

  /** Layout selection menu rows (§3.6). */
  const layoutRows = $derived<MenuRow[]>(
    RECORD_KINDS.map((kind) => ({
      id: 'view.setLayout',
      title: kind === app.outcome?.layout ? `${layoutWords(kind)} ✓` : layoutWords(kind),
      run: () => {
        const name = app.outcome?.view ?? app.selected ?? '';
        if (name) void app.run('view.setLayout', { name, layout: kind });
      },
    })),
  );
  let layoutOpen = $state(false);

  /**
   * Relation target records (Appendix C.1 picker), cached per revision.
   * `requested` is an untracked Set so writing to targets inside the effect
   * does not trigger cyclic execution.
   */
  let targets = $state<Record<string, Array<{ id: string; label: string }>>>({});
  const requested = new Set<string>();
  let resolvedFor: string | null = null;

  /** The types a resolved screen draws: every block that names one, recursively. */
  function typesIn(blocks: BlockNode[]): Set<string> {
    const kinds = new Set<string>();
    const walk = (nodes: BlockNode[]): void => {
      for (const node of nodes) {
        if (node.type) kinds.add(node.type);
        if (node.records) for (const record of node.records) kinds.add(record.type);
        if (node.instances) {
          for (const instance of node.instances) {
            kinds.add(instance.record.type);
            walk(instance.blocks);
          }
        }
        if (node.blocks) walk(node.blocks);
      }
    };
    walk(blocks);
    return kinds;
  }

  function targetsFor(field: FieldRead): Array<{ id: string; label: string }> {
    return targets[field.to ?? ''] ?? [];
  }

  async function loadTargets(kind: string): Promise<void> {
    if (kind.length === 0 || requested.has(kind)) return;
    requested.add(kind);
    const records = await app.recordsOf(kind);
    targets = {
      ...targets,
      [kind]: records.map((record) => ({ id: record.id, label: recordLabel(record) })),
    };
  }

  $effect(() => {
    const revision = app.revision;
    if (resolvedFor !== revision) {
      resolvedFor = revision;
      requested.clear();
      targets = {};
    }
    const wanted = new Set<string>();
    for (const typeName of typesIn(app.outcome?.blocks ?? [])) {
      for (const field of app.types[typeName]?.fields ?? []) {
        if (field.type === 'relation' && field.to) wanted.add(field.to);
      }
    }
    for (const kind of wanted) void loadTargets(kind);
  });

  function onkeydown(event: KeyboardEvent): void {
    // Focused text fields handle undo/copy/paste natively (§4.10).
    if (fieldOwnsUndo(event.target)) return;
    // Suppress global keyboard commands while the room is up: there is
    // no plan, so no command with a plan scope can run.
    if (app.plan === null) return;
    const id = commandForKey(event, app.keybindings, os);
    if (!id) return;
    if (id === 'edit.undo') {
      event.preventDefault();
      void app.undo();
    } else if (id === 'edit.redo') {
      event.preventDefault();
      void app.redo();
    } else if (!event.repeat) {
      event.preventDefault();
      void app.run(id);
    }
  }

  /** Appends a component to the active screen in edit mode (§4.6). */
  async function addComponent(surface: string): Promise<void> {
    app.beginScreenEdit();
    const current = app.screenSurfaces;
    app.closeSheet();
    if (current.includes(surface)) return;
    await app.setScreenSurfaces([...current, surface]);
  }

  /** Rendered DOM structure dump for parity gate verification (Appendix C.6, D11). */
  function renderDump(): Record<string, unknown> {
    const values = (attribute: string): Record<string, number> => {
      const counts: Record<string, number> = {};
      for (const node of document.querySelectorAll(`[${attribute}]`)) {
        const value = node.getAttribute(attribute) ?? '';
        counts[value] = (counts[value] ?? 0) + 1;
      }
      return counts;
    };
    const paletteScoped: Record<string, number> = {};
    for (const node of document.querySelectorAll('.cd-palette [data-command]')) {
      const value = node.getAttribute('data-command') ?? '';
      paletteScoped[value] = (paletteScoped[value] ?? 0) + 1;
    }
    // Scope queries to the canvas to exclude navigation rail disc attributes.
    const canvas = document.querySelector('.cd-canvas');
    const scope = (attribute) => canvas?.querySelector(`[${attribute}]`)?.getAttribute(attribute) ?? null;
    return {
      pane: scope('data-pane'),
      view: scope('data-view'),
      type: scope('data-type'),
      selected: app.selected,
      commands: values('data-command'),
      paletteCommands: paletteScoped,
      placements: values('data-placement'),
      columns: values('data-column'),
      settingsKeys: Object.keys(values('data-settings-key')),
      records: Object.keys(values('data-record-id')),
      menus: document.querySelectorAll('.cd-menu').length,
    };
  }

  $effect(() => {
    // One subscription, two transports (D9): the shell emits an event, the dev
    // harness streams the same publication.
    const offChange = onPlanChanged((event) => app.handlePlanChanged(event));
    // The menu bar and the context menus are the registry, projected; the id
    // they report lands in one bus, which runs it through `app.run`.
    const offMenu = onMenuChoice((id) => dispatchMenuChoice(id));
    // A second launch focuses this window and routes its argument here (§4.10):
    // a plan directory to open, or a `.samprofile` to import.
    const offLaunch = onLaunch((argv) => {
      const candidate = argv.find((argument) => !argument.startsWith('-'));
      if (candidate) void app.handleLaunchArgument(candidate);
      else app.notice('SAM is already running');
    });
    // The file association (§4.10): macOS delivers a double-click as an open
    // event, and a Windows/Linux double-click arrives as a second launch's
    // argv — both reach `handleLaunchArgument`, so one file has one behaviour.
    const offProfile = onOpenProfile((path) => {
      if (path) void app.handleLaunchArgument(path);
    });
    // An association launch that arrived before this window existed: the path
    // waited in the shell, and the app takes it now rather than dropping it.
    void takePendingOpen().then((pending) => {
      for (const path of pending) void app.handleLaunchArgument(path);
    });
    // The inspection hooks (Appendix C.6, option b): the rendered structure for
    // the parity gate, and the session itself so a gate — or a person with a
    // console — can ask what the app thinks it is showing. Dev runs and the
    // browser harness (`__SAM_WEB__`, which the packaged window never has)
    // only: in a shipped window these hooks are an engine-level write path
    // (`run()` dispatches any registry id) handed to any script that ever
    // lands in the page, so production installs neither.
    if (import.meta.env.DEV || transport() === 'web') {
      const hooks = globalThis as {
        __SAM_RENDER_DUMP__?: () => unknown;
        __SAM_APP__?: typeof app;
      };
      hooks.__SAM_RENDER_DUMP__ = renderDump;
      hooks.__SAM_APP__ = app;
    }
    return () => {
      offChange();
      offMenu();
      offLaunch();
      offProfile();
    };
  });

  $effect(() => {
    // Boot once. `paths` decides whether there is a plan to open; the picker is
    // a phase, not an error (Appendix C.5).
    if (app.phase !== 'booting') return;
    void app.boot(plan);
  });

  $effect(() => {
    // The shell watches the plan tree through the engine's own watcher (D9) and
    // publishes one event per accepted revision; the browser harness streams the
    // same publication from its own watcher.
    if (app.plan) void watchPlan(app.plan);
  });

  // Resets viewport scroll on view navigation (`design.md` §12). Keyed to destination, not revision.
  let scroller: HTMLElement | null = $state(null);
  let lastDestination: string | null | undefined = undefined;
  $effect(() => {
    const here = app.systemOpen ? 'system' : app.settingsOpen ? 'settings' : app.selected;
    if (lastDestination === here) return;
    lastDestination = here;
    if (scroller) scroller.scrollTop = 0;
  });

  // Active destination indicator: translates directly between discs without glyph sweep.
  // Toast departure delay to allow exit transition animation.
  let toastShown = $state<string | null>(null);
  let toastLeaving = $state(false);
  let leaveTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const next = app.toast;
    if (next !== null) {
      if (leaveTimer !== null) {
        clearTimeout(leaveTimer);
        leaveTimer = null;
      }
      toastShown = next;
      toastLeaving = false;
      return;
    }
    if (toastShown === null) return;
    toastLeaving = true;
    leaveTimer = setTimeout(() => {
      toastShown = null;
      toastLeaving = false;
      leaveTimer = null;
    }, 220);
    return () => {
      if (leaveTimer !== null) {
        clearTimeout(leaveTimer);
        leaveTimer = null;
      }
    };
  });

  let navEl: HTMLElement | null = $state(null);
  let indEl: HTMLElement | null = $state(null);
  let indStyle = $state('');
  let inked = $state<string[]>([]);

  let landedOn: HTMLElement | null = null;

  type Box = { l: number; t: number; w: number; h: number };

  // Closed disc bounds; offsetWidth during hover includes revealed label width.
  function discBox(el: Element): Box {
    const node = el as HTMLElement;
    return { l: node.offsetLeft, t: node.offsetTop, w: tokenPx('--disc', 44), h: node.offsetHeight };
  }

  function styleOf(box: Box, animate: boolean): string {
    const geometry = `left:${box.l}px;top:${box.t}px;width:${box.w}px;height:${box.h}px`;
    return animate
      ? `${geometry};transition:left var(--dur-2) var(--ease),top var(--dur-2) var(--ease),width var(--dur-2) var(--ease),height var(--dur-2) var(--ease);`
      : `${geometry};transition:none;`;
  }

  function keyOf(el: Element): string {
    return (el as HTMLElement).dataset.rail ?? (el as HTMLElement).dataset.view ?? 'system';
  }

  /** Navigation rail fluid pill physics (`chrome.css`). Damped spring across rest/contract/crawl/extend/home. */
  /** The tube's box, written as inline custom properties every frame. */
  let tubeEl: HTMLElement | null = $state(null);
  /** The disc the material is standing on, by rail key; `null` is the column. */
  let hugKey = $state<string | null>(null);
  /** Active rail state (`.is-hugging`), kept live through release settle. */
  let railLive = $state(false);
  /** The same disc as a DOM node for bounds measurement. */
  let hugEl: HTMLElement | null = null;
  /** Discs currently clearing exit transitions, tracked to smoothly close labels during sweeps. */
  const leavers = new Set<HTMLElement>();
  /** Last written reveal ratio per disc. */
  const lastReveal = new Map<HTMLElement, number>();
  /** Discs whose label reveal states are updated during animation frames. */
  let covered: HTMLElement[] = [];
  /** Cached label regions per disc, queried on aim to avoid layout thrashing. */
  const regions = new Map<HTMLElement, Box>();

  /** Critically damped spring constants (ζ ≈ 0.94-0.95): position spring and softer size spring. */
  const POS = { k: 220, c: 28 };
  const SIZE = { k: 140, c: 22 };
  /** Sub-pixel and still: the material is where it was asked to be. */
  const AT_REST = 0.15;
  const STILL = 2;
  const AXES = ['l', 't', 'w', 'h'] as const;

  /** Where the material is, where it is going, and how fast it is going there. */
  let cur: Box = { l: 0, t: 0, w: 0, h: 0 };
  let vel: Box = { l: 0, t: 0, w: 0, h: 0 };
  let goal: Box = { l: 0, t: 0, w: 0, h: 0 };
  /** The crawl's deformation, smoothed against the frame's own velocity. */
  let stretch = 0;
  /** The box on screen: `cur`, deformed by `stretch` — written in place by
      `paint`, and the box every reveal is a function of. */
  const painted: Box = { l: 0, t: 0, w: 0, h: 0 };
  let frameId: number | null = null;
  let lastFrame = 0;

  /**
   * Where the material is in the gesture: at rest on the column, retracting at
   * the disc it is leaving, travelling to the one it was aimed at, opening at
   * it, or — on release — flowing home to the column. The phase is the *order*
   * the third pass asked for (see the block
   * comment) and it is read and written only by `planGoal`, one frame at a time —
   * an interruption leaves the phase where it was and the goals re-plan from the
   * material's own box.
   */
  type Phase = 'rest' | 'contract' | 'crawl' | 'extend' | 'home';
  let phase = $state<Phase>('rest');
  /** The disc the material is retracting at, when the aim came off a settled
      pill: a contract happens where the material *stands*. `null` means it is
      coming from the column, whose contract target is the destination itself. */
  let contractAt: HTMLElement | null = null;
  /** A phase ends when the material's box is this close to the phase's target —
      8px of size for the square, 6px of position for the arrival. */
  const CONTRACT_EPS = 8;
  const CRAWL_EPS = 6;

  /** Whether the platform asks for less motion: the material then jumps. */
  function quietMotion(): boolean {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  }

  /** One token as a number. The script times its own beats, so it *reads* the
      register rather than repeating it — and a token that changed value in
      `tokens.css` cannot leave the motion behind. */
  function tokenPx(name: string, fallback: number): number {
    if (!navEl) return fallback;
    const value = Number.parseFloat(getComputedStyle(navEl).getPropertyValue(name));
    return Number.isFinite(value) && value > 0 ? value : fallback;
  }

  /** One disc's open pill: the disc plus the label it reveals, from the tokens. */
  function pillWidth(): number {
    return tokenPx('--disc', 44) + tokenPx('--rail-label', 70);
  }

  /** The material at rest: the column's own box — and it is the stylesheet's
      own `0/0/100%/100%`, not a number kept here.
      The column *stretches* with what the view renders, so a rest box measured
      once is stale the moment a load lands: measured on a cold start, the rail
      was 656 tall at mount and 733 once the screen's rows were in, and the glass
      ended 65px above its last disc until the first hover wrote the truth back.
      Clearing the geometry puts the material on the CSS defaults, which
      describe the column by construction — while a *hug* is still measured,
      because a disc's box is not the sheet's. */
  function restBox(): Box {
    return { l: 0, t: 0, w: navEl?.offsetWidth ?? 0, h: navEl?.offsetHeight ?? 0 };
  }

  /** Hand the geometry back to the stylesheet. The numbers written here equal
      its own defaults at the moment this runs, so nothing moves. */
  function clearGeometry(): void {
    tubeEl?.style.removeProperty('--tube-x');
    tubeEl?.style.removeProperty('--tube-y');
    tubeEl?.style.removeProperty('--tube-w');
    tubeEl?.style.removeProperty('--tube-h');
  }

  /** Back at the column: every name at zero, the leaver done. Without this the
      last disc's reveal would hold the column's own 16px overlap for good — the
      material *is* the column again, and its edge stands 16px into every label
      region. The class stays on (see `chrome.css`): a stale bared disc at a zero
      reveal is the same pixels as a closed one, and un-marking it would replay
      the system's transitions after the edge had already closed the name. */
  function closeNames(): void {
    for (const el of covered) {
      el.style.setProperty('--reveal', '0px');
      el.style.setProperty('--reveal-o', '0');
    }
    leavers.clear();
  }

  /** One disc's open pill: its own box, opened to the label's width. */
  function pillBox(el: HTMLElement): Box {
    const box = discBox(el);
    return { l: box.l, t: box.t, w: pillWidth(), h: box.h };
  }

  /**
   * The label region the material has to cover to reveal a name: the disc's own
   * width plus the label's, measured from the tokens rather than from the
   * element, whose width is mid-flight whenever this runs.
   */
  function labelRegion(el: HTMLElement): Box {
    const box = discBox(el);
    const disc = tokenPx('--disc', 44);
    return { l: box.l + disc, t: box.t, w: tokenPx('--rail-label', 70), h: disc };
  }

  // Computes pixel overlap between animated tube and disc label region.
  function revealFor(tube: Box, region: Box): number {
    const across = Math.min(tube.l + tube.w, region.l + region.w) - Math.max(tube.l, region.l);
    const down = Math.min(tube.t + tube.h, region.t + region.h) - Math.max(tube.t, region.t);
    if (across <= 0 || down <= 0) return 0;
    return Math.min(across, region.w) * Math.min(1, down / region.h);
  }

  // Fades label opacity as reveal approaches full width.
  function revealOpacity(reveal: number, width: number): number {
    const fade = width * 0.12;
    return Math.max(0, Math.min(1, (reveal - (width - fade)) / fade));
  }

  /** Velocity stretch scale and clamp for crawling phase. */
  const STRETCH_PER_VEL = 0.045;
  const STRETCH_MAX = 22;

  function paint(stretch = 0): void {
    painted.l = cur.l + stretch * 0.2;
    painted.t = cur.t - stretch / 2;
    painted.w = Math.max(8, cur.w - stretch * 0.4);
    painted.h = cur.h + stretch;
    if (tubeEl) {
      tubeEl.style.setProperty('--tube-x', `${painted.l}px`);
      tubeEl.style.setProperty('--tube-y', `${painted.t}px`);
      tubeEl.style.setProperty('--tube-w', `${painted.w}px`);
      tubeEl.style.setProperty('--tube-h', `${painted.h}px`);
    }
    for (const el of covered) {
      const region = regions.get(el) ?? labelRegion(el);
      regions.set(el, region);
      el.classList.add('is-bared');
      const reveal = revealFor(painted, region);
      const last = lastReveal.get(el) ?? 0;
      if (leavers.has(el) && (reveal <= 0 || reveal > last)) leavers.delete(el);
      const live = el === hugEl || leavers.has(el);
      if (!live && el.style.getPropertyValue('--reveal') === '0px') continue;
      const write = live ? reveal : 0;
      lastReveal.set(el, write);
      el.style.setProperty('--reveal', `${write}px`);
      el.style.setProperty('--reveal-o', `${revealOpacity(write, region.w)}`);
    }
  }

  // Advances physics simulation by dt seconds; returns whether movement continues.
  function step(dt: number): boolean {
    let moving = false;
    for (const axis of AXES) {
      const spring = axis === 'w' || axis === 'h' ? SIZE : POS;
      const accel = -spring.k * (cur[axis] - goal[axis]) - spring.c * vel[axis];
      const nextVel = vel[axis] + accel * dt;
      const next = cur[axis] + nextVel * dt;
      if (Math.abs(next - goal[axis]) > AT_REST || Math.abs(nextVel) > STILL) {
        cur[axis] = next;
        vel[axis] = nextVel;
        moving = true;
      } else {
        cur[axis] = goal[axis];
        vel[axis] = 0;
      }
    }
    return moving;
  }

  // Evaluates target bounds for the active gesture phase.
  function planGoal(): void {
    const dest = hugEl;
    if (dest === null) {
      if (phase === 'contract' && contractAt !== null) {
        goal = discBox(contractAt);
        if (Math.abs(cur.w - goal.w) < CONTRACT_EPS && Math.abs(cur.h - goal.h) < CONTRACT_EPS) {
          phase = 'home';
          contractAt = null;
        }
        return;
      }
      if (phase !== 'rest') phase = 'home';   // a crawl, an interrupted contract: it turns
      if (phase === 'home') {
        goal = restBox();
        if (
          Math.abs(cur.l - goal.l) < CRAWL_EPS && Math.abs(cur.t - goal.t) < CRAWL_EPS &&
          Math.abs(cur.w - goal.w) < CONTRACT_EPS && Math.abs(cur.h - goal.h) < CONTRACT_EPS
        ) {
          phase = 'rest';
        }
        return;
      }
      goal = restBox();
      return;
    }
    if (phase === 'rest') phase = 'extend';   // aimed with nowhere to travel: settled
    if (phase === 'extend') {
      goal = pillBox(dest);
      return;
    }
    if (phase === 'contract') {
      const at = contractAt ?? dest;
      goal = discBox(at);
      if (Math.abs(cur.w - goal.w) < CONTRACT_EPS && Math.abs(cur.h - goal.h) < CONTRACT_EPS) {
        phase = 'crawl';
      }
      return;
    }
    // crawl — the destination's own square, and its arrival ends the phase
    goal = discBox(dest);
    if (Math.abs(cur.l - goal.l) < CRAWL_EPS && Math.abs(cur.t - goal.t) < CRAWL_EPS) {
      phase = 'extend';
    }
  }

  function frame(now: number): void {
    // The reference's own clamp: a tab that was asleep must not integrate a
    // second of physics in one step.
    const dt = Math.min((now - lastFrame) / 1000, 1 / 30);
    lastFrame = now;
    planGoal();
    const moving = step(dt);
    // The crawl's deformation, read here and not in `paint` (see its comment).
    // It follows the rate with a short lag, because the velocity itself starts
    // from zero where the contract ends: a stretch that snapped to its target in
    // that one frame would be the pop the whole rewrite exists to remove (the
    // measured onset at the cap's 22px, against a 6px/frame continuity budget).
    const want = phase === 'crawl' ? Math.min(STRETCH_MAX, Math.abs(vel.t) * STRETCH_PER_VEL) : 0;
    stretch += (want - stretch) * Math.min(1, dt * 12);
    paint(stretch);
    if (moving) {
      frameId = requestAnimationFrame(frame);
    } else {
      frameId = null;
      if (hugEl === null) {
        // Settled on the column: every name was closed by the edge that closed
        // it, and the geometry goes back to the stylesheet.
        railLive = false;
        clearGeometry();
        closeNames();
      }
    }
  }

  /** Start the integration if it is not already running. The *target* is not
      set here any more: `planGoal` owns it, one frame at a time, because the
      phase is what decides where the material is going. */
  function kick(): void {
    if (frameId === null) {
      lastFrame = performance.now();
      frameId = requestAnimationFrame(frame);
    }
  }

  /** Put the material somewhere with no motion at all: a re-measure. */
  function snap(box: Box): void {
    if (frameId !== null) {
      cancelAnimationFrame(frameId);
      frameId = null;
    }
    cur = { ...box };
    goal = { ...box };
    vel = { l: 0, t: 0, w: 0, h: 0 };
    stretch = 0;      // a snap has no rate, so the body carries no deformation
    paint();
  }

  /** Where the spring starts when the material is at rest on the column: it *is*
      the column. A fresh mount has never painted a box, so it has to be read
      before the first aim — and a settled hug must *not* be reset this way, or
      the material would jump to the column's box before it left the disc. */
  function startFromRest(): void {
    if (frameId !== null || hugEl !== null) return;
    cur = restBox();
    vel = { l: 0, t: 0, w: 0, h: 0 };
  }

  /**
   * Aim the material at one disc. Where it *was* decides how it leaves: a
   * settled pill contracts at the disc it stands on before it travels, the
   * column gathers toward this one, and a body mid-crawl redirects to a
   * new destination — the phase, its target and the material's own box carry
   * the state, so an interruption mid-crawl is the same worm turning rather
   * than a new gesture. This runs on every element boundary the pointer
   * crosses, so it is the interruption path as well as the start of one.
   */
  function aimTube(el: HTMLElement): void {
    if (el === hugEl) return;          // the same disc: the pointer crossed one of its children
    startFromRest();
    railLive = true;                   // from the first aim to the settle
    if (hugEl !== null) leavers.add(hugEl);   // live until the edge has cleared its region
    if (hugEl !== null && !covered.includes(hugEl)) covered.push(hugEl);
    if (!covered.includes(el)) covered.push(el);
    regions.set(el, labelRegion(el));
    // The destination's own glass is cancelled *now*, not on the next frame:
    // `is-bared` is what makes the material this disc's background rather than a
    // second pill beside it.
    for (const node of covered) node.classList.add('is-bared');
    el.style.setProperty('--reveal', '0px');
    el.style.setProperty('--reveal-o', '0');
    if (phase === 'rest') {
      phase = 'contract';
      contractAt = null;               // from the column: the contract target is the destination
    } else if (phase === 'extend') {
      phase = 'contract';
      contractAt = hugEl;              // off a settled pill: retract where it stands
    } else if (phase === 'home') {
      phase = 'crawl';                 // flowing home: the body turns toward the new disc
    }
    hugEl = el;
    hugKey = keyOf(el);
    if (quietMotion()) {
      // No motion at all: the material is placed directly, and the names are
      // revealed (or closed) by the box it landed in.
      phase = 'extend';
      snap(pillBox(el));
      return;
    }
    kick();
  }

  // Animates indicator contraction and returns to column rest state on pointer leave.
  function releaseTube(): void {
    if (hugEl !== null && !covered.includes(hugEl)) covered.push(hugEl);
    if (hugEl !== null) leavers.add(hugEl);
    if (phase === 'extend') {
      phase = 'contract';
      contractAt = hugEl;
    } else if (phase === 'crawl' || (phase === 'contract' && contractAt === null)) {
      phase = 'home';
      contractAt = null;
    }
    hugEl = null;
    hugKey = null;
    if (quietMotion()) {
      phase = 'rest';
      contractAt = null;
      snap(restBox());
      railLive = false;
      clearGeometry();
      closeNames();
      return;
    }
    kick();
  }

  function discAt(target: EventTarget | null): HTMLElement | null {
    if (!(target instanceof Element) || !navEl) return null;
    const disc = target.closest<HTMLElement>('.cd-disc');
    return disc !== null && navEl.contains(disc) ? disc : null;
  }

  // Pointer and focus tracking for navigation rail discs.
  function onRailAim(event: PointerEvent | FocusEvent): void {
    if (event.type === 'focusout') {
      // Releasing focus outside navigation rail returns the indicator to rest.
      if (discAt(event.relatedTarget) !== null) return;
      releaseTube();
      return;
    }
    const disc = discAt(event.target);
    if (disc) {
      aimTube(disc);
      return;
    }
    // Gaps between discs inside the rail maintain current aim target.
    if (event.target instanceof Element && navEl?.contains(event.target)) return;
    releaseTube();
  }

  // Updates active destination indicator position.
  function place(_from: HTMLElement | null, to: HTMLElement): void {
    if (!indEl) return;
    landedOn = to;
    indStyle = styleOf(discBox(to), true);
    inked = [keyOf(to)];
  }

  /** The disc the rail is standing on, read from the DOM — one owner for both
      destinations and the machine's own place. */
  function currentDisc(): HTMLElement | null {
    return navEl?.querySelector<HTMLElement>('.cd-disc[aria-current="page"]') ?? null;
  }

  $effect(() => {
    // tracked: the two halves of "where you are"
    void app.selected;
    void app.systemOpen;
    void app.settingsOpen;
    const target = currentDisc();
    if (!target) return;
    const from = landedOn;
    landedOn = target;
    if (from === target) return;
    place(from, target);
  });

  $effect(() => {
    // Below the system's restack the rail is a top bar and the indicator's axis
    // changes with the window: re-measure, never re-animate. The tube is
    // measured in the same pass, for the same reason — a window that changed
    // shape under the material is not a hover, so nothing here animates.
    const settleAxis = (): void => {
      const target = currentDisc();
      if (target) {
        landedOn = target;
        indStyle = styleOf(discBox(target), false);
        inked = [keyOf(target)];
      }
      // A window that changed shape under the material is not a hover: no
      // motion is started, and the material is put where it belongs. The disc it
      // was aimed at has arrived by now, so it settles *open* — the state the
      // pointer is still asking for.
      if (hugEl?.isConnected) {
        railLive = true;
        regions.set(hugEl, labelRegion(hugEl));
        phase = 'extend';               // settled *open*: the pointer still asks for it
        snap(pillBox(hugEl));
      } else {
        railLive = false;
        phase = 'rest';
        contractAt = null;
        snap(restBox());
        clearGeometry();
        closeNames();
      }
    };
    settleAxis();
    window.addEventListener('resize', settleAxis);
    /* The window's `resize` is not the only way the column changes shape: it
       stretches with the view it holds, so a screen whose rows land after the
       first paint moves it (measured: 656 → 733 on a cold start, with the discs
       carried 77px down). A hugged pill is one measurement away from being
       wrong when that happens, so the column's own box is observed and the same
       no-beat settle runs — gated on a changed size, because only a box that
       changed needs re-measuring. */
    let lastW = navEl?.clientWidth ?? 0;
    let lastH = navEl?.clientHeight ?? 0;
    const observer = new ResizeObserver(() => {
      if (!navEl || (navEl.clientWidth === lastW && navEl.clientHeight === lastH)) return;
      lastW = navEl.clientWidth;
      lastH = navEl.clientHeight;
      settleAxis();
    });
    if (navEl) observer.observe(navEl);
    return () => {
      window.removeEventListener('resize', settleAxis);
      observer.disconnect();
      if (frameId !== null) cancelAnimationFrame(frameId);
    };
  });
</script>

<svelte:window onkeydown={onkeydown} />

<!--
  Flush window layout (UI_SPEC R1 · deviation S1 · BUILDLOG S1).
  data-window="flush" renders the app edge-to-edge without redundant nested window
  frames or inset gaps, while the OS retains native window decorations and controls
  (tauri.conf.json decorations: true).
-->
<div class="cd-window" data-window="flush">
  <!-- Titlebar actions are hidden while no plan is open (`quiet`): every
       object up there needs one. -->
  <Titlebar {os} quiet={app.plan === null} />

  {#if preferences}
    <!-- Preferences standalone window (§4.10). -->
    <div class="cd-scroller cd-scroll">
      <main class="cd-canvas">
        <Settings {os} />
      </main>
    </div>
  {:else if app.phase === 'several' || app.phase === 'noneExists'}
    <!-- No plan is open (R11, Appendix C.5): the room *is* the window. Nothing
         on disk puts it on its first screen; plans already there open it on its
         last, where the rows are — the movements are still one press back. -->
    <Onboarding />
  {:else}
    <div class="cd-body">
      <!-- Navigation rail: destinations, system configuration, list creation, and theme toggle (design.md §8.1, D12). -->
      <nav
        class="cd-nav"
        class:is-hugging={railLive}
        class:is-flowing={phase === 'contract' || phase === 'crawl' || phase === 'home'}
        aria-label="Destinations"
        bind:this={navEl}
        onpointerover={onRailAim}
        onpointerleave={() => releaseTube()}
        onfocusin={onRailAim}
        onfocusout={onRailAim}
      >
        <!-- Background fluid glass tube (`chrome.css`). -->
        <span class="cd-nav__tube" bind:this={tubeEl} aria-hidden="true"></span>
        <!-- Active destination indicator capsule. -->
        <span class="cd-nav__ind" bind:this={indEl} style={indStyle} aria-hidden="true"></span>
        {#each app.navigation as entry (entry.view)}
          {@const due = entry.view === reviewView && app.reviewCount > 0 ? app.reviewCount : null}
          <button
            class="cd-disc"
            class:cd-disc--date={entry.icon === 'date'}
            class:is-inked={inked.includes(entry.view)}
            type="button"
            aria-current={app.selected === entry.view ? 'page' : undefined}
            aria-label={due === null ? entry.title : `${entry.title} — ${due} due`}
            data-command="rail.select"
            data-placement="sidebar"
            data-view={entry.view}
            data-rail={entry.view}
            onclick={() => app.run('rail.select', { view: entry.view })}
          >
            <span class="cd-disc__ico">
              {#if entry.icon === 'date' && app.today}
                <b>{app.today.day}</b>
                <i>{app.today.weekday}</i>
              {:else}
                <Icon name={entry.icon ?? 'dot'} />
              {/if}
            </span>
            <span class="cd-disc__label">{entry.title}</span>
            {#if due !== null}
              <span class="cd-disc__n">{due}</span>
            {/if}
          </button>
        {/each}

        <!-- System configuration surface (S14, D13). -->
          <button
            class="cd-disc"
            class:cd-disc--machine={app.systemOpen}
            class:is-inked={inked.includes('system')}
            type="button"
            aria-current={app.systemOpen ? 'page' : undefined}
            aria-label="System — kinds, columns, views, rules, schedules"
            title="System — kinds, columns, views, rules, schedules"
            data-command="app.openSystem"
            data-placement="sidebar"
            data-rail="system"
            onclick={() => app.run('app.openSystem')}
          >
            <span class="cd-disc__ico"><Icon name="cube" /></span>
            <span class="cd-disc__label">System</span>
          </button>

          <span class="cd-nav__spacer"></span>
          <span class="cd-nav__sep"></span>

          <button
            class="cd-disc cd-disc--add"
            type="button"
            title="Add a list, or a new kind of thing"
            aria-label="Add a list, or a new kind of thing"
            data-command="list.new"
            data-placement="sidebar.add"
            data-rail="add"
            onclick={() => app.openSheet({ kind: 'list.new' })}
          >
            <span class="cd-disc__ico"><Icon name="plus" /></span>
            <span class="cd-disc__label">Add</span>
          </button>

        <!-- Theme mode switch (`shell/ThemeDisc.svelte`). -->
        <ThemeDisc />
      </nav>

      <div class="cd-scroller cd-scroll" bind:this={scroller}>
        <main class="cd-canvas">
          <!-- Error boundary isolating view rendering errors from the shell. -->
          <svelte:boundary>
            {#snippet failed(error)}
              <section class="cd-card">
                <h2 class="cd-card__title">This screen could not be drawn</h2>
                <p class="cd-card__sub">{String(error)}</p>
                <p class="cd-card__sub">The data is untouched. Pick another destination, or open SAM --configcheck.</p>
              </section>
            {/snippet}
          {#if app.systemOpen}
            <System />
          {:else if app.settingsOpen}
            <Settings {os} />
          {:else if app.outcome && (app.outcome.components !== undefined || app.screenEditing)}
            <!-- Composer screen (COMPOSER §3.3): renders component stack when defined or editing. -->
            <Screen title={screenTitle} />
          {:else if app.outcome}
            <!-- Panel views (D6, D8): renders designated built-in panel for the destination. -->
            {#if app.outcome.panel === 'today'}
              <TodayPanel />
            {:else if app.outcome.panel === 'plan'}
              <PlanPanel title={screenTitle} />
            {:else if app.outcome.panel === 'subjects'}
              <CoursesPanel title={screenTitle} />
            {:else if app.outcome.panel === 'practice'}
              <PracticePanel title={screenTitle} />
            {:else if app.outcome.panel === 'mocks'}
              <MocksPanel title={screenTitle} />
            {:else if app.outcome.panel === 'library' || app.outcome.panel === 'notes'}
              <ReferencePanel title={screenTitle} />
            {:else if app.outcome.panel === 'reviews'}
              <ReviewsPanel type={app.outcome.type} title={screenTitle} />
            {:else if app.outcome.panel === 'progress'}
              <ProgressPanel type={app.outcome.type} title={screenTitle} />
            {:else if app.outcome.panel}
              <header class="cd-pagehead">
                <h1 class="cd-pagehead__title">{screenTitle}</h1>
              </header>
              <section class="cd-card">
                <h2 class="cd-card__title">unknown panel: “{app.outcome.panel}”</h2>
                <p class="cd-card__sub">
                  known: today · plan · subjects · practice · mocks · library · notes · reviews · progress
                  — the name a view declares in its <em>panel</em> field.
                </p>
              </section>
            {:else}
              <!-- Screen header: destination title and layout controls (R4, D2, D3). -->
              <header class="cd-pagehead">
                <h1 class="cd-pagehead__title">{screenTitle}</h1>
                <span class="cd-pagehead__aside">
                  <span class="cd-menu-wrap">
                    <button
                      class="cd-pill cd-pill--quiet cd-pill--sm"
                      type="button"
                      aria-haspopup="menu"
                      aria-expanded={layoutOpen}
                      data-command="view.setLayout"
                      data-placement="view.head"
                      title="Draw this view as another layout — the same command the terminal has"
                      onclick={(event) => {
                        // Prevent document click handler from immediately closing the opened menu.
                        event.stopPropagation();
                        layoutOpen = !layoutOpen;
                      }}
                    >
                      {layoutWords(app.outcome.layout)} ▾
                    </button>
                    {#if layoutOpen}
                      <Menu rows={layoutRows} native={false} onclose={() => (layoutOpen = false)} />
                    {/if}
                  </span>
                  <button
                    class="cd-pill cd-pill--quiet cd-pill--sm"
                    type="button"
                    data-command="view.edit"
                    data-placement="view.head"
                    onclick={() => void app.run('view.edit', { name: app.outcome?.view ?? '' })}
                  >
                    Edit view…
                  </button>
                </span>
              </header>

              <div class="blocks">
                {#each app.outcome.blocks ?? [] as node, index (index)}
                  <Block {node} {targetsFor} />
                {/each}
              </div>

              {#if (app.outcome.blocks ?? []).length === 0}
                <section class="cd-card">
                  <h2 class="cd-card__title">This view draws nothing yet</h2>
                  <p class="cd-card__sub">
                    It has no blocks — open <em>Edit view…</em> to add one, or set a layout on the view itself.
                  </p>
                </section>
              {/if}
            {/if}
          {:else if !app.busy}
            <section class="cd-card">
              <h2 class="cd-card__title">No view selected</h2>
              <p class="cd-card__sub">Pick a destination on the rail, or add a list.</p>
            </section>
          {/if}

          {#if app.busy && !app.outcome}
            <!-- R19: skeleton matches view row geometry. -->
            <section class="cd-card" aria-busy="true">
              <p class="cd-sr" role="status">Loading this screen…</p>
              <div class="cd-skel__rows">
                {#each [0, 1, 2, 3, 4] as row (row)}
                  <div class="cd-skel__row">
                    <span class="cd-skel"></span>
                    <span class="cd-skel" style={`--w: ${row % 2 === 0 ? 62 : 44}%`}></span>
                    <span class="cd-skel"></span>
                  </div>
                {/each}
              </div>
            </section>
          {/if}
          </svelte:boundary>
        </main>
      </div>

      {#if app.sourcePane}
        <SourcePane />
      {/if}

      {#if app.detail}
        <RecordDetail />
      {/if}
    </div>
  {/if}

  {#if app.paletteOpen}
    <Palette />
  {/if}

  {#if app.sheet?.kind === 'paste'}
    <PasteSheet type={app.sheet.type} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'record.new'}
    <NewRecordSheet type={app.sheet.type} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'column.new'}
    <NewColumnSheet type={app.sheet.type} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'view.edit'}
    <ViewEditor name={app.sheet.name} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'list.new'}
    <ListDesigner onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'today.add'}
    <TodayPicker onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'catchup'}
    <Catchup items={app.sheet.items} label={app.sheet.label} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'gettingStarted'}
    <!-- The document, re-readable over a working plan (R11): the same three
         screens, with the shut control the first-run room has no use for. -->
    <Onboarding overlay onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'component.insert'}
    <Inserter
      used={app.screenSurfaces}
      onadd={(surface) => void addComponent(surface)}
      onclose={() => app.closeSheet()}
    />
  {:else if app.sheet?.kind === 'screen.rename'}
    <ScreenSheet what="rename" view={app.sheet.view} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'screen.icon'}
    <ScreenSheet what="icon" view={app.sheet.view} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'screen.remove'}
    <ScreenSheet what="remove" view={app.sheet.view} onclose={() => app.closeSheet()} />
  {:else if app.sheet?.kind === 'plan.folder'}
    <PlanFolder onclose={() => app.closeSheet()} />
  {/if}

  <ColumnSheets />

  {#if toastShown !== null}
    <div class="cd-toasts">
      <div class="cd-toast" class:is-leaving={toastLeaving} role="alert">
        <span>{toastShown}</span>
        {#if app.toastAction && !toastLeaving}
          <button
            class="cd-pill cd-pill--sm"
            type="button"
            onclick={() => {
              const action = app.toastAction;
              app.toast = null;
              app.toastAction = null;
              action?.run();
            }}
          >
            {app.toastAction.label}
          </button>
        {/if}
        <button
          class="cd-toast__x"
          type="button"
          onclick={() => {
            app.toast = null;
            app.toastAction = null;
          }}
          aria-label="Dismiss"
        >✕</button>
      </div>
    </div>
  {/if}
</div>
