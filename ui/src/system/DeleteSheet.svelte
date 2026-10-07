<!--
  Delete confirmation sheet (P3 · U6, UI_SPEC §4 E2, design/controls.md §3 L3).
  Displays consequence sentences parsed from engine dry-run payload rather than JSON (D11).
  Uses system sheet layer hierarchy (UI_SPEC §2.2, styles/sheets.css).
  Rules:
    - Destructive button text states affected count (R13, R5).
    - Typed key confirmation required when records will be removed (E2).
    - Unresolved or missing counts render retry prompt without fabricated values.
-->
<script lang="ts">
  import IdPair from '../shell/IdPair.svelte';
  import '../styles/sheets.css';

  let {
    request,
    preview,
    onclose,
    onrun,
    busy = false,
    error = null,
  }: {
    /** Deletion target descriptor. */
    request: {
      command: string;
      params: Record<string, string>;
      title: string;
      subject: string;
      key: string;
      note?: string;
    };
    /** Dry-run preview payload containing affected entity counts (D11). */
    preview: unknown;
    onclose: () => void;
    onrun: (params: Record<string, unknown>) => void;
    busy?: boolean;
    error?: string | null;
  } = $props();

  function plural(count: number): string {
    return count === 1 ? '' : 's';
  }

  function member(source: unknown, key: string): unknown {
    return source !== null && typeof source === 'object'
      ? (source as Record<string, unknown>)[key]
      : undefined;
  }

  function count(value: unknown): number {
    return typeof value === 'number' && Number.isFinite(value) && value > 0
      ? Math.trunc(value)
      : 0;
  }

  const readable = $derived(preview !== null && typeof preview === 'object');
  const records = $derived(readable ? count(member(preview, 'records')) : 0);

  const noun = $derived(
    request.command.startsWith('view.')
      ? 'view'
      : request.command.startsWith('list.')
        ? 'destination'
        : 'kind',
  );

  /** Formats dry-run preview counts into human-readable consequence statements. */
  const lines = $derived.by<string[]>(() => {
    if (!readable) return [];
    const source = preview as Record<string, unknown>;
    const removed = member(source, 'removed');
    const found: string[] = [];

    const gone = count(member(source, 'records'));
    if (gone > 0) found.push(`${gone} record${plural(gone)} go${gone === 1 ? 'es' : ''} with it`);

    const links = count(member(source, 'unlinked'));
    if (links > 0) found.push(`${links} link${plural(links)} from other records ${links === 1 ? 'is' : 'are'} cut`);

    const columns = count(member(removed, 'columns')) || count(member(source, 'relations'));
    if (columns > 0) {
      found.push(
        `${columns} column${plural(columns)} that pointed at it ${columns === 1 ? 'is' : 'are'} removed`,
      );
    }

    const parents = count(member(source, 'parents'));
    if (parents > 0) {
      found.push(
        `${parents} kind${plural(parents)} that filed under it ${parents === 1 ? 'loses' : 'lose'} that link`,
      );
    }

    const settings = count(member(removed, 'viewKeys'));
    if (settings > 0) {
      found.push(
        `${settings} view setting${plural(settings)} that named it ${settings === 1 ? 'is' : 'are'} removed`,
      );
    }

    const drawn = count(member(removed, 'blocks')) || count(member(source, 'blocks'));
    if (drawn > 0) found.push(`${drawn} block${plural(drawn)} that read${drawn === 1 ? 's' : ''} through it`);

    const metrics = count(member(removed, 'metrics'));
    if (metrics > 0) {
      found.push(
        `${metrics} metric${plural(metrics)} that folded it ${metrics === 1 ? 'is' : 'are'} removed`,
      );
    }

    const views = count(member(source, 'views'));
    const sidebar = count(member(source, 'sidebar'));
    if (views > 0) {
      const of = `${views} view${plural(views)}`;
      if (sidebar === views) found.push(`${of} and ${views === 1 ? 'its destination' : 'their destinations'}`);
      else if (sidebar > 0) found.push(`${of} and ${sidebar} destination${plural(sidebar)}`);
      else found.push(of);
    } else if (sidebar > 0) {
      found.push(`${sidebar} destination${plural(sidebar)} on the rail`);
    }

    return found;
  });

  /** A view the engine puts back in place of the one going — named, not guessed. */
  const replacement = $derived(
    readable && typeof member(preview, 'recreated') === 'string'
      ? String(member(preview, 'recreated'))
      : null,
  );

  let typed = $state('');

  /**
   * The refusal, when the caller kept one. `error` is the engine's own sentence
   * (`role="alert"`), and nothing else: a caller that hands over the dry run's
   * text form — a JSON object — is not handing over a refusal, and this surface
   * is the one that exists so no engine JSON is ever printed (D11). The payload
   * is already stated above, as sentences.
   */
  const refusal = $derived.by<string | null>(() => {
    if (typeof error !== 'string') return null;
    const text = error.trim();
    if (text === '') return null;
    if (text.startsWith('{') || text.startsWith('[')) return null;
    return text;
  });

  /** E2: a typed key is required only when records go with the kind. */
  const mustType = $derived(records > 0);
  const confirmed = $derived(!mustType || typed.trim() === request.key);

  const button = $derived(
    records > 0
      ? `Delete the ${noun} and ${records} record${plural(records)}`
      : `Delete this ${noun}`,
  );

  /**
   * The commit. `mode: 'records'` is stated explicitly when records are going —
   * the engine's default, said out loud so the button and the write agree.
   */
  function commit(): void {
    const params: Record<string, unknown> = { ...request.params };
    if (records > 0 && params.mode === undefined) params.mode = 'records';
    onrun(params);
  }

  function onkeydown(event: KeyboardEvent): void {
    // Escape abandons the sheet; nothing is destroyed until the click.
    if (event.key === 'Escape') onclose();
  }

  /**
   * Inert until the click that opened this sheet has finished propagating: a
   * window listener registered during that dispatch receives the same event,
   * with the opener as its target, and would dismiss the sheet immediately
   * (`shell/Sheet.svelte` documents the same guard).
   */
  let armed = $state(false);

  $effect(() => {
    const id = setTimeout(() => (armed = true), 0);
    return () => clearTimeout(id);
  });

  function dismissOutside(event: MouseEvent): void {
    if (!armed) return;
    const target = event.target as Element | null;
    if (!target?.closest?.('.cd-sheet')) onclose();
  }
