<!--
  Courses variant A: The Index.
  Text-forward layout using ruled rows with course codes, names, progress counts,
  and a linear progress meter in the detail view.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { chaptersOf, percentOf, type CoursesProps } from './props';

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
</script>

<div class="v-fit crsa">
  {#if opened}
    <div class="crsa-bar">
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

    <header class="crsa-title">
      <div class="crsa-titlerow">
        {#if opened.code}
          <span class="cd-chip cd-chip--code" data-w={opened.wash}>{opened.code}</span>
        {/if}
        <h1 class="crsa-h1">{opened.name}</h1>
      </div>
      <!-- One line: a 3 px meter with its own figure at the right end, written
           from the rows below — the title block can never disagree with them. -->
      <div class="crsa-meterline">
        <span class="crsa-meter" aria-hidden="true">
          <i style={`--v: ${percentOf(opened.started, opened.total)}%`}></i>
        </span>
        <span class="crsa-meterline__fig num">{opened.started} of {opened.total} started</span>
      </div>
    </header>

    <div class="crsa-chaps">
      {#each chapters as chapter (chapter.label)}
        <section>
          <h2 class="crsa-chaplabel">{chapter.label}</h2>
          <div class="cd-card crsa-topics">
            {#each chapter.topics as topic (topic.id)}
              <button
                class="crsa-topic"
                type="button"
                data-next={topic.id === opened.nextId ? '1' : undefined}
                data-command="record.panel"
                data-placement="recordTable.rowContext"
                aria-label={`${topic.label} — ${topic.stage}`}
                onclick={() => onTopic(topic.id)}
              >
                <span class="crsa-tick" data-s={topic.state} aria-hidden="true"><i></i></span>
                <span class="crsa-topic__t">{topic.label}</span>
                <span class="crsa-topic__m num">{topic.minutes === null ? '' : `${topic.minutes} min`}</span>
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

    <div class="cd-card crsa-index">
      {#each courses as course (course.id)}
        <button class="crsa-row" type="button" onclick={() => onOpen(course.id)}>
          {#if course.code}
            <span class="cd-chip cd-chip--code" data-w={course.wash}>{course.code}</span>
          {/if}
          <span class="crsa-row__n">{course.name}</span>
          <span class="crsa-row__fig num">{course.started} of {course.total}</span>
          <svg class="crsa-chev" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5"/></svg>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Overview: one card of ruled rows. The card clips so the row's hover wash
     runs to its corner, which is why every focus ring here is inset. */
  .cd-card.crsa-index {
    padding: 0;
    overflow: clip;
  }
  .crsa-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto 16px;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(56px * var(--ui-s));
    padding: 0 var(--space-lg);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .crsa-row + .crsa-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .crsa-row:hover {
    background: var(--well);
  }
  .crsa-row:active {
    background: var(--well-2);
  }
  .crsa-row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .crsa-row__n {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .crsa-row__fig {
    font-size: calc(var(--text-xs) * var(--ui-s));
    font-weight: var(--weight-label);
    color: var(--ink-3);
    letter-spacing: var(--track-title);
  }
  .crsa-chev {
    color: var(--ink-3);
    transition: transform var(--dur-1) var(--ease);
  }
  .crsa-row:hover .crsa-chev {
    transform: translateX(2px);
  }

  /* Detail */
  .crsa-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
  }
  .crsa-title {
    margin-top: var(--space-xl);
  }
  .crsa-titlerow {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .crsa-h1 {
    font-size: calc(var(--text-3xl) * var(--ui-s));
    font-weight: var(--weight-display);
    letter-spacing: var(--track-display);
    line-height: 1.03;
    margin: 0;
  }
  .crsa-meterline {
    display: flex;
    align-items: center;
    gap: var(--space-lg);
    margin-top: var(--space-md);
    flex-wrap: wrap;
  }
  .crsa-meter {
    flex: 1 1 120px;
    height: 3px;
    border-radius: var(--r-pill);
    background: var(--rule-strong);
    overflow: hidden;
  }
  .crsa-meter > i {
    display: block;
    width: var(--v, 0%);
    height: 100%;
    border-radius: var(--r-pill);
    background: var(--ink);
  }
  .crsa-meterline__fig {
    flex: none;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    letter-spacing: var(--track-title);
  }

  .crsa-chaps {
    display: grid;
    gap: var(--space-xl);
    margin-top: var(--space-2xl);
  }
  .crsa-chaplabel {
    margin: 0 0 var(--space-xs);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .cd-card.crsa-topics {
    padding: 0;
    overflow: clip;
  }
  .crsa-topic {
    display: grid;
    grid-template-columns: 2px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(48px * var(--ui-s));
    padding: 0 var(--space-lg);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .crsa-topic + .crsa-topic {
    border-top: 1px dashed var(--rule-strong);
  }
  .crsa-topic:hover {
    background: var(--well);
  }
  .crsa-topic:active {
    background: var(--well-2);
  }
  .crsa-topic:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .crsa-topic__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    color: var(--ink-2);
  }
  .crsa-topic[data-next] .crsa-topic__t {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .crsa-topic__m {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    text-align: right;
  }

  /* The state tick: a 2×16 rounded bar whose HEIGHT is the topic's rung —
     done = the full ink bar, step = the bar at half height, not started = the
     full bar in --rule-strong. Twelve of them read as a hairline ledger. */
  .crsa-tick {
    display: grid;
    align-items: end;
    width: 2px;
    height: 16px;
  }
  .crsa-tick i {
    display: block;
    width: 2px;
    border-radius: var(--r-pill);
    height: var(--h, 16px);
    background: var(--bg, var(--rule-strong));
  }
  .crsa-tick[data-s='done'] i {
    --h: 16px;
    --bg: var(--ink);
  }
  .crsa-tick[data-s='step'] i {
    --h: 8px;
    --bg: var(--ink);
  }
  .crsa-tick[data-s='none'] i {
    --h: 16px;
    --bg: var(--rule-strong);
  }

  @media (prefers-reduced-motion: reduce) {
    .crsa-row,
    .crsa-chev,
    .crsa-topic {
      transition: none;
    }
  }
</style>
