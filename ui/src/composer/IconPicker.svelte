<!--
  THE ICON PICKER (COMPOSER §4.7).

  A small grid of glyph and name over **the set this build actually draws**
  (`shell/Icon.svelte` exports it), used by the new-screen sheet and by Change
  icon. Two decisions:

  1. **A name no build can draw is a lie**, so the list is the component's own
     and not a vocabulary written here a second time.
  2. **The key is not the label.** `squareStack` is schema; the grid says
     *Square stack* and sends the key — the same bargain every identity pair in
     this app makes (D2), and the reason the string census stays at zero.
-->
<script lang="ts">
  import Icon, { ICON_NAMES } from '../shell/Icon.svelte';

  let {
    value,
    onpick,
    labelledby = undefined,
    label = 'Icon',
  }: {
    /** The icon currently chosen, or `''` for none. */
    value: string;
    onpick: (icon: string) => void;
    /** The id of the visible label this group belongs to, when there is one. */
    labelledby?: string;
    label?: string;
  } = $props();

  /** `squareStack` read as words: “Square stack”. */
  function iconWords(glyph: string): string {
    const spaced = glyph.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/[._-]+/g, ' ');
    return spaced.charAt(0).toUpperCase() + spaced.slice(1);
  }
</script>

<div
  class="cmp-iconpick"
  role="radiogroup"
  aria-label={labelledby ? undefined : label}
  aria-labelledby={labelledby}
>
  {#each ICON_NAMES as option (option)}
    <button
      class="cmp-iconpick__one"
      type="button"
      role="radio"
      aria-checked={value === option}
      aria-label={iconWords(option)}
      onclick={() => onpick(option)}
    >
      <Icon name={option} />
      <span class="cmp-iconpick__name">{iconWords(option)}</span>
    </button>
  {/each}
</div>
