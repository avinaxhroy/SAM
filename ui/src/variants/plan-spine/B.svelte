<!--
  PLAN SPINE · B · THE WORKSPACE.
  Two-pane workspace variant with an interactive phase/week navigation rail on
  the left and an active weekly topic canvas on the right.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { readOf, type PlanProps, type PlanWeek } from './props';

  let {
    title,
    weeks,
    summary,
    selectedId,
    newTopicCommand,
    onSelect,
    onTopic,
    onNewTopic,
  }: PlanProps = $props();

  /** One phase of the term: the label the plan declared, and the weeks that
      carry it, in the plan's own order. */
  type Phase = { label: string; weeks: PlanWeek[] };

  /** Weeks the plan never phased still need a home, and the rail says so. */
  const NO_PHASE = 'No phase set';

  /** The term by phase, in the order the phases first appear — the plan's own
      order read once, which is also why the group labels are unique keys. */
  const phases = $derived.by((): Phase[] => {
    const groups: Phase[] = [];
    const seen = new Map<string, Phase>();
    for (const week of weeks) {
      const label = week.phase ?? NO_PHASE;
      let group = seen.get(label);
      if (!group) {
        group = { label, weeks: [] };
        seen.set(label, group);
        groups.push(group);
      }
      group.weeks.push(week);
    }
    return groups;
  });

  /** The week on the canvas: what the panel selected, else the term's first.
      The design never keeps a selection of its own. */
  const openWeek = $derived(weeks.find((week) => week.id === selectedId) ?? weeks[0]);

  /** The canvas head's one meta run — its dates and its phase, whichever the
      plan actually declared. */
  const openMeta = $derived([openWeek.dates, openWeek.phase].filter(Boolean).join(' · '));

  let rail = $state<HTMLElement | null>(null);
  /** Arrival happens once: the rail is centred on the week you are in and then
      left wherever the student puts it. */
  let arrived = false;

  /**
   * Bring one rail row into the rail's OWN scroll, never the document's:
   * `scrollIntoView` climbs every scrollable ancestor and threw the whole page
   * down on load (the lab documents that as the bug it fixed), so the move is
   * written by hand against two rects. `force` centres the row even when it is
   * already visible — that is arrival, the lab's own centring — where a step
   * only moves the rail when the row it opened is out of view.
   */
  function centre(row: HTMLElement, force = false): void {
    const element = rail;
    if (!element) return;
    const rowRect = row.getBoundingClientRect();
    const railRect = element.getBoundingClientRect();
    if (!force && rowRect.top >= railRect.top && rowRect.bottom <= railRect.bottom) return;
    element.scrollTop += rowRect.top - railRect.top - (element.clientHeight - rowRect.height) / 2;
  }

  $effect(() => {
    const element = rail;
    if (arrived || !element) return;
    arrived = true;
    const row = element.querySelector<HTMLElement>('.psb-week[data-now]');
    if (row) centre(row, true);
  });

  /**
   * ← / → step the term — the lab's own keyboard for B (`openB(open ± dir,
   * true)`): the next week opens on the canvas, the rail follows it, and the
   * row that opened it takes focus, so the keyboard user walks the term rather
   * than a list of buttons. The selection stays the panel's: this only asks.
   */
  function stepWeek(event: KeyboardEvent): void {
    const dir = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    if (dir === 0 || weeks.length === 0) return;
    event.preventDefault();
    const from = Math.max(
      0,
      weeks.findIndex((week) => week.id === openWeek.id),
    );
    const week = weeks[Math.max(0, Math.min(weeks.length - 1, from + dir))];
    if (week.id !== openWeek.id) onSelect(week.id);
    const row = rail?.querySelector<HTMLElement>(`.psb-week[data-week="${week.id}"]`);
    if (!row) return;
    row.focus({ preventScroll: true });
    centre(row);
  }
</script>

