<!-- Timer chrome variant B: The Morphing Pill. In-place transitions between display and editor. -->
<script lang="ts">
  import type { TimerProps } from './props';

  let {
    label,
    course,
    clock,
    paused,
    targetMin,
    ended,
    command,
    placement,
    onPause,
    onStop,
    onMinutes,
    onUndo,
  }: TimerProps = $props();

  let root = $state<HTMLElement | null>(null);
  let field = $state<HTMLInputElement | null>(null);
  let edit = $state(false);
  /** The field's own text, so a half-typed number is never clamped under the
      cursor; the app's 5–180 is applied where the value is read. */
  let typed = $state(String(targetMin));

  $effect(() => {
    if (!edit) typed = String(targetMin);
  });

  /** The app's own 5–180, the range the shipped duration capsule declares. */
  function clamp(value: number): number {
    return Math.max(5, Math.min(180, Math.round(value)));
  }

  function valueNow(): number {
    const parsed = parseInt(typed.replace(/[^0-9]/g, ''), 10);
    return clamp(Number.isFinite(parsed) ? parsed : targetMin);
  }

  function close(): void {
    edit = false;
  }

  function open(): void {
    edit = true;
    // the reference waits for the shape to arrive before the field takes focus
    setTimeout(() => {
      if (edit) {
        field?.focus();
        field?.select();
      }
    }, 240);
  }

  function commit(): void {
    const next = valueNow();
    onMinutes(next);
    typed = String(next);
    close();
  }

  function abandon(): void {
    typed = String(targetMin);
    close();
  }

  function toggleEdit(): void {
    if (edit) commit();
    else open();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      abandon();
    } else if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      const step = event.key === 'ArrowUp' ? 5 : -5;
      typed = String(clamp((parseInt(typed, 10) || targetMin) + step));
    }
  }

  /** Leaving the capsule commits, like any field — but only once focus has
      really left it, so pressing the check cannot race the commit. */
  function onFocusOut(event: FocusEvent): void {
    if (!edit) return;
    const next = event.relatedTarget as Node | null;
    if (next && root?.contains(next)) return;
    commit();
  }
</script>

<div
  class="vtcb"
  bind:this={root}
  data-mode={ended ? 'receipt' : paused ? 'paused' : 'run'}
  data-edit={edit ? '1' : '0'}
  onfocusout={onFocusOut}
