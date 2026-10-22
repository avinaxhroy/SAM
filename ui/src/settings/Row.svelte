<!--
  Settings row: icon tile, setting label, schema key, optional help text, and
  trailing control. Wide controls (theme tiles, slider presets) place the control
  below the label. Collapses to single column on narrow widths via container query
  (panel-settings.css).
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../shell/Icon.svelte';
  import IdPair from '../shell/IdPair.svelte';

  let {
    icon = 'dot',
    name,
    key = null,
    keyTitle = null,
    keyLabel = null,
    wide = false,
    plate = null,
    settingsKey = null,
    check = null,
    help = null,
    trail = null,
    ctl = null,
  }: {
    /** A glyph from the icon family; it names the setting's subject at a glance. */
    icon?: string;
    /** The app's own name for the setting. */
    name: string;
    /** The schema key (or command id) the setting is written by. */
    key?: string | null;
    keyTitle?: string | null;
    keyLabel?: string | null;
    /** The control needs the full width of the row. */
    wide?: boolean;
    /** The find field's own target — `data-plate` on the row. */
    plate?: string | null;
    /** The engine's declared key for a row this screen writes directly. */
    settingsKey?: string | null;
    /** The loader's own finding against this row, when it has one. */
    check?: 'error' | null;
    /** One sentence of consequence, where a sentence teaches something. */
    help?: Snippet | null;
    /** A mark that belongs to the name itself — a state word, a keycap. */
    trail?: Snippet | null;
    /** The row's control. */
    ctl?: Snippet | null;
  } = $props();
</script>

<div
  class="st-row"
  class:st-row--wide={wide}
  data-plate={plate ?? undefined}
  data-settings-key={settingsKey ?? undefined}
  data-check={check ?? undefined}
>
  <span class="st-row__ico" aria-hidden="true"><Icon name={icon} size={16} /></span>
  <div class="st-row__label">
    <span class="st-row__name">
      <span class="st-row__title">{name}</span>
      {#if key}<IdPair label={keyLabel} value={key} title={keyTitle ?? 'Copy the key'} />{/if}
      {#if trail}{@render trail()}{/if}
    </span>
    {#if help}<span class="st-row__help">{@render help()}</span>{/if}
  </div>
  {#if ctl}<div class="st-row__ctl">{@render ctl()}</div>{/if}
</div>
