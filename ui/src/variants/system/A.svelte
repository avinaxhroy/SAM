<!--
  SYSTEM · A · THE INDEX.
  Tabbed system configuration view using a segmented control to switch between
  system places (Kinds, Screens, Rules, Scheduling, Stages), opening detailed
  editors in animated 460px sheets.
-->
<script lang="ts">
  import Editor from './Editor.svelte';
  import Head from './Head.svelte';
  import Place from './Place.svelte';
  import Receipt from './Receipt.svelte';
  import Recovery from './Recovery.svelte';
  import Rail from './Rail.svelte';
  import Search from './Search.svelte';
  import type { SystemProps } from './props';

  let {
    places,
    index,
    place,
    strip,
    query,
    searching,
    openKey,
    editor,
    projection,
    receipt,
    recovering,
    backups,
    restoring,
    hits,
    rail,
    iconNames,
    busy,
    acts,
  }: SystemProps = $props();

  const where = $derived(places.find((entry) => entry.id === place) ?? places[0]);
  const here = $derived(index[place] ?? { rows: [], groups: [] });

  let root = $state<HTMLElement | null>(null);
  let sheetEl = $state<HTMLElement | null>(null);
  /** Inert until the click that opened the sheet has finished propagating — a
   *  window listener armed before that sees the opener as its target and would
   *  close the sheet the student just asked for (`shell/Sheet.svelte` documents
   *  the same guard, and the lab found it the same way). */
  let armed = $state(false);

  $effect(() => {
    if (!editor) {
      armed = false;
      return;
    }
    const id = setTimeout(() => (armed = true), 0);
    return () => clearTimeout(id);
  });

  /**
   * Where the sheet came from. Measured **after** the render, because the row
   * it grew from only exists once the room is drawn — the same order the lab's
   * `applyOrigin()` uses, and the reason the animation is added in this pass
   * rather than declared in the markup.
   */
  $effect(() => {
    const sheet = sheetEl;
    const row = root?.querySelector('[data-origin="1"]');
    if (!sheet || !row) return;
    const from = row.getBoundingClientRect();
    const box = sheet.getBoundingClientRect();
    if (box.width === 0) return;
    sheet.style.setProperty('--dx', `${Math.round(from.left + from.width / 2 - (box.left + box.width / 2))}px`);
    sheet.style.setProperty('--dy', `${Math.round(from.top + from.height / 2 - (box.top + box.height / 2))}px`);
    sheet.style.setProperty('--k', Math.max(0.55, Math.min(0.95, from.width / box.width)).toFixed(3));
    sheet.removeAttribute('data-grow');
    void sheet.offsetWidth;
    sheet.setAttribute('data-grow', '');
  });

  /** A click outside the sheet closes it. The path is read from the event, not
   *  from the live tree: a control that closes its own popup as it is clicked
   *  (the figure's screen picker) has already detached by the time a window
   *  listener runs, and a detached target's `closest()` can no longer see the
   *  sheet it was clicked in — so one pick closed the whole sheet.
   *  `composedPath()` is the dispatch-time chain, which still names the sheet. */
  function dismissOutside(event: MouseEvent): void {
    if (!armed || !editor) return;
    if (sheetEl && !event.composedPath().includes(sheetEl)) acts.onClose();
  }
</script>

<svelte:window onclick={dismissOutside} />

<div class="v-fit sys-a" bind:this={root}>
  <Head {strip} {query} {places} {place} {recovering} {acts} />
  {#if receipt}<Receipt {receipt} {acts} />{/if}

  {#if searching}
    <Search {hits} {acts} />
  {:else if recovering}
    <Recovery {backups} {restoring} {acts} />
  {:else if editor}
    <!-- The room behind the sheet: the place's own index, live and dimmed under
         the scrim, with the row the sheet grew from still carrying its own
         `aria-expanded`. The screen does not swap. -->
    <div class="ya-room">
      <Place place={where} rows={here.rows} groups={here.groups} {openKey} room {acts} />
    </div>
  {:else}
    <Place place={where} rows={here.rows} groups={here.groups} {openKey} {acts} />
    {#if place === 'screens'}<Rail {rail} {iconNames} {acts} />{/if}
  {/if}
</div>

{#if editor}
  <div class="cd-scrim" role="presentation" data-command="app.dismiss"></div>
  <div
    class="cd-sheet"
    role="dialog"
    aria-modal="true"
    aria-label={editor.title}
    tabindex="-1"
    bind:this={sheetEl}
  >
    <header class="cd-sheet__head">
      <div>
        <h2 class="cd-sheet__title">{editor.title}</h2>
        <p class="cd-sheet__sub">{editor.sub}</p>
      </div>
      <button class="cd-iconbtn" type="button" onclick={() => acts.onClose()} aria-label="Close">✕</button>
    </header>

    <div class="cd-sheet__body">
      {#if receipt}<Receipt {receipt} {acts} />{/if}
      <Editor {editor} {projection} {busy} frame="sheet" {acts} />
    </div>

    <footer class="cd-sheet__foot">
      <span class="cd-sheet__note">Every write here is a transaction — ⌘Z brings it back.</span>
      <span class="cd-sheet__spacer"></span>
      <button class="cd-pill cd-pill--quiet" type="button" onclick={() => acts.onClose()}>Done</button>
    </footer>
  </div>
{/if}

<style>
  @keyframes sys-grow {
    from {
      transform: translate(calc(-50% + var(--dx, 0px)), calc(-50% + var(--dy, 0px))) scale(var(--k, 0.8));
    }
    to { transform: translate(-50%, -50%) scale(1); }
  }
  .cd-sheet[data-grow] { animation: sys-grow var(--dur-3) var(--ease-pop) both; }

  .ya-room { padding: var(--ui-pad) 0; }
  .sys-a { display: grid; gap: var(--ui-gap); }

  @media (prefers-reduced-motion: reduce) {
    .cd-sheet[data-grow] { animation: none; }
  }
</style>
