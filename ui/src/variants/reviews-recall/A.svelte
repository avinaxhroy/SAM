<!--
  Reviews & Recall · Variant A (The Top Card).
  Flashcard deck presentation with drag/flick dismissal or keyboard grading.
  Grade rows display the resulting mastery rung (crates/sam-core/src/scheduler.rs:305);
  when ratings yield different due dates, each row prints its specific interval.
-->
<script lang="ts">
  import Answer from './Answer.svelte';
  import Trail from './Trail.svelte';
  import { factsOf, gradeName, rungText, toneClass, type RecallProps } from './props';

  let {
    card,
    position,
    grades,
    returned,
    revealed,
    answer,
    trail,
    onReveal,
    onHide,
    onAnswer,
    onGrade,
    onUndo,
  }: RecallProps = $props();

  /** Rating currently in flight to prevent duplicate submissions. */
  let pressed = $state<string | null>(null);
  /** Direction of card dismissal animation ('l' or 'r'). */
  let fly = $state<'' | 'l' | 'r'>('');
  /** Trigger for card entrance animation. */
  let rising = $state(false);
  /** Tracks transitions initiated locally to manage focus restoration. */
  let handoff = $state(false);
  let seen = $state<string | null>(null);
  let dragDx = $state(0);
  let dragging = $state(false);
  let revealBtn = $state<HTMLButtonElement | null>(null);
  let cardEl = $state<HTMLElement | null>(null);

  let drag: { pointerId: number; x: number; last: number; lastT: number; started: number; dx: number; v: number } | null =
    null;

  const facts = $derived(factsOf(card));
  /** Number of stacked depth sheets to render behind the active card. */
  const below = $derived(position ? Math.max(0, position.of - position.at - 1) : 0);

  /** Reset card animation state and focus reveal button when transitioning cards. */
  $effect(() => {
    const id = card.id;
    if (id === seen) return;
    const mine = handoff;
    seen = id;
    if (!mine) return;
    handoff = false;
    pressed = null;
    fly = '';
    rising = true;
    revealBtn?.focus();
    window.setTimeout(() => (rising = false), 400);
  });

  async function commit(rating: string, side: 'l' | 'r'): Promise<void> {
    if (pressed || !revealed) return;
    const id = card.id;
    pressed = rating;
    handoff = true;
    fly = side;
    await onGrade(rating);
    // Reset flight state on error so user can retry.
    if (card.id === id) {
      handoff = false;
      pressed = null;
      fly = '';
    }
  }

  async function undo(): Promise<void> {
    const id = card.id;
    handoff = true;
    await onUndo();
    if (card.id === id) handoff = false;
  }

  function down(event: PointerEvent): void {
    if (pressed || !revealed || event.button !== 0) return;
    if ((event.target as HTMLElement | null)?.closest('button, input')) return;
    drag = {
      pointerId: event.pointerId,
      x: event.clientX,
      last: event.clientX,
      lastT: Date.now(),
      started: Date.now(),
      dx: 0,
      v: 0,
    };
    dragging = true;
    cardEl?.setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent): void {
    if (!drag || event.pointerId !== drag.pointerId) return;
    drag.dx = event.clientX - drag.x;
    const now = Date.now();
    const elapsed = Math.max(1, now - drag.lastT);
    drag.v = (event.clientX - drag.last) / elapsed;
    drag.last = event.clientX;
    drag.lastT = now;
    dragDx = drag.dx;
  }

  function release(commit: boolean): void {
    if (!drag) return;
    const { dx, v, started } = drag;
    drag = null;
    dragging = false;
    dragDx = 0;
    if (!commit) return;
    // Commit on >=80px drag or high-velocity flick (>=0.4 px/ms over >=24px under 250ms).
    const flick = Math.abs(v) >= 0.4 && Math.abs(dx) >= 24 && Date.now() - started < 250;
    if (Math.abs(dx) < 80 && !flick) return;
    void commit(dx < 0 ? 'again' : 'easy', dx < 0 ? 'l' : 'r');
  }
</script>

