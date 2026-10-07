<!--
  Reviews queue variant C: The Inline Defer.
  Overdue reviews list with an in-place day picker for deferring individual records.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import type { QueueProps } from './props';

  let { rows, moved, days, onDefer, onUndo }: QueueProps = $props();

  let editing = $state<string | null>(null);
  let busy = $state(false);
  let root = $state<HTMLElement | null>(null);

  function control(id: string, what: string): HTMLElement | null {
    return root?.querySelector<HTMLElement>(`[data-${what}="${id}"]`) ?? null;
  }

  async function start(id: string): Promise<void> {
    editing = id;
    await tick();
    control(id, 'for')?.focus();
  }

  async function cancel(id: string): Promise<void> {
    editing = null;
    await tick();
    control(id, 'defer')?.focus();
  }

  async function commit(id: string, offset: number): Promise<void> {
    if (busy) return;
    busy = true;
    await onDefer(id, offset);
    editing = null;
    busy = false;
    await tick();
    // The record has left the list, so focus follows it to the one control that
    // now stands for it — the pocket's own Undo — rather than falling to body.
    root?.querySelector<HTMLElement>('[data-undo]')?.focus();
  }
</script>

<div class="v-fit rqc" bind:this={root}>
  <div class="cd-card rqc-card">
      <div class="rqc-list">
        {#each rows as row (row.id)}
          <div class="rqc-row" data-edit={editing === row.id ? '' : undefined}>
            <span class="rqc-row__t">{row.title}</span>
            {#if row.course}
              <span class="cd-chip cd-chip--code num" data-w={row.wash}>{row.course}</span>
            {/if}
            <span class="rqc-slot">
              {#if editing === row.id}
                <span class="rqc-cap rqc-cap--edit">
                  {#each days as day (day.days)}
                    <button
                      class="rqc-day"
                      type="button"
                      data-for={row.id}
                      aria-label={`Defer ${row.title} to ${day.label}`}
                      onclick={() => void commit(row.id, day.days)}
                      onkeydown={(event) => {
                        if (event.key === 'Escape') {
                          event.preventDefault();
                          void cancel(row.id);
                        }
                      }}
                    >
                      {day.label}
                    </button>
                  {/each}
                </span>
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  aria-label={`Cancel deferring ${row.title}`}
                  onclick={() => void cancel(row.id)}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6.6 6.6 17.4 17.4M17.4 6.6 6.6 17.4" /></svg>
                  Cancel
                </button>
              {:else}
                <span class="rqc-cap" data-aged={row.late > 0 ? '' : undefined}>
                  <span class="rqc-cap__t num">{row.state}</span>
                </span>
                <button
                  class="cd-pill cd-pill--quiet cd-pill--sm"
                  type="button"
                  data-defer={row.id}
                  aria-expanded={false}
                  aria-label={`Defer ${row.title} to another day`}
                  onclick={() => void start(row.id)}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M4 20h4L19.2 8.8a2.1 2.1 0 0 0-3-3L5 17v3Z" /><path d="M14.6 7.4l2 2" /></svg>
                  Defer
                </button>
              {/if}
            </span>
          </div>
        {/each}
    </div>
  </div>

  {#if moved.deferred.length > 0}
    <section class="rqc-pocket" aria-label="Deferred today">
      <p class="rqc-pocket__k">
        Deferred today · <b class="num">{moved.deferred.length}</b>
      </p>
      {#each moved.deferred as row (row.id)}
        <div class="rqc-prow">
          <span class="rqc-prow__t">{row.title}</span>
          <span class="rqc-prow__d num">{row.say}</span>
          <button
            class="cd-pill cd-pill--quiet cd-pill--sm"
            type="button"
            data-undo={row.id}
            aria-label={`Undo the defer on ${row.title}`}
            onclick={() => void onUndo()}
          >
            Undo
          </button>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .rqc {
    width: min(860px, 100%);
  }
  .rqc-card {
    padding: 0;
    overflow: clip;
  }
  .rqc-list {
    max-height: calc(424px * var(--ui-s));
    overflow-y: auto;
    overflow-anchor: none;
  }
  .rqc-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(64px * var(--ui-s));
    padding: calc(8px * var(--ui-s)) var(--ui-pad);
    transition: background var(--dur-1) var(--ease);
  }
  .rqc-row + .rqc-row {
    border-top: 1px dashed var(--rule-strong);
  }
  .rqc-row:hover {
    background: var(--well);
  }
  .rqc-row[data-edit] {
    background: var(--well);
  }
  .rqc-row__t {
    font-size: var(--ui-text);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rqc-slot {
    display: flex;
    align-items: center;
    gap: var(--ui-gap-sm);
  }

  /* The capsule — the app's own recessed field species: no border, an inset
     ring, a well fill, a fixed width so opening the editor moves the title by
     nothing at all. It clips its own content, which is what lets the day
     objects arrive inside it as it grows. */
  .rqc-cap {
    --rqc-cap-from: calc(176px * var(--ui-s));
    --rqc-cap-to: calc(326px * var(--ui-s));
    display: flex;
    align-items: center;
    width: var(--rqc-cap-w, var(--rqc-cap-from));
    height: var(--hit);
    padding: 0 var(--ui-pad-sm);
    border-radius: var(--r-pill);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--rule);
    overflow: hidden;
    transition: box-shadow var(--dur-1) var(--ease), background var(--dur-1) var(--ease);
  }
  .rqc-cap__t {
    font-size: var(--ui-text-sm);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-title);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rqc-cap[data-aged] .rqc-cap__t {
    color: var(--ink);
  }
  /* The editor is a fresh element every render, so its growth is an animation
     rather than a transition — its own two widths, over the same curve. */
  @keyframes rqc-grow {
    from {
      width: var(--rqc-cap-from);
    }
    to {
      width: var(--rqc-cap-to);
    }
  }
  .rqc-cap--edit {
    --rqc-cap-w: var(--rqc-cap-to);
    padding: 0 calc(4px * var(--ui-s));
    gap: calc(6px * var(--ui-s));
    animation: rqc-grow var(--dur-2) var(--ease) both;
  }

  /* A day object: weekday and numeral on one line, named in full in its label. */
  .rqc-day {
    flex: none;
    display: grid;
    place-items: center;
    width: calc(74px * var(--ui-s));
    height: var(--hit);
    border-radius: var(--r-mini);
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--sh-1);
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    white-space: nowrap;
    transition: background var(--dur-1) var(--ease), transform var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }
  .rqc-day:hover {
    transform: translateY(-1px);
    box-shadow: var(--sh-2);
  }
  .rqc-day:active {
    transform: none;
    box-shadow: var(--sh-1);
    background: var(--well-2);
  }
  .rqc-day:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  /* The arrival uses the independent `translate` property, not `transform`, so
     the hover lift and the press stay live while it plays. */
  @keyframes rqc-in {
    from {
      opacity: 0;
      translate: calc(6px * var(--ui-s)) 0;
    }
    to {
      opacity: 1;
      translate: 0 0;
    }
  }
  .rqc-cap--edit .rqc-day {
    animation: rqc-in var(--dur-2) var(--ease) both;
  }
  .rqc-cap--edit .rqc-day:nth-child(2) {
    animation-delay: calc(30ms * var(--ui-s));
  }
  .rqc-cap--edit .rqc-day:nth-child(3) {
    animation-delay: calc(60ms * var(--ui-s));
  }
  .rqc-cap--edit .rqc-day:nth-child(4) {
    animation-delay: calc(90ms * var(--ui-s));
  }

  .rqc-pocket {
    display: grid;
    gap: calc(4px * var(--ui-s));
    margin-top: var(--ui-gap);
    padding: var(--ui-pad-sm) var(--ui-pad);
    border-radius: var(--r-card);
    background: var(--well);
  }
  .rqc-pocket__k {
    margin: 0;
    font-size: var(--ui-meta);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .rqc-prow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--ui-gap);
    min-height: calc(44px * var(--ui-s));
  }
  .rqc-prow + .rqc-prow {
    border-top: 1px dashed var(--rule-strong);
  }
  .rqc-prow__t {
    font-size: var(--ui-text);
    letter-spacing: var(--track-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rqc-prow__d {
    font-size: var(--ui-text-sm);
    color: var(--ink-3);
    white-space: nowrap;
  }

  /* ── narrow: the row stacks its slot under the title, and the capsule's
     editor keeps the four days inside the container rather than at 326px. */
  @container (max-width: 620px) {
    .rqc-row {
      grid-template-columns: minmax(0, 1fr);
      row-gap: calc(6px * var(--ui-s));
    }
    .rqc-slot {
      justify-content: flex-start;
      flex-wrap: wrap;
    }
    .rqc-cap--edit {
      --rqc-cap-to: calc(300px * var(--ui-s));
    }
    .rqc-day {
      width: calc(64px * var(--ui-s));
    }
  }
</style>
