<!--
  Courses variant B: The Shelf.
  Card grid layout with topic pip indicators and chapter-based sidebar navigation
  with circular progress glyphs.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { chaptersOf, type CourseCard, type CourseTopic, type CoursesProps } from './props';

  let {
    title,
    courses,
    summary,
    openedId,
    newCourseCommand,
    newTopicCommand,
    onOpen,
    onClose,
    onTopic,
    onNewCourse,
    onNewTopic,
  }: CoursesProps = $props();

  const opened = $derived(courses.find((course) => course.id === openedId) ?? null);
  const chapters = $derived(opened ? chaptersOf(opened) : []);

  /** The chapter whose rows are showing; resets to first chapter when open course changes. */
  let picked = $state<string | null>(null);
  const active = $derived(
    chapters.find((chapter) => chapter.label === picked) ?? chapters[0] ?? null,
  );

  /** Circle glyph circumference for r = 9 in a 28px slot. */
  const ARC = 2 * Math.PI * 9;

  /** A pip's ink: done is full, not-started is empty, a step is n/m of the way. */
  function fillOf(topic: CourseTopic): number {
    if (topic.state === 'done') return 100;
    if (topic.state === 'none') return 0;
    return topic.rungs === 0 ? 0 : (topic.rung / topic.rungs) * 100;
  }

  /** Where the row the plan says is next sits, for the glyph's target ring. */
  function isNext(course: CourseCard, topic: CourseTopic): boolean {
    return topic.id === course.nextId;
  }
</script>

