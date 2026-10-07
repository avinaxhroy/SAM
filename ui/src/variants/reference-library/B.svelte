<!--
  REFERENCE · B · THE PALETTE.
  Collapsible results deck variant anchored to a search input header. Results
  display as dual-column record cards supporting external links, note disclosures,
  and record detail panels.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { cameFrom, type ReferenceProps } from './props';
  import { safeUrl } from '../../safeUrl';

  let {
    heading,
    segments,
    segment,
    records,
    matches,
    query,
    newCommand,
    onSegment,
    onQuery,
    onClear,
    onAdd,
    onOpen,
  }: ReferenceProps = $props();

  // Results deck collapse state.
  let open = $state(true);

  // IDs of expanded note records.
  let noted = $state<string[]>([]);

  // Search input element reference.
  let field: HTMLInputElement | undefined = $state();

  function toggleNote(id: string) {
    noted = noted.includes(id) ? noted.filter((held) => held !== id) : [...noted, id];
  }

  let toggle: HTMLButtonElement | undefined = $state();

  /** The design's own root, so the window's keys only act on a key pressed
   *  inside the palette (the reference's own scope). */
  let root: HTMLDivElement | undefined = $state();

  function keys(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !open || !root?.contains(event.target as Node)) return;
    open = false;
    toggle?.focus();
  }

  /** Focusing the field opens the deck — the reference's own behaviour, kept. */
  function focusIn(event: FocusEvent) {
    if (!open && event.target instanceof HTMLInputElement && root?.contains(event.target)) open = true;
  }
</script>

<svelte:window onkeydown={keys} onfocusin={focusIn} />

