<!--
  Settings · A · THE STENCIL.

  A settings page with a room of its own: an **index** down the left side and
  one **pane** of rows beside it, not a page of setting objects floating on the
  canvas.

    · a **room** — one bordered surface, so the screen has an edge. The index is
      a tinted plate ruled off the pane; the pane is the ruled page inside it.
    · an **index** (left) — one entry per place, each carrying that place's live
      value in the app's own words, so "what is set?" is answered without a
      click; the open place's glyph is the screen's one ink object;
    · a **pane** (right) — the open place as a sheet of rows: a glyph on a tile,
      the app's name for the setting, its key beside it, one sentence where a
      sentence teaches, and the single control its type declares at the row's
      trailing edge. Nothing repeats the value outside the control that owns it;
    · a **find field** in the head — it filters the page's own table and, while
      a query is live, the pane's body *is* the matches; choosing one opens the
      place it lives in and washes the row it names;
    · a **change strip** at the pane's foot — one line per accepted write, each
      reversible, struck when taken back.

  Every pane is mounted and the four closed ones are `hidden`: the registry's
  own inspection (`__SAM_RENDER_DUMP__`, Appendix C.6) walks the whole
  document, so a place whose controls are absent from the DOM would report the
  app's own writes as having no designed control. `hidden` keeps a closed pane
  out of the layout, the hit-testing and the accessibility tree while leaving it
  in the page the gate reads.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import type { ChangeLine, FindRow, SettingsSurfaceProps } from './props';
  import '../../styles/panel-settings.css';

  let {
    title,
    say,
    tally,
    places,
    panels,
    find,
    changes,
    onUndoChange,
    onDone,
  }: SettingsSurfaceProps = $props();

  /** The place the student opened, or `null` for the default: the first one,
   *  because the plan's own numbers are what a person opens settings to change. */
  let chosen = $state<string | null>(null);
  const open = $derived(chosen ?? places[0]?.id ?? 'day');

  /** The find query. Empty means the pane draws the place. */
  let query = $state('');

  /** The row a hit jumped to: the jump focuses the row's own control, and the
   *  row washes while the keyboard is inside it (`:focus-within`). The state is
   *  kept for the polite announcement of where the jump landed. */
  let marked = $state<string | null>(null);
  let markTimer: ReturnType<typeof setTimeout> | null = null;

  /** What the matches are, out of the same table the panes are built from. */
  const hits = $derived.by<FindRow[]>(() => {
    const needle = query.trim().toLowerCase();
    if (needle.length === 0) return [];
    return find.filter(
      (row) =>
        row.name.toLowerCase().includes(needle) ||
        row.key.toLowerCase().includes(needle) ||
        (places.find((place) => place.id === row.group)?.name ?? '').toLowerCase().includes(needle),
    );
  });

  const here = $derived(places.find((place) => place.id === open) ?? places[0]);

  /** The name and glyph a hit's row is filed under. */
  const placeOf = (id: string) => places.find((place) => place.id === id);
  const placeName = (id: string): string => placeOf(id)?.name ?? id;
  const placeIcon = (id: string): string => placeOf(id)?.icon ?? 'dot';

  /** Split a setting's name on the query so the matched run can carry the
   *  weight — the mark is the app's own words, never a rewrite of them. */
  function parts(name: string): { text: string; hit: boolean }[] {
    const needle = query.trim();
    if (needle.length === 0) return [{ text: name, hit: false }];
    const at = name.toLowerCase().indexOf(needle.toLowerCase());
    if (at < 0) return [{ text: name, hit: false }];
    return [
      { text: name.slice(0, at), hit: false },
      { text: name.slice(at, at + needle.length), hit: true },
      { text: name.slice(at + needle.length), hit: false },
    ].filter((part) => part.text.length > 0);
  }

  /** Open the place a hit names, drop the query, and put the keyboard on the
   *  row's own control — a jump that states where it lands and moves nothing. */
  function jump(hit: FindRow): void {
    chosen = hit.group;
    query = '';
    marked = hit.key;
    if (markTimer !== null) clearTimeout(markTimer);
    markTimer = setTimeout(() => {
      marked = null;
      markTimer = null;
    }, 1400);
    requestAnimationFrame(() => {
      const row = document.querySelector<HTMLElement>(`[data-plate="${hit.key}"]`);
      if (!row) return;
      row.scrollIntoView({ block: 'nearest' });
      // The row's *control* is the target, not the first focusable thing in it
      // — that is the key chip beside the setting's name, and landing on the
      // copy affordance when the student asked for the setting is a jump to the
      // wrong place.
      const control =
        row.querySelector<HTMLElement>('.st-row__ctl .st-budget__track') ??
        row.querySelector<HTMLElement>('.st-row__ctl input') ??
        row.querySelector<HTMLElement>('.st-row__ctl [role="switch"]') ??
        row.querySelector<HTMLElement>('.st-row__ctl button') ??
        row.querySelector<HTMLElement>('input, button, [tabindex]');
      control?.focus();
    });
  }

  function clearFind(): void {
    query = '';
  }

  let field: HTMLInputElement | null = $state(null);

  function onFieldKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' && query.length > 0) {
      event.stopPropagation();
      clearFind();
    }
    if (event.key === 'Enter' && hits.length > 0) {
      event.preventDefault();
      jump(hits[0]);
    }
  }
