<!--
  Settings segmented control (2–5 exclusive options). Selected option uses card pill
  styling instead of ink fill to prevent competing primary actions. Options remain
  focusable and clickable after selection to preserve keyboard focus across re-renders.
-->
<script lang="ts">
  type SegmentOption = {
    id: string;
    label: string;
    /** Accessible name when visible label is an abbreviation or glyph. */
    aria?: string;
    /** Tooltip note. */
    note?: string;
  };

  let {
    label,
    value,
    options,
    onpick,
    busy = false,
    command = null,
    placement = null,
    mono = false,
  }: {
    /** The accessible name of the group. */
    label: string;
    /** The chosen option's id. */
    value: string;
    options: SegmentOption[];
    onpick: (id: string) => void;
    busy?: boolean;
    /** The command each option writes, for the registry's own gate. */
    command?: string | null;
    placement?: string | null;
    /** A letter or a number wears the mono face at the row's own size. */
    mono?: boolean;
  } = $props();
</script>

<span class="st-seg" class:st-seg--mono={mono} role="radiogroup" aria-label={label}>
  {#each options as option (option.id)}
    <button
      class="st-seg__pill"
      type="button"
      role="radio"
      aria-checked={value === option.id}
      aria-label={option.aria}
      title={option.note}
      aria-busy={busy ? 'true' : undefined}
      data-command={command ?? undefined}
      data-placement={command ? (placement ?? undefined) : undefined}
      onclick={() => onpick(option.id)}
    >
      {option.label}
    </button>
  {/each}
</span>