<div class="v-fit b-page" bind:this={root}>
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{heading}</h1>
    </div>
    <span class="cd-pagehead__aside">
      {#if segments.length > 1}
        <span class="cd-seg" role="group" aria-label="Which kind of saved thing to show">
          {#each segments as option (option.type)}
            <button
              class="cd-seg__pill"
              type="button"
              aria-pressed={option.type === segment}
              onclick={() => onSegment(option.type)}
            >
              {option.label}
            </button>
          {/each}
        </span>
      {/if}
      <span class="cd-chip num">{records.length} saved</span>
      {#if newCommand}
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          data-command={newCommand}
          data-placement="today.screen"
          aria-label={`Add a ${segment ?? 'record'}`}
          onclick={onAdd}
        >
          <Icon name="plus" size={13} />
          Add
        </button>
      {/if}
    </span>
  </header>

  <section class="b-pal" data-open={open ? '1' : '0'}>
    <div class="b-trigger">
      <label class="cd-searchpill ref-find b-find">
        <Icon name="search" size={14} />
        <input
          type="search"
          placeholder="Search saved things"
          aria-label="Search saved things"
          autocomplete="off"
          value={query}
          bind:this={field}
          oninput={(event) => onQuery(event.currentTarget.value)}
        />
      </label>
      <button
        class="cd-iconbtn b-toggle"
        type="button"
        aria-expanded={open}
        aria-controls="ref-pal-deck"
        aria-label="Results"
        bind:this={toggle}
        onclick={() => (open = !open)}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
          focusable="false"
        ><path d="M4.5 6.5 8 10l3.5-3.5" /></svg>
      </button>
    </div>

    <div class="b-deck" id="ref-pal-deck">
      <div class="b-deck__in" inert={open ? undefined : true}>
        <div class="b-deck__pad">
          {#if query.trim().length > 0}
            <div class="b-tally">
              <span class="b-tally__f">
                <b class="num">{matches.length}</b> of <b class="num">{records.length}</b> match
              </span>
              <button
                class="ref-clear"
                type="button"
                onclick={() => {
                  onClear();
                  field?.focus();
                }}
              >
                <Icon name="close" size={12} />
                Clear
              </button>
            </div>
          {/if}

          <div class="b-cards">
            {#each matches as record (record.id)}
              {#if safeUrl(record.url)}
                <a
                  class="b-card"
                  href={record.url}
                  target="_blank"
                  rel="noreferrer"
                  title={record.url}
                >
                  <span class="b-card__top">
                    <span class="b-mark"><Icon name={record.glyph} size={15} /></span>
                    <span class="b-kind">{record.kind}</span>
                    <span class="b-card__go" aria-hidden="true">
                      <svg
                        width="14"
                        height="14"
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        focusable="false"
                      ><path d="M4.6 11.4 11.4 4.6" /><path d="M6.2 4.6h5.2v5.2" /></svg>
                    </span>
                  </span>
                  <span class="b-card__t" title={record.title}>{record.title}</span>
                  <span class="b-card__f">{cameFrom(record)}</span>
                  <span class="cd-sr">— {record.title} opens in a new tab</span>
                </a>
              {:else if record.body}
                <button
                  class="b-card"
                  type="button"
                  aria-expanded={noted.includes(record.id)}
                  onclick={() => toggleNote(record.id)}
                >
                  <span class="b-card__top">
                    <span class="b-mark"><Icon name={record.glyph} size={15} /></span>
                    <span class="b-kind">{record.kind}</span>
                    <span class="b-card__go" aria-hidden="true">
                      <svg
                        width="14"
                        height="14"
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.6"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        focusable="false"
                      ><path d="M4.5 6.5 8 10l3.5-3.5" /></svg>
                    </span>
                  </span>
                  <span class="b-card__t" title={record.title}>{record.title}</span>
                  <span class="b-card__f">{cameFrom(record)}</span>
                  <span class="b-card__body" inert={noted.includes(record.id) ? undefined : true}>
                    <span class="b-card__bodyin">
                      <span class="b-card__bodytx">{record.body}</span>
                    </span>
                  </span>
                </button>
              {:else}
                <button
                  class="b-card"
                  type="button"
                  data-command="record.panel"
                  data-placement="recordTable.rowContext"
                  onclick={() => onOpen(record.id)}
                >
                  <span class="b-card__top">
                    <span class="b-mark"><Icon name={record.glyph} size={15} /></span>
                    <span class="b-kind">{record.kind}</span>
                    <span class="b-card__go" aria-hidden="true">
                      <svg
                        width="14"
                        height="14"
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.6"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        focusable="false"
                      ><path d="M6 4.5 9.5 8 6 11.5" /></svg>
                    </span>
                  </span>
                  <span class="b-card__t" title={record.title}>{record.title}</span>
                  <span class="b-card__f">{cameFrom(record)}</span>
                </button>
              {/if}
            {/each}

            {#if matches.length === 0}
              <p class="b-none">Nothing saved here matches “{query.trim()}”.</p>
            {/if}
          </div>
        </div>
      </div>
    </div>
  </section>
</div>

<style>
  /* The palette: one `--well` object on the sheet. */
  .b-pal {
    background: var(--well);
    border-radius: var(--r-card);
    padding: var(--space-md);
  }
  .b-trigger {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  /* The field one rung up out of the palette: the system's own field recipe
     (`panel-reference.css`), on a card ground of its own. */
  .b-find {
    flex: 1;
    width: auto;
    background: var(--card);
    box-shadow: var(--sh-1);
  }
  .b-toggle {
    background: var(--card);
    color: var(--ink-2);
    box-shadow: var(--sh-1);
  }
  .b-toggle:hover {
    background: var(--card);
    color: var(--ink);
  }
  .b-toggle svg {
    transition: transform var(--dur-2) var(--ease);
  }
  .b-pal[data-open='1'] .b-toggle svg {
    transform: rotate(180deg);
  }

  /* The deck's open state is the reference's own accordion. The clipped level
     carries NO padding of its own, or a closed deck would still be as tall as
     its padding. */
  .b-deck {
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    transition: grid-template-rows var(--dur-3) var(--ease), opacity var(--dur-2) var(--ease);
  }
  .b-pal[data-open='1'] .b-deck {
    grid-template-rows: 1fr;
    opacity: 1;
  }
  .b-deck__in {
    min-height: 0;
    overflow: hidden;
  }
  .b-deck__pad {
    padding: var(--space-2xs) var(--space-2xs) 0;
  }
  .b-tally {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-2xs) var(--space-sm);
  }
  .b-tally__f {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }
  .b-tally__f b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  /* Results, two up. */
  .b-cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-sm);
  }
  .b-card {
    display: grid;
    gap: var(--space-xs);
    align-content: start;
    padding: var(--space-md);
    border-radius: var(--r-tile);
    background: var(--card);
    box-shadow: var(--sh-1);
    text-align: left;
    transition: transform var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease);
  }
  a.b-card:hover,
  button.b-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--sh-2);
  }
  a.b-card:active,
  button.b-card:active {
    transform: none;
    box-shadow: var(--sh-1);
  }
  .b-card:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .b-card__top {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }
  /* B's mark is a quiet well, never a wash: the kind's glyph and its word. */
  .b-mark {
    width: 26px;
    height: 26px;
    flex: none;
    border-radius: var(--r-mark);
    background: var(--well-2);
    color: var(--ink-2);
    display: grid;
    place-items: center;
  }
  .b-card__go {
    margin-left: auto;
    color: var(--ink-3);
    display: grid;
    place-items: center;
    transition: transform var(--dur-2) var(--ease), color var(--dur-1) var(--ease);
  }
  a.b-card:hover .b-card__go {
    color: var(--ink);
    transform: translateX(2px);
  }
  button.b-card[aria-expanded='true'] .b-card__go {
    color: var(--ink);
    transform: rotate(180deg);
  }
  .b-kind {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .b-card__t {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.2;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .b-card__f {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* A note's own text, disclosed in place — never a second surface. */
  .b-card__body {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-2) var(--ease);
  }
  .b-card[aria-expanded='true'] .b-card__body {
    grid-template-rows: 1fr;
  }
  .b-card__bodyin {
    min-height: 0;
    overflow: hidden;
  }
  .b-card__bodytx {
    display: block;
    margin-top: var(--space-xs);
    padding-top: var(--space-sm);
    border-top: 1px dashed var(--rule-strong);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    line-height: 1.5;
  }
  .b-none {
    grid-column: 1 / -1;
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    padding: var(--space-lg) var(--space-2xs);
  }

  /* A card is two columns wide; on a narrow room one card is the whole deck,
     and a card never squeezes below its own title. */
  @container (max-width: 620px) {
    .b-cards {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* Four controls on one line is a line that runs out. The head wraps before it
     does, and the field takes the room that is left. */
  @container (max-width: 720px) {
    .cd-pagehead {
      flex-wrap: wrap;
      align-items: flex-start;
    }
    .cd-pagehead__aside {
      flex: 1 1 auto;
      flex-wrap: wrap;
      justify-content: flex-end;
    }
    .ref-find {
      width: auto;
      flex: 1 1 200px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .b-deck,
    .b-card,
    .b-card__body,
    .b-toggle svg {
      transition: none;
    }
  }
</style>