<div class="v-fit rva">
  <div class="rva-deck" style={`--dx: ${Math.round(dragDx)}`}>
    <div class="rva-pile" aria-hidden="true">
      <span class="rva-sheet" data-sheet="2" data-on={below > 1 ? '' : undefined}></span>
      <span class="rva-sheet" data-sheet="1" data-on={below > 0 ? '' : undefined}></span>
    </div>

    <article
      class="rva-card"
      bind:this={cardEl}
      data-revealed={revealed ? '1' : '0'}
      data-fly={fly || undefined}
      data-drag={dragging ? '' : undefined}
      data-rise={rising ? '' : undefined}
      style={dragging ? `transform: translateX(${dragDx}px) rotate(${(dragDx * 0.04).toFixed(2)}deg)` : ''}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={() => release(true)}
      onpointercancel={() => release(false)}
    >
      <header class="rva-head">
        {#if card.course}
          <span class="cd-chip cd-chip--code num" data-w={card.wash}>{card.course}</span>
        {/if}
        <span class="cd-chip {toneClass(card.tone)}">{card.state}</span>
        {#if position}
          <span class="rva-pos num">{position.at + 1} of {position.of}</span>
        {/if}
      </header>

      <h2 class="rva-q">{card.title}</h2>
      <p class="rva-facts num">{facts}</p>

      <div class="rva-act">
        <button
          class="cd-pill"
          type="button"
          bind:this={revealBtn}
          data-command="reviews.due"
          data-placement="reviews.panel"
          aria-expanded={revealed}
          aria-controls={revealed ? 'rva-after' : undefined}
          onclick={() => (revealed ? onHide() : onReveal())}
        >
          {revealed ? 'Hide the answer' : 'Reveal the answer'}
          <span class="cd-kbd">Space</span>
        </button>
      </div>

      {#if revealed}
        <div id="rva-after" class="rva-after">
          <Answer cardId={card.id} {answer} {onAnswer} />

          <div class="cd-tasks rva-grades">
            {#each grades as grade (grade.rating)}
              <button
                class="cd-task rva-grade"
                type="button"
                data-command="record.logReview"
                data-placement="reviews.panel"
                data-rating={grade.rating}
                data-sel={pressed === grade.rating ? '' : undefined}
                aria-label={gradeName(grade)}
                onclick={() => void commit(grade.rating, grade.rating === 'again' || grade.rating === 'hard' ? 'l' : 'r')}
              >
                <span class="cd-kbd">{grade.key}</span>
                <span class="rva-grade__l">{grade.label}</span>
                <span class="rva-grade__r">
                  {#if grade.consequence}
                    <span class="rva-grade__c">{grade.consequence}</span>
                  {/if}
                  <span class="cd-lvl" data-v={Math.max(1, Math.min(3, grade.rung))} aria-hidden="true">
                    <i></i><i></i><i></i>
                  </span>
                  <span class="num">{rungText(grade)}</span>
                </span>
              </button>
            {/each}
          </div>

          {#if returned}
            <p class="rva-return">
              <span class="rva-return__d num">{returned.day}</span>
              <span>{returned.say}</span>
            </p>
          {/if}
        </div>
      {/if}
    </article>

    <div class="rva-sides" aria-hidden="true">
      <span data-side="l">Forgot</span>
      <span data-side="r">Solid</span>
    </div>
  </div>

  {#if trail}
    <Trail label={trail} onUndo={() => void undo()} />
  {/if}
</div>

<style>
  /* Deck container and elevation layers. */
  .rva {
    position: relative;
    padding-bottom: calc(56px * var(--ui-s));
  }
  .rva-deck {
    position: relative;
    width: min(640px, 100%);
    margin: var(--ui-gap-lg) auto var(--ui-gap-sm);
    padding-top: calc(32px * var(--ui-s));
  }
  .rva-pile {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: calc(32px * var(--ui-s));
  }
  /* Depth sheets layered behind the active card. */
  .rva-sheet {
    position: absolute;
    left: 50%;
    height: calc(120px * var(--ui-s));
    border-radius: var(--r-card);
    background: var(--card);
    box-shadow: var(--sh-1);
    opacity: 0;
    transform: translateX(-50%) translateY(calc(8px * var(--ui-s)));
    transition: opacity var(--dur-2) var(--ease), transform var(--dur-2) var(--ease);
  }
  .rva-sheet[data-sheet='1'] {
    width: 94%;
    top: calc(16px * var(--ui-s));
  }
  .rva-sheet[data-sheet='2'] {
    width: 88%;
    top: 0;
  }
  .rva-sheet[data-on] {
    opacity: 1;
    transform: translateX(-50%);
  }

  .rva-card {
    position: relative;
    padding: calc(20px * var(--ui-s));
    background: var(--card);
    border-radius: var(--r-card);
    box-shadow: var(--sh-2);
    transition: transform var(--dur-3) var(--ease-pop), opacity var(--dur-3) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .rva-card[data-revealed='1'] {
    cursor: grab;
  }
  .rva-card[data-drag] {
    cursor: grabbing;
    box-shadow: var(--sh-3);
    transition: none;
  }
  /* Card exit animation: delays fade until card is near the edge. */
  .rva-card[data-fly] {
    transition: transform var(--dur-3) var(--ease-pop), opacity var(--dur-2) linear var(--dur-2);
  }
  .rva-card[data-fly='l'] {
    transform: translateX(-130%) rotate(-9deg);
    opacity: 0;
  }
  .rva-card[data-fly='r'] {
    transform: translateX(130%) rotate(9deg);
    opacity: 0;
  }
  @keyframes rva-rise {
    from {
      opacity: 0;
      transform: translateY(calc(20px * var(--ui-s))) scale(0.97);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .rva-card[data-rise] {
    animation: rva-rise var(--dur-3) var(--ease-pop) both;
  }

  .rva-head {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
    margin-bottom: var(--ui-gap);
  }
  .rva-pos {
    margin-left: auto;
    font-size: var(--ui-meta);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
  }
  /* The design switch takes the head's right end; this row has no spacer. */
  .rva-q {
    margin: 0;
    font-size: calc(20px * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
    line-height: 1.2;
  }
  .rva-facts {
    margin: calc(8px * var(--ui-s)) 0 0;
    font-size: var(--ui-text-sm);
    color: var(--ink-2);
  }
  .rva-act {
    margin-top: calc(20px * var(--ui-s));
  }

  /* Direction indicator labels whose opacity scales with drag displacement. */
  .rva-sides {
    position: absolute;
    top: 0;
    height: calc(32px * var(--ui-s));
    left: 0;
    right: 0;
    pointer-events: none;
    z-index: 3;
  }
  .rva-sides span {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    padding: calc(2px * var(--ui-s)) var(--ui-pad-sm);
    border-radius: var(--r-pill);
    background: var(--pane);
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink);
    white-space: nowrap;
  }
  .rva-sides span[data-side='l'] {
    left: 0;
    opacity: clamp(0, calc(var(--dx, 0) / -80), 1);
  }
  .rva-sides span[data-side='r'] {
    right: 0;
    opacity: clamp(0, calc(var(--dx, 0) / 80), 1);
  }

  /* Full-bleed grade selection rows matching card padding. */
  .rva-grades {
    margin: 0 calc(-1 * calc(20px * var(--ui-s)));
  }
  .rva-grade {
    width: 100%;
    text-align: left;
    min-height: calc(56px * var(--ui-s));
    padding-left: calc(20px * var(--ui-s));
    padding-right: calc(20px * var(--ui-s));
  }
  .rva-grade__l {
    font-size: var(--ui-text);
    font-weight: var(--weight-label);
  }
  .rva-grade__r {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    font-size: var(--ui-meta);
    color: var(--ink-3);
    white-space: nowrap;
  }
  .rva-grade__c {
    color: var(--ink-2);
  }
  .rva-grade[data-sel] {
    background: var(--well-2);
  }

  .rva-return {
    display: flex;
    align-items: baseline;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
    margin: calc(20px * var(--ui-s)) 0 0;
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .rva-return__d {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* Container query for narrow widths. */
  @container (max-width: 520px) {
    .rva-grade {
      grid-template-columns: auto minmax(0, 1fr);
      row-gap: calc(4px * var(--ui-s));
    }
    .rva-grade__r {
      grid-column: 1 / -1;
      justify-content: flex-start;
    }
    .rva-pos {
      margin-left: 0;
    }
  }
</style>
