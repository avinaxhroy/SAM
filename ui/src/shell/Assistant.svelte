<!--
  The assistant deck (§4.7's in-app door; UI_SPEC "The assistant deck and the
  working veil").

  One console on the floor, shaped like the reference the owner supplied: the
  sparkle and the name, then the two pills that decide the turn — the **model**
  the CLI should use and the **CLI** itself — and the close mark. The prompt is
  one big well with a circular send (which becomes the stop while a turn runs).

  The deck says nothing about the run's state. That is the island's job
  (`shell/Island.svelte`): the window's head carries the ink pill, the process
  is behind its press, and the deck is free of the live line it used to draw
  loose in this card — both of them are chrome for one run, and drawing the same
  state twice is how a deck starts looking like a log viewer.

  The deck's material is the rail's, at panel strength (`--pane` over the
  backdrop blur, plus `--rim-glass`): with the veil up, the washes read through
  the panel instead of being covered by a sheet.

  There is no paperclip: SAM has no attachment path into an agent CLI, and a
  control that does nothing would be a lie. Everything the agent should know
  about the window rides the prompt instead (the app-context block).

  Blockers are drawn where the prompt would be, never hidden: no plan open, no
  CLI detected, no `sam` binary, or a browser harness with no machine behind it.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import ModelPicker from './ModelPicker.svelte';
  import { assistant, buildAppContext } from '../assistant.svelte';
  import { app } from '../session.svelte';
  import { transport } from '../ipc';
  import '../styles/assistant.css';

  let field = $state<HTMLTextAreaElement | null>(null);
  let customField = $state<HTMLInputElement | null>(null);

  const desktop = transport() === 'tauri';
  const agents = $derived(assistant.available());
  const nothingInstalled = $derived(desktop && assistant.agents !== null && agents.length === 0);

  const blockers = $derived.by<string[]>(() => {
    const lines: string[] = [];
    if (!desktop) {
      lines.push('The assistant runs the coding agents installed on this machine — open the desktop app.');
    }
    if (app.plan === null) {
      lines.push('Open a plan first: the agent works inside the plan this window has open.');
    }
    if (nothingInstalled) {
      lines.push('No agent CLI found on this machine. Install one of these, then reopen this deck:');
    }
    if (assistant.sam?.available === false) {
      lines.push('The `sam` CLI was not found next to this app or on your PATH — the agent writes the plan through it.');
    }
    if (assistant.profilesError) {
      lines.push(assistant.profilesError);
    }
    return lines;
  });

  /** Focus the field when the deck opens idle (a run focuses nothing). */
  $effect(() => {
    if (assistant.open && !assistant.busy()) field?.focus();
  });

  /** The custom-model field claims focus the moment it appears. */
  $effect(() => {
    if (assistant.modelOption === '__custom__') customField?.focus();
  });

  // One handler for the whole deck, and the field does not carry its own: a
  // keydown on the textarea bubbles here, so a second listener on the field
  // would run the same Enter twice — two turns for one press, because `running`
  // only flips once the spawn answers.
  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.stopPropagation();
      assistant.close();
      return;
    }
    // Enter runs the turn from the field, and from the field only: a control
    // in the head (the pills) owns its own Enter, or the deck would submit the
    // prompt instead of the press the student made. `isComposing`/229 is the
    // IME: Enter there accepts a candidate, and sending the half-composed text
    // would start a real, auto-approved turn on it.
    if (
      event.key === 'Enter' &&
      !event.shiftKey &&
      !event.isComposing &&
      event.keyCode !== 229 &&
      !assistant.busy() &&
      event.target === field
    ) {
      event.preventDefault();
      void submit();
    }
  }

  async function submit(): Promise<void> {
    await assistant.run(app.plan, buildAppContext(app));
  }
</script>

