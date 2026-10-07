<!--
  THE CONTROL FAMILY, AT THE VALUE (record panel port, 2026-09-29).

  Every design's value is written through this one component, and it is the
  app's own family — `design/controls.md` §4, the same one
  `records/FieldControl.svelte` renders in the table:

    duration  the four chips `15m · 25m · 45m · 60m` and an exact field
    date      the day control with `Today · +1 week · +1 month`
    number    the two stepper halves and an exact field

  THE WRITE IS ONE DOOR. `onSet` is the screen's `record.setField`, dispatched
  by `records/RecordDetail.svelte` exactly as the table's cells dispatch it, and
  this component never writes anything itself. A **derived** value (`formula`,
  `progress`) never gets an editor at all — it is computed by the kind and
  printed with its `ƒ` mark, so no design can even offer to write it.

  What this component owns is the feedback the app already owns: the three words
  at the value it was about (`Saving… · Saved · Not saved`,
  `RecordTable.svelte:452-455`, as a `status` live region so a save is announced
  and not only a refusal), and a refusal in the engine's own sentence with the
  address it was about, marked `data-developer` as the file door's own line is.

  It draws no plate, no key and no value face — the design does that, and this
  sits at the value the design chose.
-->
<script lang="ts">
  import { minutes, plusDays, type FieldFact } from './props';

  let {
    field,
    address,
    today,
    open = false,
    onSet,
  }: {
    field: FieldFact;
    /** The record's own line in the plan's files — for the refusal's address. */
    address: string;
    /** The plan's own today, which the date offsets count from. */
    today: string | null;
    /** The design's own switch: the editor this component draws is open. */
    open: boolean;
    onSet: (key: string, value: string | string[] | null) => Promise<string | null>;
  } = $props();

  /** The four the app offers for a duration (`FieldControl.DURATIONS`). */
  const DURATIONS = [15, 25, 45, 60];

  let draft = $state('');
  let busy = $state(false);
  /** `save` · `ok` (for a hold) · `fail` (until the next write). */
  let written = $state<'save' | 'ok' | 'fail' | null>(null);
  let refusal = $state<string | null>(null);
  let holdTimer: ReturnType<typeof setTimeout> | null = null;

  // The editor starts from what the plan holds, each time it is opened.
  $effect(() => {
    if (open) draft = field.raw;
  });

  function announce(state: 'save' | 'ok' | 'fail'): void {
    if (holdTimer !== null) clearTimeout(holdTimer);
    written = state;
    if (state === 'ok') {
      holdTimer = setTimeout(() => {
        if (written === 'ok') written = null;
      }, 1400);
    }
  }

  /** One dispatch, then the app's own three words at this value. */
  async function send(value: string | string[] | null): Promise<void> {
    if (busy) return;
    refusal = null;
    announce('save');
    busy = true;
    const why = await onSet(field.key, value);
    busy = false;
    if (why === null) {
      announce('ok');
      return;
    }
    announce('fail');
    refusal = why;
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      void send(draft);
    } else if (event.key === 'Escape') {
      // Escape abandons the draft and leaves the value as the plan holds it —
      // the same rule `FieldControl`'s cell follows.
      draft = field.raw;
    }
  }

  function focusOnMount(node: HTMLInputElement): void {
    node.focus();
    // `select()` throws on a type that has no selection (a day control), so it
    // is asked for only where the app's own family asks for it.
    if (node.type !== 'date') node.select();
  }

  const chip = $derived(
    written === 'save' ? 'Saving…' : written === 'ok' ? 'Saved' : written === 'fail' ? 'Not saved' : null,
  );
</script>

<div class="rp-writer" data-field-key={field.key} data-command="record.setField">
  {#if open && !field.derived}
    <div class="rp-ctl" role="group" aria-label={`${field.label} — the value`}>
      {#if field.type === 'duration'}
        {#each DURATIONS as choice (choice)}
          <button
            class="rp-chipbtn"
            type="button"
            aria-pressed={field.amount === choice}
            onclick={() => void send(String(choice))}
          >
            {minutes(choice)}
          </button>
        {/each}
        <span class="rp-unit">
          <input
            class="rp-num"
            type="number"
            min="0"
            step="5"
            inputmode="numeric"
            bind:value={draft}
            onkeydown={keydown}
            use:focusOnMount
            aria-label={`${field.label} in minutes`}
          />
          <span class="rp-keycap">min</span>
        </span>
      {:else if field.type === 'date'}
        <input
          class="rp-date"
          type="date"
          bind:value={draft}
          onkeydown={keydown}
          use:focusOnMount
          aria-label={field.label}
        />
        <button class="rp-chipbtn" type="button" onclick={() => void send(today)}>Today</button>
        <button
          class="rp-chipbtn"
          type="button"
          onclick={() => void send(today ? plusDays(today, 7) : today)}>+1 week</button
        >
        <button
          class="rp-chipbtn"
          type="button"
          onclick={() => void send(today ? plusDays(today, 30) : today)}>+1 month</button
        >
      {:else}
        <button
          class="rp-plus"
          type="button"
          aria-label={`One fewer ${field.label}`}
          onclick={() => void send(String(Math.max(0, (field.amount ?? 0) - 1)))}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M5 12h14" /></svg>
        </button>
        <input
          class="rp-num"
          type="number"
          inputmode="numeric"
          bind:value={draft}
          onkeydown={keydown}
          use:focusOnMount
          aria-label={field.label}
        />
        <button
          class="rp-plus"
          type="button"
          aria-label={`One more ${field.label}`}
          onclick={() => void send(String((field.amount ?? 0) + 1))}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        </button>
        <button class="rp-quiet" type="button" onclick={() => void send(draft)}>Set</button>
      {/if}
    </div>
  {/if}

  {#if chip}
    <span
      class="rp-state cd-chip"
      class:cd-chip--ok={written === 'ok'}
      class:cd-chip--risk={written === 'fail'}
      role="status"
    >
      {chip}
    </span>
  {/if}

  {#if refusal}
    <div class="rp-say">
      <p class="rp-say__why" role="alert">{refusal}</p>
      <p class="rp-say__at" data-developer>{address}/fields/{field.key}</p>
      <button class="rp-quiet" type="button" onclick={() => (refusal = null)}>Done</button>
    </div>
  {/if}
</div>
