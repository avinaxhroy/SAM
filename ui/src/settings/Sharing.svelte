<!--
  THIS PLAN — the fifth place's rows: what travels with an export, and what
  stays behind (`profile.export`, `profile.import`, `type.setPrivate`).

  The two doors come first: one `*.samprofile` document out, and a staged,
  validated plan in. Then one row per kind — the switch is the declared control
  for a `bool`, its accessible name is the app's own words for the setting, and
  the row's state word beside the name says which way it is set, so the state
  never rides on the switch's side alone.
-->
<script lang="ts">
  import Button from '../records/Button.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import { nameOf } from '../types';
  import { app } from '../session.svelte';

  let {
    onChange = null,
    personal = $bindable(false),
  }: {
    /** The settings screen's changed-this-session record: one call per write. */
    onChange?: ((label: string, from: string, to: string, run: () => Promise<boolean>) => void) | null;
    /** Include personal records in the next export. Owned here, read by the
     *  settings screen so the place's own summary can state the same fact. */
    personal?: boolean;
  } = $props();

  let busy = $state(false);

  const types = $derived(Object.entries(app.types).sort(([a], [b]) => a.localeCompare(b)));
  const personalCount = $derived(types.filter(([, def]) => def.private === true).length);

  const travelText = $derived(
    personal ? 'personal records travel with it' : 'personal kinds stay behind',
  );

  async function exportProfile(): Promise<void> {
    // The door disables nothing while it works (a control that takes itself
    // away mid-press drops the keyboard's place); the guard is what keeps a
    // double press from raising two file dialogs.
    if (busy) return;
    busy = true;
    try {
      await app.exportProfile(personal);
    } finally {
      busy = false;
    }
  }

  async function importProfile(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await app.importProfileViaDialog();
    } finally {
      busy = false;
    }
  }

  async function setPrivate(type: string, value: boolean, restoring = false): Promise<boolean> {
    if (busy) return false;
    const was = app.types[type]?.private === true;
    if (was === value) return false;
    busy = true;
    try {
      const result = await app.run('type.setPrivate', { type, private: value });
      if (!result) return false;
      if (!restoring) {
        onChange?.(
          nameOf(type),
          was ? 'personal — stays behind' : 'shared',
          value ? 'personal — stays behind' : 'shared',
          () => setPrivate(type, was, true),
        );
      }
      app.notice(`${nameOf(type)} ${value ? 'stays behind now' : 'travels with the export now'}`);
      return true;
    } finally {
      busy = false;
    }
  }
</script>

<div class="st-rows">
  <Row icon="trayFull" name="Export this plan" key="profile.export" plate="profile.export">
    {#snippet help()}
      One <b>*.samprofile</b> — kinds, views, records. <b>{travelText}</b>.
    {/snippet}
    {#snippet ctl()}
      <!-- The declared control for a `bool` is a switch. Its label is the
           sentence the export takes as its flag, and the state is spelled out
           in the help line above, so nothing rides on the knob's side. -->
      <span class="st-choice">
        <Switch
          checked={personal}
          label="Include personal records in this export"
          word={false}
          onchange={() => {
            personal = !personal;
          }}
        />
        <span class="st-choice__say">Include personal records</span>
      </span>
      <Button
        id="profile.export"
        label="Export this plan"
        kind="quiet"
        hint={false}
        placement="settings"
        busy={busy}
        onClick={exportProfile}
      />
    {/snippet}
  </Row>

  <Row icon="folder" name="Bring a plan in" key="profile.import" plate="profile.import">
    {#snippet help()}
      Staged and validated first; the diff is shown.
    {/snippet}
    {#snippet ctl()}
      <Button
        id="profile.import"
        label="Import a plan…"
        kind="quiet"
        hint={false}
        placement="settings"
        busy={busy}
        onClick={importProfile}
      />
    {/snippet}
  </Row>
</div>

<h3 class="st-band">
  What stays behind
  <span class="st-band__count">
    {personalCount === 0 ? 'nothing yet' : `${personalCount} of ${types.length}`}
  </span>
</h3>
<p class="st-band__lead">Personal kinds stay out of an export.</p>

<div class="st-rows">
  {#each types as [name, def] (name)}
    <Row
      icon={def.icon ?? 'dot'}
      name={nameOf(name)}
      key={name}
      keyTitle="Copy the kind"
      plate={`type:${name}`}
    >
      {#snippet trail()}
        <span class="st-state">{def.private ? 'personal' : 'shared'}</span>
      {/snippet}
      {#snippet ctl()}
        <Switch
          checked={def.private === true}
          label={`Keep ${nameOf(name)} personal`}
          word={false}
          busy={busy}
          command="type.setPrivate"
          placement="settings"
          onchange={() => void setPrivate(name, def.private !== true)}
        />
      {/snippet}
    </Row>
  {/each}
</div>
