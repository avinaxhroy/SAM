<!--
  SYSTEM · THE RAIL, IN ITS OWN ORDER (2026-09-29).

  The destination list is part of the *Screens* place — a destination is the
  rail's entry into a view — and it is the one part of the machine whose name is
  a field a student writes. Both writes are the same registry id (`list.set`):
  the label the rail shows, and the glyph it wears. The shipped screen put the
  label in the row and the glyph behind the row's own `More` control, and this
  keeps that: one control per row until the row asks for the rest (D3), with the
  file door and the terminal twin beside the glyph they describe.

  `list.delete` takes the destination off the rail and leaves the screen in the
  plan, which is what its own sentence says; the delete itself rides the app's
  shipped sheet, so the count comes from the engine's dry run.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import { MARKS, type RailRow, type SystemActions } from './props';

  let {
    rail,
    iconNames,
    acts,
  }: {
    rail: RailRow[];
    iconNames: string[];
    acts: SystemActions;
  } = $props();

  /** One row's own `More` control, and which row has it open. */
  let open = $state<string | null>(null);
</script>

<section class="cd-card">
  <div class="cd-card__head">
    <span class="cd-ictile"><Icon name="dot" /></span>
    <div>
      <h2 class="cd-card__title">The rail, in its own order</h2>
      <p class="cd-card__sub">{rail.length} places, each opening one screen</p>
    </div>
  </div>

  {#if rail.length === 0}
    <div class="cd-empty cd-empty--tight">
      <span class="cd-empty__art"><Icon name="dot" size={24} /></span>
      <p class="cd-empty__t">The rail has no destinations</p>
      <p class="cd-empty__s">A screen with no destination is reachable from the console only.</p>
    </div>
  {:else}
    <div class="cd-coll">
      {#each rail as entry (entry.view)}
        <div class="cd-coll__row" data-destination={entry.view}>
          <span class="sys-mark" aria-hidden="true"><Icon name={MARKS[entry.mark] ? entry.mark : 'dot'} size={12} /></span>
          <span class="cd-coll__body">
            <label class="cd-sr" for={`rail-${entry.view}`}>What the rail calls {entry.title}</label>
            <input
              id={`rail-${entry.view}`}
              class="cd-wellfield sys-railname"
              type="text"
              value={entry.title}
              aria-label={`The rail’s name for ${entry.opens}`}
              data-command="list.set"
              data-placement="system"
              onchange={(event) => acts.onRailName(entry.view, event.currentTarget.value.trim() || entry.title)}
            />
            <span class="cd-rowmeta">
              <IdPair value={entry.view} title="Copy the screen this destination opens" />
              <span>opens {entry.opens}</span>
              {#if entry.missing}<span class="cd-chip cd-chip--risk">its screen is gone</span>{/if}
            </span>
          </span>
          <span class="cd-coll__right">
            <button
              class="cd-chip cd-chip--outline sys-open"
              type="button"
              aria-expanded={open === entry.view}
              aria-label={`More about ${entry.title}`}
              onclick={() => (open = open === entry.view ? null : entry.view)}
            >
              {open === entry.view ? 'Close' : 'More'}
            </button>
          </span>
        </div>

        {#if open === entry.view}
          <div class="cd-detail sys-detail">
            <div class="cd-detail__grid">
              <div class="sys-detail__cell">
                <p class="cd-detail__k">The glyph it wears</p>
                <p class="cd-detail__v">
                  {entry.mark ? 'Named in the plan. An unknown name draws the app’s own dot.' : 'The app draws its own fallback dot.'}
                </p>
                <label class="sys-doors">
                  <span class="cd-sr">The glyph for {entry.title}</span>
                  <input
                    class="cd-wellfield"
                    type="text"
                    list="sam-icons"
                    value={entry.mark}
                    placeholder="the app’s own glyph"
                    aria-label={`The glyph for ${entry.title}`}
                    data-command="list.set"
                    data-placement="system"
                    onchange={(event) => acts.onRailIcon(entry.view, event.currentTarget.value.trim())}
                  />
                </label>
                <datalist id="sam-icons">
                  {#each iconNames as name (name)}<option value={name}></option>{/each}
                </datalist>
              </div>
              <div class="sys-detail__cell">
                <p class="cd-detail__k">Its place in the file</p>
                <div class="sys-doors">
                  <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" data-developer title="Open the rail’s own document in the source pane" onclick={() => acts.onFile('content/shell.json')}>
                    Edit the file
                  </button>
                  <IdPair label="Terminal" value={entry.terminal} />
                </div>
              </div>
            </div>
            <div class="sys-acts">
              <button
                class="cd-chip cd-chip--outline"
                type="button"
                data-command="list.delete"
                data-placement="system"
                aria-label={`Remove ${entry.title} from the rail`}
                onclick={() => acts.onDelete({ command: 'list.delete', key: entry.view, title: entry.title, label: 'Take it off the rail', note: 'The screen itself stays in the plan — only its place on the rail goes.' })}
              >
                Take it off the rail
              </button>
              <span class="cd-hint">The screen itself stays in the plan — only its place on the rail goes.</span>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</section>

<style>
  .cd-coll__row { min-height: calc(48px * var(--ui-s)); }
  .sys-railname { max-width: 22ch; min-height: var(--hit); }
  .cd-rowmeta { flex-wrap: wrap; row-gap: var(--space-3xs); }
</style>
