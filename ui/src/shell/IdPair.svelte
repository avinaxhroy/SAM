<!--
  The identity pair (D2, §4.8's teaching device): a friendly label with its
  monospace key beside it, and a copy affordance on the key.

  It is the **one** place a schema key is read by default — beside the control
  that owns it — and the census's `data-identity` marker exempts exactly this
  pair, never page prose. Copying the key is how the other three doors (the
  file, the CLI, ⌘K) become learnable from the UI without ever being demanded:
  a student renames a column, sees `label` beside the new name, and can take
  the key to the JSON if they want to.

  `value` is what the pair shows and copies; `copy` overrides the copied text
  when the pair's natural key is not the address (a field's key is `est`, its
  address is `content/types.json#/types/topic/fields/est`).
-->
<script lang="ts">
  import { app } from '../session.svelte';

  let {
    label = null,
    value,
    copy = null,
    title = null,
  }: {
    label?: string | null;
    /** The schema key this pair teaches. */
    value: string;
    /** What the copy control puts on the clipboard, when it differs. */
    copy?: string | null;
    title?: string | null;
  } = $props();

  async function put(): Promise<void> {
    try {
      await navigator.clipboard.writeText(copy ?? value);
      app.notice(`copied ${value}`);
    } catch {
      // A webview without clipboard permission is a real state; say so rather
      // than pretending the copy happened.
      app.notice(`cannot copy ${value} — the clipboard is unavailable here`);
    }
  }
</script>

<span class="cd-identity" data-identity>
  {#if label}
    <span class="cd-identity__label">{label}</span>
  {/if}
  <button
    class="cd-identity__key"
    type="button"
    title={title ?? `Copy “${copy ?? value}”`}
    aria-label={`Copy ${copy ?? value}`}
    data-identity
    onclick={put}
  >
    {value}
  </button>
</span>
