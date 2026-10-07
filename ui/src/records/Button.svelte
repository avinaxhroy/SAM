<!--
  The app's one clickable control for a registry command.

  It exists so that the `--uicheck` contract is impossible to forget: a button
  that dispatches an id renders `data-command` (and, where the registry declares
  a placement, `data-placement`) **by construction**, not by the author
  remembering. Appendix C.6's gate walks exactly those attributes, so an
  unlabelled control is invisible to it — which is the failure the gate exists to
  catch.

  The shortcut hint beside the label is resolved from the keymap and the
  platform (`data-os`), never typed: a macOS-only "⌘K" in a string would be the
  app's first hardcoded platform assumption (§4.8, D14).
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { shortcutFor } from '../commands/registry';
  import { app } from '../session.svelte';

  let {
    id,
    label = null,
    placement = null,
    kind = 'ink',
    size = null,
    disabled = false,
    busy = false,
    hint = true,
    title = null,
    onClick,
    children,
  }: {
    /** The registry id this control dispatches. */
    id: string;
    label?: string | null;
    placement?: string | null;
    kind?: 'ink' | 'quiet' | 'ghost' | 'danger';
    /** `lg` is the system's own modifier for a card's leading action
        (`.cd-pill--lg`) — the room's one press, and the empty plan's. */
    size?: 'lg' | null;
    disabled?: boolean;
    /** A write of this control's own is in flight. It is **stated**, never
        `disabled`: a button that disables itself mid-write drops the keyboard's
        focus to the body, so the next press goes nowhere — and the write is
        over in microseconds, which is no reason to take the control away. */
    busy?: boolean;
    /** Show the bound key, resolved for this platform. */
    hint?: boolean;
    title?: string | null;
    onClick: () => void | Promise<void>;
    children?: Snippet;
  } = $props();

  const os = $derived(
    (globalThis as { document?: Document }).document?.documentElement.dataset.os ?? 'mac',
  );
  const key = $derived(hint ? shortcutFor(app.keybindings, id, os) : null);
</script>

<button
  class="cd-pill"
  class:cd-pill--quiet={kind === 'quiet'}
  class:cd-pill--ghost={kind === 'ghost'}
  class:cd-pill--lg={size === 'lg'}
  class:cd-danger={kind === 'danger'}
  type="button"
  data-command={id}
  data-placement={placement ?? undefined}
  {disabled}
  aria-busy={busy ? 'true' : undefined}
  title={title ?? app.commandDef(id)?.help ?? id}
  onclick={onClick}
>
  {#if children}{@render children()}{:else}{label ?? app.commandDef(id)?.title ?? id}{/if}
  {#if key}<kbd class="cd-kbd">{key}</kbd>{/if}
</button>
