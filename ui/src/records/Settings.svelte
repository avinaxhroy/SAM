<!--
  SETTINGS (S12, `UI_PLAN.md` §6.1, D2, D7, D8).

  A rail of five places beside one pane of rows. This file is the screen's data
  and its two textual panes; the other three places are their own components
  (`settings/Appearance.svelte`, `settings/ComponentStyles.svelte`,
  `settings/Sharing.svelte`) and `variants/settings/A.svelte` is the chrome
  (rail, find field, change strip).

    - The day      daily budget, plan timezone, the schedule's own two rows
    - How you work the plan's keys, and the loader's collision when it has one
    - Appearance   theme, machine mode, text size, token overrides, the doctor
    - Component styles  the lab's design per component, and the interface size
    - This plan    export, import, which kinds stay behind, and the plan's own files

  Every row is one setting: the app's name, its key, one sentence where a
  sentence teaches, and the single control the setting's type declares. Controls
  dispatch `settings.set` per field (or the command that owns them), so every
  write is one transaction and the change strip can take it back. `Done` exits
  and returns focus to the active destination (D7).
-->
<script lang="ts">
  import Icon from '../shell/Icon.svelte';
  import IdPair from '../shell/IdPair.svelte';
  import Appearance from '../settings/Appearance.svelte';
  import ComponentStyles from '../settings/ComponentStyles.svelte';
  import Sharing from '../settings/Sharing.svelte';
  import Variant from '../variants/Variant.svelte';
  import { CHOOSABLE } from '../variants/catalog';
  import { styles } from '../variants/styles.svelte';
  import type { ChangeLine, FindRow, SettingsPlace } from '../variants/settings/props';
  import { glyph } from '../commands/registry';
  import { durationText, nameOf } from '../types';
  import { dispatch } from '../ipc';
  import { app } from '../session.svelte';
  import '../styles/panel-settings.css';

  let { os = 'mac' }: { os?: string } = $props();

  /** The engine's own keymap warnings, never recomputed here (§3.7). */
  const collisions = $derived(app.warnings.filter((warning) => warning.code === 'shell.keybinding-collision'));

  /** True while a write for a row is in flight, and the sentence if it failed. */
  let busyKey = $state<string | null>(null);
  let refusal = $state<{ key: string; text: string } | null>(null);

  /**
   * Scheduler intervals are read from `system.view` (`app.machine`) because
   * `app.study` represents structured values as scalar summaries.
   */
  const schedulerState = $derived(
    (app.machine?.rules as Record<string, unknown> | undefined)?.scheduler as
      | Record<string, unknown>
      | undefined,
  );
  const schedulerName = $derived(String(schedulerState?.name ?? app.study.scheduler ?? ''));
  const gaps = $derived.by<number[]>(() => {
    const list = (schedulerState?.fixed as Record<string, unknown> | undefined)?.intervals;
    if (!Array.isArray(list)) return [];
    return list.map((value) => (Number.isFinite(Number(value)) ? Number(value) : 0));
  });

  $effect(() => {
    // One read, through the session, so System opens with it already in hand.
    if (app.plan && !app.machine) void app.loadMachine();
  });

  /**
   * The plan's own documents, exactly as the engine lists them: `source` with
   * no target answers the engine's `DOCUMENTS` set plus every records file the
   * plan's config declares. Nothing here is kept by hand, so a record type the
   * plan gains appears in this section with no edit to this file. Re-read when
   * the plan or its revision moves, so the list is never stale on screen.
   */
  let documents = $state<string[]>([]);

  $effect(() => {
    const plan = app.plan;
    const revision = app.revision;
    if (!plan) {
      documents = [];
      return;
    }
    let live = true;
    void (async () => {
      try {
        const result = await dispatch('source', {}, { plan });
        const files = (result.data as { files?: string[] }).files ?? [];
        // A read that lands after the plan moved on is not this screen's answer.
        if (!live || plan !== app.plan || revision !== app.revision) return;
        documents = files;
      } catch (failure) {
        if (live) app.report(failure);
      }
    })();
    return () => {
      live = false;
    };
  });

  /** The file's own name within the plan — the path after its last separator. */
  function leafOf(file: string): string {
    const cut = Math.max(file.lastIndexOf('/'), file.lastIndexOf('\\'));
    return cut < 0 ? file : file.slice(cut + 1);
  }

  /** The scheduler's three choices, in words. Their control lives in the
   *  machine's Scheduling, where the stored load can be stated beside it. */
  /** The four budgets almost everyone picks, in minutes. The field's own
   *  vocabulary is hours; these are the round answers to that same question. */
  const BUDGET_PRESETS = [30, 60, 120, 240];

  const SCHEDULERS: Record<string, { label: string; say: string }> = {
    fixed: {
      label: 'Fixed gaps',
      say: 'the same three gaps every time, whatever happened last time.',
    },
    sm2: {
      label: 'Growing gaps',
      say: 'the gap grows while you get it right, and drops back when you do not.',
    },
    fsrs: {
      label: 'Weighted gaps',
      say: 'the gap is set by how well you did, weighted over everything you have answered.',
    },
  };

  /** The plan's own daily target, as a number of minutes. */
  const targetMin = $derived.by<number>(() => {
    const value = Number(app.study.dailyTargetMin);
    return Number.isFinite(value) && value > 0 ? value : 240;
  });

  /** The number under the finger, while a slider is being dragged and before
   *  anything is written. `null` when nothing is in flight: the plan's own
   *  value is then the only answer, and there is one source for it. */
  let preview = $state<number | null>(null);
  const shownMin = $derived(preview ?? targetMin);

  /** How many sessions of forty minutes the target buys — the L2 hint. */
  const sessions = $derived(Math.max(1, Math.round(shownMin / 40)));

  /** The stepper's own field: hours, in the unit a student thinks in. A
   *  fourteen-minute day would read `0.23`; the field is therefore never the
   *  only way to write this setting, and the step is a quarter hour. */
  const targetHours = $derived(Number((shownMin / 60).toFixed(2)));

  /** The zones the platform knows, for the timezone field's suggestions. The
   *  platform's own list — never a hand-kept one. */
  const ZONES = $derived.by<string[]>(() => {
    const supported = (Intl as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf;
    if (typeof supported !== 'function') return [];
    try {
      return supported('timeZone');
    } catch {
      return [];
    }
  });

  const zoneText = $derived(String(app.study.timezone ?? ''));

  /* ══ THE PLAN'S OWN WORDS FOR THE TWO MACHINE-OWNED ROWS ═══════════════ */
  const schedulerLabel = $derived(
    SCHEDULERS[schedulerName]?.label ?? (schedulerName.length > 0 ? nameOf(schedulerName) : 'not set'),
  );
  const ifGaps = $derived(
    gaps.length === 0
      ? 'The plan uses a schedule that decides its own gaps'
      : gaps.map((gap) => `${gap} ${gap === 1 ? 'day' : 'days'}`).join(', then '),
  );

  /* ══ THE CHANGED-THIS-SESSION RECORD ══════════════════════════════════
     One line per accepted write, each reversible through the command that owns
     its setting. The record is the *page's* account of the session, so a line's
     Undo is a real write — one transaction, exactly as the write it undoes —
     and the line is struck rather than removed. */
  let changes = $state<ChangeLine[]>([]);
  let changeId = 0;

  function record(label: string, from: string, to: string, run: () => Promise<boolean>): void {
    changeId += 1;
    changes = [...changes, { id: changeId, label, from, to, taken: false, run }];
  }

  async function undoChange(id: number): Promise<void> {
    const line = changes.find((entry) => entry.id === id);
    if (!line || line.taken) return;
    const landed = await line.run();
    if (!landed) return;
    changes = changes.map((entry) => (entry.id === id ? { ...entry, taken: true } : entry));
    app.notice(`${line.label} is back to ${line.from}`);
  }

  /** The one write path: one transaction, one undo entry, and the sentence a
   *  refusal gets. */
  async function write(key: string, value: string): Promise<boolean> {
    busyKey = key;
    refusal = null;
    try {
      const result = await app.run('settings.set', { key, value });
      if (!result) {
        refusal = {
          key,
          text: app.lastDiagnostic ?? 'The engine did not accept that value. Nothing has changed.',
        };
        return false;
      }
      return true;
    } finally {
      busyKey = null;
    }
  }

  /** A command's own sentence, drawn only when it speaks the page's own
   *  vocabulary. An engine help string that names a document or a schema key
   *  belongs in the identity pair, never in the page's prose — the census
   *  gate's own rule, applied here because this is the one place a sentence the
   *  plan authored reaches the screen. */
  const SCHEMA_WORDS =
    /[\w.-]+\.json|#\/|\b[a-z]+[A-Z][a-zA-Z0-9]*\b|(?<![\w/.])[a-z][a-z0-9]*(?:\.[a-z][a-z0-9]*)+\.?(?![\w/.])/;
  function helpOf(command: string): string | null {
    const help = app.commandDef(command)?.help ?? '';
    return help.length > 0 && !SCHEMA_WORDS.test(help) ? help : null;
  }

  async function setTarget(minutes: number, restoring = false): Promise<boolean> {
    // One write per setting at a time. A press that lands while this setting's
    // own write is in flight is *ignored*, not queued: the control stays
    // enabled (so the keyboard never loses its place), and the row still shows
    // the plan's value, which is the truth until the engine has spoken.
    if (busyKey === 'dailyTargetMin') return false;
    if (minutes === targetMin || minutes < 15 || minutes > 720) return false;
    const was = targetMin;
    const saved = await write('dailyTargetMin', String(Math.round(minutes)));
    if (!saved) return false;
    if (!restoring) {
      record('A day holds', durationText(was), durationText(minutes), () => setTarget(was, true));
    }
    app.notice(`A day is now ${durationText(minutes)}`);
    return true;
  }

  /** The stepper's field speaks hours; the plan stores minutes. */
  function setHours(value: string): void {
    const hours = Number(value);
    if (!Number.isFinite(hours)) return;
    void setTarget(Math.round((hours * 60) / 15) * 15);
  }

  async function setZone(zone: string, restoring = false): Promise<boolean> {
    if (busyKey === 'timezone') return false;
    const trimmed = zone.trim();
    if (trimmed.length === 0 || trimmed === zoneText) return false;
    const was = zoneText;
    const saved = await write('timezone', trimmed);
    if (!saved) return false;
    if (!restoring) {
      record('The clock this plan counts days in', was, trimmed, () => setZone(was, true));
    }
    app.notice(`Days are now counted in ${trimmed}`);
    return true;
  }

  /**
   * The exit (D7): close the screen, then hand the keyboard back to the rail's
   * current destination. Inside the canvas the rail is live; the Preferences
   * window has no rail, so the focus step is skipped and one control exits both
   * window modes.
   */
  function done(): void {
    app.settingsOpen = false;
    requestAnimationFrame(() => {
      const current =
        document.querySelector<HTMLElement>('.cd-nav [data-view][aria-current="page"]') ??
        document.querySelector<HTMLElement>('.cd-nav [data-view]');
      current?.focus();
    });
  }

  /** One click from this window to the plan's own documents — the file door. */
  function openFile(file: string): void {
    app.openFile(file);
  }

  /** The key the loader named in its own report, when it named one. */
  function reportedKey(message: string): string | null {
    const match = /key "([^"]+)"/.exec(message);
    return match ? match[1] : null;
  }

  /** The bindings the loader's own reports named, so the command it complains
   *  about wears the mark — the loader's finding, never a recomputation. */
  const collisionBindings = $derived(
    collisions.map((collision) => reportedKey(collision.message)).filter((key): key is string => key !== null),
  );

  /* ══ THE FIVE PLACES, THEIR LIVE SUMMARIES, AND THE PAGE'S OWN INDEX ══ */
  const perComponent = Math.max(...CHOOSABLE.map((surface) => surface.variants.length));
  const kindNames = $derived(Object.keys(app.types).sort((a, b) => a.localeCompare(b)));
  const personalKinds = $derived(kindNames.filter((kind) => app.types[kind]?.private === true).length);

  /** The export door's own flag, hoisted so the plan place's summary can state
   *  the same fact the export will act on. Owned by `Sharing.svelte`. */
  let planPersonal = $state(false);

  const daySummary = $derived(
    `${durationText(targetMin)} a day · ${zoneText} · ${(SCHEDULERS[schedulerName]?.label ?? schedulerLabel).toLowerCase()}`,
  );
  const workSummary = $derived.by(() => {
    const keys = app.keybindings.map((binding) => glyph(binding.key, os));
    if (keys.length === 0) return 'no shortcuts declared';
    const shown = keys.slice(0, 3).join(' ');
    return `${keys.length} ${keys.length === 1 ? 'key' : 'keys'} · ${shown}${keys.length > 3 ? ' …' : ''}`;
  });
  const lookSummary = $derived(
    `${app.themes.find((theme) => theme.id === app.activeTheme)?.name ?? app.activeTheme ?? 'no theme yet'} · ${app.appearance?.resolvedMode ?? 'following the machine'} · ${Math.round((app.appearance?.textScale ?? 1) * 100)}%`,
  );
  const stylesSummary = $derived(`${perComponent} designs per component · ${styles.size}`);
  const planSummary = $derived(
    `${kindNames.length} kinds · ${
      personalKinds === 0 ? 'every kind travels with an export' : `${personalKinds} stay behind`
    }`,
  );

  const places = $derived<SettingsPlace[]>([
    {
      id: 'day',
      name: 'The day',
      icon: 'clock',
      summary: daySummary,
      lead: 'The plan’s own numbers: how much a day holds, the clock its dates are counted in, and how a review comes back.',
    },
    {
      id: 'work',
      name: 'How you work',
      icon: 'checklist',
      summary: workSummary,
      lead: 'Every shortcut this plan binds. Each one is also in the console and in the menus, so a key that does not suit your keyboard costs you nothing but the shortcut.',
    },
    {
      id: 'look',
      name: 'Appearance',
      icon: 'character',
      summary: lookSummary,
      lead: 'Every colour and size on this screen comes from the theme you pick.',
    },
    {
      id: 'styles',
      name: 'Component styles',
      icon: 'squareStack',
      summary: stylesSummary,
      lead: 'Three designs per component — the one you keep is the one it draws with.',
    },
    {
      id: 'plan',
      name: 'This plan',
      icon: 'flag',
      summary: planSummary,
      lead: 'What travels with an export, and what stays behind.',
    },
  ]);

  /** The index the find field filters — built from the very table the panes
   *  render, so a hit can never name a row that is not on the page. */
  const findIndex = $derived<FindRow[]>([
    { group: 'day', name: 'A day holds', key: 'dailyTargetMin' },
    { group: 'day', name: 'Deadlines are counted in', key: 'timezone' },
    { group: 'day', name: 'How a review comes back', key: 'scheduler' },
    { group: 'day', name: 'The gaps, when the schedule is fixed', key: 'fixedIntervals' },
    ...app.keybindings.map((binding) => ({
      group: 'work' as const,
      name: app.commandDef(binding.command)?.title ?? nameOf(binding.command),
      key: `key:${binding.command}`,
    })),
    ...collisions.map((collision) => ({
      group: 'work' as const,
      name: 'Two commands want the same key',
      key: `loader:${collision.path}`,
    })),
    { group: 'look', name: 'Theme', key: 'theme.set' },
    { group: 'look', name: 'Light or dark on this machine', key: 'appearance.mode' },
    { group: 'look', name: 'Text size', key: 'appearance.setTextScale' },
    { group: 'look', name: 'Token overrides', key: 'appearance.setOverride' },
    { group: 'look', name: 'The design doctor', key: 'design.check' },
    { group: 'styles', name: 'Interface size', key: 'styles:size' },
    { group: 'styles', name: 'Every component', key: 'styles:every' },
    ...CHOOSABLE.map((surface) => ({
      group: 'styles' as const,
      name: surface.title,
      key: `styles:${surface.key}`,
    })),
    { group: 'plan', name: 'Export this plan', key: 'profile.export' },
    { group: 'plan', name: 'Bring a plan in', key: 'profile.import' },
    ...kindNames.map((kind) => ({ group: 'plan' as const, name: nameOf(kind), key: `type:${kind}` })),
  ]);

  const tally = $derived([
    `${places.length} places`,
    `${findIndex.length} settings`,
    `${app.keybindings.length} ${app.keybindings.length === 1 ? 'key' : 'keys'}`,
  ]);
