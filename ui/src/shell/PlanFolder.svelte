<!--
  The plan's own folder, in a sheet (`app.changePlan` — the titlebar's folder
  door, the menu, the palette, and `{"id":"app.changePlan","params":{"path":…}}`
  over the CLI's `sam invoke`).

  This is the picker's material over a working plan: the plans on this machine,
  the ones this window remembers, the OS's own folder dialog, and a typed path
  for a window that has none. Every open goes through the same
  `session.openPlan` the start screen uses, so a folder that is not a plan is
  refused in the engine's own words rather than silently ignored — and that
  refusal becomes the sheet's other door: *Make a new plan here*, one press
  that writes the blank plan into the folder the student chose (`plan.new`
  takes an empty folder, and refuses one that already holds something).

  Sheet layout:
    1. Current plan card (machine glyph, plan name, full path).
    2. List of plans on this machine (root plans and remembered recent paths).
    3. Directory picker dialog (when available via shell) and manual path input.

  A running (or paused) session is the one thing that stops a switch: the timer
  belongs to the folder it was started in, and `session.openPlan` refuses — in
  the sentence the app already uses when a session is asked to start over a
  running one. The sheet says so where the student can read it, rather than
  letting every press land as the same refusal.
-->
<script lang="ts">
  import Sheet from './Sheet.svelte';
  import Icon from './Icon.svelte';
  import { app } from '../session.svelte';
  import { pickDirectory, transport } from '../ipc';
  import { planFolder } from '../onboarding/plan';

  let { onclose }: { onclose: () => void } = $props();

  /** A typed path, for a window with no folder dialog. */
  let manual = $state('');
  /**
   * Which of the sheet's two doors is the open one. `browse` is the default —
   * the plans on this machine are what most students are looking for — and
   * `type` gives the field the whole width for a folder that is not in the
   * list. The switcher is not decoration: it decides which door the sheet
   * *leads* with, and both remain reachable from either.
   */
  let door = $state<'browse' | 'type'>('browse');
  /** A press in flight — one at a time, whichever door it came from. */
  let busy = $state(false);
  /**
   * The folder an open was refused for, when it is a candidate for a new plan:
   * a folder that holds no plan is not a dead end, it is a destination. `null`
   * = no offer stands.
   */
  let making = $state<string | null>(null);
  /** The offer's element, so it can be brought into view when it appears. */
  let offer = $state<HTMLElement | null>(null);
  /** The create press in flight. */
  let creating = $state(false);
  /** The engine's own words, when the create was refused (the folder was not empty). */
  let createError = $state<string | null>(null);

  // The offer is the sheet's answer to the press, and on a short window it
  // lands under the body's fold: bring it into view rather than leaving a
  // press that looks like it did nothing.
  $effect(() => {
    if (making !== null) offer?.scrollIntoView({ block: 'nearest' });
  });

  /**
   * Whether this window has the OS's own folder dialog at all. The sheet is
   * read in the browser harness too (it is a designed surface, and the gates
   * drive it), and a door that is not there should be *absent* rather than
   * present-and-refusing — `transport()` is the same test `pickDirectory` makes.
   */
  const canBrowse = transport() === 'tauri';

  /**
   * The rows: what the plans root holds, then the plans this machine remembers
   * — the same list the start screen draws (`onboarding/StartPlan.svelte`),
   * because this is the same question asked from the other side of the door.
   * A remembered path the root already lists appears once, and one whose folder
   * has since moved stays listed: pressing it is how the student finds out, in
   * the engine's own words.
   */
  const listed = $derived.by(() => {
    const rows = app.plans.map((plan) => ({
      name: plan.name,
      path: plan.path,
      folder: planFolder(plan.path),
    }));
    for (const path of app.recent) {
      if (rows.some((row) => row.path === path)) continue;
      rows.push({ name: path.split('/').pop() ?? path, path, folder: planFolder(path) });
    }
    return rows;
  });

  /** The plan this window is on, named — the sheet is about to change it. */
  const current = $derived.by(() => {
    const path = app.plan;
    if (!path) return null;
    const row = listed.find((entry) => entry.path === path);
    return { name: row?.name ?? (path.split('/').pop() ?? path), path };
  });

  /**
   * Open a folder that exists. The sheet closes on the plan that opened; a
   * folder the engine refuses is asked one question before it is answered —
   * *would `plan.new` take it?* — and only a yes becomes the new-plan offer.
   * A folder that holds a broken plan, or anything else, is refused in the
   * engine's own words, here rather than as a toast over the sheet.
   */
  async function open(path: string): Promise<void> {
    if (busy) return;
    making = null;
    createError = null;
    // A new attempt retires the last one's words: a refusal about the folder
    // tried a moment ago is not a fact about the one being opened now.
    app.toast = null;
    app.toastAction = null;
    if (path === app.plan) {
      onclose();
      return;
    }
    busy = true;
    try {
      // The sheet answers its own refusal: no toast for a probe that is about
      // to become a door. The open's own answer — not a comparison of two
      // spellings of the path — is what closes the sheet: a symlink, a
      // trailing separator or the engine's own canonical form would otherwise
      // leave it standing over the plan it just opened.
      const opened = await app.openPlan(path, { quiet: true });
      if (opened) {
        onclose();
        return;
      }
      const refusal = app.lastDiagnostic;
      if (await app.canCreatePlanAt(path)) making = path;
      else app.notice(refusal ?? `${path} could not be opened`);
    } finally {
      busy = false;
    }
  }

  /**
   * The OS's own folder dialog where there is one, read straight from the
   * platform (this sheet's own door — `openPlanFolder` swallows which folder
   * was chosen, and the offer needs the path). The chosen folder is opened
   * like a typed one: refusal by refusal, it either becomes the plan or the
   * offer to make one.
   */
  async function browse(): Promise<void> {
    const chosen = await pickDirectory();
    if (!chosen) {
      app.notice('this window has no folder dialog — type the plan path instead');
      return;
    }
    manual = chosen;
    await open(chosen);
  }

  /**
   * `plan.new` at the offered folder: the folder IS the plan (no name to ask
   * for). An empty folder becomes the blank plan and the sheet closes on it; a
   * folder that already holds something is refused in the engine's own words,
   * shown here rather than as a toast the sheet would cover.
   */
  async function create(): Promise<void> {
    if (making === null || creating) return;
    creating = true;
    createError = null;
    // The offer is the answer the previous refusal asked for: the old words go
    // with the press, and a new refusal is drawn beside the button.
    app.toast = null;
    app.toastAction = null;
    try {
      const opened = await app.createPlanAt(making);
      if (opened) onclose();
      else createError = app.lastDiagnostic ?? 'the plan could not be written';
    } finally {
      creating = false;
    }
  }

  /** The rows' pills stay live only while a switch can happen at all. */
  const blocked = $derived(busy || app.timer !== null);
