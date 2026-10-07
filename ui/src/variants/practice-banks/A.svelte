<!--
  Practice banks variant A: The Row That Opens.
  Expandable rows in a list format, where selecting a row reveals an inline logging bench
  and progress forecast bar.
-->
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
    onLog,
  }: PracticeProps = $props();

  /** The typed numbers of the open bench. The screen owns which bank is open;
      the draft is this design's own, because only this design renders a field. */
  let draftAtt = $state('');
  let draftSol = $state('');

  const openBank = $derived(banks.find((bank) => bank.id === openedId) ?? null);
  const draft = $derived(
    openBank === null ? null : forecast(openBank, draftAtt, draftSol, stamp),
  );
  const canSave = $derived(openBank !== null && !busy && writable(openBank, draftAtt, draftSol));

  /** The crest: the segment of the track between the bank's committed share and
      the share the typed number would reach. Drawn beside the fill, never in it,
      and drawn only while the write is one the bank could actually take — a
      refused write reaches no level, so it gets no mark. */
  const crest = $derived.by(() => {
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
    // The first field takes the keyboard, so the flow's four gestures are real:
    // open the entry, type, Tab, type, Enter. Without it the bench opens with
    // focus still on the row that opened it and the first keystrokes go nowhere.
    setTimeout(() => {
      const field = document.getElementById(`pa-att-${bank.id}`);
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
    setTimeout(() => document.getElementById(`pa-head-${id}`)?.focus(), 0);
  }

  /** ↑/↓/←/→ walk the rows. The design's own path, on the row's button. */
  function walk(from: number, delta: number): void {
    const next = banks[from + delta];
    if (next) document.getElementById(`pa-head-${next.id}`)?.focus();
  }

  async function save(): Promise<void> {
    if (openBank === null || !canSave) return;
    const attempted = readNum(draftAtt, openBank.total);
    const solved = readNum(draftSol, openBank.total);
    if (attempted === null || solved === null) return;
    const id = openBank.id;
    if (!(await onLog(id, attempted, solved))) return;
    // The commit disabled the control it came from (there is nothing left to
    // write) and a disabled element cannot hold focus, so the field the write
    // lives beside takes it rather than dropping the keyboard on `<body>`.
    setTimeout(() => document.getElementById(`pa-att-${id}`)?.focus(), 0);
  }
</script>

<div class="v-fit pa">
  <header class="cd-pagehead">
    <div>
      <h1 class="cd-pagehead__title">{title}</h1>
      <p class="cd-pagehead__sub">{subLine(summary)}</p>
    </div>
    <span class="cd-pagehead__aside">
      <!-- ONE door for creating a bank, and it is QUIET rather than ink: this
           screen's one dark object is the bench's own Save. -->
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

  <div class="cd-card pa-card">
    <div class="pa-list" data-anyopen={openedId === null ? '0' : '1'}>
      {#each banks as bank, index (bank.id)}
        {@const open = openedId === bank.id}
        {@const reading = open ? draft : null}
        {@const triedNow = open ? readNum(draftAtt, bank.total) : null}
        <div class="pa-row" data-open={open ? '1' : '0'} data-record-id={bank.id}>
          <button
            class="pa-head"
            id={`pa-head-${bank.id}`}
            type="button"
            aria-expanded={open}
            aria-controls={`pa-bench-${bank.id}`}
            inert={open}
            onclick={() => pick(bank)}
            onkeydown={(event) => {
              if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
                event.preventDefault();
                walk(index, 1);
              } else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
                event.preventDefault();
                walk(index, -1);
              }
            }}
          >
            <span class="pa-head__n">{bank.label}</span>
            <span class="pa-head__fig num">{figureOf(bank)}</span>
            <svg class="pa-chev" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M9 5.5 15.5 12 9 18.5"></path></svg>
          </button>

          <div class="pa-bench" id={`pa-bench-${bank.id}`} inert={!open}>
            <div class="pa-bench__in">
              <div class="pa-bench__top">
                {#if namesState === 'loading'}
                  <!-- The chip's slot is held at the chip's own height, so the
                       bench is the same height before and after the name lands. -->
                  <span class="cd-skel pa-chip" aria-hidden="true"></span>
                {:else if bank.code}
                  <!-- The chip is the course's own code, as the lab draws it; the
                       course's name rides its `title`, so the abbreviation a
                       student reads has its expansion one hover away. -->
                  <span class="cd-chip cd-chip--code" data-w={bank.wash} title={bank.course ?? undefined}>{bank.code}</span>
                {/if}
                <span class="pa-bench__n">{bank.label}</span>
                <button class="cd-pill cd-pill--quiet cd-pill--sm pa-close" type="button" onclick={close}>
                  Close
                </button>
              </div>

              <div class="pa-figrow">
                <span class="pa-fig num">{figureOf(bank)}</span>
                <span class="pa-track" aria-hidden="true">
                  <i style={`--v: ${bank.share ?? 0}%`}></i>
                  {#if open && crest}
                    <span class="pa-crest" style={crest}></span>
                  {/if}
                </span>
              </div>

              <form
                class="pa-form"
                onsubmit={(event) => {
                  event.preventDefault();
                  void save();
                }}
              >
                <label class="pa-field">
                  <span class="pa-field__l">Tried</span>
                  <span class="pa-field__box" data-bad={reading?.bad ? '1' : '0'}>
                    <input
                      class="pa-field__in"
                      id={`pa-att-${bank.id}`}
                      type="text"
                      inputmode="numeric"
                      autocomplete="off"
                      spellcheck="false"
                      bind:value={draftAtt}
                      onkeydown={(event) => {
                        if (event.key === 'Escape') close();
                      }}
                    />
                    <span class="pa-field__unit">{bank.total === null ? 'tried' : `of ${bank.total}`}</span>
                  </span>
                </label>
                <label class="pa-field">
                  <span class="pa-field__l">Got right</span>
                  <span class="pa-field__box" data-bad={reading?.bad ? '1' : '0'}>
                    <input
                      class="pa-field__in"
                      id={`pa-sol-${bank.id}`}
                      type="text"
                      inputmode="numeric"
                      autocomplete="off"
                      spellcheck="false"
                      bind:value={draftSol}
                      onkeydown={(event) => {
                        if (event.key === 'Escape') close();
                      }}
                    />
                    <span class="pa-field__unit">{triedNow === null ? 'tried' : `of ${triedNow} tried`}</span>
                  </span>
                </label>
                <button
                  class="cd-pill pa-save"
                  type="submit"
                  data-command="record.setField"
                  data-placement="today.screen"
                  disabled={!canSave}
                >
                  Save attempt
                </button>
              </form>

              <p class="pa-say" data-bad={reading?.bad ? '1' : '0'}>
                {#each reading?.runs ?? [] as run, runIndex (runIndex)}{#if run.b}<b>{run.t}</b>{:else}{run.t}{/if}{/each}
              </p>

              <div class="pa-foot">
                <span class="pa-since">{bank.since}</span>
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
</div>

<style>
  /* ── the list ─────────────────────────────────────────────────────────── */
  .cd-card.pa-card {
    padding: 0;
    overflow: clip;
  }
  .pa-row {
    position: relative;
    min-height: var(--head-h, calc(56px * var(--ui-s)));
    transition: min-height var(--dur-3) var(--ease);
  }
  .pa-row + .pa-row {
    border-top: 1px dashed var(--rule-strong);
  }
  /* The rows around the open one give way, so the list keeps roughly its height
     and nothing jumps out from under the eye. */
  .pa-list[data-anyopen='1'] .pa-row:not([data-open='1']) {
    --head-h: calc(44px * var(--ui-s));
  }
  .pa-row[data-open='1'] {
    min-height: 0;
  }

  /* The closed line. Absolutely placed, so the row's height is the bench's own
     and the line folds away rather than sitting there invisible. */
  .pa-head {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: var(--head-h, calc(56px * var(--ui-s)));
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 16px;
    align-items: center;
    gap: var(--space-md);
    padding: 0 var(--space-lg);
    text-align: left;
    overflow: clip;
    transition:
      height var(--dur-3) var(--ease),
      opacity var(--dur-2) var(--ease),
      transform var(--dur-2) var(--ease),
      background var(--dur-1) var(--ease);
  }
  .pa-head:hover {
    background: var(--well);
  }
  .pa-head:active {
    background: var(--well-2);
  }
  /* The row bleeds into the clipped card corner, so the ring is inset. */
  .pa-head:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .pa-row[data-open='1'] .pa-head {
    height: 0;
    opacity: 0;
    transform: translateY(-8px);
    pointer-events: none;
  }
  .pa-head__n {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pa-head__fig {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }
  .pa-chev {
    color: var(--ink-3);
    transition: transform var(--dur-1) var(--ease);
  }
  .pa-head:hover .pa-chev {
    transform: translateY(2px);
  }

  /* ── the bench ────────────────────────────────────────────────────────── */
  .pa-bench {
    overflow: clip;
    max-height: 0;
    opacity: 0;
    transform: translateY(8px);
    transition:
      max-height var(--dur-3) var(--ease),
      opacity var(--dur-2) var(--ease) 50ms,
      transform var(--dur-2) var(--ease) 50ms;
  }
  .pa-row[data-open='1'] .pa-bench {
    max-height: calc(560px * var(--ui-s));
    opacity: 1;
    transform: none;
  }
  .pa-bench__in {
    margin: var(--space-md) var(--space-lg) var(--space-lg);
    padding: var(--space-lg);
    border-radius: var(--r-tile);
    background: var(--well);
  }
  .pa-bench__top {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .pa-bench__n {
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pa-bench__top .pa-close {
    margin-left: auto;
  }
  .pa-chip {
    width: 5ch;
    height: var(--chip-h);
    border-radius: var(--r-mini);
  }

  .pa-figrow {
    display: flex;
    align-items: center;
    gap: var(--space-lg);
    margin-top: var(--space-md);
    flex-wrap: wrap;
  }
  .pa-fig {
    flex: none;
    font-size: calc(var(--text-md) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
  }
  /* The variant's one graphic: a 6px rule; the committed fill in ink, the
     preview as a dashed crest BESIDE it — never inside it. */
  .pa-track {
    position: relative;
    flex: 1 1 56px;
    min-width: 56px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--rule-strong);
    overflow: clip;
  }
  .pa-track > i {
    display: block;
    height: 100%;
    width: var(--v, 0%);
    border-radius: var(--r-pill);
    background: var(--ink);
    transition: width var(--dur-3) var(--ease);
  }
  .pa-crest {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--from, 0%);
    width: calc(var(--to, 0%) - var(--from, 0%));
    box-sizing: border-box;
    /* Dashed = not yet; `--ink-3`, not `--ink-4`, so the mark clears 3:1
       against the track it is drawn on. */
    border: 1.5px dashed var(--ink-3);
    border-radius: var(--r-pill);
  }

  .pa-form {
    display: flex;
    align-items: flex-end;
    gap: var(--space-lg);
    flex-wrap: wrap;
    margin-top: var(--space-lg);
  }
  .pa-field {
    display: block;
  }
  .pa-field__l {
    display: block;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    margin-bottom: var(--space-2xs);
  }
  .pa-field__box {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2xs);
    height: var(--hit);
    padding: 0 var(--space-sm);
    border-radius: var(--r-mini);
    background: var(--well-2);
    transition: box-shadow var(--dur-1) var(--ease);
  }
  .pa-field__box:focus-within {
    box-shadow: inset 0 0 0 2px var(--ink);
  }
  .pa-field__box[data-bad='1'] {
    box-shadow: inset 0 0 0 1.5px var(--on-overdue);
  }
  .pa-field__in {
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
  /* `--ink-2`, not `--ink-3`: at 11px on `--well-2` the muted ink measures
     4.36:1 in the light theme, under the floor — measured, not assumed. */
  .pa-field__unit {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
  }
  .pa-save {
    margin-left: auto;
  }
  .pa-say {
    margin-top: var(--space-md);
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    max-width: 62ch;
  }
  /* Nothing to write is not a sentence: the line is empty and takes no room. */
  .pa-say:empty {
    display: none;
  }
  .pa-say b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .pa-say[data-bad='1'],
  .pa-say[data-bad='1'] b {
    color: var(--on-overdue);
  }
  .pa-foot {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    margin-top: var(--space-lg);
  }
  .pa-since {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
  }
  .pa-foot .cd-pill {
    margin-left: auto;
  }

  /* A narrow canvas wraps the fields onto their own lines before the row loses
     its own fact: a bench whose figure is gone answers nothing. */
  @container (max-width: 760px) {
    .pa-save,
    .pa-foot .cd-pill {
      margin-left: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pa-row,
    .pa-head,
    .pa-bench,
    .pa-chev,
    .pa-track > i {
      transition: none;
    }
  }
</style>
