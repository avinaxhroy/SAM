<!--
Screen III of the room (R11): get started, which here means *get the folder*.

At first run the press is the screen's work and the screen is built around it:
what the plan will be called, where it will be written, and one filled pill that
writes it and opens it (`plan.new` — the bundled schema, no configuration, no
wizard). Both answers the app needs are the student's: the folder name and the
folder it goes in, and the second one defaults to the app's own plans root, so
the press works before anything is typed. An existing plan is the other half of
the door — typed, picked, or pressed from the list — which is why the screen
says *or*.

**Over a working plan the form is not drawn.** The room re-read over a plan is a
document, and making a second plan there would be the app's first door that ends
a running session to change what is open — every screen's state (a draft, a
panel, the timer) is read once for the plan that is loaded, and this screen will
not be the one that quietly drops it. What is left is the movement's own
sentences, which is what a document being re-read is for.

The presets are one text link away and nothing else: the app's own plan is the
default, a shipped plan is a deliberate reach (`plan.new --preset`), and a list
of six plans in front of the first press would make the first press a choice.
-->
<script lang="ts">
  import { app } from '../session.svelte';
  import { BUNDLED_STARTS } from '../shell/presets';
  import { BLANK_NAME, planFacts, planFolder } from './plan';
  import Icon from '../shell/Icon.svelte';

  /** The name the press starts from — the app's own default, editable. */
  let name = $state(BLANK_NAME);
  /** The folder the student already has, as typed. */
  let manual = $state('');
  /** The presets are folded away until asked for. */
  let presets = $state(false);
  /** The press in flight: a start's id, `blank`, or `open`. */
  let pending = $state<string | null>(null);
  /** The engine's own words, when a write or an open was refused. */
  let refusal = $state<string | null>(null);

  /** The storage door's answer, else the app's own plans root. */
  const parent = $derived(app.planParent);
  /**
   * The rows: what the plans root holds, then the plans this machine remembers
   * opening — the memory is the only thing that keeps a plan kept *outside* the
   * root one press away the next morning (`session.recent`, R11). A remembered
   * path the root already lists appears once, and one whose folder has since
   * moved stays listed: pressing it is how the student finds out, in the
   * engine's own words, rather than wondering where it went.
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
  /** Where the press will write, as a place: the last two rooms of the path. */
  const dest = $derived(
    [tail(parent ?? app.plansDir), name.trim().length > 0 ? name.trim() : null]
      .filter((part) => part !== null)
      .join('/') + '/',
  );

  /** The last two rooms of a path — what tells two destinations apart. */
  function tail(path: string): string {
    const parts = path.split('/').filter((part) => part.length > 0);
    return parts.slice(-2).join('/');
  }

  /**
   * `plan.new` — one press, one folder, and the app opens what it wrote. The
   * name is handed over *as a name*: the engine's own rule refuses a separator,
   * in the engine's own words, and this screen shows them rather than guessing
   * at a path.
   */
  async function create(planName: string, preset: string): Promise<void> {
    pending = preset;
    refusal = null;
    try {
      await app.createPlan(planName, preset, parent);
      if (!app.plan) refusal = app.lastDiagnostic ?? 'the plan could not be written';
    } finally {
      pending = null;
    }
  }

  /** Open a plan that already exists — the folder becomes the plan. */
  async function open(path: string): Promise<void> {
    pending = 'open';
    refusal = null;
    try {
      await app.openPlan(path);
      if (!app.plan) refusal = app.lastDiagnostic ?? `no plan at ${path}`;
    } finally {
      pending = null;
    }
  }

  /**
   * The storage door: the OS's own folder dialog where there is one, and the
   * app's plans root everywhere else — `choosePlanParent` leaves the choice
   * alone when no dialog ran, so a window without one keeps the working default
   * rather than a half-made one.
   */
  async function chooseFolder(): Promise<void> {
    if (await app.choosePlanParent()) return;
    app.notice('this window has no folder dialog — the plan goes in your plans folder');
  }

  /** The OS's own folder dialog, for a plan that already exists. */
  async function browse(): Promise<void> {
    if (await app.openPlanFolder()) return;
    app.notice('this window has no folder dialog — type the plan path instead');
  }
</script>

