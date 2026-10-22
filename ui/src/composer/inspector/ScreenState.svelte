<!--
  The sidebar's default state (COMPOSER §4.8): screen actions (rename, icon,
  reset, delete) previously in the edit bar menu. Actions open sheets; selection
  status explains why field controls are absent when no record or component is picked.
-->
<script lang="ts">
  import Icon from '../../shell/Icon.svelte';
  import { app } from '../../session.svelte';
  import { nameOf } from '../../types';

  /** The view every screen act addresses — the id, spoken as a name. */
  const view = $derived(app.outcome?.view ?? app.selected ?? '');
  const heading = $derived(
    app.navigation.find((entry) => entry.view === view)?.title ?? nameOf(view),
  );
  /** The glyph the rail draws beside it, when the student chose one. */
  const glyph = $derived(app.navigation.find((entry) => entry.view === view)?.icon ?? '');
  /** Whether the view draws something of its own to fall back to (§4.2). */
  const hasArrangement = $derived(app.outcome?.panel != null || app.outcome?.type != null);

  /** The acts, in the order the pane offers them — the row's own glyph, its
   *  title, and the sentence the sheet it opens leads with. */
  const acts = $derived([
    {
      id: 'rename',
      icon: 'type',
      title: 'Rename screen',
      say: 'Change the screen title',
      command: 'list.set',
      run: () => void app.openSheet({ kind: 'screen.rename', view }),
    },
    {
      id: 'icon',
      icon: 'palette',
      title: 'Change icon',
      say: 'Choose a different icon',
      command: 'list.set',
      run: () => void app.openSheet({ kind: 'screen.icon', view }),
    },
    ...(app.screenIsCustom && hasArrangement
      ? [
          {
            id: 'reset',
            icon: 'squareStack',
            title: 'Use SAM’s design again',
            say: 'Take your components off and draw it as SAM designed it',
            command: 'view.setComponents',
            run: () => void app.resetScreen(),
          },
        ]
      : []),
  ]);
</script>

<header class="cmp-insp__head">
  <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
    <Icon name={glyph || 'panel'} />
  </span>
  <div class="cmp-insp__who">
    <h2 class="cmp-insp__title">{heading}</h2>
    <p class="cmp-insp__sub">{app.screenIsCustom ? 'Your arrangement' : 'SAM’s own arrangement'}</p>
  </div>
  <!-- The pane's own door out (`screen.inspector`, §4.8): putting the sidebar
       away is not leaving the mode, and a press on any component brings it
       back — the canvas is what asks for it. -->
  <button
    class="cd-iconbtn cmp-insp__close"
    type="button"
    data-command="screen.inspector"
    data-placement="composer.inspector"
    aria-label="Close the editing sidebar"
    title="Put this pane away — press any component to bring it back"
    onclick={() => app.closeInspector()}
  >
    <Icon name="close" />
  </button>
</header>

<section class="cmp-insp__group" aria-label="This screen">
  {#each acts as act (act.id)}
    <button
      class="cmp-insp__act"
      type="button"
      data-act={act.id}
      data-command={act.command}
      data-placement="composer.inspector"
      disabled={!view}
      onclick={act.run}
    >
      <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
        <Icon name={act.icon} />
      </span>
      <span class="cmp-insp__acttext">
        <span class="cmp-insp__actname">{act.title}</span>
        <span class="cmp-insp__actsay">{act.say}</span>
      </span>
      <span class="cmp-insp__go" aria-hidden="true"><Icon name="chevronright" size={14} /></span>
    </button>
  {/each}
</section>

<section class="cmp-insp__group" aria-label="Selected component">
  <p class="cmp-insp__eyebrow">Selected component</p>
  <div class="cmp-insp__state">
    <span class="cd-glyph cd-glyph--sm cd-glyph--quiet" aria-hidden="true">
      <Icon name="pointer" />
    </span>
    <span class="cmp-insp__acttext">
      <span class="cmp-insp__actname">No component selected</span>
      <span class="cmp-insp__actsay">
        Press a component to select it, or a row inside one to edit its record.
      </span>
    </span>
  </div>
</section>

<!-- The one gesture the pane cannot show, because it lives on the canvas: the
     bar already says the drag, so what is said here is only what nothing else
     says — the arrows switch the design. -->
<p class="cmp-insp__tip">The arrows beside a component switch its design.</p>

<section class="cmp-insp__group" aria-label="Remove">
  <button
    class="cmp-insp__act cmp-insp__act--danger"
    type="button"
    data-act="remove"
    data-command="view.delete"
    data-placement="composer.inspector"
    disabled={!view}
    onclick={() => void app.openSheet({ kind: 'screen.remove', view })}
  >
    <span class="cd-glyph cd-glyph--sm cd-glyph--danger" aria-hidden="true">
      <Icon name="close" />
    </span>
    <span class="cmp-insp__acttext">
      <span class="cmp-insp__actname">Remove screen</span>
      <span class="cmp-insp__actsay">Permanently delete this screen</span>
    </span>
    <span class="cmp-insp__go" aria-hidden="true"><Icon name="chevronright" size={14} /></span>
  </button>
</section>
