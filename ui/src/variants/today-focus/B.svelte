<!-- Today focus variant B: The Card That Opens. Expandable rationale and duration editor. -->
<script module lang="ts">
  /** One id per mounted card, so `aria-controls` never points at a sibling. */
  let panelSeq = 0;
</script>

<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import Duration from './Duration.svelte';
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
  }: TodayFocusProps = $props();

  let open = $state(false);
  const panelId = `today-focus-panel-${++panelSeq}`;
  let head = $state<HTMLButtonElement | null>(null);
  let panel = $state<HTMLDivElement | null>(null);

  function toggle(): void {
    const next = !open;
    open = next;
    if (!next && panel?.contains(document.activeElement)) head?.focus();
  }

  // A new state closes the card: a disclosure never inherits another's height.
  $effect(() => {
    void mode;
    open = false;
  });
</script>

<div class="v-fit tfb">
  <header class="tf-head">
    <h1 class="tf-greet">{greeting}</h1>
    <p class="tf-date">{dateLine}</p>
  </header>

  {#if mode === 'clear'}
    <div class="tf-plank xb-plank">
      <div class="tf-id">
        <p class="tf-eyebrow">
          <span class="tf-mark" aria-hidden="true"><Icon name="check" size={13} /></span>
        </p>
        <h2 class="tf-title tw-focus__text">Nothing waiting</h2>
        <p class="tf-why">
          <span class="tf-why__k">Studied today</span>
          <span><b>{studiedToday}</b></span>
        </p>
      </div>
      <div class="xb-acts">
        <div class="tf-acts">
          <button class="cd-pill cd-pill--lg cd-pill--quiet" type="button" onclick={onPlan}>Plan a session</button>
        </div>
      </div>
    </div>
  {:else}
    <div class="cd-card xb-card" data-open={open ? '1' : '0'}>
      <div class="xb-head">
        <div class="tf-id">
          <p class="tf-eyebrow">
            {#if course}
              <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.label}</span>
            {/if}
            {#if tag}
              <span class="tf-tag">{tag}</span>
            {/if}
          </p>
          <h2 class="tf-title tw-focus__text" title={title ?? undefined}>{title}</h2>
        </div>
        <span class="xb-plus" aria-hidden="true"><i></i><i></i></span>
        <button
          class="xb-open"
          type="button"
          bind:this={head}
          aria-expanded={open}
          aria-controls={panelId}
          aria-label={why ? `Why this: ${why.key}` : 'Why this card'}
          onclick={toggle}
        ></button>
      </div>

      <div class="xb-panel" id={panelId} bind:this={panel} inert={!open}>
        <div class="xb-panel__in">
          <div class="xb-panel__pad">
            {#if why}
              <p class="tf-why">
                <span class="tf-why__k">{why.key}</span>
                <span><b>{why.value}</b></span>
              </p>
            {/if}
          </div>
        </div>
      </div>

      <div class="xb-acts">
        <div class="tf-acts">
          <button
            class="cd-pill cd-pill--lg"
            type="button"
            data-command={sessionCommand ?? undefined}
            data-placement={sessionCommand ? 'today.screen' : undefined}
            disabled={!sessionCommand}
            title={sessionCommand ? 'Start a session on this' : 'This plan has no kind that records minutes'}
            onclick={onStart}
          >
            <Icon name="play" size={15} />
            Start
          </button>
          <Duration {minutes} onCommit={onMinutes} />
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* The variant's root stack, the lab's own (`.tf-screen`): the greeting sits
     one `--space-xl` above the object it introduces, the same rhythm C carries.
     Without it the port drew the greeting as a line touching the card. */
  .tfb {
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
  .tf-id {
    display: grid;
    align-content: start;
    justify-items: start;
    gap: var(--ui-gap);
  }
  .tf-eyebrow {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin: 0;
  }
  .tf-tag {
    font-size: var(--text-2xs);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .tf-title {
    margin: 0;
    font-size: calc(var(--text-3xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.08;
  }
  .tf-title.tw-focus__text {
    padding-block: 0.14em;
    margin-block: -0.14em;
  }
  .tf-why {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
    margin: 0;
    font-size: calc(var(--text-sm) * var(--ui-s));
    color: var(--ink-2);
  }
  .tf-why__k {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .tf-why b {
    color: var(--ink);
    font-weight: var(--weight-label);
    font-variant-numeric: tabular-nums;
  }
  .tf-acts {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-height: calc(var(--pill-h-lg) + var(--space-md));
  }

  /* ── the all-clear plank ────────────────────────────────────────────── */
  .tf-plank {
    display: grid;
    align-content: start;
    gap: var(--ui-gap);
    padding: var(--space-xl) var(--space-2xl);
    background: var(--well);
    border-radius: var(--r-card);
  }
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
  .tf-plank :global(.cd-pill--quiet) {
    background: var(--card);
  }
  .tf-plank :global(.cd-pill--quiet:hover) {
    background: var(--card);
    box-shadow: var(--sh-1);
  }

  /* ── B · the card, and the row that opens it ────────────────────────── */
  .xb-card {
    padding: 0;
    overflow: clip;
  }
  .xb-head {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-lg);
    padding: var(--space-xl) var(--space-2xl) var(--space-md);
    transition: background var(--dur-1) var(--ease);
  }
  /* the opener: the row, and nothing but the row */
  .xb-open {
    position: absolute;
    inset: 0;
    border-radius: 0;
  }
  .xb-head:hover {
    background: var(--well);
  }
  .xb-head:active {
    background: var(--well-2);
  }
  .xb-open:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -3px;
  }
  .xb-plus {
    position: relative;
    z-index: 1;
    pointer-events: none;
    width: var(--hit);
    height: var(--hit);
    flex: none;
    border-radius: var(--r-pill);
    background: var(--well);
    color: var(--ink-2);
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .xb-plus i {
    position: absolute;
    inset: 0;
    margin: auto;
    width: 12px;
    height: 1.5px;
    border-radius: var(--r-pill);
    background: currentColor;
    transition:
      transform var(--dur-2) var(--ease),
      opacity var(--dur-2) var(--ease);
  }
  .xb-plus i:nth-child(2) {
    transform: rotate(90deg);
  }
  .xb-card[data-open='1'] .xb-plus i:nth-child(2) {
    transform: rotate(90deg) scaleX(0);
    opacity: 0;
  }
  .xb-head:hover .xb-plus {
    background: var(--well-2);
    color: var(--ink);
  }
  .xb-panel {
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    min-height: 0;
    transition:
      grid-template-rows var(--dur-3) var(--ease-pop),
      opacity var(--dur-2) var(--ease);
  }
  .xb-card[data-open='1'] .xb-panel {
    grid-template-rows: 1fr;
    opacity: 1;
  }
  .xb-panel__in {
    min-height: 0;
    overflow: hidden;
  }
  .xb-panel__pad {
    padding: 0 var(--space-2xl) var(--space-md);
  }
  .xb-acts {
    padding: var(--space-md) var(--space-2xl) var(--space-xl);
    border-top: 1px dashed var(--rule-strong);
  }
  /* the all-clear wears the same ribs — an identity row, a dashed rule, an
     action row — on the quiet --well surface instead of the card */
  .xb-plank {
    padding: 0;
    gap: 0;
    overflow: clip;
  }
  .xb-plank > .tf-id {
    padding: var(--space-xl) var(--space-2xl) var(--space-md);
  }

  @media (prefers-reduced-motion: reduce) {
    .xb-head {
      transition: none;
    }
  }

  @container (max-width: 520px) {
    .xb-head {
      grid-template-columns: minmax(0, 1fr);
      row-gap: var(--ui-gap-sm);
    }
    .xb-head .xb-plus {
      justify-self: start;
    }
  }
</style>
