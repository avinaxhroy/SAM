<!--
  System editor row body: provides editing surfaces for schema columns,
  ladders, screen block trees, scheduler parameters, and rules (§4.8, §4.10).
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import IdPair from '../../shell/IdPair.svelte';
  import BlockTree from '../../system/BlockTree.svelte';
  import Projection from './Projection.svelte';
  import { REDUCES, type EditorBody, type Projection as PendingWrite, type SystemActions } from './props';

  let {
    editor,
    projection = null,
    busy = false,
    frame = 'index',
    acts,
  }: {
    editor: EditorBody;
    projection?: PendingWrite | null;
    busy?: boolean;
    frame?: 'index' | 'sheet' | 'page';
    acts: SystemActions;
  } = $props();

  /** A column's own verbs are one control per row until the row asks for them
   *  (D3), so which column is open is the editor's own, presentational state. */
  let openColumn = $state<string | null>(null);

  /** A figure's screen is chosen from the plan's own views — 20-odd labels, so
   *  it is a picker with a find field rather than a native select
   *  (`design/controls.md` §2), and which one is open is the editor's state. */
  let picking = $state(false);
  let find = $state('');
  const choices = $derived.by(() => {
    const all = editor.figure?.fields.views ?? [];
    const needle = find.trim().toLowerCase();
    return needle === '' ? all : all.filter((entry) => entry.label.toLowerCase().includes(needle));
  });
</script>

