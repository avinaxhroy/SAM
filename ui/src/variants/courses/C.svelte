<!--
  Courses variant C: The Next Step.
  Highlights the most urgent course with a featured card, followed by row listings
  and a next-topic hero banner in the detail view.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { chaptersOf, percentOf, type CoursesProps, type CourseTopic } from './props';

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

  /** The hero's subject: the topic the plan's own row marks as next. */
  const next = $derived(opened ? (opened.topics.find((topic) => topic.id === opened.nextId) ?? null) : null);

  /** The featured course: nearest dated thing wins; ties and dateless keep order. */
  const featured = $derived.by(() => {
    let best = courses[0] ?? null;
    for (const course of courses) {
      if (course.days === null) continue;
      if (!best || best.days === null || course.days < best.days) best = course;
    }
    return best;
  });
  const rest = $derived(featured ? courses.filter((course) => course.id !== featured.id) : courses);

  /** A square's ink is measured from its own bottom: done full, step n/m, none empty. */
  function squareFill(topic: CourseTopic): number {
    return topic.rungs === 0 ? 0 : (topic.rung / topic.rungs) * 100;
  }
</script>

<div class="v-fit crsc">
  {#if opened}
    <div class="crsc-bar">
      <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={onClose}>
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

    <!-- The ink hero: this screen's one dark object, and the only place the
         course's next step is said before the list says it again in rows. -->
    <header class="crsc-hero">
      <span class="crsc-hero__top">
        {#if opened.code}
          <span class="cd-chip cd-chip--code crsc-chip-onink">{opened.code}</span>
        {/if}
        <span class="crsc-hero__label">Next up</span>
      </span>
      {#if next}
        <h1 class="crsc-hero__t">{next.label}</h1>
        <p class="crsc-hero__meta num">
          {next.stage}{next.minutes === null ? '' : ` · ${next.minutes} min`}
        </p>
        <span class="cd-pill crsc-onink crsc-hero__act" aria-hidden="true">Continue</span>
      {:else}
        <h1 class="crsc-hero__t">{opened.name}</h1>
        <p class="crsc-hero__meta">
          {opened.started === opened.total && opened.total > 0
            ? 'Everything here has been started'
            : 'Nothing is planned under this course yet'}
        </p>
      {/if}
    </header>

    <div class="crsc-chaps">
      {#each chapters as chapter (chapter.label)}
        <section>
          <h2 class="crsc-chaplabel">{chapter.label}</h2>
          <div class="cd-card crsc-topics">
            {#each chapter.topics as topic (topic.id)}
              <button
                class="crsc-topic"
                type="button"
                data-next={topic.id === opened.nextId ? '1' : undefined}
                data-command="record.panel"
                data-placement="recordTable.rowContext"
                aria-label={`${topic.label} — ${topic.stage}`}
                onclick={() => onTopic(topic.id)}
              >
                <span class="crsc-sq" data-s={topic.state} aria-hidden="true">
                  {#if topic.state === 'done'}
                    <svg width="16" height="16" viewBox="0 0 16 16" focusable="false"><path d="M4.6 8.5 7 10.9 11.4 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
                  {:else if topic.state === 'step'}
                    <i style={`--v: ${squareFill(topic)}%`}></i>
                  {/if}
                </span>
                <span class="crsc-topic__t">{topic.label}</span>
                <span class="crsc-topic__m num">{topic.minutes === null ? '' : `${topic.minutes} min`}</span>
              </button>
            {/each}
          </div>
        </section>
      {/each}
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

    {#if featured}
      <!-- The card is the button; “Continue” is a span inside it, so the card
           never nests a control and the whole surface stays the door. -->
      <button class="crsc-feat" type="button" data-w={featured.wash} onclick={() => onOpen(featured.id)}>
        <span class="crsc-feat__body">
          <span class="crsc-feat__top">
            {#if featured.code}
              <span class="cd-chip cd-chip--code cd-chip--onwash">{featured.code}</span>
            {/if}
            <span class="crsc-feat__n">{featured.name}</span>
          </span>
          <span class="crsc-feat__fig num">{featured.started} of {featured.total} started</span>
        </span>
        <span class="cd-pill crsc-continue" aria-hidden="true">Continue</span>
        <span class="cd-sr">{percentOf(featured.started, featured.total)} per cent started</span>
      </button>
    {/if}

    {#if rest.length > 0}
      <div class="cd-card crsc-plain">
        {#each rest as course (course.id)}
          <button class="crsc-row" type="button" onclick={() => onOpen(course.id)}>
            {#if course.code}
              <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
            {/if}
            <span class="crsc-row__n">{course.name}</span>
            <svg class="crsc-chev" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5"/></svg>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  /* Overview — the featured card is the screen's one washed surface. */
  .crsc-feat {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-lg);
    width: 100%;
    min-height: calc(128px * var(--ui-s));
    padding: var(--space-lg) var(--space-xl);
    text-align: left;
    background-color: var(--wash);
    background-image: var(--wash-grad);
    color: var(--onwash);
    border-radius: var(--r-card);
    box-shadow: var(--sh-2);
    transition: box-shadow var(--dur-2) var(--ease);
  }
  .crsc-feat:hover {
    box-shadow: var(--sh-3), var(--catch);
  }
  .crsc-feat:active {
    box-shadow: var(--sh-2);
  }
  .crsc-feat:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  @container (max-width: 520px) {
    .crsc-feat {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .crsc-feat__body {
    display: grid;
    gap: var(--space-xs);
    min-width: 0;
  }
  .crsc-feat__top {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
    flex-wrap: wrap;
  }
  .crsc-feat__n {
    font-size: calc(var(--text-xl) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.1;
  }
  .crsc-feat__fig {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--onwash);
    letter-spacing: var(--track-title);
  }
  /* The card's one action, as the system's on-wash recipe — never plain white.
     A span: the card is the button, so no control nests inside it. */
  .crsc-continue {
    flex: none;
    background: var(--pane);
    color: var(--onwash);
    height: var(--pill-h);
    padding: 0 var(--space-lg);
  }

  .cd-card.crsc-plain {
    padding: 0;
    overflow: clip;
    margin-top: var(--space-lg);
  }
  .crsc-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 16px;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(56px * var(--ui-s));
    padding: 0 var(--space-lg);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .crsc-row + .crsc-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .crsc-row:hover {
    background: var(--well);
  }
  .crsc-row:active {
    background: var(--well-2);
  }
  .crsc-row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .crsc-row__n {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  .crsc-chev {
    color: var(--ink-3);
    transition: transform var(--dur-1) var(--ease);
  }
  .crsc-row:hover .crsc-chev {
    transform: translateX(2px);
  }

  /* Detail — the ink hero. */
  .crsc-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
  }
  .crsc-hero {
    display: grid;
    gap: var(--space-sm);
    margin-top: var(--space-xl);
    padding: var(--space-xl);
    background: var(--ink);
    color: var(--ink-inv);
    border-radius: var(--r-card);
  }
  .crsc-hero__top {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .crsc-chip-onink {
    background: var(--fill-on-ink);
    background-image: none;
    color: var(--ink-inv);
  }
  .crsc-hero__label {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-inv);
    opacity: 0.72;
  }
  .crsc-hero__t {
    font-size: calc(var(--text-xl) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.1;
    margin: 0;
  }
  .crsc-hero__meta {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-inv);
    opacity: 0.72;
    letter-spacing: var(--track-title);
  }
  .crsc-hero__act {
    justify-self: start;
    margin-top: var(--space-xs);
  }
  .crsc-onink {
    background: var(--fill-on-ink);
    color: var(--ink-inv);
    height: var(--pill-h);
    padding: 0 var(--space-lg);
  }

  .crsc-chaps {
    display: grid;
    gap: var(--space-xl);
    margin-top: var(--space-2xl);
  }
  .crsc-chaplabel {
    margin: 0 0 var(--space-xs);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .cd-card.crsc-topics {
    padding: 0;
    overflow: clip;
  }
  .crsc-topic {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(56px * var(--ui-s));
    padding: 0 var(--space-lg);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .crsc-topic + .crsc-topic {
    border-top: 1px dashed var(--rule-strong);
  }
  .crsc-topic:hover {
    background: var(--well);
  }
  .crsc-topic:active {
    background: var(--well-2);
  }
  .crsc-topic:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* All rows equal weight — the hero already carried “next”. */
  .crsc-topic__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }
  .crsc-topic__m {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: right;
  }

  /* The square family: ink measured from its own bottom. */
  .crsc-sq {
    position: relative;
    display: block;
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: var(--r-key);
  }
  .crsc-sq[data-s='done'] {
    background: var(--ink);
    color: var(--ink-inv);
    display: grid;
    place-items: center;
  }
  .crsc-sq[data-s='step'] {
    background: var(--card);
    box-shadow: inset 0 0 0 1px var(--rule);
    overflow: hidden;
  }
  .crsc-sq[data-s='step'] i {
    position: absolute;
    inset: auto 0 0 0;
    display: block;
    height: var(--v, 0%);
    background: var(--ink);
  }
  .crsc-sq[data-s='none'] {
    box-shadow: inset 0 0 0 1.5px var(--rule-strong);
  }

  @media (prefers-reduced-motion: reduce) {
    .crsc-feat,
    .crsc-row,
    .crsc-chev,
    .crsc-topic {
      transition: none;
    }
  }
</style>