</script>

<svelte:window onkeydown={onkeydown} onclick={dismissOutside} />

<div class="cd-scrim" role="presentation" data-command="app.dismiss"></div>

<div class="cd-sheet" role="dialog" aria-modal="true" aria-label={request.title} tabindex="-1">
  <header class="cd-sheet__head">
    <div>
      <h2 class="cd-sheet__title">{request.title}</h2>
      {#if request.note}<p class="cd-sheet__sub">{request.note}</p>{/if}
    </div>
    <button class="cd-iconbtn" type="button" onclick={onclose} aria-label="Close">✕</button>
  </header>

  <div class="cd-sheet__body">
    <div>
      {#if request.key !== ''}
        <div class="cd-sheet__row">
          <IdPair label={request.subject} title={`Copy the key of ${request.subject}`} value={request.key} />
        </div>
      {/if}

      {#if !readable}
        <p class="cd-sheet__note">
          The engine did not say what this would take with it. Nothing has been deleted — press the
          button to try again, and it will refuse rather than take anything it cannot name.
        </p>
      {:else}
        {#if lines.length === 0}
          <p class="cd-sheet__note">Nothing else changes: just the {noun} itself.</p>
        {:else}
          <p class="cd-sheet__note">What goes with it:</p>
          <ul class="cd-del__lines">
            {#each lines as line (line)}
              <li class="cd-del__line">{line}</li>
            {/each}
          </ul>
        {/if}
        {#if replacement}
          <div class="cd-sheet__row">
            <span class="cd-sheet__hint">A view is put back in its place:</span>
            <IdPair label="Put back" value={replacement} />
          </div>
        {/if}
      {/if}

      {#if mustType}
        <div class="cd-sheet__field">
          <label class="cd-sheet__label" for="del-confirm">
            Type {request.key} to confirm
          </label>
          <input
            id="del-confirm"
            class="cd-sheet__well"
            type="text"
            autocomplete="off"
            spellcheck="false"
            aria-label={`Type ${request.key} to confirm`}
            value={typed}
            oninput={(event) => (typed = event.currentTarget.value)}
          />
        </div>
      {/if}

      {#if refusal}
        <p class="cd-sheet__error" role="alert">{refusal}</p>
      {/if}
    </div>
  </div>

  <footer class="cd-sheet__foot">
    <span class="cd-sheet__note">
      Nothing is deleted until you press the button, and ⌘Z brings it back.
    </span>
    <span class="cd-sheet__spacer"></span>
    <button class="cd-pill cd-pill--quiet" type="button" onclick={onclose}>Keep it</button>
    <button
      class="cd-pill"
      type="button"
      data-command={request.command}
      data-placement="system"
      disabled={busy || !confirmed}
      onclick={commit}
    >
      {button}
    </button>
  </footer>
</div>

<style>
  /* The consequence is a list of facts, so it is a plain list of sentences —
     one per line, in the sheet's own small voice. */
  .cd-del__lines {
    display: grid;
    gap: var(--space-3xs);
    margin: 0;
    padding-left: var(--space-md);
    list-style: disc;
  }
  .cd-del__line {
    font-size: var(--text-xs);
    color: var(--ink-2);
    line-height: 1.5;
  }
</style>