{#if editor.place === 'kinds'}
  <h3 class="cd-sec">
    <span class="cd-sec__t">Its columns, in its own order</span>
    <span class="cd-sec__act">
      <button
        class="cd-pill cd-pill--quiet cd-pill--sm"
        type="button"
        data-command="column.new"
        data-placement="system"
        disabled={busy}
        onclick={() => acts.onNewColumn(editor.key)}
      >
        {editor.addColumn ?? 'Add a column'}
      </button>
    </span>
  </h3>

  <div class="cd-coll">
    {#each editor.columns ?? [] as column (column.key)}
      {@const open = openColumn === column.key}
      <div class="cd-coll__row sys-colrow" data-column-key={column.key}>
        <span class="sys-mark" aria-hidden="true"><Icon name={column.mark} size={12} /></span>
        <span class="cd-coll__body">
          <button
            class="cd-coll__title sys-colname"
            type="button"
            aria-expanded={open}
            aria-label={`${column.label} — its verbs`}
            onclick={() => (openColumn = open ? null : column.key)}
          >
            {column.label}
          </button>
          <span class="cd-rowmeta">
            <IdPair value={column.key} title={`Copy the key of ${column.label}`} />
            <span class="cd-chip" class:cd-chip--outline={column.holds.mod === 'outline'}>{column.holds.label}</span>
            {#if column.place}<span class="cd-chip">{column.place}</span>{/if}
            <span class="cd-hint">{column.pinned}</span>
          </span>
        </span>
        <span class="cd-coll__right">
          <button
            class="cd-iconbtn"
            type="button"
            aria-label={`Move ${column.label} earlier`}
            disabled={column.first || busy}
            data-command="column.reorder"
            data-placement="system"
            onclick={() => acts.onColumn('column.reorder', editor.key, column.key, -1)}
          >
            <Icon name="arrowup" />
          </button>
          <button
            class="cd-iconbtn"
            type="button"
            aria-label={`Move ${column.label} later`}
            disabled={column.last || busy}
            data-command="column.reorder"
            data-placement="system"
            onclick={() => acts.onColumn('column.reorder', editor.key, column.key, 1)}
          >
            <Icon name="arrowdown" />
          </button>
        </span>
      </div>

      {#if open}
        <div class="cd-detail sys-detail">
          <div class="sys-acts">
            <button class="cd-chip cd-chip--outline" type="button" data-command="column.rename" data-placement="system" disabled={busy} onclick={() => acts.onColumn('column.rename', editor.key, column.key)}>Rename</button>
            <button class="cd-chip cd-chip--outline" type="button" data-command="column.retype" data-placement="system" disabled={busy} onclick={() => acts.onColumn('column.retype', editor.key, column.key)}>Change what it holds</button>
            <button class="cd-chip cd-chip--outline" type="button" data-command="column.choices" data-placement="system" disabled={busy} onclick={() => acts.onColumn('column.choices', editor.key, column.key)}>The choices</button>
            <button class="cd-chip cd-chip--outline" type="button" data-command="column.duplicate" data-placement="system" disabled={busy} onclick={() => acts.onColumn('column.duplicate', editor.key, column.key)}>Copy it</button>
            <button class="cd-chip cd-chip--outline" type="button" data-command="column.delete" data-placement="system" disabled={busy} onclick={() => acts.onColumn('column.delete', editor.key, column.key)}>Delete it</button>
          </div>
          <div class="sys-doors">
            <span class="cd-hint">The same thing, the other door</span>
            <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" data-developer title="Open the kinds document in the source pane" onclick={() => acts.onFile(editor.file)}>
              Edit the file
            </button>
            <IdPair label="Terminal" value={column.terminal} />
          </div>
        </div>
      {/if}
    {/each}
  </div>

  {#if (editor.columns ?? []).length === 0}
    <p class="sys-nothing__say">This kind declares no columns — it cannot hold anything yet.</p>
  {/if}

  {#if editor.privacy}
    <div class="sys-acts sys-note">
      <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" data-command="type.setPrivate" data-placement="system" disabled={busy} onclick={() => acts.onPrivate(editor.key, editor.privacy?.next ?? false)}>
        {editor.privacy.label}
      </button>
      <span class="cd-hint">{editor.privacy.note}</span>
    </div>
  {/if}

  {#if editor.ladder}
    <h3 class="cd-sec"><span class="cd-sec__t">{editor.ladder.steps.length > 0 && editor.ladder.options ? 'The ladder it climbs' : 'Its steps'}</span></h3>
    <div class="sys-ladder">
      {#each editor.ladder.steps as step, index (step.stage)}
        {#if index > 0}<span class="sys-ladder__rule" aria-hidden="true"></span>{/if}
        <span class="cd-chip cd-chip--outline sys-ladder__step">{step.stage}</span>
      {/each}
    </div>
    {#if editor.ladder.completeWhen}
      <p class="sys-ladder__gate">It is finished at <b>{editor.ladder.completeWhen}</b>.
        {#each editor.ladder.gates as gate (gate.stage)}<span> · {gate.stage} {gate.gate}</span>{/each}
      </p>
    {/if}

    {#if editor.ladder.options}
      <div class="sys-picks sys-picks--row" role="radiogroup" aria-label="The ladder to move to">
        {#each editor.ladder.options as option (option.name)}
          <button
            class="sys-pick"
            type="button"
            role="radio"
            aria-checked={editor.ladder.chosen === option.name}
            aria-label={`The ${option.label} ladder`}
            data-sel={editor.ladder.chosen === option.name ? '1' : '0'}
            disabled={option.inUse || busy}
            onclick={() => acts.onChooseLadder(editor.key, option.name)}
          >
            <span class="sys-pick__head">
              <span class="sys-pick__tick" aria-hidden="true">
                {#if editor.ladder.chosen === option.name}<Icon name="check" size={12} />{/if}
              </span>
              <span class="sys-pick__t">{option.label} ladder</span>
              {#if option.inUse}<span class="cd-chip">in use</span>{/if}
            </span>
            <span class="sys-pick__s">{option.stages}</span>
          </button>
        {/each}
      </div>
      <p class="cd-hint sys-note">
        Changing it re-bases every record of this kind and <b>clears the review date stored against
        each one</b> — the engine does this, not the screen, and it is why the consequence is shown
        before the button.
      </p>

      {#if editor.ladder.strategy}
        <h3 class="cd-sec"><span class="cd-sec__t">What happens to the stages it is on</span></h3>
        <div class="sys-picks sys-picks--row" role="radiogroup" aria-label="What happens to the stages">
          <button class="sys-pick" type="button" role="radio" aria-checked={editor.ladder.strategy === 'fresh'} data-sel={editor.ladder.strategy === 'fresh' ? '1' : '0'} onclick={() => acts.onStrategy(true)}>
            <span class="sys-pick__head">
              <span class="sys-pick__tick" aria-hidden="true">{#if editor.ladder.strategy === 'fresh'}<Icon name="check" size={12} />{/if}</span>
              <span class="sys-pick__t">Start every record again</span>
            </span>
            <span class="sys-pick__s">Nothing is carried across. Every record that has a stage loses it and its review date.</span>
          </button>
          <button class="sys-pick" type="button" role="radio" aria-checked={editor.ladder.strategy === 'carry'} data-sel={editor.ladder.strategy === 'carry' ? '1' : '0'} onclick={() => acts.onStrategy(false)}>
            <span class="sys-pick__head">
              <span class="sys-pick__tick" aria-hidden="true">{#if editor.ladder.strategy === 'carry'}<Icon name="check" size={12} />{/if}</span>
              <span class="sys-pick__t">Carry each stage across</span>
            </span>
            <span class="sys-pick__s">You choose where each current stage lands. A stage with no target is dropped.</span>
          </button>
        </div>
      {/if}

      {#if editor.ladder.strategy === 'carry' && editor.ladder.map}
        <div class="sys-map">
          {#each editor.ladder.map as row (row.from)}
            <div class="sys-map__row">
              <span class="sys-map__from">{row.from}</span>
              <span class="sys-picks sys-picks--row sys-map__targets">
                {#each row.targets as target (target.to)}
                  <button
                    class="sys-pick sys-pick--mini"
                    type="button"
                    role="radio"
                    aria-checked={target.chosen}
                    aria-label={`${row.from} becomes ${target.label}`}
                    data-sel={target.chosen ? '1' : '0'}
                    onclick={() => acts.onMapStage(row.from, target.to)}
                  >
                    <span class="sys-pick__tick" aria-hidden="true">{#if target.chosen}<Icon name="check" size={10} />{/if}</span>
                    <span class="sys-pick__t">{target.label}</span>
                  </button>
                {/each}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
{:else if editor.place === 'screens'}
  {#if editor.panels}
    <h3 class="cd-sec"><span class="cd-sec__t">The screen this opens</span></h3>
    <p class="cd-hint sys-note">
      {editor.panels.some((card) => card.current && card.value !== '')
        ? `It opens the ${editor.panels.find((card) => card.current && card.value !== '')?.label} screen today.`
        : 'It draws its own blocks today.'}
      Its blocks stay in the file either way, and stay editable below.
    </p>
    <div class="sys-picks sys-picks--grid" role="radiogroup" aria-label="The screen this destination draws">
      {#each editor.panels as card (card.value)}
        <button
          class="sys-pick"
          type="button"
          role="radio"
          aria-checked={card.current}
          aria-label={`Draw ${card.label}`}
          data-sel={card.current ? '1' : '0'}
          disabled={busy}
          onclick={() => acts.onChoosePanel(editor.blocks?.view ?? '', card.value)}
        >
          <span class="sys-pick__head">
            <span class="sys-pick__tick" aria-hidden="true">{#if card.current}<Icon name="check" size={12} />{/if}</span>
            <span class="sys-pick__t">{card.label}</span>
          </span>
          <span class="sys-pick__s">{card.say}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if editor.blocks}
    <h3 class="cd-sec">
      <span class="cd-sec__t">Its blocks</span>
      {#if editor.ownQuery}
        <span class="cd-sec__act">
          <button class="cd-pill cd-pill--quiet cd-pill--sm" type="button" data-command="view.edit" data-placement="system" disabled={busy} onclick={() => acts.onEditView(editor.key)}>
            Its own query
          </button>
        </span>
      {/if}
    </h3>
    <BlockTree
      view={editor.blocks.view}
      blocks={editor.blocks.nodes}
      busy={busy}
      onrun={(id, params) => acts.onBlock(id, params)}
    />
  {/if}
{:else if editor.place === 'rules'}
  {#if editor.figure}
    {@const figure = editor.figure.fields}
    <h3 class="cd-sec">
      <span class="cd-sec__t">The figure</span>
      <span class="cd-sec__act cd-hint">one key per field, in the rules the engine reads</span>
    </h3>
    <div class="cd-form sys-fig">
      <div class="cd-formrow">
        <label class="cd-formrow__label" for="fig-label">Label</label>
        <IdPair value="label" />
        <input
          id="fig-label"
          class="cd-wellfield cd-formrow__input"
          type="text"
          autocomplete="off"
          spellcheck="false"
          aria-label={`What the figure is called: label`}
          value={figure.label}
          disabled={busy}
          data-command="metric.set"
          data-placement="system"
          onchange={(event) => acts.onFigure('label', event.currentTarget.value)}
        />
      </div>

      <div class="cd-formrow">
        <label class="cd-formrow__label" for="fig-view">Screen</label>
        <IdPair value="view" />
        <button
          id="fig-view"
          class="cd-chip cd-chip--outline"
          type="button"
          aria-expanded={picking}
          disabled={busy}
          data-command="metric.set"
          data-placement="system"
          onclick={() => (picking = !picking)}
        >
          {figure.views.find((entry) => entry.value === figure.view)?.label ?? editor.figure.source}
        </button>
      </div>

      {#if picking}
        <div class="cd-form sys-fig__picker">
          <label class="cd-searchpill">
            <Icon name="search" size={14} />
            <span class="cd-sr">Find a screen to fold</span>
            <input
              type="search"
              autocomplete="off"
              spellcheck="false"
              placeholder="Find a screen…"
              aria-label="Find a screen to fold"
              oninput={(event) => (find = event.currentTarget.value)}
            />
          </label>
          <div class="cd-chips" role="group" aria-label="The screens a figure can fold">
            {#each choices as entry (entry.value)}
              <button
                class="cd-chip"
                type="button"
                disabled={busy}
                aria-pressed={entry.value === figure.view}
                aria-label={`Fold ${entry.label}`}
                data-command="metric.set"
                data-placement="system"
                onclick={() => {
                  picking = false;
                  find = '';
                  acts.onFigure('view', entry.value);
                }}
              >
                {entry.label}
              </button>
            {/each}
          </div>
          {#if choices.length === 0}
            <p class="cd-hint">No screen in this plan is called that.</p>
          {/if}
        </div>
      {/if}

      <div class="cd-formrow">
        <label class="cd-formrow__label" for="fig-expr">Folds</label>
        <IdPair value="expr" />
        <input
          id="fig-expr"
          class="cd-wellfield cd-formrow__input"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="every record"
          aria-label={`The column it folds, or every record: expr`}
          value={figure.expr}
          disabled={busy}
          data-command="metric.set"
          data-placement="system"
          onchange={(event) => acts.onFigure('expr', event.currentTarget.value)}
        />
      </div>

      <div class="cd-formrow">
        <span class="cd-formrow__label" id="fig-fold">How</span>
        <IdPair value="reduce" />
        <span class="cd-segment sys-fig__segment" role="radiogroup" aria-labelledby="fig-fold">
          {#each Object.entries(REDUCES) as [id, word] (id)}
            <button
              type="button"
              role="radio"
              aria-checked={figure.reduce === id}
              aria-label={`Fold: ${word}`}
              disabled={busy}
              data-command="metric.set"
              data-placement="system"
              onclick={() => acts.onFigure('reduce', id)}
            >
              {word}
            </button>
          {/each}
        </span>
      </div>

      <div class="cd-formrow">
        <label class="cd-formrow__label" for="fig-unit">Unit</label>
        <IdPair value="unit" />
        <input
          id="fig-unit"
          class="cd-wellfield cd-formrow__input"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="no unit"
          aria-label={`The unit the figure reads in: unit`}
          value={figure.unit}
          disabled={busy}
          data-command="metric.set"
          data-placement="system"
          onchange={(event) => acts.onFigure('unit', event.currentTarget.value)}
        />
      </div>
    </div>
    <p class="cd-hint sys-note">
      Every field is one key of the figure; clearing the expression, the fold or the unit drops that
      key, and the label and the screen are what make a figure a figure.
    </p>
  {/if}

  <div class="cd-detail sys-detail sys-note">
    <div class="cd-detail__grid">
      <div class="sys-detail__cell">
        <p class="cd-detail__k">What it folds</p>
        <p class="cd-detail__v">{editor.figure?.folds}</p>
      </div>
      <div class="sys-detail__cell">
        <p class="cd-detail__k">Its place in the file</p>
        <div class="sys-doors">
          <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" data-developer title="Open the rules document in the source pane" onclick={() => acts.onFile(editor.file)}>
            Edit the file
          </button>
          <IdPair label="Terminal" value={editor.terminal} />
        </div>
      </div>
    </div>
  </div>
{:else if editor.place === 'scheduling'}
  {#if editor.schedulers}
    <h3 class="cd-sec"><span class="cd-sec__t">How a review comes back</span></h3>
    <div class="sys-picks sys-picks--row" role="radiogroup" aria-label="How a review comes back">
      {#each editor.schedulers as card (card.id)}
        <button
          class="sys-pick"
          type="button"
          role="radio"
          aria-checked={card.inUse}
          data-sel={card.inUse ? '1' : '0'}
          disabled={busy}
          onclick={() => acts.onChooseSchedule(card.id)}
        >
          <span class="sys-pick__head">
            <span class="sys-pick__tick" aria-hidden="true">{#if card.inUse}<Icon name="check" size={12} />{/if}</span>
            <span class="sys-pick__t">{card.label}</span>
            {#if card.inUse}<span class="cd-chip">in use</span>{/if}
          </span>
          <span class="sys-pick__s">{card.say}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if editor.gaps}
    <h3 class="cd-sec">
      <span class="cd-sec__t">The gaps, when the schedule is fixed</span>
      <span class="cd-sec__act cd-hint">only used by fixed gaps</span>
    </h3>
    <p class="cd-hint sys-note">
      A review you get right comes back after the first gap, then the second, then the third — and then
      it stops. Longer gaps mean fewer reviews a week and more forgetting between them.
    </p>
    <div class="sys-acts">
      {#each editor.gaps as gap, index (index)}
        <span class="cd-cluster">
          <span class="cd-hint">{index === 0 ? 'first' : 'then'}</span>
          <button class="cd-iconbtn" type="button" aria-label={`Shorten gap ${index + 1}`} disabled={gap <= 1 || busy} onclick={() => acts.onGap(index, -1)}>
            <Icon name="minus" />
          </button>
          <span class="cd-chip">{gap} {gap === 1 ? 'day' : 'days'}</span>
          <button class="cd-iconbtn" type="button" aria-label={`Lengthen gap ${index + 1}`} disabled={gap >= 90 || busy} onclick={() => acts.onGap(index, 1)}>
            <Icon name="plus" />
          </button>
        </span>
      {/each}
      <button class="cd-pill cd-pill--ghost cd-pill--sm" type="button" disabled={editor.gaps.join() === '1,7,30' || busy} onclick={() => acts.onGapsReset()}>
        Back to 1, 7, 30
      </button>
      <IdPair label="Key" value="fixedIntervals" title="Copy the key" />
    </div>
  {/if}

  <h3 class="cd-sec"><span class="cd-sec__t">The plan’s own load, from its stored dates</span></h3>
  {#if (editor.load ?? []).length === 0}
    <p class="sys-nothing__say">Nothing is scheduled ahead. That is a fact, not a warning — a review appears here once something has been graded.</p>
  {:else}
    <div class="sys-curve">
      {#each editor.load ?? [] as day (day.when)}
        <div class="cd-meterrow">
          <span class="cd-meterrow__say">
            <span>{day.when}</span>
            <span class="cd-metric">
              <span class="cd-metric__n">{day.due}</span>
              <span class="cd-metric__d">{day.due === 1 ? 'review due' : 'reviews due'}</span>
            </span>
          </span>
          <span class="cd-meter">
            <span class="cd-meter__track">
              <span class="cd-meter__fill" style={`--v: ${day.share}%`}></span>
            </span>
          </span>
        </div>
      {/each}
    </div>
  {/if}
{:else if editor.place === 'stages'}
  {#if editor.ladder}
    <div class="sys-ladder">
      {#each editor.ladder.steps as step, index (step.stage)}
        {#if index > 0}<span class="sys-ladder__rule" aria-hidden="true"></span>{/if}
        <span class="cd-chip cd-chip--outline sys-ladder__step">{step.stage}</span>
      {/each}
    </div>
    {#if editor.ladder.completeWhen}
      <p class="sys-ladder__gate">It is finished at <b>{editor.ladder.completeWhen}</b>.</p>
    {/if}
    <ul class="sys-proj__lines sys-note">
      {#each editor.ladder.steps as step (step.stage)}
        <li class="sys-proj__line"><b>{step.stage}</b> — {step.gate ?? 'no gate: a session can put a record here'}</li>
      {/each}
    </ul>
    <p class="cd-hint sys-note">
      A kind’s ladder is changed on the kind itself — open it under Kinds, and the change shows what it
      would do to every record before anything is written.
    </p>
  {/if}
{/if}

{#if editor.door}
  <div class="sys-acts sys-note">
    <button class="cd-chip cd-chip--outline" type="button" data-command={editor.door.command} data-placement="system" disabled={busy} aria-label={editor.door.label} onclick={() => acts.onDelete(editor.door!)}>
      {editor.door.label}
    </button>
    <span class="cd-hint">{editor.door.note}</span>
  </div>
{/if}

{#if projection}
  <Projection {projection} {acts} />
{/if}

<style>
  /* A column's name is how its verbs are reached, so it takes the hit floor
     while still reading as a title — the same trick the lab used for the
     machine's key control. */
  .sys-colname {
    min-height: var(--hit);
    min-width: var(--hit);
    padding-block: calc(8px * var(--ui-s));
    padding-inline-end: var(--space-xs);
    margin-block: calc(-8px * var(--ui-s));
    text-align: left;
    font: inherit;
    color: inherit;
  }
  .sys-note { margin-top: var(--ui-gap-sm); }
  /* The figure's picker sits under the row it belongs to, across the whole form
     grid — a pick that opened inside the third column would clip the labels it
     exists to show. */
  .sys-fig__picker { grid-column: 1 / -1; }
  /* The fold’s five options are phrases, not words: at a form row’s control
     column (170px in the sheet) a wrapped segment’s pill radius — 999px — clamps
     into a circle. So the row gives the segment its own line, the pills tighten
     to the padding a menu’s own segment uses (`components.css` §5), and the
     corner falls back to a tile for the text scale that pushes them to two
     lines. Words are `REDUCES`’ own, the ones the row’s fact prints. */
  .sys-fig__segment {
    grid-column: 1 / -1;
    justify-self: start;
    max-width: 100%;
    flex-wrap: wrap;
    border-radius: var(--r-tile);
  }
  .sys-fig__segment button { padding: 0 var(--space-sm); white-space: nowrap; }
  /* The machine's place segment is this screen's one solid-ink object
     (design.md §6), so an option chosen inside the sheet is the pressed well
     and not a second ink pill — the treatment `reviews.css` gives its method
     pills for the same reason, on the same well. The inset rule is the
     chosen option's non-colour cue, beside the ink-worded label. */
  .sys-fig__segment button[aria-checked='true'],
  .sys-fig__picker .cd-chips button[aria-pressed='true'] {
    background: var(--well-2);
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--rule-strong);
  }
  .sys-ladder__gate { font-size: var(--ui-text-sm); }
  /* A column's row carries a name and its own verbs, so it needs both lines —
     and the name keeps its full width rather than ellipsising to "We…". */
  .sys-colrow { min-height: calc(48px * var(--ui-s)); }
  .sys-acts { margin-top: var(--ui-gap-sm); }
</style>
