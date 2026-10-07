<!--
  The plan's own folder, in a sheet (`app.changePlan` — the titlebar's folder
  door, the menu, the palette, and `{"id":"app.changePlan","params":{"path":…}}`
  over the CLI's `sam invoke`).

  This is the picker's material over a working plan: the plans on this machine,
  the ones this window remembers, the OS's own folder dialog, and a typed path
  for a window that has none. Every press goes through the same
  `session.openPlan` the start screen uses, so a folder that is not a plan is
  refused in the engine's own words rather than silently ignored.

  A running (or paused) session is the one thing that stops a switch: the timer
  belongs to the folder it was started in, and `session.openPlan` refuses — in
  the sentence the app already uses when a session is asked to start over a
  running one. The sheet says so where the student can read it, rather than
  letting every press land as the same refusal.
-->
<script lang="ts">
  import Sheet from './Sheet.svelte';
  import { app } from '../session.svelte';
  import { planFolder } from '../onboarding/plan';

  let { onclose }: { onclose: () => void } = $props();

  /** A typed path, for a window with no folder dialog. */
  let manual = $state('');
  /** A press in flight — one at a time, whichever door it came from. */
  let busy = $state(false);

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

  /** Open a folder that exists. The sheet closes on the plan that opened. */
  async function open(path: string): Promise<void> {
    if (busy) return;
    if (path === app.plan) {
      onclose();
      return;
    }
    busy = true;
    try {
      await app.openPlan(path);
      if (app.plan === path) onclose();
    } finally {
      busy = false;
    }
  }

  /**
   * The OS's own folder dialog where there is one. `openPlanFolder` answers
   * whether a dialog ran at all — a window without one keeps its typed field
   * rather than reading a half-made choice — and the open it attempted reports
   * itself, in the engine's words, on the same toast every other refusal uses.
   */
  async function browse(): Promise<void> {
    const was = app.plan;
    if (!(await app.openPlanFolder())) {
      app.notice('this window has no folder dialog — type the plan path instead');
      return;
    }
    if (app.plan !== was) onclose();
  }

  /** The rows' pills stay live only while a switch can happen at all. */
  const blocked = $derived(busy || app.timer !== null);
</script>

<Sheet
  title="Change the plan folder"
  subtitle="A plan is one folder on this machine — opening one changes everything the window shows"
  {onclose}
>
  {#if current}
    <p class="pf-on">
      This window is on <b>{current.name}</b> — <code class="cd-code">{current.path}</code>
    </p>
  {/if}

  {#if app.timer}
    <p class="cd-sheet__hint">
      The session on {app.timer.item.label} belongs to this folder: stop it and the plan can
      change.
    </p>
  {/if}

  {#if listed.length === 0}
    <div class="cd-empty">
      <div class="cd-empty__t">No plan is on this machine yet</div>
      <div class="cd-empty__s">
        Making one is the start screen's work — this sheet opens folders that already hold a
        plan.
      </div>
    </div>
  {:else}
    <div class="cd-tasks">
      {#each listed as row (row.path)}
        <div class="cd-task pf-row">
          <div>
            <div class="cd-task__title">{row.name}</div>
            <div class="cd-task__meta">
              <span>{row.folder}</span>
              {#if row.path === app.plan}
                <span>open now</span>
              {/if}
            </div>
          </div>
          <div class="cd-task__right">
            <button
              class="cd-pill cd-pill--quiet cd-pill--sm"
              type="button"
              data-command="app.changePlan"
              data-placement="app.window"
              disabled={blocked}
              aria-label={`Open the plan in ${row.path}`}
              onclick={() => void open(row.path)}
            >
              Open
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="cd-sheet__field">
    <label class="cd-sheet__label" for="pf-path">Or type a folder</label>
    <input
      id="pf-path"
      class="cd-sheet__well"
      type="text"
      placeholder="the folder a plan lives in"
      autocomplete="off"
      spellcheck="false"
      bind:value={manual}
      onkeydown={(event) => {
        if (event.key === 'Enter' && manual.trim().length > 0) void open(manual.trim());
      }}
    />
  </div>

  <div class="pf-acts">
    <button class="cd-pill cd-pill--quiet" type="button" disabled={blocked} onclick={() => void browse()}>
      Choose a folder…
    </button>
    {#if manual.trim().length > 0}
      <button
        class="cd-pill cd-pill--quiet"
        type="button"
        disabled={blocked}
        onclick={() => void open(manual.trim())}
      >
        Open it
      </button>
    {/if}
  </div>

  {#snippet footer()}
    <span class="cd-sheet__hint">
      In a terminal, <code class="cd-code">sam plans</code> lists these folders and
      <code class="cd-code">--plan &lt;folder&gt;</code> reads the one you name.
    </span>
    <span class="cd-sheet__spacer"></span>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Close</button>
  {/snippet}
</Sheet>

<style>
  /* The lead: where the window is now, before it moves. Mono for the path, so
     it reads as an address and can be selected as one. */
  .pf-on {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--ink-2);
    overflow-wrap: anywhere;
  }
  .pf-on b {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  /* The row grid without a state dot: `.cd-task`'s template starts with an
     `auto` column for the dot the Today screen draws, and a plan row has no
     state to dot. */
  .pf-row {
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .pf-acts {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
</style>