<section class="cd-deck" role="region" aria-label="Assistant" onkeydown={onkeydown}>
  <header class="cd-deck__head">
    <span class="cd-deck__mark" aria-hidden="true"><Icon name="sparkle" size={16} /></span>
    <h2 class="cd-deck__title">Assistant</h2>

    <span class="cd-deck__spacer"></span>

    <ModelPicker
      label={assistant.modelLabel()}
      value={assistant.modelOption}
      models={assistant.models}
      source={assistant.modelSource}
      loading={assistant.modelsLoading}
      disabled={assistant.running || assistant.agentId === null}
      onPick={(option) => assistant.setModelOption(option)}
      onOpen={() => void assistant.ensureModels()}
    />

    {#if assistant.modelOption === '__custom__'}
      <input
        class="cd-deck__custom"
        bind:this={customField}
        value={assistant.customModel}
        placeholder="the model id the CLI accepts"
        aria-label="A model id for the chosen CLI"
        disabled={assistant.running}
        oninput={(event) => assistant.setCustomModel(event.currentTarget.value)}
      />
    {/if}

    {#if agents.length > 0}
      <label class="cd-deck__pill" class:is-disabled={assistant.running}>
        <Icon name="terminal" size={13} />
        <select
          class="cd-deck__select"
          value={assistant.agentId ?? ''}
          disabled={assistant.running}
          aria-label="Which agent CLI to run"
          onchange={(event) => assistant.choose(event.currentTarget.value || null)}
        >
          {#each agents as agent (agent.id)}
            <option value={agent.id}>
              {agent.name}{agent.version ? ` · ${agent.version}` : ''}
            </option>
          {/each}
        </select>
        <span class="cd-deck__chev" aria-hidden="true"><Icon name="chevronDown" size={12} /></span>
      </label>
    {/if}

    <button
      class="cd-iconbtn cd-deck__close"
      type="button"
      aria-label="Close the assistant"
      onclick={() => assistant.close()}
    >
      <Icon name="close" size={14} />
    </button>
  </header>

  {#if blockers.length > 0}
    <div class="cd-deck__blocked">
      {#each blockers as line}
        <p class="cd-hint">{line}</p>
      {/each}
      {#if nothingInstalled && assistant.agents}
        <ul class="cd-deck__installs">
          {#each assistant.agents as agent (agent.id)}
            <li>
              <span class="cd-deck__installname">{agent.name}</span>
              <span class="cd-hint">{agent.installHint}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if assistant.receipt}
    <div class="cd-deck__receipt">
      <span class="cd-deck__receipttext">{assistant.receipt.summary}</span>
      <span class="cd-deck__txid">{assistant.receipt.txid}</span>
      <button class="cd-pill cd-pill--sm" type="button" onclick={() => void assistant.undo()}>
        Undo
      </button>
    </div>
  {/if}

  <div class="cd-deck__fieldwrap">
    <!-- A prompt to a coding agent is not prose to spell: the platform's
         inline checker runs on every word boundary (the space press), and its
         correction popover can swallow the next key. Every other typing well
         in this app says the same. -->
    <textarea
      class="cd-deck__field"
      bind:this={field}
      bind:value={assistant.prompt}
      rows="2"
      placeholder="Tell the assistant what to do…"
      aria-label="What should the agent do"
      disabled={assistant.busy() || !desktop}
      spellcheck="false"
      autocorrect="off"
      autocapitalize="off"
      autocomplete="off"
    ></textarea>
    {#if assistant.busy()}
      <button
        class="cd-deck__send is-stop"
        type="button"
        aria-label="Stop the turn"
        onclick={() => void assistant.cancel()}
      >
        <Icon name="stopSquare" size={16} />
      </button>
    {:else}
      <button
        class="cd-deck__send"
        type="button"
        aria-label="Run the turn"
        disabled={!assistant.canRun(app.plan)}
        onclick={() => void submit()}
      >
        <Icon name="arrowUp" size={16} />
      </button>
    {/if}
  </div>
</section>
