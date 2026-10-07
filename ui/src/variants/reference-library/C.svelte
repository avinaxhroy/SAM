<!--
  REFERENCE · C · THE FOLDERS.
  Folder-sleeve variant grouped by course with an additional unfiled folder.
  Clicking a folder pocket slides out its card sheet and displays filed items.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { foldersOf, type ReferenceProps } from './props';
  import { safeUrl } from '../../safeUrl';

  let {
    heading,
    segments,
    segment,
    courses,
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

  // IDs of expanded course folders.
  let opened = $state<string[]>([]);

  // IDs of expanded note rows.
  let noted = $state<string[]>([]);

  // Search input element reference.
  let field: HTMLInputElement | undefined = $state();

  const folders = $derived(foldersOf(records, courses));
  /** The records the query admits, so a folder can say `n of m`. */
  const admitted = $derived(new Set(matches.map((record) => record.id)));

  function toggleNote(id: string) {
    noted = noted.includes(id) ? noted.filter((held) => held !== id) : [...noted, id];
  }

  function toggle(key: string, face: HTMLButtonElement | undefined) {
    if (opened.includes(key)) {
      opened = opened.filter((held) => held !== key);
      face?.focus();
    } else {
      opened = [...opened, key];
    }
  }

  /** The design's own root, so a window-level Escape only acts on a key pressed
   *  inside the board. */
  let root: HTMLDivElement | undefined = $state();

  /** Escape shuts the folder the focus is in, and returns focus to its face. */
  function keys(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !root?.contains(event.target as Node)) return;
    const held = (event.target as HTMLElement | null)?.closest('.c-folder');
    const key = held?.getAttribute('data-key');
    if (!key || !opened.includes(key)) return;
    opened = opened.filter((open) => open !== key);
    (held?.querySelector('.c-face') as HTMLButtonElement | null)?.focus();
  }
</script>

<svelte:window onkeydown={keys} />