<div class="ob-start">
  <div class="ob-say">
    {#if app.plan === null}
      <h1 class="ob-h1" tabindex="-1">Make the plan.</h1>
      <p class="ob-lede">
        A plan is one folder of plain files on this machine: your courses, topics, resources
        and review dates, in files you can read. Name it, and SAM writes a whole plan into
        that folder and opens it: courses, topics, and the rules that space your reviews.
      </p>
    {:else}
      <h1 class="ob-h1" tabindex="-1">Now work the plan.</h1>
      <p class="ob-lede">
        The plan is open, and everything this document promised happens on the screens you
        already have — there is nothing else to set up.
      </p>
    {/if}
  </div>

  {#if app.plan === null}
    <section class="ob-create">
      <div class="ob-field">
        <label class="ob-label" for="ob-plan-name">Plan name</label>
        <input
          id="ob-plan-name"
          class="ob-input"
          autocomplete="off"
          spellcheck="false"
          bind:value={name}
        />
      </div>

      <p class="ob-where-row">
        Written to
        <span class="ob-dest">{dest}</span>
        <button class="ob-link" type="button" onclick={() => void chooseFolder()}>
          {parent ? 'Change the folder…' : 'Choose a folder…'}
        </button>
        {#if parent}
          <button class="ob-link" type="button" onclick={() => (app.planParent = null)}>
            Use the plans folder
          </button>
        {/if}
      </p>

      <div class="ob-acts">
        <button
          class="cd-pill cd-pill--lg"
          type="button"
          data-command="plan.new"
          data-placement="app.planPicker"
          disabled={pending !== null}
          onclick={() => void create(name, 'blank')}
        >
          {pending === 'blank' ? 'Writing the folder…' : 'Create plan'}
        </button>
        {#if pending !== null && pending !== 'blank' && pending !== 'open'}
          <span class="ob-busy">Copying {pending} into your plan folder…</span>
        {/if}
      </div>

      <!-- The presets, one deliberate reach away: `aria-expanded` on the link,
           and the list under it — the screen above it does not change. -->
      <button
        class="ob-link"
        type="button"
        aria-expanded={presets}
        aria-controls="ob-presets"
        onclick={() => (presets = !presets)}
      >
        <Icon name="chevronright" size={12} />
        Check the presets
      </button>

      {#if presets}
        <div class="ob-presets" id="ob-presets">
          {#each BUNDLED_STARTS as start (start.id)}
            <button
              class="ob-preset"
              type="button"
              data-command="plan.new"
              data-placement="app.planPicker"
              disabled={pending !== null}
              onclick={() => void create(start.title, start.id)}
            >
              <span>
                <span class="ob-preset__title">{start.title}</span>
                <p class="ob-preset__line">{start.line}</p>
              </span>
              <span class="ob-preset__facts"
                >{planFacts(start.work, start.workLabel, start.minutes)}</span
              >
            </button>
          {/each}
          <p class="ob-note">
            A preset is a whole plan the app ships — courses, topics and their rules already
            in it — copied into your folder and yours to edit from the first screen onwards.
            The same plans are one command away in a terminal (<code class="cd-code">SAM
            plan.new --preset jee</code>).
          </p>
        </div>
      {/if}
    </section>

    <section class="ob-open">
      <p class="ob-label" id="ob-open-lead">Or open one you already have</p>
      <div class="ob-field">
        <input
          class="ob-input"
          aria-labelledby="ob-open-lead"
          placeholder="the folder your plan lives in"
          autocomplete="off"
          spellcheck="false"
          bind:value={manual}
          onkeydown={(event) => {
            if (event.key === 'Enter' && manual.trim().length > 0) void open(manual.trim());
          }}
        />
      </div>
      <div class="ob-acts">
        <button
          class="cd-pill cd-pill--quiet"
          type="button"
          disabled={pending !== null}
          onclick={() => void browse()}
        >
          Open folder…
        </button>
        {#if manual.trim().length > 0}
          <button
            class="cd-pill cd-pill--quiet"
            type="button"
            disabled={pending !== null}
            onclick={() => void open(manual.trim())}
          >
            {pending === 'open' ? 'Opening…' : 'Open it'}
          </button>
        {/if}
      </div>

      {#if listed.length > 0}
        <ul class="ob-rows">
          {#each listed as plan (plan.path)}
            <li>
              <button
                class="ob-row"
                type="button"
                disabled={pending !== null}
                onclick={() => void open(plan.path)}
              >
                <span class="ob-row__name">{plan.name}</span>
                <span class="ob-row__where">{plan.folder}</span>
                <span class="ob-row__go"><Icon name="chevronright" size={14} /></span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <p class="ob-note">
      Learn, prove, anchor — that is the plan's own loop. The session is the learning, your
      press is the proof, and the grade is the anchor: the plan works out the date it comes
      back from it.
    </p>
  {:else}
    <div class="ob-hand">
      <p>
        <b>Today</b> — the day's queue. Its empty card names the first thing to add, in the
        plan's own words, and every row on it opens the work.
      </p>
      <p>
        <b>The row's card</b> — press a topic and the session starts on it; the minutes it
        took are written back onto the row, and the resource it names opens from it.
      </p>
      <p>
        <b>Reviews</b> — grade the recall honestly and the plan sets the next date itself:
        back in a day, in a week, in a month. Miss one and the topic returns, already on the
        day you are looking at.
      </p>
    </div>
  {/if}

  {#if refusal}
    <p class="ob-refusal">{refusal}</p>
  {/if}
</div>
