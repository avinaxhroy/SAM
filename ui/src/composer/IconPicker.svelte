<!--
  THE ICON PICKER (COMPOSER §4.7).

  A grid of glyphs over **the set this build actually draws**
  (`shell/Icon.svelte` exports it), used by the new-screen sheet and by Change
  icon. Four decisions:

  1. **A name no build can draw is a lie**, so the list is the component's own
     and not a vocabulary written here a second time.
  2. **The grid is the glyph.** One tile states one thing every time, so the eye
     compares shapes across the grid instead of reading eighteen words. The name
     is still there for whoever needs it — as the control's accessible name and
     as its tooltip.
  3. **The key is not the label.** `squareStack` is schema; the tooltip says
     *Square stack* and the button sends the key — the same bargain every
     identity pair in this app makes (D2), and the reason the string census
     stays at zero.
  4. **`clears` is the one optional act.** A screen's glyph may be taken away
     again (the app draws its own); a *kind's* may not — `type.new` has no
     `clear-icon`, so a None tile there would offer a write the engine refuses.
     The caller says which it is, and the tile appears only where the command
     can honour it.
-->
<script lang="ts">
  import Icon, { SCREEN_ICONS } from '../shell/Icon.svelte';

  let {
    value,
    onpick,
    labelledby = undefined,
    label = 'Icon',
    clears = true,
    search = false,
  }: {
    /** The icon currently chosen, or `''` for none. */
    value: string;
    onpick: (icon: string) => void;
    /** The id of the visible label this group belongs to, when there is one. */
    labelledby?: string;
    label?: string;
    /** Whether clearing the glyph is a write this caller's command can make. */
    clears?: boolean;
    /** Whether the grid carries its own search field (the browsing sheets). */
    search?: boolean;
  } = $props();

  let query = $state('');

  const shown = $derived.by(() => {
    const text = query.trim().toLowerCase();
    if (!text) return SCREEN_ICONS;
    return SCREEN_ICONS.filter((name) => name.toLowerCase().includes(text));
  });

  /** `squareStack` read as words: “Square stack”. */
  function iconWords(glyph: string): string {
    const spaced = glyph.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/[._-]+/g, ' ');
    return spaced.charAt(0).toUpperCase() + spaced.slice(1);
  }
</script>

{#if search}
  <div class="cmp-iconpick__find">
    <Icon name="search" />
    <input
      class="cmp-iconpick__field"
      type="search"
      placeholder="Search icons…"
      aria-label="Search icons"
      autocomplete="off"
      spellcheck="false"
      bind:value={query}
    />
  </div>
{/if}

<div
  class="cmp-iconpick"
  role="radiogroup"
  aria-label={labelledby ? undefined : label}
  aria-labelledby={labelledby}
>
  {#each shown as option (option)}
    <button
      class="cmp-iconpick__one"
      type="button"
      role="radio"
      aria-checked={value === option}
      aria-label={iconWords(option)}
      title={iconWords(option)}
      onclick={() => onpick(option)}
    >
      <Icon name={option} size={18} />
    </button>
  {/each}
  {#if shown.length === 0}
    <p class="cmp-iconpick__none" role="status">No icon is called “{query.trim()}”.</p>
  {/if}
  {#if clears && query.trim() === ''}
    <!-- The app's own fallback, offered as a tile rather than as a second
         control: it is one more thing the glyph can be, and it is the default. -->
    <button
      class="cmp-iconpick__one cmp-iconpick__one--none"
      type="button"
      role="radio"
      aria-checked={value === ''}
      aria-label="No icon"
      title="No icon — let the app draw its own"
      onclick={() => onpick('')}
    >
      <Icon name="minus" size={18} />
    </button>
  {/if}
</div>
