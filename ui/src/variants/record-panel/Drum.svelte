<!--
  B · THE DRUM (port, 2026-09-29) — one digit place, made of ten numerals.

  Every digit place is one clipped window holding all ten numerals; the numerals
  are translated by the reference's offset times the numeral height **measured
  off the pad's own rect** (`bind:clientHeight`, the reference's `useMeasure`),
  and the offset is taken the shorter way round (`drum.ts`). Nothing about the
  height is declared in CSS.

  AT REST IT STANDS; A WRITE ROLLS IT. A window arriving in the document is
  placed where it stands — an instrument already at its reading, never a numeral
  that slides in from zero. The roll belongs to the change a **write** made:
  `roll` is the design's own mark (it counts writes at this value), so a re-read
  that lands without one places silently. A place that appeared or went is
  rebuilt (a place cannot be rolled into existence), seeded at the digits the
  surviving places were standing at, and rolls from there. The numeral that
  crosses the seam is teleported off-clip inside one frame, as the reference
  does, instead of travelling the long way through the window.

  The markup is Svelte's; only the two-frame sequencing is imperative, because a
  placement that must not tween and the tween that follows are two paints of the
  same element. `prefers-reduced-motion` switches the roll off in CSS.
-->
<script lang="ts">
  import { digitAt, offsetOf, partsOf, samePlaces, windowsOf, type Window } from './drum';

  let { text, roll = 0 }: { text: string; roll?: number } = $props();

  const NUMERALS = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

  /** The places the drum has. */
  let windows = $state<Window[]>([]);
  /** The digits standing in them — what each numeral's offset is read from. */
  let shown = $state<Window[]>([]);
  /** The numeral height, measured. */
  let height = $state(0);
  /** `true` while a placement is a position, not a move. */
  let silent = $state(true);
  /** The reading the windows stand at, so a no-op change does nothing. */
  let placed = $state('');
  let seenRoll = $state(0);

  const parts = $derived(partsOf(text));

  $effect(() => {
    const wanted = text;
    const mark = roll;
    const unit = height;
    if (wanted === placed) return;
    const next = windowsOf(parts);
    const rollNow = mark !== seenRoll;
    seenRoll = mark;
    placed = wanted;
    const same = samePlaces(next, windows);
    if (!same) {
      // A place appeared or went: the windows are rebuilt (a place cannot be
      // rolled into existence), seeded at the digits the surviving places were
      // standing at, and the roll runs from there.
      const held = windows;
      silent = true;
      windows = next;
      shown = next.map((win) => {
        const before = held.find((was) => was.place === win.place);
        return { place: win.place, value: before ? before.value : 0 };
      });
    }
    // A placement with no measured height yet is a position, not a move: the
    // roll needs the pad's own rect, which exists once the pad is painted.
    if (!rollNow || unit === 0) {
      silent = true;
      shown = next;
      return;
    }
    // The digits stay where they stand, then move on the next frame — one
    // placement that does not tween, one that does.
    silent = true;
    const target = next;
    requestAnimationFrame(() => {
      silent = false;
      shown = target;
    });
  });

  function numeralStyle(win: Window, index: number, numeral: number): string {
    const digit = digitAt(win);
    const was = shown[index] ? digitAt(shown[index]) : digit;
    const to = offsetOf(numeral, digit);
    const jump = Math.abs(to - offsetOf(numeral, was)) > 5;
    const y = Math.round(to * height * 100) / 100;
    return `transform: translateY(${y}px);${silent || jump ? ' transition: none;' : ''}`;
  }
</script>

<span class="rp-nums" aria-hidden="true">
  {#each windows as win, index (win.place)}
    {#if parts.sep && index === parts.intPlaces.length}
      <span class="rp-drum__sep">{parts.sep}</span>
    {/if}
    <span class="rp-drum">
      <span class="rp-drum__pad" bind:clientHeight={height}>0</span>
      {#each NUMERALS as numeral (numeral)}
        <span class="rp-drum__n" style={numeralStyle(win, index, numeral)}>{numeral}</span>
      {/each}
    </span>
  {/each}
</span>
