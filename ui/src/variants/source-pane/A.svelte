<!--
  Source Pane · Variant A (The Door).
  Inline document and record source editor pane (Appendix C.4).
  Provides raw file/record viewing and editing, target switching, syntax/diagnostic
  findings display, and transactional save/commit dispatch.
-->
<script lang="ts">
  import type { SourcePaneProps, SourceStatus } from './props';

  let {
    options,
    file,
    identity,
    text,
    findings,
    loading,
    pointed,
    external,
    status,
    receipt,
    commitKey,
    toggleKey,
    onPick,
    onText,
    onCommit,
    onReload,
    onClose,
    onCopy,
    onFlag,
  }: SourcePaneProps = $props();

  /** Visible diagnostics capped at six entries. */
  const shown = $derived(findings.slice(0, 6));

  /** The non-blank lines of what is in the well — a record's one line, a file's many. */
  const lines = $derived(text === '' ? 0 : text.split('\n').filter((line) => line.trim() !== '').length);

  /** Maps commit status to button label. */
  function wordOf(which: SourceStatus): string {
    if (which === 'working') return 'Committing…';
    if (which === 'done') return 'Committed';
    return `Commit ${commitKey}`;
  }

  /** Accessible status announcement for the open document. */
  const readout = $derived(
    loading
      ? 'Reading the document…'
      : pointed
        ? `${identity?.label || identity?.key || 'the document'} — ${lines} ${lines === 1 ? 'line' : 'lines'}`
        : '',
  );

  // ── the target list ────────────────────────────────────────────────────
  /** Whether the target file menu is open. */
  let picking = $state(false);
  let box = $state<HTMLDivElement | undefined>(undefined);
  let menu = $state<HTMLUListElement | undefined>(undefined);
  let trigger = $state<HTMLButtonElement | undefined>(undefined);

  /** Arrows move the option, Enter presses it (the rows are buttons), Escape closes. */
  function move(event: KeyboardEvent): void {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    const rows = Array.from(menu?.querySelectorAll<HTMLElement>('.sp-opt') ?? []);
    const at = rows.indexOf(document.activeElement as HTMLElement);
    const next = rows[Math.min(rows.length - 1, Math.max(0, at + (event.key === 'ArrowDown' ? 1 : -1)))];
    next?.focus();
  }

  function choose(key: string): void {
    picking = false;
    onPick(key);
    trigger?.focus();
  }

  /** Keyboard shortcuts: Mod-Enter commits, Escape closes dropdown or pane. */
  function keys(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      if (picking) {
        picking = false;
        trigger?.focus();
        return;
      }
      onClose();
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
      event.preventDefault();
      onCommit();
    }
  }

  // Dismiss dropdown on outside clicks.
  $effect(() => {
    if (!picking) return;
    const away = (event: MouseEvent) => {
      const node = event.target as Node | null;
      if (node && box?.contains(node)) return;
      picking = false;
    };
    document.addEventListener('click', away);
    return () => document.removeEventListener('click', away);
  });

  // Focus active option on menu open.
  $effect(() => {
    if (!picking) return;
    const here = menu?.querySelector<HTMLElement>('[data-current]') ?? menu?.querySelector<HTMLElement>('.sp-opt');
    here?.focus();
  });
</script>

