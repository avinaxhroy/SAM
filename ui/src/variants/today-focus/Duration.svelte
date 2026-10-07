<!-- Session duration editor capsule (.cd-dur) with 5–180 minute validation bounds. -->
<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '../../shell/Icon.svelte';

  let {
    minutes,
    onCommit,
    raised = false,
  }: {
    minutes: number;
    onCommit: (value: number) => void;
    raised?: boolean;
  } = $props();

  const DUR_MIN = 5;
  const DUR_MAX = 180;

  let edit = $state(false);
  let bad = $state(false);
  let input = $state<HTMLInputElement | null>(null);
  let act = $state<HTMLButtonElement | null>(null);

  /** The digit string as typed, so a refused value keeps its digits. */
  let typed = $state('');

  /** A new length from the panel lands in the field as typed text. */
  $effect(() => {
    if (!edit) typed = String(minutes);
  });

  function commit(): boolean {
    const value = Number(typed.replace(/[^0-9]/g, ''));
    const refused = typed === '' || !Number.isFinite(value) || value < DUR_MIN || value > DUR_MAX;
    bad = refused;
    if (!refused) onCommit(value);
    return !refused;
  }

  async function open(): Promise<void> {
    edit = true;
    bad = false;
    await tick();
    input?.focus();
    input?.select();
  }

  /** A close the person asked for hands the ring back to the cap they were
      using; a close caused by leaving must not pull focus back to where they
      just left. */
  async function close(refocus: boolean): Promise<void> {
    edit = false;
    if (!refocus) return;
    await tick();
    act?.focus();
  }

  function toggle(): void {
    if (edit) {
      if (commit()) void close(true);
      return; // a refused value keeps the editor open
    }
    void open();
  }
</script>

<span class="cd-dur" data-edit={edit ? '1' : '0'} data-bad={bad ? '1' : '0'} class:raised>
  <span class="cd-dur__part cd-dur__part--v">
    <input
      bind:this={input}
      type="text"
      inputmode="numeric"
      maxlength="3"
      value={typed}
      tabindex={edit ? 0 : -1}
      aria-label="Session length in minutes"
      aria-invalid={bad ? 'true' : undefined}
      oninput={(event) => {
        const clean = (event.currentTarget as HTMLInputElement).value.replace(/[^0-9]/g, '').slice(0, 3);
        typed = clean;
      }}
      onblur={() => {
        if (edit && commit()) void close(false);
      }}
      onkeydown={(event) => {
        if (event.key !== 'Enter') return;
        event.preventDefault();
        if (commit()) void close(true);
      }}
    />
    <span class="cd-dur__unit">min</span>
  </span>
  <button
    class="cd-dur__part cd-dur__part--act"
    type="button"
    bind:this={act}
    aria-label={edit ? 'Save the session length' : 'Change the session length'}
    onclick={toggle}
  >
    <span class="cd-dur__ico cd-dur__ico--pencil"><Icon name="pencil" size={15} /></span>
    <span class="cd-dur__ico cd-dur__ico--check"><Icon name="check" size={15} /></span>
  </button>
</span>

<style>
  .cd-dur__part--act .cd-dur__ico {
    transition:
      transform var(--dur-1) var(--ease),
      opacity var(--dur-2) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .cd-dur__part--act:hover .cd-dur__ico {
    color: var(--ink);
  }
  .cd-dur[data-edit='0'] .cd-dur__part--act:active .cd-dur__ico--pencil {
    transform: scale(0.86);
  }
  .cd-dur[data-edit='1'] .cd-dur__part--act:active .cd-dur__ico--check {
    transform: scale(0.86);
  }
  .cd-dur[data-bad='1'] .cd-dur__part--v {
    box-shadow: inset 0 0 0 2px var(--on-risk);
  }
  /* variant A's surface: the capsule is raised off the track it sits in */
  .cd-dur.raised .cd-dur__part {
    background: var(--card);
  }
  .cd-dur.raised .cd-dur__part--act:hover {
    background: var(--well);
  }
</style>
