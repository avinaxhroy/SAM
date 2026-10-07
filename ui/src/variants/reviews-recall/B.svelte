<!-- Reviews & Recall · Variant B (Three Stages). Three-step review flow (Prompt, Evidence, Grade) with keyboard shortcuts (1–4). -->
<script lang="ts">
  import { tick } from 'svelte';
  import Answer from './Answer.svelte';
  import Trail from './Trail.svelte';
  import { factsOf, gradeName, rungBars, rungText, toneClass, type RecallProps } from './props';

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

  const STEPS = ['Prompt', 'Evidence', 'Grade'];

  let step = $state(0);
  let shine = $state<number | null>(null);
  let pressed = $state<string | null>(null);
  let swap = $state(false);
  let handoff = $state(false);
  let seen = $state<string | null>(null);
  let panes = $state<HTMLElement | null>(null);

  const facts = $derived(factsOf(card));

  /** Focuses the primary control of the newly active stage. */
  async function focusPanel(): Promise<void> {
    await tick();
    panes?.querySelector<HTMLElement>('button, input')?.focus();
  }

  $effect(() => {
    const id = card.id;
    if (id === seen) return;
    const mine = handoff;
    seen = id;
    step = 0;
    pressed = null;
    if (!mine) return;
    handoff = false;
    swap = true;
    void focusPanel();
    window.setTimeout(() => (swap = false), 400);
  });

  /** Synchronizes the active step with the revealed state. */
  $effect(() => {
    if (revealed && step === 0) setStep(1, true);
    else if (!revealed && step !== 0) setStep(0, true);
  });

  function setStep(next: number, sweep: boolean): void {
    step = next;
    if (!sweep) return;
    shine = next;
    window.setTimeout(() => (shine = null), 640);
  }

  function toggle(): void {
    if (pressed) return;
    if (revealed) {
      onHide();
      setStep(0, true);
      void focusPanel();
      return;
    }
    onReveal();
    setStep(1, true);
    void focusPanel();
  }

  async function commit(rating: string): Promise<void> {
    if (pressed || !revealed) return;
    const id = card.id;
    pressed = rating;
    handoff = true;
    await onGrade(rating);
    if (card.id === id) {
      handoff = false;
      pressed = null;
    }
  }

  async function undo(): Promise<void> {
    const id = card.id;
    handoff = true;
    await onUndo();
    if (card.id === id) handoff = false;
  }
</script>

