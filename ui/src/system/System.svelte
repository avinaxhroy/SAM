<!--
  System configuration panel (S14, ui_PLAN.md §6.1, D12, D13 altitude 3).

  Provides schema, screen, rule, scheduling, and pipeline inspection and editing:
    - Five primary categories: Kinds, Screens, Rules, Scheduling, Stages.
    - Two-stage structural writes: Selection preview / dry-run impact projection
      preceding destructive commit actions.
    - Delegates presentation to variants (`ui/src/variants/system/`).
-->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import DeleteSheet from './DeleteSheet.svelte';
  import Variant from '../variants/Variant.svelte';
  import { PLACES, type PlaceId } from '../variants/system/places';
  import {
    LAYOUTS,
    PANELS,
    REDUCES,
    SEVERITIES,
    TYPES,
    TYPE_MARKS,
    UNITS,
    plural,
    words,
    type ColumnRow,
    type Door,
    type EditorBody,
    type Group,
    type Hit,
    type Ladder,
    type LoadDay,
    type PanelCard,
    type PlaceRows,
    type Projection,
    type ProjectionLine,
    type RailRow,
    type Receipt,
    type Row,
    type SchedulerCard,
    type Strip,
    type SystemActions,
  } from '../variants/system/props';
  import { app } from '../session.svelte';
  import {
    durationText,
    nameOf,
    type FieldRead,
    type MachineField,
    type MachineKind,
    type MachineRead,
    type MachineView,
  } from '../types';
  import '../styles/panel-system.css';

  /**
   * Extension of `MachineRead` including `today` and `dueCurve`
   * returned by `system.view`.
   */
  type Machine = MachineRead & {
    today?: string | null;
    dueCurve?: Array<{ date: string; due: number }>;
  };

  type Backup = { txid: string; date: string; summary: string; files: string[] };

  // ── the screen's own state ───────────────────────────────────────────────
  let place = $state<PlaceId>('kinds');
  let query = $state('');
  let recovering = $state(false);
  /** The row whose editor is open — A grows its sheet from this row, B and C
   *  draw the editor under it. */
  let openKey = $state<string | null>(null);
  /** The row opened as a page of its own (B's page door, C's breadcrumb route). */
  let pageKey = $state<string | null>(null);
  /** The chosen schedule, the chosen designed screen and the chosen ladder —
   *  each a *pending* choice until its own projection is accepted. */
  let schedulerChoice = $state<string | null>(null);
  let screenChoice = $state<{ view: string; panel: string } | null>(null);
  let gapDraft = $state<number[] | null>(null);
  let restoring = $state<string | null>(null);
  let committing = $state(false);
  let backups = $state<Backup[] | null>(null);
  let receipt = $state<Receipt | null>(null);

  /** The engine's retained backups, newest first — read on demand (F14). */
  async function loadBackups(): Promise<void> {
    const result = await app.run('tx.list', {}, { tracked: false });
    const data = result?.data as { backups?: Backup[] } | undefined;
    backups = (data?.backups ?? []).filter((backup) => backup.summary.includes('delete'));
  }

  // ── the machine's own read ───────────────────────────────────────────────
  const machine = $derived(app.machine as Machine | null);
  const kinds = $derived(machine?.kinds ?? []);
  const views = $derived(machine?.views ?? []);
  const panels = $derived(machine?.panels ?? []);
  const metrics = $derived(machine?.metrics ?? []);
  const pipelines = $derived(machine?.pipelines ?? []);
  const dueCurve = $derived(machine?.dueCurve ?? []);

  /**
   * The lint checks, read from the shape the plan actually holds:
   * `rules.json#/lint/rules` is a **map** of id → `{ severity, waived, note }`,
   * so `lint` itself is an object. The screen this replaces cast `lint` to an
   * array, which Svelte's `{#each}` accepted by iterating the object's values —
   * one row per value, with `rule.id` and `rule.note` undefined, so the list
   * rendered a single blank row whose identity pair read "undefined".
   */
  const checks = $derived.by<Array<Record<string, unknown>>>(() => {
    const lint = machine?.rules.lint as { rules?: Record<string, unknown> } | undefined;
    const declared = lint?.rules;
    if (!declared || typeof declared !== 'object') return [];
    return Object.entries(declared).map(([id, value]) => ({
      id,
      ...(value !== null && typeof value === 'object' ? (value as Record<string, unknown>) : {}),
    }));
  });

  const searching = $derived(query.trim().length > 0);

  /** The scheduler in use, named in words. */
  const schedulerName = $derived(
    String((machine?.rules.scheduler as Record<string, unknown> | undefined)?.name ?? ''),
  );

  /** The fixed gaps, when the plan is on the fixed scheduler. */
  const fixedGaps = $derived.by<number[]>(() => {
    const fixed = (machine?.rules.scheduler as Record<string, unknown> | undefined)?.fixed as
      | Record<string, unknown>
      | undefined;
    const list = fixed?.intervals;
    if (!Array.isArray(list)) return [];
    return list.map((value) => (Number.isFinite(Number(value)) ? Number(value) : 1));
  });
  const gaps = $derived(gapDraft ?? fixedGaps);

  // ── the plain words for the engine's vocabulary ──────────────────────────
  /** A figure's field, named in the receipt its write leaves (§4.10). */
  const FIGURE_WORDS: Record<'label' | 'view' | 'expr' | 'reduce' | 'unit', string> = {
    label: 'label',
    view: 'screen',
    expr: 'expression',
    reduce: 'fold',
    unit: 'unit',
  };
  const SCHEDULERS: Record<string, { label: string; say: string; brief: string }> = {
    fixed: {
      label: 'Fixed gaps',
      brief: 'the same three gaps every time',
      say: 'The same three gaps every time, whatever happened last time.',
    },
    sm2: {
      label: 'Growing gaps',
      brief: 'the gap grows with every pass',
      say: 'The gap grows while you get it right, and drops back when you do not.',
    },
    fsrs: {
      label: 'Weighted gaps',
      brief: 'weighted by your answers',
      say: 'The gap is set by how well you did, weighted over everything you have answered.',
    },
  };

  /** A view named as a place: its rail title when it has one, else its own
   *  name spoken. The key itself rides in the identity pair, never as prose. */
  function viewName(name: string): string {
    const destination = views.find((view) => view.name === name)?.destinations[0]?.title;
    return destination ?? nameOf(name);
  }

  /**
   * The views a figure may fold: every declared view that reads records
   * (`kind` set — a composed screen holds none), by name and by the words the
   * pick shows. The figure's own view leads the list even when it is a screen
   * or a view the plan has since lost, so the pick can never show a choice the
   * stored figure does not hold.
   */
  function foldViews(current: string): Array<{ value: string; label: string }> {
    const folds = views
      .filter((entry) => entry.kind !== null)
      .map((entry) => ({ value: entry.name, label: viewName(entry.name) }));
    if (folds.some((entry) => entry.value === current)) return folds;
    return [{ value: current, label: viewName(current) }, ...folds];
  }

  function pipelineOf(name: string | null) {
    return pipelines.find((entry) => entry.name === name) ?? null;
  }

  /** “in 3 days” — the relative distance `controls.md` §2 asks a date to state. */
  function distance(iso: string): string {
    const today = machine?.today;
    if (!today) return iso;
    const days = Math.round(
      (Date.parse(`${iso}T00:00:00Z`) - Date.parse(`${today}T00:00:00Z`)) / 86_400_000,
    );
    if (days <= 0) return 'today';
    if (days === 1) return 'tomorrow';
    if (days < 7) return `in ${days} days`;
    const weeks = Math.round(days / 7);
    return weeks <= 1 ? 'in a week' : `in ${weeks} weeks`;
  }

  /** “today at 18:26” — a transaction's own instant, spoken. */
  function when(iso: string): string {
    const at = new Date(iso);
    if (Number.isNaN(at.getTime())) return iso;
    const time = `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}`;
    if (machine?.today === at.toISOString().slice(0, 10)) return `today at ${time}`;
    return `${at.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })} at ${time}`;
  }

  /**
   * The gate a ladder states for one of its steps — the *requirement* only,
   * because the row it is printed in already names the stage. It reads
   * "Proved — needs Learned first", and printing the stage twice was the bug
   * this shape was written to remove.
   */
  function gate(pipeline: { stages: string[]; gates: unknown }, stage: string): string | null {
    const gates = pipeline.gates;
    if (!gates || typeof gates !== 'object') return null;
    const need = (gates as Record<string, unknown>)[stage];
    if (!need || typeof need !== 'object') return null;
    const required = (need as Record<string, unknown>).require;
    if (typeof required !== 'string' || required.length === 0) return null;
    return `needs ${nameOf(required)} first`;
  }

  /** The owning file of a row's write — the file door's target. */
  function openFile(file: string): void {
    app.paneFile = file;
    app.draft = null;
    app.sourcePane = true;
  }

  /** A row's terminal twin: the registry's own generic verb, spelled with this
   *  row's parameters. Copyable through the identity pair's button. */
  function shellQuote(value: string): string {
    return /^[A-Za-z0-9._@:/=-]+$/.test(value) ? value : `'${value.replaceAll("'", `'\\''`)}'`;
  }
  function terminal(id: string, params: Record<string, string | null | undefined>): string {
    const parts = [`sam ${id}`];
    for (const [name, value] of Object.entries(params)) {
      if (value === null || value === undefined || value === '') continue;
      parts.push(`--${name} ${shellQuote(value)}`);
    }
    return parts.join(' ');
  }

  /** The machine row's field as the sheets and the column verbs expect it. */
  function asField(field: MachineField): FieldRead {
    return {
      key: field.key,
      type: field.type,
      label: field.label,
      to: field.to,
      options: field.options,
      cardinality: null,
      required: field.required,
      expr: field.expr,
      pinned: field.pinned,
    };
  }

  /**
   * A derived figure folds one **column**. Naming it by the column's own label
   * ("Minutes") rather than by its key is the whole of D2 here: the key rides
   * in the identity pair on the same line, and the chip is the fact.
   */
  function columnLabel(view: string, key: string | null): string {
    if (!key) return 'every record';
    const kindName = views.find((entry) => entry.name === view)?.kind;
    const field = kinds.find((kind) => kind.id === kindName)?.fields.find((entry) => entry.key === key);
    return field?.label ?? nameOf(key);
  }

  /**
   * User-facing description of setting values. Structured rows (such as
   * intervals) format values directly from `system.view`.
   */
  function settingValue(key: string): string {
    if (key === 'fixedIntervals') {
      return gaps.length === 0
        ? 'a set of gaps — the numbers are in Scheduling'
        : gaps.map((gap) => `${gap} ${gap === 1 ? 'day' : 'days'}`).join(', then ');
    }
    if (key === 'scheduler') {
      return SCHEDULERS[schedulerName]?.label ?? (schedulerName.length > 0 ? nameOf(schedulerName) : 'not set');
    }
    if (key === 'dailyTargetMin') {
      const minutes = Number(app.study[key]);
      return Number.isFinite(minutes) && minutes > 0 ? durationText(minutes) : 'the shipped default';
    }
    const value = app.study[key];
    return value === undefined || value === null || value === '' ? 'the shipped default' : String(value);
  }

  /** The value a row's terminal twin would write — the same words, run through
   *  the same formatter, so the door and the screen cannot disagree. */
  function settingWrite(key: string): string {
    if (key === 'fixedIntervals') return JSON.stringify(gaps);
    if (key === 'dailyTargetMin') return String(Number(app.study[key]) || 240);
    return String(app.study[key] ?? '');
  }

  /** The view's blocks, as the tree draws them — the engine's own JSON. */
  function blocksOf(name: string): Array<Record<string, unknown>> {
    const defined = app.views?.views[name];
    return (defined?.blocks ?? []) as Array<Record<string, unknown>>;
  }

  /* ══ THE ROWS ═══════════════════════════════════════════════════════════
     One row per thing a place holds: a name, its key (read by the identity
     pair), at most two facts and one chip that opens the row's own editor. */

  function kindRow(kind: MachineKind): Row {
    const ladder = pipelineOf(kind.pipeline);
    const chips = ladder
      ? [{ label: ladder.stages.map(nameOf).join(' → ') }]
      : kind.private
        ? [{ label: 'personal' }]
        : [];
    if (kind.views.length === 0) chips.push({ label: 'no screen reads it', mod: 'risk' as const });
    return {
      key: kind.id,
      mark: kind.icon ?? 'cube',
      title: kind.title ?? nameOf(kind.id),
      facts: [plural(kind.records, 'record')],
      chips,
      action: `Columns · ${kind.fields.length}`,
      opens: true,
    };
  }

  /**
   * A view's layout in the student's words, with the composed case stated
   * rather than crashed: a screen made of components has **no** layout, so the
   * layouts map is never asked about `null` (asking it threw
   * `Cannot read properties of null (reading 'replace')` the first time a
   * composed screen appeared in this surface — the boundary card the composer
   * session saw). "your components" is what such a screen is.
   */
  function layoutWords(view: MachineView): string {
    if (view.components !== null && view.components !== undefined) return 'your components';
    if (view.layout === null || view.layout === undefined) return 'your components';
    return LAYOUTS[view.layout] ?? nameOf(view.layout);
  }

  function viewRow(view: MachineView): Row {
    const composed = view.components !== null && view.components !== undefined;
    const chips = view.panel
      ? [{ label: `the ${PANELS[view.panel] ?? nameOf(view.panel)} screen` }]
      : composed
        ? [{ label: 'your components', mod: 'outline' as const }]
        : [{ label: 'its own blocks', mod: 'outline' as const }];
    if (view.destinations.length > 0) {
      chips.push({
        label: `on the rail as ${view.destinations.map((entry) => entry.title).join(', ')}`,
        mod: 'ok' as const,
      });
    }
    return {
      key: view.name,
      mark: 'layout',
      title: viewName(view.name),
      facts: [
        layoutWords(view),
        composed ? plural(view.components ?? 0, 'component') : plural(view.blocks, 'block'),
      ],
      chips,
      action: composed || view.blocks > 0 ? 'Open' : 'Its query',
      opens: true,
    };
  }

  function metricRow(metric: MachineRead['metrics'][number]): Row {
    const chips = [{ label: columnLabel(metric.view, metric.expr) }];
    if (metric.viewMissing) chips.push({ label: 'its screen is gone', mod: 'risk' as const });
    return {
      key: metric.name,
      mark: 'chart',
      title: metric.label,
      facts: [`${REDUCES[metric.reduce ?? 'sum'] ?? nameOf(metric.reduce ?? 'sum')} over ${viewName(metric.view)}`],
      chips,
      action: 'What it folds',
      opens: true,
    };
  }

  function settingRow(row: FieldRead): Row {
    return {
      key: row.key,
      mark: 'dot',
      title: row.label ?? nameOf(row.key),
      facts: [settingValue(row.key)],
      chips: [],
      action: '',
      opens: false,
      terminal: terminal('settings.set', { key: row.key, value: settingWrite(row.key) }),
    };
  }

  function checkRow(check: Record<string, unknown>): Row {
    const chips = [
      {
        label: SEVERITIES[String(check.severity ?? 'advisory')] ?? 'advice',
        mod: String(check.severity) === 'error' ? ('risk' as const) : undefined,
      },
    ];
    if (check.waived) chips.push({ label: 'waived', mod: 'info' as const });
    return {
      key: String(check.id ?? 'a check'),
      mark: 'checklist',
      title: String(check.note ?? 'A check the app runs'),
      facts: [],
      chips,
      action: '',
      opens: false,
    };
  }

  function scheduleRow(id: string): Row {
    const entry = SCHEDULERS[id];
    const inUse = id === schedulerName;
    return {
      key: id,
      mark: 'refresh',
      title: entry?.label ?? nameOf(id),
      facts: [entry?.brief ?? 'the gap rule'],
      chips: inUse ? [{ label: 'in use' }] : [],
      action: inUse ? 'Its gaps' : 'Project this',
      opens: true,
    };
  }

  function pipelineRow(pipeline: MachineRead['pipelines'][number]): Row {
    return {
      key: pipeline.name,
      mark: 'stairs',
      title: `The ${nameOf(pipeline.name)} ladder`,
      facts: [plural(pipeline.stages.length, 'stage')],
      chips: pipeline.completeWhen
        ? [{ label: `finished at ${nameOf(pipeline.completeWhen)}`, mod: 'ok' as const }]
        : [{ label: 'no top rung', mod: 'outline' as const }],
      action: 'Its gates',
      opens: true,
      tail:
        pipeline.usedBy.length === 0
          ? 'no kind climbs it'
          : `climbed by ${pipeline.usedBy.map(nameOf).join(', ')}`,
    };
  }

  const index = $derived.by<Record<PlaceId, PlaceRows>>(() => {
    const settings: Group = {
      heading: 'What your plan is set to',
      rows: app.settingsSchema.map(settingRow),
    };
    const lint: Group = {
      heading: 'Checks it runs before it will load',
      rows: checks.map(checkRow),
    };
    return {
      kinds: { rows: kinds.map(kindRow), groups: [] },
      screens: { rows: views.map(viewRow), groups: [] },
      rules: { rows: metrics.map(metricRow), groups: [settings, lint] },
      scheduling: { rows: Object.keys(SCHEDULERS).map(scheduleRow), groups: [] },
      stages: { rows: pipelines.map(pipelineRow), groups: [] },
    };
  });

  const strip = $derived<Strip[]>([
    { value: kinds.length, label: 'kinds' },
    { value: kinds.reduce((sum, kind) => sum + kind.fields.length, 0), label: 'columns' },
    { value: views.length, label: 'screens' },
    { value: pipelines.length, label: 'stage ladders' },
    { value: metrics.length, label: 'derived figures' },
  ]);

  const rail = $derived<RailRow[]>(
    (machine?.destinations ?? []).map((entry) => ({
      view: entry.view,
      title: entry.title,
      mark: entry.icon ?? 'dot',
      opens: viewName(entry.view),
      missing: entry.missing,
      terminal: terminal('list.set', { view: entry.view, title: entry.title }),
    })),
  );

  /** Every icon name the plan already uses, plus the app's own chrome set — so
   *  a destination's icon field suggests real names. */
  const iconNames = $derived(
    [...new Set([
      ...kinds.map((kind) => kind.icon).filter((icon): icon is string => !!icon),
      ...(machine?.destinations ?? []).map((entry) => entry.icon).filter((icon): icon is string => !!icon),
      'bolt', 'book', 'calendar', 'checklist', 'clock', 'chart', 'cube', 'flag', 'globe',
      'graduationcap', 'hammer', 'pencil', 'route', 'squareStack', 'textQuote', 'trayFull', 'waveform',
    ])].sort(),
  );

  /* ══ THE OPEN ROW'S EDITOR ══════════════════════════════════════════════ */

  /** The engine's own stored load, as the meters draw it. */
  const load = $derived.by<LoadDay[]>(() => {
    const busiest = dueCurve.reduce((most, day) => Math.max(most, day.due), 0);
    return dueCurve.map((day) => ({
      when: distance(day.date),
      due: day.due,
      share: Math.max(6, Math.round((day.due / (busiest || 1)) * 100)),
    }));
  });

  /** The projection's own reading of the stored load. */
  const stored = $derived.by<{ total: number; busiest: { date: string; due: number } | null }>(() => {
    if (dueCurve.length === 0) return { total: 0, busiest: null };
    let total = 0;
    let busiest: { date: string; due: number } | null = null;
    for (const day of dueCurve) {
      total += day.due;
      if (!busiest || day.due > busiest.due) busiest = day;
    }
    return { total, busiest };
  });

  function ladderOf(name: string | null, kind: string | null): Ladder | null {
    const pipeline = pipelineOf(name);
    if (!pipeline) return null;
    const steps = pipeline.stages.map((stage) => ({ stage: nameOf(stage), gate: gate(pipeline, stage) }));
    const ladder: Ladder = {
      steps,
      completeWhen: pipeline.completeWhen ? nameOf(pipeline.completeWhen) : null,
    };
    if (kind && ladder_.kind === kind) {
      ladder.options = pipelines.map((option) => ({
        name: option.name,
        label: nameOf(option.name),
        stages: option.stages.map(nameOf).join(' → '),
        inUse: option.name === name,
      }));
      ladder.chosen = ladder_.to;
      ladder.strategy = ladder_.fresh ? 'fresh' : 'carry';
      ladder.complete = ladderComplete;
      const target = pipelineOf(ladder_.to);
      if (!ladder_.fresh && target) {
        ladder.map = pipeline.stages.map((stage) => ({
          from: nameOf(stage),
          targets: target.stages.map((to) => ({
            to,
            label: nameOf(to),
            chosen: ladder_.map[stage] === to,
          })),
        }));
      }
    }
    return ladder;
  }

  function columnRows(kind: MachineKind): ColumnRow[] {
    return kind.fields.map((field, index) => ({
      key: field.key,
      label: field.label ?? nameOf(field.key),
      mark: TYPE_MARKS[field.type] ?? 'cube',
      holds: field.to
        ? { label: `points at ${nameOf(field.to)}` }
        : { label: TYPES[field.type] ?? nameOf(field.type), mod: 'outline' as const },
      place: field.expr ? `computed from ${nameOf(field.expr)}` : '',
      pinned:
        field.pinned === 0
          ? 'no screen pins it'
          : field.pinned === 1
            ? 'pinned in 1 screen'
            : `pinned in ${field.pinned} screens`,
      first: index === 0,
      last: index === kind.fields.length - 1,
      terminal: terminal('column.rename', { spec: `${kind.id}.${field.key}`, label: field.label ?? field.key }),
    }));
  }

  function panelsFor(view: MachineView): PanelCard[] {
    const own: PanelCard = {
      value: '',
      label: 'Its own blocks',
      say: 'the blocks below are what you see',
      current: view.panel === null,
    };
    return [
      own,
      ...panels.map((panel) => ({
        value: panel,
        label: PANELS[panel] ?? nameOf(panel),
        say:
          view.blocks === 0
            ? 'a designed screen · nothing in the file yet'
            : `a designed screen · its ${plural(view.blocks, 'block')} stay in the file`,
        current: view.panel === panel,
      })),
    ];
  }

  function editorFor(where: PlaceId, key: string | null): EditorBody | null {
    if (!key) return null;
    if (where === 'kinds') {
      const kind = kinds.find((entry) => entry.id === key);
      if (!kind) return null;
      return {
        place: 'kinds',
        key: kind.id,
        title: kind.title ?? nameOf(kind.id),
        sub: `${plural(kind.records, 'record')} · ${plural(kind.fields.length, 'column')}`,
        terminal: terminal('column.new', { spec: `${kind.id}.focus`, kind: 'date', label: 'Planned for' }),
        columns: columnRows(kind),
        ladder: ladderOf(kind.pipeline, kind.trackable ? kind.id : null),
        addColumn: 'Add a column',
        privacy: {
          label: kind.private ? 'Share it too' : 'Keep it personal',
          note: kind.private
            ? `Personal — ${kind.title ?? nameOf(kind.id)} records stay behind and no progress travels.`
            : 'Shared — its records travel with the plan.',
          next: !kind.private,
        },
        door: {
          command: 'type.delete',
          key: kind.id,
          title: kind.title ?? nameOf(kind.id),
          label: 'Delete this kind',
          note: `A delete takes its ${plural(kind.records, 'record')} with it — the sheet says how many before anything is written.`,
        },
        file: 'content/types.json',
      };
    }
    if (where === 'screens') {
      const view = views.find((entry) => entry.name === key);
      if (!view) return null;
      return {
        place: 'screens',
        key: view.name,
        title: viewName(view.name),
        sub: `${layoutWords(view)} · ${plural(view.blocks, 'block')}`,
        terminal: terminal('view.setPanel', { name: view.name, panel: view.panel ?? '' }),
        blocks: { view: view.name, nodes: blocksOf(view.name) },
        panels: panelsFor(view),
        ownQuery: true,
        door: {
          command: 'view.delete',
          key: view.name,
          title: viewName(view.name),
          label: 'Delete this screen',
          note: 'Its blocks, its place on the rail and anything that folded it go with it — the sheet says how many before anything is written.',
        },
        file: 'content/views.json',
      };
    }
    if (where === 'rules') {
      const metric = metrics.find((entry) => entry.name === key);
      if (!metric) return null;
      // `MetricDef::reduce_kind()`'s own default: a figure with no expression
      // counts its records, one with an expression adds it up.
      const fold = metric.reduce ?? (metric.expr === null ? 'count' : 'sum');
      return {
        place: 'rules',
        key: metric.name,
        title: metric.label,
        sub: `a derived figure — what the app folds for you`,
        terminal: terminal('metric.set', {
          name: metric.name,
          label: metric.label,
          view: metric.view,
          expr: metric.expr,
          reduce: metric.reduce,
          unit: metric.unit,
        }),
        figure: {
          folds: `${REDUCES[fold] ?? nameOf(fold)} · ${columnLabel(metric.view, metric.expr)} · ${metric.unit ? UNITS[metric.unit] ?? nameOf(metric.unit) : 'no unit'}`,
          source: viewName(metric.view),
          fields: {
            label: metric.label,
            view: metric.view,
            expr: metric.expr ?? '',
            reduce: fold,
            unit: metric.unit ?? '',
            views: foldViews(metric.view),
          },
        },
        file: 'content/rules.json',
      };
    }
    if (where === 'scheduling') {
      const chosen = schedulerChoice ?? schedulerName;
      return {
        place: 'scheduling',
        key: chosen,
        title: 'Scheduling',
        sub: 'when a review comes back, and what is already scheduled',
        terminal: terminal('settings.set', { key: 'scheduler', value: chosen }),
        schedulers: Object.entries(SCHEDULERS).map(([id, entry]) => ({
          id,
          label: entry.label,
          brief: entry.brief,
          say: entry.say,
          inUse: id === chosen,
        })),
        gaps: fixedGaps.length > 0 ? gaps : null,
        load,
        file: 'content/rules.json',
      };
    }
    const pipeline = pipelineOf(key);
    if (!pipeline) return null;
    return {
      place: 'stages',
      key: pipeline.name,
      title: `The ${nameOf(pipeline.name)} ladder`,
      sub: pipeline.usedBy.length > 0
        ? `${plural(pipeline.stages.length, 'stage')} · climbed by ${pipeline.usedBy.map(nameOf).join(', ')}`
        : `${plural(pipeline.stages.length, 'stage')} · no kind climbs it`,
      terminal: terminal('type.setPipeline', { type: pipeline.usedBy[0] ?? '', pipeline: pipeline.name }),
      ladder: ladderOf(pipeline.name, null),
      file: 'content/rules.json',
    };
  }

  const editor = $derived(editorFor(place, openKey));
  const page = $derived(editorFor(place, pageKey));

  /* ══ THE PROJECTIONS ════════════════════════════════════════════════════ */

  /** A kind's ladder: choose, project, accept. */
  let ladder_ = $state<{
    kind: string;
    to: string;
    map: Record<string, string>;
    fresh: boolean;
    previewing: boolean;
    data: Record<string, unknown> | null;
    refusal: string | null;
  } | null>(null);

  function startLadder(kind: MachineKind): void {
    const to = pipelines.find((entry) => entry.name !== kind.pipeline)?.name ?? '';
    ladder_ = { kind: kind.id, to, map: {}, fresh: true, previewing: false, data: null, refusal: null };
  }
  function chooseStrategy(fresh: boolean): void {
    if (!ladder_) return;
    ladder_ = { ...ladder_, fresh, map: {}, data: null, refusal: null };
  }
  function chooseTarget(from: string, to: string): void {
    if (!ladder_) return;
    ladder_ = { ...ladder_, map: { ...ladder_.map, [from]: to }, data: null, refusal: null };
  }
  function chooseTargetPipeline(to: string): void {
    if (!ladder_) return;
    ladder_ = { ...ladder_, to, map: {}, data: null, refusal: null };
  }

  /** Every old stage has a target, so the map is allowed to be sent. */
  const ladderComplete = $derived.by<boolean>(() => {
    if (!ladder_) return false;
    if (ladder_.fresh) return true;
    const current = pipelineOf(kinds.find((kind) => kind.id === ladder_?.kind)?.pipeline ?? null);
    return (current?.stages ?? []).every((stage) => !!ladder_?.map[stage]);
  });

  /**
   * The dry run is the projection. It is fetched when the choice is complete
   * and never before, because a partial map is not a plan the engine will act
   * on and a number computed from one would be a fiction.
   *
   * The token is what makes a late answer harmless: a student who changes the
   * target while the engine is thinking must not have the *old* answer land on
   * the *new* choice.
   */
  let ladderToken = 0;
  $effect(() => {
    const current = ladder_;
    if (!current || !ladderComplete || current.data || current.previewing) return;
    const request = { ...current };
    ladderToken += 1;
    const mine = ladderToken;
    ladder_ = { ...request, previewing: true };
    void app
      .run(
        'type.setPipeline',
        {
          type: request.kind,
          pipeline: request.to,
          ...(request.fresh
            ? { fresh: true }
            : { map: Object.entries(request.map).map(([from, to]) => `${from}=${to}`) }),
          'dry-run': true,
        },
        { tracked: false },
      )
      .then((result) => {
        if (mine !== ladderToken) return;
        const data = (result?.data ?? null) as Record<string, unknown> | null;
        ladder_ = data
          ? { ...request, previewing: false, data, refusal: null }
          : {
              ...request,
              previewing: false,
              refusal:
                app.lastDiagnostic && !app.lastDiagnostic.trim().startsWith('{')
                  ? app.lastDiagnostic
                  : 'The engine did not say what this would change. Nothing has been written.',
            };
      });
  });

  /** The three projections, as one object the design draws. */
  const projection = $derived.by<Projection | null>(() => {
    const open = editor;
    if (!open) return null;

    if (open.place === 'kinds' && ladder_ && ladder_.kind === open.key) {
      const kind = kinds.find((entry) => entry.id === ladder_?.kind);
      const current = pipelineOf(kind?.pipeline ?? null);
      const target = pipelineOf(ladder_.to);
      const moved = ((ladder_.data?.moved ?? []) as unknown[]).length;
      const unmapped = ((ladder_.data?.unmapped ?? []) as string[]).length;
      const lines: ProjectionLine[] = [];
      if (ladder_.data) {
        lines.push(
          moved === 0
            ? {
                lead: 'No record of this kind carries a stage yet,',
                rest: ' so none loses one — and none has a stored review date to lose either. The change is safe today and will not be after the first graded review.',
              }
            : {
                lead: `${plural(moved, 'record')} of ${kind?.title ?? nameOf(open.key)}`,
                rest: ` ${moved === 1 ? 'carries' : 'carry'} a stage today. Each one loses that stage and the review date stored against it, so every one of them comes back into the queue as if it had never been seen.`,
              },
        );
        if (unmapped > 0) {
          lines.push({
            lead: `${plural(unmapped, 'stage')} would have no target`,
            rest: ' and is dropped.',
          });
        }
        lines.push({
          lead: `The kind’s ${plural(current?.stages.length ?? 0, 'stage')} become`,
          rest: ` ${(target?.stages ?? []).map(nameOf).join(' → ')}${target?.completeWhen ? `, and it is finished at ${nameOf(target.completeWhen)}` : ''}.`,
        });
        lines.push({
          lead: 'Nothing else in the plan moves:',
          rest: ' other kinds keep their own ladder, and ⌘Z brings this back.',
        });
      }
      return {
        command: 'type.setPipeline',
        accept:
          moved === 0
            ? 'Re-base this kind'
            : `Re-base it — ${plural(moved, 'record')} lose${moved === 1 ? 's' : ''} its review date`,
        disabled: committing || !ladderComplete || !ladder_.data,
        danger: false,
        busy: committing,
        lines,
        terminal: `${terminal('type.setPipeline', { type: open.key, pipeline: ladder_.to })}${ladder_.fresh ? ' --fresh true' : Object.entries(ladder_.map).map(([from, to]) => ` --map ${from}=${to}`).join('')}`,
        pending: ladder_.previewing,
        wait: !ladderComplete ? 'Choose a target for every stage before the engine can say what this would change.' : null,
        error: ladder_.refusal,
        confirm: null,
      };
    }

    if (open.place === 'screens' && screenChoice && screenChoice.view === open.key) {
      const view = views.find((entry) => entry.name === open.key);
      if (!view) return null;
      const to = screenChoice.panel;
      const said = to === ''
        ? {
            lead: `${viewName(view.name)} would go back to drawing its own blocks.`,
            rest: ` Nothing is removed — its ${plural(view.blocks, 'block')} are already there.`,
          }
        : view.panel === to
          ? { lead: `It already draws the ${PANELS[to] ?? nameOf(to)} screen.`, rest: ' Nothing would change.' }
          : {
              lead: `${viewName(view.name)} would draw the ${PANELS[to] ?? nameOf(to)} screen`,
              rest: ` instead of ${view.panel ? `the ${PANELS[view.panel] ?? nameOf(view.panel)} screen` : 'its own blocks'}. Its ${plural(view.blocks, 'block')} stay in the file and stay editable below — only the one key that says which screen to draw changes.`,
            };
      return {
        command: 'view.setPanel',
        accept: to === '' ? 'Draw its own blocks' : `Draw the ${PANELS[to] ?? nameOf(to)} screen`,
        disabled: committing || (to === '' ? view.panel === null : view.panel === to),
        danger: false,
        busy: committing,
        lines: [
          said,
          { lead: 'The rail disc opens the same either way;', rest: ' only what it shows inside changes. ⌘Z brings it back.' },
        ],
        terminal: terminal('view.setPanel', { name: view.name, panel: to }),
        pending: false,
        wait: null,
        error: null,
        confirm: null,
      };
    }

    if (open.place === 'scheduling') {
      const chosen = schedulerChoice ?? schedulerName;
      const inUse = chosen === schedulerName;
      const lines: ProjectionLine[] =
        stored.total === 0
          ? [
              {
                lead: 'Nothing is scheduled ahead in this plan,',
                rest: ' so there is no stored load to state. The schedule you choose decides the first date the moment a review is graded.',
              },
            ]
          : [
              {
                lead: `${plural(stored.total, 'review')} ${stored.total === 1 ? 'is' : 'are'} already scheduled`,
                rest: ` across the next 14 days, from the plan’s own stored dates${stored.busiest ? ` — the most in one day is ${stored.busiest.due} on ${distance(stored.busiest.date)}` : ''}.`,
              },
              { lead: 'None of them moves.', rest: ' A review keeps the date it has; the schedule you choose decides each review’s next date, from the next grade on.' },
              {
                lead: 'The app cannot yet tell you what the load would be under a different schedule —',
                rest: ' it reads the dates already written rather than simulating one, so the rule that nothing stored is rewritten is the whole of this projection.',
              },
            ];
      return {
        command: 'settings.set',
        accept: `${SCHEDULERS[chosen]?.label ?? 'This schedule'} — ${inUse ? 'already in use' : 'use this'}`,
        disabled: inUse || committing,
        danger: false,
        busy: committing,
        lines,
        terminal: terminal('settings.set', { key: 'scheduler', value: chosen }),
        pending: false,
        wait: null,
        error: null,
        confirm: null,
      };
    }

    return null;
  });

  /* ══ THE WRITES ═════════════════════════════════════════════════════════ */

  /**
   * One machine write, through the ordinary door: `app.run` validates, commits
   * and re-reads (the session's `afterWrite` reloads the machine), and the
   * read-back is drawn **in the surface that wrote** with the one action that
   * reverses it — the lab's own rule, and the reason this screen does not raise
   * a floating toast beside a receipt it has already drawn.
   */
  async function commit(command: string, params: Record<string, unknown>, landed: string): Promise<boolean> {
    committing = true;
    const result = await app.run(command, params).finally(() => {
      committing = false;
    });
    if (!result) {
      receipt = { text: app.lastDiagnostic ?? `the engine refused the ${command} write`, undo: false };
      app.toast = null;
      return false;
    }
    const data = result.data as { removed?: Record<string, number> } | undefined;
    const alsoGone = Object.entries(data?.removed ?? {})
      .filter(([, count]) => count > 0)
      .map(([what, count]) => {
        const noun = { blocks: 'block', columns: 'column', viewKeys: 'sort key', metrics: 'metric' }[what] ?? what;
        return `${plural(count, noun)}`;
      })
      .join(' · ');
    receipt = {
      text: `${landed}${alsoGone.length > 0 ? ` — ${alsoGone} went with it` : ''}`,
      undo: true,
    };
    return true;
  }

  async function acceptLadder(): Promise<void> {
    const request = ladder_;
    if (!request) return;
    const target = pipelineOf(request.to);
    const landed = `${nameOf(request.kind)} now climbs ${(target?.stages ?? []).map(nameOf).join(' → ')}`;
    const done = await commit(
      'type.setPipeline',
      {
        type: request.kind,
        pipeline: request.to,
        ...(request.fresh
          ? { fresh: true }
          : { map: Object.entries(request.map).map(([from, to]) => `${from}=${to}`) }),
      },
      landed,
    );
    if (done) ladder_ = null;
  }

  async function acceptScreen(): Promise<void> {
    const choice = screenChoice;
    if (!choice) return;
    const landed =
      choice.panel === ''
        ? `${viewName(choice.view)} draws its own blocks again`
        : `${viewName(choice.view)} draws the ${PANELS[choice.panel] ?? nameOf(choice.panel)} screen`;
    const done = await commit('view.setPanel', { name: choice.view, panel: choice.panel }, landed);
    if (done) screenChoice = null;
  }

  async function acceptScheduler(): Promise<void> {
    const name = schedulerChoice;
    if (!name || name === schedulerName) return;
    const done = await commit(
      'settings.set',
      { key: 'scheduler', value: name },
      `Reviews now follow ${SCHEDULERS[name]?.label.toLowerCase() ?? name}`,
    );
    if (done) schedulerChoice = null;
  }

  /** The fixed gaps: a stepper, not a JSON string. */
  async function writeGaps(next: number[]): Promise<void> {
    gapDraft = next;
    await commit(
      'settings.set',
      { key: 'fixedIntervals', value: JSON.stringify(next) },
      `Gaps are now ${next.map((gap) => plural(gap, 'day')).join(', ')}`,
    );
    gapDraft = null;
  }

  /**
   * A restore is a transaction like any other (`tx.restore`), so it is
   * previewable, undoable and refused on a stale revision — the recovery door
   * F14 asks for, not a second write path.
   */
  async function restore(txid: string): Promise<void> {
    restoring = txid;
    try {
      const result = await app.run('tx.restore', { txid });
      if (result) {
        receipt = { text: 'Brought back — the plan is as it was before that change', undo: true };
        await loadBackups();
      }
    } finally {
      restoring = null;
    }
  }

  /* ══ ONE SEARCH, EVERY PLACE ════════════════════════════════════════════ */

  /** What a search hit *is*, in one word. The place names where it lives; this
   *  names the thing, and the two together are what lets a mixed list be read. */
  const WHAT: Record<string, string> = {
    kind: 'a kind',
    column: 'a column',
    screen: 'a screen',
    destination: 'a rail place',
    figure: 'a figure',
    ladder: 'a ladder',
    check: 'a check',
    schedule: 'a schedule',
  };

  const hits = $derived.by<Hit[]>(() => {
    if (!machine || !searching) return [];
    const needle = query.trim().toLowerCase();
    const found: Hit[] = [];
    const matches = (...parts: string[]) => parts.join(' ').toLowerCase().includes(needle);
    for (const kind of kinds) {
      if (matches(kind.id, kind.title ?? '', ...kind.fields.map((field) => `${field.key} ${field.label ?? ''}`))) {
        found.push({
          place: 'kinds',
          what: WHAT.kind,
          mark: kind.icon ?? 'cube',
          title: kind.title ?? nameOf(kind.id),
          key: kind.id,
          meta: plural(kind.records, 'record'),
        });
      }
      for (const field of kind.fields) {
        if (!matches(field.key, field.label ?? '')) continue;
        found.push({
          place: 'kinds',
          what: WHAT.column,
          mark: TYPE_MARKS[field.type] ?? 'cube',
          title: `${field.label ?? nameOf(field.key)} — a column of ${kind.title ?? nameOf(kind.id)}`,
          key: `${kind.id}.${field.key}`,
          meta: TYPES[field.type] ?? nameOf(field.type),
        });
      }
    }
    for (const view of views) {
      if (matches(view.name, viewName(view.name), layoutWords(view))) {
        const composed = view.components !== null && view.components !== undefined;
        found.push({
          place: 'screens',
          what: WHAT.screen,
          mark: 'layout',
          title: viewName(view.name),
          key: view.name,
          meta: `${layoutWords(view)} · ${
            composed ? plural(view.components ?? 0, 'component') : plural(view.blocks, 'block')
          }`,
        });
      }
    }
    for (const entry of machine.destinations) {
      if (!matches(entry.title, entry.view)) continue;
      found.push({
        place: 'screens',
        what: WHAT.destination,
        mark: entry.icon ?? 'dot',
        title: entry.title,
        key: entry.view,
        meta: 'a place on the rail',
      });
    }
    for (const metric of metrics) {
      if (!matches(metric.name, metric.label, metric.view)) continue;
      found.push({
        place: 'rules',
        what: WHAT.figure,
        mark: 'chart',
        title: metric.label,
        key: metric.name,
        meta: `${REDUCES[metric.reduce ?? ''] ?? nameOf(metric.reduce ?? 'sum')} from ${viewName(metric.view)}`,
      });
    }
    for (const check of checks) {
      if (!matches(String(check.id ?? ''), String(check.note ?? ''))) continue;
      found.push({
        place: 'rules',
        what: WHAT.check,
        mark: 'checklist',
        title: String(check.note ?? 'A check the app runs'),
        key: String(check.id ?? ''),
        meta: SEVERITIES[String(check.severity ?? 'advisory')] ?? 'advice',
      });
    }
    for (const pipeline of pipelines) {
      if (!matches(pipeline.name, ...pipeline.stages)) continue;
      found.push({
        place: 'stages',
        what: WHAT.ladder,
        mark: 'stairs',
        title: `The ${nameOf(pipeline.name)} ladder`,
        key: pipeline.name,
        meta: pipeline.stages.map(nameOf).join(' → '),
      });
    }
    for (const [id, entry] of Object.entries(SCHEDULERS)) {
      if (!matches(id, entry.label)) continue;
      found.push({
        place: 'scheduling',
        what: WHAT.schedule,
        mark: 'refresh',
        title: entry.label,
        key: id,
        meta: 'the gap rule',
      });
    }
    return found;
  });

  function clearSearch(): void {
    query = '';
    const field = document.querySelector<HTMLInputElement>('input[type="search"]');
    field?.focus();
  }

  /* ══ THE DESIGN'S CALLBACKS ═════════════════════════════════════════════ */

  function openRow(key: string): void {
    if (openKey === key) {
      openKey = null;
      screenChoice = null;
      ladder_ = null;
      return;
    }
    openKey = key;
    screenChoice = null;
    ladder_ = null;
    if (place === 'scheduling') schedulerChoice = key;
    if (place === 'kinds') {
      const kind = kinds.find((entry) => entry.id === key);
      if (kind) ladder_ = null;
    }
  }

  const acts: SystemActions = {
    onPlace(id) {
      place = id;
      openKey = null;
      pageKey = null;
      recovering = false;
      screenChoice = null;
      ladder_ = null;
      schedulerChoice = null;
    },
    onQuery(value) {
      query = value;
    },
    onClear() {
      clearSearch();
    },
    onOpen(key) {
      openRow(key);
    },
    onClose() {
      openKey = null;
      screenChoice = null;
      ladder_ = null;
    },
    onPage(key) {
      openKey = key;
      pageKey = key;
    },
    onBack() {
      pageKey = null;
    },
    onRecover() {
      recovering = !recovering;
      openKey = null;
      pageKey = null;
      if (recovering && backups === null) void loadBackups();
    },
    onRestore(txid) {
      void restore(txid);
    },
    onReloadBackups() {
      void loadBackups();
    },
    onGoHit(hit) {
      place = hit.place;
      openKey = hit.key;
      pageKey = null;
      recovering = false;
      query = '';
      screenChoice = null;
      ladder_ = null;
      schedulerChoice = hit.place === 'scheduling' ? hit.key : null;
      const kind = kinds.find((entry) => entry.id === hit.key);
      if (hit.place === 'kinds' && kind) ladder_ = null;
    },
    onFile(file) {
      openFile(file);
    },
    onRailName(view, title) {
      void commit('list.set', { view, title }, `The rail reads “${title}” now`);
    },
    onRailIcon(view, icon) {
      const entry = rail.find((row) => row.view === view);
      void commit('list.set', { view, title: entry?.title ?? view, icon }, entry?.title ? `The rail’s glyph for ${entry.title} is set` : 'The glyph is set');
    },
    onColumn(command, kind, key, step) {
      const field = kinds.find((entry) => entry.id === kind)?.fields.find((entry) => entry.key === key);
      if (!field) return;
      if (command === 'column.duplicate') {
        void commit('column.duplicate', { spec: `${kind}.${key}` }, `${field.label ?? nameOf(key)} is copied`);
        return;
      }
      if (command === 'column.reorder') {
        const list = kinds.find((entry) => entry.id === kind)?.fields ?? [];
        const at = list.findIndex((entry) => entry.key === key);
        const before = step === -1 ? (list[at - 1]?.key ?? '') : (list[at + 2]?.key ?? '');
        void commit(
          'column.reorder',
          { spec: `${kind}.${key}`, before },
          `Reordered the columns — ${field.label ?? nameOf(key)} is ${step === -1 ? 'earlier' : 'later'} in the file now`,
        );
        return;
      }
      void app.openSheet({
        kind: command as 'column.rename' | 'column.retype' | 'column.choices' | 'column.delete',
        type: kind,
        field: asField(field),
      });
    },
    onNewColumn(kind) {
      void app.openSheet({ kind: 'column.new', type: kind });
    },
    onPrivate(kind, next) {
      const entry = kinds.find((candidate) => candidate.id === kind);
      void commit(
        'type.setPrivate',
        { type: kind, private: next },
        next
          ? `${entry?.title ?? nameOf(kind)} records stay behind now`
          : `${entry?.title ?? nameOf(kind)} records travel with the plan now`,
      );
    },
    onEditView(view) {
      void app.run('view.edit', { name: view });
    },
    onDelete(door: Door) {
      if (door.command === 'type.delete') {
        void app.openSheet({ kind: 'type.delete', id: door.key, title: door.title });
      } else if (door.command === 'view.delete') {
        void app.openSheet({ kind: 'view.delete', name: door.key });
      } else {
        void app.openSheet({ kind: 'list.delete', view: door.key, title: door.title });
      }
    },
    onChooseLadder(kind, to) {
      const entry = kinds.find((candidate) => candidate.id === kind);
      if (!entry) return;
      if (!ladder_ || ladder_.kind !== kind) startLadder(entry);
      ladder_ = { ...(ladder_ as NonNullable<typeof ladder_>), to, map: {}, data: null, refusal: null };
    },
    onStrategy(fresh) {
      chooseStrategy(fresh);
    },
    onMapStage(from, to) {
      const stage = pipelineOf(kinds.find((entry) => entry.id === ladder_?.kind)?.pipeline ?? null)?.stages.find(
        (name) => nameOf(name) === from,
      );
      if (stage) chooseTarget(stage, to);
    },
    onAccept() {
      if (ladder_ && editor?.place === 'kinds') void acceptLadder();
      else if (screenChoice && editor?.place === 'screens') void acceptScreen();
      else void acceptScheduler();
    },
    onCancel() {
      if (ladder_ && editor?.place === 'kinds') ladder_ = null;
      else if (screenChoice) screenChoice = null;
      else schedulerChoice = null;
    },
    onChooseSchedule(id) {
      schedulerChoice = id;
    },
    onGap(index, step) {
      const next = [...gaps];
      next[index] = Math.max(1, Math.min(90, (next[index] ?? 1) + step));
      void writeGaps(next);
    },
    onFigure(key, value) {
      // A figure's fields are one key each, written to its own member of
      // `content/rules.json` — the row's own body, like a block's keys.
      const open = editor;
      if (!open || open.place !== 'rules') return;
      const word = FIGURE_WORDS[key] ?? nameOf(key);
      void commit('metric.set', { name: open.key, [key]: value }, `${open.title} — its ${word} is what the file says now`);
    },
    onGapsReset() {
      void writeGaps([1, 7, 30]);
    },
    onChoosePanel(view, panel) {
      screenChoice = { view, panel };
    },
    onTyped() {
      // The kind delete's typed confirmation belongs to `DeleteSheet`, which
      // owns that field; the projections here ask for no typed key.
    },
    onUndo() {
      void app.undo().then(() => {
        receipt = { text: 'Undone — the file is as it was', undo: false };
      });
    },
    onBlock(command, params) {
      return app.run(command, params).then((result) => {
        if (result) receipt = { text: 'The change is in — undo brings it back', undo: true };
        return result;
      });
    },
  };

  /* ══ KEYS ═══════════════════════════════════════════════════════════════ */

  /** The one key handler: `Esc` gives the search its value back before it
   *  closes the surface, and it stands aside entirely while a sheet owns the
   *  keyboard (`DeleteSheet` closes itself on the same key). */
  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || app.sheet) return;
    if (pageKey) {
      event.preventDefault();
      pageKey = null;
      return;
    }
    if (openKey) {
      event.preventDefault();
      acts.onClose();
      return;
    }
    if (searching) {
      event.preventDefault();
      clearSearch();
      return;
    }
    app.systemOpen = false;
  }
</script>

<svelte:window onkeydown={onkeydown} />

{#if app.machineError}
  <section class="cd-card" role="alert">
    <div class="cd-card__head">
      <span class="cd-ictile"><Icon name="flag" /></span>
      <div>
        <h2 class="cd-card__title">The machine could not be read</h2>
        <p class="cd-card__sub">{app.machineError}</p>
      </div>
    </div>
    <p class="cd-hint">
      Your data is untouched — nothing has been written. The engine's own check is the terminal, and
      the file it read is the one in the source pane.
    </p>
    <div class="sys-acts">
      <button class="cd-pill cd-pill--quiet" type="button" data-command="app.openSystem" data-placement="system" onclick={() => app.loadMachine()}>
        Try again
      </button>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={() => openFile('content/types.json')}>
        Read the file instead
      </button>
    </div>
  </section>
{:else if !machine}
  <!-- LOADING borrows the arriving geometry: a card head and a collection of
       rows at the collection layer's own row height, so nothing shifts when the
       rows land. -->
  <section class="cd-card" aria-busy="true">
    <span class="cd-sr">Reading the machine…</span>
    <div class="cd-card__head" aria-hidden="true">
      <span class="cd-ictile"><Icon name="cube" /></span>
      <span class="cd-skel" style="--w: 7ch"></span>
    </div>
    <div class="cd-skel__rows" aria-hidden="true">
      {#each { length: 5 } as _, index (index)}
        <div class="cd-skel__row cd-skel__row--coll">
          <span class="cd-skel"></span>
          <span class="cd-skel"></span>
          <span class="cd-skel"></span>
        </div>
      {/each}
    </div>
  </section>
{:else}
  <Variant
    surface="system"
    {place}
    {strip}
    {query}
    {searching}
    index={index}
    places={PLACES}
    {openKey}
    {editor}
    {page}
    {projection}
    {receipt}
    {recovering}
    {backups}
    {restoring}
    {hits}
    {rail}
    {iconNames}
    busy={committing}
    {acts}
  />
{/if}

<!-- ── the machine's three deletes: one sheet, three requests (E2) ─────── -->
{#if app.sheet?.kind === 'type.delete'}
  <DeleteSheet
    request={{
      command: 'type.delete',
      params: { name: app.sheet.id, mode: 'records' },
      title: `Delete the kind ${app.sheet.title ?? nameOf(app.sheet.id)}?`,
      subject: app.sheet.title ?? nameOf(app.sheet.id),
      key: app.sheet.id,
      note: 'Its records, the columns that pointed at it and the screens it owned go with it.',
    }}
    preview={app.sheetData}
    error={app.sheetPreview}
    busy={committing}
    onclose={() => app.closeSheet()}
    onrun={(params) => void commit('type.delete', params, 'Deleted the kind')}
  />
{:else if app.sheet?.kind === 'view.delete'}
  <DeleteSheet
    request={{
      command: 'view.delete',
      params: { name: app.sheet.name },
      title: `Delete the screen ${viewName(app.sheet.name)}?`,
      subject: viewName(app.sheet.name),
      key: app.sheet.name,
      note: 'Its blocks and its place on the rail go with it; a kind left with no screen keeps a default one.',
    }}
    preview={app.sheetData}
    error={app.sheetPreview}
    busy={committing}
    onclose={() => app.closeSheet()}
    onrun={(params) => void commit('view.delete', params, 'Deleted the screen')}
  />
{:else if app.sheet?.kind === 'list.delete'}
  <DeleteSheet
    request={{
      command: 'list.delete',
      params: { name: app.sheet.view, title: app.sheet.title },
      title: `Take ${app.sheet.title} off the rail?`,
      subject: app.sheet.title,
      key: app.sheet.title,
      note: 'The screen itself stays in the plan; only its place on the rail goes.',
    }}
    preview={app.sheetData}
    error={app.sheetPreview}
    busy={committing}
    onclose={() => app.closeSheet()}
    onrun={(params) => void commit('list.delete', params, 'Took it off the rail')}
  />
{/if}

{#if committing}
  <span class="cd-sr" role="status">Writing…</span>
{/if}