</script>

<!--
  THE DAY. Two settings this screen writes, and two rows the machine owns — the
  schedule and its gaps are read here and written in System ▸ Scheduling, where
  the stored load can be stated beside them. A machine-owned row is a statement
  with a door (`the cube on the rail`), never a disabled control.
-->
{#snippet dayPanel()}
  <div class="st-rows">
    <!-- The plan's own daily budget. `controls.md` §4 names this field's control
         (`dailyTargetMin` → slider + direct numeric field + presets) and §2 says
         why: a budget is *magnitude* — "about four hours" — so the slider is the
         shape of the answer: it moves in quarter hours, states its consequence
         as it moves (§3's L2), and writes once, on release. The stepper beside
         it is the exact number for anyone who has one, and the presets are the
         four answers almost everyone gives. -->
    <div
      class="st-row st-row--wide"
      data-plate="dailyTargetMin"
      data-settings-key="dailyTargetMin"
    >
      <div class="st-row__label">
        <span class="st-row__name">
          <label for="st-daily-target">A day holds</label>
          <IdPair value="dailyTargetMin" title="Copy the key" />
        </span>
        <span class="st-row__help">
          <b>{durationText(shownMin)}</b> — about {sessions}
          {sessions === 1 ? 'session' : 'sessions'} of forty minutes. The plan never books your time,
          and nothing is red when you miss a day.
        </span>
      </div>
      <div class="st-row__ctl">
        <div class="st-budget">
          <input
            class="st-budget__track"
            type="range"
            min="15"
            max="720"
            step="15"
            value={shownMin}
            aria-label="A day holds"
            aria-valuetext={durationText(shownMin)}
            aria-busy={busyKey === 'dailyTargetMin'}
            data-command="settings.set"
            data-placement="settings"
            oninput={(event) => {
              preview = Number((event.currentTarget as HTMLInputElement).value);
            }}
            onchange={(event) => {
              const minutes = Number((event.currentTarget as HTMLInputElement).value);
              preview = null;
              void setTarget(minutes);
            }}
          />
          <span class="st-steps">
            <button
              class="st-steps__btn"
              type="button"
              aria-label="A quarter of an hour less a day"
              aria-busy={busyKey === 'dailyTargetMin'}
              disabled={targetMin <= 15}
              onclick={() => void setTarget(targetMin - 15)}
            >
              <Icon name="minus" size={12} />
            </button>
            <span class="st-steps__box">
              <input
                id="st-daily-target"
                class="st-steps__field"
                type="number"
                inputmode="decimal"
                min="0.25"
                max="12"
                step="0.25"
                value={String(targetHours)}
                aria-label="Hours of study a day"
                aria-busy={busyKey === 'dailyTargetMin'}
                data-command="settings.set"
                data-placement="settings"
                onchange={(event) => setHours((event.currentTarget as HTMLInputElement).value)}
              />
              <span class="st-steps__unit" aria-hidden="true">h</span>
            </span>
            <button
              class="st-steps__btn"
              type="button"
              aria-label="A quarter of an hour more a day"
              aria-busy={busyKey === 'dailyTargetMin'}
              disabled={targetMin >= 720}
              onclick={() => void setTarget(targetMin + 15)}
            >
              <Icon name="plus" size={12} />
            </button>
          </span>
          <span class="st-seg" role="group" aria-label="Budgets a day">
            {#each BUDGET_PRESETS as preset (preset)}
              <button
                class="st-seg__pill"
                type="button"
                aria-pressed={targetMin === preset}
                aria-busy={busyKey === 'dailyTargetMin'}
                data-command="settings.set"
                data-placement="settings"
                onclick={() => void setTarget(preset)}
              >
                {durationText(preset)}
              </button>
            {/each}
          </span>
        </div>
      </div>
    </div>

    <!-- A text setting whose vocabulary is the platform's: an inline field with
         the platform's own zone list as suggestions, never a dropdown of 400
         engine terms and never a hand-kept copy. -->
    <div class="st-row" data-plate="timezone" data-settings-key="timezone">
      <div class="st-row__label">
        <span class="st-row__name">
          <label for="st-timezone">The clock this plan counts days in</label>
          <IdPair value="timezone" title="Copy the key" />
        </span>
        <span class="st-row__help">
          Deadlines, <b>today</b> and every review date are counted in this zone, so a deadline at
          midnight means midnight here. Your computer’s zone is not assumed.
        </span>
      </div>
      <div class="st-row__ctl">
        <input
          id="st-timezone"
          class="cd-wellfield"
          type="text"
          list="st-zones"
          value={zoneText}
          spellcheck="false"
          placeholder="the zone the plan counts days in"
          aria-label="The clock this plan counts days in"
          aria-busy={busyKey === 'timezone'}
          data-command="settings.set"
          data-placement="settings"
          onchange={(event) => void setZone((event.currentTarget as HTMLInputElement).value)}
        />
        <datalist id="st-zones">
          {#each ZONES as zone (zone)}<option value={zone}></option>{/each}
        </datalist>
      </div>
    </div>
  </div>

  <div class="st-rows">
    <div class="st-row" data-plate="scheduler" data-settings-key="scheduler">
      <div class="st-row__label">
        <span class="st-row__name">
          <span>How a review comes back</span>
          <IdPair value="scheduler" title="Copy the key" />
        </span>
        <span class="st-row__help">
          {SCHEDULERS[schedulerName]?.say ?? 'the plan names a schedule this build does not know.'}
          Reviews already scheduled keep their dates — the schedule decides each review’s <b>next</b>
          one.
        </span>
      </div>
      <div class="st-row__ctl">
        <span class="st-owned">
          <span class="st-owned__value">{schedulerLabel}</span>
          <span class="st-owned__where">
            <Icon name="cube" size={12} />
            <span>The cube on the rail, then <b>Scheduling</b></span>
          </span>
        </span>
      </div>
    </div>

    <!-- The gaps, which are only used by fixed gaps. The numbers come from the
         machine's own read, because the settings read reports this row as the
         string "(structure)". -->
    <div class="st-row" data-plate="fixedIntervals" data-settings-key="fixedIntervals">
      <div class="st-row__label">
        <span class="st-row__name">
          <span>The gaps, when the schedule is fixed</span>
          <IdPair value="fixedIntervals" title="Copy the key" />
        </span>
        <span class="st-row__help">
          {#if gaps.length === 0}
            These three numbers are only read by fixed gaps, and this plan is not using them.
          {:else}
            A review you get right comes back after the first gap, then the second, then the third —
            and then stops for good. Longer gaps mean fewer reviews a week and more forgetting.
          {/if}
        </span>
      </div>
      <div class="st-row__ctl">
        <span class="st-owned">
          <span class="st-owned__value">{ifGaps}</span>
          <span class="st-owned__where">
            <Icon name="cube" size={12} />
            <span>Each gap has its own stepper in <b>Scheduling</b></span>
          </span>
        </span>
      </div>
    </div>
  </div>

  {#if refusal?.key === 'dailyTargetMin' || refusal?.key === 'timezone'}
    <p class="cd-error st-store" role="alert">{refusal.text}</p>
  {/if}

  <div class="st-store">
    <span class="st-store__say">Saved into</span>
    <IdPair value="content/rules.json" title="Copy the path" />
    <button class="st-store__open" type="button" onclick={() => openFile('content/rules.json')}>
      Open the file
    </button>
  </div>
{/snippet}

<!--
  HOW YOU WORK. Every key the plan binds, as the platform draws it, in the
  plan's own order. Read-only here by design: `shell.setKeybinding` is U6's
  command and until it ships the keyboard is configured in the file, which is
  what the store line at the foot points at. A collision is the loader's own
  finding, in the loader's own sentence, marked on the command it names.
-->
{#snippet workPanel()}
  <div class="st-rows">
    {#each app.keybindings as binding (binding.key + binding.command)}
      <div
        class="st-row"
        data-plate={`key:${binding.command}`}
        data-check={collisionBindings.includes(binding.key) ? 'error' : undefined}
      >
        <div class="st-row__label">
          <span class="st-row__name">
            <span>{app.commandDef(binding.command)?.title ?? nameOf(binding.command)}</span>
            <IdPair value={binding.command} title="Copy the command" />
          </span>
          <span class="st-row__help">
            {helpOf(binding.command) ??
              `The key this plan binds to ${app.commandDef(binding.command)?.title ?? nameOf(binding.command)} — every command is also in the console and in the menus.`}
          </span>
        </div>
        <div class="st-row__ctl">
          <kbd class="st-keycap">{glyph(binding.key, os)}</kbd>
        </div>
      </div>
    {/each}
  </div>

  {#each collisions as collision, at (`${collision.code}:${collision.path}:${at}`)}
    <div class="st-rows">
      <div class="st-row st-row--wide" data-plate={`loader:${collision.path}`} data-loader>
        <div class="st-row__label">
          <span class="st-row__name">
            <span class="st-mark" data-sev="error">error · the loader</span>
            <span>Two commands want the same key</span>
            {#if reportedKey(collision.message)}
              <kbd class="st-keycap">{glyph(reportedKey(collision.message) ?? '', os)}</kbd>
            {/if}
          </span>
          <span class="st-row__help cd-error">
            The loader found two commands on one key. The plan keeps the second binding and says so;
            nothing is dropped.
          </span>
        </div>
        <div class="st-row__ctl">
          <IdPair label="The loader says" value={collision.message} copy={collision.path} />
        </div>
      </div>
    </div>
  {/each}

  {#if app.keybindings.length === 0}
    <p class="st-hits__none">This plan binds no keys — every command is still in the console and in the menus.</p>
  {/if}

  <div class="st-store">
    <span class="st-store__say">Saved into</span>
    <IdPair value="content/shell.json" title="Copy the path" />
    <button class="st-store__open" type="button" onclick={() => openFile('content/shell.json')}>
      Open the file
    </button>
  </div>
{/snippet}

{#snippet lookPanel()}
  <Appearance onChange={record} />
{/snippet}

{#snippet stylesPanel()}
  <ComponentStyles />
{/snippet}

{#snippet planPanel()}
  <Sharing onChange={record} bind:personal={planPersonal} />

  <!--
    THE PLAN'S OWN FILES. Every JSON/JSONL document the open plan carries, as
    the engine itself lists them (`source` with no target). The list is the
    engine's, never a table kept here, so a records file a new kind declares
    appears with no edit to this screen. Each row's door opens the file in the
    app's own source editor; the store line names the folder they are read from.
  -->
  <h3 class="st-band">
    The plan's own files
    <span class="st-band__count">
      {documents.length === 1 ? '1 document' : `${documents.length} documents`}
    </span>
  </h3>
  <p class="st-band__lead">
    Every file this plan keeps — its config, its state and one file per kind of record. Press a
    file to open it in the editor; nothing here changes the plan.
  </p>

  {#if documents.length === 0}
    <p class="st-hits__none">This plan carries no documents yet.</p>
  {:else}
    <div class="st-rows">
      {#each documents as file (file)}
        <div class="st-row" data-plate={`file:${file}`}>
          <div class="st-row__label">
            <span class="st-row__name">
              <span class="st-file__name">{leafOf(file)}</span>
              <IdPair value={file} title="Copy the path" />
            </span>
          </div>
          <div class="st-row__ctl">
            <button
              class="st-store__open"
              type="button"
              aria-label={`Open ${file} in the editor`}
              onclick={() => openFile(file)}
            >
              Open in the editor
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="st-store">
    <span class="st-store__say">Reading the plan in</span>
    <IdPair value={app.plan ?? ''} title="Copy the folder" />
  </div>
{/snippet}

<Variant
  surface="settings"
  title="Settings"
  say="Saved into the plan itself, never into an account."
  {tally}
  {places}
  panels={{ day: dayPanel, work: workPanel, look: lookPanel, styles: stylesPanel, plan: planPanel }}
  find={findIndex}
  {changes}
  onUndoChange={(id) => void undoChange(id)}
  onDone={done}
/>
