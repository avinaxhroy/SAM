<!-- Reviews & Recall · Variant C (The Split). Split layout with review queue on left and answer/evidence on right. -->
<script lang="ts">
  import { tick } from 'svelte';
  import Answer from './Answer.svelte';
  import Trail from './Trail.svelte';
  import { gradeName, rungFill, rungText, toneClass, type RecallProps } from './props';

  let {
    card,
    sitting,
    more,
    grades,
    returned,
    revealed,
    answer,
    trail,
    onReveal,
    onHide,
    onAnswer,
    onGrade,
    onSelect,
    onUndo,
  }: RecallProps = $props();

  let ghost = $state<HTMLElement | null>(null);
  let live = $state<HTMLElement | null>(null);
  let pressed = $state<string | null>(null);
  /** A snapshot has been taken and the record is about to change. */
  let pending = $state(false);
  /** The live plane is settling out of the blur the snapshot is leaving behind. */
  let arriving = $state(false);
  /** The snapshot is fading behind the live plane. */
  let fading = $state(false);
  let seen = $state<string | null>(null);
  let revealBtn = $state<HTMLButtonElement | null>(null);

  const reduced = $derived(
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches,
  );

  /** Clones current card into transition layer before switching cards.
   *  `cloneNode` copies nodes; it never re-parses serialized HTML, so this
   *  stays a paint trick even if the card ever renders richer content. */
  function snapshot(): void {
    if (reduced || !ghost || !live) return;
    ghost.replaceChildren(live.cloneNode(true));
    for (const node of ghost.querySelectorAll('[id]')) node.removeAttribute('id');
    pending = true;
  }

  $effect(() => {
    const id = card.id;
    if (id === seen) return;
    const mine = pending;
    seen = id;
    if (!mine) return;
    pending = false;
    arriving = true;
    fading = true;
    window.setTimeout(() => {
      if (ghost) ghost.innerHTML = '';
      fading = false;
      arriving = false;
    }, 420);
  });

  function go(id: string): void {
    if (id === card.id || pressed) return;
    snapshot();
    onSelect(id);
  }

  function toggle(): void {
    if (pressed) return;
    if (revealed) onHide();
    else onReveal();
  }

  async function commit(rating: string): Promise<void> {
    if (pressed || !revealed) return;
    const id = card.id;
    pressed = rating;
    snapshot();
    await onGrade(rating);
    if (card.id === id) {
      pressed = null;
      return;
    }
    // Return focus to the reveal button after card progression.
    await tick();
    revealBtn?.focus();
  }

  async function undo(): Promise<void> {
    const id = card.id;
    snapshot();
    await onUndo();
    if (card.id === id) return;
    await tick();
    revealBtn?.focus();
  }
</script>

