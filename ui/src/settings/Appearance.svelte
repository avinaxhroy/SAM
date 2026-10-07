<!--
  APPEARANCE — the third place's rows (§6 Phase 7, §4.8 walkthrough C, §5, §1.4.A).

  Five rows, each one setting and its own control:
    Theme            `theme.set` for preset themes (`themes/<id>.json`)
    Machine mode     light or dark on this machine, unless a theme pins its own
    Text size        `appearance.setTextScale`
    Token overrides  `appearance.setOverride` · `appearance.clearOverride`
    Design doctor    `design.check` — the same read the terminal runs (§1.4.A)

  The row grammar is the screen's own (`panel-settings.css`): the app's name for
  the setting and its key on the left, one sentence where a sentence teaches,
  and the control at the right edge. A row never repeats its value outside the
  control that owns it.
-->
<script lang="ts">
  import Button from '../records/Button.svelte';
  import IdPair from '../shell/IdPair.svelte';
  import { dispatch } from '../ipc';
  import { app } from '../session.svelte';
  import { OVERRIDE_SUGGESTIONS, storedMode, type DesignFinding, type DesignReport } from '../appearance';

  let {
    onChange = null,
  }: {
    /** The settings screen's changed-this-session record: one call per write. */
    onChange?: ((label: string, from: string, to: string, run: () => Promise<boolean>) => void) | null;
  } = $props();

  let overrideKey = $state<string>(OVERRIDE_SUGGESTIONS[0]);
  let overrideValue = $state('');
  let busy = $state(false);
  /** One write per concern at a time. The flags are the *guards*; `busy` is
   *  what the controls state (`aria-busy`) and never what takes them away: a
   *  control that disables itself mid-write drops the keyboard's focus to the
   *  body, so the next press goes nowhere. */
  let overrideBusy = false;
  let checking = false;
  let report = $state<DesignReport | null>(null);

  const appearance = $derived(app.appearance);
  const overrides = $derived(Object.entries(appearance?.overrides ?? {}));
  const mode = $derived(storedMode());
  const scale = $derived(appearance?.textScale ?? 1);
  const scaleChoices = [
    { value: 1, label: '100%' },
    { value: 1.15, label: '115%' },
    { value: 1.3, label: '130%' },
  ];

  const themeName = $derived(
    app.themes.find((theme) => theme.id === app.activeTheme)?.name ?? (app.activeTheme ?? '—'),
  );
  const shownMode = $derived(appearance?.resolvedMode ?? '—');
  const overridesText = $derived(
    overrides.length === 0 ? 'no overrides' : `${overrides.length} override${overrides.length === 1 ? '' : 's'}`,
  );
  const findings = $derived(report ? report.rules.flatMap((rule) => rule.findings) : []);
  const doctorText = $derived(
    report === null
      ? 'reading the plan…'
      : `${report.errors} error${report.errors === 1 ? '' : 's'} · ${report.advisories} advisor${
          report.advisories === 1 ? 'y' : 'ies'
        } · ${report.waived} waived`,
  );

  /** A finding's own severity as a word and as the mark's own state. */
  function severityOf(finding: DesignFinding): string {
    if (finding.waived) return 'waived';
    return finding.severity === 'error' ? 'error' : 'advisory';
  }

  let picked: string | null = $state(null);

  async function pickTheme(id: string, restoring = false): Promise<boolean> {
    // The strip's line names the themes; the writes take their **ids**. Two
    // names are not ids, and an undo that sent one would be refused by the
    // engine (the conversation that found this: `no theme "Cadence Light" in
    // …/themes — declared: cadence-dark, …`).
    const wasId = app.activeTheme;
    const was = themeName;
    busy = true;
    picked = id;
    try {
      const result = await app.run('theme.set', { id });
      if (!result) {
        picked = null;
        return false;
      }
      await app.loadAppearance();
      const now = app.themes.find((theme) => theme.id === id)?.name ?? id;
      if (!restoring) {
        onChange?.('Theme', was, now, () =>
          wasId === null ? Promise.resolve(false) : pickTheme(wasId, true),
        );
      }
      return true;
    } finally {
      busy = false;
    }
  }

  // A write re-renders the swatches; the choice must keep the keyboard focus it
  // was made with, or a keyboard-only user loses their place on every change.
  $effect(() => {
    if (picked === null) return;
    const node = document.querySelector(`[data-command="theme.set"][data-theme-id="${picked}"]`);
    if (node instanceof HTMLElement) {
      node.focus();
      picked = null;
    }
  });

  async function pickScale(value: number, restoring = false): Promise<boolean> {
    const was = scale;
    if (Math.abs(value - was) < 0.001) return false;
    busy = true;
    try {
      const result = await app.run('appearance.setTextScale', { value });
      if (!result) return false;
      await app.loadAppearance();
      if (!restoring) {
        onChange?.(
          'Text size',
          `${Math.round(was * 100)}%`,
          `${Math.round(value * 100)}%`,
          () => pickScale(was, true),
        );
      }
      return true;
    } finally {
      busy = false;
    }
  }

  /** Write one override into the plan. The strip's line states the setting in
   *  its own words — a count of overrides — never the token path, which is the
   *  identity pair's own subject. */
  async function setOverride(restoring = false): Promise<boolean> {
    if (overrideBusy) return false;
    const key = overrideKey.trim();
    const value = overrideValue.trim();
    if (key.length === 0 || value.length === 0) {
      app.notice('an override needs a token path and a value');
      return false;
    }
    // What the token held before this write: an undo puts *that* back, and only
    // a token that had nothing gets cleared.
    const before = overrides.find(([name]) => name === key)?.[1] ?? null;
    const was = overridesText;
    busy = true;
    overrideBusy = true;
    try {
      const result = await app.run('appearance.setOverride', { key, value });
      if (!result) return false;
      overrideValue = '';
      await app.loadAppearance();
      await runDoctor();
      if (!restoring) {
        onChange?.('Token overrides', was, overridesText, async () => {
          if (before === null) return clearOverride(key, true);
          overrideKey = key;
          overrideValue = before;
          return setOverride(true);
        });
      }
      return true;
    } finally {
      overrideBusy = false;
      busy = false;
    }
  }

  async function clearOverride(key: string, restoring = false): Promise<boolean> {
    if (overrideBusy) return false;
    if (key.length === 0) return false;
    const held = overrides.find(([name]) => name === key);
    if (!held) {
      app.notice(`nothing is overridden at ${key}`);
      return false;
    }
    const was = overridesText;
    busy = true;
    overrideBusy = true;
    try {
      const result = await app.run('appearance.clearOverride', { key });
      if (!result) return false;
      await app.loadAppearance();
      await runDoctor();
      if (!restoring) {
        onChange?.('Token overrides', was, overridesText, async () => {
          overrideKey = key;
          overrideValue = held[1];
          return setOverride(true);
        });
      }
      return true;
    } finally {
      overrideBusy = false;
      busy = false;
    }
  }

  /** `design.check` — the same read the terminal runs, over the same plan. */
  async function runDoctor(): Promise<void> {
    if (checking) return;
    if (!app.plan) return;
    checking = true;
    busy = true;
    try {
      const result = await dispatch('design.check', { mode }, { plan: app.plan });
      report = result.data as DesignReport;
    } catch (failure) {
      app.report(failure);
    } finally {
      checking = false;
      busy = false;
    }
  }

  // One read on mount: the doctor's own numbers, never a remembered copy.
  $effect(() => {
    if (app.plan && !report) void runDoctor();
  });