</script>

<div class="st-page">
  <header class="st-head">
    <div>
      <h1 class="st-head__title">{title}</h1>
      <p class="st-head__say">{say}</p>
      <div class="st-head__tally">
        {#each tally as fact (fact.text)}
          <span class="st-tally">
            <Icon name={fact.icon} size={13} />
            {fact.text}
          </span>
        {/each}
      </div>
    </div>
    <div class="st-head__tools">
      <div class="st-find">
        <Icon name="search" size={14} />
        <label class="cd-sr" for="st-find">Find a setting</label>
        <input
          id="st-find"
          class="st-find__field"
          type="search"
          autocomplete="off"
          spellcheck="false"
          placeholder="Find a setting"
          bind:this={field}
          bind:value={query}
          onkeydown={onFieldKey}
        />
        {#if query.length > 0}
          <span class="st-find__count" aria-live="polite">
            {hits.length === 1 ? '1 match' : `${hits.length} matches`}
          </span>
          <button
            class="st-find__clear"
            type="button"
            aria-label="Clear the find field"
            onclick={clearFind}
          >
            <Icon name="close" size={12} />
          </button>
        {:else}
          <span class="st-find__count">{find.length} settings</span>
        {/if}
      </div>
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" onclick={onDone}>Done</button>
    </div>
  </header>

  <div class="st-room">
    <aside class="st-side" aria-label="Where a setting lives">
      <div class="st-side__inner">
        <header class="st-side__head">
          <span class="st-side__label">Places</span>
          <span class="st-side__count">{places.length}</span>
        </header>
        <nav class="st-rail" aria-label="Places in settings">
          {#each places as place (place.id)}
            <button
              class="st-rail__item"
              type="button"
              aria-current={open === place.id && query.length === 0}
              aria-label={`${place.name} — ${place.summary}`}
              onclick={() => {
                chosen = place.id;
                query = '';
              }}
            >
              <span class="st-rail__ico"><Icon name={place.icon} size={15} /></span>
              <span class="st-rail__name">{place.name}</span>
              <span class="st-rail__sum">{place.summary}</span>
            </button>
          {/each}
        </nav>
      </div>
    </aside>

    <div class="st-pane">
      {#if query.trim().length > 0}
        <section class="st-sheet" aria-label="Settings that match">
          <div class="st-hits">
            <div class="st-hits__top">
              <p class="st-hits__say">
                {#if hits.length === 0}
                  Nothing matches <b>“{query.trim()}”</b>.
                {:else}
                  {hits.length === 1 ? 'One setting matches' : `${hits.length} settings match`}
                  <b>“{query.trim()}”</b>.
                {/if}
              </p>
            </div>
            {#each hits as hit (hit.group + '·' + hit.key)}
              <button class="st-hits__row" type="button" onclick={() => jump(hit)}>
                <span class="st-hits__ico" aria-hidden="true">
                  <Icon name={placeIcon(hit.group)} size={15} />
                </span>
                <span class="st-hits__name">
                  {#each parts(hit.name) as part (part.text + part.hit)}
                    {#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}
                  {/each}
                </span>
                <span class="st-hits__where">{placeName(hit.group)}</span>
              </button>
            {/each}
            {#if hits.length === 0}
              <p class="st-hits__none">
                Five places: the day · how you work · appearance · component styles · this plan.
              </p>
            {/if}
          </div>
        </section>
      {:else}
        {#if here}
          <header class="st-pane__head">
            <h2 class="st-pane__title">{here.name}</h2>
            <p class="st-pane__sum">{here.summary}</p>
            {#if here.lead}
              <p class="st-pane__lead">{here.lead}</p>
            {/if}
          </header>
        {/if}
      {/if}

      <!-- Every place is mounted; the closed ones are hidden, not unmounted. -->
      <div hidden={query.trim().length > 0}>
        {#each places as place (place.id)}
          <div hidden={open !== place.id}>
            {#if marked !== null && find.some((row) => row.key === marked && row.group === place.id)}
              <span class="cd-sr" aria-live="polite">{place.name} — the setting that matched is marked</span>
            {/if}
            <div class="st-sheet">
              {@render panels[place.id]()}
            </div>
          </div>
        {/each}
      </div>

      <footer class="st-strip" aria-label="Changed this session">
        <div class="st-strip__top">
          <h2 class="st-strip__title">Changed this session</h2>
          {#if changes.length > 0}
            <span class="st-strip__count">{changes.length}</span>
          {/if}
        </div>
        {#if changes.length === 0}
          <p class="st-strip__empty">Nothing written yet.</p>
        {:else}
          <ol class="st-strip__lines">
            {#each changes as line (line.id)}
              <li class="st-strip__line" data-taken={line.taken ? '' : undefined}>
                <span class="st-strip__what">{line.label}</span>
                <span class="st-strip__move">
                  {line.from} <span class="st-strip__arrow">→</span> <b>{line.to}</b>
                </span>
                {#if !line.taken}
                  <button
                    class="st-strip__undo"
                    type="button"
                    aria-label={`Undo ${line.label} — back to ${line.from}`}
                    onclick={() => onUndoChange(line.id)}
                  >
                    Undo
                  </button>
                {/if}
              </li>
            {/each}
          </ol>
        {/if}
      </footer>
    </div>
  </div>
</div>