<section class="cd-card src-pane sp-pane v-fit" data-pane="source" aria-label="Source" onkeydown={keys}>
  <header class="sp-head">
    <div class="sp-head__top">
      <h2 class="cd-card__title">Source</h2>
      <!-- The control the shipped pane never drew: the only door out was the
           titlebar's toggle. It runs the same command. -->
      <button
        class="sp-close"
        type="button"
        aria-label="Close the source pane"
        title={toggleKey ? `Close the source pane (${toggleKey})` : 'Close the source pane'}
        data-command="app.toggleSourcePane"
        data-placement="app.window"
        onclick={onClose}
      >
        <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" aria-hidden="true" focusable="false"><path d="M4.4 4.4 11.6 11.6M11.6 4.4 4.4 11.6" /></svg>
      </button>
    </div>

    <!-- Target identity display (§D2): human label and engine address key. -->
    {#if identity}
      <p class="sp-ident">
        {#if identity.label !== ''}
          <span class="sp-ident__label">{identity.label}</span>
        {/if}
        <button
          class="sp-ident__key"
          type="button"
          data-identity
          data-copy={identity.address}
          title={`Copy “${identity.address}”`}
          aria-label={`Copy ${identity.address}`}
          onclick={onCopy}
        >
          {identity.key}
        </button>
      </p>
    {/if}
  </header>

  <div class="sp-pick" bind:this={box}>
    <span class="sp-pick__label" id="src-pick-label">Target</span>
    <button
      class="sp-target"
      type="button"
      aria-haspopup="listbox"
      aria-expanded={picking}
      aria-labelledby="src-pick-label src-target-value"
      data-developer
      bind:this={trigger}
      onclick={() => (picking = !picking)}
    >
      <span class="sp-target__value" id="src-target-value">{file === '' ? 'no target' : file}</span>
      <svg class="sp-target__chev" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M4.4 6.4 8 10l3.6-3.6" /></svg>
    </button>

    {#if picking}
      <ul class="sp-menu" role="listbox" aria-labelledby="src-pick-label" data-developer bind:this={menu} onkeydown={move}>
        {#each options as option (option.key)}
          <li>
            <button
              class="sp-opt"
              type="button"
              role="option"
              aria-selected={option.current}
              data-current={option.current ? '' : undefined}
              onclick={() => choose(option.key)}
            >
              <span class="sp-opt__path">{option.label}</span>
              {#if option.tail !== ''}
                <span class="sp-opt__tail">{option.tail}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if external}
    <p class="sp-banner" role="status">
      <span>File changed externally — Reload to take it, or Commit over it</span>
      <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" onclick={onReload}>Reload</button>
    </p>
  {/if}

  {#if pointed}
    <!-- Raw source document editor (data-developer, §D2). -->
    <div class="sp-well" data-developer data-only={loading ? 'loading' : 'ready'}>
      <div class="sp-skel" aria-hidden="true">
        <span class="cd-skel" style="--w: 78%"></span>
        <span class="cd-skel" style="--w: 92%"></span>
        <span class="cd-skel" style="--w: 54%"></span>
        <span class="cd-skel" style="--w: 86%"></span>
      </div>
      <textarea
        class="sp-doc"
        spellcheck="false"
        autocomplete="off"
        autocapitalize="off"
        aria-label="Document text"
        value={text}
        oninput={(event) => onText((event.currentTarget as HTMLTextAreaElement).value)}
      ></textarea>
    </div>
    <p class="cd-sr" role="status">{readout}</p>

    {#if shown.length > 0}
      <!-- Diagnostic findings with line address and highlight binding. -->
      <ul class="sp-findings" data-developer>
        {#each shown as finding, index (`${finding.code}:${finding.line ?? 0}:${index}`)}
          <li
            class="sp-finding"
            onmouseenter={() => onFlag(finding)}
            onmouseleave={() => onFlag(null)}
            onfocusin={() => onFlag(finding)}
            onfocusout={() => onFlag(null)}
          >
            <span class="sp-finding__glyph" aria-hidden="true">⚠</span>
            <span class="sp-finding__body">
              <span class="sp-finding__msg">{finding.message}</span>
              <span class="sp-finding__addr">{finding.path}{finding.line === null ? '' : `:${finding.line}`} · {finding.code}</span>
            </span>
          </li>
        {/each}
        {#if findings.length > 6}
          <li class="sp-more">+{findings.length - 6} more</li>
        {/if}
      </ul>
    {/if}

    <footer class="sp-foot">
      {#if receipt}
        <span class="sp-receipt" role="status" data-developer>{receipt}</span>
      {:else}
        <span class="sp-count" role="status">{lines} {lines === 1 ? 'line' : 'lines'}</span>
      {/if}
      <!-- Commit button with status transitions. -->
      <button
        class="cd-pill sp-save"
        type="button"
        data-command="source.apply"
        data-placement="sourcePane"
        data-status={status}
        title={`Commit (${commitKey})`}
        disabled={status !== 'idle' || file === ''}
        onclick={onCommit}
      >
        <!-- Stacked status labels to size the button to the widest label without layout shift. -->
        <span class="sp-save__words">
          {#each ['idle', 'working', 'done'] as which (which)}
            <span class="sp-save__word" data-on={which === status ? '' : undefined}>{wordOf(which)}</span>
          {/each}
        </span>
        <span class="sp-save__bead" aria-hidden="true">
          <svg class="sp-save__spin" width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" opacity=".4" d="M12 2A10 10 0 1 0 22 12A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8A8 8 0 0 1 12 20Z" /><path fill="currentColor" d="M20 12h2A10 10 0 0 0 12 2V4A8 8 0 0 1 20 12Z" /></svg>
          <svg class="sp-save__tick" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.4 8.4l3 3 6.2-7" /></svg>
        </span>
      </button>
    </footer>
  {:else}
    <p class="cd-empty">nothing selected — pick a record, or a document</p>
  {/if}
</section>

<style>
  /* ── THE PANE ───────────────────────────────────────────────────────────
     A column of the room: full width of the track the shell gives it (the
     declared width is `.cd-body > .src-pane` in `styles/source-pane.css`), the
     well taking the slack. */
  .sp-pane {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--ui-gap-sm);
    padding: var(--ui-pad);
    border-radius: 0;
    /* The findings are capped below and the well has a floor, so a short window
       is the only case that scrolls — and it scrolls the pane rather than
       letting the foot fall out of the column (the shipped card had neither). */
    overflow-y: auto;
  }

  /* ── THE HEAD ────────────────────────────────────────────────────────── */
  .sp-head {
    display: grid;
    gap: var(--space-2xs);
    min-width: 0;
  }
  .sp-head__top {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .sp-head__top .cd-card__title {
    flex: 1;
    min-width: 0;
  }
  .sp-close {
    flex: none;
    width: var(--hit);
    height: var(--hit);
    display: grid;
    place-items: center;
    border-radius: var(--r-pill);
    color: var(--ink-3);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  .sp-close:hover {
    background: var(--well);
    color: var(--ink);
  }

  /* The identity pair. A record reads as its label plus its id; a document
     reads as its path alone, because the engine has no other name for a file. */
  .sp-ident {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    min-width: 0;
  }
  .sp-ident__label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: calc(var(--text-xs) * var(--ui-s));
    color: var(--ink-2);
  }
  .sp-ident__key {
    flex: none;
    font-family: var(--font-mono);
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-3);
    background: transparent;
    border-radius: var(--r-key);
    padding: 0 var(--space-3xs);
    /* A document's key IS its path, and a path has no length the pane can
       promise: the key ellipsises rather than pushing the pane's own column. */
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* `IdPair`'s key is a 14px text button — under the tap floor. The box is
       grown to `--hit` and the row's air given back with a negative margin, the
       same repair the identity pair makes for the same reason. */
    min-width: var(--hit);
    min-height: var(--hit);
    padding-block: calc(9px * var(--ui-s));
    margin-block: calc(-9px * var(--ui-s));
  }
  .sp-ident__key:hover {
    background: var(--well-2);
    color: var(--ink);
  }

  /* ── THE TARGET (the engine's list) ──────────────────────────────────── */
  .sp-pick {
    position: relative;
    display: grid;
    gap: var(--space-3xs);
    min-width: 0;
  }
  .sp-pick__label {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    font-weight: var(--weight-label);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .sp-target {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    width: 100%;
    min-height: var(--hit);
    padding: 0 var(--space-xs);
    border-radius: var(--r-mini);
    background: var(--well);
    color: var(--ink);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .sp-target:hover {
    background: var(--well-2);
  }
  .sp-target__value {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: calc(var(--text-2xs) * var(--ui-s));
  }
  .sp-target__chev {
    flex: none;
    color: var(--ink-3);
  }

  /* Target file selection menu. */
  .sp-menu {
    position: absolute;
    top: calc(100% + var(--space-2xs));
    left: 0;
    right: 0;
    z-index: 3;
    max-height: calc(232px * var(--ui-s));
    overflow-y: auto;
    margin: 0;
    padding: var(--space-2xs);
    list-style: none;
    background: var(--card);
    border-radius: var(--r-tile);
    box-shadow: var(--sh-pop);
    animation: sp-drop var(--dur-2) var(--ease-pop) both;
  }
  @keyframes sp-drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .sp-opt {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    width: 100%;
    min-height: var(--hit);
    padding: 0 var(--space-xs);
    border-radius: var(--r-item);
    text-align: left;
    color: var(--ink-2);
    font-size: calc(var(--text-2xs) * var(--ui-s));
    transition: background var(--dur-1) var(--ease);
  }
  .sp-opt:hover,
  .sp-opt[data-current] {
    background: var(--well-2);
    color: var(--ink);
  }
  .sp-opt__path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }
  .sp-opt__tail {
    flex: none;
    margin-left: auto;
    color: var(--ink-3);
    font-family: var(--font-mono);
  }
  .sp-opt[data-current]::before {
    content: "·";
    flex: none;
    color: var(--ink-3);
  }
  /* High contrast colors for active and hovered menu rows. */
  .sp-opt:hover .sp-opt__tail,
  .sp-opt[data-current] .sp-opt__tail,
  .sp-opt:hover::before,
  .sp-opt[data-current]::before {
    color: var(--ink-2);
  }

  /* ── THE EXTERNAL-CHANGE BANNER (§4.8 P6) ────────────────────────────── */
  .sp-banner {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-2xs) var(--space-xs) var(--space-2xs) var(--space-sm);
    border-radius: var(--r-mini);
    background: var(--chip-risk);
    color: var(--on-risk);
    font-size: calc(var(--text-2xs) * var(--ui-s));
    line-height: 1.35;
  }
  .sp-banner > span {
    flex: 1;
    min-width: 0;
  }
  .sp-banner .cd-pill--quiet {
    background: color-mix(in oklab, var(--on-risk) 12%, transparent);
    color: var(--on-risk);
  }

  /* ── THE DOCUMENT'S WELL ─────────────────────────────────────────────── */
  .sp-well {
    flex: 1 1 auto;
    position: relative;
    min-height: calc(236px * var(--ui-s));
    border-radius: var(--r-mini);
    background: var(--well);
    overflow: hidden;
  }
  /* Focus ring shown on keyboard focus (:has(:focus-visible)). */
  .sp-well:has(.sp-doc:focus-visible) {
    box-shadow: 0 0 0 2px var(--focus);
  }
  .sp-doc {
    display: block;
    width: 100%;
    height: 100%;
    min-height: calc(236px * var(--ui-s));
    padding: var(--ui-pad-sm);
    border: 0;
    background: none;
    resize: none;
    outline: none;
    font-family: var(--font-mono);
    font-size: calc(var(--text-xs) * var(--ui-s));
    line-height: 1.55;
    color: var(--ink-2);
    caret-color: var(--ink);
    /* Preserve formatting without horizontal scrolling. */
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .sp-doc::selection {
    background: var(--well-2);
  }
  .sp-well[data-only="loading"] .sp-doc {
    visibility: hidden;
  }
  .sp-skel {
    position: absolute;
    inset: var(--ui-pad-sm);
    display: grid;
    align-content: start;
    gap: var(--space-xs);
  }
  .sp-skel .cd-skel {
    height: 12px;
  }
  .sp-well:not([data-only="loading"]) .sp-skel {
    display: none;
  }

  /* ── THE FINDINGS (the engine's own diagnostics) ─────────────────────── */
  .sp-findings {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3xs);
    /* Limit findings list height so well area retains adequate space. */
    max-height: calc(132px * var(--ui-s));
    overflow-y: auto;
  }
  .sp-finding {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2xs);
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--r-mini);
    background: var(--chip-risk);
    color: var(--on-risk);
    font-size: calc(var(--text-2xs) * var(--ui-s));
    line-height: 1.4;
  }
  .sp-finding__glyph {
    flex: none;
  }
  .sp-finding__body {
    flex: 1 1 auto;
    min-width: 0;
    display: grid;
    gap: 1px;
  }
  /* Diagnostic error address and code. */
  .sp-finding__addr {
    font-family: var(--font-mono);
    opacity: 0.86;
    overflow-wrap: anywhere;
  }
  .sp-more {
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-3);
  }

  /* ── THE FOOT: the count, and the screen's one action ────────────────── */
  /* Footer layout: line count and save button with full-width receipt below. */
  .sp-foot {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-2xs) var(--space-sm);
    padding-top: var(--space-2xs);
  }
  .sp-count {
    grid-column: 1;
    grid-row: 1;
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .sp-receipt {
    grid-column: 1 / -1;
    grid-row: 2;
    font-family: var(--font-mono);
    font-size: calc(var(--text-2xs) * var(--ui-s));
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* In wide layouts, render receipt inline between count and save button. */
  @container (min-width: 520px) {
    .sp-foot {
      grid-template-columns: minmax(0, 1fr) minmax(0, 46ch) auto;
    }
    .sp-receipt {
      grid-column: 2;
      grid-row: 1;
    }
    .sp-save {
      grid-column: 3;
    }
  }

  /* ── THE SAVE CONTROL AND ITS BEAD ─────────────────────────────────── */
  .sp-save {
    position: relative;
    grid-column: 2;
    grid-row: 1;
    justify-self: end;
    flex: none;
    justify-content: center;
    /* The pill is a control of this pane, so its own type step moves with the
       register like every other step in here. */
    font-size: calc(var(--text-xs) * var(--ui-s));
  }
  .sp-save__words {
    display: grid;
  }
  .sp-save__word {
    grid-area: 1 / 1;
    visibility: hidden;
    opacity: 0;
    transition: opacity var(--dur-2) var(--ease);
  }
  .sp-save__word[data-on] {
    visibility: visible;
    opacity: 1;
  }
  .sp-save[data-status="working"],
  .sp-save[data-status="done"] {
    background: var(--well-2);
    color: var(--ink);
  }
  .sp-save[disabled] {
    opacity: 1;
  }
  .sp-save__bead {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 20px;
    height: 20px;
    border-radius: var(--r-pill);
    background: var(--ink);
    color: var(--ink-inv);
    display: grid;
    place-items: center;
    box-shadow: 0 0 0 2px var(--card);
  }
  .sp-save[data-status="idle"] .sp-save__bead {
    display: none;
  }
  .sp-save__spin {
    animation: sp-turn var(--dur-loop) linear infinite;
  }
  .sp-save__tick {
    display: none;
  }
  .sp-save[data-status="done"] .sp-save__spin {
    display: none;
  }
  .sp-save[data-status="done"] .sp-save__tick {
    display: block;
  }
  @keyframes sp-turn {
    to {
      transform: rotate(360deg);
    }
  }

  /* ── FOCUS ────────────────────────────────────────────────────────────── */
  /* Explicit focus-visible ring styles for pane controls. */
  .sp-close:focus-visible,
  .sp-target:focus-visible,
  .sp-opt:focus-visible,
  .sp-ident__key:focus-visible,
  .sp-save:focus-visible,
  .sp-banner .cd-pill:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  /* The list's own rows sit inside the menu's box, so their ring is inset. */
  .sp-opt:focus-visible {
    outline-offset: -2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .sp-menu {
      animation: none;
    }
    /* `base.css` already caps every animation at 1ms/1 iteration; the turn is
       named here so the bead's arc is stated as still, not left to a cap. */
    .sp-save__spin {
      animation: none;
    }
  }
</style>
