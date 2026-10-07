<!--
  THE THEME DISC — the rail's own control, extracted on the
  2026-10-03 pass, so the two places that
  need it draw one implementation: the rail (every screen,
  plan open) and the room's own head (no plan is open,
  so the rail — and with it the only mode switch — is not
  drawn there at all).

  The rule that makes its glyph mean something (design.md
  §8.1): it shows the mode the press will **switch to**, never
  the one already in effect. A mirror glyph is a question the
  student has to answer before they can act; a forward glyph
  is an instruction.

  In the rail the disc sits in a column, and its hover
  unfolds to the right, out past the tube. On the room it is the last
  object in a `space-between` row, so its
  right edge is pinned to the window's and the same
  expansion grows leftward, into the room — no override, the
  flexbox decides the direction.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import { app } from '../session.svelte';

  const nextMode = $derived.by(() => {
    // Read the *resolved* mode off the session, not off the
    // DOM attribute: the engine resolves `auto` against the
    // platform, and a `$derived` that read `document` would
    // not re-run when the answer changes — the disc would
    // keep promising the mode the student just left.
    const on =
      (app.appearance?.resolvedMode ?? document.documentElement.getAttribute('data-theme')) === 'dark';
    return on ? 'light' : 'dark';
  });

  // `setMode` is the session's own door: it stores the
  // preference where the pre-paint script reads it on the
  // next launch, then re-resolves the whole custom-property
  // set through the engine — so the register (not just the
  // two `data-theme` values) moves together and the choice
  // survives a relaunch instead of being handed the
  // operating system's answer back every time.
  function flipMode(): void {
    app.setMode(nextMode);
  }
</script>

<button
  class="cd-disc cd-disc--theme"
  type="button"
  aria-label={nextMode === 'dark' ? 'Switch to dark' : 'Switch to light'}
  title={nextMode === 'dark' ? 'Switch to dark' : 'Switch to light'}
  data-command="appearance.mode"
  data-placement="sidebar"
  data-rail="theme"
  onclick={flipMode}
>
  <span class="cd-disc__ico"><Icon name={nextMode === 'dark' ? 'moon' : 'sun'} /></span>
  <span class="cd-disc__label">{nextMode === 'dark' ? 'Dark' : 'Light'}</span>
</button>
