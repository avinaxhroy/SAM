<!--
  PLAN SPINE · A · THE DECK.
  Stacked deck variant representing the academic term as layered weekly sheets.
  Scrolling or navigation controls advance through weeks with depth scaling and
  tab stacking transitions.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from '../../shell/Icon.svelte';
  import { readOf, type PlanProps } from './props';

  let {
    title,
    weeks,
    summary,
    selectedId,
    newTopicCommand,
    onTopic,
    onSelect,
    onNewTopic,
  }: PlanProps = $props();

  /**
   * The reference's geometry: one 240px slot of scroll per week, a 132px band
   * the parked tabs live in, a 40px tab, a 24px floor under the tallest sheet.
   * Every one of them is the register's (`calc(<n>px * var(--ui-s))` in the
   * CSS below) and the pass reads `--ui-s` back, so the two cannot disagree.
   */
  let scale = $state(1);
  const slot = $derived(240 * scale);
  const band = $derived(132 * scale);
  const tabH = $derived(40 * scale);
  const floor = $derived(24 * scale);

  /** The window's own height: the band, the tallest sheet, the floor. Measured
   *  on the frame the deck paints, so the pane is never taller than what it
   *  holds — a short week is a short sheet on the deck's own surface instead of
   *  white space under a foot-pinned write. */
  let winH = $state(0);
  /** Each sheet's own height, cached from the same measurement. */
  let heights: number[] = [];
  let booted = false;
  let lastHeld = -1;
  let raf = 0;

  let win = $state<HTMLDivElement | null>(null);
  let deck = $state<HTMLDivElement | null>(null);
  let here = $state<HTMLParagraphElement | null>(null);
  let prevBtn = $state<HTMLButtonElement | null>(null);
  let nextBtn = $state<HTMLButtonElement | null>(null);

  const nowPosition = $derived(weeks.findIndex((week) => week.now));
  const runway = $derived(winH + Math.max(0, weeks.length - 1) * slot);

  /** The register's multiplier, as a number the arithmetic can use. */
  function measure(): void {
    if (!win || !deck) return;
    const value = parseFloat(getComputedStyle(win).getPropertyValue('--ui-s').trim());
    scale = Number.isFinite(value) && value > 0 ? value : 1;
    const sheets = deck.querySelectorAll<HTMLElement>('.psa-sheet');
    if (sheets.length === 0) return;
    heights = [...sheets].map((sheet) => sheet.offsetHeight);
    const tallest = heights.reduce((tall, height) => Math.max(tall, height), 0);
    const want = Math.round(band + tallest + floor);
    if (want !== winH) winH = want;
  }

  /** The reference's keyframes, written to the sheets of one deck: `y = 900`
   *  until the sheet's own step (here the window's height plus a step of
   *  slack), `0` at it, `−48 × depth` after it, and `scale(1 − depth × 0.038)`
   *  once it is behind you. A sheet the stack has pushed off the window is
   *  `inert` and unpainted rather than drawn four deep. */
  function draw(): void {
    if (!win || !deck) return;
    const sheets = [...deck.querySelectorAll<HTMLElement>('.psa-sheet')];
    if (sheets.length === 0) return;
    if (heights.length !== sheets.length) measure();
    const height = win.clientHeight || winH;
    const step = win.scrollTop / slot;
    const held = Math.max(0, Math.min(sheets.length - 1, Math.round(step)));
    sheets.forEach((sheet, index) => {
      const depth = step - index;
      const y = depth < 0 ? Math.min(1, -depth) * (height + 80) : -48 * depth;
      const shrink = depth < 0 ? 1 : Math.max(0.51, 1 - 0.038 * depth);
      sheet.style.transform = `translateY(${y.toFixed(1)}px) scale(${shrink.toFixed(4)})`;
      /* Only the sheets that can move this frame hold a layer; sixteen of them
         `will-change`d at once is a promise the deck cannot keep. */
      sheet.style.willChange = Math.abs(depth) < 2.5 ? 'transform' : 'auto';
      /* Off the window entirely: below its floor, or pushed above the band. */
      const away = y > height || y + band + tabH < 0;
      sheet.toggleAttribute('inert', away);
      sheet.toggleAttribute('aria-hidden', away);
      sheet.style.visibility = away ? 'hidden' : '';
      sheet.toggleAttribute('data-held', index === held);
      const grip = sheet.firstElementChild;
      if (grip instanceof HTMLButtonElement) grip.disabled = index === held;
      /* A parked sheet shows its TAB and nothing else: its face is already
         under the sheet you hold. Above a third of a step the face is fully
         covered, so this never pops. */
      const face = sheet.lastElementChild;
      if (face instanceof HTMLElement) {
        face.toggleAttribute('inert', index !== held);
        face.style.visibility = away || depth > 0.3 ? 'hidden' : '';
      }
    });
    if (prevBtn) prevBtn.disabled = win.scrollTop <= 1;
    if (nextBtn) nextBtn.disabled = win.scrollTop >= (sheets.length - 1) * slot - 1;
    /* Is the current week's own tab still on the screen? If the deck has
       carried it off the window, the floor states it instead — never both. */
    if (here && nowPosition >= 0) {
      const depth = step - nowPosition;
      const y = depth < 0 ? Math.min(1, -depth) * (height + 80) : -48 * depth;
      const top = band + y;
      here.hidden = top + tabH > 0 && top < height;
    }
    /* The week the reader has settled on is the panel's selection, so the three
       designs agree on where the student is when they switch. */
    if (held !== lastHeld && weeks[held]) {
      lastHeld = held;
      onSelect(weeks[held].id);
    }
  }

  /** One pass per frame, however fast the scroll arrives. */
  function tick(): void {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      draw();
    });
  }

  // Left/Right arrow keys navigate between weeks.
  function onKey(event: KeyboardEvent): void {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement) return;
    event.preventDefault();
    const held = Math.round((win?.scrollTop ?? 0) / slot);
    seek(held + (event.key === 'ArrowRight' ? 1 : -1));
  }

  let easing: ((progress: number) => number) | null = null;
  function easeOf(element: HTMLElement): (progress: number) => number {
    if (easing) return easing;
    const declared = getComputedStyle(element).getPropertyValue('--ease').trim();
    const parts = /cubic-bezier\(\s*([\d.]+)\s*,\s*(-?[\d.]+)\s*,\s*([\d.]+)\s*,\s*(-?[\d.]+)\s*\)/.exec(
      declared,
    );
    if (!parts) return (easing = (progress) => 1 - (1 - progress) ** 3);
    const [, x1, y1, x2, y2] = parts.map(Number);
    const at = (a: number, b: number, t: number) => ((1 - t) ** 2 * t * a + (1 - t) * t * t * b) * 3 + t ** 3;
    return (easing = (x) => {
      let low = 0;
      let high = 1;
      let t = x;
      for (let step = 0; step < 24; step += 1) {
        if (at(x1, x2, t) < x) low = t;
        else high = t;
        t = (low + high) / 2;
      }
      return at(y1, y2, t);
    });
  }

  /**
   * Go to a week. The settle is the deck's one tween — `--dur-3` on `--ease`,
   * both read from the tokens they are declared on — and reduced motion makes
   * it a jump. Focus lands on the sheet, because the sheet is what moved.
   */
  function seek(index: number): void {
    if (!win || weeks.length === 0) return;
    const to = Math.max(0, Math.min(weeks.length - 1, index)) * slot;
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const settle = () => {
      const sheet = deck?.querySelectorAll<HTMLElement>('.psa-sheet')[Math.round(win.scrollTop / slot)];
      sheet?.focus({ preventScroll: true });
    };
    if (reduced || Math.abs(to - win.scrollTop) < 2) {
      win.scrollTop = to;
      draw();
      settle();
      return;
    }
    const from = win.scrollTop;
    const declared = parseFloat(getComputedStyle(win).getPropertyValue('--dur-3').trim());
    const duration = Number.isFinite(declared) ? declared : 350;
    const ease = easeOf(win);
    let started = -1;
    const frame = (now: number) => {
      if (started < 0) started = now;
      const progress = Math.min(1, (now - started) / duration);
      win.scrollTop = from + (to - from) * ease(progress);
      /* The pass that writes the sheets has to run on every frame of the tween:
         the scroll listener yields while this holds the frame. */
      draw();
      if (progress < 1) raf = requestAnimationFrame(frame);
      else {
        raf = 0;
        win.scrollTop = to;
        draw();
        settle();
      }
    };
    raf = requestAnimationFrame(frame);
  }

  /* The first pass, and again whenever the term changes: measure, then draw.
     On arrival the deck opens on the week the panel has selected — the engine's
     week, or whatever the student left the screen reading — and that first
     settle is a jump, not a tween. */
  $effect(() => {
    const count = weeks.length;
    const anchor = selectedId;
    untrack(() => {
      if (!win || !deck || count === 0) return;
      measure();
      if (!booted) {
        booted = true;
        const index = Math.max(0, weeks.findIndex((week) => week.id === anchor));
        lastHeld = index;
        win.scrollTop = index * slot;
      }
      draw();
    });
  });

  /* A resize re-measures: the sheets' heights move with the container's width
     and with the size register, so the window and the arithmetic follow. */
  $effect(() => {
    const element = win;
    if (!element) return;
    const pass = () => {
      measure();
      draw();
    };
    const observer = new ResizeObserver(pass);
    observer.observe(element);
    window.addEventListener('resize', pass);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', pass);
      if (raf) cancelAnimationFrame(raf);
    };
  });
