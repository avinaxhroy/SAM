<!--
  THE LIST CARD · B · THE SPIKE.
  Vertical stack variant representing records as indexed paper slips threaded
  along a central axis. Slips render title, course code, due distance, and
  action controls.
-->
<script lang="ts">
  import BlockHead from '../../blocks/BlockHead.svelte';
  import Doors from './Doors.svelte';
  import { emptyWords, type ListCardProps } from './props';

  let {
    title,
    view,
    type,
    rows,
    candidates,
    selected,
    menuFor,
    menuRows,
    onSelect,
    onPanel,
    onMenu,
    onNew,
    onPaste,
    onRenumber,
  }: ListCardProps = $props();

  // Alternating tilt angles for stacked slips.
  const TILT = [-1.6, 0, 1.6];
</script>

<section class="cd-card pdz-card" data-view={view} data-type={type}>
  <BlockHead {title} {type} word="List" returned={rows.length} {candidates} {onNew} {onPaste} {onRenumber} />

  {#if rows.length === 0}
    <p class="cd-empty"><span class="cd-empty__s">{emptyWords(candidates, type)}</span></p>
  {:else}
    <div class="pdz-spike">
      <div class="pdz-field">
        <span class="pdz-plate" aria-hidden="true"></span>
        <span class="pdz-needle" aria-hidden="true"></span>
        <ul class="pdz-slips">
          {#each rows as row, i (row.id)}
            <li
              class="pdz-slip"
              data-record-id={row.id}
              style="--tilt:{TILT[i % TILT.length]}deg"
            >
              <span class="pdz-hole" aria-hidden="true"></span>
              <button
                class="pdz-press"
                type="button"
                aria-pressed={selected === row.id}
                aria-label={row.sentence}
                onclick={() => onSelect(row.id)}
              >
                <span class="pdz-nm">{row.label}</span>
                <span class="pdz-fact">
                  {#if row.course}
                    <span class="cd-chip cd-chip--code" data-w={row.course.wash}>{row.course.code}</span>
                  {/if}
                  {#if row.due}
                    <span class="pdz-when num" data-late={row.late}>{row.due}</span>
                  {/if}
                </span>
              </button>
              <div class="pdz-acts">
                <Doors {row} open={menuFor === row.id} {menuRows} {onPanel} {onMenu} />
              </div>
            </li>
          {/each}
        </ul>
      </div>
    </div>
  {/if}
</section>

<style>
  .pdz-card {
    container-type: inline-size;
  }

  /* The spike is an object, not a column: its rod, plate and the slips threaded
     on it are one composition measured from the container and centred. The
     slips stand in normal flow — one grid row each, their own height — so no
     slip can ever cover the next however tall its name runs; the rod and plate
     are drawn behind them as an absolutely-positioned layer. Every offset the
     hole depends on is a token, so the whole rod scales with the register and
     the hole never leaves it. */
  .pdz-spike {
    --pdz-w: calc(440px * var(--ui-s));
    --pdz-rod: calc(74px * var(--ui-s));
    --pdz-left: calc(52px * var(--ui-s));
    --pdz-hole: calc(15px * var(--ui-s));
    --pdz-padl: calc(40px * var(--ui-s));
    --pdz-origin: calc(24px * var(--ui-s));
    display: grid;
    justify-items: center;
  }
  /* The head above the first slip and the plate's own room below the last are
     the field's own padding, so the rod it carries is always the slips' span
     plus those two ends — never a constant. */
  .pdz-field {
    position: relative;
    width: calc(var(--pdz-w) + var(--pdz-left));
    max-width: 100%;
    padding: calc(38px * var(--ui-s)) 0 calc(20px * var(--ui-s)) var(--pdz-left);
  }

  /* One needle and one plate, and nothing else: a printed rule and a flat base,
     both drawn — no ball head, no cast stand, no chrome. The rod is `--ink-3`,
     a step off the ink the screen's one dark object uses, so the two never
     compete. It stands behind the slips (`z-index: 0`), so what shows is above
     the first slip, in the gaps and inside the punched holes. */
  .pdz-needle {
    position: absolute;
    z-index: 0;
    left: var(--pdz-rod);
    top: calc(16px * var(--ui-s));
    bottom: calc(5px * var(--ui-s));
    width: 2px;
    background: var(--ink-3);
  }
  .pdz-plate {
    position: absolute;
    z-index: 0;
    left: var(--pdz-rod);
    bottom: 0;
    translate: -50% 0;
    width: calc(104px * var(--ui-s));
    height: 3px;
    background: var(--well-2);
    border-top: 1px solid var(--ink-3);
  }

  .pdz-slips {
    position: relative;
    z-index: 1;
    display: grid;
    grid-auto-rows: auto;
    /* the rod is read in the gaps between slips; the room also holds the fan's
       own swing, so a tilted slip never touches its neighbour */
    gap: calc(14px * var(--ui-s));
    list-style: none;
    margin: 0;
    padding: 0;
  }
  /* One slip: a receipt's own shape — long, narrow, punched at its left end and
     threaded on the rod. Its identity is that shape, the punched hole and the
     rod it hangs on; the paper is warm by a fifth off the card, which is the
     one step the material needs to be an object at all. It sits in the flow, so
     its height is its own and its hole stays centred on the rod at any height. */
  .pdz-slip {
    position: relative;
    z-index: 1;
    width: var(--pdz-w);
    max-width: 100%;
    display: grid;
    gap: var(--space-2xs);
    padding: var(--space-sm) var(--space-md) var(--space-sm) var(--pdz-padl);
    background-color: color-mix(in oklab, var(--wash-peach) 45%, var(--card));
    border-radius: 0 0 var(--r-mini) var(--r-mini);
    box-shadow: var(--sh-1);
    transform: rotate(var(--tilt, 0deg));
    /* about the hole, so the fan never pulls the hole off the rod */
    transform-origin: var(--pdz-origin) 50%;
    /* the fan is a position change; shadows are state-explicit, never
       transitioned. Gated below by the reduced-motion query. */
    transition: transform var(--dur-2) var(--ease);
  }
  /* The paper is cut, so the hole shows the card the slip lies on; the rod
     inside it is the slip's own 2px mark, at the rod's own place. */
  .pdz-hole {
    position: absolute;
    left: var(--pdz-hole);
    top: 50%;
    margin-top: calc(-7px * var(--ui-s));
    width: calc(14px * var(--ui-s));
    height: calc(14px * var(--ui-s));
    border-radius: var(--r-pill);
    overflow: hidden;
    background: var(--card);
    box-shadow: inset 0 0 0 1.5px var(--rule-strong);
  }
  .pdz-hole::after {
    content: '';
    position: absolute;
    left: 50%;
    top: -2px;
    bottom: -2px;
    width: 2px;
    translate: -50% 0;
    background: var(--ink-3);
  }
  .pdz-slip:has(.pdz-press[aria-pressed='true']) {
    box-shadow: var(--sh-1), inset 0 0 0 1.5px var(--rule-strong);
  }

  /* The slip's press: the object's own control. It is the shape it names, never
     a second paper — the paper is the slip. */
  .pdz-press {
    display: grid;
    gap: var(--space-2xs);
    min-width: 0;
    text-align: left;
    border-radius: var(--r-mini);
  }
  .pdz-press:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .pdz-nm {
    font-size: calc(var(--text-base) * var(--ui-s));
    line-height: 1.3;
    color: var(--ink);
    overflow-wrap: anywhere;
    text-wrap: pretty;
  }
  .pdz-fact {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    min-width: 0;
  }
  .pdz-when {
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
  }
  .pdz-when[data-late='true'] {
    color: var(--ink);
    font-weight: var(--weight-label);
  }
  .pdz-acts {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
  }

  /* ── narrow: the same object standing up ───────────────────────────────
     The rod moves in and the slips run to the container's width, while every
     offset the hole depends on moves with it — so the hole still sits on the
     rod. */
  @container (max-width: 520px) {
    .pdz-spike {
      --pdz-w: 100%;
      --pdz-rod: calc(52px * var(--ui-s));
      --pdz-left: calc(40px * var(--ui-s));
      --pdz-hole: calc(5px * var(--ui-s));
      --pdz-padl: calc(32px * var(--ui-s));
      --pdz-origin: calc(12px * var(--ui-s));
    }
    .pdz-field {
      width: 100%;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pdz-slip {
      transition: none;
    }
  }
</style>
