<!-- Practice banks variant B: Folders In A Rack. Folder-tab layout with sliding progress slip. -->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
    exactShare,
    figureOf,
    forecast,
    readNum,
    subLine,
    writable,
    type PracticeBank,
    type PracticeProps,
  } from './props';

  let {
    title,
    banks,
    summary,
    openedId,
    namesState,
    newBankCommand,
    stamp,
    busy,
    onOpen,
    onClose,
    onRecord,
    onNewBank,
    onRetryNames,
    onLog,
  }: PracticeProps = $props();

  let draftAtt = $state('');
  let draftSol = $state('');
  let rack = $state<HTMLElement | null>(null);

  const openBank = $derived(banks.find((bank) => bank.id === openedId) ?? null);
  const draft = $derived(openBank === null ? null : forecast(openBank, draftAtt, draftSol, stamp));
  const canSave = $derived(openBank !== null && !busy && writable(openBank, draftAtt, draftSol));

  /** The preview rides the open folder's pocket lip, above the fields that make
      it: the pocket's own solid ink does not move and only the dashed extension
      is drawn. A refused write draws no extension at all. */
  const ext = $derived.by(() => {
    if (openBank === null || draft === null || draft.bad) return null;
    const solved = readNum(draftSol, openBank.total);
    if (solved === null || solved === openBank.solved) return null;
    const from = exactShare(openBank);
    const to = openBank.total === null || openBank.total === 0 ? from : (solved / openBank.total) * 100;
    return `--from: ${Math.min(from, to).toFixed(2)}%; --to: ${Math.max(from, to).toFixed(2)}%`;
  });

  function pick(bank: PracticeBank): void {
    if (openedId === bank.id) {
      onClose();
      return;
    }
    draftAtt = String(bank.attempted);
    draftSol = String(bank.solved);
    onOpen(bank.id);
    setTimeout(() => {
      const field = document.getElementById(`pb-att-${bank.id}`);
      if (field instanceof HTMLInputElement) {
        field.focus();
        field.select();
      }
    }, 0);
  }

  function close(): void {
    const id = openedId;
    onClose();
    if (id === null) return;
    setTimeout(() => document.getElementById(`pb-front-${id}`)?.focus(), 0);
  }

  /** ←/→ step one cell, ↑/↓ one rack row — and the step is read off the rack
      that actually rendered (4 / 3 / 2 columns by width), never assumed. */
  function walk(from: number, key: string): void {
    let step = 1;
    if (key === 'ArrowUp' || key === 'ArrowDown') {
      const columns = rack === null ? 4 : getComputedStyle(rack).gridTemplateColumns.split(' ').length;
      step = Math.max(1, columns);
    }
    const next = banks[from + (key === 'ArrowRight' || key === 'ArrowDown' ? step : -step)];
    if (next) document.getElementById(`pb-front-${next.id}`)?.focus();
  }

  async function save(): Promise<void> {
    if (openBank === null || !canSave) return;
    const attempted = readNum(draftAtt, openBank.total);
    const solved = readNum(draftSol, openBank.total);
    if (attempted === null || solved === null) return;
    const id = openBank.id;
    if (!(await onLog(id, attempted, solved))) return;
    // The commit disabled the control it came from, and a disabled element
    // cannot hold focus: the write's own first field takes it rather than
    // dropping the keyboard on `<body>`.
    setTimeout(() => document.getElementById(`pb-att-${id}`)?.focus(), 0);
  }
</script>