<div class="v-fit rvb">
  <article class="cd-card rvb-card" data-swap={swap ? '' : undefined}>
    <header class="rvb-head">
      {#if card.course}
        <span class="cd-chip cd-chip--code num" data-w={card.wash}>{card.course}</span>
      {/if}
      <span class="cd-chip {toneClass(card.tone)}">{card.state}</span>
      {#if position}
        <span class="rvb-pos num">{position.at + 1} of {position.of}</span>
      {/if}
    </header>

    <h2 class="rvb-q">{card.title}</h2>

    <ol class="rvb-steps">
      {#each STEPS as word, index (word)}
        <li>
          <button
            class="rvb-step"
            type="button"
            aria-current={step === index ? 'step' : undefined}
            aria-disabled={index > 0 && !revealed ? 'true' : undefined}
            data-shine={shine === index ? '' : undefined}
            onclick={() => {
              if (index > 0 && !revealed) return;
              if (index !== step) setStep(index, true);
            }}
          >
            <span class="rvb-step__n num">{index + 1}</span>
            <span class="rvb-step__w">{word}</span>
          </button>
        </li>
      {/each}
    </ol>

    <div class="rvb-panes" bind:this={panes}>
      {#if step === 0}
        <section class="rvb-pane" aria-label="Prompt">
          <p class="rvb-facts num">{facts}</p>
          <div class="rvb-act">
            <button
              class="cd-pill"
              type="button"
              data-command="reviews.due"
              data-placement="reviews.panel"
              aria-expanded={revealed}
              onclick={toggle}
            >
              {revealed ? 'Hide the answer' : 'Reveal the answer'}
              <span class="cd-kbd">Space</span>
            </button>
          </div>
        </section>
      {:else if step === 1}
        <section class="rvb-pane" aria-label="Evidence">
          <Answer cardId={card.id} {answer} {onAnswer} />
          <p class="rvb-note num">
            {card.recalls} {card.recalls === 1 ? 'recall' : 'recalls'} logged{#if card.minutes !== null} · {card.minutes} min in the plan{/if}
          </p>
        </section>
      {:else}
        <section class="rvb-pane" aria-label="Grade">
          <div class="rvb-grades" data-count={grades.length}>
            {#each grades as grade (grade.rating)}
              <button
                class="rvb-grade"
                type="button"
                data-command="record.logReview"
                data-placement="reviews.panel"
                data-rating={grade.rating}
                data-sel={pressed === grade.rating ? '' : undefined}
                aria-label={gradeName(grade)}
                onclick={() => void commit(grade.rating)}
              >
                <span class="cd-kbd">{grade.key}</span>
                <span class="rvb-grade__l">{grade.label}</span>
                <span class="rvb-grade__m" aria-hidden="true">
                  {#each [0, 1, 2] as bar (bar)}
                    <i data-on={bar < rungBars(grade) ? '' : undefined}></i>
                  {/each}
                </span>
                {#if grade.consequence}
                  <span class="rvb-grade__c">{grade.consequence}</span>
                {/if}
                <span class="rvb-grade__r num">{rungText(grade)}</span>
              </button>
            {/each}
          </div>
          {#if returned}
            <p class="rvb-return">
              <span class="rvb-return__d num">{returned.day}</span>
              <span>{returned.say}</span>
            </p>
          {/if}
        </section>
      {/if}
    </div>
  </article>

  {#if trail}
    <Trail label={trail} onUndo={() => void undo()} />
  {/if}
</div>

<style>
  .rvb {
    position: relative;
    padding-bottom: calc(56px * var(--ui-s));
    display: grid;
    justify-items: center;
  }
  .rvb-card {
    width: min(720px, 100%);
  }
  @keyframes rvb-swap {
    from {
      opacity: 0;
      transform: translateY(calc(6px * var(--ui-s)));
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .rvb-card[data-swap] {
    animation: rvb-swap var(--dur-2) var(--ease) both;
  }

  .rvb-head {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
  }
  .rvb-pos {
    margin-left: auto;
    font-size: var(--ui-meta);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
  }
  /* The design switch takes the head's right end; this row has no spacer. */
  .rvb-q {
    margin: calc(20px * var(--ui-s)) 0 0;
    font-size: calc(24px * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.1;
  }

  /* Step indicator pills with label expansion when active. */
  .rvb-steps {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    margin: calc(20px * var(--ui-s)) 0 0;
    padding: 0;
    list-style: none;
    flex-wrap: wrap;
  }
  .rvb-step {
    position: relative;
    overflow: hidden;
    display: inline-flex;
    align-items: center;
    height: var(--pill-h);
    padding: 0 var(--ui-pad-sm);
    border-radius: var(--r-pill);
    background: var(--well);
    color: var(--ink-2);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .rvb-step:hover {
    background: var(--well-2);
    color: var(--ink);
  }
  .rvb-step:active {
    transform: scale(0.97);
  }
  .rvb-step[aria-current='step'] {
    background: var(--card);
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink), var(--sh-1);
  }
  /* Disabled step styling using theme ink token. */
  .rvb-step[aria-disabled='true'] {
    color: var(--ink-3);
    pointer-events: none;
  }
  .rvb-step__n {
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
  }
  .rvb-step__w {
    max-width: 0;
    opacity: 0;
    overflow: hidden;
    white-space: nowrap;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    transition: max-width var(--dur-2) var(--ease-pop), opacity var(--dur-1) var(--ease),
      margin-left var(--dur-2) var(--ease-pop);
  }
  .rvb-step[aria-current='step'] .rvb-step__w {
    max-width: 11ch;
    opacity: 1;
    margin-left: calc(8px * var(--ui-s));
  }
  /* Activation sweep animation on step selection. */
  .rvb-step::after {
    content: '';
    position: absolute;
    top: -20%;
    bottom: -20%;
    left: -55%;
    width: 45%;
    background: linear-gradient(105deg, transparent, var(--glass-line) 50%, transparent);
    opacity: 0;
    pointer-events: none;
  }
  .rvb-step[data-shine]::after {
    animation: rvb-shine 600ms var(--ease) 1;
  }
  @keyframes rvb-shine {
    from {
      transform: translateX(0);
      opacity: 0.9;
    }
    to {
      transform: translateX(360%);
      opacity: 0;
    }
  }

  /* Stage panel transitions. */
  .rvb-panes {
    margin-top: calc(24px * var(--ui-s));
  }
  .rvb-pane {
    border-radius: var(--r-tile);
    background: var(--well);
    padding: var(--ui-pad);
    animation: rvb-in var(--dur-2) var(--ease) both;
  }
  @keyframes rvb-in {
    from {
      opacity: 0;
      filter: blur(4px);
      transform: translateY(calc(4px * var(--ui-s)));
    }
    to {
      opacity: 1;
      filter: blur(0);
      transform: none;
    }
  }
  .rvb-facts {
    margin: 0;
    font-size: var(--ui-text);
    color: var(--ink-2);
  }
  .rvb-note {
    margin: var(--ui-gap-sm) 0 0;
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .rvb-act {
    margin-top: var(--ui-gap);
  }

  /* Segmented grade selection strip. */
  .rvb-grades {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: calc(4px * var(--ui-s));
    padding: calc(4px * var(--ui-s));
    border-radius: var(--r-tile);
    background: var(--well-2);
  }
  /* Two-rating configuration layout. */
  .rvb-grades[data-count='2'] {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    width: calc(50% - var(--ui-gap-sm) / 2);
  }
  .rvb-grade {
    display: grid;
    justify-items: center;
    gap: calc(4px * var(--ui-s));
    padding: var(--ui-pad-sm) calc(8px * var(--ui-s));
    border-radius: var(--r-item);
    background: var(--card);
    color: var(--ink-2);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .rvb-grade:hover {
    color: var(--ink);
    box-shadow: var(--sh-1);
  }
  .rvb-grade:active {
    transform: scale(0.98);
  }
  .rvb-grade[data-sel] {
    background: var(--ink);
    color: var(--ink-inv);
    box-shadow: var(--sh-ink);
  }
  .rvb-grade__l {
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-label);
  }
  .rvb-grade__m {
    display: flex;
    gap: 3px;
  }
  .rvb-grade__m i {
    width: 9px;
    height: 4px;
    border-radius: var(--r-pill);
    background: var(--rule-strong);
  }
  .rvb-grade__m i[data-on] {
    background: var(--ink);
  }
  .rvb-grade__r {
    font-size: var(--ui-meta);
    letter-spacing: var(--track-title);
    color: var(--ink-3);
  }
  .rvb-grade__c {
    font-size: var(--ui-meta);
    color: var(--ink-2);
    text-align: center;
  }
  .rvb-grade[data-sel] .rvb-grade__m i[data-on] {
    background: var(--ink-inv);
  }
  .rvb-grade[data-sel] .rvb-grade__r,
  .rvb-grade[data-sel] .rvb-grade__c {
    color: var(--ink-inv);
  }

  .rvb-return {
    display: flex;
    align-items: baseline;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
    margin: var(--ui-gap) 0 0;
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .rvb-return__d {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  @container (max-width: 560px) {
    .rvb-grades {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .rvb-pos {
      margin-left: 0;
    }
  }
</style>
