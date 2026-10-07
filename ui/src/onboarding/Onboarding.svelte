<!--
The room (R11): the document SAM ships — what it is · how it works · get
started — as three screens on one pastel ground.

Two ways in, one surface. At first run (`app.phase === 'noneExists'`) the room
*is* the window and the last screen makes the plan; with a plan already open
(`app.gettingStarted` inside the shell) it is the same three screens over the
app, and it closes instead of creating anything — which is why the one prop is
`overlay`, and why the shut control only exists when there is something to shut.

Every movement is drawn as the app's own material rather than described: the
first screen's promises, the second's two specimens, the third's actual form.
The ground does not move between screens — one pastel wash with a hairline weave
drawn across it, painted on the room and not on the slide — because a ground
that changed colour as you step would make the three movements look like three
documents.

Keyboard, in one place: ← and → move, Enter continues from anywhere that is not
a control of its own (a field's caret, a button's own Enter), Escape closes the
overlay and does nothing at first run — there is nothing behind the room to go
back to. Focus moves with the screen: the first control on the way in, the new
screen's own heading after a step, so a screen reader reads the movement it has
just arrived at. Tab wraps inside the room, because a document that pages
sideways should not lose you to the browser's chrome.
-->
<script lang="ts">
  import { MOVEMENTS } from '../words';
  import { app } from '../session.svelte';
  import Icon from '../shell/Icon.svelte';
  import ThemeDisc from '../shell/ThemeDisc.svelte';
  import Welcome from './Welcome.svelte';
  import HowItWorks from './HowItWorks.svelte';
  import StartPlan from './StartPlan.svelte';

  let {
    overlay = false,
    onclose = null,
  }: { overlay?: boolean; onclose?: (() => void) | null } = $props();

  /**
   * Which movement is on screen, and which way it was entered from. A machine
   * with plans already on disk (`several`, none of them open) starts on the last
   * screen, because the movement that matters there is the one with the rows on
   * it — the two before it are a press back, not a wall to walk through again.
   */
  let step = $state(!overlay && app.phase === 'several' ? MOVEMENTS.length - 1 : 0);
  let dir = $state<1 | -1>(1);
  let root: HTMLDivElement | undefined = $state();
  let slide: HTMLDivElement | undefined = $state();

  /** Move to a screen, remembering the direction. Out-of-range is clamped. */
  function go(next: number): void {
    const target = Math.min(MOVEMENTS.length - 1, Math.max(0, next));
    if (target === step) return;
    dir = target > step ? 1 : -1;
    step = target;
  }

  /**
   * Focus follows the movement. On the way in the room takes the keyboard (the
   * Continue pill, so Enter works before anything is aimed at), and after a
   * step it lands on the new screen's heading — a heading is not a control, so
   * `tabindex="-1"` is the whole cost of the announcement. The last screen has
   * no Continue pill, and `several` opens there: the first plan on disk takes
   * the keyboard instead (Enter opens it), and a heading is the last resort,
   * because an outlined heading over a list of plans reads as a broken list.
   */
  let arrived = false;
  $effect(() => {
    step;
    if (!arrived) {
      arrived = true;
      const pill = root?.querySelector<HTMLElement>('[data-auto]');
      const row = slide?.querySelector<HTMLElement>('.ob-row');
      const heading = slide?.querySelector<HTMLElement>('.ob-h1');
      (pill ?? row ?? heading)?.focus();
      return;
    }
    slide?.querySelector<HTMLElement>('.ob-h1')?.focus();
  });

  /** Everything in the room the keyboard can reach, in document order. */
  function stops(): HTMLElement[] {
    if (!root) return [];
    const found = root.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
    );
    return [...found].filter((element) => element.offsetParent !== null);
  }

  function keys(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target as HTMLElement | null;
    const inField =
      target !== null &&
      (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
    if (event.key === 'ArrowRight' && !inField) {
      event.preventDefault();
      go(step + 1);
    } else if (event.key === 'ArrowLeft' && !inField) {
      event.preventDefault();
      go(step - 1);
    } else if (event.key === 'Enter' && !inField) {
      // A control's own Enter is the control's (a press, a row of the list, the
      // open field): the room only continues when the key had nowhere else to go.
      const control = target !== null && (target.tagName === 'BUTTON' || target.tagName === 'A');
      if (!control && step < MOVEMENTS.length - 1) go(step + 1);
    } else if (event.key === 'Escape' && overlay) {
      event.preventDefault();
      onclose?.();
    } else if (event.key === 'Tab') {
      // The wrap is hand-made: the room is not a `<dialog>`, so the platform
      // will not do it, and the whole window is the room — the next stop after
      // the last control is the browser's chrome.
      const list = stops();
      const here = list.indexOf(document.activeElement as HTMLElement);
      if (list.length === 0 || here === -1) return;
      const last = list.length - 1;
      if (event.shiftKey ? here === 0 : here === last) {
        event.preventDefault();
        list[event.shiftKey ? last : 0].focus();
      }
    }
  }
</script>

<div
  class="ob-room"
  role="dialog"
  aria-modal="true"
  aria-label="Getting started"
  bind:this={root}
  onkeydown={keys}
>
  <div class="ob-inner">
    <header class="ob-head">
      <button
        class="ob-back"
        type="button"
        data-quiet={step === 0 ? '1' : '0'}
        onclick={() => go(step - 1)}
      >
        <Icon name="chevronleft" size={14} />
        Back
      </button>

      <div class="ob-where">
        <div class="ob-dots">
          {#each MOVEMENTS as label, i (label)}
            <button
              class="ob-dot"
              type="button"
              aria-label={`Screen ${i + 1} of ${MOVEMENTS.length}: ${label}`}
              aria-current={step === i ? 'step' : undefined}
              onclick={() => go(i)}
            ></button>
          {/each}
        </div>
        <p class="ob-name">{MOVEMENTS[step]}</p>
      </div>

      {#if overlay}
        <div class="ob-out">
          <ThemeDisc />
          <button
            class="ob-shut"
            type="button"
            aria-label="Close"
            data-quiet={step === 0 ? '1' : '0'}
            onclick={() => onclose?.()}
          >
            <Icon name="close" size={16} />
          </button>
        </div>
      {:else}
        <!-- The room's own corner at first run: the theme, and nothing else.
             There is nothing behind the room to shut, so no shut is offered —
             and the disc is here because there is no rail behind it yet. -->
        <div class="ob-out">
          <ThemeDisc />
        </div>
      {/if}
    </header>

    <div class="ob-stage">
      {#key step}
        <div class="ob-slide" data-dir={dir} bind:this={slide}>
          {#if step === 0}
            <Welcome />
          {:else if step === 1}
            <HowItWorks />
          {:else}
            <StartPlan />
          {/if}
        </div>
      {/key}
    </div>

    <footer class="ob-foot">
      <p class="ob-keys">
        <kbd class="cd-kbd">←</kbd>
        <kbd class="cd-kbd">→</kbd>
        to move
        {#if step < MOVEMENTS.length - 1}
          <span aria-hidden="true">·</span>
          <kbd class="cd-kbd">Enter</kbd>
          to continue
        {/if}
        {#if overlay}
          <span aria-hidden="true">·</span>
          <kbd class="cd-kbd">Esc</kbd>
          to close
        {/if}
      </p>
      <div class="ob-acts">
        {#if step < MOVEMENTS.length - 1}
          <button class="cd-pill cd-pill--lg" type="button" data-auto onclick={() => go(step + 1)}>
            Continue
          </button>
        {/if}
      </div>
    </footer>
  </div>
</div>