</script>

<div class="v-fit psa">
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">
        {summary.weeks} {summary.weeks === 1 ? 'week' : 'weeks'} · {summary.things}
        {summary.things === 1 ? 'thing' : 'things'} to get through · {summary.done} of {summary.things} done
        {#if summary.unplaced > 0}· {summary.unplaced} not placed in a week{/if}
      </p>
    </div>
  </header>

  <div
    class="psa-win"
    bind:this={win}
    tabindex="0"
    role="region"
    aria-label="The term's deck"
    style={`--x-win-h: ${winH}px`}
    onscroll={tick}
    onkeydown={onKey}
  >
    <div class="psa-runway" style={`height: ${runway}px`}>
      <div class="psa-deck" bind:this={deck}>
        {#each weeks as week, index (week.id)}
          <article
            class="psa-sheet"
            data-week={week.id}
            data-now={week.now ? '' : undefined}
            tabindex="-1"
            style={`z-index: ${700 + index * 10}`}
          >
            <button
              class="psa-tab"
              type="button"
              aria-current={week.now ? 'true' : undefined}
              onclick={() => seek(index)}
            >
              <span class="psa-tab__n num">W{week.index}</span>
              {#if week.dates}<span class="psa-tab__d num">{week.dates}</span>{/if}
              {#if week.now}<span class="psa-now">now</span>{/if}
            </button>
            <div class="psa-face">
              <div class="psa-face__head">
                {#if week.phase}<span class="psa-face__p">{week.phase}</span>{:else}<span></span>{/if}
                <span class="psa-face__fig num">{readOf(week)}</span>
              </div>
              {#if week.topics.length === 0}
                <p class="psa-void">No topics in this week.</p>
              {:else}
                <div class="psa-rows">
                  {#each week.topics as topic (topic.id)}
                    <button
                      class="psa-row"
                      type="button"
                      data-command="record.panel"
                      data-placement="recordTable.rowContext"
                      data-done={topic.state === 'done' ? '1' : undefined}
                      onclick={() => onTopic(topic)}
                    >
                      <span class="psa-slot" aria-hidden="true">
                        {#if topic.state !== 'none'}
                          <span class="cd-urgency" data-lvl={topic.state === 'done' ? 'done' : 'next'}></span>
                        {/if}
                      </span>
                      <span class="psa-row__t">{topic.label}</span>
                      <span class="psa-row__s">{topic.kind ?? topic.stage}</span>
                      <span class="cd-sr">{topic.stage}</span>
                      <span class="psa-row__m num">{topic.minutes === null ? '' : `${topic.minutes} min`}</span>
                    </button>
                  {/each}
                </div>
              {/if}
              {#if newTopicCommand}
                <button
                  class="psa-write"
                  type="button"
                  data-command={newTopicCommand}
                  data-placement="today.screen"
                  onclick={onNewTopic}
                >
                  <Icon name="plus" size={13} />
                  New topic
                </button>
              {/if}
            </div>
          </article>
        {/each}

        <!-- “You are here”, kept on the deck even when the week's own tab has
             been carried off the window: the deck's floor states it, and it is
             hidden the moment the tab is visible, so there are never two. -->
        <p class="psa-here" bind:this={here} hidden>
          <span class="psa-dot" aria-hidden="true"></span>
          {#if nowPosition >= 0}
            Now · W{weeks[nowPosition].index}{#if weeks[nowPosition].dates}{` · ${weeks[nowPosition].dates}`}{/if}
          {/if}
        </p>

        <!-- The deck's own two steps, in the stack band's right corner — the
             tabs stack at the left, so this corner is free at every depth.
             They are disabled at the term's two ends. -->
        <div class="psa-nav">
          <button
            class="psa-nav__b"
            type="button"
            bind:this={prevBtn}
            aria-label="The week before"
            onclick={() => seek(Math.round((win?.scrollTop ?? 0) / slot) - 1)}
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
              focusable="false"><path d="M14.5 5.5 8 12l6.5 6.5" /></svg
            >
          </button>
          <button
            class="psa-nav__b"
            type="button"
            bind:this={nextBtn}
            aria-label="The week after"
            onclick={() => seek(Math.round((win?.scrollTop ?? 0) / slot) + 1)}
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
              focusable="false"><path d="M9.5 5.5 16 12l-6.5 6.5" /></svg
            >
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  /* THE DECK'S WINDOW. The pane is the deck's own measured height (band +
     tallest sheet + floor), so it never holds more room than the sheets need;
     the fallback is the register's version of the lab's 660px replica. */
  .psa-win {
    position: relative;
    height: var(--x-win-h, calc(620px * var(--ui-s)));
    overflow: hidden auto;
    overscroll-behavior: contain;
  }
  .psa-win:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* The runway is the deck's whole scroll: one slot per week after the first.
     Its height is written from the same arithmetic the pass uses. */
  .psa-runway {
    position: relative;
  }
  .psa-deck {
    position: sticky;
    top: 0;
    height: var(--x-win-h, calc(620px * var(--ui-s)));
    margin: 0;
    overflow: hidden;
  }
  /* The sheet you are holding sits a band down, so the weeks behind it have
     room to stack their tabs in. */
  .psa-sheet {
    --psa-line: var(--space-xl);
    position: absolute;
    left: 0;
    right: 0;
    top: calc(132px * var(--ui-s));
    transform-origin: center top;
  }
  .psa-sheet:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -3px;
    border-radius: var(--r-item) var(--r-item) var(--r-card) var(--r-card);
  }

  /* The folder tab: the week's own label, on the sheet's top-left. Its surface
     eases over `--dur-2` when a step hands the sheet the frame. */
  .psa-tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    height: calc(40px * var(--ui-s));
    padding: 0 var(--psa-line);
    border: 0;
    border-radius: var(--r-item) var(--r-item) 0 0;
    background: var(--well);
    color: var(--ink-2);
    font: inherit;
    font-size: calc(var(--text-xs) * var(--ui-s));
    cursor: pointer;
    transition: background var(--dur-2) var(--ease), color var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .psa-tab:hover { background: var(--well-2); color: var(--ink); }
  .psa-tab:active { background: var(--well-2); }
  .psa-tab:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .psa-tab:disabled { cursor: default; }
  /* A tab still in the stack carries the hairline that says it is a sheet
     behind you; the held sheet's tab merges into its own face and drops it. */
  .psa-sheet:not([data-held]) .psa-tab { box-shadow: inset 0 0 0 1px var(--rule); }
  .psa-sheet[data-held] .psa-tab { background: var(--card); color: var(--ink-2); }
  /* The week you are IN wears the ink tab wherever it stands in the deck — the
     screen's one dark object, and it does not move when you do. */
  .psa-sheet[data-now] .psa-tab { background: var(--ink); color: var(--ink-inv); }
  .psa-sheet[data-now] .psa-tab:hover {
    background-image: linear-gradient(var(--fill-on-ink), var(--fill-on-ink));
  }
  .psa-tab__n {
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .psa-tab__d { letter-spacing: var(--track-title); }
  .psa-now {
    padding: 1px var(--space-xs);
    border-radius: var(--r-pill);
    background: var(--fill-on-ink);
    color: var(--ink-inv);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
  }

  /* The sheet's face: content-sized, and its content sits on ONE line
     (`--psa-line`, the sheet's own 24). The rows and the write reach that line
     from the face's edge, so the sheet's head, its rows, its write and the
     tab's own label are all on one left edge at every width. */
  .psa-face {
    display: flex;
    flex-direction: column;
    padding: var(--space-xl) var(--psa-line) var(--space-lg);
    background: var(--card);
    border-radius: 0 var(--r-card) var(--r-card) var(--r-card);
    box-shadow: var(--sh-2);
  }
  .psa-face > .psa-write { margin-top: var(--space-md); }
  .psa-face__head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-md);
    margin-bottom: var(--space-xs);
  }
  .psa-face__p {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .psa-face__fig {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }

  .psa-rows { display: grid; margin: 0 calc(var(--psa-line) * -1); }
  .psa-row {
    display: grid;
    grid-template-columns: var(--sam-mark) minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(44px * var(--ui-s));
    padding: 0 var(--psa-line);
    text-align: left;
    border: 0;
    background: none;
    font: inherit;
    cursor: pointer;
    transition: background var(--dur-1) var(--ease);
  }
  .psa-row + .psa-row { border-top: 1px dashed var(--rule-strong); }
  .psa-row:hover { background: var(--well); }
  .psa-row:active { background: var(--well-2); }
  .psa-row:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .psa-row__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }
  .psa-row[data-done] .psa-row__t { color: var(--ink); font-weight: var(--weight-label); }
  .psa-row__s {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }
  .psa-row__m {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: right;
    white-space: nowrap;
  }
  /* ONE MARK COLUMN, ALWAYS PRESENT: a row that states no mark still owes the
     body its own track. */
  .psa-slot {
    display: grid;
    place-items: center;
    width: var(--sam-mark);
    height: var(--sam-mark);
  }

  /* The deck's own two steps: 32px boxes in the stack band's right corner. */
  .psa-nav {
    position: absolute;
    inset-block-start: var(--space-xs);
    inset-inline-end: var(--space-xl);
    display: flex;
    gap: var(--space-2xs);
    z-index: 900;
  }
  .psa-nav__b {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border: 0;
    border-radius: var(--r-mini);
    background: var(--card);
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong), var(--sh-1);
    cursor: pointer;
    transition: color var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .psa-nav__b:hover { color: var(--ink); box-shadow: inset 0 0 0 1.5px var(--ink-3), var(--sh-1); }
  .psa-nav__b:active { background: var(--well); }
  .psa-nav__b:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .psa-nav__b:disabled { opacity: 0.45; cursor: default; }
  .psa-nav__b:disabled:hover {
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong), var(--sh-1);
  }

  /* The deck's floor: “you are here”, while the week's own tab is off the
     window. Never two of it. */
  .psa-here {
    position: absolute;
    inset-block-end: var(--space-2xs);
    inset-inline-start: var(--space-xl);
    z-index: 900;
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin: 0;
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .psa-here[hidden] { display: none; }
  .psa-dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--ink);
  }

  /* A row that writes, at the sheet's foot. Deliberately one ink step quieter
     than the rows above it, so it reads as the sheet's door and not as a
     second action competing with them. */
  .psa-write {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    width: calc(100% + var(--psa-line) * 2);
    min-height: var(--hit);
    padding: var(--space-2xs) var(--psa-line);
    margin-inline: calc(var(--psa-line) * -1);
    border: 0;
    border-radius: var(--r-item);
    background: none;
    font: inherit;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: left;
    cursor: pointer;
    transition: color var(--dur-1) var(--ease), background var(--dur-1) var(--ease);
  }
  .psa-write:hover { color: var(--ink); background: var(--well); }
  .psa-write:active { background: var(--well-2); }
  .psa-write:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }

  /* A state line where a list would be: an empty week. */
  .psa-void {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin: 0;
    padding: var(--space-md) 0;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* At a narrow container the deck keeps its deck: the sheet's own line follows
     its padding down, so the tabs, the rows and the write stay on one line, and
     the window's height is re-measured from the shorter sheets. */
  @container (max-width: 760px) {
    .psa-sheet { --psa-line: var(--space-md); }
    .psa-face { padding: var(--space-lg) var(--psa-line) var(--space-md); }
    .psa-nav { inset-inline-end: var(--space-md); }
    .psa-here { inset-inline-start: var(--space-md); }
  }
</style>
