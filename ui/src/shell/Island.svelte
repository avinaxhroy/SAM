<!--
  The assistant island (§4.7; UI_SPEC "The assistant island").

  The run's state as the window's own chrome: one capsule at the head of the
  room — *not* inside the deck, never inside the prompt well. The reading is
  Apple's Dynamic Island, which the owner asked for: by default the surface is a
  small ink pill (black in the light theme, white in the dark one) that says
  what is happening in a word, and the press *grows the same surface* into the
  process — the live line and the turn's own rows — so the morph never changes
  colour in the middle.

  The morph is animated on the surface's own measure, not on a ceiling: the
  width the pill has, the width of the room's head; the height the pill has, the
  height the process actually takes. A `max-height` toward `--deck-max-h` was
  the older reading, and it spent most of `--dur-expand` moving through space the
  panel did not use — the panel arrived early, and a close did not move at all
  until the ceiling crossed the content, which is a press that reads as stuck.
  The panel also stays mounted until the collapse lands, so there is a shape to
  shrink instead of a word that vanishes.

  The surface is up while there is a run to report (`showsIsland()`): live, or
  just finished and not yet put away. It is the one ink object in the chrome on
  purpose, so it stands out over whatever screen is behind it.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import { assistant } from '../assistant.svelte';
  import '../styles/assistant.css';

  let log = $state<HTMLElement | null>(null);
  let island = $state<HTMLElement | null>(null);
  let surface = $state<HTMLElement | null>(null);
  let toggle = $state<HTMLElement | null>(null);
  let content = $state<HTMLElement | null>(null);

  /** The capsule's box while it morphs, in px: `null` is the pill, sized by its
      own word. Both ends of a transition are real measures, so the curve is
      the movement a reader sees. */
  let size = $state<{ w: number; h: number } | null>(null);
  /** The panel's width — the head of the room — set before the content mounts,
      so the process lays out at its final width while the surface is still the
      pill, and its height is measured once rather than re-wrapped per frame. */
  let panelWidth = $state(0);
  /** The panel outlives the open state by one transition: this is the collapse. */
  let closing = $state(false);
  let wasOpen = false;
  let settle: ReturnType<typeof setTimeout> | null = null;

  const who = $derived.by(() => {
    const agent = assistant.chosen();
    const model = assistant.modelLabel();
    if (!agent) return 'The assistant';
    return model === 'Default' ? agent.name : `${agent.name} · ${model}`;
  });

  const label = $derived(assistant.busy() ? assistant.brief() : assistant.status || 'Ready');

  /** The panel's width: the wrapper's own measure (min of the room's head and
      the window), read from the element rather than restated in CSS. */
  function headWidth(): number {
    return island ? Math.max(1, Math.round(island.clientWidth)) : 0;
  }

  /** The pill's height and the panel's: the toggle, and the toggle plus the
      process under it. */
  function panelHeight(): number {
    if (!toggle || !content) return 0;
    return Math.round(toggle.offsetHeight + content.offsetHeight);
  }

  /** The pill's width for the label as it stands now. The process is taken out
      of the flow for the read — the surface's own width is then the toggle's —
      and both are put back before anything paints, so a close lands on the word
      that is on screen and not on the width the panel was opened from. */
  function pillMeasure(): number {
    if (!surface || !content) return 0;
    const position = content.style.position;
    const width = surface.style.width;
    content.style.position = 'absolute';
    surface.style.width = 'max-content';
    const measured = Math.round(surface.getBoundingClientRect().width);
    surface.style.width = width;
    content.style.position = position;
    return measured;
  }

  /** Put the capsule back to being the pill: the content leaves the tree and
      the box returns to its own measure. */
  function land(): void {
    if (settle !== null) {
      clearTimeout(settle);
      settle = null;
    }
    closing = false;
    size = null;
  }

  // The two ends of the morph. A pre-effect reads the DOM as it still stands,
  // so the pill's measure is the pill's, and the panel is already open in the
  // same update the content mounts in.
  $effect.pre(() => {
    const nowOpen = assistant.island;
    if (nowOpen === wasOpen) return;
    const was = wasOpen;
    wasOpen = nowOpen;
    if (nowOpen) {
      if (settle !== null) {
        clearTimeout(settle);
        settle = null;
      }
      closing = false;
      panelWidth = headWidth();
      // The start is whatever is on screen: the pill at rest, or a capsule a
      // fast second press caught mid-collapse.
      const rect = surface?.getBoundingClientRect();
      size = rect && surface ? { w: Math.round(rect.width), h: surface.offsetHeight } : null;
    } else if (was) {
      // The surface shrinks to the pill it came from; the content stays until
      // the transition has landed, then the box goes back to its own measure.
      const width = pillMeasure() || (surface ? Math.round(surface.getBoundingClientRect().width) : 0);
      if (width > 0) {
        closing = true;
        size = { w: width, h: toggle?.offsetHeight ?? 0 };
        if (settle !== null) clearTimeout(settle);
        // The event is the landing; the timer is the path that never fires one
        // (reduced motion, or a height that did not actually change).
        settle = setTimeout(land, 900);
      }
    }
  });

  // The close lands when the height has finished travelling; the timer above
  // is only the fallback for a surface that never reports the event.
  function landed(event: TransitionEvent): void {
    if (event.target !== surface || event.propertyName !== 'height') return;
    if (assistant.island || !closing) return;
    land();
  }

  // The open end: the process's own measure, written once the content is in
  // the tree. A row that streams in afterwards moves it again (below).
  $effect(() => {
    if (!assistant.island || !surface || !toggle || !content) return;
    // A first open can land before the wrapper has been measured (a turn that
    // ends on its first frame): take the head's width here rather than letting
    // the process lay out at the pill's.
    const width = panelWidth || headWidth();
    if (width && width !== panelWidth) panelWidth = width;
    const height = panelHeight();
    if (!size || size.w !== width || size.h !== height) size = { w: width, h: height };
  });

  // The panel grows as the turn's rows land: follow the content rather than
  // clip it. The content is laid out at a fixed width, so this is the rows'
  // own height and never a wrap caused by the surface's animating box.
  $effect(() => {
    if (!content || !assistant.island) return;
    const observer = new ResizeObserver(() => {
      if (!toggle || !content) return;
      const height = panelHeight();
      const width = panelWidth || size?.w || 0;
      if (width > 0 && (!size || Math.abs(size.h - height) > 1)) size = { w: width, h: height };
    });
    observer.observe(content);
    return () => observer.disconnect();
  });

  // A window that changes width moves the head of the room with it.
  $effect(() => {
    if (!island) return;
    const onResize = () => {
      const width = headWidth();
      if (!width || width === panelWidth) return;
      panelWidth = width;
      if (assistant.island && size) size = { w: width, h: size.h };
    };
    globalThis.addEventListener('resize', onResize);
    return () => globalThis.removeEventListener('resize', onResize);
  });

  $effect(() => () => {
    if (settle !== null) clearTimeout(settle);
  });

  // Keep the process pinned to its newest row while the turn streams: the flush
  // counter moves once per frame of buffered text, so this is not scheduled per
  // token — and opening the island re-pins it.
  $effect(() => {
    void assistant.flushed;
    void assistant.rows.length;
    void assistant.status;
    void assistant.island;
    if (log) log.scrollTop = log.scrollHeight;
  });

  // A nested surface answers Escape before the window's own handlers: while
  // open, Escape puts the process away and stops there (a second Escape closes
  // the deck or the palette, whichever the window is on).
  function escape(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !assistant.island) return;
    event.stopPropagation();
    assistant.island = false;
  }

  $effect(() => {
    globalThis.addEventListener('keydown', escape, true);
    return () => globalThis.removeEventListener('keydown', escape, true);
  });
