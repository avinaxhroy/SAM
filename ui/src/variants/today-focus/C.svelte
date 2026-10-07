<!--
  Today focus variant C: The Focus Card.
  The demo's own `.cd-focus` (design/components.css §9, design/demo) rebuilt as
  a screen: the course wash, the chip-and-tag eyebrow with `Not this` beside its
  spacer, the title at the card's own step, the why-line, Start with the length
  and the record's own door — and the 180° coverage arc in the second track.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import Duration from './Duration.svelte';
  import { nameOf } from '../../types';
  import type { TodayFocusProps } from './props';

  let {
    greeting,
    dateLine,
    mode,
    title,
    course,
    tag,
    why,
    studiedToday,
    minutes,
    onMinutes,
    sessionCommand,
    onStart,
    onPlan,
    coverage,
    kind,
    onOpen,
    onDefer,
  }: TodayFocusProps = $props();

  /** The arc, clamped the way the demo clamps it (demo.js `gauge()`). */
  const arc = $derived(
    coverage ? Math.max(0, Math.min(100, Math.round(coverage.value))) : 0,
  );

  /**
   * `topic` → `Open topic`: the pill's noun is the plan's own word for the
   * kind, taken through `nameOf`, never a word this screen invented.
   */
  const openLabel = $derived(
    kind ? `Open ${nameOf(kind).replace(/^./, (letter) => letter.toLowerCase())}` : null,
  );
</script>

