<!--
  Review answer slot: provides view and inline edit modes for the topic answer text,
  keyed by record ID with Enter-to-save and Escape-to-cancel semantics.
-->
<script lang="ts">
  import { tick } from 'svelte';

  let {
    cardId,
    answer,
    onAnswer,
    onink = false,
  }: { cardId: string; answer: string; onAnswer: (text: string) => void; onink?: boolean } = $props();

  let editing = $state(false);
  /** The field's own value while it is open; the record's answer is what it is
   *  seeded and cleared with, never a captured copy of it. */
  let draft = $state('');
  let field = $state<HTMLInputElement | null>(null);
  let door = $state<HTMLButtonElement | null>(null);
  /** The record the slot is showing, so a new card is a new, closed slot. */
  let held = $state<string | null>(null);

  $effect(() => {
    if (held === cardId) return;
    held = cardId;
    editing = false;
    draft = answer;
  });

  /** The door's one job, both ways: open the field, or save what is in it. */
  async function toggle(): Promise<void> {
    if (!editing) {
      draft = answer;
      editing = true;
      await tick();
      field?.focus();
      return;
    }
    onAnswer(draft.trim());
    editing = false;
    door?.focus();
  }

  function key(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      void toggle();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      draft = answer;
      editing = false;
      door?.focus();
    }
  }
</script>

<div class="ans" class:ans--ink={onink}>
  <span class="ans__caps">Your answer</span>
  <div class="ans__row">
    {#if editing}
      <input
        class="ans__field"
        type="text"
        bind:this={field}
        bind:value={draft}
        placeholder="In your own words"
        aria-label="Your answer, in your own words"
        onkeydown={key}
      />
    {:else}
      <p class="ans__line" data-set={answer ? '' : undefined}>
        {answer ? `“${answer}”` : 'Nothing written yet'}
      </p>
    {/if}
    <button
      class="cd-pill cd-pill--quiet cd-pill--sm"
      type="button"
      bind:this={door}
      aria-expanded={editing}
      onclick={() => void toggle()}
    >
      {editing ? 'Save' : answer ? 'Edit' : 'Write one'}
    </button>
  </div>
</div>

<style>
  .ans {
    margin-top: calc(20px * var(--ui-s));
  }
  .ans__caps {
    display: block;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .ans__row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ui-gap-sm);
    margin-top: calc(4px * var(--ui-s));
  }
  .ans__line {
    margin: 0;
    font-size: var(--ui-text);
    color: var(--ink-3);
  }
  .ans__line[data-set] {
    color: var(--ink);
  }
  /* A field is a recessed thing: no border, an inset ring, a well fill — the
     recipe `.rv__input` already uses in this screen's own stylesheet. */
  .ans__field {
    width: 100%;
    min-height: var(--hit);
    padding: 0 var(--ui-pad-sm);
    border: 0;
    border-radius: var(--r-item);
    background: var(--well);
    color: var(--ink);
    font: inherit;
    font-size: var(--ui-text);
  }
  .ans__field::placeholder {
    color: var(--ink-3);
  }
  .ans__field:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--ink);
  }

  /* On the ink plane. */
  .ans--ink .ans__caps,
  .ans--ink .ans__line {
    color: color-mix(in oklab, var(--ink-inv) 74%, transparent);
  }
  .ans--ink .ans__line[data-set] {
    color: var(--ink-inv);
  }
  .ans--ink .ans__field,
  .ans--ink .cd-pill {
    background: var(--fill-on-ink);
    color: var(--ink-inv);
  }
  .ans--ink .ans__field:hover,
  .ans--ink .cd-pill:hover {
    background: var(--fill-on-ink-hi);
  }
  .ans--ink .ans__field::placeholder {
    color: color-mix(in oklab, var(--ink-inv) 74%, transparent);
  }
  .ans--ink .ans__field:focus-visible {
    box-shadow: inset 0 0 0 2px var(--ink-inv);
  }
</style>