>
  <div class="vtcb__box">
    <div class="cd-dur" data-edit={edit ? '1' : '0'} role="group" aria-label="Study session">
      <span class="cd-dur__part cd-dur__part--v">
        <span class="vtcb__live" aria-hidden="true" hidden={paused}></span>
        <span class="vtcb__ring" aria-hidden="true" hidden={!paused}></span>
        <span class="vtcb__clock num" role="timer" aria-label={`${clock} elapsed${paused ? ', paused' : ''}`}>{clock}</span>
        {#if paused}<span class="vtcb__state">Paused</span>{/if}
        {#if !paused}<span class="vtcb__subject" title={label}>{label}</span>{/if}
      </span>
      <!-- the length, as the field it is: a real input in BOTH states -->
      <span class="cd-dur__part cd-dur__part--mid">
        <input
          class="num"
          type="text"
          inputmode="numeric"
          bind:this={field}
          bind:value={typed}
          tabindex={edit ? 0 : -1}
          aria-label="Session length in minutes"
          onkeydown={onKeydown}
        />
        <span class="cd-dur__unit">min</span>
      </span>
      <button
        class="cd-dur__part cd-dur__part--act"
        type="button"
        aria-label={edit ? 'Save the session length' : 'Change the session length'}
        aria-expanded={edit}
        onclick={toggleEdit}
      >
        <span class="cd-dur__ico cd-dur__ico--pencil" aria-hidden="true">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" focusable="false"><path d="M10.6 2.9 13.1 5.4 5.9 12.6 3 13l.4-2.9z" /><path d="M9.6 3.9 12.1 6.4" /></svg>
        </span>
        <span class="cd-dur__ico cd-dur__ico--check" aria-hidden="true">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" focusable="false"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
        </span>
      </button>
    </div>
  </div>

  <button
    class="vtcb__verb"
    type="button"
    aria-label={paused ? 'Resume the session' : 'Pause the session'}
    title={paused ? 'Resume the session' : 'Pause the session'}
    onclick={onPause}
  >
    <svg class="vtcb__g vtcb__g--pause" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M6 4v8M10 4v8" /></svg>
    <svg class="vtcb__g vtcb__g--play" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M5.6 3.4 12.4 8l-6.8 4.6z" /></svg>
  </button>
  <button
    class="vtcb__verb"
    type="button"
    data-command={command ?? undefined}
    data-placement={command ? placement : undefined}
    aria-label="Stop and log the session"
    title="Stop and log the session"
    onclick={onStop}
  >
    <svg class="vtcb__g" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
  </button>

  <div class="vtcb__receipt">
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
    <span class="vtcb__rt">Logged <b class="num">{clock}</b> to {course ?? label}</span>
    <button class="vtcb__verb vtcb__verb--undo" type="button" aria-label="Undo the log" onclick={onUndo}>Undo</button>
  </div>
</div>

<style>
  /* The declared reserve, and the capsule's own box inside it. The box is the
     width of the SPLIT state, so the two verbs beside it do not move when the
     pill opens. */
  .vtcb {
    --b-name: calc(76px * var(--ui-s));
    --b-dur: 304px;
    --b-reserve: 384px;
    position: relative;
    flex: none;
    width: var(--b-reserve);
    height: var(--hit);
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    align-self: center;
    -webkit-app-region: no-drag;
  }
  .vtcb__box {
    position: relative;
    flex: none;
    width: var(--b-dur);
    height: var(--hit);
    display: flex;
  }

  /* the row's scale, said with three classes: the design layer's own
     `[data-edit="1"] .cd-dur__part--v` rule is three too */
  .vtcb .cd-dur {
    flex: none;
  }
  .vtcb .cd-dur .cd-dur__part {
    height: var(--hit);
  }
  .vtcb .cd-dur .cd-dur__part--v {
    padding: 0 var(--space-sm);
  }
  .vtcb .cd-dur .cd-dur__part--mid {
    border-radius: 0;
    padding: 0 var(--space-sm);
  }
  /* the design layer rings the VALUE part on `:focus-within`; in this row the
     field lives in the middle part, so the ring is declared for it too */
  .vtcb .cd-dur .cd-dur__part--mid:focus-within {
    box-shadow: inset 0 0 0 2px var(--ink);
  }
  .vtcb .cd-dur .cd-dur__part--act {
    width: 40px;
  }
  /* fused: the seams live INSIDE the parts (the air the gaps will take) */
  .vtcb .cd-dur[data-edit='0'] .cd-dur__part--v,
  .vtcb .cd-dur[data-edit='0'] .cd-dur__part--mid {
    padding-right: var(--space-xl);
  }
  .vtcb .cd-dur[data-edit='1'] {
    gap: var(--space-sm);
  }
  .vtcb .cd-dur input {
    width: 2.5ch;
    font-size: var(--text-xs);
    font-weight: var(--weight-display);
    letter-spacing: var(--track-title);
  }
  .vtcb .cd-dur__unit {
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
  }
  .vtcb .cd-dur__ico {
    width: 13px;
    height: 13px;
  }

  /* the mark: one 6px box in one place, solid running and hollow paused.
     Nothing pulses — permanent chrome is the last place for an animation. */
  .vtcb__live {
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--ink);
    flex: none;
  }
  .vtcb__ring {
    width: 6px;
    height: 6px;
    border-radius: var(--r-pill);
    box-shadow: inset 0 0 0 1.5px var(--ink);
    flex: none;
  }
  .vtcb__live[hidden],
  .vtcb__ring[hidden] {
    display: none;
  }

  /* the clock is pinned twice — a declared 6ch box and tabular figures — for
     the same reason the shipped `.cd-timer` pins it */
  .vtcb__clock {
    flex: none;
    min-width: 6ch;
    font-size: var(--text-xs);
    font-weight: var(--weight-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-title);
  }
  .vtcb[data-mode='paused'] .vtcb__clock {
    opacity: 0.62;
  }
  /* the state word takes the name's own declared box, so paused is as wide as
     running */
  .vtcb__state {
    flex: none;
    width: var(--b-name);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-2);
  }
  /* ONE line in a DECLARED width: the name yields, the row does not */
  .vtcb__subject {
    flex: none;
    width: var(--b-name);
    font-size: var(--text-2xs);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* the two verbs beside the pill, at the 32px floor. Glyphs: the words live
     in the accessible name, which is the declared trade of a row this tight. */
  .vtcb__verb {
    flex: none;
    height: var(--hit);
    min-width: var(--hit);
    padding: 0;
    border-radius: var(--r-pill);
    background: transparent;
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2xs);
    font-size: var(--text-2xs);
    font-weight: var(--weight-label);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  .vtcb__verb:hover {
    background: var(--well);
  }
  .vtcb__verb:active {
    background: var(--well-2);
  }
  .vtcb__verb:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .vtcb__g {
    width: 13px;
    height: 13px;
  }
  /* One glyph per (state, verb) and never two in the same box. The lab's own
     rules: a boolean attribute on an `<svg>` does not remove the element from
     layout, so the state is the mode the capsule already carries. */
  .vtcb__g--play {
    display: none;
  }
  .vtcb[data-mode='paused'] .vtcb__g--pause {
    display: none;
  }
  .vtcb[data-mode='paused'] .vtcb__g--play {
    display: block;
  }

  /* the receipt: the same declared reserve, one line, the app's own 8s */
  .vtcb__receipt {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 0 var(--space-2xs) 0 var(--space-sm);
    border-radius: var(--r-pill);
    background: var(--well-2);
    color: var(--ink);
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--dur-2) var(--ease), visibility 0s linear var(--dur-2);
  }
  .vtcb__rt {
    flex: 1 1 auto;
    min-width: 0;
    font-size: var(--text-2xs);
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vtcb__rt b {
    color: var(--ink);
    font-weight: var(--weight-display);
  }
  .vtcb[data-mode='receipt'] .vtcb__box > .cd-dur,
  .vtcb[data-mode='receipt'] > .vtcb__verb {
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--dur-2) var(--ease), visibility 0s linear var(--dur-2);
  }
  .vtcb[data-mode='receipt'] .vtcb__receipt {
    opacity: 1;
    visibility: visible;
  }
  .vtcb__verb--undo {
    margin-left: auto;
    padding: 0 var(--space-sm);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .vtcb__verb--undo:hover {
    background: var(--card);
  }

  /* The room decides the reserve, and the reserve is a DECLARED number on each
     side of the line — three tiers, each one a smaller box, never a box that
     shrinks by itself. In the app the window IS the viewport, so the lab's
     `@container frame` tiers are media queries on the window, which is the same
     range the shell's own narrow rules answer to. */
  @media (max-width: 980px) {
    .vtcb {
      --b-reserve: 308px;
      --b-dur: 228px;
    }
    .vtcb__subject,
    .vtcb__state {
      display: none;
    }
  }
  @media (max-width: 800px) {
    .vtcb {
      --b-reserve: 212px;
      --b-dur: 144px;
      gap: var(--space-2xs);
    }
    /* one part at a time in the tight box: the clock while it is a reading,
       the field while it is an editor, and the act cap beside either */
    .vtcb[data-edit='0'] .cd-dur__part--mid {
      display: none;
    }
    .vtcb[data-edit='1'] .cd-dur__part--v {
      display: none;
    }
    .vtcb .cd-dur .cd-dur__part--v {
      padding: 0 var(--space-xs);
    }
  }
  @media (max-width: 530px) {
    .vtcb {
      --b-reserve: 200px;
      --b-dur: 136px;
      gap: 0;
    }
    .vtcb .cd-dur .cd-dur__part--act {
      width: var(--hit);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .cd-dur,
    .cd-dur__part,
    .cd-dur__ico,
    .vtcb__verb,
    .vtcb__receipt {
      transition: none;
    }
  }
</style>