</script>

<Sheet
  title="Change the plan folder"
  subtitle="A plan is one folder on this machine — opening one changes everything the window shows."
  icon="folder"
  wash="sky"
  {onclose}
>
  {#if current}
    <!-- Where the window is now. The card is the sheet's own answer to its own
         title, and it is the one row the student has to be able to read before
         deciding on another. -->
    <div class="pf-where">
      <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true"><Icon name="laptop" /></span>
      <span class="pf-where__text">
        <span class="pf-where__name">This window is on <b>{current.name}</b></span>
        <code class="pf-where__path">{current.path}</code>
      </span>
    </div>
  {/if}

  {#if app.timer}
    <p class="cd-sheet__hint">
      The session on {app.timer.item.label} belongs to this folder: stop it and the plan can
      change.
    </p>
  {/if}

  <!-- Which door. A segment, because it is a two-way exclusive choice and the
       chosen side is a card rather than ink: this sheet's one dark object is
       the row's own Open. -->
  <div class="pf-pick">
    <p class="pf-band__say">Select a folder</p>
    <div class="cd-segment cd-segment--card" role="radiogroup" aria-label="How to choose the folder">
      <button
        type="button"
        role="radio"
        aria-checked={door === 'browse'}
        onclick={() => (door = 'browse')}
      >Browse</button>
      <button
        type="button"
        role="radio"
        aria-checked={door === 'type'}
        onclick={() => (door = 'type')}
      >Type path</button>
    </div>
  </div>

  {#if door === 'browse' && listed.length > 0}
    <!-- The folders on this machine. Each row is a folder, and its whole left
         side is the press that opens it. -->
    <div class="pf-band">
      <div class="pf-rows">
        {#each listed as row (row.path)}
          <div class="pf-row" class:is-here={row.path === app.plan}>
            <button
              class="pf-row__open"
              type="button"
              data-command="app.changePlan"
              data-placement="app.window"
              disabled={blocked || row.path === app.plan}
              aria-label={`Open the plan in ${row.path}`}
              onclick={() => void open(row.path)}
            >
              <span class="cd-glyph cd-glyph--sm" data-w={row.path === app.plan ? 'sky' : undefined} aria-hidden="true">
                <Icon name="folder" />
              </span>
              <span class="pf-row__text">
                <span class="pf-row__name">{row.name}</span>
                <span class="pf-row__meta">
                  <code>{row.folder === '' ? 'plans/' : row.folder}</code>
                </span>
              </span>
            </button>
            {#if row.path === app.plan}
              <span class="pf-row__here">Open now</span>
            {:else}
              <button
                class="pf-row__go"
                type="button"
                data-command="app.changePlan"
                data-placement="app.window"
                disabled={blocked}
                aria-label={`Open the plan in ${row.path}`}
                onclick={() => void open(row.path)}
              >
                Open
              </button>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {:else if door === 'browse'}
    <div class="cd-empty">
      <div class="cd-empty__t">No plan is on this machine yet</div>
      <div class="cd-empty__s">An empty folder becomes a new blank plan.</div>
    </div>
  {/if}

  {#if door === 'browse'}
    <div class="pf-or" aria-hidden="true"><span>Or</span></div>
  {/if}

  <div class="pf-doors" class:is-open={door === 'type'}>
    <div class="pf-door">
      <div class="pf-door__well">
        <Icon name="folder" />
        <input
          id="pf-path"
          type="text"
          placeholder="Type or paste a folder path…"
          aria-label="Type or paste a folder path"
          autocomplete="off"
          spellcheck="false"
          bind:value={manual}
          onkeydown={(event) => {
            if (event.key === 'Enter' && manual.trim().length > 0) void open(manual.trim());
          }}
        />
      </div>
    </div>
    <div class="pf-acts">
      {#if door === 'type' || manual.trim().length > 0}
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          disabled={blocked || manual.trim().length === 0}
          onclick={() => void open(manual.trim())}
        >
          Open it
        </button>
      {/if}
      {#if canBrowse && door === 'browse'}
        <button class="cd-pill cd-pill--quiet" type="button" disabled={blocked} onclick={() => void browse()}>
          <Icon name="folder" size={14} />
          Choose a folder…
        </button>
      {/if}
    </div>
  </div>

  {#if making !== null}
    <!-- The refused folder, answered with a door (COMPOSER's own rule for a
         refusal: the next step stands beside it). The folder IS the plan — no
         name is asked for — and an empty one becomes the blank plan in one
         press; one that already holds something is refused in the engine's
         own words, here rather than behind the sheet. -->
    <div class="pf-new" bind:this={offer}>
      <div class="pf-new__head">
        <span class="cd-glyph cd-glyph--sm" data-w="mint" aria-hidden="true"><Icon name="plus" /></span>
        <span>
          <span class="pf-new__title">Make a new plan here</span>
          <code class="pf-new__where">{making}</code>
        </span>
      </div>
      <p class="cd-sheet__hint">
        A folder that holds nothing becomes the plan; one that already holds something is
        refused rather than overwritten.
      </p>
      <div class="pf-acts">
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          disabled={creating || app.timer !== null}
          onclick={() => void create()}
        >
          {creating ? 'Writing the plan…' : 'Create a blank plan'}
        </button>
      </div>
      {#if createError}
        <p class="pf-refusal">{createError}</p>
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
      <Icon name="terminal" />
    </span>
    <span class="cd-sheet__hint">
      In a terminal, <code class="cd-code">sam plans</code> lists these folders and
      <code class="cd-code">--plan &lt;folder&gt;</code> reads the one you name.
    </span>
    <span class="cd-sheet__spacer"></span>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Close</button>
  {/snippet}
</Sheet>

<style>
  /* ── where the window is now ─────────────────────────────────────────────
     The card the sheet is really about. A well, not a floating layer: it is a
     *statement about the present*, and the press that changes it is in the list
     below. The path is mono and selectable, because a folder is an address and
     a student may need to copy it. */
  .pf-where {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm);
    border-radius: var(--r-tile);
    background: var(--well);
  }
  .pf-where__text { display: grid; gap: 1px; min-width: 0; }
  .pf-where__name { font-size: var(--text-xs); color: var(--ink-2); }
  .pf-where__name b { color: var(--ink); font-weight: var(--weight-label); }
  .pf-where__path {
    font-family: var(--font-mono, ui-monospace);
    font-size: var(--text-2xs);
    color: var(--ink-3);
    overflow-wrap: anywhere;
    user-select: text;
  }

  /* ── the folders ─────────────────────────────────────────────────────────
     One row per plan. The row is a hairline card; the press is its whole left
     side, so the pointer target is the folder and its name rather than a 60px
     pill, and the button at the trailing edge is the same write for a hand that
     arrived at the right edge first. */
  /* The label and the switcher on one line: "which door?" and "which one is
     open?" are one question, and the reference draws them together. */
  .pf-pick {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  /* Sentence case, at the reference's own weight: the label asks a question and
     the switcher answers it, so it is not a micro-caps section head. */
  .pf-pick .pf-band__say {
    margin-right: auto;
    font-size: var(--text-sm);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-body);
    text-transform: none;
    color: var(--ink);
  }

  .pf-band { display: grid; gap: var(--space-2xs); }
  .pf-band__say {
    margin: 0;
    font-size: var(--text-2xs);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .pf-rows { display: grid; gap: var(--space-2xs); }
  .pf-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-xs);
    padding-right: var(--space-xs);
    border: 1px solid var(--rule);
    border-radius: var(--r-mini);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .pf-row:hover { background: var(--card); box-shadow: var(--sh-1); }
  /* The folder this window is already on: its own room, and a statement where
     the button would be. Never disabled-looking — it is not refused, it is
     where the student already is. */
  .pf-row.is-here { background: var(--well); }
  .pf-row.is-here:hover { box-shadow: none; }

  .pf-row__open {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
    min-height: calc(var(--hit) + var(--space-sm));
    padding: var(--space-xs);
    border-radius: var(--r-mini);
    text-align: left;
  }
  .pf-row__open[disabled] { pointer-events: none; }
  .pf-row__text { display: grid; gap: 1px; min-width: 0; }
  .pf-row__name {
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink);
    overflow-wrap: anywhere;
  }
  .pf-row__meta { font-size: var(--text-2xs); color: var(--ink-3); }
  .pf-row__meta code { font-family: var(--font-mono, ui-monospace); font-size: var(--text-2xs); }
  /* The row's one affirmative act, as a card pill behind a hairline ring — the
     same chosen object the segment and the lead segment wear, and never ink:
     the sheet's single dark object is its own commit, and a folder row is not
     it. It steps forward against the hairline it sits in. */
  .pf-row__go {
    height: var(--hit);
    padding: 0 var(--space-md);
    border-radius: var(--r-pill);
    background: var(--card);
    color: var(--ink);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px var(--rule-strong);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .pf-row__go:hover { box-shadow: inset 0 0 0 1.5px var(--ink-4), var(--sh-1); }
  .pf-row.is-here .pf-row__go { box-shadow: none; }
  .pf-row__go[disabled] { opacity: 0.45; pointer-events: none; }
  .pf-row__here {
    padding: 0 var(--space-sm);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    color: var(--ink-3);
    white-space: nowrap;
  }

  /* ── the two doors that are not a list ───────────────────────────────────
     A rule with a word on it, which is the oldest way to say "either of these"
     and the one that needs no explaining. */
  .pf-or {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--text-2xs);
  }
  .pf-or::before,
  .pf-or::after {
    content: '';
    height: 1px;
    background: var(--rule);
  }
  .pf-or span { color: var(--ink-3); }

  .pf-doors { display: grid; gap: var(--space-xs); }
  /* The open door leads: with Browse chosen the field is the narrower
     alternative under the "Or", and with Type path chosen it is the sheet's own
     width and the list is gone — which is the whole point of the switcher. */
  .pf-doors.is-open .pf-door__well { height: var(--pill-h-lg); }
  .pf-door { display: grid; gap: var(--space-2xs); min-width: 0; }
  /* The field is the catalog's search well and the icon grid's find well: one
     shape for "type here", carrying the glyph that says what is being typed. */
  .pf-door__well {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    height: var(--pill-h);
    padding: 0 var(--space-sm);
    border-radius: var(--r-pill);
    background: var(--well);
    color: var(--ink-3);
    transition: background var(--dur-1) var(--ease), box-shadow var(--dur-1) var(--ease);
  }
  .pf-door__well:focus-within {
    background: var(--card);
    box-shadow: inset 0 0 0 1.5px var(--ink-4);
  }
  .pf-door__well input {
    flex: 1 1 auto;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--ink);
    font: inherit;
    font-size: var(--text-xs);
  }
  .pf-door__well input:focus { outline: none; }
  .pf-door__well input::placeholder { color: var(--ink-4); }

  .pf-acts {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  /* ── the new-plan offer ──────────────────────────────────────────────────
     The refused folder's own address, then the one press, on the sheet's own
     ground — a section, not a floating layer. Its tile is the mint room,
     because this is the one *constructive* act on a sheet about switching. */
  .pf-new {
    display: grid;
    gap: var(--space-xs);
    padding-top: var(--space-sm);
    border-top: 1px solid var(--rule);
  }
  .pf-new__head {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: var(--space-sm);
  }
  .pf-new__title {
    display: block;
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
    color: var(--ink);
  }
  .pf-new__where {
    display: block;
    margin-top: 1px;
    font-family: var(--font-mono, ui-monospace);
    font-size: var(--text-2xs);
    color: var(--ink-3);
    overflow-wrap: anywhere;
  }
  /* The engine's refusal, in the register's readable pair — the same pairing
     the start screen's own refusal wears. */
  .pf-refusal {
    margin: 0;
    justify-self: start;
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--r-item);
    background: var(--chip-overdue);
    color: var(--on-overdue);
    font-size: var(--text-xs);
    font-weight: var(--weight-label);
  }

  @media (prefers-reduced-motion: reduce) {
    .pf-row,
    .pf-row__go,
    .pf-door__well { transition: none; }
  }
</style>
