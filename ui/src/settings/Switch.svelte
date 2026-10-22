<!--
  Settings switch for boolean values. Draws an explicit state word beside the
  track for accessibility. Hit target covers full 32px band (controls.md).
-->
<script lang="ts">
  let {
    checked,
    label,
    onchange,
    busy = false,
    word = true,
    on = 'On',
    off = 'Off',
    command = null,
    placement = null,
  }: {
    checked: boolean;
    /** The accessible name — the app's own words for the setting. */
    label: string;
    onchange: () => void;
    busy?: boolean;
    /** Draw the state word beside the track. Off for a row that already states it. */
    word?: boolean;
    on?: string;
    off?: string;
    command?: string | null;
    placement?: string | null;
  } = $props();
</script>

<button
  class="st-switch"
  class:st-switch--bare={!word}
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  aria-busy={busy ? 'true' : undefined}
  data-command={command ?? undefined}
  data-placement={command ? (placement ?? undefined) : undefined}
  onclick={onchange}
>
  <span class="st-switch__track" aria-hidden="true">
    <span class="st-switch__knob"></span>
  </span>
  {#if word}<span class="st-switch__word" aria-hidden="true">{checked ? on : off}</span>{/if}
</button>
