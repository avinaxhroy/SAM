<!--
  Practice banks variant C: The Tactile Commit.
  List view with square progress indicators and an anchored tactile button
  for logging practice sessions.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import {
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
  /** The row group's one tab stop, and the cap whose release is replaying. */
  let stopId = $state<string | null>(null);
  let hitId = $state<string | null>(null);

  const stop = $derived(stopId ?? banks[0]?.id ?? null);
  const openBank = $derived(banks.find((bank) => bank.id === openedId) ?? null);
  const draft = $derived(openBank === null ? null : forecast(openBank, draftAtt, draftSol, stamp));
  const canSave = $derived(openBank !== null && !busy && writable(openBank, draftAtt, draftSol));

  function pick(bank: PracticeBank): void {
    if (openedId === bank.id) {
      close();
      return;
    }
    stopId = bank.id;
    draftAtt = String(bank.attempted);
    draftSol = String(bank.solved);
    onOpen(bank.id);
    setTimeout(() => {
      const field = document.getElementById(`pc-att-${bank.id}`);
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
    setTimeout(() => document.getElementById(`pc-row-${id}`)?.focus(), 0);
  }

  /** ↑/↓ walk the rows and carry the pen with them. */
  function walk(from: number, delta: number): void {
    const next = banks[from + delta];
    if (!next) return;
    const hadPen = openedId !== null;
    stopId = next.id;
    if (hadPen) {
      draftAtt = String(next.attempted);
      draftSol = String(next.solved);
      onOpen(next.id);
    }
    setTimeout(() => document.getElementById(`pc-row-${next.id}`)?.focus(), 0);
  }

  async function save(): Promise<void> {
    if (openBank === null || !canSave) return;
    const attempted = readNum(draftAtt, openBank.total);
    const solved = readNum(draftSol, openBank.total);
    if (attempted === null || solved === null) return;
    const id = openBank.id;
    if (!(await onLog(id, attempted, solved))) return;
    // The release, after the press has landed and `:active` is gone.
    hitId = id;
    setTimeout(() => (hitId = hitId === id ? null : hitId), 260);
    // The cap disabled itself by its own success and cannot hold focus, so the
    // write's own first field takes the keyboard rather than `<body>`.
    setTimeout(() => document.getElementById(`pc-att-${id}`)?.focus(), 0);
  }
</script>

<div class="v-fit pc">
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
    <div class="pc-retry">
      <span class="pc-retry__s">The course names did not come back. Every bank and every number below is your own data.</span>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={onRetryNames}>Read them again</button>
    </div>
  {/if}

  <div class="cd-card pc-board">
    <div class="pc-rows" role="group" aria-label="Banks">
      {#each banks as bank, index (bank.id)}
        {@const open = openedId === bank.id}
        {@const reading = open ? draft : null}
        {@const runs = reading?.runs ?? []}
        {@const refused = reading?.bad ?? false}
        {@const triedNow = open ? readNum(draftAtt, bank.total) : null}
        <button
          class="pc-row"
          id={`pc-row-${bank.id}`}
          type="button"
          data-record-id={bank.id}
          aria-expanded={open}
          aria-controls={`pc-log-${bank.id}`}
          tabindex={stop === bank.id ? 0 : -1}
          onclick={() => pick(bank)}
          onkeydown={(event) => {
            if (event.key === 'ArrowDown') {
              event.preventDefault();
              walk(index, 1);
            } else if (event.key === 'ArrowUp') {
              event.preventDefault();
              walk(index, -1);
            }
          }}
        >
          <span class="pc-sq" aria-hidden="true"><i style={`--v: ${bank.share ?? 0}%`}></i></span>
          <span class="pc-row__n">{bank.label}</span>
          <span class="pc-row__fig num">{figureOf(bank)}</span>
        </button>

        {#if open}
          <!-- The pen, docked under its own row in DOM order — the app's own
               disclosure (§ the lab's `.cd-detail`). The gap above it is the gap
               below it: the pen belongs to the row above, and is not part of it. -->
          <form
            class="cd-detail pc-log"
            id={`pc-log-${bank.id}`}
            onsubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <div class="pc-log__top">
              <span class="pc-log__n">{bank.label}</span>
              {#if namesState === 'loading'}
                <span class="cd-skel pc-chip" aria-hidden="true"></span>
              {:else if bank.code}
                <span class="cd-chip cd-chip--code" data-w={bank.wash} title={bank.course ?? undefined}>{bank.code}</span>
              {/if}
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

            <div class="pc-fields">
              <label class="pc-field">
                <span class="pc-field__l">Tried</span>
                <span class="pc-field__box" data-bad={reading?.bad ? '1' : '0'}>
                  <input
                    class="pc-field__in"
                    id={`pc-att-${bank.id}`}
                    type="text"
                    inputmode="numeric"
                    autocomplete="off"
                    spellcheck="false"
                    bind:value={draftAtt}
                    onkeydown={(event) => {
                      if (event.key === 'Escape') close();
                    }}
                  />
                  <span class="pc-field__u">{bank.total === null ? 'tried' : `of ${bank.total}`}</span>
                </span>
              </label>
              <label class="pc-field">
                <span class="pc-field__l">Got right</span>
                <span class="pc-field__box" data-bad={reading?.bad ? '1' : '0'}>
                  <input
                    class="pc-field__in"
                    id={`pc-sol-${bank.id}`}
                    type="text"
                    inputmode="numeric"
                    autocomplete="off"
                    spellcheck="false"
                    bind:value={draftSol}
                    onkeydown={(event) => {
                      if (event.key === 'Escape') close();
                    }}
                  />
                  <span class="pc-field__u">{triedNow === null ? 'tried' : `of ${triedNow} tried`}</span>
                </span>
              </label>
            </div>

            <div class="pc-commit">
              <span class="pc-socket">
                <button
                  class="pc-cap"
                  type="submit"
                  data-command="record.setField"
                  data-placement="today.screen"
                  data-hit={hitId === bank.id ? '1' : undefined}
                  disabled={!canSave}
                >
                  <span class="pc-bead" aria-hidden="true"></span>
                  Log attempt
                </button>
              </span>
              {#if runs.length > 0}
                <p class="pc-preview" data-bad={refused ? '1' : '0'}>
                  <!-- The box's own label says what kind of sentence follows, so a
                       refusal is never labelled as a consequence that happened —
                       and the sentence does not repeat the label it sits under. -->
                  <span class="pc-preview__l">{refused ? 'Not saved' : 'After saving'}</span>
                  <span class="pc-preview__v">
                    {#each (refused ? runs : runs.slice(1)) as run, runIndex (runIndex)}{#if run.b}<b>{run.t}</b>{:else}{run.t}{/if}{/each}
                  </span>
                </p>
              {/if}
            </div>
          </form>
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .pc-retry {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    flex-wrap: wrap;
    margin-bottom: var(--space-lg);
  }
  .pc-retry__s {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
  }

  .cd-card.pc-board {
    padding: 0;
    overflow: clip;
  }
  .pc-rows > * + * {
    border-top: 1px dashed var(--rule-strong);
  }
  .pc-row {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    min-height: calc(48px * var(--ui-s));
    padding: 0 var(--space-lg);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .pc-row:hover {
    background: var(--well);
  }
  .pc-row:active {
    background: var(--well-2);
  }
  .pc-row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .pc-row[aria-expanded='true'] {
    background: var(--well);
  }
  .pc-row__n {
    font-size: calc(var(--text-base) * var(--ui-s));
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pc-row[aria-expanded='true'] .pc-row__n {
    font-weight: var(--weight-label);
  }
  .pc-row__fig {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-3);
    white-space: nowrap;
  }
  /* The square: the bank's share, measured from its own bottom. */
  .pc-sq {
    position: relative;
    display: block;
    width: 16px;
    height: 16px;
    border-radius: var(--r-key);
    overflow: clip;
    box-shadow: inset 0 0 0 1.5px var(--rule-strong);
  }
  .pc-sq > i {
    position: absolute;
    inset: auto 0 0 0;
    display: block;
    height: var(--v, 0%);
    background: var(--ink);
    transition: height var(--dur-3) var(--ease);
  }

  .cd-detail.pc-log {
    margin: var(--space-xs) 0;
    animation: pc-fold var(--dur-2) var(--ease) both;
  }
  @keyframes pc-fold {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .pc-log__top {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .pc-log__n {
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pc-log__top .cd-pill {
    margin-left: auto;
  }
  .pc-chip {
    width: 5ch;
    height: var(--chip-h);
    border-radius: var(--r-mini);
  }

  .pc-fields {
    display: flex;
    align-items: flex-end;
    gap: var(--space-lg);
    margin-top: var(--space-lg);
    flex-wrap: wrap;
  }
  .pc-field {
    display: block;
  }
  .pc-field__l {
    display: block;
    font-size: calc(var(--text-2xs) * var(--ui-s));
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
    margin-bottom: var(--space-2xs);
  }
  /* A capsule like the system's own duration control: the number and its unit
     fused in one pill. */
  .pc-field__box {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    height: var(--pill-h);
    padding: 0 var(--space-md);
    border-radius: var(--r-pill);
    background: var(--well-2);
    transition: box-shadow var(--dur-1) var(--ease);
  }
  .pc-field__box:focus-within {
    box-shadow: inset 0 0 0 2px var(--ink);
  }
  .pc-field__box[data-bad='1'] {
    box-shadow: inset 0 0 0 1.5px var(--on-overdue);
  }
  .pc-field__in {
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
  .pc-field__u {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
  }

  .pc-commit {
    display: flex;
    align-items: stretch;
    gap: var(--space-lg);
    margin-top: var(--space-lg);
    flex-wrap: wrap;
  }
  /* The socket: one well, one inset shadow, nothing else. */
  .pc-socket {
    flex: none;
    align-self: center; /* the socket hugs the cap, it never stretches */
    padding: 4px;
    border-radius: var(--r-pill);
    background: var(--well-2);
    box-shadow: inset 0 2px 4px -1px color-mix(in oklab, var(--ink) 22%, transparent);
    transition: box-shadow var(--dur-2) var(--ease);
  }
  .pc-socket:has(.pc-cap:active) {
    box-shadow: inset 0 3px 6px -1px color-mix(in oklab, var(--ink) 34%, transparent);
  }
  .pc-cap {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    height: var(--pill-h-lg);
    padding: 0 var(--space-xl);
    border-radius: var(--r-pill);
    background: var(--ink);
    color: var(--ink-inv);
    font-family: var(--font-display);
    font-size: calc(var(--text-base) * var(--ui-s));
    font-weight: var(--weight-label);
    /* THE CONTACT EDGE. The cap meets the well along its bottom, so that is
       where the squash is anchored: the bottom stays put and the cap comes down
       into the socket, instead of shrinking toward its own middle. */
    transform-origin: center bottom;
    box-shadow: var(--sh-ink);
    transition: transform var(--dur-2) var(--ease), box-shadow var(--dur-2) var(--ease);
  }
  .pc-cap:hover {
    transform: translateY(-1px);
  }
  .pc-cap:active {
    transform: translateY(3px) scaleY(0.9);
    box-shadow: var(--sh-1);
    transition-duration: var(--dur-1);
  }
  .pc-cap:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 3px;
  }
  .pc-cap[disabled] {
    opacity: 0.45;
    pointer-events: none;
  }
  /* The release, after the click has landed and `:active` is gone: the cap
     springs back out of the well from its own contact edge. */
  .pc-cap[data-hit='1'] {
    animation: pc-settle var(--dur-2) var(--ease-pop) both;
  }
  @keyframes pc-settle {
    from {
      transform: translateY(3px) scaleY(0.9);
    }
  }
  /* The bead: the reference's indicator light, in this system's one ink. */
  .pc-bead {
    width: 7px;
    height: 7px;
    border-radius: var(--r-pill);
    background: color-mix(in oklab, var(--ink-inv) 34%, transparent);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .pc-cap:hover .pc-bead {
    background: color-mix(in oklab, var(--ink-inv) 62%, transparent);
  }
  .pc-cap:active .pc-bead {
    background: var(--ink-inv);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--ink-inv) 20%, transparent);
  }

  /* The consequence, beside the cap, dashed because it has not happened yet. */
  .pc-preview {
    flex: 1 1 12ch;
    min-width: 0;
    display: grid;
    align-content: center;
    gap: var(--space-3xs);
    margin: 0;
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--r-tile);
    border: 1.5px dashed var(--rule-strong);
  }
  .pc-preview__l {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .pc-preview__v {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
  }
  .pc-preview__v b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .pc-preview[data-bad='1'] {
    border-color: var(--on-overdue);
  }
  .pc-preview[data-bad='1'] .pc-preview__v,
  .pc-preview[data-bad='1'] .pc-preview__v b {
    color: var(--on-overdue);
  }

  @media (prefers-reduced-motion: reduce) {
    .pc-row,
    .pc-sq > i,
    .pc-cap,
    .pc-bead,
    .pc-socket {
      transition: none;
    }
    .pc-cap[data-hit='1'],
    .cd-detail.pc-log {
      animation: none;
    }
  }
</style>
