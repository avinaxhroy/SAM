<!--
  A context menu — the OS's own where there is one (§4.10: right-click menus
  that are the platform's, per row, cell and selection), DOM where there is not.

  The item list is the caller's, built from the registry (`commands/registry.ts`
  declares each context's ids); what this component decides is *rendering*. A
  native menu is requested first; when the shell lacks a native menu API
  (`ipc.popupMenu` returns null, as in browser harnesses), DOM items are rendered
  (§4.10: one list, two renderings).

  **The keyboard belongs to the DOM rendering only.** Everything below — the
  first-item focus on open, Arrow/Home/End, the caret handed back to whatever
  held it when the menu mounted — is inert while the platform's own menu is in
  play (`domMenu` is false until the shell has said it has none), because a
  native menu runs its own event loop and steals nothing from it.
-->
<script lang="ts">
  import { popupMenu, type MenuItemSpec } from '../ipc';
  import { claimMenuChoices } from '../commands/menuBus';
  import type { MenuRow } from '../commands/registry';

  let {
    rows,
    onclose,
    native = true,
  }: {
    rows: MenuRow[];
    onclose: () => void;
    /**
     * Ask the shell for the platform's own menu first. A right-click gets it
     * (§4.10); a left-click on a column header is an in-window menu, because
     * the click that opened it is the app's gesture, not the OS's.
     */
    native?: boolean;
  } = $props();

  const isAction = (row: MenuRow): row is Exclude<MenuRow, { separator: true }> =>
    !('separator' in row && row.separator);

  let nativeTried = $state(false);

  /**
   * The shell has answered "no native menu here" (`native={false}` never asks,
   * and an in-window menu is the rendering from the first frame).
   */
  let noNative = $state(false);

  /**
   * True while the DOM list below is the rendering. `native={false}` starts
   * here; the native case flips it only when `popupMenu` has answered `false`.
   */
  const domMenu = $derived(!native || noNative);

  /** The list itself: the focus boundary for the keys below. */
  let list = $state<HTMLElement | null>(null);

  /**
   * What held the keyboard when this menu mounted. Read at component init —
   * the menu mounts inside the click that opened it, and an effect would run
   * after that click has finished, by which time focus may have moved on.
   */
  const opener: HTMLElement | null =
    typeof document === 'undefined' ? null : (document.activeElement as HTMLElement | null);

  /** The enabled items, in DOM order — the arrows walk this and nothing else. */
  function items(): HTMLElement[] {
    const root = list;
    if (!root) return [];
    return Array.from(root.querySelectorAll<HTMLElement>('button:not([disabled])')).filter(
      (el) => el.getClientRects().length > 0,
    );
  }

  function step(delta: 1 | -1): void {
    const all = items();
    if (all.length === 0) return;
    const at = all.indexOf(document.activeElement as HTMLElement);
    // A menu opened with nothing under the caret starts at its near end, and
    // the walk wraps: the list is a ring, not a dead end.
    const next = at === -1 ? (delta === 1 ? 0 : all.length - 1) : (at + delta + all.length) % all.length;
    all[next]?.focus();
  }

  function edge(last: boolean): void {
    const all = items();
    all[last ? all.length - 1 : 0]?.focus();
  }

  function spec(rows: MenuRow[]): MenuItemSpec[] {
    return rows.map((row) =>
      isAction(row)
        ? { id: row.id, title: row.title, enabled: row.enabled !== false }
        : { separator: true },
    );
  }

  async function run(row: MenuRow): Promise<void> {
    if (!isAction(row) || row.enabled === false) return;
    onclose();
    await row.run();
  }

  $effect(() => {
    // One attempt per mount: the shell either has the platform's menu API or it
    // does not, and asking twice would open the native menu twice.
    if (nativeTried || !native) return;
    nativeTried = true;
    // While this menu is open it claims its own ids, because its actions are
    // closures only this component holds.
    const release = claimMenuChoices((id) => {
      const row = rows.find((candidate) => isAction(candidate) && candidate.id === id);
      if (!row) return false;
      void run(row);
      return true;
    });
    void (async () => {
      const shown = await popupMenu(spec(rows));
      // `false` = no native menu here, so the DOM below is the rendering; the
      // claim stays registered either way and is released on unmount.
      if (!shown) noNative = true;
    })();
    return release;
  });

  /**
   * On open the keyboard goes to the first enabled item, else to the menu
   * itself (which carries `tabindex="-1"`), so Escape always has somewhere to
   * land. Nothing moves under a native menu.
   */
  $effect(() => {
    if (!domMenu) return;
    const root = list;
    if (!root || root.contains(document.activeElement)) return;
    (items()[0] ?? root).focus();
  });

  /**
   * On close the caret goes back to whatever held it when the menu mounted —
   * the fallback, not a second owner: an action that opens a sheet focuses into
   * that sheet, and this only runs where the menu itself was the rendering.
   */
  $effect(() => {
    if (!domMenu) return;
    const back = opener;
    return () => {
      if (back?.isConnected && back !== document.body) back.focus();
    };
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      onclose();
      return;
    }
    if (!domMenu) return;
    // The arrows drive the menu only while it holds the keyboard; a menu left
    // open beside some other focused control never eats its arrow keys.
    const root = list;
    if (!root || !root.contains(document.activeElement)) return;
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        step(1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        step(-1);
        break;
      case 'Home':
        event.preventDefault();
        edge(false);
        break;
      case 'End':
        event.preventDefault();
        edge(true);
        break;
    }
  }
</script>

<svelte:window
  onclick={(event) => {
    // A click inside the menu is the menu's; anywhere else dismisses it. The
    // test is the DOM's own containment rather than `stopPropagation`, so the
    // menu markup stays a plain list of items.
    const target = event.target as Element | null;
    if (!target?.closest?.('.cd-menu')) onclose();
  }}
  onkeydown={onkeydown}
/>

<div class="cd-menu cd-menu--context" role="menu" tabindex="-1" bind:this={list}>
  {#each rows as row, index (index)}
    {#if isAction(row)}
      <button
        type="button"
        role="menuitem"
        class:cd-danger={row.danger}
        class:cd-menu__row--note={row.note !== undefined}
        disabled={row.enabled === false}
        data-command={row.id}
        onclick={() => run(row)}
      >
        <span class="cd-menu__rowtext">
          <span>{row.title}</span>
          {#if row.note}<span class="cd-menu__note">{row.note}</span>{/if}
        </span>
        {#if row.hint}<kbd class="cd-kbd">{row.hint}</kbd>{/if}
      </button>
    {:else}
      <hr />
    {/if}
  {/each}
</div>

<style>
  /* The menu arrives: a short fade and a small lift (`--dur-1`), the sheet's own
     arrival at the menu's scale. The keyframe states only the start, so the
     element's own position is untouched once the animation ends. */
  .cd-menu--context {
    animation: cd-menu-in var(--dur-1) var(--ease-pop);
  }

  @keyframes cd-menu-in {
    from { opacity: 0; transform: translateY(-4px); }
  }

  @media (prefers-reduced-motion: reduce) {
    .cd-menu--context { animation: none; }
  }
</style>