<div class="v-fit crsb">
  {#if opened}
    <div class="crsb-bar">
      <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={() => { picked = null; onClose(); }}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 5.5 8.5 12 15 18.5"/></svg>
        {title}
      </button>
      {#if newTopicCommand}
        <button
          class="cd-pill cd-pill--quiet cd-pill--sm"
          type="button"
          data-command={newTopicCommand}
          data-placement="today.screen"
          onclick={onNewTopic}
        >
          New topic
        </button>
      {/if}
    </div>

    <header class="crsb-header">
      <div class="crsb-titlerow">
        {#if opened.code}
          <span class="cd-chip cd-chip--code" data-w={opened.wash}>{opened.code}</span>
        {/if}
        <h1 class="crsb-title">{opened.name}</h1>
      </div>
      <!-- The header strip is read off the rows beneath it, across every
           chapter — hidden ones included — so shape and rows cannot drift. A
           chapter boundary is a gap. -->
      <span class="crsb-strip crsb-strip--detail" aria-hidden="true">
        {#each chapters as chapter, chapterIndex (chapter.label)}
          {#each chapter.topics as topic, topicIndex (topic.id)}
            <i
              class="crsb-pip"
              class:crsb-pip--sep={chapterIndex > 0 && topicIndex === 0}
              data-s={topic.state}
              style={`--v: ${fillOf(topic)}%`}
            ></i>
          {/each}
        {/each}
      </span>
      <p class="cd-sr">{opened.started} of {opened.total} topics started</p>
    </header>

    <div class="crsb-body">
      <nav class="cd-card crsb-nav" aria-label="Chapters">
        <h2 class="crsb-nav__head">Chapters</h2>
        {#each chapters as chapter (chapter.label)}
          <button
            class="crsb-navrow"
            type="button"
            aria-current={active?.label === chapter.label ? 'true' : 'false'}
            onclick={() => (picked = chapter.label)}
          >
            <span class="crsb-navrow__n">{chapter.label}</span>
            <span class="crsb-navrow__c num">{chapter.topics.length} {chapter.topics.length === 1 ? 'topic' : 'topics'}</span>
          </button>
        {/each}
      </nav>

      {#if active}
        <div class="crsb-pane">
          <div class="cd-card crsb-topics">
            {#each active.topics as topic (topic.id)}
              <button
                class="crsb-topic"
                type="button"
                data-next={isNext(opened, topic) ? '1' : undefined}
                data-command="record.panel"
                data-placement="recordTable.rowContext"
                aria-label={`${topic.label} — ${topic.stage}`}
                onclick={() => onTopic(topic.id)}
              >
                <span class="crsb-glyph" data-s={topic.state}>
                  <svg width="28" height="28" viewBox="0 0 28 28" aria-hidden="true" focusable="false">
                    {#if isNext(opened, topic)}
                      <circle class="crsb-you" cx="14" cy="14" r="12.75" />
                    {/if}
                    <circle class="crsb-disc" cx="14" cy="14" r="9" />
                    {#if topic.state === 'step'}
                      <circle
                        class="crsb-arc"
                        cx="14"
                        cy="14"
                        r="9"
                        style={`--dash:${((topic.rung / (topic.rungs || 1)) * ARC).toFixed(2)} ${ARC.toFixed(2)}`}
                      />
                    {/if}
                    {#if topic.state === 'done'}
                      <path class="crsb-check" d="M10.2 14.4l2.4 2.4 5.2-5.4" />
                    {/if}
                  </svg>
                </span>
                <span class="crsb-topic__t">{topic.label}</span>
                <span class="crsb-topic__m num">{topic.minutes === null ? '' : `${topic.minutes} min`}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <header class="cd-pagehead">
      <div>
        <h1 class="cd-pagehead__title">{title}</h1>
        <p class="cd-pagehead__sub">
          {summary.courses} {summary.courses === 1 ? 'course' : 'courses'}
          {#if summary.topics > 0}
            · {summary.topics} {summary.topics === 1 ? 'topic' : 'topics'}
            {#if summary.started > 0}
              · {summary.started} started
            {/if}
          {/if}
        </p>
      </div>
      <span class="cd-pagehead__aside">
        {#if newCourseCommand}
          <button
            class="cd-pill"
            type="button"
            data-command={newCourseCommand}
            data-placement="today.screen"
            onclick={onNewCourse}
          >
            <Icon name="plus" size={13} />
            New course
          </button>
        {/if}
      </span>
    </header>

    <div class="crsb-grid">
      {#each courses as course (course.id)}
        <button class="crsb-tile" type="button" data-w={course.wash} onclick={() => onOpen(course.id)}>
          <span class="crsb-tile__top">
            {#if course.code}
              <span class="cd-chip cd-chip--code cd-chip--onwash">{course.code}</span>
            {/if}
            <span class="crsb-tile__n">{course.name}</span>
          </span>
          <span class="crsb-strip crsb-strip--list" aria-hidden="true">
            {#each course.topics as topic (topic.id)}
              <i class="crsb-pip" data-s={topic.state} style={`--v: ${fillOf(topic)}%`}></i>
            {/each}
          </span>
          <span class="cd-sr">{course.started} of {course.total} topics started</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Overview: the shelf. Two columns at a comfortable width, one under it —
     the reflow is the container's, so a narrow canvas gives narrow tiles. */
  .crsb-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-lg);
  }
  @container (max-width: 620px) {
    .crsb-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .crsb-tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-sm);
    min-height: calc(132px * var(--ui-s));
    padding: var(--space-lg);
    background-color: var(--wash);
    background-image: var(--wash-grad);
    color: var(--onwash);
    border-radius: var(--r-card);
    box-shadow: var(--sh-1);
    text-align: left;
    transition: transform var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease);
  }
  .crsb-tile:hover {
    transform: translateY(-2px);
    box-shadow: var(--sh-2);
  }
  .crsb-tile:active {
    transform: none;
    box-shadow: var(--sh-1);
  }
  .crsb-tile:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .crsb-tile__top {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-xs);
  }
  .crsb-tile__n {
    font-size: calc(var(--text-lg) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.15;
  }

  /* The pip strip: one pip per topic in plan order; a pip's ink runs from its
     own bottom to that topic's rung. Never animated — it is data. */
  .crsb-strip {
    display: inline-flex;
    align-items: center;
    gap: var(--pip-gap, 3px);
  }
  .crsb-strip--list {
    --pip-w: 6px;
    --pip-h: 16px;
    --pip-gap: 3.5px;
  }
  .crsb-strip--detail {
    --pip-w: 8px;
    --pip-h: 22px;
    --pip-gap: 4px;
  }
  .crsb-pip {
    position: relative;
    flex: none;
    width: var(--pip-w);
    height: var(--pip-h);
    border-radius: var(--r-pill);
    background: var(--well-2);
    overflow: hidden;
  }
  .crsb-pip::after {
    content: '';
    position: absolute;
    inset: auto 0 0 0;
    height: var(--v, 0%);
    border-radius: var(--r-pill);
    background: var(--ink);
  }
  /* On a washed tile the strip takes the tile's own on-wash ink; the empty pip
     is that ink at 22%, a translucent well rather than a border. */
  .crsb-strip--list .crsb-pip {
    background: color-mix(in oklab, var(--onwash) 22%, transparent);
  }
  .crsb-strip--list .crsb-pip,
  .crsb-strip--list .crsb-pip::after {
    border-radius: var(--r-key);
  }
  .crsb-strip--list .crsb-pip::after {
    background: var(--onwash);
  }
  .crsb-pip--sep {
    margin-left: var(--space-sm);
  }

  /* Detail */
  .crsb-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
  }
  .crsb-header {
    display: grid;
    justify-items: start;
    gap: var(--space-sm);
    margin-top: var(--space-xl);
  }
  .crsb-titlerow {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .crsb-title {
    font-size: calc(var(--text-3xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.03;
    margin: 0;
  }
  .crsb-body {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
    gap: var(--space-xl);
    align-items: start;
    margin-top: var(--space-2xl);
  }
  @container (max-width: 720px) {
    .crsb-body {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .cd-card.crsb-nav {
    padding: var(--space-md);
  }
  .crsb-nav__head {
    margin: var(--space-xs) var(--space-xs) var(--space-sm);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .crsb-navrow {
    display: grid;
    gap: 2px;
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    text-align: left;
    border-radius: var(--r-item);
    transition: background var(--dur-1) var(--ease);
  }
  .crsb-navrow:hover {
    background: var(--well);
  }
  .crsb-navrow:active {
    background: var(--well-2);
  }
  .crsb-navrow:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .crsb-navrow[aria-current='true'] {
    background: var(--well);
  }
  .crsb-navrow__n {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }
  .crsb-navrow[aria-current='true'] .crsb-navrow__n {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .crsb-navrow__c {
    font-size: var(--text-2xs);
    color: var(--ink-3);
  }
  .crsb-pane {
    animation: crsb-fade var(--dur-2) var(--ease) both;
  }
  @keyframes crsb-fade {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .cd-card.crsb-topics {
    padding: 0;
    overflow: clip;
  }
  .crsb-topic {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(64px * var(--ui-s));
    padding: 0 var(--space-lg);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .crsb-topic + .crsb-topic {
    border-top: 1px dashed var(--rule-strong);
  }
  .crsb-topic:hover {
    background: var(--well);
  }
  .crsb-topic:active {
    background: var(--well-2);
  }
  .crsb-topic:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .crsb-topic__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }
  .crsb-topic[data-next] .crsb-topic__t {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .crsb-topic__m {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: right;
  }

  /* The circle glyph — a 28 px slot: a 20 px disc (r = 9) plus the “you are
     here” ring 2 px outside it. A step is an ink arc of n/m. */
  .crsb-glyph {
    display: grid;
    place-items: center;
  }
  .crsb-glyph svg {
    color: var(--ink);
  }
  .crsb-disc {
    fill: none;
    stroke: var(--rule-strong);
    stroke-width: 1.5;
  }
  .crsb-glyph[data-s='done'] .crsb-disc {
    fill: var(--ink);
    stroke: none;
  }
  .crsb-glyph[data-s='step'] .crsb-disc {
    stroke: var(--ink-3);
  }
  .crsb-arc {
    fill: none;
    stroke: var(--ink);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-dasharray: var(--dash);
    transform: rotate(-90deg);
    transform-origin: 14px 14px;
  }
  .crsb-check {
    fill: none;
    stroke: var(--ink-inv);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .crsb-you {
    fill: none;
    stroke: var(--ink);
    stroke-width: 2;
  }

  @media (prefers-reduced-motion: reduce) {
    .crsb-tile,
    .crsb-navrow,
    .crsb-topic {
      transition: none;
    }
    .crsb-pane {
      animation: none;
    }
  }
</style>