<div class="v-fit pb">
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{subLine(summary)}</p>
    </div>
    <span class="cd-pagehead__aside">
      {#if newBankCommand}
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          data-command={newBankCommand}
          data-placement="today.screen"
          onclick={onNewBank}
        >
          <Icon name="plus" size={13} />
          Add a bank
        </button>
      {/if}
    </span>
  </header>

  {#if namesState === 'failed'}
    <!-- Only the course names are missing: the banks, their counts and their
         pockets are read from the destination's own view and are unaffected. -->
    <div class="pb-retry">
      <span class="pb-retry__s">The course names did not come back. Every bank and every number below is your own data.</span>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={onRetryNames}>Read them again</button>
    </div>
  {/if}

  <div class="pb-rack" bind:this={rack}>
    {#each banks as bank, index (bank.id)}
      {@const open = openedId === bank.id}
      {@const reading = open ? draft : null}
      {@const triedNow = open ? readNum(draftAtt, bank.total) : null}
      <div class="pb-cell" data-open={open ? '1' : '0'} data-record-id={bank.id}>
        <button
          class="pb-front"
          id={`pb-front-${bank.id}`}
          data-w={bank.wash}
          type="button"
          aria-expanded={open}
          aria-controls={`pb-card-${bank.id}`}
          onclick={() => pick(bank)}
          onkeydown={(event) => {
            if (event.key === 'Escape') {
              close();
              return;
            }
            if (['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight'].includes(event.key)) {
              event.preventDefault();
              walk(index, event.key);
            }
          }}
        >
          <span class="pb-tab" aria-hidden="true"></span>
          {#if namesState === 'loading'}
            <span class="cd-skel pb-chip" aria-hidden="true"></span>
          {:else if bank.code}
            <!-- The chip is the course's own code (the lab's chip, neutral well);
                 the course's name rides its `title`. -->
            <span class="cd-chip cd-chip--code" title={bank.course ?? undefined}>{bank.code}</span>
          {/if}
          <span class="pb-front__n">{bank.label}</span>
          <span class="pb-front__meta">
            <span class="pb-front__fig num">{figureOf(bank)}</span>
            <svg class="pb-arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M5.5 9 12 15.5 18.5 9"></path></svg>
          </span>
          <span class="pb-pocket" aria-hidden="true">
            <i style={`--v: ${bank.share ?? 0}%`}></i>
            {#if open && ext}
              <span class="pb-ext" style={ext}></span>
            {/if}
          </span>
        </button>

        <div class="pb-card" id={`pb-card-${bank.id}`} inert={!open}>
          <div class="pb-card__in">
            <form
              class="pb-form"
              onsubmit={(event) => {
                event.preventDefault();
                void save();
              }}
            >
              <label class="pb-field">
                <span class="pb-field__box" data-bad={reading?.bad ? '1' : '0'}>
                  <span class="pb-field__l">Tried</span>
                  <input
                    class="pb-field__in"
                    id={`pb-att-${bank.id}`}
                    type="text"
                    inputmode="numeric"
                    autocomplete="off"
                    spellcheck="false"
                    bind:value={draftAtt}
                    onkeydown={(event) => {
                      if (event.key === 'Escape') close();
                    }}
                  />
                  <span class="pb-field__unit">{bank.total === null ? 'tried' : `of ${bank.total}`}</span>
                </span>
              </label>
              <label class="pb-field">
                <span class="pb-field__box" data-bad={reading?.bad ? '1' : '0'}>
                  <span class="pb-field__l">Right</span>
                  <input
                    class="pb-field__in"
                    id={`pb-sol-${bank.id}`}
                    type="text"
                    inputmode="numeric"
                    autocomplete="off"
                    spellcheck="false"
                    bind:value={draftSol}
                    onkeydown={(event) => {
                      if (event.key === 'Escape') close();
                    }}
                  />
                  <span class="pb-field__unit">{triedNow === null ? 'tried' : `of ${triedNow} tried`}</span>
                </span>
              </label>
              <button
                class="cd-pill pb-save"
                type="submit"
                data-command="record.setField"
                data-placement="today.screen"
                disabled={!canSave}
              >
                Save attempt
              </button>
            </form>

            <p class="pb-say" data-bad={reading?.bad ? '1' : '0'}>
              {#each reading?.runs ?? [] as run, runIndex (runIndex)}{#if run.b}<b>{run.t}</b>{:else}{run.t}{/if}{/each}
            </p>

            <div class="pb-card__foot">
              <span class="pb-since">{bank.since}</span>
              <button
                class="cd-pill cd-pill--quiet cd-pill--sm"
                type="button"
                data-command="record.panel"
                data-placement="recordTable.rowContext"
                onclick={() => onRecord(bank.id)}
              >
                Record
              </button>
            </div>
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .pb-retry {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    flex-wrap: wrap;
    margin-bottom: var(--space-lg);
  }
  .pb-retry__s {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
  }

  .pb-rack {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-lg);
    align-items: start;
  }
  .pb-cell {
    display: grid;
    grid-template-rows: calc(144px * var(--ui-s)) 0fr;
    /* A grid item's automatic minimum size would let a long sentence push the
       cell wider than its own rack column. */
    min-width: 0;
    transition: grid-template-rows var(--dur-3) var(--ease);
  }
  .pb-cell[data-open='1'] {
    grid-template-rows: calc(144px * var(--ui-s)) 1fr;
  }

  .pb-front {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-xs);
    min-width: 0;
    width: 100%;
    height: 100%;
    padding: var(--space-lg) var(--space-lg) calc(var(--space-lg) + 20px);
    text-align: left;
    background: var(--card);
    border-radius: var(--r-tile);
    box-shadow: var(--sh-1);
    transition: box-shadow var(--dur-2) var(--ease), transform var(--dur-2) var(--ease);
  }
  .pb-front:hover {
    transform: translateY(-2px);
    box-shadow: var(--sh-2);
  }
  .pb-front:active {
    transform: none;
    box-shadow: var(--sh-1);
  }
  /* One ring around the whole folder, drawn out of the button: the strip stands
     above the sheet, so an outline on the sheet's own box would cut through it. */
  .pb-front:focus-visible {
    outline: none;
  }
  .pb-front:focus-visible::after {
    content: '';
    position: absolute;
    inset: calc(-14px * var(--ui-s)) -2px -2px;
    border: 2px solid var(--focus);
    border-radius: calc(var(--r-tile) + 2px);
    pointer-events: none;
  }
  /* One wash per folder — the tab — so the code chip stays the neutral well. */
  .pb-front .cd-chip {
    --wash: var(--well);
    --wash-hi: var(--well);
    --wash-lo: var(--well);
    --wash-grad: none;
    --onwash: var(--ink-2);
  }
  .pb-chip {
    width: 5ch;
    height: var(--chip-h);
    border-radius: var(--r-mini);
  }

  /* ── the tab ──────────────────────────────────────────────────────────── */
  .pb-tab {
    --tab-h: 13px;
    --tab-w: 50%;
    --tab-flank: 13px;
    position: absolute;
    left: 0;
    bottom: calc(100% - 1px);
    width: var(--tab-w);
    height: var(--tab-h);
    background-color: var(--wash, var(--well));
    background-image: linear-gradient(
      180deg,
      var(--wash-hi, var(--wash)) 0%,
      var(--wash, var(--well)) 62%,
      var(--wash-lo, var(--wash)) 100%
    );
    clip-path: polygon(0 0, calc(100% - var(--tab-flank)) 0, 100% 100%, 0 100%);
    transition: width var(--dur-2) var(--ease), transform var(--dur-1) var(--ease);
  }
  /* Handling the folder turns the strip toward the sheet's own light; pressing
     it turns the strip toward the room's ink. One layer, the token's two ends. */
  .pb-tab::after {
    content: '';
    position: absolute;
    inset: 0;
    background-color: var(--wash-hi, var(--wash));
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease), background-color var(--dur-1) var(--ease);
  }
  .pb-cell[data-open='1'] .pb-tab {
    --tab-w: 63%;
  }
  .pb-cell[data-open='1'] .pb-tab::after {
    opacity: 0.25;
  }
  .pb-front:hover .pb-tab {
    --tab-w: 63%;
  }
  .pb-front:hover .pb-tab::after {
    opacity: 0.45;
  }
  .pb-front:active .pb-tab {
    --tab-w: 53%;
    transform: translateY(1px);
  }
  .pb-front:active .pb-tab::after {
    background-color: var(--wash-lo, var(--wash));
    opacity: 0.85;
    transition-duration: var(--dur-1);
  }

  .pb-front__n {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    line-height: 1.2;
    margin-top: var(--space-2xs);
    overflow-wrap: anywhere;
  }
  .pb-front__meta {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    width: 100%;
    margin-top: auto;
  }
  .pb-front__fig {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .pb-arrow {
    margin-left: auto;
    color: var(--ink-3);
    transition: transform var(--dur-2) var(--ease), color var(--dur-1) var(--ease);
  }
  .pb-front:hover .pb-arrow {
    color: var(--ink);
    transform: translateY(2px);
  }
  .pb-cell[data-open='1'] .pb-arrow {
    transform: rotate(180deg);
  }

  /* ── the pocket ───────────────────────────────────────────────────────── */
  .pb-pocket {
    position: absolute;
    left: var(--space-lg);
    right: var(--space-lg);
    bottom: var(--space-md);
    height: 8px;
    border-radius: var(--r-pill);
    background: var(--well-2);
    overflow: clip;
  }
  .pb-pocket > i {
    display: block;
    height: 100%;
    width: var(--v, 0%);
    border-radius: var(--r-pill);
    background: var(--ink);
    transition: width var(--dur-3) var(--ease);
  }
  .pb-ext {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--from, 0%);
    width: calc(var(--to, 0%) - var(--from, 0%));
    box-sizing: border-box;
    border: 1.5px dashed var(--ink-3);
    border-radius: var(--r-pill);
  }

  /* ── the logging card, slid out of the pocket ─────────────────────────── */
  .pb-card {
    min-width: 0;
    min-height: 0;
    overflow: clip;
  }
  .pb-card__in {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-md);
    min-width: 0;
    margin-top: var(--space-2xs);
    padding: var(--space-lg);
    border-radius: var(--r-tile);
    background: var(--well);
    box-shadow: inset 0 6px 8px -6px color-mix(in oklab, var(--ink) 16%, transparent);
    transform: translateY(calc(-12px * var(--ui-s)));
    opacity: 0;
    transition: transform var(--dur-3) var(--ease), opacity var(--dur-2) var(--ease);
  }
  .pb-cell[data-open='1'] .pb-card__in {
    transform: none;
    opacity: 1;
  }

  .pb-form {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-sm);
    min-width: 0;
  }
  .pb-field {
    display: block;
    min-width: 0;
  }
  .pb-field__box {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
    height: var(--pill-h);
    padding: 0 var(--space-sm);
    border-radius: var(--r-mini);
    background: var(--well-2);
    transition: box-shadow var(--dur-1) var(--ease);
  }
  .pb-field__box:focus-within {
    box-shadow: inset 0 0 0 2px var(--ink);
  }
  .pb-field__box[data-bad='1'] {
    box-shadow: inset 0 0 0 1.5px var(--on-overdue);
  }
  .pb-field__l {
    margin-right: auto;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pb-field__in {
    flex: none;
    width: 4ch;
    padding: 0;
    border: 0;
    background: none;
    outline: none;
    text-align: right;
    font-family: var(--font-display);
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    color: var(--ink);
  }
  .pb-field__unit {
    flex: none;
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
  }
  .pb-say {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
  }
  .pb-say:empty {
    display: none;
  }
  .pb-say b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .pb-say[data-bad='1'],
  .pb-say[data-bad='1'] b {
    color: var(--on-overdue);
  }
  .pb-card__foot {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    flex-wrap: wrap;
  }
  .pb-since {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .pb-card__foot .cd-pill {
    margin-left: auto;
  }
  .pb-save {
    width: 100%;
    justify-content: center;
  }

  /* The rack steps down with the room it is in — and it steps on its own
     container, not the window: the component is the same component at 360px in
     the record panel. */
  @container (max-width: 1000px) {
    .pb-rack {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  @container (max-width: 760px) {
    .pb-rack {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .pb-card__foot .cd-pill {
      margin-left: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pb-cell,
    .pb-tab,
    .pb-tab::after,
    .pb-arrow,
    .pb-card__in,
    .pb-pocket > i,
    .pb-front {
      transition: none;
    }
  }
</style>