<div class="v-fit rvc">
  <div class="rvc-split">
    <div class="rvc-left">
      <ol class="rvc-list">
        {#each sitting as row (row.id)}
          <li>
            <button
              class="rvc-row"
              type="button"
              aria-current={row.id === card.id ? 'true' : undefined}
              onclick={() => go(row.id)}
            >
              <span class="rvc-row__n num">{row.numeral}</span>
              <span class="rvc-row__t">{row.title}</span>
              <span class="rvc-row__s num">{row.state ?? ''}</span>
              <span class="rvc-glyph" aria-hidden="true"><i></i><i></i></span>
            </button>
          </li>
        {/each}
      </ol>
      {#if more}
        <p class="rvc-more num">{more}</p>
      {/if}
    </div>

    <div class="rvc-right">
      <div class="rvc-plane" data-revealed={revealed ? '1' : '0'}>
        <div class="rvc-ghost" bind:this={ghost} class:is-out={fading} aria-hidden="true" inert></div>
        <div class="rvc-live" bind:this={live} class:is-in={arriving}>
          <header class="rvc-phead">
            {#if card.course}
              <span class="cd-chip rvc-onink num" data-w={card.wash}>{card.course}</span>
            {/if}
            <span class="cd-chip rvc-onsink">{card.state}</span>
          </header>
          <div class="rvc-pbody">
            {#if !revealed}
              <div class="rvc-cover">
                <button
                  class="cd-pill rvc-onink"
                  type="button"
                  bind:this={revealBtn}
                  data-command="reviews.due"
                  data-placement="reviews.panel"
                  aria-expanded={false}
                  onclick={toggle}
                >
                  Reveal the answer
                  <span class="cd-kbd">Space</span>
                </button>
              </div>
            {:else}
              <div class="rvc-after">
                <Answer cardId={card.id} {answer} {onAnswer} onink />
                <p class="rvc-facts num">
                  {card.since ?? 'Never recalled'} · {card.recalls}
                  {card.recalls === 1 ? 'recall' : 'recalls'} logged
                  {#if card.minutes !== null}· {card.minutes} min in the plan{/if}
                </p>
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Expandable rating options shown once answer is revealed. -->
  <div class="rvc-band" data-open={revealed ? '' : undefined} inert={revealed ? undefined : true} aria-hidden={!revealed}>
    <div class="rvc-band__in">
      <div class="rvc-band__box">
        <div class="rvc-grades" data-count={grades.length}>
          {#each grades as grade (grade.rating)}
            <button
              class="rvc-grade"
              type="button"
              data-command="record.logReview"
              data-placement="reviews.panel"
              data-rating={grade.rating}
              data-sel={pressed === grade.rating ? '' : undefined}
              aria-label={gradeName(grade)}
              onclick={() => void commit(grade.rating)}
            >
              <span class="rvc-grade__top">
                <span class="cd-kbd">{grade.key}</span>
                <span class="rvc-grade__l">{grade.label}</span>
              </span>
              <span class="rvc-grade__m" aria-hidden="true">
                <i style={`--v: ${rungFill(grade)}%`}></i>
              </span>
              {#if grade.consequence}
                <span class="rvc-grade__c">{grade.consequence}</span>
              {/if}
              <span class="rvc-grade__r num">{rungText(grade)}</span>
            </button>
          {/each}
        </div>
        {#if returned}
          <p class="rvc-return">
            <span class="rvc-return__d num">{returned.day}</span>
            <span>{returned.say}</span>
          </p>
        {/if}
      </div>
    </div>
  </div>

  {#if trail}
    <Trail label={trail} onUndo={() => void undo()} />
  {/if}
</div>

<style>
  .rvc {
    position: relative;
    padding-bottom: calc(56px * var(--ui-s));
    --rvc-pane-pad: calc(20px * var(--ui-s));
  }
  .rvc-split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.06fr);
    align-items: stretch;
  }
  .rvc-left {
    padding: var(--rvc-pane-pad) var(--ui-pad-sm) var(--ui-gap);
  }
  .rvc-right {
    display: flex;
    padding: var(--rvc-pane-pad) var(--rvc-pane-pad) var(--ui-gap);
    border-left: 1px solid var(--rule);
  }

  /* Numbered question queue column. */
  .rvc-list {
    margin: calc(8px * var(--ui-s)) 0 0;
    padding: 0;
    list-style: none;
  }
  .rvc-row {
    display: grid;
    grid-template-columns: calc(30px * var(--ui-s)) minmax(0, 1fr) auto calc(16px * var(--ui-s));
    align-items: center;
    gap: var(--ui-gap-sm);
    width: calc(100% + var(--ui-pad-sm) * 2);
    min-height: calc(52px * var(--ui-s));
    margin: 0 calc(-1 * var(--ui-pad-sm));
    padding: calc(8px * var(--ui-s)) var(--ui-pad-sm);
    text-align: left;
    color: var(--ink-3);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  li + li .rvc-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .rvc-row:hover {
    background: var(--well);
    color: var(--ink-2);
  }
  .rvc-row[aria-current='true'] {
    color: var(--ink);
  }
  .rvc-row__n {
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-label);
  }
  .rvc-row__t {
    font-size: var(--ui-text);
    font-weight: var(--weight-body);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rvc-row[aria-current='true'] .rvc-row__t {
    font-weight: var(--weight-label);
  }
  .rvc-row__s {
    font-size: var(--ui-meta);
    text-align: right;
    white-space: nowrap;
    letter-spacing: var(--track-title);
  }
  .rvc-row[aria-current='true'] .rvc-row__s {
    color: var(--ink-2);
  }
  /* Plus to minus rotation indicator. */
  .rvc-glyph {
    position: relative;
    width: 14px;
    height: 14px;
    justify-self: end;
  }
  .rvc-glyph i {
    position: absolute;
    background: currentColor;
    border-radius: var(--r-pill);
    transition: transform var(--dur-2) var(--ease);
  }
  .rvc-glyph i:first-child {
    left: 0;
    top: 50%;
    width: 100%;
    height: 1.5px;
    transform: translateY(-50%);
  }
  .rvc-glyph i:last-child {
    left: 50%;
    top: 0;
    width: 1.5px;
    height: 100%;
    transform: translateX(-50%);
  }
  .rvc-row[aria-current='true'] .rvc-glyph i:last-child {
    transform: translateX(-50%) rotate(90deg);
  }
  .rvc-more {
    margin: calc(16px * var(--ui-s)) 0 0;
    padding: 0 calc(4px * var(--ui-s));
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }

  /* Answer plane with crossfade transition container. */
  .rvc-plane {
    position: relative;
    display: flex;
    flex: 1;
    min-height: calc(360px * var(--ui-s));
    padding: calc(20px * var(--ui-s));
    border-radius: var(--r-card);
    background: var(--ink);
    color: var(--ink-inv);
  }
  .rvc-live {
    position: relative;
    z-index: 1;
    display: flex;
    flex: 1;
    flex-direction: column;
  }
  .rvc-ghost {
    position: absolute;
    inset: 0;
    z-index: 0;
    padding: calc(20px * var(--ui-s));
    pointer-events: none;
  }
  .rvc-live.is-in {
    animation: rvc-in var(--dur-3) var(--ease) both;
  }
  .rvc-ghost.is-out {
    animation: rvc-out var(--dur-3) var(--ease) both;
  }
  @keyframes rvc-in {
    from {
      opacity: 0;
      filter: blur(10px);
      transform: scale(1.02);
    }
    to {
      opacity: 1;
      filter: blur(0);
      transform: none;
    }
  }
  @keyframes rvc-out {
    from {
      opacity: 1;
      filter: blur(0);
      transform: none;
    }
    to {
      opacity: 0;
      filter: blur(10px);
      transform: scale(1.03);
    }
  }

  .rvc-phead {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
  }
  /* The design switch takes the head's right end; this row has no spacer. */
  .rvc-pbody {
    display: flex;
    flex: 1;
    flex-direction: column;
    justify-content: center;
  }
  .rvc-cover {
    display: grid;
    flex: 1;
    place-items: center;
  }
  /* Pill and chip styles for high-contrast dark plane. */
  .rvc-onink {
    background: var(--fill-on-ink);
    color: var(--ink-inv);
  }
  .rvc-onink:hover {
    background: var(--fill-on-ink-hi);
  }
  .rvc-onsink {
    background: var(--fill-on-ink);
    color: var(--ink-inv);
  }
  .rvc-facts {
    margin: calc(20px * var(--ui-s)) 0 0;
    font-size: var(--ui-text-sm);
    color: color-mix(in oklab, var(--ink-inv) 74%, transparent);
  }

  /* Expandable rating band transition. */
  .rvc-band {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-3) var(--ease);
  }
  .rvc-band[data-open] {
    grid-template-rows: 1fr;
  }
  .rvc-band__in {
    overflow: hidden;
    min-height: 0;
  }
  .rvc-band__box {
    border-top: 1px solid var(--rule);
    padding: calc(20px * var(--ui-s)) var(--rvc-pane-pad) var(--ui-gap);
  }
  .rvc-grades {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--ui-gap-sm);
  }
  /* Two cells are the same cells as four (`reviews.css`'s `.rv__rate--two`). */
  .rvc-grades[data-count='2'] {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    width: calc(50% - var(--ui-gap-sm) / 2);
  }
  .rvc-grade {
    display: grid;
    gap: calc(4px * var(--ui-s));
    text-align: left;
    padding: calc(10px * var(--ui-s));
    border-radius: var(--r-tile);
    background: var(--well);
    color: var(--ink-2);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease), transform var(--dur-1) var(--ease);
  }
  .rvc-grade:hover {
    background: var(--well-2);
    color: var(--ink);
  }
  .rvc-grade:active {
    transform: scale(0.98);
  }
  .rvc-grade[data-sel] {
    box-shadow: inset 0 0 0 2px var(--ink);
    color: var(--ink);
  }
  .rvc-grade__top {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
  }
  .rvc-grade__l {
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-label);
  }
  .rvc-grade__m {
    height: 3px;
    border-radius: var(--r-pill);
    background: var(--rule-strong);
    overflow: hidden;
  }
  .rvc-grade__m i {
    display: block;
    height: 100%;
    width: var(--v, 0%);
    border-radius: var(--r-pill);
    background: var(--ink);
    transition: width var(--dur-3) var(--ease);
  }
  .rvc-grade__r {
    font-size: var(--ui-meta);
    color: var(--ink-3);
  }
  .rvc-grade__c {
    font-size: var(--ui-meta);
    color: var(--ink-2);
  }
  .rvc-return {
    display: flex;
    align-items: baseline;
    gap: var(--ui-gap-sm);
    flex-wrap: wrap;
    margin: var(--ui-gap) 0 0;
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
  }
  .rvc-return__d {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* ── narrow: the two panes stack, the seam turns into a top rule, and the
     four grade cells fold to two columns. Measured from the container, not the
     window: this surface sits in a scrolling canvas of its own. */
  @container (max-width: 900px) {
    .rvc-split {
      grid-template-columns: minmax(0, 1fr);
    }
    .rvc-right {
      padding-top: 0;
      border-left: 0;
    }
    .rvc-left {
      border-bottom: 1px solid var(--rule);
    }
  }
  @container (max-width: 620px) {
    .rvc-grades {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