<div class="v-fit psb">
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub num">
        {summary.weeks} weeks · {summary.things} things to get through · {summary.done} of {summary.things} done ·
        {summary.unplaced} not placed in a week
      </p>
    </div>
  </header>

  <div class="psb-ws">
    <!-- The rail: the term in phases, and the pane that navigates. -->
    <nav class="psb-rail" bind:this={rail} tabindex="0" aria-label="The term by phase" onkeydown={stepWeek}>
      <div class="psb-nav">
        {#each phases as phase (phase.label)}
          <section class="psb-phase">
            <div class="psb-phase__head">
              <h3 class="psb-phase__t">{phase.label}</h3>
              <span class="psb-phase__n num">
                {phase.weeks.length} {phase.weeks.length === 1 ? 'week' : 'weeks'}
              </span>
            </div>
            <div class="psb-weeks">
              {#each phase.weeks as week (week.id)}
                <button
                  class="psb-week"
                  type="button"
                  data-week={week.id}
                  data-on={week.id === selectedId ? '' : undefined}
                  data-now={week.now ? '' : undefined}
                  aria-current={week.now ? 'true' : undefined}
                  onclick={() => onSelect(week.id)}
                >
                  <span class="psb-week__n num">W{week.index}</span>
                  {#if week.dates}<span class="psb-week__d num">{week.dates}</span>{/if}
                </button>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    </nav>

    <!-- The canvas: the week you are reading, and nothing else. -->
    <div
      class="psb-canvas"
      tabindex="0"
      role="region"
      aria-label="The open week"
      onkeydown={stepWeek}
    >
      {#key openWeek.id}
        <div class="psb-canvas__in">
          <header class="psb-chead">
            <h2 class="psb-chead__t">{openWeek.name}</h2>
            {#if openMeta}<p class="psb-chead__s num">{openMeta}</p>{/if}
            <p class="psb-chead__fig num">{readOf(openWeek)}</p>
          </header>

          {#if openWeek.topics.length === 0}
            <p class="psb-void">No topics in this week.</p>
          {:else}
            <div class="psb-topics">
              {#each openWeek.topics as topic (topic.id)}
                <button
                  class="psb-topic"
                  type="button"
                  data-done={topic.state === 'done' ? '1' : undefined}
                  data-command="record.panel"
                  data-placement="recordTable.rowContext"
                  onclick={() => onTopic(topic)}
                >
                  <span class="psb-topic__body">
                    <span class="psb-topic__t">{topic.label}</span>
                    {#if topic.kind}<span class="psb-topic__note">{topic.kind}</span>{/if}
                  </span>
                  <span class="psb-badge" data-s={topic.state}>{topic.stage}</span>
                  {#if topic.minutes !== null}
                    <span class="psb-topic__m num">{topic.minutes} min</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}

          {#if newTopicCommand}
            <button
              class="psb-write"
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
      {/key}
    </div>
  </div>
</div>

<style>
  /* ── the frame: two panes, each scrolling itself ───────────────────────── */
  .psb-ws {
    display: flex;
    height: calc(620px * var(--ui-s));
  }

  /* ── the rail ──────────────────────────────────────────────────────────── */
  .psb-rail {
    flex: 0 0 calc(232px * var(--ui-s));
    padding: calc(12px * var(--ui-s));
    background: var(--well);
    box-shadow: inset -1px 0 0 var(--rule);
    overflow-x: hidden;
    overflow-y: auto;
  }
  .psb-rail:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .psb-nav {
    display: grid;
    gap: calc(12px * var(--ui-s));
  }
  .psb-phase {
    display: grid;
    gap: var(--ui-gap-sm);
  }
  /* The rail has ONE line: its own 12px padding. The phase's label, its count
     and the week rows' boxes all sit on it, and a week row's text sits one
     declared 8 inside its own box (the row's padding). */
  .psb-phase__head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--ui-gap-sm);
    padding: 0 var(--ui-gap-sm);
  }
  .psb-phase__t {
    margin: 0;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .psb-phase__n {
    font-size: var(--ui-meta);
    letter-spacing: var(--track-title);
    color: var(--ink-3);
  }
  .psb-weeks {
    display: grid;
    gap: calc(2px * var(--ui-s));
  }
  .psb-week {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: 100%;
    height: calc(32px * var(--ui-s));
    min-height: var(--hit);
    padding: 0 var(--ui-gap-sm);
    border: 0;
    border-radius: var(--r-item);
    background: none;
    font: inherit;
    text-align: left;
    color: var(--ink-2);
    cursor: pointer;
    transition:
      background var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .psb-week:hover {
    background: var(--well-2);
    color: var(--ink);
  }
  .psb-week:active {
    background: var(--well-2);
  }
  .psb-week:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* The open week is the raised row. It needs its own hover: the fill rule
     above out-ranks `.psb-week:hover`, so without this the row you are reading
     would be the one row in the rail that answered the pointer with nothing. */
  .psb-week[data-on] {
    background: var(--card);
    box-shadow: var(--sh-1);
    color: var(--ink);
  }
  .psb-week[data-on]:hover {
    background: var(--card);
    box-shadow: var(--sh-2);
  }
  .psb-week__n {
    flex: none;
    display: grid;
    place-items: center;
    min-width: calc(26px * var(--ui-s));
    height: calc(22px * var(--ui-s));
    border-radius: var(--r-key);
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  /* The week you are in: the rail's one ink object, at any position. */
  .psb-week[data-now] .psb-week__n {
    background: var(--ink);
    color: var(--ink-inv);
  }
  .psb-week__d {
    font-size: var(--ui-meta);
    letter-spacing: var(--track-title);
    color: inherit;
    white-space: nowrap;
  }

  /* ── the canvas ────────────────────────────────────────────────────────── */
  .psb-canvas {
    flex: 1;
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
  }
  .psb-canvas:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .psb-canvas__in {
    /* The canvas's own line (24) and how far an inner row may reach toward the
       edge — the write row and the void line bleed to it and pad back. */
    --psb-line: calc(24px * var(--ui-s));
    max-width: calc(640px * var(--ui-s));
    margin: 0 auto;
    padding: calc(24px * var(--ui-s)) var(--psb-line) calc(32px * var(--ui-s));
    /* The open week arrives: `--dur-2` on a fade and a declared 8 of lift, so
       the swap of the pane is a movement and not a flash of new text. */
    animation: psb-arrive var(--dur-2) var(--ease);
  }
  @keyframes psb-arrive {
    from {
      opacity: 0;
      transform: translateY(calc(8px * var(--ui-s)));
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  .psb-chead {
    margin-bottom: calc(20px * var(--ui-s));
  }
  .psb-chead__t {
    margin: 0;
    font-size: calc(var(--text-2xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.05;
  }
  .psb-chead__s {
    margin-top: calc(4px * var(--ui-s));
    font-size: var(--ui-text);
    color: var(--ink-3);
  }
  .psb-chead__fig {
    margin-top: var(--ui-gap-sm);
    font-size: var(--ui-text);
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }

  /* ── the topic rows: the reference's pipeline item ─────────────────────── */
  .psb-topics {
    display: grid;
    gap: var(--ui-gap-sm);
  }
  .psb-topic {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--ui-pad);
    width: 100%;
    min-width: 0;
    min-height: calc(56px * var(--ui-s));
    padding: var(--ui-gap-sm);
    border: 0;
    border-radius: var(--r-item);
    background: var(--card);
    box-shadow: var(--sh-1);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      box-shadow var(--dur-2) var(--ease),
      background var(--dur-1) var(--ease);
  }
  .psb-topic:hover {
    box-shadow: var(--sh-2);
  }
  .psb-topic:active {
    background: var(--well);
    box-shadow: var(--sh-1);
  }
  .psb-topic:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .psb-topic__body {
    display: grid;
    gap: calc(2px * var(--ui-s));
    min-width: 0;
  }
  .psb-topic__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    white-space: normal;
  }
  /* A done topic's name steps up to `--ink` and label weight — the collection's
     own reading of done. */
  .psb-topic[data-done] .psb-topic__t {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .psb-topic__note {
    font-size: var(--ui-meta);
    color: var(--ink-3);
  }
  /* The state badge speaks the system's own chip wash with its `--on-*` ink. */
  .psb-badge {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: var(--chip-h);
    padding: 0 var(--ui-gap-sm);
    border-radius: var(--r-pill);
    background: var(--well-2);
    color: var(--ink-2);
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    white-space: nowrap;
  }
  .psb-badge[data-s='done'] {
    background: var(--chip-ok);
    color: var(--on-ok);
  }
  .psb-badge[data-s='step'] {
    background: var(--chip-info);
    color: var(--on-info);
  }
  .psb-topic__m {
    font-size: var(--ui-text);
    color: var(--ink-3);
    white-space: nowrap;
  }

  /* ── the one write, at the foot of the thing it writes into ────────────── */
  .psb-write {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: calc(100% + var(--psb-line) * 2);
    min-height: var(--hit);
    margin-top: var(--ui-gap-sm);
    margin-inline: calc(var(--psb-line) * -1);
    padding: calc(4px * var(--ui-s)) var(--psb-line);
    border: 0;
    border-radius: var(--r-item);
    background: none;
    font: inherit;
    font-size: var(--ui-text);
    text-align: left;
    color: var(--ink-3);
    cursor: pointer;
    transition:
      color var(--dur-1) var(--ease),
      background var(--dur-1) var(--ease);
  }
  .psb-write:hover {
    color: var(--ink);
    background: var(--well);
  }
  .psb-write:active {
    background: var(--well-2);
  }
  .psb-write:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }

  /* A state line where a list would be: a week with nothing in it. */
  .psb-void {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
    width: calc(100% + var(--psb-line) * 2);
    margin: 0 calc(var(--psb-line) * -1);
    padding: var(--ui-pad) var(--psb-line);
    font-size: var(--ui-text);
    color: var(--ink-3);
  }

  /* ── the lab's own 760px probe, on this component's container ───────────
     The rail becomes one horizontal run of week chips above the canvas: the
     phase heads are put away, the phase sections become `display: contents`
     so their weeks join the single run, and the canvas keeps its own scroll —
     which is why it needs `min-height: 0` in a column (a flex item's own
     `min-height: auto` would push the pane past its bounded frame instead of
     scrolling inside it). */
  @container (max-width: 760px) {
    .psb-ws {
      flex-direction: column;
      height: calc(560px * var(--ui-s));
    }
    .psb-rail {
      flex: none;
      display: block;
      padding: calc(4px * var(--ui-s)) calc(12px * var(--ui-s));
      box-shadow: inset 0 -1px 0 var(--rule);
      overflow-x: auto;
      overflow-y: hidden;
    }
    .psb-nav {
      display: flex;
      align-items: center;
      gap: calc(4px * var(--ui-s));
    }
    .psb-phase {
      display: contents;
    }
    .psb-phase__head {
      display: none;
    }
    .psb-weeks {
      display: flex;
      gap: calc(4px * var(--ui-s));
    }
    .psb-week {
      flex: none;
      width: auto;
      padding: 0 calc(12px * var(--ui-s));
      white-space: nowrap;
    }
    .psb-canvas {
      min-height: 0;
    }
    .psb-canvas__in {
      --psb-line: var(--ui-pad);
      padding: calc(20px * var(--ui-s)) var(--psb-line) calc(32px * var(--ui-s));
    }
  }

  /* `base.css` already collapses every animation and transition to 1ms under
     reduced motion; this states the same intent where the drawing reads it. */
  @media (prefers-reduced-motion: reduce) {
    .psb-week,
    .psb-topic,
    .psb-write {
      transition: none;
    }
    .psb-canvas__in {
      animation: none;
    }
  }
</style>