</script>

<div class="st-rows">
  <!-- The one row in this place whose control needs room: four colour ways,
       each a name and the mode it pins. `--wide` because four tiles cannot
       stand beside a label without squeezing it to a column of two words. -->
  <div class="st-row st-row--wide" data-plate="theme.set">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>Theme</span>
        <IdPair value="theme.set" title="Copy the command" />
      </span>
      <span class="st-row__help">
        A theme pins its own light or dark mode and sets every colour and size the plan draws with —
        this plan is wearing <b>{themeName}</b>.
      </span>
    </div>
    <div class="st-row__ctl">
      <div class="st-tiles" role="radiogroup" aria-label="Theme">
        {#each app.themes as theme (theme.id)}
          <button
            class="st-tile"
            class:st-tile--on={app.activeTheme === theme.id}
            type="button"
            role="radio"
            aria-checked={app.activeTheme === theme.id}
            aria-label={`${theme.name} — ${theme.mode}`}
            title={theme.description ?? theme.name}
            data-command="theme.set"
            data-placement="settings"
            data-theme-id={theme.id}
            aria-busy={busy}
            onclick={() => {
              if (!busy) void pickTheme(theme.id);
            }}
          >
            <span class="st-tile__name">{theme.name}</span>
            <span class="st-tile__mode">{theme.mode}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>

  <div class="st-row" data-plate="appearance.mode">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>Light or dark on this machine</span>
        <IdPair value="appearance.mode" title="Copy the command" />
      </span>
      <span class="st-row__help">
        Follow the machine's own light or dark mode — the plan is showing <b>{shownMode}</b>. A theme
        that pins its mode ignores this.
      </span>
    </div>
    <div class="st-row__ctl">
      <!-- A `switch`, not a two-option segment, and the reason is
           `controls.md` §2: yes/no gets a **switch**. The track is the well and
           the knob is the card, so the state reads from the knob's side without
           colour. -->
      <button
        class="cd-switch"
        type="button"
        role="switch"
        aria-checked={mode === 'dark'}
        aria-label="Follow the machine's light or dark mode"
        data-command="appearance.mode"
        data-placement="settings"
        onclick={() => {
          const was = shownMode;
          const next = mode === 'dark' ? 'light' : 'dark';
          app.setMode(next);
          onChange?.('Machine mode', `the plan is showing ${was}`, `the plan is showing ${next}`, async () => {
            app.setMode(was === 'dark' ? 'dark' : 'light');
            return true;
          });
        }}
      ></button>
    </div>
  </div>

  <div class="st-row" data-plate="appearance.setTextScale">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>Text size</span>
        <IdPair value="appearance.setTextScale" title="Copy the command" />
      </span>
      <span class="st-row__help">
        The window's own type ramp scales; the layout does not move.
      </span>
    </div>
    <div class="st-row__ctl">
      <!-- A segmented control, not buttons that disable themselves: the chosen
           option stays focusable (and re-clicking it writes nothing), so a
           keyboard user keeps their place after the write. -->
      <span class="cd-seg" role="radiogroup" aria-label="Text size">
        {#each scaleChoices as choice (choice.value)}
          <button
            class="cd-seg__pill"
            type="button"
            role="radio"
            aria-checked={Math.abs(scale - choice.value) < 0.001}
            aria-busy={busy}
            data-command="appearance.setTextScale"
            data-placement="settings"
            onclick={() => {
              if (!busy) void pickScale(choice.value);
            }}
          >
            {choice.label}
          </button>
        {/each}
      </span>
    </div>
  </div>

  <!-- The advanced row, and the only one whose control carries text of its own:
       a token path and a value, which is exactly what the write takes. -->
  <div class="st-row st-row--wide" data-plate="appearance.setOverride">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>Token overrides</span>
        <IdPair value="appearance.setOverride" title="Copy the command" />
      </span>
      <span class="st-row__help">
        Advanced: the theme's own values, replaced by hand — <b>{overridesText}</b>. An override holds
        until the token it names is reset.
      </span>
    </div>
    <div class="st-row__ctl">
      <div class="st-overrides">
        {#each overrides as [key, value] (key)}
          <div class="st-override">
            <IdPair label="Token" value={key} />
            <span class="st-override__value">{value}</span>
            <Button
              id="appearance.clearOverride"
              label="Reset"
              kind="quiet"
              hint={false}
              placement="settings"
              busy={busy}
              onClick={() => void clearOverride(key)}
            />
          </div>
        {/each}
        <div class="st-override st-override--new">
          <label class="cd-sr" for="override-key">The token to override</label>
          <input
            id="override-key"
            class="cd-wellfield st-override__key"
            type="text"
            list="override-suggestions"
            bind:value={overrideKey}
            spellcheck="false"
            placeholder="the token to override"
          />
          <datalist id="override-suggestions">
            {#each OVERRIDE_SUGGESTIONS as suggestion (suggestion)}
              <option value={suggestion}></option>
            {/each}
          </datalist>
          <label class="cd-sr" for="override-value">The value to give it</label>
          <input
            id="override-value"
            class="cd-wellfield st-override__value-field"
            type="text"
            bind:value={overrideValue}
            spellcheck="false"
            placeholder="a colour, a size, a family"
          />
          <Button
            id="appearance.setOverride"
            label="Set"
            kind="quiet"
            hint={false}
            placement="settings"
            busy={busy}
            onClick={() => void setOverride()}
          />
          <!-- Always rendered: an override the file already carries is reset from
               its own row above, and a *new* one is reset here without a write
               first. Both are `appearance.clearOverride`. -->
          <Button
            id="appearance.clearOverride"
            label="Reset this one"
            kind="quiet"
            hint={false}
            placement="settings"
            busy={busy}
            disabled={overrides.every(([key]) => key !== overrideKey.trim())}
            onClick={() => void clearOverride(overrideKey.trim())}
          />
        </div>
      </div>
    </div>
  </div>

  <div class="st-row st-row--wide" data-plate="design.check">
    <div class="st-row__label">
      <span class="st-row__name">
        <span>The design doctor</span>
        <IdPair value="design.check" title="Copy the command" />
      </span>
      <span class="st-row__help">
        The engine's own lints over the resolved colours — the same read the terminal runs.
        <b>{doctorText}</b>
      </span>
    </div>
    <div class="st-row__ctl">
      <div class="st-doctor">
        <Button
          id="design.check"
          label={report ? 'Run the check again' : 'Run the design check'}
          kind="quiet"
          hint={false}
          placement="settings"
          busy={busy}
          onClick={() => void runDoctor()}
        />
        {#if findings.length > 0}
          <div class="st-findings">
            {#each findings as finding, at (`${finding.rule}:${finding.path}:${at}`)}
              <div class="st-finding">
                <div class="st-finding__head">
                  <span class="st-mark" data-sev={severityOf(finding)}>{severityOf(finding)}</span>
                  <IdPair value={finding.path} />
                </div>
                <p class="st-finding__say">
                  {finding.message}{#if finding.waived} · waived — {finding.reason ?? 'recorded'}{/if}
                </p>
              </div>
            {/each}
          </div>
        {:else if report}
          <p class="st-finding__say">
            Nothing to report — every rule that can be read off the plan is clean;
            {report.unobservable.length} of the {report.rules.length} are not observable from plan data.
          </p>
        {/if}
      </div>
    </div>
  </div>
</div>