<div class="v-fit tfc">
  <header class="tf-head">
    <h1 class="tf-greet">{greeting}</h1>
    <p class="tf-date">{dateLine}</p>
  </header>

  <!-- The card is `.cd-focus` itself — its surface, its two tracks and its own
       slots — so every state is one shape and the states cannot drift apart.
       The all-clear wears the demo's mint; the decision and the gap wear the
       focused course's wash. -->
  <section
    class="cd-focus xc-card"
    data-w={mode === 'clear' ? 'mint' : (course?.wash ?? undefined)}
  >
    <div>
      {#if mode === 'clear'}
        <div class="cd-focus__eyebrow">
          <span class="tf-mark" aria-hidden="true"><Icon name="check" size={13} /></span>
        </div>
        <h2 class="cd-focus__text tw-focus__text xc-title">Nothing waiting</h2>
        <p class="cd-focus__why">
          <span class="xc-why__k">Studied today</span>
          <b>{studiedToday}</b>
        </p>
        <div class="cd-focus__acts">
          <button class="cd-pill cd-pill--lg cd-pill--quiet" type="button" onclick={onPlan}>
            Plan a session
          </button>
        </div>
      {:else}
        <div class="cd-focus__eyebrow">
          {#if course}
            <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.label}</span>
          {/if}
          {#if tag}
            <span class="cd-focus__tag">{tag}</span>
          {/if}
          <!-- "Not this" sits with the claim, not with the actions: it is a
               verdict on the recommendation, not a way to run the session. -->
          <span class="cd-focus__spacer"></span>
          <button
            class="cd-pill cd-pill--sm cd-pill--quiet"
            type="button"
            data-command="record.defer"
            data-placement="today.screen"
            onclick={() => void onDefer()}
          >
            Not this
          </button>
        </div>
        <h2 class="cd-focus__text tw-focus__text xc-title" title={title ?? undefined}>{title}</h2>
        {#if why}
          <p class="cd-focus__why">
            <span class="xc-why__k">{why.key}</span>
            <b>{why.value}</b>
          </p>
        {/if}
        <div class="cd-focus__acts">
          <button
            class="cd-pill cd-pill--lg"
            type="button"
            data-command={sessionCommand ?? undefined}
            data-placement={sessionCommand ? 'today.screen' : undefined}
            disabled={!sessionCommand}
            title={sessionCommand ? 'Start a session on this' : 'This plan has no kind that records minutes'}
            onclick={onStart}
          >
            <Icon name="clock" size={13} />
            Start
          </button>
          <Duration {minutes} onCommit={onMinutes} />
          {#if openLabel}
            <button
              class="cd-pill cd-pill--ghost cd-pill--lg"
              type="button"
              data-command="record.panel"
              data-placement="today.screen"
              onclick={onOpen}
            >
              {openLabel}
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <!-- One arc, 180°, rounded caps, the course's own ink on a light track —
         `.cd-gauge` as the demo draws it. The svg says nothing (aria-hidden);
         the percent and the caption beside it are the text a reader gets. -->
    {#if coverage && mode !== 'clear'}
      <div data-w={course?.wash ?? undefined}>
        <div class="cd-gauge">
          <svg viewBox="0 0 132 74" aria-hidden="true">
            <path class="cd-gauge__track" d="M8 66 A 58 58 0 0 1 124 66"></path>
            <path
              class="cd-gauge__arc"
              pathLength="100"
              stroke-dasharray={`${arc} 100`}
              d="M8 66 A 58 58 0 0 1 124 66"
            ></path>
          </svg>
          <div class="cd-gauge__val">{arc}%</div>
          <div class="cd-gauge__cap">{coverage.label}</div>
        </div>
      </div>
    {/if}
  </section>
</div>

<style>
  .tfc {
    display: grid;
    align-content: start;
    gap: var(--space-xl);
  }
  .tf-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-md);
  }
  .tf-greet {
    margin: 0;
    color: var(--ink-2);
    font-size: calc(var(--text-xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.05;
  }
  .tf-date {
    margin: 0;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  /* ── C · the focus card ─────────────────────────────────────────────────
     Everything the card itself says — its surface, radius and shadow, its two
     tracks, the eyebrow, the why-line, the acts row and the gauge — is the
     shipped `.cd-focus` / `.cd-gauge` of `design/components.css` (§9, §10), so
     this screen adds only its own four corrections. */
  /* 1 · THE CLAMP NEEDS THE INK'S OWN ROOM. `.tw-focus__text` clamps the title
     to two lines and clips at its own padding box, and the card's leading is
     tighter than the type's ink: measured in the lab, a 40px line carries 51px
     of glyphs (3.9 above / 3.8 below), so caps and descenders were sliced flat
     without this. The ink gets the room as padding and the layout pays nothing,
     because the same measure comes back off as a negative margin — the
     eyebrow→title→why rhythm stays the card's own. */
  .xc-title {
    padding-block: 0.14em;
    margin-block: -0.14em;
  }
  /* 2 · THE WHY'S KEY. The card's why-line puts the value in ink; the key is
     this screen's apparatus — micro caps at --ink-3, so the fact reads in the
     two voices the shared why-line rule states: the key names the fact, the
     value carries its number in ink. A key the record does not carry is
     omitted rather than printed empty. */
  .xc-why__k {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  /* 3 · THE ALL-CLEAR'S MARK. The card carries no eyebrow copy in this state,
     so the slot keeps its shape: the same 20px --card disc a tick sits in,
     on the card's own wash. */
  .tf-mark {
    width: var(--chip-h);
    height: var(--chip-h);
    flex: none;
    border-radius: var(--r-pill);
    background: var(--card);
    color: var(--ink-2);
    display: grid;
    place-items: center;
  }
  /* 4 · THE EYEBROW'S LINE. `Not this` is a --hit (32px) pill, so without
     this the eyebrow would stand 20px in the all-clear and 32px where the
     pill stands — and the title, the why and the acts would all drop 12px
     with the state. One 32px line in every state; the chip and the disc
     centre on it. */
  .xc-card .cd-focus__eyebrow {
    min-height: var(--hit);
  }

  /* The arc is 132px of instrument: when the card cannot hold it beside the
     identity it stands under them, on the card's own text line, instead of
     squeezing the title's measure to nothing. */
  @container (max-width: 560px) {
    .xc-card {
      grid-template-columns: minmax(0, 1fr);
      row-gap: var(--space-lg);
      justify-items: start;
    }
  }
</style>