<div class="v-fit c-page" bind:this={root}>
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
      <label class="cd-searchpill ref-find">
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
      {#if query.trim().length > 0}
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
      {/if}
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

  {#if query.trim().length > 0}
    <div class="c-head">
      <span class="c-head__f">
        <b class="num">{matches.length}</b> of <b class="num">{records.length}</b> match
      </span>
    </div>
  {/if}

  <div class="c-board">
    {#each folders as folder (folder.key)}
      {@const rows = folder.records.filter((record) => admitted.has(record.id))}
      {@const open = opened.includes(folder.key)}
      <section class="c-folder" data-key={folder.key} data-open={open ? '1' : '0'}>
        <div class="c-letter">
          <div class="c-items" id={`ref-items-${folder.key}`} inert={open ? undefined : true}>
            <div class="c-items__in">
              {#if rows.length > 0}
                <ul class="c-list">
                  {#each rows as record (record.id)}
                    <li>
                      {#if safeUrl(record.url)}
                        <a
                          class="c-item"
                          href={record.url}
                          target="_blank"
                          rel="noreferrer"
                          title={record.url}
                        >
                          <span class="c-mark"><Icon name={record.glyph} size={15} /></span>
                          <span class="c-item__b">
                            <span class="c-item__t" title={record.title}>{record.title}</span>
                            <span class="c-item__f">{record.provider ?? record.kind}</span>
                          </span>
                          <span class="c-item__go" aria-hidden="true">
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
                          <span class="cd-sr">— {record.title} opens in a new tab</span>
                        </a>
                      {:else if record.body}
                        <button
                          class="c-item"
                          type="button"
                          aria-expanded={noted.includes(record.id)}
                          onclick={() => toggleNote(record.id)}
                        >
                          <span class="c-mark"><Icon name={record.glyph} size={15} /></span>
                          <span class="c-item__b">
                            <span class="c-item__t" title={record.title}>{record.title}</span>
                            <span class="c-item__f">{record.provider ?? record.kind}</span>
                          </span>
                          <span class="c-item__go" aria-hidden="true">
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
                          <span class="c-item__note" inert={noted.includes(record.id) ? undefined : true}>
                            <span class="c-item__notein">
                              <span class="c-item__notetx">{record.body}</span>
                            </span>
                          </span>
                        </button>
                      {:else}
                        <button
                          class="c-item"
                          type="button"
                          data-command="record.panel"
                          data-placement="recordTable.rowContext"
                          onclick={() => onOpen(record.id)}
                        >
                          <span class="c-mark"><Icon name={record.glyph} size={15} /></span>
                          <span class="c-item__b">
                            <span class="c-item__t" title={record.title}>{record.title}</span>
                            <span class="c-item__f">{record.provider ?? record.kind}</span>
                          </span>
                          <span class="c-item__go" aria-hidden="true">
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
                        </button>
                      {/if}
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="c-none">No match</p>
              {/if}
            </div>
          </div>
        </div>

        <div class="c-pocket">
          <span class="c-name">{folder.label}</span>
          <span class="cd-chip num">
            {query.trim().length > 0 ? `${rows.length} of ${folder.records.length}` : folder.records.length}
          </span>
        </div>
        <button
          class="c-face"
          type="button"
          aria-expanded={open}
          aria-controls={`ref-items-${folder.key}`}
          onclick={(event) => toggle(folder.key, event.currentTarget)}
        >
          <span class="cd-sr">
            {folder.label} — {rows.length} of {folder.records.length} saved
          </span>
        </button>
      </section>
    {/each}
  </div>
</div>

<style>
  /* The board: three folders across, aligned at their tops so opening one grows
     downward without moving the other two. */
  .c-board {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-lg);
    align-items: start;
  }
  .c-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
  }
  .c-head__f {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .c-head__f b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }

  .c-folder {
    position: relative;
    display: grid;
  }
  /* The paper. Its top edge stands above the pocket's rim, and the pocket covers
     its bottom: the margin and the pocket's height are one measurement together,
     so both move with the size register. */
  .c-letter {
    position: relative;
    z-index: 1;
    margin: 0 var(--space-lg);
    min-height: calc(36px * var(--ui-s));
    border-radius: var(--r-tile) var(--r-tile) 0 0;
    background: var(--card);
    box-shadow: var(--sh-1);
    transition: transform var(--dur-3) var(--ease);
  }
  .c-folder:hover .c-letter {
    transform: translateY(-4px);
  }
  .c-folder[data-open='1'] .c-letter {
    transform: translateY(-8px);
  }
  .c-folder[data-open='1']:hover .c-letter {
    transform: translateY(-10px);
  }
  /* The pocket: the reference's own sleeve, with its cut moved from the right
     edge to the top edge, where a folder's thumb-cut belongs. The mask's second
     stop is a token at full opacity — a mask reads alpha, not colour. */
  .c-pocket {
    position: relative;
    z-index: 2;
    margin-top: calc(-26px * var(--ui-s));
    height: calc(126px * var(--ui-s));
    padding: var(--space-md);
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-sm);
    border-radius: var(--r-card);
    background: var(--well);
    box-shadow: var(--sh-1);
    -webkit-mask-image: radial-gradient(circle 15px at 78% 0%, transparent 14px, var(--ink) 15px);
    mask-image: radial-gradient(circle 15px at 78% 0%, transparent 14px, var(--ink) 15px);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-2) var(--ease);
  }
  .c-folder:hover .c-pocket {
    background: var(--well-2);
    box-shadow: var(--sh-2);
  }
  /* A chip's default fill is `--well` and so is the pocket's, so on the pocket
     the count takes the deeper well instead of vanishing into it. */
  .c-pocket .cd-chip {
    background: var(--well-2);
  }
  .c-name {
    font-size: calc(var(--text-lg) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.1;
  }
  .c-folder[data-open='1'] .c-name {
    color: var(--ink);
  }
  /* The pocket's face is a real button over the whole sleeve, at
     `aria-expanded`, naming its own list. */
  .c-face {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 3;
    height: calc(126px * var(--ui-s));
    border-radius: var(--r-card);
  }
  .c-face:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  /* The items, unfolding inside the paper. */
  .c-items {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-2) var(--ease);
  }
  .c-folder[data-open='1'] .c-items {
    grid-template-rows: 1fr;
  }
  .c-items__in {
    min-height: 0;
    overflow: hidden;
  }
  .c-list {
    display: grid;
  }
  .c-item {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) 16px;
    align-items: center;
    gap: var(--space-sm);
    width: 100%;
    min-height: calc(56px * var(--ui-s));
    padding: 0 var(--space-md);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .c-list li + li .c-item {
    border-top: 1px dashed var(--rule-strong);
  }
  a.c-item:hover,
  button.c-item:hover {
    background: var(--well);
  }
  a.c-item:active,
  button.c-item:active {
    background: var(--well-2);
  }
  .c-item:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .c-mark {
    color: var(--ink-3);
    display: grid;
    place-items: center;
  }
  .c-item__b {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .c-item__t {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .c-item__f {
    font-size: var(--text-2xs);
    color: var(--ink-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .c-item__go {
    color: var(--ink-3);
    display: grid;
    place-items: center;
    transition: transform var(--dur-2) var(--ease), color var(--dur-1) var(--ease);
  }
  a.c-item:hover .c-item__go {
    color: var(--ink);
    transform: translateX(2px);
  }
  button.c-item[aria-expanded='true'] .c-item__go {
    color: var(--ink);
    transform: rotate(180deg);
  }
  /* A note's own text, disclosed in place: the last column of the row, under it. */
  .c-item__note {
    grid-column: 2 / -1;
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-2) var(--ease);
  }
  .c-item[aria-expanded='true'] .c-item__note {
    grid-template-rows: 1fr;
  }
  .c-item__notein {
    min-height: 0;
    overflow: hidden;
  }
  .c-item__notetx {
    display: block;
    padding: var(--space-xs) 0 var(--space-sm);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    line-height: 1.5;
    white-space: normal;
  }
  .c-none {
    display: flex;
    align-items: center;
    min-height: calc(56px * var(--ui-s));
    padding: 0 var(--space-md);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* Three folders across a wide room; the board folds to two and then to one,
     measured on its own container, never the window. */
  @container (max-width: 880px) {
    .c-board {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @container (max-width: 560px) {
    .c-board {
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
    .c-letter,
    .c-pocket,
    .c-items,
    .c-item,
    .c-item__note,
    .c-item__go {
      transition: none;
    }
  }
</style>