</script>

{#if assistant.showsIsland()}
  <div class="cd-ai-island" bind:this={island}>
    <div
      class="cd-ai-island__surface"
      class:is-open={assistant.island}
      class:is-work={assistant.running}
      style:width={size ? `${size.w}px` : null}
      style:height={size ? `${size.h}px` : null}
      role="region"
      aria-label="The run so far"
      ontransitionend={landed}
      bind:this={surface}
    >
      <button
        class="cd-ai-island__toggle"
        bind:this={toggle}
        type="button"
        aria-expanded={assistant.island}
        aria-controls="cd-ai-process"
        title={assistant.island ? 'Hide the process' : 'What the assistant is doing'}
        onclick={() => assistant.toggleIsland()}
      >
        <span class="cd-ai-island__dot" aria-hidden="true"></span>
        <span class="cd-ai-island__text">{label}</span>
        <span class="cd-ai-island__chev" aria-hidden="true"><Icon name="chevronDown" size={12} /></span>
      </button>
      <!-- The visible word does not announce itself, so the same word rides a
           live region: a collapsed island is still a run you can hear. -->
      <p class="cd-sr" role="status">{label}</p>

      {#if assistant.island || closing}
        <div
          class="cd-ai-island__content"
          id="cd-ai-process"
          style:width={panelWidth ? `${panelWidth}px` : null}
          bind:this={content}
        >
          <p class="cd-ai-island__who">{who}</p>

          {#if assistant.busy()}
            <p class="cd-ai-island__now">
              <span class="cd-ai-island__pulse" aria-hidden="true"></span>
              {assistant.activity()}
            </p>
          {/if}

          {#if assistant.rows.length > 0}
            <div class="cd-ai-island__log" bind:this={log} aria-label="What the agent is doing">
              {#if assistant.trimmed}
                <p class="cd-ai-island__row cd-ai-island__row--note">
                  Earlier steps were dropped: this turn keeps its newest.
                </p>
              {/if}
              {#each assistant.rows as row, index (index)}
                {#if row.kind === 'tool'}
                  <p class="cd-ai-island__row cd-ai-island__row--tool">
                    <span class="cd-ai-island__toolname">{row.name}</span>
                    {#if row.summary}<span class="cd-ai-island__toolwhat">{row.summary}</span>{/if}
                    {#if row.file}<span class="cd-ai-island__toolfile">{row.file}</span>{/if}
                  </p>
                {:else if row.kind === 'text'}
                  <p class="cd-ai-island__row cd-ai-island__row--text">{row.text}</p>
                {:else if row.kind === 'error'}
                  <p class="cd-ai-island__row cd-ai-island__row--error" role="alert">{row.text}</p>
                {:else}
                  <p class="cd-ai-island__row cd-ai-island__row--note">{row.text}</p>
                {/if}
              {/each}
            </div>
          {:else if !assistant.busy()}
            <p class="cd-ai-island__quiet">Nothing yet — a turn's steps land here.</p>
          {/if}

          {#if assistant.busy()}
            <!-- The deck can be closed mid-turn, and the island is what is left:
                 it must carry the one control that ends the run, or a closed
                 deck has no Stop anywhere on screen. -->
            <button
              class="cd-ai-island__put"
              type="button"
              aria-label="Stop the turn"
              onclick={() => void assistant.cancel()}
            >
              Stop the turn
            </button>
          {:else}
            <button class="cd-ai-island__put" type="button" onclick={() => assistant.dismissProcess()}>
              Put the process away
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
